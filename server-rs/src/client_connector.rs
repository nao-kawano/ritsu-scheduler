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

    fn create_socket(port: u16) -> Result<UdpSocket, std::io::Error> {
        let socket = UdpSocket::bind(format!("127.0.0.1:{}", port))?;
        socket.set_read_timeout(Some(ClientConnector::TIMEOUT))?;
        Ok(socket)
    }

    fn thread_receiver_process_udp_msg(
        src_addr: SocketAddr,
        recv_buf: &[u8],
        clients: &Mutex<HashMap<u16, SocketAddr>>,
        tx_channel: &Sender<Event>,
    ) {
        let recv_msg = std::str::from_utf8(recv_buf).unwrap(); // todo: error handling.
        // convert packet to message.
        let recv_msg = Message::from_msg(recv_msg);
        if let Ok(msg) = recv_msg {
            // store client addr for response.
            if msg.message_type == MessageType::Join {
                let mut clients = clients.lock().unwrap();
                clients.insert(msg.client_id, src_addr);
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
                Ok((buf_size, src_addr)) => {
                    let recv_buf = &recv_buf[..buf_size];
                    ClientConnector::thread_receiver_process_udp_msg(
                        src_addr,
                        recv_buf,
                        clients.as_ref(),
                        &tx_channel,
                    );
                }
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

    pub fn start(&mut self, tx_channel: Sender<Event>) -> bool {
        info!(
            "ClientConnector: start port={}, t/o={}",
            self.port,
            ClientConnector::TIMEOUT_MS
        );

        // clear connected clients.
        {
            let mut clients = self.clients.lock().unwrap();
            clients.clear();
        }

        // setup socket.
        let s = match ClientConnector::create_socket(self.port) {
            Ok(socket) => socket,
            Err(e) => {
                error!("ClientConnector: Failed to create socket: {:?}", e);
                return false;
            }
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
                if let Some(to_addr) = clients.get(&msg.client_id) {
                    let udpmsg = msg.to_msg().unwrap(); // todo: error handling.
                    _ = socket.send_to(udpmsg.as_bytes(), to_addr);
                } else {
                    warn!("Connector: client is not connected id={}", msg.client_id);
                }
            }
        } else {
            warn!("Connector: not started");
        }
    }
}
