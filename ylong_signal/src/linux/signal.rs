// Copyright (c) 2023 Huawei Device Co., Ltd.
// Licensed under the Apache License, Version 2.0 (the "License");
// you may not use this file except in compliance with the License.
// You may obtain a copy of the License at
//
//     http://www.apache.org/licenses/LICENSE-2.0
//
// Unless required by applicable law or agreed to in writing, software
// distributed under the License is distributed on an "AS IS" BASIS,
// WITHOUT WARRANTIES OR CONDITIONS OF ANY KIND, either express or implied.
// See the License for the specific language governing permissions and
// limitations under the License.

use std::sync::Arc;
use std::{io, mem};

use libc::{c_int, c_void, sigaction, siginfo_t, SIGFPE, SIGILL, SIGKILL, SIGSEGV, SIGSTOP};

use crate::linux::signal_manager::{SigAction, SignalManager};

// These signals should not be handled at all due to POSIX settings or their
// specialness
const SIGNAL_BLOCK_LIST: &[c_int] = &[SIGSEGV, SIGKILL, SIGSTOP, SIGILL, SIGFPE];

type SigHandler = dyn Fn(&siginfo_t) + Send + Sync;

#[derive(Clone)]
pub struct Signal {
    sig_num: c_int,
    old_act: sigaction,
    new_act: Option<Arc<SigHandler>>,
}

impl Signal {
    pub(crate) unsafe fn register_action<F>(sig_num: c_int, handler: F) -> io::Result<()>
    where
        F: Fn(&siginfo_t) + Sync + Send + 'static,
    {
        if SIGNAL_BLOCK_LIST.contains(&sig_num) {
            return Err(io::ErrorKind::InvalidInput.into());
        }

        let global = SignalManager::get_instance();
        let act = Arc::new(handler);
        let mut write_guard = global.data.write();
        let mut signal_map = write_guard.clone();

        if let Some(signal) = signal_map.get_mut(&sig_num) {
            if signal.new_act.is_some() {
                return Err(io::ErrorKind::AlreadyExists.into());
            } else {
                signal.new_act = Some(act);
            }
        } else {
            let old_act = SigAction::get_old_action(sig_num)?;
            global.race_old.write().store(Some(old_act));

            let signal = Signal::new(sig_num, act)?;
            signal_map.insert(sig_num, signal);
        }
        write_guard.store(signal_map);
        Ok(())
    }

    pub(crate) fn deregister_action(sig_num: c_int) {
        let global = SignalManager::get_instance();
        let mut write_guard = global.data.write();
        let mut signal_map = write_guard.clone();
        if let Some(signal) = signal_map.get_mut(&sig_num) {
            signal.new_act = None;
        }
        write_guard.store(signal_map);
    }

    pub(crate) fn deregister_hook(sig_num: c_int) -> io::Result<()> {
        let mut new_act: libc::sigaction = unsafe { mem::zeroed() };
        let mut old_act: libc::sigaction = unsafe { mem::zeroed() };
        new_act.sa_sigaction = libc::SIG_DFL;

        let global = SignalManager::get_instance();
        let mut write_guard = global.data.write();
        let mut signal_map = write_guard.clone();

        unsafe {
            if libc::sigaction(sig_num, &new_act, &mut old_act) != 0 {
                return Err(io::Error::last_os_error());
            }
        }
        signal_map.remove(&sig_num);
        write_guard.store(signal_map);
        Ok(())
    }

    fn new(sig_num: c_int, new_act: Arc<SigHandler>) -> io::Result<Signal> {
        // c structure, initialized it to all zeros
        let mut handler: libc::sigaction = unsafe { mem::zeroed() };
        let mut old_act: libc::sigaction = unsafe { mem::zeroed() };

        handler.sa_sigaction = sig_handler as usize;
        handler.sa_flags = libc::SA_RESTART | libc::SA_SIGINFO;

        unsafe {
            if libc::sigaction(sig_num, &handler, &mut old_act) != 0 {
                return Err(io::Error::last_os_error());
            }
        }
        Ok(Signal {
            sig_num,
            old_act,
            new_act: Some(new_act),
        })
    }
}

extern "C" fn sig_handler(sig_num: c_int, sig_info: *mut siginfo_t, data: *mut c_void) {
    let global = SignalManager::get_instance();
    let race_fallback = global.race_old.read();
    let signal_map = global.data.read();

    if let Some(signal) = signal_map.get(&sig_num) {
        execute_act(&signal.old_act, signal.sig_num, sig_info, data);

        // sig_info should not be null, but in a sig handler we cannot panic directly,
        // therefore we abort instead
        if sig_info.is_null() {
            unsafe { libc::abort() };
        }

        let info = unsafe { &*sig_info };
        if let Some(act) = &signal.new_act {
            act(info);
        }
    } else if let Some(old_act) = race_fallback.as_ref() {
        // There could be a race condition between swapping the old handler with the new
        // handler and storing the change back to the global during the register
        // procedure. Because of the race condition, the old handler and the new
        // action could both not get executed. In order to prevent this, we
        // store the old handler into global before swapping the handler in
        // register. And during the handler execution, if the the action
        // of the signal cannot be found, we execute this old handler instead if the
        // sig_num matches.
        if old_act.sig_num == sig_num {
            execute_act(&old_act.act, sig_num, sig_info, data);
        }
    }
}

fn execute_act(act: &sigaction, sig_num: c_int, sig_info: *mut siginfo_t, data: *mut c_void) {
    let handler = act.sa_sigaction;

    // SIG_DFL for the default action.
    // SIG_IGN to ignore this signal.
    if handler == libc::SIG_DFL || handler == libc::SIG_IGN {
        return;
    }

    // If SA_SIGINFO flag is set, then the signal handler takes three arguments, not
    // one. In this case, sa_sigaction should be set instead of sa_handler.
    // We transmute the handler from ptr to actual function type according to
    // definition.
    if act.sa_flags & libc::SA_SIGINFO == 0 {
        let action = unsafe { mem::transmute::<usize, extern "C" fn(c_int)>(handler) };
        action(sig_num);
    } else {
        type Action = extern "C" fn(c_int, *mut siginfo_t, *mut c_void);
        let action = unsafe { mem::transmute::<usize, Action>(handler) };
        action(sig_num, sig_info, data);
    }
}
