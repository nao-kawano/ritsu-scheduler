//!
//! Event definition.
//!

use rt_message::Message;

#[derive(Debug, Clone)]
pub enum Event {
    Abort,
    CycleStart(u64),
    ClientMsg(Message, std::time::Instant),
}
