//!
//! Event definition.
//!

use rt_message::Message;

use std::time;

#[derive(Debug, Clone)]
pub enum Event {
    Abort,
    CycleStart(u64),
    ClientMsg(Message, time::Instant),
}
