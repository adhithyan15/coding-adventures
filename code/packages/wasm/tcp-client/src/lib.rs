//! # tcp-client-wasm
//!
//! WebAssembly **stub** for the tcp-client crate.
//!
//! Raw TCP sockets are **not available** in browser WebAssembly sandboxes.
//! The browser security model prohibits direct socket access — all networking
//! must go through higher-level APIs like `fetch()` or `WebSocket`.
//!
//! This crate exists so that downstream Wasm packages can depend on
//! tcp-client types for mocking/testing without a compile error. The
//! `connect()` function always returns an error explaining the limitation.
//!
//! ## Architecture
//!
//! ```text
//!   Browser Wasm sandbox
//!   ┌──────────────────────────────────┐
//!   │  JavaScript                      │
//!   │    │                             │
//!   │    ▼                             │
//!   │  tcp-client-wasm.connect()       │
//!   │    → ERROR: sockets unavailable  │
//!   │                                  │
//!   │  (Use fetch() or WebSocket       │
//!   │   for browser networking)        │
//!   └──────────────────────────────────┘
//! ```
//!
//! For server-side (Node.js) usage, use the TypeScript tcp-client package
//! which wraps Node's `net.Socket` API.

use wasm_bindgen::prelude::*;

pub const VERSION: &str = "0.1.0";

// ============================================================================
// Types exported to JavaScript (for downstream type-checking)
// ============================================================================

/// Connection options — exported for type compatibility with downstream
/// packages, even though `connect()` always fails in Wasm.
#[wasm_bindgen]
pub struct WasmConnectOptions {
    #[wasm_bindgen(getter_with_clone)]
    pub connect_timeout_ms: u32,
    #[wasm_bindgen(getter_with_clone)]
    pub read_timeout_ms: u32,
    #[wasm_bindgen(getter_with_clone)]
    pub write_timeout_ms: u32,
    pub buffer_size: u32,
}

#[wasm_bindgen]
impl WasmConnectOptions {
    #[wasm_bindgen(constructor)]
    pub fn new() -> WasmConnectOptions {
        WasmConnectOptions {
            connect_timeout_ms: 30000,
            read_timeout_ms: 30000,
            write_timeout_ms: 30000,
            buffer_size: 8192,
        }
    }
}

/// Attempt to establish a TCP connection.
///
/// **Always fails** in browser Wasm — TCP sockets are not available.
/// Returns a descriptive error message explaining the limitation.
#[wasm_bindgen]
pub fn connect(_host: &str, _port: u16) -> Result<(), JsValue> {
    Err(JsValue::from_str(
        "TCP sockets are not available in WebAssembly browser sandboxes. \
         Use the TypeScript tcp-client package for Node.js, or use \
         fetch()/WebSocket for browser networking.",
    ))
}

// ============================================================================
// Tests
// ============================================================================

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn connect_options_defaults() {
        let opts = WasmConnectOptions::new();
        assert_eq!(opts.connect_timeout_ms, 30000);
        assert_eq!(opts.read_timeout_ms, 30000);
        assert_eq!(opts.write_timeout_ms, 30000);
        assert_eq!(opts.buffer_size, 8192);
    }

    #[test]
    fn version_exists() {
        assert_eq!(VERSION, "0.1.0");
    }

    // Note: connect() returns Err(JsValue) which panics on non-wasm32 targets.
    // We test the underlying concept: connect should always fail in Wasm.
    // The actual JsValue error path is verified when built for wasm32.
}
