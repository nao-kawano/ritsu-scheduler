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
#[cfg(test)]
use super::*;

#[test]
fn test_message_type_to_str() {
    assert_eq!(MessageType::Join.as_str(), "JOIN");
    assert_eq!(MessageType::Ready.as_str(), "READY");
    assert_eq!(MessageType::Done.as_str(), "DONE");
    assert_eq!(MessageType::Exit.as_str(), "EXIT");
    assert_eq!(MessageType::Joined.as_str(), "JOINED");
    assert_eq!(MessageType::Start.as_str(), "START");
    assert_eq!(MessageType::Ok.as_str(), "OK");
    assert_eq!(MessageType::Skip.as_str(), "SKIP");
    assert_eq!(MessageType::Late.as_str(), "LATE");
    assert_eq!(MessageType::Error.as_str(), "ERROR");
}

#[test]
fn test_message_type_from_str() {
    assert_eq!(MessageType::from_str("JOIN"), Ok(MessageType::Join));
    assert_eq!(MessageType::from_str("READY"), Ok(MessageType::Ready));
    assert_eq!(MessageType::from_str("DONE"), Ok(MessageType::Done));
    assert_eq!(MessageType::from_str("EXIT"), Ok(MessageType::Exit));
    assert_eq!(MessageType::from_str("JOINED"), Ok(MessageType::Joined));
    assert_eq!(MessageType::from_str("START"), Ok(MessageType::Start));
    assert_eq!(MessageType::from_str("OK"), Ok(MessageType::Ok));
    assert_eq!(MessageType::from_str("SKIP"), Ok(MessageType::Skip));
    assert_eq!(MessageType::from_str("LATE"), Ok(MessageType::Late));
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
    let message_id = 0;
    let client_id = 10;

    let result = Message::new(message_type, message_id, client_id, None);

    assert!(result.is_ok());
    let message = result.unwrap();
    assert_eq!(message.mtype, MessageType::Ready);
    assert_eq!(message.mid, 0);
    assert_eq!(message.cid, 10);
    assert_eq!(message.extras, Vec::<(String, String)>::new());
}

#[test]
fn test_message_new_with_extras() {
    let message_type = MessageType::Ready;
    let message_id = 1;
    let client_id = 10;
    let extras = Some(vec![
        ("key1".to_string(), "val1".to_string()),
        ("key2".to_string(), "val2".to_string()),
    ]);

    let result = Message::new(message_type, message_id, client_id, extras);

    assert!(result.is_ok());
    let message = result.unwrap();
    assert_eq!(message.mtype, MessageType::Ready);
    assert_eq!(message.cid, 10);
    assert_eq!(
        message.extras,
        vec![
            ("key1".to_string(), "val1".to_string()),
            ("key2".to_string(), "val2".to_string()),
        ]
    );
}

#[test]
fn test_message_new_invalid_message_id() {
    let message_type = MessageType::Done;
    let message_id = 10;
    let client_id = 10;
    let extras = Some(vec![("k".to_string(), "v".to_string())]);

    let result = Message::new(message_type, message_id, client_id, extras);

    assert!(result.is_err());
    assert_eq!(result.unwrap_err(), ParseError::InvalidMessageId);
}

#[test]
fn test_message_new_invalid_client_id() {
    let message_type = MessageType::Done;
    let message_id = 1;
    let client_id = 1000;
    let extras = Some(vec![("k".to_string(), "v".to_string())]);

    let result = Message::new(message_type, message_id, client_id, extras);

    assert!(result.is_err());
    assert_eq!(result.unwrap_err(), ParseError::InvalidClientId);
}

#[test]
fn test_message_get_set_extra() {
    let mut message = Message::new(MessageType::Ready, 0, 10, None).unwrap();
    assert_eq!(message.get_extra("key1"), None);

    message.set_extra("key1", "val1");
    assert_eq!(message.get_extra("key1"), Some(&"val1".to_string()));

    message.set_extra("key1", "val2");
    assert_eq!(message.get_extra("key1"), Some(&"val2".to_string()));

    message.set_extra("key2", "val3");
    assert_eq!(message.get_extra("key2"), Some(&"val3".to_string()));
    assert_eq!(message.extras.len(), 2);
}

#[test]
fn test_message_from_str_no_extras() {
    {
        let msg = "READY@0:100";
        let msg = Message::from_str(msg);
        assert!(msg.is_ok());

        let msg = msg.unwrap();
        assert_eq!(msg.mtype, MessageType::Ready);
        assert_eq!(msg.mid, 0);
        assert_eq!(msg.cid, 100);
        assert_eq!(msg.extras.len(), 0);
    }
    {
        let msg = "READY@9:100,";
        let msg = Message::from_str(msg);
        assert!(msg.is_ok());

        let msg = msg.unwrap();
        assert_eq!(msg.mtype, MessageType::Ready);
        assert_eq!(msg.mid, 9);
        assert_eq!(msg.cid, 100);
        assert_eq!(msg.extras.len(), 0); // Trailing comma should not result in an empty string extra.
    }
}

#[test]
fn test_message_from_str_with_extras() {
    let msg = "READY@1:000,key1=val1,key2=val2,key3";
    let msg = Message::from_str(msg);
    assert!(msg.is_ok());

    let msg = msg.unwrap();
    assert_eq!(msg.mtype, MessageType::Ready);
    assert_eq!(msg.mid, 1);
    assert_eq!(msg.cid, 0);
    assert_eq!(msg.extras.len(), 3);
    assert_eq!(msg.extras[0], ("key1".to_string(), "val1".to_string()));
    assert_eq!(msg.extras[1], ("key2".to_string(), "val2".to_string()));
    assert_eq!(msg.extras[2], ("key3".to_string(), "".to_string()));
}

#[test]
fn test_message_from_str_errors() {
    {
        // Missing colon (TypeNotFound as split_once(":") fails)
        let msg = "abcdefg";
        let msg = Message::from_str(msg);
        assert!(msg.is_err());
        assert_eq!(msg.unwrap_err(), ParseError::TypeNotFound);
    }
    {
        // Missing colon (TypeNotFound as split_once(":") fails)
        let msg = "READY";
        let msg = Message::from_str(msg);
        assert!(msg.is_err());
        assert_eq!(msg.unwrap_err(), ParseError::TypeNotFound);
    }
    {
        // Unknown message type
        let msg = "UNKNOWN@1:000";
        let msg = Message::from_str(msg);
        assert!(msg.is_err());
        assert_eq!(msg.unwrap_err(), ParseError::TypeNotFound);
    }
    {
        // Missing @ (MessageIdNotFound)
        let msg = "xx:";
        let msg = Message::from_str(msg);
        assert!(msg.is_err());
        assert_eq!(msg.unwrap_err(), ParseError::MessageIdNotFound);
    }
    {
        // Missing @ (MessageIdNotFound)
        let msg = "READY:";
        let msg = Message::from_str(msg);
        assert!(msg.is_err());
        assert_eq!(msg.unwrap_err(), ParseError::MessageIdNotFound);
    }
    {
        // Invalid Message ID length (empty)
        let msg = "READY@:000";
        let msg = Message::from_str(msg);
        assert!(msg.is_err());
        assert_eq!(msg.unwrap_err(), ParseError::InvalidMessageId);
    }
    {
        // Invalid Message ID length (too long)
        let msg = "READY@12:000";
        let msg = Message::from_str(msg);
        assert!(msg.is_err());
        assert_eq!(msg.unwrap_err(), ParseError::InvalidMessageId);
    }
    {
        // Invalid Message ID (not a number)
        let msg = "READY@X:000";
        let msg = Message::from_str(msg);
        assert!(msg.is_err());
        assert_eq!(msg.unwrap_err(), ParseError::InvalidMessageId);
    }
    {
        // Missing Client ID part
        let msg = "READY@1:";
        let msg = Message::from_str(msg);
        assert!(msg.is_err());
        assert_eq!(msg.unwrap_err(), ParseError::InvalidClientId);
    }
    {
        // Invalid Client ID length (too short)
        let msg = "READY@1:1";
        let msg = Message::from_str(msg);
        assert!(msg.is_err());
        assert_eq!(msg.unwrap_err(), ParseError::InvalidClientId);
    }
    {
        // Invalid Client ID length (too long)
        let msg = "READY@1:1234";
        let msg = Message::from_str(msg);
        assert!(msg.is_err());
        assert_eq!(msg.unwrap_err(), ParseError::InvalidClientId);
    }
    {
        // Invalid Client ID (not a number)
        let msg = "READY@1:abc";
        let msg = Message::from_str(msg);
        assert!(msg.is_err());
        assert_eq!(msg.unwrap_err(), ParseError::InvalidClientId);
    }
    {
        // Message too long
        let msg = format!("READY@1:000,{}", "0".repeat(MESSAGE_LEN_MAX + 1));
        let msg = Message::from_str(&msg);
        assert!(msg.is_err());
        assert_eq!(msg.unwrap_err(), ParseError::MessageTooLong);
    }
}

#[test]
fn test_message_to_str_no_extras() {
    let message = Message {
        mtype: MessageType::Ready,
        mid: 0,
        cid: 123,
        extras: vec![],
    };
    assert_eq!(message.to_str().unwrap(), "READY@0:123".to_string());
}

#[test]
fn test_message_to_str_with_extras() {
    let message = Message {
        mtype: MessageType::Done,
        mid: 1,
        cid: 456,
        extras: vec![
            ("key1".to_string(), "val1".to_string()),
            ("key2".to_string(), "val2".to_string()),
            ("key3".to_string(), "".to_string()),
        ],
    };
    assert_eq!(
        message.to_str().unwrap(),
        "DONE@1:456,key1=val1,key2=val2,key3".to_string()
    );
}

#[test]
fn test_message_to_str_invalid_msg_id() {
    let message = Message {
        mtype: MessageType::Exit,
        mid: 10,
        cid: 123,
        extras: vec![],
    };
    assert!(message.to_str().is_err());
    assert_eq!(message.to_str().unwrap_err(), ParseError::InvalidMessageId);
}

#[test]
fn test_message_to_str_invalid_client_id() {
    let message = Message {
        mtype: MessageType::Exit,
        mid: 9,
        cid: 1000,
        extras: vec![],
    };
    assert!(message.to_str().is_err());
    assert_eq!(message.to_str().unwrap_err(), ParseError::InvalidClientId);
}

#[test]
fn test_message_to_str_too_long() {
    let message = Message {
        mtype: MessageType::Exit,
        mid: 0,
        cid: 999,
        extras: vec![("k".to_string(), "a".to_string().repeat(MESSAGE_LEN_MAX + 1))],
    };
    assert!(message.to_str().is_err());
    assert_eq!(message.to_str().unwrap_err(), ParseError::MessageTooLong);
}
