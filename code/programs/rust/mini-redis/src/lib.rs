//! # mini-redis
//!
//! Small Redis-compatible TCP server built on top of the in-memory data-store
//! pipeline and the repository's `tcp-runtime`.
//!
//! This crate keeps the Redis-facing logic in one place:
//!
//! - RESP framing enters through the TCP runtime read callback
//! - per-connection Redis session state lives in `tcp-runtime`
//! - decoded command frames execute against `DataStoreManager`
//! - engine responses return to the client as RESP values

mod resp_adapter;

use std::io;
use std::net::SocketAddr;
use std::path::PathBuf;
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::{Arc, Mutex};

use in_memory_data_store::DataStoreManager;
use resp_adapter::{command_frame_from_resp, engine_response_to_resp};
use resp_protocol::{decode, encode, RespError, RespValue};
use tcp_runtime::{PlatformError, StopHandle, TcpHandlerResult, TcpRuntime, TcpRuntimeOptions};

const MAX_BUFFERED_REQUEST_BYTES: usize = 1024 * 1024;

#[cfg(any(
    target_os = "macos",
    target_os = "freebsd",
    target_os = "openbsd",
    target_os = "netbsd",
    target_os = "dragonfly"
))]
type RedisRuntime =
    TcpRuntime<transport_platform::bsd::KqueueTransportPlatform, RedisConnectionState>;

#[cfg(target_os = "linux")]
type RedisRuntime =
    TcpRuntime<transport_platform::linux::EpollTransportPlatform, RedisConnectionState>;

#[cfg(target_os = "windows")]
type RedisRuntime =
    TcpRuntime<transport_platform::windows::WindowsTransportPlatform, RedisConnectionState>;

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct MiniRedisOptions {
    pub host: String,
    pub port: u16,
    pub aof_path: Option<PathBuf>,
    pub max_connections: usize,
}

impl Default for MiniRedisOptions {
    fn default() -> Self {
        Self {
            host: "127.0.0.1".to_string(),
            port: 6379,
            aof_path: None,
            max_connections: TcpRuntimeOptions::default().max_connections,
        }
    }
}

#[derive(Debug, Default, Clone, PartialEq, Eq)]
struct RedisConnectionState {
    read_buffer: Vec<u8>,
    selected_db: usize,
}

#[derive(Clone)]
pub struct MiniRedisServer {
    runtime: Arc<Mutex<Option<RedisRuntime>>>,
    local_addr: SocketAddr,
    stop_handle: StopHandle,
    serving: Arc<AtomicBool>,
}

impl MiniRedisServer {
    pub fn new(options: MiniRedisOptions) -> io::Result<Self> {
        let manager = Arc::new(DataStoreManager::new(options.aof_path.clone())?);
        manager.start_background_workers();

        let runtime = build_runtime(&options, Arc::clone(&manager)).map_err(into_io_error)?;
        let local_addr = runtime.local_addr();
        let stop_handle = runtime.stop_handle();

        Ok(Self {
            runtime: Arc::new(Mutex::new(Some(runtime))),
            local_addr,
            stop_handle,
            serving: Arc::new(AtomicBool::new(false)),
        })
    }

    /// `tcp-runtime` binds the listener eagerly during construction, so `start`
    /// is now a no-op kept for compatibility with existing call sites and
    /// tests.
    pub fn start(&self) -> io::Result<()> {
        Ok(())
    }

    pub fn serve(&self) -> io::Result<()> {
        let mut runtime = self
            .runtime
            .lock()
            .expect("mini-redis runtime mutex poisoned")
            .take()
            .ok_or_else(|| {
                io::Error::new(
                    io::ErrorKind::AlreadyExists,
                    "mini-redis runtime is already serving or has already served",
                )
            })?;

        self.serving.store(true, Ordering::SeqCst);
        let result = runtime.serve().map_err(into_io_error);
        self.serving.store(false, Ordering::SeqCst);
        result
    }

    pub fn serve_forever(&self) -> io::Result<()> {
        self.serve()
    }

    pub fn stop(&self) {
        self.stop_handle.stop();
    }

    pub fn address(&self) -> Option<SocketAddr> {
        Some(self.local_addr)
    }

    pub fn try_address(&self) -> io::Result<SocketAddr> {
        Ok(self.local_addr)
    }

    pub fn is_running(&self) -> bool {
        self.serving.load(Ordering::SeqCst)
    }
}

fn handle_connection_data(
    manager: &DataStoreManager,
    state: &mut RedisConnectionState,
    data: &[u8],
) -> TcpHandlerResult {
    if state.read_buffer.len().saturating_add(data.len()) > MAX_BUFFERED_REQUEST_BYTES {
        state.read_buffer.clear();
        return TcpHandlerResult::write_and_close(protocol_error_response(
            "ERR protocol error: request exceeds maximum buffered size",
        ));
    }

    state.read_buffer.extend_from_slice(data);
    let mut responses = Vec::new();

    loop {
        match decode(&state.read_buffer) {
            Ok(Some((value, consumed))) => {
                state.read_buffer.drain(..consumed);
                let Some(frame) = command_frame_from_resp(value) else {
                    responses.extend(protocol_error_response(
                        "ERR protocol error: expected array of bulk strings",
                    ));
                    continue;
                };

                let engine_resp = manager.execute(&mut state.selected_db, &frame);
                let resp_val = engine_response_to_resp(engine_resp);
                responses.extend(encode(resp_val).expect("engine responses should encode"));
            }
            Ok(None) => break,
            Err(err) => {
                state.read_buffer.clear();
                responses.extend(protocol_error_response(&format!("ERR {err}")));
                break;
            }
        }
    }

    if responses.is_empty() {
        TcpHandlerResult::default()
    } else {
        TcpHandlerResult::write(responses)
    }
}

fn protocol_error_response(message: &str) -> Vec<u8> {
    encode(RespValue::Error(RespError::new(message))).expect("RESP error responses should encode")
}

fn runtime_options(options: &MiniRedisOptions) -> TcpRuntimeOptions {
    let mut runtime_options = TcpRuntimeOptions::default();
    runtime_options.max_connections = options.max_connections.max(1);
    runtime_options
}

#[cfg(any(
    target_os = "macos",
    target_os = "freebsd",
    target_os = "openbsd",
    target_os = "netbsd",
    target_os = "dragonfly"
))]
fn build_runtime(
    options: &MiniRedisOptions,
    manager: Arc<DataStoreManager>,
) -> Result<RedisRuntime, PlatformError> {
    let host = options.host.clone();
    TcpRuntime::bind_kqueue_with_state(
        (host.as_str(), options.port),
        runtime_options(options),
        |_| RedisConnectionState::default(),
        move |_, state, bytes| handle_connection_data(&manager, state, bytes),
        |_, _| {},
    )
}

#[cfg(target_os = "linux")]
fn build_runtime(
    options: &MiniRedisOptions,
    manager: Arc<DataStoreManager>,
) -> Result<RedisRuntime, PlatformError> {
    let host = options.host.clone();
    TcpRuntime::bind_epoll_with_state(
        (host.as_str(), options.port),
        runtime_options(options),
        |_| RedisConnectionState::default(),
        move |_, state, bytes| handle_connection_data(&manager, state, bytes),
        |_, _| {},
    )
}

#[cfg(target_os = "windows")]
fn build_runtime(
    options: &MiniRedisOptions,
    manager: Arc<DataStoreManager>,
) -> Result<RedisRuntime, PlatformError> {
    let host = options.host.clone();
    TcpRuntime::bind_windows_with_state(
        (host.as_str(), options.port),
        runtime_options(options),
        |_| RedisConnectionState::default(),
        move |_, state, bytes| handle_connection_data(&manager, state, bytes),
        |_, _| {},
    )
}

fn into_io_error(error: PlatformError) -> io::Error {
    use io::ErrorKind;

    let kind = match error {
        PlatformError::AddressInUse => ErrorKind::AddrInUse,
        PlatformError::AddressNotAvailable => ErrorKind::AddrNotAvailable,
        PlatformError::PermissionDenied => ErrorKind::PermissionDenied,
        PlatformError::ConnectionRefused => ErrorKind::ConnectionRefused,
        PlatformError::ConnectionReset => ErrorKind::ConnectionReset,
        PlatformError::BrokenPipe => ErrorKind::BrokenPipe,
        PlatformError::TimedOut => ErrorKind::TimedOut,
        PlatformError::Interrupted => ErrorKind::Interrupted,
        PlatformError::InvalidResource => ErrorKind::InvalidInput,
        PlatformError::ResourceClosed => ErrorKind::BrokenPipe,
        PlatformError::Unsupported(_) => ErrorKind::Unsupported,
        PlatformError::Io(_) | PlatformError::ProviderFault(_) => ErrorKind::Other,
    };

    io::Error::new(kind, error.to_string())
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::io::{self, ErrorKind, Read, Write};
    use std::net::TcpStream;
    use std::thread;
    use std::time::Duration;

    fn command(parts: &[&str]) -> Vec<u8> {
        let values = parts
            .iter()
            .map(|part| RespValue::BulkString(Some(part.as_bytes().to_vec())))
            .collect::<Vec<_>>();
        encode(RespValue::Array(Some(values))).expect("command should encode")
    }

    fn read_response(stream: &mut TcpStream) -> io::Result<RespValue> {
        let mut buffer = Vec::new();
        let mut chunk = [0u8; 4096];
        loop {
            match stream.read(&mut chunk) {
                Ok(0) => {
                    return Err(io::Error::new(
                        ErrorKind::UnexpectedEof,
                        "server closed before sending a full RESP frame",
                    ));
                }
                Ok(n) => {
                    buffer.extend_from_slice(&chunk[..n]);
                    match decode(&buffer) {
                        Ok(Some((value, consumed))) => {
                            if consumed != buffer.len() {
                                return Err(io::Error::new(
                                    ErrorKind::InvalidData,
                                    "server returned extra bytes after one response",
                                ));
                            }
                            return Ok(value);
                        }
                        Ok(None) => continue,
                        Err(err) => {
                            return Err(io::Error::new(
                                ErrorKind::InvalidData,
                                format!("invalid RESP response: {err}"),
                            ));
                        }
                    }
                }
                Err(err) if matches!(err.kind(), ErrorKind::WouldBlock | ErrorKind::TimedOut) => {
                    continue;
                }
                Err(err) => return Err(err),
            }
        }
    }

    fn connect_with_retries(addr: SocketAddr) -> io::Result<TcpStream> {
        let mut last_error = None;
        for _ in 0..40 {
            match TcpStream::connect(addr) {
                Ok(stream) => {
                    stream.set_read_timeout(Some(Duration::from_millis(200)))?;
                    return Ok(stream);
                }
                Err(err) => {
                    last_error = Some(err);
                    thread::sleep(Duration::from_millis(10));
                }
            }
        }
        Err(last_error.unwrap_or_else(|| {
            io::Error::new(
                ErrorKind::TimedOut,
                "server did not accept connections in time",
            )
        }))
    }

    fn start_server() -> (
        MiniRedisServer,
        thread::JoinHandle<io::Result<()>>,
        SocketAddr,
    ) {
        let server = MiniRedisServer::new(MiniRedisOptions {
            port: 0,
            ..MiniRedisOptions::default()
        })
        .expect("server init");
        server.start().expect("server start");
        let addr = server.try_address().expect("bound address");
        let background = server.clone();
        let handle = thread::spawn(move || background.serve());
        (server, handle, addr)
    }

    #[test]
    fn runtime_options_honor_configured_connection_cap() {
        let options = MiniRedisOptions {
            max_connections: 10_000,
            ..MiniRedisOptions::default()
        };

        assert_eq!(runtime_options(&options).max_connections, 10_000);

        let options = MiniRedisOptions {
            max_connections: 0,
            ..MiniRedisOptions::default()
        };

        assert_eq!(runtime_options(&options).max_connections, 1);
    }

    #[test]
    fn responds_to_ping_over_tcp() {
        let (server, handle, addr) = start_server();
        let mut stream = connect_with_retries(addr).expect("connect");

        stream.write_all(&command(&["PING"])).expect("write ping");
        let response = read_response(&mut stream).expect("read pong");
        assert_eq!(response, RespValue::SimpleString("PONG".to_string()));

        server.stop();
        handle.join().expect("server thread").expect("server exit");
    }

    #[test]
    fn set_and_get_round_trip_on_one_connection() {
        let (server, handle, addr) = start_server();
        let mut stream = connect_with_retries(addr).expect("connect");

        stream
            .write_all(&command(&["SET", "greeting", "hello"]))
            .expect("write set");
        assert_eq!(
            read_response(&mut stream).expect("read set response"),
            RespValue::SimpleString("OK".to_string())
        );

        stream
            .write_all(&command(&["GET", "greeting"]))
            .expect("write get");
        assert_eq!(
            read_response(&mut stream).expect("read get response"),
            RespValue::BulkString(Some(b"hello".to_vec()))
        );

        server.stop();
        handle.join().expect("server thread").expect("server exit");
    }

    #[test]
    fn select_persists_database_choice_for_the_connection() {
        let (server, handle, addr) = start_server();
        let mut stream = connect_with_retries(addr).expect("connect");

        stream
            .write_all(&command(&["SELECT", "1"]))
            .expect("write select");
        assert_eq!(
            read_response(&mut stream).expect("read select response"),
            RespValue::SimpleString("OK".to_string())
        );

        stream
            .write_all(&command(&["SET", "scoped", "db1"]))
            .expect("write set");
        assert_eq!(
            read_response(&mut stream).expect("read set response"),
            RespValue::SimpleString("OK".to_string())
        );

        stream
            .write_all(&command(&["GET", "scoped"]))
            .expect("write get");
        assert_eq!(
            read_response(&mut stream).expect("read get response"),
            RespValue::BulkString(Some(b"db1".to_vec()))
        );

        let mut other = connect_with_retries(addr).expect("connect second client");
        other
            .write_all(&command(&["GET", "scoped"]))
            .expect("write isolated get");
        assert_eq!(
            read_response(&mut other).expect("read isolated get response"),
            RespValue::BulkString(None)
        );

        server.stop();
        handle.join().expect("server thread").expect("server exit");
    }

    #[test]
    fn buffers_fragmented_resp_commands_until_complete() {
        let (server, handle, addr) = start_server();
        let mut stream = connect_with_retries(addr).expect("connect");
        let payload = command(&["PING"]);
        let split = payload.len() / 2;

        stream
            .write_all(&payload[..split])
            .expect("write first fragment");
        let mut probe = [0u8; 16];
        match stream.read(&mut probe) {
            Ok(0) => panic!("server closed after incomplete command"),
            Ok(_) => panic!("server should not respond before the command is complete"),
            Err(err) => {
                assert!(
                    matches!(err.kind(), ErrorKind::WouldBlock | ErrorKind::TimedOut),
                    "expected timeout while waiting for fragmented RESP, got {err}"
                );
            }
        }

        stream
            .write_all(&payload[split..])
            .expect("write second fragment");
        let response = read_response(&mut stream).expect("read pong");
        assert_eq!(response, RespValue::SimpleString("PONG".to_string()));

        server.stop();
        handle.join().expect("server thread").expect("server exit");
    }

    #[test]
    fn closes_connections_with_an_error_when_buffered_input_exceeds_the_cap() {
        let (server, handle, addr) = start_server();
        let mut stream = connect_with_retries(addr).expect("connect");

        let oversized = MAX_BUFFERED_REQUEST_BYTES + 1;
        let header = format!("*1\r\n${oversized}\r\n");
        stream
            .write_all(header.as_bytes())
            .expect("write oversized header");

        let first_chunk = vec![b'a'; MAX_BUFFERED_REQUEST_BYTES - header.len()];
        stream.write_all(&first_chunk).expect("write first chunk");
        let final_chunk = [b'b'; 1];
        stream.write_all(&final_chunk).expect("write overflow byte");

        let response = read_response(&mut stream).expect("read oversized error");
        assert_eq!(
            response,
            RespValue::Error(RespError::new(
                "ERR protocol error: request exceeds maximum buffered size"
            ))
        );

        let mut probe = [0u8; 1];
        match stream.read(&mut probe) {
            Ok(0) => {}
            Ok(_) => panic!("server should close after rejecting an oversized request"),
            Err(err) => {
                assert!(
                    matches!(
                        err.kind(),
                        ErrorKind::WouldBlock
                            | ErrorKind::TimedOut
                            | ErrorKind::BrokenPipe
                            | ErrorKind::ConnectionReset
                            | ErrorKind::UnexpectedEof
                    ),
                    "expected close-like behavior after oversized request, got {err}"
                );
            }
        }

        server.stop();
        handle.join().expect("server thread").expect("server exit");
    }
}
