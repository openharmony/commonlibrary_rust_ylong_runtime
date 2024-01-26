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
#![cfg(feature = "signal")]

use std::sync::atomic::AtomicUsize;
use std::sync::atomic::Ordering::{Acquire, Release};
use std::sync::Arc;

use ylong_runtime::futures::poll_fn;
use ylong_runtime::signal::{signal, SignalKind};

/// SDV cases for signal `recv()`.
///
/// # Brief
/// 1. Generate a counter to ensure that notifications are received every time
///    listening.
/// 2. Spawns a task to loop and listen to a signal.
/// 3. Send notification signals in a loop until all waiting is completed.
#[test]
fn signal_recv_test() {
    let handle = ylong_runtime::spawn(async move {
        let mut stream = signal(SignalKind::alarm()).unwrap();
        for _ in 0..10 {
            unsafe { libc::raise(libc::SIGALRM) };
            stream.recv().await;
        }
    });
    let _ = ylong_runtime::block_on(handle);
}

/// SDV cases for signal `recv()` in multi thread.
///
/// # Brief
/// 1. Generate a counter to confirm that all signals are waiting.
/// 2. Spawns some tasks to listen to a signal.
/// 3. Send a notification signal when all signals are waiting.
#[test]
fn signal_recv_multi_thread_test() {
    let num = Arc::new(AtomicUsize::new(0));
    let mut handles = Vec::new();
    for _ in 0..10 {
        let num_clone = num.clone();
        handles.push(ylong_runtime::spawn(async move {
            let mut stream = signal(SignalKind::child()).unwrap();
            num_clone.fetch_add(1, Release);
            stream.recv().await;
        }));
    }
    while num.load(Acquire) < 10 {}
    unsafe { libc::raise(libc::SIGCHLD) };
    for handle in handles {
        let _ = ylong_runtime::block_on(handle);
    }
}

/// SDV cases for signal `poll_recv()`.
///
/// # Brief
/// 1. Generate a counter to ensure that notifications are received every time
///    listening.
/// 2. Spawns a task to loop and listen to a signal.
/// 3. Send notification signals in a loop until all waiting is completed.
#[test]
fn signal_poll_recv_test() {
    let handle = ylong_runtime::spawn(async move {
        let mut stream = signal(SignalKind::hangup()).unwrap();
        for _ in 0..10 {
            unsafe { libc::raise(libc::SIGHUP) };
            poll_fn(|cx| stream.poll_recv(cx)).await;
        }
    });
    let _ = ylong_runtime::block_on(handle);
}

/// SDV cases for signal `poll_recv()` in multi thread.
///
/// # Brief
/// 1. Generate a counter to confirm that all signals are waiting.
/// 2. Spawns some tasks to listen to a signal.
/// 3. Send a notification signal when all signals are waiting.
#[test]
fn signal_poll_recv_multi_thread_test() {
    let num = Arc::new(AtomicUsize::new(0));
    let mut handles = Vec::new();
    for _ in 0..10 {
        let num_clone = num.clone();
        handles.push(ylong_runtime::spawn(async move {
            let mut stream = signal(SignalKind::io()).unwrap();
            num_clone.fetch_add(1, Release);
            stream.recv().await;
        }));
    }
    while num.load(Acquire) < 10 {}
    unsafe { libc::raise(libc::SIGIO) };
    for handle in handles {
        let _ = ylong_runtime::block_on(handle);
    }
}
