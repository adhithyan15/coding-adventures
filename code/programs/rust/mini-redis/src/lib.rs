//! # mini-redis
//!
//! Small Redis-compatible TCP server built on top of the in-memory data-store
//! pipeline and the repository's current TCP server substrate.
//!
//! This crate deliberately keeps the Redis-facing logic in one place:
//!
//! - RESP framing enters through the TCP handler
//! - per-connection session state lives on the mutable `tcp-server` connection
//! - decoded command frames execute against `DataStoreManager`
//! - engine responses return to the client as RESP values

mod resp_adapter;

use std::io;
use std::net::SocketAddr;
use std::path::PathBuf;
use std::sync::Arc;

use in_memory_data_store::DataStoreManager;
use resp_adapter::{command_frame_from_resp, engine_response_to_resp};
use resp_protocol::{decode, encode, RespError, RespValue};
use tcp_server::{Connection, TcpServer};

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct MiniRedisOptions {
    pub host: String,
    pub port: u16,
    pub aof_path: Option<PathBuf>,
}

impl Default for MiniRedisOptions {
    fn default() -> Self {
        Self {
            host: "127.0.0.1".to_string(),
            port: 6379,
            aof_path: None,
        }
    }
}

#[derive(Clone)]
pub struct MiniRedisServer {
    server: TcpServer,
}

impl MiniRedisServer {
    pub fn new(options: MiniRedisOptions) -> io::Result<Self> {
        let manager = Arc::new(DataStoreManager::new(options.aof_path.clone())?);
        manager.start_background_workers();
        let shared_manager = Arc::clone(&manager);

        let server = TcpServer::with_handler(options.host, options.port, move |conn, data| {
            handle_connection_data(&shared_manager, conn, data)
        });

        Ok(Self { server })
    }

    pub fn start(&self) -> io::Result<()> {
        self.server.start()
    }

    pub fn serve(&self) -> io::Result<()> {
        self.server.serve()
    }

    pub fn serve_forever(&self) -> io::Result<()> {
        self.server.serve_forever()
    }

    pub fn stop(&self) {
        self.server.stop();
    }

    pub fn address(&self) -> Option<SocketAddr> {
        self.server.address()
    }

    pub fn try_address(&self) -> io::Result<SocketAddr> {
        self.server.try_address()
    }

    pub fn is_running(&self) -> bool {
        self.server.is_running()
    }
}

fn handle_connection_data(
    manager: &DataStoreManager,
    conn: &mut Connection,
    data: &[u8],
) -> Vec<u8> {
    conn.read_buffer.extend_from_slice(data);
    let mut responses = Vec::new();

    loop {
        match decode(&conn.read_buffer) {
            Ok(Some((value, consumed))) => {
                conn.read_buffer.drain(..consumed);
                let Some(frame) = command_frame_from_resp(value) else {
                    let response = RespValue::Error(RespError::new(
                        "ERR protocol error: expected array of bulk strings",
                    ));
                    responses.extend(encode(response).expect("RESP error responses should encode"));
                    continue;
                };

                let engine_resp = manager.execute(&mut conn.selected_db, &frame);
                let resp_val = engine_response_to_resp(engine_resp);
                responses.extend(encode(resp_val).expect("engine responses should encode"));
            }
            Ok(None) => break,
            Err(err) => {
                conn.read_buffer.clear();
                let response = RespValue::Error(RespError::new(format!("ERR {err}")));
                responses.extend(encode(response).expect("RESP error responses should encode"));
                break;
            }
        }
    }

    responses
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
}
