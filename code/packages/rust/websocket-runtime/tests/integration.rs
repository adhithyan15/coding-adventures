use std::io::{BufRead, BufReader, Read, Write};
use std::net::{TcpListener, TcpStream};
use std::sync::{Arc, Mutex};
use std::thread;
use std::time::Duration;

use websocket_core::{accept_server_request, Frame, MessageEvent, WebSocketError};
use websocket_runtime::{
    EntropySource, WebSocketClient, WebSocketClientOptions, WebSocketConnectionInfo,
    WebSocketHandlerResult, WebSocketRuntime, WebSocketRuntimeError, WebSocketServerOptions,
};

#[cfg(any(
    target_os = "macos",
    target_os = "freebsd",
    target_os = "openbsd",
    target_os = "netbsd",
    target_os = "dragonfly"
))]
type HostPlatform = transport_platform::bsd::KqueueTransportPlatform;
#[cfg(target_os = "linux")]
type HostPlatform = transport_platform::linux::EpollTransportPlatform;
#[cfg(target_os = "windows")]
type HostPlatform = transport_platform::windows::WindowsTransportPlatform;

fn bind_echo_server() -> WebSocketRuntime<HostPlatform, ()> {
    let options = WebSocketServerOptions::default();
    let handler = |_: WebSocketConnectionInfo, event: MessageEvent| match event {
        MessageEvent::Text(text) => WebSocketHandlerResult::send(Frame::text(text)),
        MessageEvent::Binary(bytes) => WebSocketHandlerResult::send(Frame::binary(bytes)),
        MessageEvent::Ping(_) | MessageEvent::Pong(_) | MessageEvent::Close(_) => {
            WebSocketHandlerResult::default()
        }
    };

    #[cfg(any(
        target_os = "macos",
        target_os = "freebsd",
        target_os = "openbsd",
        target_os = "netbsd",
        target_os = "dragonfly"
    ))]
    return WebSocketRuntime::bind_kqueue("127.0.0.1:0", options, handler).unwrap();

    #[cfg(target_os = "linux")]
    return WebSocketRuntime::bind_epoll("127.0.0.1:0", options, handler).unwrap();

    #[cfg(target_os = "windows")]
    return WebSocketRuntime::bind_windows("127.0.0.1:0", options, handler).unwrap();
}

fn start_echo_server() -> (
    std::net::SocketAddr,
    websocket_runtime::StopHandle,
    thread::JoinHandle<()>,
) {
    let mut runtime = bind_echo_server();
    let address = runtime.local_addr();
    let stop = runtime.stop_handle();
    let _mailbox = runtime.tcp_mailbox();
    let thread = thread::spawn(move || runtime.serve().unwrap());
    (address, stop, thread)
}

#[test]
fn client_and_reactor_server_exchange_all_v1_event_classes() {
    let (address, stop, server) = start_echo_server();
    let mut client = WebSocketClient::connect(
        "127.0.0.1",
        address.port(),
        "/chief",
        WebSocketClientOptions::default(),
    )
    .unwrap();
    assert_eq!(client.peer_addr().unwrap(), address);
    assert_eq!(client.local_addr().unwrap().ip().to_string(), "127.0.0.1");

    client.send_text("hello").unwrap();
    assert_eq!(
        client.receive().unwrap(),
        MessageEvent::Text("hello".into())
    );

    client.send_binary(vec![0, 1, 2, 255]).unwrap();
    assert_eq!(
        client.receive().unwrap(),
        MessageEvent::Binary(vec![0, 1, 2, 255])
    );

    client.send_ping(b"health".to_vec()).unwrap();
    assert_eq!(
        client.receive().unwrap(),
        MessageEvent::Pong(b"health".to_vec())
    );

    client.close(Some(1000), "done").unwrap();
    assert!(matches!(client.receive().unwrap(), MessageEvent::Close(_)));
    assert!(client.is_closing());
    client.close(Some(1000), "already closing").unwrap();
    assert!(matches!(
        client.send_text("too late"),
        Err(WebSocketRuntimeError::ClosedSession)
    ));
    assert!(matches!(
        client.receive(),
        Err(WebSocketRuntimeError::ClosedSession)
    ));

    stop.stop();
    server.join().unwrap();
}

#[test]
fn raw_browser_wire_client_upgrades_and_receives_unmasked_echo() {
    let (address, stop, server) = start_echo_server();
    let mut stream = TcpStream::connect(address).unwrap();
    stream
        .set_read_timeout(Some(Duration::from_secs(5)))
        .unwrap();
    stream
        .write_all(
            b"GET /chief HTTP/1.1\r\nHost: localhost\r\nUpgrade: websocket\r\nConnection: Upgrade\r\nSec-WebSocket-Key: dGhlIHNhbXBsZSBub25jZQ==\r\nSec-WebSocket-Version: 13\r\n\r\n",
        )
        .unwrap();
    stream
        .write_all(&[
            0x81, 0x85, 0x37, 0xfa, 0x21, 0x3d, 0x7f, 0x9f, 0x4d, 0x51, 0x58,
        ])
        .unwrap();

    let mut reader = BufReader::new(stream);
    let response = read_http_head(&mut reader);
    assert!(response.starts_with(b"HTTP/1.1 101 Switching Protocols\r\n"));
    assert!(response
        .windows(28)
        .any(|window| window == b"s3pPLMBiTxaQ9kYGzzhZRbK+xOo="));
    let mut echo = [0_u8; 7];
    reader.read_exact(&mut echo).unwrap();
    assert_eq!(&echo, b"\x81\x05Hello");

    stop.stop();
    server.join().unwrap();
}

fn read_http_head(reader: &mut BufReader<TcpStream>) -> Vec<u8> {
    let mut head = Vec::new();
    while !head.ends_with(b"\r\n\r\n") {
        let mut line = Vec::new();
        reader.read_until(b'\n', &mut line).unwrap();
        assert!(!line.is_empty());
        head.extend_from_slice(&line);
    }
    head
}

#[derive(Clone)]
struct RecordingEntropy {
    calls: Arc<Mutex<Vec<usize>>>,
}

impl EntropySource for RecordingEntropy {
    fn fill(&mut self, output: &mut [u8]) -> Result<(), WebSocketRuntimeError> {
        let mut calls = self.calls.lock().unwrap();
        calls.push(output.len());
        output.fill(calls.len() as u8);
        Ok(())
    }
}

#[test]
fn client_requests_fresh_nonce_and_mask_entropy_for_every_frame() {
    let listener = TcpListener::bind("127.0.0.1:0").unwrap();
    let port = listener.local_addr().unwrap().port();
    let captured_masks = Arc::new(Mutex::new(Vec::new()));
    let server_masks = Arc::clone(&captured_masks);
    let server = thread::spawn(move || capture_two_client_frames(listener, server_masks));

    let calls = Arc::new(Mutex::new(Vec::new()));
    let entropy = RecordingEntropy {
        calls: Arc::clone(&calls),
    };
    let mut client = WebSocketClient::connect_with_entropy(
        "127.0.0.1",
        port,
        "/",
        WebSocketClientOptions::default(),
        entropy,
    )
    .unwrap();
    client.send_text("a").unwrap();
    client.send_text("b").unwrap();
    drop(client);
    server.join().unwrap();

    assert_eq!(*calls.lock().unwrap(), vec![16, 4, 4]);
    assert_eq!(*captured_masks.lock().unwrap(), vec![[2; 4], [3; 4]]);
}

fn capture_two_client_frames(listener: TcpListener, masks: Arc<Mutex<Vec<[u8; 4]>>>) {
    let (stream, _) = listener.accept().unwrap();
    stream
        .set_read_timeout(Some(Duration::from_secs(5)))
        .unwrap();
    let mut reader = BufReader::new(stream);
    let request = read_http_head(&mut reader);
    let handshake = accept_server_request(&request).unwrap();
    reader.get_mut().write_all(handshake.response()).unwrap();
    reader.get_mut().flush().unwrap();

    let mut wire = [0_u8; 14];
    reader.read_exact(&mut wire).unwrap();
    assert_eq!(&wire[0..2], &[0x81, 0x81]);
    assert_eq!(&wire[7..9], &[0x81, 0x81]);
    let first: [u8; 4] = wire[2..6].try_into().unwrap();
    let second: [u8; 4] = wire[9..13].try_into().unwrap();
    masks.lock().unwrap().extend([first, second]);
}

#[test]
fn client_preserves_coalesced_frames_after_the_upgrade_head() {
    let listener = TcpListener::bind("127.0.0.1:0").unwrap();
    let port = listener.local_addr().unwrap().port();
    let server = thread::spawn(move || {
        let (stream, _) = listener.accept().unwrap();
        let mut reader = BufReader::new(stream);
        let request = read_http_head(&mut reader);
        let handshake = accept_server_request(&request).unwrap();
        let mut response = handshake.response().to_vec();
        response.extend_from_slice(b"\x81\x03one\x82\x02\x04\x05");
        reader.get_mut().write_all(&response).unwrap();
        reader.get_mut().flush().unwrap();
    });

    let mut client =
        WebSocketClient::connect("127.0.0.1", port, "/", WebSocketClientOptions::default())
            .unwrap();
    assert_eq!(client.receive().unwrap(), MessageEvent::Text("one".into()));
    assert_eq!(client.receive().unwrap(), MessageEvent::Binary(vec![4, 5]));
    server.join().unwrap();
}

#[test]
fn client_answers_invalid_peer_frames_with_a_masked_protocol_close() {
    let listener = TcpListener::bind("127.0.0.1:0").unwrap();
    let port = listener.local_addr().unwrap().port();
    let server = thread::spawn(move || {
        let (stream, _) = listener.accept().unwrap();
        stream
            .set_read_timeout(Some(Duration::from_secs(5)))
            .unwrap();
        let mut reader = BufReader::new(stream);
        let request = read_http_head(&mut reader);
        let handshake = accept_server_request(&request).unwrap();
        reader.get_mut().write_all(handshake.response()).unwrap();
        reader.get_mut().write_all(&[0x83, 0]).unwrap();
        reader.get_mut().flush().unwrap();

        let mut close = [0_u8; 8];
        reader.read_exact(&mut close).unwrap();
        assert_eq!(&close[..2], &[0x88, 0x82]);
        let code = [close[6] ^ close[2], close[7] ^ close[3]];
        assert_eq!(u16::from_be_bytes(code), 1002);
    });

    let mut client =
        WebSocketClient::connect("127.0.0.1", port, "/", WebSocketClientOptions::default())
            .unwrap();
    assert!(matches!(
        client.receive(),
        Err(WebSocketRuntimeError::Protocol(
            WebSocketError::InvalidOpcode
        ))
    ));
    assert!(client.is_closing());
    server.join().unwrap();
}
