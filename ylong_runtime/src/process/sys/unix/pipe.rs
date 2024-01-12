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

use std::fs::File;
use std::io;
use std::io::{IoSlice, Write};
use std::os::unix::io::{AsFd, AsRawFd, BorrowedFd, FromRawFd, IntoRawFd, RawFd};

use ylong_io::sys::SourceFd;
use ylong_io::{Interest, Selector, Source, Token};

#[derive(Debug)]
pub(crate) struct Pipe {
    pub(crate) fd: File,
}

impl<T: IntoRawFd> From<T> for Pipe {
    fn from(value: T) -> Self {
        let fd = unsafe { File::from_raw_fd(value.into_raw_fd()) };
        Self { fd }
    }
}

impl<'a> io::Read for &'a Pipe {
    fn read(&mut self, buf: &mut [u8]) -> io::Result<usize> {
        (&self.fd).read(buf)
    }
}

impl<'a> Write for &'a Pipe {
    fn write(&mut self, buf: &[u8]) -> io::Result<usize> {
        (&self.fd).write(buf)
    }

    fn write_vectored(&mut self, bufs: &[IoSlice<'_>]) -> io::Result<usize> {
        (&self.fd).write_vectored(bufs)
    }

    fn flush(&mut self) -> io::Result<()> {
        (&self.fd).flush()
    }
}

impl AsRawFd for Pipe {
    fn as_raw_fd(&self) -> RawFd {
        self.fd.as_raw_fd()
    }
}

impl AsFd for Pipe {
    fn as_fd(&self) -> BorrowedFd<'_> {
        unsafe { BorrowedFd::borrow_raw(self.fd.as_raw_fd()) }
    }
}

impl Source for Pipe {
    fn register(
        &mut self,
        selector: &Selector,
        token: Token,
        interests: Interest,
    ) -> io::Result<()> {
        SourceFd(&AsRawFd::as_raw_fd(self)).register(selector, token, interests)
    }

    fn reregister(
        &mut self,
        selector: &Selector,
        token: Token,
        interests: Interest,
    ) -> io::Result<()> {
        SourceFd(&AsRawFd::as_raw_fd(self)).reregister(selector, token, interests)
    }

    fn deregister(&mut self, selector: &Selector) -> io::Result<()> {
        SourceFd(&AsRawFd::as_raw_fd(self)).deregister(selector)
    }

    fn as_raw_fd(&self) -> ylong_io::Fd {
        self.fd.as_raw_fd()
    }
}
