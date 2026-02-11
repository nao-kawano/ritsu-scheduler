//!
//! Client for DPS.
//!

extern crate log;
#[allow(unused_imports)]
use log::{debug, error, info, trace, warn};
const LOG_TAG: &str = "DPSClient";

use dps_message::{MESSAGE_LEN_MAX, Message, MessageType};

use crate::dpsclientconfig::DPSClientConfig;

use std::net::UdpSocket;
use std::time::Duration;

#[cfg(target_os = "windows")]
use windows::Win32::Media::{timeBeginPeriod, timeEndPeriod};

#[cfg(test)]
#[path = "dpsclient_test.rs"]
mod dpsclient_test;

/* -------------------------------------------------------------------------- */

pub struct DPSClient {
    pub config: DPSClientConfig,
    server_addr: String,
    client_id: u16,
    sock: Option<UdpSocket>,
    connected: bool,
    startup: bool,
    message_id: u8,
}

impl DPSClient {
    pub fn new(
        host: String,
        port: u16,
        client_id: u16,
        run_cycle_sec: f64,
        startup_wait_sec: f64,
    ) -> Self {
        DPSClient {
            server_addr: format!("{}:{}", host, port),
            client_id,
            config: DPSClientConfig::new(run_cycle_sec, startup_wait_sec),
            sock: None,
            connected: false,
            startup: true,
            message_id: 0,
        }
    }

    pub fn new_with_config(
        host: String,
        port: u16,
        client_id: u16,
        config: DPSClientConfig,
    ) -> Self {
        DPSClient {
            server_addr: format!("{}:{}", host, port),
            client_id,
            config,
            sock: None,
            connected: false,
            startup: true,
            message_id: 0,
        }
    }

    pub fn join(&mut self) -> bool {
        if self.connected {
            warn!("{}: already joined, skip", LOG_TAG);
            return true;
        } else {
            match UdpSocket::bind("0.0.0.0:0") {
                Ok(sock) => {
                    self.sock = Some(sock);
                    let resp_type = self._send_request(
                        MessageType::Join,
                        self.config.retry_sec_join,
                        self.config.retry_count_join,
                    );
                    if resp_type == MessageType::Ok {
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

    pub fn exit(&mut self) {
        if !self.connected {
            warn!("{}: not connected, skip", LOG_TAG);
        } else {
            let _ = self._send_request(
                MessageType::Exit,
                self.config.retry_sec_exit,
                self.config.retry_count_exit,
            );
            if let Some(sock) = &self.sock {
                // Drop the socket to close it
                let _ = sock;
                self.sock = None;
                self.connected = false;
            }
        }
    }

    pub fn wait_next(&mut self) -> MessageType {
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

        let resp_type = self._send_request(MessageType::Ready, timeout_sec, retry_count);
        self.startup = false;

        resp_type
    }

    pub fn notify_done(&mut self) -> MessageType {
        if !self.connected {
            panic!("notify_done called before connected");
        }
        let resp_type = self._send_request(
            MessageType::Done,
            self.config.retry_sec_done,
            self.config.retry_count_done,
        );
        resp_type
    }

    // -----

    fn _send_request(
        &mut self,
        req_type: MessageType,
        timeout_sec: f64,
        retry_count: u32,
    ) -> MessageType {
        // check socket.
        let Some(sock) = &self.sock else {
            warn!("{}: invalid socket", LOG_TAG);
            return MessageType::Error;
        };
        DPSClient::_clear_recv_buffer(sock);
        // create request.
        self.message_id = (self.message_id + 1) % 10;
        let request: Message =
            Message::new(req_type, self.message_id, self.client_id, None).unwrap();
        let Ok(request_str) = request.to_str() else {
            warn!("{}: failed to create request for {:?}", LOG_TAG, request);
            return MessageType::Error;
        };
        // send request and wait response.
        #[cfg(target_os = "windows")]
        unsafe {
            // for high precision timeout.
            timeBeginPeriod(1);
        }
        let mut ret_resp_type: MessageType = MessageType::Error;
        sock.set_read_timeout(Some(Duration::from_secs_f64(timeout_sec)))
            .expect("set_read_timeout call failed");
        let mut recv_buf = [0u8; MESSAGE_LEN_MAX];
        for count in 0..=retry_count {
            trace!(
                "{}: >> send {:?}@{} ({}/{}) with t/o {} sec",
                LOG_TAG,
                req_type,
                self.message_id,
                count + 1,
                1 + retry_count,
                timeout_sec
            );
            match sock.send_to(&request_str.as_bytes(), &self.server_addr) {
                Ok(_) => {
                    let (response, need_retry) = DPSClient::_recv_response(sock, &mut recv_buf);
                    if need_retry {
                        trace!("{}: -- {:?} timeout, retrying...", LOG_TAG, req_type);
                        continue; // timeout, retry.
                    }
                    if let Some(response) = response {
                        if response.mid != self.message_id {
                            warn!(
                                "{}: << !! {:?} mid mismatch, expected {}, actual {}, continue",
                                LOG_TAG, req_type, self.message_id, response.mid
                            );
                            continue; // invalid mid, retry.
                        }
                        trace!(
                            "{}: << recv {:?} for {:?}",
                            LOG_TAG, response.mtype, req_type
                        );
                        ret_resp_type = response.mtype;
                    }
                    break;
                }
                Err(e) => {
                    warn!("{}: !! Error sending packet {:?} = {}", LOG_TAG, request, e);
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
        return ret_resp_type;
    }

    fn _recv_response(
        sock: &UdpSocket,
        recv_buf: &mut [u8; MESSAGE_LEN_MAX],
    ) -> (Option<Message>, bool) {
        match sock.recv_from(recv_buf) {
            Ok((buf_size, _)) => match str::from_utf8(&recv_buf[..buf_size]) {
                Ok(recv_msg) => match Message::from_str(recv_msg) {
                    Ok(response) => {
                        return (Some(response), false);
                    }
                    Err(e) => {
                        warn!("{}: failed to convert response {:?}", LOG_TAG, e);
                        return (None, false);
                    }
                },
                Err(e) => {
                    warn!("{}: invalid UTF-8 {:?}", LOG_TAG, e);
                    return (None, false);
                }
            },
            Err(e) => {
                if e.kind() == std::io::ErrorKind::TimedOut {
                    return (None, true);
                } else {
                    warn!("{}: failed to receive: {:?}", LOG_TAG, e);
                    return (None, false);
                }
            }
        }
    }

    fn _clear_recv_buffer(sock: &UdpSocket) {
        match sock.set_nonblocking(true) {
            Ok(_) => {
                let mut buffer = [0u8; MESSAGE_LEN_MAX];
                loop {
                    match sock.recv_from(&mut buffer) {
                        Ok((_size, _src)) => {
                            trace!("{}: drop old recv msg", LOG_TAG);
                            continue; // keep reading until the buffer is empty.
                        }
                        Err(e) => {
                            if e.kind() == std::io::ErrorKind::WouldBlock {
                                break; // buffer is empty.
                            } else {
                                warn!("{}: recv_from error: {:?}", LOG_TAG, e);
                                break;
                            }
                        }
                    }
                }
                let _ = sock.set_nonblocking(false);
            }
            Err(e) => {
                warn!("{}: failed to set non-blocking mode: {:?}", LOG_TAG, e);
            }
        }
    }
}
