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

use std::sync::atomic::AtomicUsize;
use std::sync::atomic::Ordering::{Acquire, Release};
use std::sync::Arc;
use std::time::{Duration, Instant};

use ylong_runtime::signal::{signal, SignalKind};

fn print_time(duration: Duration) {
    let hours = duration.as_secs() / 3600;
    let minutes = duration.as_secs() % 3600 / 60;
    let seconds = duration.as_secs() % 60;
    let formatted_time = format!("{:02}:{:02}:{:02}", hours, minutes, seconds);
    println!("duration : {:?}", formatted_time);
}

fn run_multi_thread_signal() {
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

fn main() {
    let start = Instant::now();
    for _ in 0..100000 {
        run_multi_thread_signal();
    }
    let end = Instant::now();
    let duration = end - start;
    print_time(duration);
}
