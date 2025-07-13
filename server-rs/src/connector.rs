//!
//! Handles client connections and message exchange.
//!

extern crate log;
#[allow(unused_imports)]
use log::{debug, error, info, trace, warn};

use std::collections::HashMap;
use std::net::{SocketAddr, UdpSocket};
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::mpsc::Sender;
use std::sync::{Arc, Mutex};
use std::thread;
use std::time;

use dps_message::{MESSAGE_LEN_MAX, Message, MessageType};

use crate::Event;

pub struct ClientConnector {
    port: u16,
    socket: Option<UdpSocket>,
    clients: Arc<Mutex<HashMap<u16, SocketAddr>>>,
    stop_flag: Arc<AtomicBool>,
    thread_handle: Option<thread::JoinHandle<()>>,
}

impl ClientConnector {
    const TIMEOUT_MS: u64 = 500;
    const TIMEOUT: std::time::Duration = time::Duration::from_millis(ClientConnector::TIMEOUT_MS);

    pub fn new(port: u16) -> Self {
        ClientConnector {
            port,
            socket: None,
            clients: Arc::new(Mutex::new(HashMap::new())),
            stop_flag: Arc::new(AtomicBool::new(false)),
            thread_handle: None,
        }
    }

    pub fn start(&mut self, tx_channel: Sender<Event>) -> bool {
        info!(
            "ClientConnector: start port={}, t/o={}",
            self.port,
            ClientConnector::TIMEOUT_MS
        );

        // clear connected clients.
        self.clients.lock().unwrap().clear();

        // setup socket.
        let Ok(s) = ClientConnector::create_socket(self.port) else {
            error!("ClientConnector: Failed to create socket");
            return false;
        };
        self.socket = Some(s.try_clone().unwrap());

        // setup thread data and launch thread.
        let socket = s;
        let clients = Arc::clone(&(self.clients));
        let stop_flag = Arc::clone(&self.stop_flag);
        self.thread_handle = Some(thread::spawn(move || {
            ClientConnector::thread_receiver(stop_flag, socket, clients, tx_channel);
        }));

        return true;
    }

    pub fn stop(&mut self) {
        if let Some(handle) = self.thread_handle.take() {
            info!("ClientConnector: stop requested");
            self.stop_flag.store(true, Ordering::Relaxed);
            handle.join().unwrap();
            info!("ClientConnector: stopped");
            self.stop_flag.store(false, Ordering::Relaxed);
        } else {
            warn!("ClientConnector: not started");
        }
    }

    pub fn send_responses(&mut self, msgs: Vec<Message>) {
        if let Some(socket) = &self.socket {
            let clients = self.clients.lock().unwrap();
            for msg in msgs {
                if let Some(to_addr) = clients.get(&msg.cid) {
                    self.send_message(&msg, socket, to_addr);
                } else {
                    warn!("Connector: client is not connected id={}", msg.cid);
                }
            }
        } else {
            warn!("Connector: not started");
        }
    }

    // -----
    // private methods.

    fn create_socket(port: u16) -> Result<UdpSocket, std::io::Error> {
        let socket = UdpSocket::bind(format!("127.0.0.1:{}", port))?;
        socket.set_read_timeout(Some(ClientConnector::TIMEOUT))?;
        Ok(socket)
    }

    fn thread_receiver_process_udp_msg(
        src_addr: SocketAddr,
        recv_msg: &str,
        clients: &Mutex<HashMap<u16, SocketAddr>>,
        tx_channel: &Sender<Event>,
    ) {
        // convert packet to message.
        let recv_msg = Message::from_str(recv_msg);
        if let Ok(msg) = recv_msg {
            // store client addr for response.
            if msg.mtype == MessageType::Join {
                let mut clients = clients.lock().unwrap();
                clients.insert(msg.cid, src_addr);
            }
            // notify.
            _ = tx_channel.send(Event::ClientMsg(msg));
        } else {
            warn!(
                "Connector: invalid message from {}, {:?}",
                src_addr, recv_msg
            );
        }
    }

    fn thread_receiver(
        stop_flag: Arc<AtomicBool>,
        socket: UdpSocket,
        clients: Arc<Mutex<HashMap<u16, SocketAddr>>>,
        tx_channel: Sender<Event>,
    ) {
        debug!("ClientConnector: receive thread started.");
        let mut recv_buf = [0u8; MESSAGE_LEN_MAX];
        loop {
            // check stop request.
            if stop_flag.load(Ordering::Relaxed) == true {
                info!("ClientConnector: stop request detected, exitting");
                break;
            }
            // wait client w/ timeout for checking stop request.
            match socket.recv_from(&mut recv_buf) {
                Ok((buf_size, src_addr)) => match str::from_utf8(&recv_buf[..buf_size]) {
                    Ok(recv_msg) => {
                        ClientConnector::thread_receiver_process_udp_msg(
                            src_addr,
                            recv_msg,
                            clients.as_ref(),
                            &tx_channel,
                        );
                    }
                    Err(e) => {
                        warn!("Connector: invalid UTF-8 from {}, {:?}", src_addr, e);
                    }
                },
                Err(e) => {
                    if e.kind() == std::io::ErrorKind::TimedOut {
                        // keep going.
                    } else {
                        warn!("Connector: failed to read: {:?}", e);
                        // todo: report and shutdown.
                    }
                }
            }
        }
        debug!("ClientConnector: receive thread stopped.");
    }

    // ---

    fn send_message(&self, msg: &Message, socket: &UdpSocket, to_addr: &SocketAddr) {
        match msg.to_str() {
            Ok(udpmsg) => {
                if let Err(e) = socket.send_to(udpmsg.as_bytes(), to_addr) {
                    error!("Failed to send to {}: {:?}", to_addr, e);
                }
            }
            Err(e) => {
                error!(
                    "Failed to serialize message for client {}: {:?}",
                    msg.cid, e
                );
            }
        }
    }
}
