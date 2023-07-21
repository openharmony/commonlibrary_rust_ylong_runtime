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
use std::mem::{self, MaybeUninit};
use std::os::unix::io::{AsRawFd, FromRawFd, IntoRawFd, RawFd};
use std::os::unix::net;
use std::path::Path;

use crate::source::Fd;
use crate::sys::unix::{SocketAddr, UnixStream};
use crate::{Interest, Selector, Source, Token};

/// A UDS server.
pub struct UnixListener {
    pub(crate) inner: net::UnixListener,
}

impl UnixListener {
    /// Creates a new `UnixListener` bound to the specified socket.
    ///
    /// # Examples
    /// ```no_run
    /// use ylong_io::UnixListener;
    ///
    /// let listener = match UnixListener::bind("/socket/path") {
    ///     Ok(sock) => sock,
    ///     Err(e) => {
    ///         println!("connect fail: {e:?}");
    ///     }
    /// };
    /// ```
    pub fn bind<P: AsRef<Path>>(path: P) -> io::Result<UnixListener> {
        super::socket::bind(path.as_ref()).map(UnixListener::from_std)
    }

    /// Waits a new incoming connection for this listener.
    ///
    /// # Examples
    /// ```no_run
    /// use ylong_io::UnixListener;
    ///
    /// fn test() -> std::io::Result<()> {
    ///     let listener = UnixListener::bind("/socket/path")?;
    ///
    ///     match listener.accept() {
    ///         Ok((socket, addr)) => println!("accept success: {addr:?}"),
    ///         Err(err) => println!("accept failed: {err:?}"),
    ///     }
    ///     Ok(())
    /// }
    /// ```
    pub fn accept(&self) -> io::Result<(UnixStream, SocketAddr)> {
        let mut addr = unsafe { MaybeUninit::<libc::sockaddr_un>::zeroed().assume_init() };

        addr.sun_family = libc::AF_UNIX as libc::sa_family_t;
        let mut socklen = mem::size_of_val(&addr) as libc::socklen_t;

        let flags = libc::SOCK_NONBLOCK | libc::SOCK_CLOEXEC;
        let socket = syscall!(accept4(
            self.inner.as_raw_fd(),
            &mut addr as *mut libc::sockaddr_un as *mut libc::sockaddr,
            &mut socklen,
            flags
        ))
        .map(|socket| unsafe { net::UnixStream::from_raw_fd(socket) })?;

        Ok((
            crate::UnixStream::from_std(socket),
            SocketAddr::from_parts(addr, socklen),
        ))
    }

    /// Creates a UnixListener bound from std `UnixListener`.
    ///
    /// # Examples
    /// ```no_run
    /// use std::os::unix::net::UnixListener;
    ///
    /// use ylong_io::UnixListener as Ylong_UnixListener;
    ///
    /// let sock = match UnixListener::bind("/socket/path") {
    ///     Ok(sock) => sock,
    ///     Err(err) => {
    ///         println!("bind fail: {err:?}");
    ///     }
    /// };
    /// let ylong_sock = Ylong_UnixListener::from_std(sock);
    /// ```
    pub fn from_std(socket: net::UnixListener) -> UnixListener {
        UnixListener { inner: socket }
    }

    /// Creates a new independently owned handle to the underlying socket.
    ///
    /// # Examples
    /// ```no_run
    /// use ylong_io::UnixListener;
    ///
    /// fn test() -> std::io::Result<()> {
    ///     let listener = UnixListener::bind("/socket/path")?;
    ///     let listener_copy = listener.try_clone().expect("try_clone failed");
    ///     Ok(())
    /// }
    /// ```
    pub fn try_clone(&self) -> io::Result<UnixListener> {
        Ok(Self::from_std(self.inner.try_clone()?))
    }

    /// Returns the local socket address of this listener.
    ///
    /// # Examples
    /// ```no_run
    /// use std::os::unix::net::UnixListener;
    ///
    /// fn test() -> std::io::Result<()> {
    ///     let listener = UnixListener::bind("/socket/path")?;
    ///     let addr = listener.local_addr().expect("get local_addr() fail");
    ///     Ok(())
    /// }
    /// ```
    pub fn local_addr(&self) -> io::Result<net::SocketAddr> {
        self.inner.local_addr()
    }

    /// Returns the error of the `SO_ERROR` option.
    ///
    /// # Examples
    /// ```no_run
    /// use ylong_io::UnixListener;
    ///
    /// fn test() -> std::io::Result<()> {
    ///     let listener = UnixListener::bind("/socket/path")?;
    ///     if let Ok(Some(err)) = listener.take_error() {
    ///         println!("get error: {err:?}");
    ///     }
    ///     Ok(())
    /// }
    /// ```
    pub fn take_error(&self) -> io::Result<Option<io::Error>> {
        self.inner.take_error()
    }
}

impl Source for UnixListener {
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

impl std::fmt::Debug for UnixListener {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        self.inner.fmt(f)
    }
}

impl IntoRawFd for UnixListener {
    fn into_raw_fd(self) -> RawFd {
        self.inner.into_raw_fd()
    }
}

impl FromRawFd for UnixListener {
    unsafe fn from_raw_fd(fd: RawFd) -> UnixListener {
        UnixListener::from_std(FromRawFd::from_raw_fd(fd))
    }
}
