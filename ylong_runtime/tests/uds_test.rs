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

#![cfg(all(target_os = "linux", feature = "net"))]

use ylong_runtime::io::{AsyncReadExt, AsyncWriteExt};
use ylong_runtime::net::{UnixDatagram, UnixListener, UnixStream};

#[test]
/// uds UnixListener/UnixStream test case.
///
/// # Title
/// uds_stream_test
///
/// # Brief
/// 1. Creates a server and a client.
/// 2. Sends and writes message to each other.
///
/// # Note
/// Each execution will leave a file under PATH which must be deleted,
/// otherwise the next bind operation will fail.
fn uds_stream_test() {
    const PATH: &str = "/tmp/uds_path";

    async fn server() {
        let mut read_buf = [0_u8; 12];
        let listener = UnixListener::bind(PATH).unwrap();
        loop {
            let (mut stream, _) = listener.accept().await.unwrap();
            match stream.write(b"hello client").await {
                Ok(n) => {
                    assert_eq!(n, "hello client".len());
                }
                Err(e) => {
                    assert_eq!(0, 1, "client send failed {e}");
                }
            }
            match stream.read(&mut read_buf).await {
                Ok(n) => {
                    assert_eq!(
                        std::str::from_utf8(&read_buf).unwrap(),
                        "hello server".to_string()
                    );
                    assert_eq!(n, "hello server".len());
                    break;
                }
                Err(e) => {
                    assert_eq!(0, 1, "client recv failed {e}");
                }
            }
        }
    }

    async fn client() {
        let mut read_buf = [0_u8; 12];
        loop {
            if let Ok(mut stream) = UnixStream::connect(PATH).await {
                match stream.read(&mut read_buf).await {
                    Ok(n) => {
                        assert_eq!(n, "hello server".len());
                    }
                    Err(e) => {
                        assert_eq!(0, 1, "client send failed {e}");
                    }
                }
                match stream.write(b"hello server").await {
                    Ok(n) => {
                        assert_eq!(
                            std::str::from_utf8(&read_buf).unwrap(),
                            "hello client".to_string()
                        );
                        assert_eq!(n, "hello client".len());
                        break;
                    }
                    Err(e) => {
                        assert_eq!(0, 1, "client recv failed {e}");
                    }
                }
            }
        }
    }

    std::thread::spawn(|| {
        ylong_runtime::block_on(client());
    });
    ylong_runtime::block_on(server());

    std::fs::remove_file(PATH).unwrap();
}

/// uds UnixDatagram test case.
///
/// # Title
/// uds_datagram_test
///
/// # Brief
/// 1. Creates a server and a client.
/// 2. Client Sends message and server recv it.
///
/// # Note
/// Each execution will leave a file under PATH which must be deleted,
/// otherwise the next bind operation will fail.
#[test]
fn uds_datagram_test() {
    const PATH: &str = "/tmp/uds_path1";

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

    std::thread::spawn(|| {
        ylong_runtime::block_on(client());
    });
    ylong_runtime::block_on(server());

    std::fs::remove_file(PATH).unwrap();
}
