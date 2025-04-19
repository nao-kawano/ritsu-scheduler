//!
//! Message types for DPS.
//!

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
    pub message_type: MessageType,
    pub client_id: u16,
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
                message_type,
                client_id,
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
            message_type,
            client_id,
            extras,
        })
    }

    /// Convert to a string.
    pub fn to_msg(&self) -> Result<String, ParseError> {
        if self.client_id > CLIENT_ID_MAX {
            return Err(ParseError::InvalidClientId);
        }

        let msg: String;
        if self.extras.len() < 1 {
            msg = format!("{}:{:>03}", self.message_type.to_str(), self.client_id);
        } else {
            msg = format!(
                "{}:{:>03},{}",
                self.message_type.to_str(),
                self.client_id,
                self.extras.join(",")
            );
        }

        if msg.len() > MESSAGE_LEN_MAX {
            return Err(ParseError::MessageTooLong);
        }

        Ok(msg)
    }
}

/* ============================================================================================== */

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_message_type_to_str() {
        assert_eq!(MessageType::Ready.to_str(), "READY".to_string());
        assert_eq!(MessageType::Done.to_str(), "DONE".to_string());
        assert_eq!(MessageType::Exit.to_str(), "EXIT".to_string());
        assert_eq!(MessageType::Ok.to_str(), "OK".to_string());
        assert_eq!(MessageType::Skip.to_str(), "SKIP".to_string());
        assert_eq!(MessageType::Error.to_str(), "ERROR".to_string());
    }

    #[test]
    fn test_message_type_from_str() {
        assert_eq!(MessageType::from_str("READY"), Ok(MessageType::Ready));
        assert_eq!(MessageType::from_str("DONE"), Ok(MessageType::Done));
        assert_eq!(MessageType::from_str("EXIT"), Ok(MessageType::Exit));
        assert_eq!(MessageType::from_str("OK"), Ok(MessageType::Ok));
        assert_eq!(MessageType::from_str("SKIP"), Ok(MessageType::Skip));
        assert_eq!(MessageType::from_str("ERROR"), Ok(MessageType::Error));
        assert_eq!(
            MessageType::from_str("INVALID"),
            Err(ParseError::TypeNotFound)
        );
    }

    /* ---------------------------------------------------------------------- */

    #[test]
    fn test_message_new_no_extras() {
        let message_type = MessageType::Ready;
        let client_id = 10;

        let result = Message::new(message_type, client_id, None);

        assert!(result.is_ok());
        let message = result.unwrap();
        assert_eq!(message.message_type, MessageType::Ready);
        assert_eq!(message.client_id, 10);
        assert_eq!(message.extras, Vec::<String>::new());
    }

    #[test]
    fn test_message_new_with_extras() {
        let message_type = MessageType::Ready;
        let client_id = 10;
        let extras = Some(vec!["extra1".to_string(), "extra2".to_string()]);

        let result = Message::new(message_type, client_id, extras);

        assert!(result.is_ok());
        let message = result.unwrap();
        assert_eq!(message.message_type, MessageType::Ready);
        assert_eq!(message.client_id, 10);
        assert_eq!(
            message.extras,
            vec!["extra1".to_string(), "extra2".to_string()]
        );
    }

    #[test]
    fn test_message_new_invalid_client_id() {
        let message_type = MessageType::Done;
        let client_id = 1000;
        let extras = Some(vec!["extra1".to_string(), "extra2".to_string()]);

        let result = Message::new(message_type, client_id, extras);

        assert!(result.is_err());
        assert_eq!(result.unwrap_err(), ParseError::InvalidClientId);
    }

    #[test]
    fn test_message_from_msg_no_extras() {
        {
            let msg = "READY:100";
            let msg = Message::from_msg(msg);
            assert!(msg.is_ok());

            let msg = msg.unwrap();
            assert_eq!(msg.message_type, MessageType::Ready);
            assert_eq!(msg.client_id, 100);
            assert_eq!(msg.extras.len(), 0);
        }
        {
            let msg = "READY:100,";
            let msg = Message::from_msg(msg);
            assert!(msg.is_ok());

            let msg = msg.unwrap();
            assert_eq!(msg.message_type, MessageType::Ready);
            assert_eq!(msg.client_id, 100);
            assert_eq!(msg.extras.len(), 1);
            assert_eq!(msg.extras[0], String::new());
        }
    }

    #[test]
    fn test_message_from_msg_with_extras() {
        let msg = "READY:000,ex1,ex2";
        let msg = Message::from_msg(msg);
        assert!(msg.is_ok());

        let msg = msg.unwrap();
        assert_eq!(msg.message_type, MessageType::Ready);
        assert_eq!(msg.client_id, 000);
        assert_eq!(msg.extras.len(), 2);
        assert_eq!(msg.extras[0], "ex1");
        assert_eq!(msg.extras[1], "ex2");
    }

    #[test]
    fn test_message_from_msg_errors() {
        {
            let msg = "abcdefg";
            let msg = Message::from_msg(msg);
            assert!(msg.is_err());
            assert_eq!(msg.unwrap_err(), ParseError::TypeNotFound);
        }
        {
            let msg = "READY";
            let msg = Message::from_msg(msg);
            assert!(msg.is_err());
            assert_eq!(msg.unwrap_err(), ParseError::TypeNotFound);
        }
        {
            let msg = "xx:";
            let msg = Message::from_msg(msg);
            assert!(msg.is_err());
            assert_eq!(msg.unwrap_err(), ParseError::TypeNotFound);
        }
        {
            let msg = "READY:";
            let msg = Message::from_msg(msg);
            assert!(msg.is_err());
            assert_eq!(msg.unwrap_err(), ParseError::InvalidClientId);
        }
        {
            let msg = "READY:1";
            let msg = Message::from_msg(msg);
            assert!(msg.is_err());
            assert_eq!(msg.unwrap_err(), ParseError::InvalidClientId);
        }
        {
            let msg = "READY:1234";
            let msg = Message::from_msg(msg);
            assert!(msg.is_err());
            assert_eq!(msg.unwrap_err(), ParseError::InvalidClientId);
        }
        {
            let msg = format!("READY:000,{}", "0".repeat(MESSAGE_LEN_MAX + 1));
            let msg = Message::from_msg(&msg);
            assert!(msg.is_err());
            assert_eq!(msg.unwrap_err(), ParseError::MessageTooLong);
        }
    }

    #[test]
    fn test_message_to_msg_no_extras() {
        let message = Message {
            message_type: MessageType::Ready,
            client_id: 123,
            extras: vec![],
        };
        assert_eq!(message.to_msg().unwrap(), "READY:123".to_string());
    }

    #[test]
    fn test_message_to_msg_with_extras() {
        let message = Message {
            message_type: MessageType::Done,
            client_id: 456,
            extras: vec!["extra1".to_string(), "extra2".to_string()],
        };
        assert_eq!(
            message.to_msg().unwrap(),
            "DONE:456,extra1,extra2".to_string()
        );
    }

    #[test]
    fn test_message_to_msg_invalid_client_id() {
        let message = Message {
            message_type: MessageType::Exit,
            client_id: 1000,
            extras: vec![],
        };
        assert!(message.to_msg().is_err());
        assert_eq!(message.to_msg().unwrap_err(), ParseError::InvalidClientId);
    }

    #[test]
    fn test_message_to_msg_too_long() {
        let message = Message {
            message_type: MessageType::Exit,
            client_id: 999,
            extras: vec!["a".to_string().repeat(MESSAGE_LEN_MAX + 1)],
        };
        assert!(message.to_msg().is_err());
        assert_eq!(message.to_msg().unwrap_err(), ParseError::MessageTooLong);
    }
}
