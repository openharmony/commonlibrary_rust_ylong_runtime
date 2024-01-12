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

use std::future::Future;
use std::pin::Pin;
use std::task::{Context, Poll};

pub(crate) async fn try_join3<F1, F2, F3, R1, R2, R3, E>(
    fut1: F1,
    fut2: F2,
    fut3: F3,
) -> Result<(R1, R2, R3), E>
where
    F1: Future<Output = Result<R1, E>>,
    F2: Future<Output = Result<R2, E>>,
    F3: Future<Output = Result<R3, E>>,
{
    let mut fut1 = future_done(fut1);
    let mut fut2 = future_done(fut2);
    let mut fut3 = future_done(fut3);

    crate::futures::poll_fn(move |cx| {
        let mut is_pending = false;

        let mut fut1 = unsafe { Pin::new_unchecked(&mut fut1) };
        if fut1.as_mut().poll(cx).is_pending() {
            is_pending = true;
        } else if fut1.as_mut().output().unwrap().is_err() {
            return Poll::Ready(Err(fut1.take_output().unwrap().err().unwrap()));
        }

        let mut fut2 = unsafe { Pin::new_unchecked(&mut fut2) };
        if fut2.as_mut().poll(cx).is_pending() {
            is_pending = true;
        } else if fut2.as_mut().output().unwrap().is_err() {
            return Poll::Ready(Err(fut2.take_output().unwrap().err().unwrap()));
        }

        let mut fut3 = unsafe { Pin::new_unchecked(&mut fut3) };
        if fut3.as_mut().poll(cx).is_pending() {
            is_pending = true;
        } else if fut3.as_mut().output().unwrap().is_err() {
            return Poll::Ready(Err(fut3.take_output().unwrap().err().unwrap()));
        }

        if is_pending {
            Poll::Pending
        } else {
            // All fut have ended in a Ready state and will only take_output() here.
            Poll::Ready(Ok((
                fut1.take_output().unwrap().ok().unwrap(),
                fut2.take_output().unwrap().ok().unwrap(),
                fut3.take_output().unwrap().ok().unwrap(),
            )))
        }
    })
    .await
}

pub(crate) enum FutureDone<F: Future> {
    Pending(F),
    Ready(F::Output),
    None,
}

pub(crate) fn future_done<F: Future>(future: F) -> FutureDone<F> {
    FutureDone::Pending(future)
}

impl<F: Future + Unpin> Unpin for FutureDone<F> {}

impl<F: Future> FutureDone<F> {
    pub(crate) fn output(self: Pin<&mut Self>) -> Option<&mut F::Output> {
        // Safety: inner data never move.
        unsafe {
            match self.get_unchecked_mut() {
                FutureDone::Ready(output) => Some(output),
                _ => None,
            }
        }
    }

    pub(crate) fn take_output(self: Pin<&mut Self>) -> Option<F::Output> {
        // Safety: inner data never move.
        unsafe {
            let inner = self.get_unchecked_mut();
            match inner {
                FutureDone::Ready(_) => {}
                _ => return None,
            }
            if let FutureDone::Ready(output) = std::mem::replace(inner, FutureDone::None) {
                return Some(output);
            }
            unreachable!()
        }
    }
}

impl<F: Future> Future for FutureDone<F> {
    type Output = ();

    fn poll(mut self: Pin<&mut Self>, cx: &mut Context<'_>) -> Poll<Self::Output> {
        // Safety: inner data never move.
        unsafe {
            match self.as_mut().get_unchecked_mut() {
                FutureDone::Pending(fut) => match Pin::new_unchecked(fut).poll(cx) {
                    Poll::Ready(res) => {
                        self.set(FutureDone::Ready(res));
                        Poll::Ready(())
                    }
                    Poll::Pending => Poll::Pending,
                },
                FutureDone::Ready(_) => Poll::Ready(()),
                FutureDone::None => panic!("FutureDone output has gone"),
            }
        }
    }
}

#[cfg(test)]
mod test {
    use std::future::Future;
    use std::pin::Pin;
    use std::task::Poll;

    use crate::process::try_join3::{future_done, try_join3};
    /// UT test cases for `try_join()`.
    ///
    /// # Brief
    /// 1. Create 3 future with 1 return err.
    /// 2. try_join() return error.
    #[test]
    fn ut_try_join_error_test() {
        async fn ok() -> Result<(), &'static str> {
            Ok(())
        }
        async fn err() -> Result<(), &'static str> {
            Err("test")
        }
        let handle = crate::spawn(async {
            let fut1 = err();
            let fut2 = ok();
            let fut3 = ok();
            let res = try_join3(fut1, fut2, fut3).await;
            assert!(res.is_err());

            let fut1 = ok();
            let fut2 = err();
            let fut3 = ok();
            let res = try_join3(fut1, fut2, fut3).await;
            assert!(res.is_err());

            let fut1 = ok();
            let fut2 = ok();
            let fut3 = err();
            let res = try_join3(fut1, fut2, fut3).await;
            assert!(res.is_err());
        });
        crate::block_on(handle).unwrap();
    }

    /// UT test cases for `FutureDone`.
    ///
    /// # Brief
    /// 1. Create FutureDone with future_done().
    /// 2. Check the result.
    #[test]
    fn ut_future_done_test() {
        let handle = crate::spawn(async {
            let fut = async { 1 };
            let mut fut = future_done(fut);

            crate::futures::poll_fn(move |cx| {
                let mut fut = unsafe { Pin::new_unchecked(&mut fut) };
                if fut.as_mut().poll(cx).is_pending() {
                    Poll::Pending
                } else {
                    let output = fut.as_mut().take_output();
                    assert!(output.is_some());
                    assert!(fut.as_mut().take_output().is_none());
                    assert!(fut.output().is_none());
                    Poll::Ready(output.unwrap())
                }
            })
            .await;
        });
        crate::block_on(handle).unwrap();
    }
}
