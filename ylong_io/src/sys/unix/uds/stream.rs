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

use std::fmt::Debug;
use std::io::{self, IoSlice, IoSliceMut, Read, Write};
use std::os::unix::io::{AsRawFd, FromRawFd, IntoRawFd, RawFd};
use std::os::unix::net;
use std::path::Path;

use crate::source::Fd;
use crate::{Interest, Selector, Source, Token};

/// A non-blocking UDS Stream between two local sockets.
pub struct UnixStream {
    pub(crate) inner: net::UnixStream,
}

impl UnixStream {
    /// Connects to the specific socket.
    ///
    /// # Examples
    /// ```no_run
    /// use ylong_io::UnixStream;
    ///
    /// let socket = match UnixStream::connect("/tmp/sock") {
    ///     Ok(sock) => sock,
    ///     Err(err) => {
    ///         println!("connect fail: {err:?}");
    ///     }
    /// };
    /// ```
    pub fn connect<P: AsRef<Path>>(path: P) -> io::Result<UnixStream> {
        super::socket::connect(path.as_ref()).map(UnixStream::from_std)
    }

    /// Creates a new `UnixStream` from a standard `net::UnixStream`
    ///
    /// # Examples
    /// ```no_run
    /// use std::os::unix::net::UnixStream;
    ///
    /// use ylong_io::UnixStream as Ylong_UnixStream;
    ///
    /// let stream = match UnixStream::bind("/path/to/the/socket") {
    ///     Ok(stream) => stream,
    ///     Err(e) => {
    ///         println!("Couldn't bind: {e:?}");
    ///     }
    /// };
    /// let _stream = Ylong_UnixStream::from_std(stream);
    /// ```
    pub fn from_std(stream: net::UnixStream) -> UnixStream {
        UnixStream { inner: stream }
    }

    /// Creates an unnamed pair of connected sockets.
    /// Returns two `UnixStream`s which are connected to each other.
    ///
    /// # Examples
    /// ```no_run
    /// use ylong_io::UnixStream;
    ///
    /// let (stream1, stream2) = match UnixStream::pair() {
    ///     Ok((stream1, stream2)) => (stream1, stream2),
    ///     Err(err) => {
    ///         println!("Couldn't create a pair of sockets: {err:?}");
    ///     }
    /// };
    /// ```
    pub fn pair() -> io::Result<(UnixStream, UnixStream)> {
        super::socket::stream_pair().map(|(stream1, stream2)| {
            (UnixStream::from_std(stream1), UnixStream::from_std(stream2))
        })
    }

    /// Creates a new independently owned handle to the underlying socket.
    ///
    /// # Examples
    /// ```no_run
    /// use ylong_io::UnixStream;
    ///
    /// fn test() -> std::io::Result<()> {
    ///     let socket = UnixStream::connect("/tmp/sock")?;
    ///     let sock_copy = socket.try_clone().expect("try_clone() fail");
    ///     Ok(())
    /// }
    /// ```
    pub fn try_clone(&self) -> io::Result<UnixStream> {
        Ok(Self::from_std(self.inner.try_clone()?))
    }

    /// Returns the local socket address of this UnixStream.
    ///
    /// # Examples
    /// ```no_run
    /// use ylong_io::UnixStream;
    ///
    /// fn test() -> std::io::Result<()> {
    ///     let socket = UnixStream::connect("/tmp/sock")?;
    ///     let addr = socket.local_addr().expect("get local_addr() fail");
    ///     Ok(())
    /// }
    /// ```
    pub fn local_addr(&self) -> io::Result<net::SocketAddr> {
        self.inner.local_addr()
    }

    /// Returns the local socket address of this UnixStream's peer.
    ///
    /// # Examples
    /// ```no_run
    /// use ylong_io::UnixStream;
    ///
    /// fn test() -> std::io::Result<()> {
    ///     let socket = UnixStream::connect("/tmp/sock")?;
    ///     let addr = socket.peer_addr().expect("get peer_addr() fail");
    ///     Ok(())
    /// }
    /// ```
    pub fn peer_addr(&self) -> io::Result<net::SocketAddr> {
        self.inner.peer_addr()
    }

    /// Returns the error of the `SO_ERROR` option.
    ///
    /// # Examples
    /// ```no_run
    /// use ylong_io::UnixStream;
    ///
    /// fn test() -> std::io::Result<()> {
    ///     let socket = UnixStream::connect("/tmp/sock")?;
    ///     if let Ok(Some(err)) = socket.take_error() {
    ///         println!("get error: {err:?}");
    ///     }
    ///     Ok(())
    /// }
    /// ```
    pub fn take_error(&self) -> io::Result<Option<io::Error>> {
        self.inner.take_error()
    }

    /// Shuts down this connection.
    ///
    /// # Examples
    /// ```no_run
    /// use std::net::Shutdown;
    ///
    /// use ylong_io::UnixStream;
    ///
    /// fn test() -> std::io::Result<()> {
    ///     let socket = UnixStream::connect("/tmp/sock")?;
    ///     socket.shutdown(Shutdown::Both).expect("shutdown() failed");
    ///     Ok(())
    /// }
    /// ```
    pub fn shutdown(&self, how: std::net::Shutdown) -> io::Result<()> {
        self.inner.shutdown(how)
    }
}

impl Read for UnixStream {
    fn read(&mut self, buf: &mut [u8]) -> io::Result<usize> {
        self.inner.read(buf)
    }

    fn read_vectored(&mut self, bufs: &mut [IoSliceMut<'_>]) -> io::Result<usize> {
        self.inner.read_vectored(bufs)
    }
}

impl Write for UnixStream {
    fn write(&mut self, buf: &[u8]) -> io::Result<usize> {
        self.inner.write(buf)
    }

    fn write_vectored(&mut self, bufs: &[IoSlice<'_>]) -> io::Result<usize> {
        self.inner.write_vectored(bufs)
    }

    fn flush(&mut self) -> io::Result<()> {
        self.inner.flush()
    }
}
impl Read for &UnixStream {
    fn read(&mut self, buf: &mut [u8]) -> io::Result<usize> {
        let mut inner = &self.inner;
        inner.read(buf)
    }

    fn read_vectored(&mut self, bufs: &mut [IoSliceMut<'_>]) -> io::Result<usize> {
        let mut inner = &self.inner;
        inner.read_vectored(bufs)
    }
}

impl Write for &UnixStream {
    fn write(&mut self, buf: &[u8]) -> io::Result<usize> {
        let mut inner = &self.inner;
        inner.write(buf)
    }

    fn write_vectored(&mut self, bufs: &[IoSlice<'_>]) -> io::Result<usize> {
        let mut inner = &self.inner;
        inner.write_vectored(bufs)
    }

    fn flush(&mut self) -> io::Result<()> {
        let mut inner = &self.inner;
        inner.flush()
    }
}

impl Debug for UnixStream {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        self.inner.fmt(f)
    }
}

impl Source for UnixStream {
    fn register(
        &mut self,
        selector: &Selector,
        token: Token,
        interests: Interest,
    ) -> io::Result<()> {
        selector.register(self.inner.as_raw_fd(), token, interests)
    }

    fn reregister(
        &mut self,
        selector: &Selector,
        token: Token,
        interests: Interest,
    ) -> io::Result<()> {
        selector.reregister(self.inner.as_raw_fd(), token, interests)
    }

    fn deregister(&mut self, selector: &Selector) -> io::Result<()> {
        selector.deregister(self.inner.as_raw_fd())
    }

    fn as_raw_fd(&self) -> Fd {
        self.inner.as_raw_fd()
    }
}

impl IntoRawFd for UnixStream {
    fn into_raw_fd(self) -> RawFd {
        self.inner.into_raw_fd()
    }
}

impl FromRawFd for UnixStream {
    unsafe fn from_raw_fd(fd: RawFd) -> UnixStream {
        UnixStream::from_std(FromRawFd::from_raw_fd(fd))
    }
}
