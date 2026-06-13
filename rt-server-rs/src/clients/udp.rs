// Copyright 2026 Naoyuki Kawano
//
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
// =============================================================================
//!
//! UDP based Client Transport with dual-socket (Rx/Tx) design.
//!

#[allow(unused_imports)]
use log::{debug, error, info, trace, warn};

use rt_message::{MESSAGE_LEN_MAX, Message, MessageType};

use super::ClientTransport;

use std::collections::HashMap;
use std::io;
use std::net::{SocketAddr, UdpSocket};
use std::str;
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::{Arc, Mutex};
use std::time;

/* -------------------------------------------------------------------------- */

pub struct UdpTransport {
    port: u16,
    /// Separate socket handles for receiving and sending to allow concurrent I/O without lock contention.
    /// Wrapped in Mutex for thread-safe initialization and shutdown.
    rx_socket: Mutex<Option<UdpSocket>>,
    tx_socket: Mutex<Option<UdpSocket>>,
    sessions: Arc<Mutex<HashMap<u16, SocketAddr>>>,
}

impl UdpTransport {
    const TIMEOUT_MS: u64 = 100;
    const TIMEOUT: time::Duration = time::Duration::from_millis(UdpTransport::TIMEOUT_MS);

    pub fn new(port: u16) -> Self {
        UdpTransport {
            port,
            rx_socket: Mutex::new(None),
            tx_socket: Mutex::new(None),
            sessions: Arc::new(Mutex::new(HashMap::new())),
        }
    }

    fn create_socket(port: u16) -> Result<UdpSocket, io::Error> {
        let socket = UdpSocket::bind(format!("127.0.0.1:{}", port))?;
        socket.set_read_timeout(Some(UdpTransport::TIMEOUT))?;
        Ok(socket)
    }
}

impl ClientTransport for UdpTransport {
    fn on_start(&self) -> Result<(), String> {
        info!("on_start port={}", self.port);
        self.sessions.lock().unwrap().clear();

        // Create socket.
        let s_rx = UdpTransport::create_socket(self.port)
            .map_err(|e| format!("Failed to create socket: {}", e))?;
        let s_tx = s_rx
            .try_clone()
            .map_err(|e| format!("Failed to clone socket: {}", e))?;

        // Store socket.
        *(self.rx_socket.lock().unwrap()) = Some(s_rx);
        *(self.tx_socket.lock().unwrap()) = Some(s_tx);

        Ok(())
    }

    fn on_shutdown(&self) {
        info!("on_shutdown");
        // Clearing sockets will release OS resources.
        *(self.rx_socket.lock().unwrap()) = None;
        *(self.tx_socket.lock().unwrap()) = None;
    }

    fn receive(&self, stop_flag: &Arc<AtomicBool>) -> Result<Option<Message>, String> {
        let lock = self.rx_socket.lock().unwrap();
        let socket = lock.as_ref().ok_or("UdpTransport not started")?;
        let mut recv_buf = [0u8; MESSAGE_LEN_MAX];

        loop {
            // Check stop request.
            if stop_flag.load(Ordering::Relaxed) {
                return Ok(None);
            }
            // Receive one message.
            match socket.recv_from(&mut recv_buf) {
                Ok((buf_size, src_addr)) => {
                    match str::from_utf8(&recv_buf[..buf_size]) {
                        Ok(recv_msg) => {
                            let msg = Message::from_str(recv_msg)
                                .map_err(|e| format!("Invalid message: {:?}", e))?;
                            // store client addr for response.
                            if msg.mtype == MessageType::Join {
                                let mut sessions = self.sessions.lock().unwrap();
                                sessions.insert(msg.cid, src_addr);
                            }
                            return Ok(Some(msg));
                        }
                        Err(e) => {
                            warn!("invalid UTF-8 from {}, {:?}", src_addr, e);
                            continue;
                        }
                    }
                }
                Err(e) => {
                    if e.kind() == io::ErrorKind::TimedOut {
                        continue;
                    } else if e.kind() == io::ErrorKind::ConnectionReset {
                        // On Windows, recv_from may return ConnectionReset if a previous send_to
                        // failed with Port Unreachable. We should ignore this and continue.
                        warn!("receive error (ignored): {:?}", e);
                        continue;
                    } else {
                        return Err(format!("failed to read: {:?}", e));
                    }
                }
            }
        }
    }

    fn send_all(&self, msgs: Vec<Message>) -> Result<(), String> {
        let lock = self.tx_socket.lock().unwrap();
        let socket = lock.as_ref().ok_or("UdpTransport not started")?;
        let sessions = self.sessions.lock().unwrap();

        for msg in msgs {
            let Some(to_addr) = sessions.get(&msg.cid) else {
                warn!("client CID:{:03} is not connected, dropped.", msg.cid);
                continue;
            };
            match msg.to_str() {
                Ok(udpmsg) => {
                    if let Err(e) = socket.send_to(udpmsg.as_bytes(), to_addr) {
                        error!("failed to send to {}: {:?}", to_addr, e);
                    }
                }
                Err(e) => {
                    error!(
                        "failed to serialize message for CID:{:03}: {:?}",
                        msg.cid, e
                    );
                }
            }
        }

        Ok(())
    }
}
