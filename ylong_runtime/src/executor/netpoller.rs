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

#[cfg(feature = "time")]
use crate::time::Driver as TimerDriver;
use std::cell::RefCell;
use std::sync::Arc;
use std::thread;

#[cfg(any(not(feature = "ffrt"), all(feature = "net", feature = "ffrt")))]
const NET_POLL_INTERVAL_TIME: std::time::Duration = std::time::Duration::from_millis(10);

/// Net poller thread creation and management
#[derive(Clone)]
pub(crate) struct NetLooper {
    inner: Arc<Inner>,
}

unsafe impl Send for NetLooper {}
unsafe impl Sync for NetLooper {}

struct Inner {
    join_handle: RefCell<Option<thread::JoinHandle<()>>>,
}

impl NetLooper {
    pub(crate) fn new() -> Self {
        NetLooper {
            inner: Arc::new(Inner {
                join_handle: RefCell::new(None),
            }),
        }
    }

    pub(crate) fn create_net_poller_thread(&self) {
        // todo: now we use the default thread stack size, could be smaller
        let builder = thread::Builder::new().name("yl_net_poller".to_string());
        let netpoller_handle = self.clone();

        let result = builder.spawn(move || netpoller_handle.run());
        match result {
            Ok(join_handle) => {
                *self.inner.join_handle.borrow_mut() = Some(join_handle);
            }
            Err(e) => panic!("os cannot spawn the monitor thread: {}", e),
        }
    }

    fn run(&self) {
        loop {
            // run time driver
            #[cfg(feature = "time")]
            TimerDriver::get_ref().run();

            thread::sleep(NET_POLL_INTERVAL_TIME);
        }
    }
}
