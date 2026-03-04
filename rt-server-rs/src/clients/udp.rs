//!
//! UDP based Client Transport with dual-socket (Rx/Tx) design.
//!

extern crate log;
#[allow(unused_imports)]
use log::{debug, error, info, trace, warn};
const LOG_TAG: &str = "UdpTransport";

use std::collections::HashMap;
use std::net::{SocketAddr, UdpSocket};
use std::sync::{
    Arc, Mutex,
    atomic::{AtomicBool, Ordering},
};
use std::time;

use super::ClientTransport;
use rt_message::{MESSAGE_LEN_MAX, Message, MessageType};

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
    const TIMEOUT: std::time::Duration = time::Duration::from_millis(UdpTransport::TIMEOUT_MS);

    pub fn new(port: u16) -> Self {
        UdpTransport {
            port,
            rx_socket: Mutex::new(None),
            tx_socket: Mutex::new(None),
            sessions: Arc::new(Mutex::new(HashMap::new())),
        }
    }

    fn create_socket(port: u16) -> Result<UdpSocket, std::io::Error> {
        let socket = UdpSocket::bind(format!("127.0.0.1:{}", port))?;
        socket.set_read_timeout(Some(UdpTransport::TIMEOUT))?;
        Ok(socket)
    }
}

impl ClientTransport for UdpTransport {
    fn on_start(&self) -> Result<(), String> {
        info!("{}: on_start port={}", LOG_TAG, self.port);
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
        info!("{}: on_shutdown", LOG_TAG);
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
                    match std::str::from_utf8(&recv_buf[..buf_size]) {
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
                            warn!("{}: invalid UTF-8 from {}, {:?}", LOG_TAG, src_addr, e);
                            continue;
                        }
                    }
                }
                Err(e) => {
                    if e.kind() == std::io::ErrorKind::TimedOut {
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
                warn!("{}: client is not connected id={}", LOG_TAG, msg.cid);
                continue;
            };
            match msg.to_str() {
                Ok(udpmsg) => {
                    if let Err(e) = socket.send_to(udpmsg.as_bytes(), to_addr) {
                        error!("{}: Failed to send to {}: {:?}", LOG_TAG, to_addr, e);
                    } else {
                        trace!("{}: sent response {:?}", LOG_TAG, msg);
                    }
                }
                Err(e) => {
                    error!(
                        "{}: Failed to serialize message for client {}: {:?}",
                        LOG_TAG, msg.cid, e
                    );
                }
            }
        }

        Ok(())
    }
}
