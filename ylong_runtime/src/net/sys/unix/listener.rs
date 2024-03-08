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

use std::fmt;
use std::io::{Error, Result};
use std::os::unix::io::{AsFd, AsRawFd, BorrowedFd, RawFd};
use std::os::unix::net;
use std::path::Path;

use ylong_io::{Interest, SocketAddr, Source};

use crate::net::{AsyncSource, UnixStream};

/// A UDS server.
pub struct UnixListener {
    source: AsyncSource<ylong_io::UnixListener>,
}

impl UnixListener {
    /// Creates a new `UnixListener` from `ylong_io::UnixListener`
    fn new(listener: ylong_io::UnixListener) -> Result<Self> {
        let source = AsyncSource::new(listener, None)?;
        Ok(UnixListener { source })
    }

    /// Creates a new `UnixListener` bound to the specified socket.
    ///
    /// # Examples
    /// ```no_run
    /// use ylong_runtime::net::UnixListener;
    ///
    /// let _ = UnixListener::bind("/socket/path").unwrap();
    /// ```
    pub fn bind<P: AsRef<Path>>(path: P) -> Result<UnixListener> {
        let listener = ylong_io::UnixListener::bind(path)?;
        UnixListener::new(listener)
    }

    /// Waits a new incoming connection for this listener.
    ///
    /// # Examples
    /// ```no_run
    /// use ylong_runtime::net::UnixListener;
    ///
    /// async fn test() -> std::io::Result<()> {
    ///     let listener = UnixListener::bind("/socket/path")?;
    ///
    ///     let res = listener.accept().await;
    ///     match res {
    ///         Ok((socket, addr)) => println!("accept success: {addr:?}"),
    ///         Err(err) => println!("accept failed: {err:?}"),
    ///     }
    ///     Ok(())
    /// }
    /// ```
    pub async fn accept(&self) -> Result<(UnixStream, SocketAddr)> {
        let (stream, addr) = self
            .source
            .async_process(Interest::READABLE, || self.source.accept())
            .await?;

        let stream = UnixStream::new(stream)?;
        Ok((stream, addr))
    }

    /// Creates a UnixListener bound from std `UnixListener`.
    ///
    /// # Examples
    /// ```no_run
    /// use std::os::unix::net::UnixListener as StdUnixListener;
    ///
    /// use ylong_runtime::net::UnixListener;
    ///
    /// let socket = StdUnixListener::bind("/socket/path").unwrap();
    /// let ylong_socket = UnixListener::from_std(socket);
    /// ```
    pub fn from_std(listener: net::UnixListener) -> Result<UnixListener> {
        let listener = ylong_io::UnixListener::from_std(listener);
        UnixListener::new(listener)
    }

    /// Returns the value of the `SO_ERROR` option.
    ///
    /// # Examples
    /// ```no_run
    /// use ylong_runtime::net::UnixListener;
    ///
    /// fn test() -> std::io::Result<()> {
    ///     let listener = UnixListener::bind("/socket/path")?;
    ///     if let Ok(Some(err)) = listener.take_error() {
    ///         println!("get error: {err:?}");
    ///     }
    ///     Ok(())
    /// }
    /// ```
    pub fn take_error(&self) -> Result<Option<Error>> {
        self.source.take_error()
    }
}

impl fmt::Debug for UnixListener {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        self.source.fmt(f)
    }
}

impl AsRawFd for UnixListener {
    fn as_raw_fd(&self) -> RawFd {
        self.source.get_fd()
    }
}

impl AsFd for UnixListener {
    fn as_fd(&self) -> BorrowedFd<'_> {
        unsafe { BorrowedFd::borrow_raw(self.as_raw_fd()) }
    }
}

#[cfg(test)]
mod test {
    use std::os::fd::{AsFd, AsRawFd};

    use crate::io::{AsyncReadExt, AsyncWriteExt};
    use crate::net::{UnixListener, UnixStream};

    /// Uds UnixListener test case.
    ///
    /// # Title
    /// ut_uds_listener_baisc_test
    ///
    /// # Brief
    /// 1. Create a std UnixListener with `bind()`.
    /// 2. Convert std UnixListener to Ylong_runtime UnixListener.
    /// 3. Check result is correct.
    #[test]
    fn ut_uds_listener_baisc_test() {
        const PATH: &str = "/tmp/uds_listener_path1";
        let _ = std::fs::remove_file(PATH);
        let listener = std::os::unix::net::UnixListener::bind(PATH).unwrap();
        let handle = crate::spawn(async {
            let res = UnixListener::from_std(listener);
            assert!(res.is_ok());
            let listener = res.unwrap();
            assert!(listener.as_fd().as_raw_fd() >= 0);
            assert!(listener.as_raw_fd() >= 0);
            assert!(listener.take_error().is_ok());
        });
        crate::block_on(handle).unwrap();
        let _ = std::fs::remove_file(PATH);
    }

    /// Uds UnixListener test case.
    ///
    /// # Title
    /// ut_uds_listener_read_write_test
    ///
    /// # Brief
    /// 1. Create a server with `bind()` and `accept()`.
    /// 2. Create a client with `connect()`.
    /// 3. Server Sends message and client recv it.
    #[test]
    fn ut_uds_listener_read_write_test() {
        const PATH: &str = "/tmp/uds_listener_path2";
        let _ = std::fs::remove_file(PATH);
        let client_msg = "hello client";
        let server_msg = "hello server";

        crate::block_on(async {
            let mut read_buf = [0_u8; 12];
            let listener = UnixListener::bind(PATH).unwrap();

            let handle = crate::spawn(async {
                let mut stream = UnixStream::connect(PATH).await;
                while stream.is_err() {
                    stream = UnixStream::connect(PATH).await;
                }
                let mut stream = stream.unwrap();
                let mut read_buf = [0_u8; 12];
                stream.read_exact(&mut read_buf).await.unwrap();
                assert_eq!(
                    std::str::from_utf8(&read_buf).unwrap(),
                    client_msg.to_string()
                );
                stream.write_all(server_msg.as_bytes()).await.unwrap();
            });

            let (mut stream, _) = listener.accept().await.unwrap();
            stream.write_all(client_msg.as_bytes()).await.unwrap();

            stream.read_exact(&mut read_buf).await.unwrap();
            assert_eq!(
                std::str::from_utf8(&read_buf).unwrap(),
                server_msg.to_string()
            );

            handle.await.unwrap();
        });

        let _ = std::fs::remove_file(PATH);
    }
}
