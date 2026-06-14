//! WebAssembly bindings for the Rust MACSYMA runtime JSON facade.

use coding_adventures_macsyma_wasm::{eval_source_json, MacsymaWasmSession};
use wasm_bindgen::prelude::*;

#[wasm_bindgen]
pub struct WasmMacsymaSession {
    inner: MacsymaWasmSession,
}

#[wasm_bindgen]
impl WasmMacsymaSession {
    #[wasm_bindgen(constructor)]
    pub fn new() -> Self {
        Self {
            inner: MacsymaWasmSession::new(),
        }
    }

    /// Evaluate MACSYMA source with this session's persistent bindings/history.
    #[wasm_bindgen]
    pub fn eval(&mut self, source: &str) -> String {
        self.inner.eval_json(source)
    }

    /// Return this session's current history counters as JSON.
    #[wasm_bindgen(js_name = "historyJson")]
    pub fn history_json(&self) -> String {
        self.inner.history_json()
    }

    /// Clear `%i`/`%o` history while preserving the session object.
    #[wasm_bindgen(js_name = "resetHistory")]
    pub fn reset_history(&mut self) {
        self.inner.reset_history();
    }
}

impl Default for WasmMacsymaSession {
    fn default() -> Self {
        Self::new()
    }
}

/// Evaluate MACSYMA source in a fresh session and return JSON.
#[wasm_bindgen(js_name = "evalSource")]
pub fn eval_source(source: &str) -> String {
    eval_source_json(source)
}

#[cfg(not(target_arch = "wasm32"))]
#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn single_shot_eval_exports_json() {
        let json = eval_source("1 + 2;");
        assert!(json.contains("\"ok\":true"));
        assert!(json.contains("\"output_macsyma\":\"3\""));
    }

    #[test]
    fn session_preserves_bindings() {
        let mut session = WasmMacsymaSession::new();
        session.eval("x : 5$");
        let json = session.eval("x + 1;");
        assert!(json.contains("\"input_index\":2"));
        assert!(json.contains("\"output_macsyma\":\"6\""));
    }
}
