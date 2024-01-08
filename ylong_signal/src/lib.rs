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

#![cfg(target_os = "linux")]

//! # ylong_signal
//! Provides methods to set or unset an action for signal handler. Currently the
//! crate only supports linux platform.

pub(crate) mod linux;
pub(crate) mod spin_rwlock;

use std::io;
use std::os::raw::c_int;

use crate::linux::signal::Signal;

/// Registers a user-defined function for the signal action.
///
/// Currently this function only supports registering one action for each
/// signal. However, if another component in the process sets a signal action
/// without using this method, this old action will not be overwritten by the
/// new registered action. Instead, the newly set signal hook will execute the
/// old action, then the new action.
///
/// # Errors
///
/// Calling this fuction twice on the same signal without a call to
/// [`deregister_signal_action`] will result in an `AlreadyExists` error.
///
/// Calling this function on the following signal will result in a
/// `InvalidInput` error:
/// - `SIGSEGV`
/// - `SIGKILL`
/// - `SIGSTOP`
/// - `SIGFPE`
/// - `SIGILL`
/// This function doesn't support setting actions for these signals due to POSIX
/// settings or their needs of special handling.
///
/// # Safety
///
/// This function is unsafe, because it sets a function to be run in a signal
/// handler as there are a lot of limitations (async-signal-safe) on what you
/// can do inside a signal handler. For example, you should not use blocking
/// Mutex since it could cause the program become deadlocked.
///
/// # Example
/// ```no run
/// let res = unsafe {
///     ylong_signal::register_signal_action(libc::SIGTERM, move || {
///         println!("inside SIGTERM signal handler");
///     })
/// };
/// assert!(res.is_ok());
/// ```
pub unsafe fn register_signal_action<F>(sig_num: c_int, handler: F) -> io::Result<()>
where
    F: Fn() + Sync + Send + 'static,
{
    Signal::register_action(sig_num, move |_| handler())
}

/// Deregisters the signal action set to a specific signal by a previous call to
/// [`register_signal_action`].
///
/// If the signal passed in has not been set before by
/// [`register_signal_action`], this function will do nothing.
///
/// If the signal passed in has been set before by [`register_signal_action`],
/// then the action of the signal will be dropped, but the signal handler still
/// exists, which means the program will not execute the default signal handler
/// when signals come; this signal will just be ignored.
///
/// After calling this function, you could call [`register_signal_action`] again
/// on the same signal.
///
/// # Example
/// ```no run
/// ylong_signal::deregister_signal_action(libc::SIGTERM);
/// ```
pub fn deregister_signal_action(sig_num: c_int) {
    Signal::deregister_action(sig_num);
}

/// Deregisters the signal handler of a signal along with all its previous
/// registered actions.
///
/// The remove of the signal handler will influence all components inside the
/// process, therefore you should be cautious when calling this function.
///
/// # Example
/// ```no run
/// let res = ylong_signal::deregister_signal_hook(libc::SIGTERM);
/// assert!(res.is_ok());
/// ```
pub fn deregister_signal_hook(sig_num: c_int) -> io::Result<()> {
    Signal::deregister_hook(sig_num)
}
