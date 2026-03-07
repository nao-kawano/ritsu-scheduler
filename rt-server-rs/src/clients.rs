//!
//! Client Connector engine and Transport trait.
//!

extern crate log;
#[allow(unused_imports)]
use log::{debug, error, info, trace, warn};

use std::sync::Arc;
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::mpsc::Sender;
use std::thread;

use crate::event::Event;
use rt_message::Message;

pub mod udp;

/* -------------------------------------------------------------------------- */

/// Trait for client communication transports.
pub trait ClientTransport: Send + Sync {
    /// Called when the connector starts (within the receiver thread).
    fn on_start(&self) -> Result<(), String>;
    /// Called when the connector stops (within the receiver thread).
    fn on_shutdown(&self);
    /// Receives a single message. Returns Some(Message) on success, None on stop request.
    fn receive(&self, stop_flag: &Arc<AtomicBool>) -> Result<Option<Message>, String>;
    /// Sends multiple messages.
    fn send_all(&self, msgs: Vec<Message>) -> Result<(), String>;
}

/* -------------------------------------------------------------------------- */

/// Common engine for client connection management.
pub struct ClientConnector {
    transport: Arc<dyn ClientTransport>,
    stop_flag: Arc<AtomicBool>,
    thread_handle: Option<thread::JoinHandle<()>>,
}

impl ClientConnector {
    /// Constructor.
    pub fn new(transport: Box<dyn ClientTransport>) -> Self {
        ClientConnector {
            transport: Arc::from(transport),
            stop_flag: Arc::new(AtomicBool::new(false)),
            thread_handle: None,
        }
    }

    /// Starts the receiver thread.
    pub fn start(&mut self, tx_channel: Sender<Event>) -> Result<(), String> {
        info!("starting");

        let transport = Arc::clone(&self.transport);
        let stop_flag = Arc::clone(&self.stop_flag);

        self.thread_handle = Some(thread::spawn(move || {
            debug!("receive thread started");
            // Initialize transport.
            if let Err(e) = transport.on_start() {
                error!("failed to start transport: {}", e);
                return;
            }
            // Receive loop.
            loop {
                match transport.receive(&stop_flag) {
                    Ok(Some(msg)) => {
                        trace!(
                            "<RECV> CID:{:03} MID:{} ({:?})",
                            msg.cid, msg.mid, msg.mtype
                        );
                        if let Err(e) = tx_channel.send(Event::ClientMsg(msg)) {
                            error!("failed to send event to manager: {:?}", e);
                            break;
                        }
                    }
                    Ok(None) => break, // Stop request detected.
                    Err(e) => {
                        error!("receive error: {}", e);
                        // TODO: handle error/reconnect.
                        break;
                    }
                }
            }
            // Cleanup.
            transport.on_shutdown();
            debug!("receive thread stopped");
        }));

        Ok(())
    }

    /// Stops the receiver thread.
    pub fn stop(&mut self) {
        if let Some(handle) = self.thread_handle.take() {
            info!("stop requested");
            self.stop_flag.store(true, Ordering::Relaxed);
            handle.join().unwrap();
            info!("stopped");
            self.stop_flag.store(false, Ordering::Relaxed);
        } else {
            warn!("not started");
        }
    }

    /// Sends responses to clients.
    pub fn send_responses(&self, msgs: Vec<Message>) {
        for msg in &msgs {
            trace!(
                "<SEND> CID:{:03} MID:{} ({:?})",
                msg.cid, msg.mid, msg.mtype
            );
        }
        if let Err(e) = self.transport.send_all(msgs) {
            error!("failed to send responses: {}", e);
        }
    }
}
