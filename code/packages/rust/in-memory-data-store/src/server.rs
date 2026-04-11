use std::io;
use std::path::PathBuf;
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::{Arc, Mutex};
use std::thread::{self, JoinHandle};
use std::time::Duration;

use in_memory_data_store_engine::{DataStoreEngine, Store};
use in_memory_data_store_protocol::command_frame_from_resp;
use resp_protocol::{decode, encode, RespError, RespValue};
use tcp_server::{Connection, TcpServer};

pub struct DataStoreServer {
    engine: Arc<DataStoreEngine>,
    server: TcpServer,
    expirer_stop: Arc<AtomicBool>,
    expirer_handle: Mutex<Option<JoinHandle<()>>>,
}

impl DataStoreServer {
    pub fn new(port: u16) -> Self {
        Self::with_options("127.0.0.1", port, None::<PathBuf>)
            .expect("failed to create DataStoreServer")
    }

    pub fn with_aof_path(port: u16, aof_path: impl Into<PathBuf>) -> Self {
        Self::with_options("127.0.0.1", port, Some(aof_path.into()))
            .expect("failed to create DataStoreServer")
    }

    pub fn with_options(
        host: impl Into<String>,
        port: u16,
        aof_path: Option<PathBuf>,
    ) -> io::Result<Self> {
        let host = host.into();
        let engine = Arc::new(DataStoreEngine::new(aof_path)?);

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
        self.engine.execute_parts(command)
    }

    pub fn execute_owned(&self, command: Vec<Vec<u8>>) -> RespValue {
        self.engine.execute_parts(&command)
    }

    pub fn store(&self) -> Store {
        self.engine.store()
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
                engine.active_expire_all();
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

fn handle_connection(engine: &Arc<DataStoreEngine>, conn: &mut Connection, data: &[u8]) -> Vec<u8> {
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
                    responses.extend(encode(response).unwrap());
                    continue;
                };
                let (active_db, response) = engine.execute_with_db(conn.selected_db, &frame, true);
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
