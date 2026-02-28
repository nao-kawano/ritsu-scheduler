//!
//! Message for Ritsu.
//!

mod message;

// export.
pub use message::CLIENT_ID_MAX;
pub use message::MESSAGE_LEN_MAX;
pub use message::MSG_ID_MAX;
pub use message::Message;
pub use message::MessageType;
pub use message::ParseError;
