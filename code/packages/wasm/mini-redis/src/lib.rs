mod resp_adapter;

use in_memory_data_store::DataStoreManager;
use resp_protocol::{decode, encode, RespError, RespValue};
use resp_adapter::{command_frame_from_resp, engine_response_to_resp};
use wasm_bindgen::prelude::*;

#[wasm_bindgen]
pub struct MiniRedisWasm {
    manager: DataStoreManager,
    selected_db: usize,
    buffer: Vec<u8>,
}

#[wasm_bindgen]
impl MiniRedisWasm {
    #[wasm_bindgen(constructor)]
    pub fn new() -> Result<MiniRedisWasm, JsValue> {
        let manager = DataStoreManager::new(None)
            .map_err(|e| JsValue::from_str(&format!("Failed to initialize Pipeline: {}", e)))?;
        
        // No background worker thread starts since WASM is single threaded.
        
        Ok(Self {
            manager,
            selected_db: 0,
            buffer: Vec::new(),
        })
    }

    #[wasm_bindgen]
    pub fn execute(&mut self, data: &[u8]) -> Vec<u8> {
        self.buffer.extend_from_slice(data);
        
        let mut responses = Vec::new();
        loop {
            match decode(&self.buffer) {
                Ok(Some((value, consumed))) => {
                    self.buffer.drain(..consumed);
                    let Some(frame) = command_frame_from_resp(value) else {
                        let response = RespValue::Error(RespError::new(
                            "ERR protocol error: expected array of bulk strings",
                        ));
                        responses.extend(encode(response).unwrap());
                        continue;
                    };
                    
                    let engine_resp = self.manager.execute(&mut self.selected_db, &frame);
                    let resp_val = engine_response_to_resp(engine_resp);
                    responses.extend(encode(resp_val).unwrap());
                }
                Ok(None) => break,
                Err(err) => {
                    self.buffer.clear();
                    let response = RespValue::Error(RespError::new(format!("ERR {err}")));
                    responses.extend(encode(response).unwrap());
                    break;
                }
            }
        }
        
        responses
    }
}

impl Default for MiniRedisWasm {
    fn default() -> Self {
        Self::new().unwrap()
    }
}
