#[cfg(test)]
use super::*;

use std::sync::{Arc, Mutex};
use std::thread::{self, JoinHandle};

struct MockResponse {
    response: Message,
    delay_sec: f64,
}

fn start_mock_server(
    address: &str,
    mut responses: Vec<MockResponse>,
    received: &Arc<Mutex<Vec<Message>>>,
) -> JoinHandle<()> {
    let mut sock: Option<UdpSocket> = None;
    // setup socket. retry for paralell test.
    const SLEEP_MSEC: u64 = 100;
    for _ in 0..(120 * 1000 / SLEEP_MSEC) {
        let r = UdpSocket::bind(address);
        if r.is_ok() {
            sock = Some(r.unwrap());
            break;
        }
        std::thread::sleep(Duration::from_millis(SLEEP_MSEC));
    }
    if sock.is_none() {
        panic!("MockServer bind failed");
    }

    let sock: UdpSocket = sock.unwrap();
    sock.set_read_timeout(Some(Duration::from_millis(100)))
        .expect("MockServer Failed to set read timeout");
    let received_clone = received.clone();

    // launch server thread.
    let handle = thread::spawn(move || {
        println!("MockServer started with {} response", responses.len());
        let mut buf = [0u8; MESSAGE_LEN_MAX];
        // send response based on pre-defined params.
        for seq in 0..responses.len() {
            match sock.recv_from(&mut buf) {
                Ok((size, addr)) => {
                    // parse request.
                    let message = Message::from_str(str::from_utf8(&buf[..size]).unwrap()).unwrap();
                    let req_mid = message.mid;
                    let req_cid = message.cid;
                    println!("MockServer recv {:?}", message);
                    // store request.
                    {
                        let mut received_clone = received_clone.lock().unwrap();
                        received_clone.push(message);
                    }
                    // response.
                    if let Some(r) = responses.get_mut(seq) {
                        // delay for response.
                        if r.delay_sec > 0.0 {
                            println!("MockServer delay {} sec", r.delay_sec);
                            std::thread::sleep(Duration::from_secs_f64(r.delay_sec));
                        }
                        // send.
                        r.response.mid = req_mid;
                        r.response.cid = req_cid;
                        let _ = sock.send_to(r.response.to_str().unwrap().as_bytes(), addr);
                        println!("MockServer respond {:?}", r.response);
                    }
                }
                Err(e) => {
                    println!("MockServer recv error {}", e);
                    break;
                }
            }
        }
        // collect remaining request, such as retransmission.
        match sock.set_nonblocking(true) {
            Ok(_) => {
                loop {
                    match sock.recv_from(&mut buf) {
                        Ok((size, _addr)) => {
                            // parse request.
                            let message =
                                Message::from_str(str::from_utf8(&buf[..size]).unwrap()).unwrap();
                            println!("MockServer recv remaining {:?}", message);
                            {
                                let mut received_clone = received_clone.lock().unwrap();
                                received_clone.push(message);
                            }
                            continue; // keep reading until the buffer is empty.
                        }
                        Err(_) => {
                            break; // buffer is empty or error.
                        }
                    }
                }
                let _ = sock.set_nonblocking(false);
            }
            Err(_) => {
                // do nothing.
            }
        }
        println!(
            "MockServer exit with {} request",
            received_clone.lock().unwrap().len()
        );
    });

    return handle;
}

#[test]
fn test_join() {
    // create objects.
    let _ = env_logger::builder()
        .is_test(true)
        .format_timestamp_millis()
        .try_init();
    let mut client: DPSClient = DPSClient::new("127.0.0.1".to_string(), 7878, 0, 0.1, 1.0);

    // setup condition.
    let responses: Vec<MockResponse> = vec![
        // Join.
        MockResponse {
            response: Message::new(MessageType::Ok, 0, 0, None).unwrap(),
            delay_sec: 0.0,
        },
    ];
    let requests: Arc<Mutex<Vec<Message>>> = Arc::new(Mutex::new(vec![]));
    let mock_handle = start_mock_server("127.0.0.1:7878", responses, &requests);

    // do.
    let ret = client.join();
    let _ = mock_handle.join();
    // check result.
    assert_eq!(ret, true);
    assert_eq!(client.connected, true);
    assert_eq!(client.startup, true);
    {
        let requests = requests.lock().unwrap();
        assert_eq!(requests.len(), 1);
        assert_eq!(requests[0].mtype, MessageType::Join);
    }
}

#[test]
fn test_join_retry_ok() {
    // create objects.
    let _ = env_logger::builder()
        .is_test(true)
        .format_timestamp_millis()
        .try_init();
    let mut client: DPSClient = DPSClient::new("127.0.0.1".to_string(), 7878, 0, 0.1, 1.0);

    // setup condition.
    let responses: Vec<MockResponse> = vec![
        // Join.
        MockResponse {
            response: Message::new(MessageType::Ok, 0, 0, None).unwrap(),
            delay_sec: client.config.retry_sec_join + 0.005,
        },
    ];
    let requests: Arc<Mutex<Vec<Message>>> = Arc::new(Mutex::new(vec![]));
    let mock_handle = start_mock_server("127.0.0.1:7878", responses, &requests);

    // do.
    let ret = client.join();
    let _ = mock_handle.join();
    // check result.
    assert_eq!(ret, true);
    assert_eq!(client.connected, true);
    assert_eq!(client.startup, true);
    {
        let requests = requests.lock().unwrap();
        assert!(requests.len() > 1);
        assert_eq!(requests[0].mtype, MessageType::Join);
        assert_eq!(requests[1].mtype, MessageType::Join);
    }
}

#[test]
fn test_join_retry_ng() {
    // create objects.
    let _ = env_logger::builder()
        .is_test(true)
        .format_timestamp_millis()
        .try_init();
    let mut client: DPSClient = DPSClient::new("127.0.0.1".to_string(), 7878, 0, 0.1, 1.0);

    // setup condition.
    let responses: Vec<MockResponse> = vec![
        // Join.
        MockResponse {
            response: Message::new(MessageType::Ok, 0, 0, None).unwrap(),
            delay_sec: client.config.retry_sec_join * (client.config.retry_count_join as f64 + 1.0)
                + 0.005,
        },
    ];
    let requests: Arc<Mutex<Vec<Message>>> = Arc::new(Mutex::new(vec![]));
    let mock_handle = start_mock_server("127.0.0.1:7878", responses, &requests);

    // do.
    let ret = client.join();
    let _ = mock_handle.join();
    // check result.
    assert_eq!(ret, false);
    assert_eq!(client.connected, false);
    assert_eq!(client.startup, true);
    {
        let requests = requests.lock().unwrap();
        assert_eq!(
            requests.len(),
            (1 + client.config.retry_count_join) as usize
        );
        assert_eq!(requests[0].mtype, MessageType::Join);
        assert_eq!(requests[1].mtype, MessageType::Join);
    }
}

#[test]
fn test_join_precond() {
    // create objects.
    let _ = env_logger::builder()
        .is_test(true)
        .format_timestamp_millis()
        .try_init();
    let mut client: DPSClient = DPSClient::new("127.0.0.1".to_string(), 7878, 0, 0.1, 1.0);

    // setup condition.
    let responses: Vec<MockResponse> = vec![
        // Join.
        MockResponse {
            response: Message::new(MessageType::Ok, 0, 0, None).unwrap(),
            delay_sec: 0.0,
        },
    ];
    let requests: Arc<Mutex<Vec<Message>>> = Arc::new(Mutex::new(vec![]));
    let mock_handle = start_mock_server("127.0.0.1:7878", responses, &requests);

    // do.
    let _ = client.join();
    let ret = client.join(); // check already connected.
    let _ = mock_handle.join();
    // check result.
    assert_eq!(ret, true);
    assert_eq!(client.connected, true);
    assert_eq!(client.startup, true);
}

#[test]
fn test_ready_startup() {
    // create objects.
    let _ = env_logger::builder()
        .is_test(true)
        .format_timestamp_millis()
        .try_init();
    let mut client: DPSClient = DPSClient::new("127.0.0.1".to_string(), 7878, 0, 0.1, 1.0);
    client.config.retry_sec_ready_startup = 0.1;
    client.config.retry_count_ready_startup = 2;

    // setup condition.
    let responses: Vec<MockResponse> = vec![
        // Join.
        MockResponse {
            response: Message::new(MessageType::Ok, 0, 0, None).unwrap(),
            delay_sec: 0.0,
        },
        // Ready.
        MockResponse {
            response: Message::new(MessageType::Ok, 0, 0, None).unwrap(),
            delay_sec: client.config.retry_sec_ready_startup / 2.0,
        },
    ];
    let requests: Arc<Mutex<Vec<Message>>> = Arc::new(Mutex::new(vec![]));
    let mock_handle = start_mock_server("127.0.0.1:7878", responses, &requests);

    // do.
    let _ = client.join();
    let ret = client.wait_next();
    let _ = mock_handle.join();
    // check result.
    assert_eq!(ret, MessageType::Ok);
    assert_eq!(client.connected, true);
    assert_eq!(client.startup, false);
    {
        let requests = requests.lock().unwrap();
        assert_eq!(requests.len(), 2);
        assert_eq!(requests[0].mtype, MessageType::Join);
        assert_eq!(requests[1].mtype, MessageType::Ready);
    }
}

#[test]
fn test_ready_startup_retry_ok() {
    // create objects.
    let _ = env_logger::builder()
        .is_test(true)
        .format_timestamp_millis()
        .try_init();
    let mut client: DPSClient = DPSClient::new("127.0.0.1".to_string(), 7878, 0, 0.1, 1.0);
    client.config.retry_sec_ready_startup = 0.1;
    client.config.retry_count_ready_startup = 2;

    // setup condition.
    let responses: Vec<MockResponse> = vec![
        // Join.
        MockResponse {
            response: Message::new(MessageType::Ok, 0, 0, None).unwrap(),
            delay_sec: 0.0,
        },
        // Ready.
        MockResponse {
            response: Message::new(MessageType::Ok, 0, 0, None).unwrap(),
            delay_sec: client.config.retry_sec_ready_startup + 0.005,
        },
    ];
    let requests: Arc<Mutex<Vec<Message>>> = Arc::new(Mutex::new(vec![]));
    let mock_handle = start_mock_server("127.0.0.1:7878", responses, &requests);

    // do.
    let _ = client.join();
    let ret = client.wait_next();
    let _ = mock_handle.join();
    // check result.
    assert_eq!(ret, MessageType::Ok);
    assert_eq!(client.connected, true);
    assert_eq!(client.startup, false);
    {
        let requests = requests.lock().unwrap();
        assert!(requests.len() > 1);
        assert_eq!(requests[0].mtype, MessageType::Join);
        assert_eq!(requests[1].mtype, MessageType::Ready);
        assert_eq!(requests[2].mtype, MessageType::Ready);
    }
}

#[test]
fn test_ready_startup_retry_ng() {
    // create objects.
    let _ = env_logger::builder()
        .is_test(true)
        .format_timestamp_millis()
        .try_init();
    let mut client: DPSClient = DPSClient::new("127.0.0.1".to_string(), 7878, 0, 0.1, 1.0);
    client.config.retry_sec_ready_startup = 0.1;
    client.config.retry_count_ready_startup = 2;

    // setup condition.
    let responses: Vec<MockResponse> = vec![
        // Join.
        MockResponse {
            response: Message::new(MessageType::Ok, 0, 0, None).unwrap(),
            delay_sec: 0.0,
        },
        // Ready.
        MockResponse {
            response: Message::new(MessageType::Ok, 0, 0, None).unwrap(),
            delay_sec: client.config.retry_sec_ready_startup
                * (client.config.retry_count_ready_startup as f64 + 1.0)
                + 0.005,
        },
    ];
    let requests: Arc<Mutex<Vec<Message>>> = Arc::new(Mutex::new(vec![]));
    let mock_handle = start_mock_server("127.0.0.1:7878", responses, &requests);

    // do.
    let _ = client.join();
    let ret = client.wait_next();
    let _ = mock_handle.join();
    // check result.
    assert_eq!(ret, MessageType::Error);
    assert_eq!(client.connected, true);
    assert_eq!(client.startup, false);
    {
        let requests = requests.lock().unwrap();
        assert_eq!(
            requests.len(),
            (2 + client.config.retry_count_ready_startup) as usize
        );
        assert_eq!(requests[0].mtype, MessageType::Join);
        assert_eq!(requests[1].mtype, MessageType::Ready);
        assert_eq!(requests[2].mtype, MessageType::Ready);
    }
}

#[test]
#[should_panic]
fn test_ready_startup_precond() {
    // create objects.
    let _ = env_logger::builder()
        .is_test(true)
        .format_timestamp_millis()
        .try_init();
    let mut client: DPSClient = DPSClient::new("127.0.0.1".to_string(), 7878, 0, 0.1, 1.0);
    client.config.retry_sec_ready_startup = 0.1;
    client.config.retry_count_ready_startup = 2;

    // setup condition.
    // do nothing.

    // do.
    let _ = client.wait_next(); // without join.
    // check result.
    // should panic.
}

#[test]
fn test_ready() {
    // create objects.
    let _ = env_logger::builder()
        .is_test(true)
        .format_timestamp_millis()
        .try_init();
    let mut client: DPSClient = DPSClient::new("127.0.0.1".to_string(), 7878, 0, 0.1, 1.0);
    client.config.retry_sec_ready_startup = 0.1;
    client.config.retry_count_ready_startup = 2;

    // setup condition.
    let responses: Vec<MockResponse> = vec![
        // Join.
        MockResponse {
            response: Message::new(MessageType::Ok, 0, 0, None).unwrap(),
            delay_sec: 0.0,
        },
        // Ready.
        MockResponse {
            response: Message::new(MessageType::Ok, 0, 0, None).unwrap(),
            delay_sec: 0.0,
        },
        // Done.
        MockResponse {
            response: Message::new(MessageType::Ok, 0, 0, None).unwrap(),
            delay_sec: 0.0,
        },
        // Ready.
        MockResponse {
            response: Message::new(MessageType::Ok, 0, 0, None).unwrap(),
            delay_sec: client.config.retry_sec_ready / 2.0,
        },
    ];
    let requests: Arc<Mutex<Vec<Message>>> = Arc::new(Mutex::new(vec![]));
    let mock_handle = start_mock_server("127.0.0.1:7878", responses, &requests);

    // do.
    let _ = client.join();
    let _ = client.wait_next();
    let _ = client.notify_done();
    let ret = client.wait_next();
    let _ = mock_handle.join();
    // check result.
    assert_eq!(ret, MessageType::Ok);
    assert_eq!(client.connected, true);
    assert_eq!(client.startup, false);
    {
        let requests = requests.lock().unwrap();
        assert_eq!(requests.len(), 4);
        assert_eq!(requests[0].mtype, MessageType::Join);
        assert_eq!(requests[1].mtype, MessageType::Ready);
        assert_eq!(requests[2].mtype, MessageType::Done);
        assert_eq!(requests[3].mtype, MessageType::Ready);
    }
}

#[test]
fn test_ready_retry_ok() {
    // create objects.
    let _ = env_logger::builder()
        .is_test(true)
        .format_timestamp_millis()
        .try_init();
    let mut client: DPSClient = DPSClient::new("127.0.0.1".to_string(), 7878, 0, 0.1, 1.0);
    client.config.retry_sec_ready_startup = 0.1;
    client.config.retry_count_ready_startup = 2;

    // setup condition.
    let responses: Vec<MockResponse> = vec![
        // Join.
        MockResponse {
            response: Message::new(MessageType::Ok, 0, 0, None).unwrap(),
            delay_sec: 0.0,
        },
        // Ready.
        MockResponse {
            response: Message::new(MessageType::Ok, 0, 0, None).unwrap(),
            delay_sec: 0.0,
        },
        // Done.
        MockResponse {
            response: Message::new(MessageType::Ok, 0, 0, None).unwrap(),
            delay_sec: 0.0,
        },
        // Ready.
        MockResponse {
            response: Message::new(MessageType::Ok, 0, 0, None).unwrap(),
            delay_sec: client.config.retry_sec_ready + 0.005,
        },
    ];
    let requests: Arc<Mutex<Vec<Message>>> = Arc::new(Mutex::new(vec![]));
    let mock_handle = start_mock_server("127.0.0.1:7878", responses, &requests);

    // do.
    let _ = client.join();
    let _ = client.wait_next();
    let _ = client.notify_done();
    let ret = client.wait_next();
    let _ = mock_handle.join();
    // check result.
    assert_eq!(ret, MessageType::Ok);
    assert_eq!(client.connected, true);
    assert_eq!(client.startup, false);
    {
        let requests = requests.lock().unwrap();
        assert!(requests.len() > 4);
        assert_eq!(requests[0].mtype, MessageType::Join);
        assert_eq!(requests[1].mtype, MessageType::Ready);
        assert_eq!(requests[2].mtype, MessageType::Done);
        assert_eq!(requests[3].mtype, MessageType::Ready);
    }
}

#[test]
fn test_ready_retry_ng() {
    // create objects.
    let _ = env_logger::builder()
        .is_test(true)
        .format_timestamp_millis()
        .try_init();
    let mut client: DPSClient = DPSClient::new("127.0.0.1".to_string(), 7878, 0, 0.1, 1.0);
    client.config.retry_sec_ready_startup = 0.1;
    client.config.retry_count_ready_startup = 2;

    // setup condition.
    let responses: Vec<MockResponse> = vec![
        // Join.
        MockResponse {
            response: Message::new(MessageType::Ok, 0, 0, None).unwrap(),
            delay_sec: 0.0,
        },
        // Ready.
        MockResponse {
            response: Message::new(MessageType::Ok, 0, 0, None).unwrap(),
            delay_sec: 0.0,
        },
        // Done.
        MockResponse {
            response: Message::new(MessageType::Ok, 0, 0, None).unwrap(),
            delay_sec: 0.0,
        },
        // Ready.
        MockResponse {
            response: Message::new(MessageType::Ok, 0, 0, None).unwrap(),
            delay_sec: client.config.retry_sec_ready
                * (client.config.retry_count_ready as f64 + 1.0)
                + 0.005,
        },
    ];
    let requests: Arc<Mutex<Vec<Message>>> = Arc::new(Mutex::new(vec![]));
    let mock_handle = start_mock_server("127.0.0.1:7878", responses, &requests);

    // do.
    let _ = client.join();
    let _ = client.wait_next();
    let _ = client.notify_done();
    let ret = client.wait_next();
    let _ = mock_handle.join();
    // check result.
    assert_eq!(ret, MessageType::Error);
    assert_eq!(client.connected, true);
    assert_eq!(client.startup, false);
    {
        let requests = requests.lock().unwrap();
        assert_eq!(
            requests.len(),
            (4 + client.config.retry_count_ready) as usize
        );
        assert_eq!(requests[0].mtype, MessageType::Join);
        assert_eq!(requests[1].mtype, MessageType::Ready);
        assert_eq!(requests[2].mtype, MessageType::Done);
        assert_eq!(requests[3].mtype, MessageType::Ready);
    }
}

#[test]
fn test_done_retry_ng() {
    // create objects.
    let _ = env_logger::builder()
        .is_test(true)
        .format_timestamp_millis()
        .try_init();
    let mut client: DPSClient = DPSClient::new("127.0.0.1".to_string(), 7878, 0, 0.1, 1.0);

    // setup condition.
    let responses: Vec<MockResponse> = vec![
        // Join.
        MockResponse {
            response: Message::new(MessageType::Ok, 0, 0, None).unwrap(),
            delay_sec: 0.0,
        },
        // Ready.
        MockResponse {
            response: Message::new(MessageType::Ok, 0, 0, None).unwrap(),
            delay_sec: 0.0,
        },
        // Done.
        MockResponse {
            response: Message::new(MessageType::Ok, 0, 0, None).unwrap(),
            delay_sec: client.config.retry_sec_done * (client.config.retry_count_done as f64 + 1.0)
                + 0.005,
        },
    ];
    let requests: Arc<Mutex<Vec<Message>>> = Arc::new(Mutex::new(vec![]));
    let mock_handle = start_mock_server("127.0.0.1:7878", responses, &requests);

    // do.
    let _ = client.join();
    let _ = client.wait_next();
    let ret = client.notify_done();
    let _ = mock_handle.join();
    // check result.
    assert_eq!(ret, MessageType::Error);
    assert_eq!(client.connected, true);
    assert_eq!(client.startup, false);
    {
        let requests = requests.lock().unwrap();
        assert_eq!(
            requests.len(),
            (3 + client.config.retry_count_done) as usize
        );
        assert_eq!(requests[0].mtype, MessageType::Join);
        assert_eq!(requests[1].mtype, MessageType::Ready);
        assert_eq!(requests[2].mtype, MessageType::Done);
        assert_eq!(requests[3].mtype, MessageType::Done);
    }
}

#[test]
#[should_panic]
fn test_done_precond() {
    // create objects.
    let _ = env_logger::builder()
        .is_test(true)
        .format_timestamp_millis()
        .try_init();
    let mut client: DPSClient = DPSClient::new("127.0.0.1".to_string(), 7878, 0, 0.1, 1.0);

    // setup condition.
    // do nothing.

    // do.
    let _ = client.notify_done();
    // check result.
    // should panic.
}

#[test]
fn test_exit() {
    // create objects.
    let _ = env_logger::builder()
        .is_test(true)
        .format_timestamp_millis()
        .try_init();
    let mut client: DPSClient = DPSClient::new("127.0.0.1".to_string(), 7878, 0, 0.1, 1.0);

    // setup condition.
    let responses: Vec<MockResponse> = vec![
        // Join.
        MockResponse {
            response: Message::new(MessageType::Ok, 0, 0, None).unwrap(),
            delay_sec: 0.0,
        },
        // Exit.
        MockResponse {
            response: Message::new(MessageType::Ok, 0, 0, None).unwrap(),
            delay_sec: client.config.retry_sec_exit / 2.0,
        },
    ];
    let requests: Arc<Mutex<Vec<Message>>> = Arc::new(Mutex::new(vec![]));
    let mock_handle = start_mock_server("127.0.0.1:7878", responses, &requests);

    // do.
    let _ = client.join();
    client.exit();
    let _ = mock_handle.join();
    // check result.
    assert_eq!(client.connected, false);
    assert_eq!(client.startup, true);
    assert_eq!(client.sock.is_none(), true);
    {
        let requests = requests.lock().unwrap();
        assert_eq!(requests.len(), 2);
        assert_eq!(requests[0].mtype, MessageType::Join);
        assert_eq!(requests[1].mtype, MessageType::Exit);
    }
}

#[test]
fn test_exit_retry_ok() {
    // create objects.
    let _ = env_logger::builder()
        .is_test(true)
        .format_timestamp_millis()
        .try_init();
    let mut client: DPSClient = DPSClient::new("127.0.0.1".to_string(), 7878, 0, 0.1, 1.0);

    // setup condition.
    let responses: Vec<MockResponse> = vec![
        // Join.
        MockResponse {
            response: Message::new(MessageType::Ok, 0, 0, None).unwrap(),
            delay_sec: 0.0,
        },
        // Exit.
        MockResponse {
            response: Message::new(MessageType::Ok, 0, 0, None).unwrap(),
            delay_sec: client.config.retry_sec_exit + 0.005,
        },
    ];
    let requests: Arc<Mutex<Vec<Message>>> = Arc::new(Mutex::new(vec![]));
    let mock_handle = start_mock_server("127.0.0.1:7878", responses, &requests);

    // do.
    let _ = client.join();
    client.exit();
    let _ = mock_handle.join();
    // check result.
    assert_eq!(client.connected, false);
    assert_eq!(client.startup, true);
    assert_eq!(client.sock.is_none(), true);
    {
        let requests = requests.lock().unwrap();
        assert!(requests.len() > 1);
        assert_eq!(requests[0].mtype, MessageType::Join);
        assert_eq!(requests[1].mtype, MessageType::Exit);
        assert_eq!(requests[2].mtype, MessageType::Exit);
    }
}

#[test]
fn test_exit_retry_ng() {
    // create objects.
    let _ = env_logger::builder()
        .is_test(true)
        .format_timestamp_millis()
        .try_init();
    let mut client: DPSClient = DPSClient::new("127.0.0.1".to_string(), 7878, 0, 0.1, 1.0);

    // setup condition.
    let responses: Vec<MockResponse> = vec![
        // Join.
        MockResponse {
            response: Message::new(MessageType::Ok, 0, 0, None).unwrap(),
            delay_sec: 0.0,
        },
        // Exit.
        MockResponse {
            response: Message::new(MessageType::Ok, 0, 0, None).unwrap(),
            delay_sec: client.config.retry_sec_exit * (client.config.retry_count_exit as f64 + 1.0)
                + 0.005,
        },
    ];
    let requests: Arc<Mutex<Vec<Message>>> = Arc::new(Mutex::new(vec![]));
    let mock_handle = start_mock_server("127.0.0.1:7878", responses, &requests);

    // do.
    let _ = client.join();
    client.exit();
    let _ = mock_handle.join();
    // check result.
    assert_eq!(client.connected, false);
    assert_eq!(client.startup, true);
    assert_eq!(client.sock.is_none(), true);
    {
        let requests = requests.lock().unwrap();
        assert_eq!(
            requests.len(),
            (2 + client.config.retry_count_exit) as usize
        );
        assert_eq!(requests[0].mtype, MessageType::Join);
        assert_eq!(requests[1].mtype, MessageType::Exit);
        assert_eq!(requests[2].mtype, MessageType::Exit);
    }
}

#[test]
fn test_exit_precond() {
    // create objects.
    let _ = env_logger::builder()
        .is_test(true)
        .format_timestamp_millis()
        .try_init();
    let mut client: DPSClient = DPSClient::new("127.0.0.1".to_string(), 7878, 0, 0.1, 1.0);

    // setup condition.
    // do nothing.

    // do.
    client.exit(); // without join.
    // check result.
    assert_eq!(client.connected, false);
    assert_eq!(client.startup, true);
    assert_eq!(client.sock.is_none(), true);
}
