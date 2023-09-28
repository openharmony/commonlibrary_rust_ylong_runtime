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

use std::sync::{Arc, Mutex};
use std::time::Duration;

cfg_event!(
    use std::io;
    use crate::executor::worker::{get_current_handle};
);

cfg_time! {
    use std::fmt::Error;
    use std::ptr::NonNull;
    use crate::time::{Clock, TimeDriver, TimeHandle};
    use std::time::Instant;
}
cfg_net! {
    use crate::util::slab::Ref;
    use crate::net::{IoDriver, IoHandle};
    use ylong_io::{Interest, Source};
    use crate::net::ScheduleIO;
}

// Flag used to identify whether to park on condvar.
pub(crate) enum ParkFlag {
    NotPark,
    Park,
    ParkTimeout(Duration),
}

pub(crate) struct Driver {
    #[cfg(feature = "net")]
    io: IoDriver,
    #[cfg(feature = "time")]
    time: Arc<TimeDriver>,
}

pub(crate) struct Handle {
    #[cfg(feature = "net")]
    io: IoHandle,
    #[cfg(feature = "time")]
    time: TimeHandle,
}

impl Driver {
    pub(crate) fn initialize() -> (Arc<Handle>, Arc<Mutex<Driver>>) {
        #[cfg(feature = "net")]
        let (io_handle, io_driver) = IoDriver::initialize();
        #[cfg(feature = "time")]
        let (time_handle, time_driver) = TimeDriver::initialize();
        let handle = Handle {
            #[cfg(feature = "net")]
            io: io_handle,
            #[cfg(feature = "time")]
            time: time_handle,
        };
        let driver = Driver {
            #[cfg(feature = "net")]
            io: io_driver,
            #[cfg(feature = "time")]
            time: time_driver,
        };
        (Arc::new(handle), Arc::new(Mutex::new(driver)))
    }

    pub(crate) fn run(&mut self) -> ParkFlag {
        let _duration: Option<Duration> = None;
        #[cfg(feature = "time")]
        let _duration = self.time.run();
        #[cfg(feature = "net")]
        self.io.drive(_duration).expect("io driver running failed");
        if cfg!(feature = "net") {
            ParkFlag::NotPark
        } else {
            match _duration {
                None => ParkFlag::Park,
                Some(duration) => ParkFlag::ParkTimeout(duration),
            }
        }
    }

    pub(crate) fn run_once(&mut self) {
        #[cfg(feature = "time")]
        self.time.run();
        #[cfg(feature = "net")]
        self.io
            .drive(Some(Duration::from_millis(0)))
            .expect("io driver running failed");
    }
}

impl Handle {
    pub(crate) fn wake(&self) {
        #[cfg(feature = "net")]
        self.io.waker.wake().expect("ylong_io wake failed");
    }

    #[cfg(any(feature = "net", feature = "time"))]
    pub(crate) fn get_handle() -> io::Result<Arc<Handle>> {
        let context = get_current_handle()
            .ok_or_else(|| io::Error::new(io::ErrorKind::Other, "get_current_ctx() fail"))?;
        Ok(context._handle.clone())
    }
}

#[cfg(feature = "net")]
impl Handle {
    pub(crate) fn io_register(
        &self,
        io: &mut impl Source,
        interest: Interest,
    ) -> io::Result<Ref<ScheduleIO>> {
        self.io.register_source(io, interest)
    }

    pub(crate) fn io_deregister(&self, io: &mut impl Source) -> io::Result<()> {
        self.io.deregister_source(io)
    }

    #[cfg(feature = "metrics")]
    pub(crate) fn get_register_count(&self) -> usize {
        self.io.get_register_count()
    }

    #[cfg(feature = "metrics")]
    pub(crate) fn get_ready_count(&self) -> usize {
        self.io.get_ready_count()
    }
}

#[cfg(feature = "time")]
impl Handle {
    pub(crate) fn start_time(&self) -> Instant {
        self.time.start_time()
    }

    pub(crate) fn timer_register(&self, clock_entry: NonNull<Clock>) -> Result<u64, Error> {
        let res = self.time.timer_register(clock_entry);
        self.wake();
        res
    }

    pub(crate) fn timer_cancel(&self, clock_entry: NonNull<Clock>) {
        self.time.timer_cancel(clock_entry);
    }
}
