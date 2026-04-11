use std::collections::HashMap;
use std::fs::{File, OpenOptions};
use std::io::{self, Write};
use std::path::Path;
use std::path::PathBuf;
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::{Arc, Mutex};

use in_memory_data_store_protocol::{command_frame_from_resp, CommandFrame};
use resp_protocol::{decode_all, encode, RespError, RespValue};

use crate::commands::register_builtin_commands;
use crate::store::Store;

pub type CommandHandler = Arc<dyn Fn(Store, &[Vec<u8>]) -> (Store, RespValue) + Send + Sync>;

#[derive(Clone)]
pub struct CommandRegistration {
    pub handler: CommandHandler,
    pub mutating: bool,
    pub skip_lazy_expire: bool,
}

pub trait DataStoreBackend: Send + Sync {
    fn execute_frame(&self, command: &CommandFrame) -> RespValue;

    fn execute_frame_owned(&self, command: CommandFrame) -> RespValue {
        self.execute_frame(&command)
    }

    fn execute(&self, command: &[Vec<u8>]) -> RespValue {
        match CommandFrame::from_parts(command.to_vec()) {
            Some(frame) => self.execute_frame(&frame),
            None => RespValue::Error(RespError::new(
                "ERR protocol error: expected array of bulk strings",
            )),
        }
    }

    fn execute_owned(&self, command: Vec<Vec<u8>>) -> RespValue {
        self.execute(&command)
    }

    fn store(&self) -> Store;
    fn active_expire_all(&self);
}

#[derive(Clone)]
pub struct DataStoreEngine {
    store: Arc<Mutex<Store>>,
    aof_file: Arc<Mutex<Option<File>>>,
    commands: Arc<Mutex<HashMap<String, CommandRegistration>>>,
    frozen: Arc<AtomicBool>,
}

impl DataStoreEngine {
    pub fn new(aof_path: Option<PathBuf>) -> io::Result<Self> {
        Self::with_store_and_aof(Store::empty(), aof_path)
    }

    pub fn from_store(store: Store) -> Self {
        Self::with_store_and_aof(store, None).expect("failed to create engine")
    }

    fn with_store_and_aof(store: Store, aof_path: Option<PathBuf>) -> io::Result<Self> {
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
            store: Arc::new(Mutex::new(store)),
            aof_file: Arc::new(Mutex::new(aof_file)),
            commands: Arc::new(Mutex::new(HashMap::new())),
            frozen: Arc::new(AtomicBool::new(false)),
        };

        register_builtin_commands(&engine);

        if let Some(path) = &aof_path {
            if path.exists() {
                engine.replay_aof(path)?;
            }
        }

        Ok(engine)
    }

    pub fn register_command<F>(
        &self,
        name: impl Into<String>,
        mutating: bool,
        skip_lazy_expire: bool,
        handler: F,
    ) where
        F: Fn(Store, &[Vec<u8>]) -> (Store, RespValue) + Send + Sync + 'static,
    {
        assert!(
            !self.frozen.load(Ordering::SeqCst),
            "Cannot register commands on a frozen engine"
        );
        self.commands.lock().expect("command registry mutex poisoned").insert(
            name.into().to_ascii_uppercase(),
            CommandRegistration {
                handler: Arc::new(handler),
                mutating,
                skip_lazy_expire,
            },
        );
    }

    pub fn freeze(&self) {
        self.frozen.store(true, Ordering::SeqCst);
    }

    pub fn is_frozen(&self) -> bool {
        self.frozen.load(Ordering::SeqCst)
    }

    pub fn execute_with_db(
        &self,
        db_index: usize,
        command: &CommandFrame,
        record_aof: bool,
    ) -> (usize, RespValue) {
        self.execute_parts_with_db(db_index, &command.command, &command.args, record_aof)
    }

    pub fn execute_frame(&self, command: &CommandFrame) -> RespValue {
        let db_index = self.store.lock().expect("store mutex poisoned").active_db;
        self.execute_with_db(db_index, command, true).1
    }

    pub fn execute_parts(&self, command: &[Vec<u8>]) -> RespValue {
        match CommandFrame::from_parts(command.to_vec()) {
            Some(frame) => self.execute_frame(&frame),
            None => RespValue::Error(RespError::new(
                "ERR protocol error: expected array of bulk strings",
            )),
        }
    }

    pub fn store(&self) -> Store {
        self.store.lock().expect("store mutex poisoned").clone()
    }

    pub fn active_expire_all(&self) {
        if let Ok(mut store) = self.store.lock() {
            *store = store.clone().active_expire_all();
        }
    }

    fn execute_parts_with_db(
        &self,
        db_index: usize,
        command: &str,
        args: &[Vec<u8>],
        record_aof: bool,
    ) -> (usize, RespValue) {
        let command = command.to_ascii_uppercase();
        let registration = self
            .commands
            .lock()
            .expect("command registry mutex poisoned")
            .get(&command)
            .cloned();

        let Some(registration) = registration else {
            return (
                db_index,
                RespValue::Error(RespError::new(format!("ERR unknown command '{}'", command))),
            );
        };

        let mut store = self.store.lock().expect("store mutex poisoned").clone();
        store = store.with_active_db(db_index);
        if !registration.skip_lazy_expire {
            store = store.expire_lazy(args.first().map(|bytes| bytes.as_slice()));
        }

        let (new_store, response) = (registration.handler)(store, args);

        if record_aof
            && registration.mutating
            && command != "SELECT"
        {
            append_aof(self, &command, args);
        }

        let active_db = new_store.active_db;
        *self.store.lock().expect("store mutex poisoned") = new_store;
        (active_db, response)
    }

    fn replay_aof(&self, path: &Path) -> io::Result<()> {
        let bytes = std::fs::read(path)?;
        let (messages, _) = decode_all(&bytes).map_err(map_resp_decode_error)?;
        let mut db_index = self.store.lock().expect("store mutex poisoned").active_db;
        for message in messages {
            if let Some(frame) = command_frame_from_resp(message) {
                let (next_db, _) = self.execute_with_db(db_index, &frame, false);
                db_index = next_db;
            }
        }
        Ok(())
    }
}

impl DataStoreBackend for DataStoreEngine {
    fn execute_frame(&self, command: &CommandFrame) -> RespValue {
        DataStoreEngine::execute_frame(self, command)
    }

    fn store(&self) -> Store {
        DataStoreEngine::store(self)
    }

    fn active_expire_all(&self) {
        DataStoreEngine::active_expire_all(self);
    }
}

fn append_aof(engine: &DataStoreEngine, command: &str, args: &[Vec<u8>]) {
    let mut guard = engine.aof_file.lock().expect("aof file mutex poisoned");
    let Some(file) = guard.as_mut() else {
        return;
    };
    let payload = RespValue::Array(Some(
        std::iter::once(command.as_bytes().to_vec())
            .chain(args.iter().cloned())
            .map(|bytes| RespValue::BulkString(Some(bytes)))
            .collect(),
    ));
    if let Ok(encoded) = encode(payload) {
        let _ = file.write_all(&encoded);
        let _ = file.flush();
    }
}

fn map_resp_decode_error(err: resp_protocol::RespDecodeError) -> io::Error {
    io::Error::new(io::ErrorKind::InvalidData, err.message)
}
