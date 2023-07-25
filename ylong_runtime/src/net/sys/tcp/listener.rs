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
use std::net::SocketAddr;

use ylong_io::Interest;

use crate::net::sys::addr::ToSocketAddrs;
use crate::net::{AsyncSource, TcpStream};

/// An asynchronous version of [`std::net::TcpListener`]. Provides async
/// bind/accept methods.
///
/// # Example
/// ```rust
/// use std::io;
///
/// use ylong_runtime::net::TcpListener;
///
/// async fn io_func() -> io::Result<()> {
///     let addr = "127.0.0.1:8080";
///     let server = TcpListener::bind(addr).await?;
///     let (stream, address) = server.accept().await?;
///     Ok(())
/// }
/// ```
pub struct TcpListener {
    source: AsyncSource<ylong_io::TcpListener>,
}

impl TcpListener {
    /// A TCP socket server, asynchronously listening for connections.
    ///
    /// After creating a `TcpListener` by binding it to a socket address, it
    /// listens for incoming TCP connections asynchronously. These
    /// connections can be accepted by calling [`TcpListener::accept`]
    ///
    /// # Note
    ///
    /// If there are multiple addresses in SocketAddr, it will attempt to
    /// connect them in sequence until one of the addrs returns success. If
    /// all connections fail, it returns the error of the last connection.
    /// This behavior is consistent with std.
    ///
    /// # Example
    /// ```rust
    /// use std::io;
    ///
    /// use ylong_runtime::net::TcpListener;
    ///
    /// async fn io_func() -> io::Result<()> {
    ///     let addr = "127.0.0.1:8080";
    ///     let server = TcpListener::bind(addr).await?;
    ///     Ok(())
    /// }
    /// ```
    pub async fn bind<A: ToSocketAddrs>(addr: A) -> io::Result<TcpListener> {
        super::super::addr::each_addr(addr, ylong_io::TcpListener::bind)
            .await
            .map(TcpListener::new)
            .and_then(|op| op)
    }

    /// Asynchronously accepts a new incoming connection from this listener.
    ///
    /// When connection gets established, the corresponding [`TcpStream`] and
    /// the remote peer's address will be returned.
    ///
    ///
    /// # Example
    /// ```rust
    /// use std::io;
    ///
    /// use ylong_runtime::net::TcpListener;
    ///
    /// async fn io_func() -> io::Result<()> {
    ///     let addr = "127.0.0.1:8080";
    ///     let server = TcpListener::bind(addr).await?;
    ///     let (stream, address) = server.accept().await?;
    ///     Ok(())
    /// }
    /// ```
    pub async fn accept(&self) -> io::Result<(TcpStream, SocketAddr)> {
        let (stream, addr) = self
            .source
            .async_process(Interest::READABLE, || self.source.accept())
            .await?;
        let stream = TcpStream::new(stream)?;
        Ok((stream, addr))
    }

    // Registers the ylong_io::TcpListener's fd to the reactor, and returns the
    // async TcpListener
    pub(crate) fn new(listener: ylong_io::TcpListener) -> io::Result<Self> {
        let source = AsyncSource::new(listener, None)?;
        Ok(TcpListener { source })
    }
}
