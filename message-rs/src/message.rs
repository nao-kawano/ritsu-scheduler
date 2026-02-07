//!
//! Message for DPS.
//!

#[cfg(test)]
#[path = "message_test.rs"]
mod message_test;

/* -------------------------------------------------------------------------- */

/// Message ID length in string.
pub const MSG_ID_LEN: usize = 1;
/// Maximum Message ID in number.
pub const MSG_ID_MAX: u8 = 9;
/// Client ID length in string.
pub const CLIENT_ID_LEN: usize = 3;
/// Maximum client ID in number.
pub const CLIENT_ID_MAX: u16 = 999;
/// Maximum message length in bytes.
pub const MESSAGE_LEN_MAX: usize = 512;

/* -------------------------------------------------------------------------- */

/// Parse error types.
#[derive(Debug, PartialEq, Eq, Clone, Copy)]
pub enum ParseError {
    TypeNotFound,
    MessageIdNotFound,
    InvalidMessageId,
    InvalidClientId,
    MessageTooLong,
}

/* -------------------------------------------------------------------------- */

/// Message types.
#[derive(Debug, PartialEq, Eq, Clone, Copy)]
pub enum MessageType {
    Join,
    Ready,
    Done,
    Exit,
    Ok,
    Skip,
    Late,
    Error,
}

impl MessageType {
    /// Convert message type to string.
    pub fn to_str(&self) -> String {
        match self {
            MessageType::Join => "JOIN".to_string(),
            MessageType::Ready => "READY".to_string(),
            MessageType::Done => "DONE".to_string(),
            MessageType::Exit => "EXIT".to_string(),
            MessageType::Ok => "OK".to_string(),
            MessageType::Skip => "SKIP".to_string(),
            MessageType::Late => "LATE".to_string(),
            MessageType::Error => "ERROR".to_string(),
        }
    }

    /// Convert string to message type.
    pub fn from_str(s: &str) -> Result<Self, ParseError> {
        match s {
            "JOIN" => Ok(MessageType::Join),
            "READY" => Ok(MessageType::Ready),
            "DONE" => Ok(MessageType::Done),
            "EXIT" => Ok(MessageType::Exit),
            "OK" => Ok(MessageType::Ok),
            "SKIP" => Ok(MessageType::Skip),
            "LATE" => Ok(MessageType::Late),
            "ERROR" => Ok(MessageType::Error),
            _ => Err(ParseError::TypeNotFound),
        }
    }
}

/* -------------------------------------------------------------------------- */

/// Message for DPS.
#[derive(Debug, Clone)]
pub struct Message {
    /// Message Type.
    pub mtype: MessageType,
    /// Message ID.
    pub mid: u8,
    /// Client ID.
    pub cid: u16,
    /// Extra Info.
    pub extras: Vec<String>,
}

impl Message {
    /// Create a new message.
    pub fn new(
        message_type: MessageType,
        message_id: u8,
        client_id: u16,
        extras: Option<Vec<String>>,
    ) -> Result<Self, ParseError> {
        if client_id > CLIENT_ID_MAX {
            return Err(ParseError::InvalidClientId);
        }
        if message_id > MSG_ID_MAX {
            return Err(ParseError::InvalidMessageId);
        }
        Ok(Message {
            mtype: message_type,
            mid: message_id,
            cid: client_id,
            extras: extras.unwrap_or(vec![]),
        })
    }

    /// Create a new message from a string.
    pub fn from_str(msg: &str) -> Result<Self, ParseError> {
        if msg.len() > MESSAGE_LEN_MAX {
            return Err(ParseError::MessageTooLong);
        }
        let (msg_header_str, msg_payload_str) =
            msg.split_once(":").ok_or(ParseError::TypeNotFound)?;

        // -- header
        let (msg_type_str, msg_id_str) = msg_header_str
            .split_once("@")
            .ok_or(ParseError::MessageIdNotFound)?;

        // check message type.
        let message_type: MessageType = MessageType::from_str(msg_type_str)?;
        // check message id.
        if msg_id_str.len() != MSG_ID_LEN {
            return Err(ParseError::InvalidMessageId);
        }
        let message_id: u8 = msg_id_str
            .parse::<u8>()
            .ok() // Result to Option
            .ok_or(ParseError::InvalidMessageId)?;

        // -- payload
        let msg_payload_str_vec: Vec<&str> = msg_payload_str.split(",").collect();

        // check client id.
        if msg_payload_str_vec[0].len() != CLIENT_ID_LEN {
            return Err(ParseError::InvalidClientId);
        }
        let client_id: u16 = msg_payload_str_vec[0]
            .parse::<u16>()
            .ok() // Result to Option
            .ok_or(ParseError::InvalidClientId)?;

        // check extras.
        let extras: Vec<String>;
        if msg_payload_str_vec.len() == 1 {
            extras = vec![];
        } else {
            extras = msg_payload_str_vec[1..]
                .iter()
                .map(|s| s.to_string())
                .collect();
        }

        Ok(Message {
            mtype: message_type,
            mid: message_id,
            cid: client_id,
            extras,
        })
    }

    /// Convert to a string.
    pub fn to_str(&self) -> Result<String, ParseError> {
        if self.mid > MSG_ID_MAX {
            return Err(ParseError::InvalidMessageId);
        }
        if self.cid > CLIENT_ID_MAX {
            return Err(ParseError::InvalidClientId);
        }

        let msg: String;
        if self.extras.len() < 1 {
            msg = format!("{}@{:1}:{:>03}", self.mtype.to_str(), self.mid, self.cid);
        } else {
            msg = format!(
                "{}@{:1}:{:>03},{}",
                self.mtype.to_str(),
                self.mid,
                self.cid,
                self.extras.join(",")
            );
        }

        if msg.len() > MESSAGE_LEN_MAX {
            return Err(ParseError::MessageTooLong);
        }

        Ok(msg)
    }
}
