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

#![cfg(all(target_family = "unix", feature = "net"))]

use std::io;
use std::os::fd::AsRawFd;

use ylong_runtime::io::{AsyncReadExt, AsyncWriteExt};
use ylong_runtime::net::{UnixDatagram, UnixListener, UnixStream};

#[test]
/// uds UnixListener/UnixStream test case.
///
/// # Title
/// sdv_uds_stream_test
///
/// # Brief
/// 1. Creates a server and a client.
/// 2. Sends and writes message to each other.
///
/// # Note
/// Each execution will leave a file under PATH which must be deleted,
/// otherwise the next bind operation will fail.
fn sdv_uds_stream_test() {
    const PATH: &str = "/tmp/uds_path1";
    let _ = std::fs::remove_file(PATH);

    async fn server() {
        let mut read_buf = [0_u8; 12];
        let listener = UnixListener::bind(PATH).unwrap();
        let (mut stream, _) = listener.accept().await.unwrap();
        let n = stream.write(b"hello client").await.unwrap();
        assert_eq!(n, "hello client".len());
        let n = stream.read(&mut read_buf).await.unwrap();
        assert_eq!(n, "hello server".len());
        assert_eq!(
            std::str::from_utf8(&read_buf).unwrap(),
            "hello server".to_string()
        );
    }

    async fn client() {
        let mut read_buf = [0_u8; 12];
        loop {
            if let Ok(mut stream) = UnixStream::connect(PATH).await {
                let n = stream.read(&mut read_buf).await.unwrap();
                assert_eq!(n, "hello server".len());
                assert_eq!(
                    std::str::from_utf8(&read_buf).unwrap(),
                    "hello client".to_string()
                );

                let n = stream.write(b"hello server").await.unwrap();
                assert_eq!(n, "hello client".len());
                break;
            }
        }
    }

    let handle = ylong_runtime::spawn(client());
    ylong_runtime::block_on(server());
    ylong_runtime::block_on(handle).unwrap();

    std::fs::remove_file(PATH).unwrap();
}

/// uds UnixDatagram test case.
///
/// # Title
/// sdv_uds_datagram_test
///
/// # Brief
/// 1. Creates a server and a client.
/// 2. Client Sends message and server recv it.
///
/// # Note
/// Each execution will leave a file under PATH which must be deleted,
/// otherwise the next bind operation will fail.
#[test]
fn sdv_uds_datagram_test() {
    const PATH: &str = "/tmp/uds_path2";
    let _ = std::fs::remove_file(PATH);

    async fn server() {
        let socket = UnixDatagram::bind(PATH).unwrap();

        let mut buf = vec![0; 11];
        socket.recv(buf.as_mut_slice()).await.expect("recv failed");
        assert_eq!(
            std::str::from_utf8(&buf).unwrap(),
            "hello world".to_string()
        );
    }

    async fn client() {
        let socket = UnixDatagram::unbound().unwrap();
        loop {
            if socket.connect(PATH).is_ok() {
                socket.send(b"hello world").await.expect("send failed");
                break;
            };
        }
    }

    let handle = ylong_runtime::spawn(client());
    ylong_runtime::block_on(server());
    ylong_runtime::block_on(handle).unwrap();

    std::fs::remove_file(PATH).unwrap();
}

/// Uds UnixStream try_xxx() test case.
///
/// # Title
/// sdv_uds_stream_try_test
///
/// # Brief
/// 1. Creates a server and a client with `pair()`.
/// 2. Server send message with `writable()` and `try_write()`.
/// 3. Client receive message with `readable()` and `try_read()`.
/// 4. Check result is correct.
#[test]
fn sdv_uds_stream_try_test() {
    let handle = ylong_runtime::spawn(async {
        let (server, client) = UnixStream::pair().unwrap();
        loop {
            server.writable().await.unwrap();
            match server.try_write(b"hello") {
                Ok(n) => {
                    assert_eq!(n, "hello".len());
                    break;
                }
                Err(ref e) if e.kind() == io::ErrorKind::WouldBlock => continue,
                Err(e) => panic!("{e:?}"),
            }
        }
        loop {
            client.readable().await.unwrap();
            let mut data = vec![0; 5];
            match client.try_read(&mut data) {
                Ok(n) => {
                    assert_eq!(n, "hello".len());
                    assert_eq!(std::str::from_utf8(&data).unwrap(), "hello".to_string());
                    break;
                }
                Err(ref e) if e.kind() == io::ErrorKind::WouldBlock => continue,
                Err(e) => panic!("{e:?}"),
            }
        }
    });
    ylong_runtime::block_on(handle).unwrap();
}

/// Uds UnixDatagram try_xxx() test case.
///
/// # Title
/// sdv_uds_datagram_try_test
///
/// # Brief
/// 1. Creates a server and a client with `pair()`.
/// 2. Server send message with `writable()` and `try_send()`.
/// 3. Client receive message with `readable()` and `try_recv()`.
/// 4. Check result is correct.
#[test]
fn sdv_uds_datagram_try_test() {
    let handle = ylong_runtime::spawn(async {
        let (server, client) = UnixDatagram::pair().unwrap();
        loop {
            server.writable().await.unwrap();
            match server.try_send(b"hello") {
                Ok(n) => {
                    assert_eq!(n, "hello".len());
                    break;
                }
                Err(ref e) if e.kind() == io::ErrorKind::WouldBlock => continue,
                Err(e) => panic!("{e:?}"),
            }
        }
        loop {
            client.readable().await.unwrap();
            let mut data = vec![0; 5];
            match client.try_recv(&mut data) {
                Ok(n) => {
                    assert_eq!(n, "hello".len());
                    assert_eq!(std::str::from_utf8(&data).unwrap(), "hello".to_string());
                    break;
                }
                Err(ref e) if e.kind() == io::ErrorKind::WouldBlock => continue,
                Err(e) => panic!("{e:?}"),
            }
        }
    });
    ylong_runtime::block_on(handle).unwrap();
}

/// Uds UnixListener test case.
///
/// # Title
/// sdv_uds_listener_baisc_test
///
/// # Brief
/// 1. Create a std UnixListener with `bind()`.
/// 2. Convert std UnixListener to Ylong_runtime UnixListener.
/// 3. Check result is correct.
#[test]
fn sdv_uds_listener_baisc_test() {
    const PATH: &str = "/tmp/uds_path3";
    let _ = std::fs::remove_file(PATH);
    let listener = std::os::unix::net::UnixListener::bind(PATH).unwrap();
    let handle = ylong_runtime::spawn(async {
        let res = UnixListener::from_std(listener);
        assert!(res.is_ok());
        let listener = res.unwrap();
        assert!(listener.as_raw_fd() >= 0);
        assert!(listener.take_error().is_ok());
    });
    ylong_runtime::block_on(handle).unwrap();
    let _ = std::fs::remove_file(PATH);
}

/// Uds UnixStream test case.
///
/// # Title
/// sdv_uds_stream_baisc_test
///
/// # Brief
/// 1. Create a std UnixStream with `pair()`.
/// 2. Convert std UnixStream to Ylong_runtime UnixStream.
/// 3. Check result is correct.
#[test]
fn sdv_uds_stream_baisc_test() {
    let (stream, _) = std::os::unix::net::UnixStream::pair().unwrap();
    let handle = ylong_runtime::spawn(async {
        let res = UnixStream::from_std(stream);
        assert!(res.is_ok());
        let stream = res.unwrap();
        assert!(stream.as_raw_fd() >= 0);
        assert!(stream.take_error().is_ok());
    });
    ylong_runtime::block_on(handle).unwrap();
}

/// Uds UnixDatagram test case.
///
/// # Title
/// sdv_uds_datagram_baisc_test
///
/// # Brief
/// 1. Create a std UnixDatagram with `pair()`.
/// 2. Convert std UnixDatagram to Ylong_runtime UnixDatagram.
/// 3. Check result is correct.
#[test]
fn sdv_uds_datagram_baisc_test() {
    let (datagram, _) = std::os::unix::net::UnixDatagram::pair().unwrap();
    let handle = ylong_runtime::spawn(async {
        let res = UnixDatagram::from_std(datagram);
        assert!(res.is_ok());
        let datagram = res.unwrap();
        assert!(datagram.as_raw_fd() >= 0);
        assert!(datagram.take_error().is_ok());
    });
    ylong_runtime::block_on(handle).unwrap();
}
