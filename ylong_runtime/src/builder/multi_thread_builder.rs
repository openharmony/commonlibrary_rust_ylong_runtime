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
use std::sync::Mutex;

cfg_ffrt!(
    use ylong_ffrt::{ffrt_set_cpu_worker_max_num, Qos};
    use std::collections::HashMap;
    use libc::c_uint;
);

use crate::builder::common_builder::impl_common;
use crate::builder::CommonBuilder;
#[cfg(feature = "multi_instance_runtime")]
use crate::executor::{AsyncHandle, Runtime};

pub(crate) static GLOBAL_BUILDER: Mutex<Option<MultiThreadBuilder>> = Mutex::new(None);

/// Runtime builder that configures a multi-threaded runtime, or the global
/// runtime.
pub struct MultiThreadBuilder {
    pub(crate) common: CommonBuilder,

    #[cfg(not(feature = "ffrt"))]
    /// Maximum thread number for core thread pool
    pub(crate) core_thread_size: Option<usize>,

    #[cfg(feature = "ffrt")]
    /// Thread number for each qos
    pub(crate) thread_num_by_qos: HashMap<Qos, u32>,
}

impl MultiThreadBuilder {
    pub(crate) fn new() -> Self {
        MultiThreadBuilder {
            common: CommonBuilder::new(),
            #[cfg(not(feature = "ffrt"))]
            core_thread_size: None,
            #[cfg(feature = "ffrt")]
            thread_num_by_qos: HashMap::new(),
        }
    }

    /// Configures the global runtime.
    ///
    /// # Error
    /// If the global runtime is already running or this method has been called
    /// before, then it will return an `AlreadyExists` error.
    pub fn build_global(self) -> io::Result<()> {
        let mut builder = GLOBAL_BUILDER.lock().unwrap();
        if builder.is_some() {
            return Err(io::ErrorKind::AlreadyExists.into());
        }

        #[cfg(feature = "ffrt")]
        {
            for (qos, worker_num) in self.thread_num_by_qos.iter() {
                unsafe {
                    ffrt_set_cpu_worker_max_num(*qos, worker_num.clone() as c_uint);
                }
            }
        }

        *builder = Some(self);
        Ok(())
    }
}

#[cfg(feature = "ffrt")]
impl MultiThreadBuilder {
    /// Sets the maximum worker number for a specific qos group.
    ///
    /// If a worker number has already been set for a qos, calling the method
    /// with the same qos will overwrite the old value.
    ///
    /// # Error
    /// The accepted worker number range for each qos is [1, 20]. If 0 is passed
    /// in, then the maximum worker number will be set to 1. If a number
    /// greater than 20 is passed in, then the maximum worker number will be
    /// set to 20.
    pub fn max_worker_num_by_qos(mut self, qos: Qos, num: u32) -> Self {
        let worker = match num {
            0 => 1,
            n if n > 20 => 20,
            n => n,
        };
        self.thread_num_by_qos.insert(qos, worker);
        self
    }
}

#[cfg(not(feature = "ffrt"))]
impl MultiThreadBuilder {
    /// Initializes the runtime and returns its instance.
    #[cfg(feature = "multi_instance_runtime")]
    pub fn build(&mut self) -> io::Result<Runtime> {
        use crate::builder::initialize_async_spawner;
        let async_spawner = initialize_async_spawner(self)?;

        Ok(Runtime {
            async_spawner: AsyncHandle::MultiThread(async_spawner),
        })
    }

    /// Sets the number of core worker threads.
    ///
    ///
    /// The boundary of thread number is 1-64:
    /// If sets a number smaller than 1, then thread number would be set to 1.
    /// If sets a number larger than 64, then thread number would be set to 64.
    /// The default value is the number of cores of the cpu.
    ///
    /// # Examples
    /// ```
    /// use crate::ylong_runtime::builder::RuntimeBuilder;
    ///
    /// let runtime = RuntimeBuilder::new_multi_thread().worker_num(8);
    /// ```
    pub fn worker_num(mut self, core_pool_size: usize) -> Self {
        if core_pool_size < 1 {
            self.core_thread_size = Some(1);
        } else if core_pool_size > 64 {
            self.core_thread_size = Some(64);
        } else {
            self.core_thread_size = Some(core_pool_size);
        }
        self
    }
}

impl_common!(MultiThreadBuilder);

#[cfg(feature = "full")]
#[cfg(test)]
mod test {
    use crate::builder::RuntimeBuilder;
    use crate::executor::{global_default_async, AsyncHandle};

    /// UT test cases for blocking on a time sleep without initializing the
    /// runtime.
    ///
    /// # Brief
    /// 1. Configure the global runtime to make it have six core threads
    /// 2. Get the global runtime
    /// 3. Check the core thread number of the runtime
    /// 4. Call build_global once more
    /// 5. Check the error
    #[test]
    fn ut_build_global() {
        let ret = RuntimeBuilder::new_multi_thread()
            .worker_num(6)
            .max_blocking_pool_size(3)
            .build_global();
        assert!(ret.is_ok());

        let async_pool = global_default_async();
        match &async_pool.async_spawner {
            AsyncHandle::CurrentThread(_) => unreachable!(),
            AsyncHandle::MultiThread(x) => {
                assert_eq!(x.inner.total, 6);
            }
        }

        let ret = RuntimeBuilder::new_multi_thread()
            .worker_num(2)
            .max_blocking_pool_size(3)
            .build_global();
        assert!(ret.is_err());
    }
}

#[cfg(feature = "ffrt")]
#[cfg(test)]
mod ffrt_test {
    use ylong_ffrt::Qos::{Default, UserInteractive};

    use crate::builder::MultiThreadBuilder;

    /// UT test cases for max_worker_num_by_qos
    /// runtime.
    ///
    /// # Brief
    /// 1. Sets UserInteractive qos group to have 0 maximum worker number.
    /// 2. Checks if the actual value is 1
    /// 3. Sets UserInteractive qos group to have 21 maximum worker number.
    /// 4. Checks if the actual value is 20
    /// 5. Set Default qos group to have 8 maximum worker number.
    /// 6. Checks if the actual value is 8.
    /// 7. Calls build_global on the builder, check if the return value is Ok
    #[test]
    fn ut_set_max_worker() {
        let builder = MultiThreadBuilder::new();
        let builder = builder.max_worker_num_by_qos(UserInteractive, 0);
        let num = builder.thread_num_by_qos.get(&UserInteractive).unwrap();
        assert_eq!(*num, 1);

        let builder = builder.max_worker_num_by_qos(UserInteractive, 21);
        let num = builder.thread_num_by_qos.get(&UserInteractive).unwrap();
        assert_eq!(*num, 20);

        let builder = MultiThreadBuilder::new().max_worker_num_by_qos(Default, 8);
        let num = builder.thread_num_by_qos.get(&Default).unwrap();
        assert_eq!(*num, 8);
    }
}
