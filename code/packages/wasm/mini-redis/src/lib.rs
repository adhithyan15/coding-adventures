use in_memory_data_store::DataStorePipeline;
use wasm_bindgen::prelude::*;

#[wasm_bindgen]
pub struct MiniRedisWasm {
    pipeline: DataStorePipeline,
    selected_db: usize,
    buffer: Vec<u8>,
}

#[wasm_bindgen]
impl MiniRedisWasm {
    #[wasm_bindgen(constructor)]
    pub fn new() -> Result<MiniRedisWasm, JsValue> {
        let pipeline = DataStorePipeline::new(None)
            .map_err(|e| JsValue::from_str(&format!("Failed to initialize Pipeline: {}", e)))?;
        
        // No background worker thread starts since WASM is single threaded.
        
        Ok(Self {
            pipeline,
            selected_db: 0,
            buffer: Vec::new(),
        })
    }

    #[wasm_bindgen]
    pub fn execute(&mut self, data: &[u8]) -> Vec<u8> {
        self.buffer.extend_from_slice(data);
        self.pipeline.execute(&mut self.buffer, &mut self.selected_db)
    }
}

impl Default for MiniRedisWasm {
    fn default() -> Self {
        Self::new().unwrap()
    }
}
