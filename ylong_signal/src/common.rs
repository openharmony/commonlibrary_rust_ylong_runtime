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

use std::io;
use std::sync::Arc;

use libc::{c_int, SIGFPE, SIGILL, SIGSEGV};
#[cfg(not(windows))]
use libc::{siginfo_t, SIGKILL, SIGSTOP};

use crate::sig_map::SigMap;

/// These signals should not be handled at all due to POSIX settings or their
/// specialness
#[cfg(windows)]
pub const SIGNAL_BLOCK_LIST: &[c_int] = &[SIGILL, SIGFPE, SIGSEGV];

/// These signals should not be handled at all due to POSIX settings or their
/// specialness
#[cfg(not(windows))]
pub const SIGNAL_BLOCK_LIST: &[c_int] = &[SIGSEGV, SIGKILL, SIGSTOP, SIGILL, SIGFPE];

#[cfg(windows)]
type Action = libc::sighandler_t;
#[cfg(not(windows))]
type Action = libc::sigaction;

#[cfg(not(windows))]
use crate::unix::sig_handler;
#[cfg(windows)]
use crate::windows::sig_handler;

#[cfg(windows)]
type ActionPtr = libc::sighandler_t;
#[cfg(not(windows))]
type ActionPtr = usize;

#[allow(non_camel_case_types)]
#[cfg(windows)]
pub(crate) struct siginfo_t;

type SigHandler = dyn Fn(&siginfo_t) + Send + Sync;

#[derive(Clone)]
pub(crate) struct Signal {
    pub(crate) old_act: Action,
    pub(crate) new_act: Option<Arc<SigHandler>>,
}

pub(crate) struct SigAction {
    pub(crate) sig_num: c_int,
    pub(crate) act: Action,
}

impl Signal {
    pub(crate) fn new(sig_num: c_int, new_act: Arc<SigHandler>) -> io::Result<Signal> {
        let old_act = Self::replace_sigaction(sig_num, sig_handler as ActionPtr)?;

        Ok(Signal {
            old_act,
            new_act: Some(new_act),
        })
    }

    pub(super) unsafe fn register_action<F>(sig_num: c_int, handler: F) -> io::Result<()>
    where
        F: Fn(&siginfo_t) + Sync + Send + 'static,
    {
        if SIGNAL_BLOCK_LIST.contains(&sig_num) {
            return Err(io::ErrorKind::InvalidInput.into());
        }

        let sig_map = SigMap::get_instance();
        let act = Arc::new(handler);
        let mut write_guard = sig_map.data.write();
        let mut new_map = write_guard.clone();

        if let Some(signal) = new_map.get_mut(&sig_num) {
            if signal.new_act.is_some() {
                return Err(io::ErrorKind::AlreadyExists.into());
            } else {
                signal.new_act = Some(act);
            }
        } else {
            let old_act = SigAction::get_old_action(sig_num)?;
            sig_map.race_old.write().store(Some(old_act));

            let signal = Signal::new(sig_num, act)?;
            new_map.insert(sig_num, signal);
        }
        write_guard.store(new_map);
        Ok(())
    }

    pub(super) fn deregister_action(sig_num: c_int) -> io::Result<()> {
        let sig_map = SigMap::get_instance();
        let mut write_guard = sig_map.data.write();
        let mut new_map = write_guard.clone();
        if let Some(signal) = new_map.remove(&sig_num) {
            #[cfg(not(windows))]
            Self::replace_sigaction(sig_num, signal.old_act.sa_sigaction)?;
            #[cfg(windows)]
            Self::replace_sigaction(sig_num, signal.old_act)?;
        }
        write_guard.store(new_map);
        Ok(())
    }

    pub(super) fn deregister_hook(sig_num: c_int) -> io::Result<()> {
        let global = SigMap::get_instance();
        let mut write_guard = global.data.write();
        let mut signal_map = write_guard.clone();

        Self::replace_sigaction(sig_num, libc::SIG_DFL)?;

        signal_map.remove(&sig_num);
        write_guard.store(signal_map);
        Ok(())
    }
}
