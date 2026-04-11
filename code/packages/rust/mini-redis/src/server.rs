use std::fs::{File, OpenOptions};
use std::io::{self, Write};
use std::path::{Path, PathBuf};
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::{Arc, Mutex};
use std::thread::{self, JoinHandle};
use std::time::Duration;

use resp_protocol::{decode, decode_all, encode, RespError, RespValue};
use tcp_server::{Connection, TcpServer};

use crate::commands::{dispatch, is_mutating};
use crate::store::Store;

pub trait RedisBackend: Send + Sync {
    fn execute(&self, command: &[Vec<u8>]) -> RespValue;
    fn execute_owned(&self, command: Vec<Vec<u8>>) -> RespValue {
        self.execute(&command)
    }
    fn store(&self) -> Store;
    fn active_expire_all(&self);
}

fn backend_execute(backend: &dyn RedisBackend, command: &[Vec<u8>]) -> RespValue {
    backend.execute(command)
}

fn backend_store(backend: &dyn RedisBackend) -> Store {
    backend.store()
}

fn backend_active_expire_all(backend: &dyn RedisBackend) {
    backend.active_expire_all();
}

#[derive(Clone)]
pub struct MiniRedisEngine {
    store: Arc<Mutex<Store>>,
    aof_file: Arc<Mutex<Option<File>>>,
}

pub struct MiniRedis {
    engine: Arc<MiniRedisEngine>,
    server: TcpServer,
    expirer_stop: Arc<AtomicBool>,
    expirer_handle: Mutex<Option<JoinHandle<()>>>,
}

impl MiniRedis {
    pub fn new(port: u16) -> Self {
        Self::with_options("127.0.0.1", port, None::<PathBuf>).expect("failed to create MiniRedis")
    }

    pub fn with_aof_path(port: u16, aof_path: impl Into<PathBuf>) -> Self {
        Self::with_options("127.0.0.1", port, Some(aof_path.into()))
            .expect("failed to create MiniRedis")
    }

    pub fn with_options(
        host: impl Into<String>,
        port: u16,
        aof_path: Option<PathBuf>,
    ) -> io::Result<Self> {
        let host = host.into();
        let engine = Arc::new(MiniRedisEngine::new(aof_path)?);

        let handler_engine = engine.clone();
        let server = TcpServer::with_handler(host, port, move |conn, data| {
            handle_connection(&handler_engine, conn, data)
        });

        Ok(Self {
            engine,
            server,
            expirer_stop: Arc::new(AtomicBool::new(false)),
            expirer_handle: Mutex::new(None),
        })
    }

    pub fn start(&self) -> io::Result<()> {
        self.start_expirer();
        let result = self.server.serve_forever();
        self.stop_expirer();
        result
    }

    pub fn stop(&self) {
        self.expirer_stop.store(true, Ordering::SeqCst);
        self.server.stop();
    }

    pub fn execute(&self, command: &[Vec<u8>]) -> RespValue {
        backend_execute(self.engine.as_ref(), command)
    }

    pub fn execute_owned(&self, command: Vec<Vec<u8>>) -> RespValue {
        backend_execute(self.engine.as_ref(), &command)
    }

    pub fn store(&self) -> Store {
        backend_store(self.engine.as_ref())
    }

    fn start_expirer(&self) {
        let mut guard = self
            .expirer_handle
            .lock()
            .expect("expirer handle mutex poisoned");
        if guard.is_some() {
            return;
        }
        self.expirer_stop.store(false, Ordering::SeqCst);
        let engine = self.engine.clone();
        let stop = self.expirer_stop.clone();
        *guard = Some(thread::spawn(move || {
            while !stop.load(Ordering::SeqCst) {
                thread::sleep(Duration::from_millis(100));
                backend_active_expire_all(engine.as_ref());
            }
        }));
    }

    fn stop_expirer(&self) {
        self.expirer_stop.store(true, Ordering::SeqCst);
        if let Some(handle) = self
            .expirer_handle
            .lock()
            .expect("expirer handle mutex poisoned")
            .take()
        {
            let _ = handle.join();
        }
    }
}

impl MiniRedisEngine {
    pub fn new(aof_path: Option<PathBuf>) -> io::Result<Self> {
        let aof_file = if let Some(path) = &aof_path {
            Some(
                OpenOptions::new()
                    .create(true)
                    .append(true)
                    .read(true)
                    .open(path)?,
            )
        } else {
            None
        };
        let engine = Self {
            store: Arc::new(Mutex::new(Store::empty())),
            aof_file: Arc::new(Mutex::new(aof_file)),
        };
        if let Some(path) = &aof_path {
            if path.exists() {
                engine.replay_aof(path)?;
            }
        }
        Ok(engine)
    }

    pub fn execute(&self, command: &[Vec<u8>]) -> RespValue {
        apply_parts_inner(self, command, true)
    }

    pub fn execute_owned(&self, command: Vec<Vec<u8>>) -> RespValue {
        self.execute(&command)
    }

    pub fn store(&self) -> Store {
        self.store.lock().expect("store mutex poisoned").clone()
    }

    pub fn active_expire_all(&self) {
        if let Ok(mut store) = self.store.lock() {
            *store = store.clone().active_expire_all();
        }
    }

    fn replay_aof(&self, path: &Path) -> io::Result<()> {
        let bytes = std::fs::read(path)?;
        let (messages, _) = decode_all(&bytes).map_err(map_resp_decode_error)?;
        for message in messages {
            if let Some(parts) = command_parts_from_resp(message) {
                let _ = apply_parts_inner(self, &parts, false);
            }
        }
        Ok(())
    }
}

impl RedisBackend for MiniRedisEngine {
    fn execute(&self, command: &[Vec<u8>]) -> RespValue {
        self.execute(command)
    }

    fn store(&self) -> Store {
        self.store()
    }

    fn active_expire_all(&self) {
        self.active_expire_all();
    }
}

fn handle_connection(engine: &Arc<MiniRedisEngine>, conn: &mut Connection, data: &[u8]) -> Vec<u8> {
    conn.read_buffer.extend_from_slice(data);
    let mut responses = Vec::new();

    loop {
        match decode(&conn.read_buffer) {
            Ok(Some((value, consumed))) => {
                conn.read_buffer.drain(..consumed);
                let Some(parts) = command_parts_from_resp(value) else {
                    let response = RespValue::Error(RespError::new(
                        "ERR protocol error: expected array of bulk strings",
                    ));
                    responses.extend(encode(response).unwrap());
                    continue;
                };
                let (active_db, response) =
                    apply_parts_inner_with_db(engine, conn.selected_db, &parts, true);
                conn.selected_db = active_db;
                responses.extend(encode(response).unwrap());
            }
            Ok(None) => break,
            Err(err) => {
                conn.read_buffer.clear();
                let response = RespValue::Error(RespError::new(format!("ERR {err}")));
                responses.extend(encode(response).unwrap());
                break;
            }
        }
    }

    responses
}

fn apply_parts_inner(engine: &MiniRedisEngine, parts: &[Vec<u8>], record_aof: bool) -> RespValue {
    let active_db = engine
        .store
        .lock()
        .expect("store mutex poisoned")
        .active_db;
    apply_parts_inner_with_db(engine, active_db, parts, record_aof).1
}

fn apply_parts_inner_with_db(
    engine: &MiniRedisEngine,
    db_index: usize,
    parts: &[Vec<u8>],
    record_aof: bool,
) -> (usize, RespValue) {
    let mut store = engine.store.lock().expect("store mutex poisoned").clone();
    store = store.with_active_db(db_index);
    let (new_store, response) = dispatch(store, parts);

    if record_aof && is_mutating(parts) && !matches!(parts.first().map(|v| v.as_slice()), Some(cmd) if ascii_upper(cmd) == "SELECT") {
        append_aof(engine, parts);
    }

    let active_db = new_store.active_db;
    *engine.store.lock().expect("store mutex poisoned") = new_store;
    (active_db, response)
}

fn append_aof(engine: &MiniRedisEngine, parts: &[Vec<u8>]) {
    let mut guard = engine.aof_file.lock().expect("aof file mutex poisoned");
    let Some(file) = guard.as_mut() else {
        return;
    };
    let payload = RespValue::Array(Some(
        parts
            .iter()
            .cloned()
            .map(|bytes| RespValue::BulkString(Some(bytes)))
            .collect(),
    ));
    if let Ok(encoded) = encode(payload) {
        let _ = file.write_all(&encoded);
        let _ = file.flush();
    }
}

fn command_parts_from_resp(value: RespValue) -> Option<Vec<Vec<u8>>> {
    match value {
        RespValue::Array(Some(values)) => {
            let mut parts = Vec::with_capacity(values.len());
            for item in values {
                match item {
                    RespValue::BulkString(Some(bytes)) => parts.push(bytes),
                    RespValue::SimpleString(text) => parts.push(text.into_bytes()),
                    RespValue::Integer(n) => parts.push(n.to_string().into_bytes()),
                    _ => return None,
                }
            }
            Some(parts)
        }
        _ => None,
    }
}

fn map_resp_decode_error(err: resp_protocol::RespDecodeError) -> io::Error {
    io::Error::new(io::ErrorKind::InvalidData, err.message)
}

fn ascii_upper(bytes: &[u8]) -> String {
    bytes
        .iter()
        .map(|byte| byte.to_ascii_uppercase() as char)
        .collect()
}
