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

use std::cmp;
use std::convert::TryInto;
use std::future::Future;
use std::pin::Pin;
use std::ptr::NonNull;
use std::task::{Context, Poll};
use std::time::{Duration, Instant};

cfg_not_ffrt!(
    use std::sync::Arc;
    use crate::executor::driver::Handle;
);

use crate::time::Clock;
#[cfg(feature = "ffrt")]
use crate::time::TimeDriver;

const TEN_YEARS: Duration = Duration::from_secs(86400 * 365 * 10);

/// Waits until 'instant' has reached.
///
/// # Panic
/// Calling this method outside of a Ylong Runtime could cause panic, for
/// example, outside of an async closure that is passed to ylong_runtime::spawn
/// or ylong_runtime::block_on. The async wrapping is necessary since it makes
/// the function become lazy in order to get successfully executed on the
/// runtime.
pub fn sleep_until(instant: Instant) -> Sleep {
    Sleep::new_timeout(instant)
}

/// Waits until 'duration' has elapsed.
///
/// # Panic
/// Calling this method outside of a Ylong Runtime could cause panic, for
/// example, outside of an async closure that is passed to ylong_runtime::spawn
/// or ylong_runtime::block_on. The async wrapping is necessary since it makes
/// the function become lazy in order to get successfully executed on the
/// runtime.
pub fn sleep(duration: Duration) -> Sleep {
    // If the time reaches the maximum value,
    // then set the default timing time to 10 years.
    match Instant::now().checked_add(duration) {
        Some(deadline) => Sleep::new_timeout(deadline),
        None => Sleep::new_timeout(Instant::now() + TEN_YEARS),
    }
}

/// A structure that implements Future. returned by func [`sleep`].
///
/// [`sleep`]: sleep
/// # Examples
///
/// ```
/// use std::time::Duration;
///
/// use ylong_runtime::time::sleep;
///
/// async fn sleep_test() {
///     let sleep = sleep(Duration::from_secs(2)).await;
///     println!("2 secs have elapsed");
/// }
/// ```
pub struct Sleep {
    // During the polling of this structure, no repeated insertion.
    need_insert: bool,

    // The time at which the structure should end.
    deadline: Instant,

    // Corresponding Timer structure.
    timer: Clock,

    #[cfg(not(feature = "ffrt"))]
    handle: Arc<Handle>,
}

impl Sleep {
    // Creates a Sleep structure based on the given deadline.
    fn new_timeout(deadline: Instant) -> Self {
        #[cfg(not(feature = "ffrt"))]
        let handle = Handle::get_handle().expect("sleep new out of worker ctx");

        #[cfg(feature = "ffrt")]
        let handle = TimeDriver::get_ref();

        let start_time = handle.start_time();
        let deadline = cmp::max(deadline, start_time);

        let timer = Clock::new();
        Self {
            need_insert: true,
            deadline,
            timer,
            #[cfg(not(feature = "ffrt"))]
            handle,
        }
    }

    // Returns the deadline of the Sleep
    pub(crate) fn deadline(&self) -> Instant {
        self.deadline
    }

    // Resets the deadline of the Sleep
    pub(crate) fn reset(&mut self, new_deadline: Instant) {
        self.need_insert = true;
        self.deadline = new_deadline;
        self.timer.set_result(false);
    }

    // Cancels the Sleep
    fn cancel(&mut self) {
        #[cfg(not(feature = "ffrt"))]
        let driver = &self.handle;
        #[cfg(feature = "ffrt")]
        let driver = TimeDriver::get_ref();
        driver.timer_cancel(NonNull::from(&self.timer));
    }
}

impl Future for Sleep {
    type Output = ();

    fn poll(self: Pin<&mut Self>, cx: &mut Context<'_>) -> Poll<Self::Output> {
        let this = self.get_mut();

        #[cfg(not(feature = "ffrt"))]
        let driver = &this.handle;
        #[cfg(feature = "ffrt")]
        let driver = TimeDriver::get_ref();
        if this.need_insert {
            let ms = this
                .deadline
                .checked_duration_since(driver.start_time())
                .unwrap()
                .as_millis()
                .try_into()
                .unwrap_or(u64::MAX);
            this.timer.set_expiration(ms);
            this.timer.set_waker(cx.waker().clone());

            match driver.timer_register(NonNull::from(&this.timer)) {
                Ok(_) => this.need_insert = false,
                Err(_) => {
                    // Even if the insertion fails, there is no need to insert again here,
                    // it is a timeout clock and needs to be triggered immediately at the next poll.
                    this.need_insert = false;
                    this.timer.set_result(true);
                }
            }
        }

        if this.timer.result() {
            Poll::Ready(())
        } else {
            Poll::Pending
        }
    }
}

impl Drop for Sleep {
    fn drop(&mut self) {
        // For some uses, for example, Timeout,
        // `Sleep` enters the `Pending` state first and inserts the `TimerHandle` into
        // the `DRIVER`, the future of timeout returns `Ready` in advance of the
        // next polling, as a result, the `TimerHandle` pointer in the `DRIVER`
        // is invalid. need to cancel the `TimerHandle` operation during `Sleep`
        // drop.
        self.cancel()
    }
}

#[cfg(test)]
mod test {
    use std::time::Duration;

    use crate::time::sleep;
    use crate::{block_on, spawn};

    /// UT test cases for new_sleep
    ///
    /// # Brief
    /// 1. Uses sleep to create a Sleep Struct.
    /// 2. Uses block_on to test different sleep duration.
    #[test]
    fn new_timer_sleep() {
        block_on(async move {
            sleep(Duration::new(0, 20_000_000)).await;
            sleep(Duration::new(0, 20_000_000)).await;
            sleep(Duration::new(0, 20_000_000)).await;
        });

        let handle_one = spawn(async {
            sleep(Duration::new(0, 20_000_000)).await;
        });
        let handle_two = spawn(async {
            sleep(Duration::new(0, 20_000_000)).await;
        });
        let handle_three = spawn(async {
            sleep(Duration::new(0, 20_000_000)).await;
        });
        block_on(handle_one).unwrap();
        block_on(handle_two).unwrap();
        block_on(handle_three).unwrap();
    }
}
