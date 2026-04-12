use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::{Arc, Mutex};
use std::thread::{self, JoinHandle};
use std::time::Duration;
use std::path::PathBuf;

use in_memory_data_store_engine::DataStoreEngine;
use in_memory_data_store_protocol::{CommandFrame, EngineResponse};

pub struct DataStoreManager {
    engine: Arc<DataStoreEngine>,
    expirer_stop: Arc<AtomicBool>,
    expirer_handle: Mutex<Option<JoinHandle<()>>>,
}

impl DataStoreManager {
    pub fn new(aof_path: Option<PathBuf>) -> std::io::Result<Self> {
        let engine = Arc::new(DataStoreEngine::new(aof_path)?);

        Ok(Self {
            engine,
            expirer_stop: Arc::new(AtomicBool::new(false)),
            expirer_handle: Mutex::new(None),
        })
    }

    pub fn start_background_workers(&self) {
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

    pub fn stop_background_workers(&self) {
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

    pub fn execute(&self, selected_db: &mut usize, frame: &CommandFrame) -> EngineResponse {
        let (active_db, response) = self.engine.execute_with_db(*selected_db, frame, true);
        *selected_db = active_db;
        response
    }
}

impl Drop for DataStoreManager {
    fn drop(&mut self) {
        self.stop_background_workers();
    }
}
