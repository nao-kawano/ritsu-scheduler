// Copyright 2026 Naoyuki Kawano
//
// Licensed under the Apache License, Version 2.0 (the "License");
// you may not use this file except in compliance with the License.
// You may obtain a copy of the License at
//
//     http://www.apache.org/licenses/LICENSE-2.0
//
// Unless required by applicable law or agreed to in writing, software
// distributed under the License is distributed on an "AS IS" BASIS,
// WITHOUT WARRANTIES OR CONDITIONS OF ANY KIND, either express or implied.
// See the License for the specific language governing permissions and
// limitations under the License.
// =============================================================================
//!
//! Message for Ritsu.
//!

use std::fmt;

#[cfg(test)]
#[path = "message_test.rs"]
mod message_test;

/* -------------------------------------------------------------------------- */

/// Protocol version.
pub const PROTOCOL_VERSION: &str = "1";

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
    // Client Requests
    Join,
    Ready,
    Done,
    Exit,
    // Server Responses
    Joined,
    Start,
    Ok,
    Skip,
    Late,
    Error,
}

impl MessageType {
    /// Convert message type to string.
    pub fn as_str(&self) -> &'static str {
        match self {
            MessageType::Join => "JOIN",
            MessageType::Ready => "READY",
            MessageType::Done => "DONE",
            MessageType::Exit => "EXIT",
            MessageType::Joined => "JOINED",
            MessageType::Start => "START",
            MessageType::Ok => "OK",
            MessageType::Skip => "SKIP",
            MessageType::Late => "LATE",
            MessageType::Error => "ERROR",
        }
    }

    /// Convert string to message type.
    pub fn from_str(s: &str) -> Result<Self, ParseError> {
        match s {
            "JOIN" => Ok(MessageType::Join),
            "READY" => Ok(MessageType::Ready),
            "DONE" => Ok(MessageType::Done),
            "EXIT" => Ok(MessageType::Exit),
            "JOINED" => Ok(MessageType::Joined),
            "START" => Ok(MessageType::Start),
            "OK" => Ok(MessageType::Ok),
            "SKIP" => Ok(MessageType::Skip),
            "LATE" => Ok(MessageType::Late),
            "ERROR" => Ok(MessageType::Error),
            _ => Err(ParseError::TypeNotFound),
        }
    }
}

impl fmt::Display for MessageType {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(self.as_str())
    }
}

/* -------------------------------------------------------------------------- */

/// Message for Ritsu.
#[derive(Debug, Clone)]
pub struct Message {
    /// Message Type.
    pub mtype: MessageType,
    /// Message ID.
    pub mid: u8,
    /// Client ID.
    pub cid: u16,
    /// Extra Info.
    pub extras: Vec<(String, String)>,
}

impl Message {
    /// Create a new message.
    pub fn new(
        message_type: MessageType,
        message_id: u8,
        client_id: u16,
        extras: Option<Vec<(String, String)>>,
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
            extras: extras.unwrap_or_default(),
        })
    }

    /// Get extra value by key.
    pub fn get_extra(&self, key: &str) -> Option<&String> {
        self.extras.iter().find(|(k, _)| k == key).map(|(_, v)| v)
    }

    /// Set extra value by key.
    pub fn set_extra(&mut self, key: &str, value: &str) {
        if let Some(pos) = self.extras.iter().position(|(k, _)| k == key) {
            self.extras[pos].1 = value.to_string();
        } else {
            self.extras.push((key.to_string(), value.to_string()));
        }
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
        let mut msg_payload_parts = msg_payload_str.split(",");

        // check client id.
        let client_id_str = msg_payload_parts
            .next()
            .ok_or(ParseError::InvalidClientId)?;
        if client_id_str.len() != CLIENT_ID_LEN {
            return Err(ParseError::InvalidClientId);
        }
        let client_id: u16 = client_id_str
            .parse::<u16>()
            .ok() // Result to Option
            .ok_or(ParseError::InvalidClientId)?;

        // check extras.
        let mut extras = Vec::new();
        for extra_str in msg_payload_parts {
            if extra_str.is_empty() {
                continue;
            }
            if let Some((k, v)) = extra_str.split_once("=") {
                extras.push((k.to_string(), v.to_string()));
            } else {
                extras.push((extra_str.to_string(), "".to_string()));
            }
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

        let mut msg = String::with_capacity(MESSAGE_LEN_MAX);
        use fmt::Write;
        write!(
            msg,
            "{}@{:1}:{:>03}",
            self.mtype.as_str(),
            self.mid,
            self.cid
        )
        .unwrap();

        for (k, v) in &self.extras {
            msg.push(',');
            if v.is_empty() {
                msg.push_str(k);
            } else {
                msg.push_str(k);
                msg.push('=');
                msg.push_str(v);
            }
        }

        if msg.len() > MESSAGE_LEN_MAX {
            return Err(ParseError::MessageTooLong);
        }

        Ok(msg)
    }
}
