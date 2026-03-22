//!
//! Client for Ritsu.
//!

#[allow(unused_imports)]
use log::{debug, error, info, trace, warn};

use rt_message::{MESSAGE_LEN_MAX, Message, MessageType, PROTOCOL_VERSION};

use crate::rtclientconfig::RtClientConfig;

use std::net::UdpSocket;
use std::time::{Duration, Instant};

#[cfg(target_os = "windows")]
use windows::Win32::Media::{timeBeginPeriod, timeEndPeriod};

#[cfg(test)]
#[path = "rtclient_test.rs"]
mod rtclient_test;

/* -------------------------------------------------------------------------- */

/// Client for interacting with the Ritsu server.
pub struct RtClient {
    /// Configuration for retries and timeouts.
    pub config: RtClientConfig,
    /// Address of the Ritsu server in "host:port" format.
    server_addr: String,
    /// Unique identifier for this client.
    client_id: u16,
    /// UDP socket for communication.
    sock: Option<UdpSocket>,
    /// Whether the client is currently connected to the server.
    connected: bool,
    /// Whether the client is in the startup phase.
    startup: bool,
    /// Current message ID for tracking requests and responses. 0 ~ 9.
    message_id: u8,
}

impl RtClient {
    /// Creates a new RtClient with default configuration.
    ///
    /// # Arguments
    ///
    /// * `host` - The server hostname or IP address.
    /// * `port` - The server port number.
    /// * `client_id` - The unique identifier for this client. 0 ~ 999.
    /// * `run_cycle_sec` - The expected execution cycle of the client in seconds.
    ///   For example, if the server's Cycle Time is 100ms and the client's Cycle is 2, set this to 0.2 (200ms).
    /// * `startup_wait_sec` - The total time to wait during the startup phase in seconds.
    pub fn new(
        host: String,
        port: u16,
        client_id: u16,
        run_cycle_sec: f64,
        startup_wait_sec: f64,
    ) -> Self {
        RtClient {
            server_addr: format!("{}:{}", host, port),
            client_id,
            config: RtClientConfig::new(run_cycle_sec, startup_wait_sec),
            sock: None,
            connected: false,
            startup: true,
            message_id: 0,
        }
    }

    /// Creates a new RtClient with a specific configuration.
    ///
    /// # Arguments
    ///
    /// * `host` - The server hostname or IP address.
    /// * `port` - The server port number.
    /// * `client_id` - The unique identifier for this client. 0 ~ 999.
    /// * `config` - A pre-configured `RtClientConfig` instance.
    pub fn new_with_config(
        host: String,
        port: u16,
        client_id: u16,
        config: RtClientConfig,
    ) -> Self {
        RtClient {
            server_addr: format!("{}:{}", host, port),
            client_id,
            config,
            sock: None,
            connected: false,
            startup: true,
            message_id: 0,
        }
    }

    /// Connects to the Ritsu server by sending a Join request.
    ///
    /// Returns `true` if the join was successful, `false` otherwise.
    pub fn join(&mut self) -> bool {
        if self.connected {
            warn!("already joined, skip");
            return true;
        } else {
            match UdpSocket::bind("0.0.0.0:0") {
                Ok(sock) => {
                    self.sock = Some(sock);
                    let resp = self._send_request(
                        MessageType::Join,
                        self.config.retry_sec_join,
                        self.config.retry_count_join,
                        Some(vec![("version".to_string(), PROTOCOL_VERSION.to_string())]),
                    );
                    if resp.mtype == MessageType::Joined {
                        self.connected = true;
                        self.startup = true;
                        return true;
                    } else {
                        return false;
                    }
                }
                Err(e) => {
                    error!("Error creating socket: {}", e);
                    return false;
                }
            }
        }
    }

    /// Disconnects from the Ritsu server by sending an Exit request.
    pub fn exit(&mut self) {
        if !self.connected {
            warn!("not connected, skip");
        } else {
            let _ = self._send_request(
                MessageType::Exit,
                self.config.retry_sec_exit,
                self.config.retry_count_exit,
                None,
            );
            if let Some(sock) = &self.sock {
                // Drop the socket to close it
                let _ = sock;
                self.sock = None;
                self.connected = false;
            }
        }
    }

    /// Waits for the next execution cycle by sending a Ready request to the server.
    ///
    /// This method blocks until the server responds or all retries are exhausted.
    /// It automatically handles different timeouts and retry counts for the startup phase.
    pub fn wait_next(&mut self) -> Message {
        if !self.connected {
            panic!("wait_next called before connected");
        }

        let timeout_sec = if self.startup {
            self.config.retry_sec_ready_startup
        } else {
            self.config.retry_sec_ready
        };
        let retry_count = if self.startup {
            self.config.retry_count_ready_startup
        } else {
            self.config.retry_count_ready
        };

        let resp = self._send_request(MessageType::Ready, timeout_sec, retry_count, None);
        self.startup = false;

        resp
    }

    /// Notifies the server that the current execution cycle is complete.
    ///
    /// Returns the message received from the server.
    pub fn notify_done(&mut self) -> Message {
        if !self.connected {
            panic!("notify_done called before connected");
        }
        let resp = self._send_request(
            MessageType::Done,
            self.config.retry_sec_done,
            self.config.retry_count_done,
            None,
        );
        resp
    }

    // -----

    /// Internal helper to send a request and wait for a response with retries.
    ///
    /// # Arguments
    ///
    /// * `req_type` - The type of message to send.
    /// * `timeout_sec` - Timeout for each attempt in seconds.
    /// * `retry_count` - Number of times to retry on timeout.
    /// * `extras` - Optional extra information to include in the request.
    fn _send_request(
        &mut self,
        req_type: MessageType,
        timeout_sec: f64,
        retry_count: u32,
        extras: Option<Vec<(String, String)>>,
    ) -> Message {
        // check socket.
        let Some(sock) = &self.sock else {
            warn!("invalid socket");
            return Message::new(MessageType::Error, self.message_id, self.client_id, None)
                .unwrap();
        };
        RtClient::_clear_recv_buffer(sock);
        // create request.
        self.message_id = (self.message_id + 1) % 10;
        let request: Message =
            Message::new(req_type, self.message_id, self.client_id, extras).unwrap();
        let Ok(request_str) = request.to_str() else {
            warn!("failed to create request for {:?}", request);
            return Message::new(MessageType::Error, self.message_id, self.client_id, None)
                .unwrap();
        };
        // send request and wait response.
        #[cfg(target_os = "windows")]
        unsafe {
            // for high precision timeout.
            timeBeginPeriod(1);
        }
        let mut ret_msg: Message =
            Message::new(MessageType::Error, self.message_id, self.client_id, None).unwrap();
        let mut recv_buf = [0u8; MESSAGE_LEN_MAX];
        for count in 0..=retry_count {
            trace!(
                ">> send {:?} CID:{:03} MID:{} ({}/{}) t/o:{:.3}s",
                req_type,
                self.client_id,
                self.message_id,
                count + 1,
                1 + retry_count,
                timeout_sec
            );
            match sock.send_to(&request_str.as_bytes(), &self.server_addr) {
                Ok(_) => {
                    if let Some(msg) = self._wait_for_matching_response(
                        sock,
                        timeout_sec,
                        req_type,
                        self.message_id,
                        &mut recv_buf,
                    ) {
                        ret_msg = msg;
                        break;
                    }
                }
                Err(e) => {
                    warn!("failed to send packet: {}", e);
                    break;
                }
            }
        }
        #[cfg(target_os = "windows")]
        unsafe {
            // revert to the default precision.
            timeEndPeriod(1);
        }
        //
        return ret_msg;
    }

    /// Internal helper to wait for a response that matches the expected MessageID.
    ///
    /// Returns `Some(Message)` if a match is found before `timeout_sec` elapses, otherwise `None`.
    fn _wait_for_matching_response(
        &self,
        sock: &UdpSocket,
        timeout_sec: f64,
        req_type: MessageType,
        expected_mid: u8,
        recv_buf: &mut [u8; MESSAGE_LEN_MAX],
    ) -> Option<Message> {
        let wait_start = Instant::now();
        loop {
            let now = Instant::now();
            let elapsed = now.duration_since(wait_start);
            if elapsed >= Duration::from_secs_f64(timeout_sec) {
                warn!("timeout, retrying... {:?}", req_type);
                return None;
            }
            let remaining = Duration::from_secs_f64(timeout_sec) - elapsed;
            let _ = sock.set_read_timeout(Some(remaining));

            let (response, is_timeout) = RtClient::_recv_response(sock, recv_buf);
            if is_timeout {
                warn!("timeout, retrying... {:?}", req_type);
                return None;
            }
            if let Some(response) = response {
                if response.mid == expected_mid {
                    trace!(
                        "<< recv {:?} for {:?} CID:{:03} MID:{}",
                        response.mtype, req_type, self.client_id, expected_mid
                    );
                    return Some(response);
                }
                warn!(
                    "<< mid mismatch, expected MID:{}, actual MID:{}, discard and keep waiting",
                    expected_mid, response.mid
                );
                continue; // invalid mid, keep waiting for the next packet.
            }
            // Other error in _recv_response (like parse error), keep waiting until deadline.
            continue;
        }
    }

    /// Internal helper to receive a single response from the socket.
    ///
    /// Returns a tuple containing the parsed message (if successful) and a boolean indicating if a timeout occurred.
    fn _recv_response(
        sock: &UdpSocket,
        recv_buf: &mut [u8; MESSAGE_LEN_MAX],
    ) -> (Option<Message>, bool) {
        match sock.recv_from(recv_buf) {
            Ok((buf_size, _)) => match std::str::from_utf8(&recv_buf[..buf_size]) {
                Ok(recv_msg) => match Message::from_str(recv_msg) {
                    Ok(response) => {
                        return (Some(response), false);
                    }
                    Err(e) => {
                        warn!("failed to convert response {:?}", e);
                        return (None, false);
                    }
                },
                Err(e) => {
                    warn!("invalid UTF-8 {:?}", e);
                    return (None, false);
                }
            },
            Err(e) => {
                if e.kind() == std::io::ErrorKind::TimedOut {
                    return (None, true);
                } else {
                    warn!("failed to receive: {:?}", e);
                    return (None, false);
                }
            }
        }
    }

    /// Clears the receive buffer of the socket by reading all pending messages.
    fn _clear_recv_buffer(sock: &UdpSocket) {
        match sock.set_nonblocking(true) {
            Ok(_) => {
                let mut buffer = [0u8; MESSAGE_LEN_MAX];
                loop {
                    match sock.recv_from(&mut buffer) {
                        Ok((_size, _src)) => {
                            trace!("drop old recv msg");
                            continue; // keep reading until the buffer is empty.
                        }
                        Err(e) => {
                            if e.kind() == std::io::ErrorKind::WouldBlock {
                                break; // buffer is empty.
                            } else {
                                warn!("recv_from error: {:?}", e);
                                break;
                            }
                        }
                    }
                }
                let _ = sock.set_nonblocking(false);
            }
            Err(e) => {
                warn!("failed to set non-blocking mode: {:?}", e);
            }
        }
    }
}
