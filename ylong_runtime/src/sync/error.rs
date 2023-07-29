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

//! Error of sync

use std::error::Error;
use std::fmt::{Debug, Display, Formatter};

/// Error returned by `send`
#[derive(Debug, Eq, PartialEq)]
pub struct SendError<T>(pub T);

impl<T> Display for SendError<T> {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        write!(f, "channel is closed")
    }
}

impl<T: Debug> Error for SendError<T> {}

/// Error returned by `recv`
#[derive(Debug, Eq, PartialEq)]
pub struct RecvError;

impl Display for RecvError {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        write!(f, "channel is closed")
    }
}

impl Error for RecvError {}

/// Error returned by `try_send`.
#[derive(Debug, Eq, PartialEq)]
pub enum TrySendError<T> {
    /// The channel is full now.
    Full(T),
    /// The receiver of channel was closed or dropped.
    Closed(T),
}

impl<T> Display for TrySendError<T> {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        match self {
            TrySendError::Full(_) => write!(f, "channel is full"),
            TrySendError::Closed(_) => write!(f, "channel is closed"),
        }
    }
}

impl<T: Debug> Error for TrySendError<T> {}

/// Error returned by `try_recv`.
#[derive(Debug, Eq, PartialEq, Clone)]
pub enum TryRecvError {
    /// sender has not sent a value yet.
    Empty,
    /// sender was dropped.
    Closed,
}

impl Display for TryRecvError {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        match self {
            TryRecvError::Empty => write!(f, "channel is empty"),
            TryRecvError::Closed => write!(f, "channel is closed"),
        }
    }
}

impl Error for TryRecvError {}

cfg_time! {

    /// Error returned by `send_timeout`
    #[derive(Debug,Eq,PartialEq)]
    pub enum SendTimeoutError<T> {
        /// The receiver of channel was closed or dropped.
        Closed(T),
        /// Sending timeout.
        TimeOut(T),
    }

    impl<T> Display for SendTimeoutError<T> {
        fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
            match self {
                SendTimeoutError::Closed(_) => write!(f, "channel is closed"),
                SendTimeoutError::TimeOut(_) => write!(f, "channel sending timeout"),
            }
        }
    }
    impl<T: Debug> Error for SendTimeoutError<T> {}

    /// Error returned by `recv_timeout`.
    #[derive(Debug, Eq, PartialEq)]
    pub enum RecvTimeoutError {
        /// sender was dropped.
        Closed,
        /// Receiving timeout.
        Timeout,
    }

    impl Display for RecvTimeoutError {
        fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
            match self {
                RecvTimeoutError::Closed => write!(f, "channel is closed"),
                RecvTimeoutError::Timeout => write!(f, "channel receiving timeout"),
            }
        }
    }

    impl Error for RecvTimeoutError {}
}
