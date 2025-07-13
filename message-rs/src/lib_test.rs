#[cfg(test)]
use super::*;

#[test]
fn test_message_type_to_str() {
    assert_eq!(MessageType::Join.to_str(), "JOIN".to_string());
    assert_eq!(MessageType::Ready.to_str(), "READY".to_string());
    assert_eq!(MessageType::Done.to_str(), "DONE".to_string());
    assert_eq!(MessageType::Exit.to_str(), "EXIT".to_string());
    assert_eq!(MessageType::Ok.to_str(), "OK".to_string());
    assert_eq!(MessageType::Skip.to_str(), "SKIP".to_string());
    assert_eq!(MessageType::Error.to_str(), "ERROR".to_string());
}

#[test]
fn test_message_type_from_str() {
    assert_eq!(MessageType::from_str("JOIN"), Ok(MessageType::Join));
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
    assert_eq!(message.mtype, MessageType::Ready);
    assert_eq!(message.cid, 10);
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
    assert_eq!(message.mtype, MessageType::Ready);
    assert_eq!(message.cid, 10);
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
fn test_message_from_str_no_extras() {
    {
        let msg = "READY:100";
        let msg = Message::from_str(msg);
        assert!(msg.is_ok());

        let msg = msg.unwrap();
        assert_eq!(msg.mtype, MessageType::Ready);
        assert_eq!(msg.cid, 100);
        assert_eq!(msg.extras.len(), 0);
    }
    {
        let msg = "READY:100,";
        let msg = Message::from_str(msg);
        assert!(msg.is_ok());

        let msg = msg.unwrap();
        assert_eq!(msg.mtype, MessageType::Ready);
        assert_eq!(msg.cid, 100);
        assert_eq!(msg.extras.len(), 1);
        assert_eq!(msg.extras[0], String::new());
    }
}

#[test]
fn test_message_from_str_with_extras() {
    let msg = "READY:000,ex1,ex2";
    let msg = Message::from_str(msg);
    assert!(msg.is_ok());

    let msg = msg.unwrap();
    assert_eq!(msg.mtype, MessageType::Ready);
    assert_eq!(msg.cid, 000);
    assert_eq!(msg.extras.len(), 2);
    assert_eq!(msg.extras[0], "ex1");
    assert_eq!(msg.extras[1], "ex2");
}

#[test]
fn test_message_from_str_errors() {
    {
        let msg = "abcdefg";
        let msg = Message::from_str(msg);
        assert!(msg.is_err());
        assert_eq!(msg.unwrap_err(), ParseError::TypeNotFound);
    }
    {
        let msg = "READY";
        let msg = Message::from_str(msg);
        assert!(msg.is_err());
        assert_eq!(msg.unwrap_err(), ParseError::TypeNotFound);
    }
    {
        let msg = "xx:";
        let msg = Message::from_str(msg);
        assert!(msg.is_err());
        assert_eq!(msg.unwrap_err(), ParseError::TypeNotFound);
    }
    {
        let msg = "READY:";
        let msg = Message::from_str(msg);
        assert!(msg.is_err());
        assert_eq!(msg.unwrap_err(), ParseError::InvalidClientId);
    }
    {
        let msg = "READY:1";
        let msg = Message::from_str(msg);
        assert!(msg.is_err());
        assert_eq!(msg.unwrap_err(), ParseError::InvalidClientId);
    }
    {
        let msg = "READY:1234";
        let msg = Message::from_str(msg);
        assert!(msg.is_err());
        assert_eq!(msg.unwrap_err(), ParseError::InvalidClientId);
    }
    {
        let msg = format!("READY:000,{}", "0".repeat(MESSAGE_LEN_MAX + 1));
        let msg = Message::from_str(&msg);
        assert!(msg.is_err());
        assert_eq!(msg.unwrap_err(), ParseError::MessageTooLong);
    }
}

#[test]
fn test_message_to_str_no_extras() {
    let message = Message {
        mtype: MessageType::Ready,
        cid: 123,
        extras: vec![],
    };
    assert_eq!(message.to_str().unwrap(), "READY:123".to_string());
}

#[test]
fn test_message_to_str_with_extras() {
    let message = Message {
        mtype: MessageType::Done,
        cid: 456,
        extras: vec!["extra1".to_string(), "extra2".to_string()],
    };
    assert_eq!(
        message.to_str().unwrap(),
        "DONE:456,extra1,extra2".to_string()
    );
}

#[test]
fn test_message_to_str_invalid_client_id() {
    let message = Message {
        mtype: MessageType::Exit,
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
        cid: 999,
        extras: vec!["a".to_string().repeat(MESSAGE_LEN_MAX + 1)],
    };
    assert!(message.to_str().is_err());
    assert_eq!(message.to_str().unwrap_err(), ParseError::MessageTooLong);
}
