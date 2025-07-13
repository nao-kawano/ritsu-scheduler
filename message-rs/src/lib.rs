//!
//! Message types for DPS.
//!

#[cfg(test)]
#[path = "lib_test.rs"]
mod lib_test;

/* -------------------------------------------------------------------------- */

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
    InvalidType,
    ClientIdNotFound,
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
    /// Client ID.
    pub cid: u16,
    /// Extra Info.
    pub extras: Vec<String>,
}

impl Message {
    /// Create a new message.
    pub fn new(
        message_type: MessageType,
        client_id: u16,
        extras: Option<Vec<String>>,
    ) -> Result<Self, ParseError> {
        if client_id > CLIENT_ID_MAX {
            Err(ParseError::InvalidClientId)
        } else {
            Ok(Message {
                mtype: message_type,
                cid: client_id,
                extras: extras.unwrap_or(vec![]),
            })
        }
    }

    /// Create a new message from a string.
    pub fn from_msg(msg: &str) -> Result<Self, ParseError> {
        if msg.len() > MESSAGE_LEN_MAX {
            return Err(ParseError::MessageTooLong);
        }
        let (msg_type_str, msg_payload_str) =
            msg.split_once(":").ok_or(ParseError::TypeNotFound)?;

        // check type.
        let message_type: MessageType = MessageType::from_str(msg_type_str)?;

        // check client id.
        let msg_payload_str_vec: Vec<&str> = msg_payload_str.split(",").collect();
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
            cid: client_id,
            extras,
        })
    }

    /// Convert to a string.
    pub fn to_msg(&self) -> Result<String, ParseError> {
        if self.cid > CLIENT_ID_MAX {
            return Err(ParseError::InvalidClientId);
        }

        let msg: String;
        if self.extras.len() < 1 {
            msg = format!("{}:{:>03}", self.mtype.to_str(), self.cid);
        } else {
            msg = format!(
                "{}:{:>03},{}",
                self.mtype.to_str(),
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
