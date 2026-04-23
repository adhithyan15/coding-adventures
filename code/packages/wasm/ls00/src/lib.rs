// lib.rs -- WASM Bindings for the ls00 LSP Framework
// ====================================================
//
// This module wraps the Rust `coding-adventures-ls00` LSP framework for use
// in browsers and web workers via WebAssembly. The core challenge is that
// browsers do not have stdio -- there is no stdin to read from and no stdout
// to write to. This wrapper replaces the stdio transport with a callback-based
// message-passing API.
//
// # Architecture
//
// ```text
// ┌───────────────────────────────────┐
// │  Browser / Web Worker / Deno      │
// │                                   │
// │  const server = new WasmLspServer(│
// │    (json) => sendToEditor(json)   │  <-- outgoing callback
// │  );                               │
// │                                   │
// │  server.handleMessage(incomingJson);  <-- incoming messages
// └───────────────────────────────────┘
//        |                    ^
//        v                    |
// ┌───────────────────────────────────┐
// │  WasmLspServer (this crate)       │
// │                                   │
// │  ┌──────────────────────────────┐ │
// │  │ DocumentManager              │ │  tracks open file contents
// │  │ ParseCache                   │ │  avoids redundant parses
// │  │ LanguageBridge (from JS)     │ │  language-specific logic
// │  │ Capability builder           │ │  advertises LSP features
// │  └──────────────────────────────┘ │
// └───────────────────────────────────┘
// ```
//
// # Design Decisions
//
// 1. **Message-passing, not stdio**: The server accepts incoming JSON-RPC
//    messages as strings via `handleMessage()` and sends outgoing messages
//    through a JS callback function. This matches how web workers and
//    browser extensions communicate.
//
// 2. **JS-based LanguageBridge**: The `WasmLanguageBridge` struct implements
//    the Rust `LanguageBridge` trait by calling JS callback functions for
//    `tokenize` and `parse`. This lets language-specific logic live in
//    JavaScript/TypeScript while reusing the Rust LSP framework.
//
// 3. **JSON serialization at the boundary**: All data crosses the WASM
//    boundary as JSON strings. This avoids complex wasm-bindgen type
//    mappings and lets JS consumers work with plain objects.
//
// 4. **No WASM-target tests**: Tests run natively with `cargo test` and
//    are guarded with `#[cfg(not(target_arch = "wasm32"))]`.

use coding_adventures_ls00::capabilities::build_capabilities;
use coding_adventures_ls00::document_manager::DocumentManager;
use coding_adventures_ls00::language_bridge::LanguageBridge;
use coding_adventures_ls00::parse_cache::ParseCache;
use coding_adventures_ls00::types::*;
use serde_json::{json, Value};
use std::any::Any;
use wasm_bindgen::prelude::*;

// ---------------------------------------------------------------------------
// WasmLanguageBridge -- language logic provided by JavaScript
// ---------------------------------------------------------------------------
//
// In the native Rust LSP server, the LanguageBridge is a Rust struct that
// implements tokenize() and parse(). In the WASM version, these operations
// are implemented in JavaScript. The WasmLanguageBridge stores JS callback
// functions and calls them when the server needs to tokenize or parse.
//
// The JS callbacks receive and return JSON strings:
//
//   tokenize(source: string) -> string  // returns JSON: Token[] or {error: string}
//   parse(source: string) -> string     // returns JSON: {ast: any, diagnostics: Diagnostic[]}
//                                       //   or {error: string}
//
// This keeps the WASM boundary simple -- everything is a string.

/// A language bridge that delegates to JavaScript callback functions.
///
/// Create one with `WasmLanguageBridge::new(tokenize_fn, parse_fn)` where
/// each function takes a source string and returns a JSON string with the
/// results.
///
/// ## JavaScript Usage
///
/// ```javascript
/// const bridge = new WasmLanguageBridge(
///   (source) => {
///     // tokenize the source, return JSON array of tokens
///     const tokens = myLexer.tokenize(source);
///     return JSON.stringify(tokens);
///   },
///   (source) => {
///     // parse the source, return JSON with ast + diagnostics
///     const result = myParser.parse(source);
///     return JSON.stringify({
///       ast: result.ast,
///       diagnostics: result.diagnostics
///     });
///   }
/// );
/// ```
#[wasm_bindgen]
pub struct WasmLanguageBridge {
    /// JS function: (source: string) -> JSON string of Token[]
    tokenize_fn: js_sys::Function,
    /// JS function: (source: string) -> JSON string of {ast, diagnostics}
    parse_fn: js_sys::Function,
}

#[wasm_bindgen]
impl WasmLanguageBridge {
    /// Create a new language bridge from two JavaScript callback functions.
    ///
    /// - `tokenize_fn`: `(source: string) -> string` -- returns JSON array of tokens.
    ///   Each token must have: `{ token_type: string, value: string, line: number, column: number }`
    ///
    /// - `parse_fn`: `(source: string) -> string` -- returns JSON object with:
    ///   `{ ast: string, diagnostics: [...] }` where `ast` is an opaque JSON string
    ///   that will be passed back to JS for hover/completion/etc., and `diagnostics`
    ///   is an array of `{ range: {start: {line, character}, end: {line, character}},
    ///   severity: number, message: string, code?: string }`.
    #[wasm_bindgen(constructor)]
    pub fn new(tokenize_fn: js_sys::Function, parse_fn: js_sys::Function) -> WasmLanguageBridge {
        WasmLanguageBridge {
            tokenize_fn,
            parse_fn,
        }
    }
}

// ---------------------------------------------------------------------------
// Implementing the Rust LanguageBridge trait for WasmLanguageBridge
// ---------------------------------------------------------------------------
//
// This is the key integration point. The Rust LSP server calls these methods
// when it needs to tokenize or parse a document. We forward the calls to
// JavaScript by invoking the stored callback functions.
//
// The `Send + Sync` requirement is satisfied because WASM is single-threaded:
// there is only one thread, so all types are trivially Send + Sync.

// SAFETY: WASM is single-threaded, so js_sys::Function can be sent between
// "threads" (there is only one). This is the standard pattern for wasm-bindgen.
unsafe impl Send for WasmLanguageBridge {}
unsafe impl Sync for WasmLanguageBridge {}

/// An opaque AST container that wraps a JSON string.
///
/// When the JS `parse_fn` returns an AST, we store it as a JSON string inside
/// this struct. When the server later needs the AST for hover/completion/etc.,
/// it passes this value back. Since the WASM wrapper doesn't interpret the AST
/// itself (that's the JS bridge's job), the JSON string is opaque to us.
struct JsAst {
    /// The opaque AST JSON string from the JS bridge. Stored for potential
    /// future use when the bridge needs to downcast and inspect the AST.
    #[allow(dead_code)]
    json: String,
}

impl LanguageBridge for WasmLanguageBridge {
    /// Tokenize the source by calling the JS tokenize callback.
    ///
    /// The JS function receives the source string and must return a JSON
    /// string representing an array of tokens:
    ///
    /// ```json
    /// [
    ///   {"token_type": "KEYWORD", "value": "let", "line": 1, "column": 1},
    ///   {"token_type": "IDENTIFIER", "value": "x", "line": 1, "column": 5}
    /// ]
    /// ```
    ///
    /// If the JS function returns an object with an `error` field, that error
    /// is propagated as `Err(message)`.
    fn tokenize(&self, source: &str) -> Result<Vec<Token>, String> {
        // Call the JS function with the source string.
        let this = JsValue::NULL;
        let source_js = JsValue::from_str(source);
        let result = self
            .tokenize_fn
            .call1(&this, &source_js)
            .map_err(|e| format!("tokenize callback failed: {:?}", e))?;

        // Convert the JsValue result to a Rust string.
        let json_str = result
            .as_string()
            .ok_or_else(|| "tokenize callback did not return a string".to_string())?;

        // Parse the JSON string.
        let parsed: Value =
            serde_json::from_str(&json_str).map_err(|e| format!("invalid JSON from tokenize: {}", e))?;

        // Check for error response.
        if let Some(err) = parsed.get("error") {
            return Err(err.as_str().unwrap_or("unknown error").to_string());
        }

        // Parse as array of tokens.
        let arr = parsed
            .as_array()
            .ok_or_else(|| "tokenize result is not an array".to_string())?;

        let tokens: Vec<Token> = arr
            .iter()
            .map(|item| Token {
                token_type: item["token_type"]
                    .as_str()
                    .unwrap_or("UNKNOWN")
                    .to_string(),
                value: item["value"].as_str().unwrap_or("").to_string(),
                line: item["line"].as_i64().unwrap_or(1) as i32,
                column: item["column"].as_i64().unwrap_or(1) as i32,
            })
            .collect();

        Ok(tokens)
    }

    /// Parse the source by calling the JS parse callback.
    ///
    /// The JS function receives the source string and must return a JSON string:
    ///
    /// ```json
    /// {
    ///   "ast": "<opaque JSON string representing the AST>",
    ///   "diagnostics": [
    ///     {
    ///       "range": {
    ///         "start": {"line": 0, "character": 0},
    ///         "end": {"line": 0, "character": 5}
    ///       },
    ///       "severity": 1,
    ///       "message": "unexpected token"
    ///     }
    ///   ]
    /// }
    /// ```
    ///
    /// The `ast` field is stored as an opaque string and will be passed back
    /// to the JS bridge when hover/completion/etc. are requested.
    fn parse(
        &self,
        source: &str,
    ) -> Result<(Box<dyn Any + Send + Sync>, Vec<Diagnostic>), String> {
        let this = JsValue::NULL;
        let source_js = JsValue::from_str(source);
        let result = self
            .parse_fn
            .call1(&this, &source_js)
            .map_err(|e| format!("parse callback failed: {:?}", e))?;

        let json_str = result
            .as_string()
            .ok_or_else(|| "parse callback did not return a string".to_string())?;

        let parsed: Value =
            serde_json::from_str(&json_str).map_err(|e| format!("invalid JSON from parse: {}", e))?;

        // Check for error response.
        if let Some(err) = parsed.get("error") {
            return Err(err.as_str().unwrap_or("unknown error").to_string());
        }

        // Extract the opaque AST string.
        let ast_str = match &parsed["ast"] {
            Value::String(s) => s.clone(),
            other => serde_json::to_string(other).unwrap_or_default(),
        };

        // Extract diagnostics.
        let diagnostics = parse_diagnostics_from_json(&parsed["diagnostics"]);

        let ast = Box::new(JsAst { json: ast_str }) as Box<dyn Any + Send + Sync>;
        Ok((ast, diagnostics))
    }
}

// ---------------------------------------------------------------------------
// Diagnostic parsing helper
// ---------------------------------------------------------------------------

/// Parse an array of diagnostics from a JSON value.
///
/// Each diagnostic has the shape:
/// ```json
/// {
///   "range": {
///     "start": {"line": 0, "character": 0},
///     "end": {"line": 0, "character": 5}
///   },
///   "severity": 1,
///   "message": "unexpected token",
///   "code": "E001"  // optional
/// }
/// ```
fn parse_diagnostics_from_json(value: &Value) -> Vec<Diagnostic> {
    let arr = match value.as_array() {
        Some(a) => a,
        None => return Vec::new(),
    };

    arr.iter()
        .map(|item| {
            let range = &item["range"];
            Diagnostic {
                range: Range {
                    start: Position {
                        line: range["start"]["line"].as_i64().unwrap_or(0) as i32,
                        character: range["start"]["character"].as_i64().unwrap_or(0) as i32,
                    },
                    end: Position {
                        line: range["end"]["line"].as_i64().unwrap_or(0) as i32,
                        character: range["end"]["character"].as_i64().unwrap_or(0) as i32,
                    },
                },
                severity: match item["severity"].as_i64().unwrap_or(1) {
                    2 => DiagnosticSeverity::Warning,
                    3 => DiagnosticSeverity::Information,
                    4 => DiagnosticSeverity::Hint,
                    _ => DiagnosticSeverity::Error,
                },
                message: item["message"].as_str().unwrap_or("").to_string(),
                code: item["code"].as_str().map(|s| s.to_string()),
            }
        })
        .collect()
}

// ---------------------------------------------------------------------------
// WasmLspServer -- the main exported type
// ---------------------------------------------------------------------------
//
// This is the WASM equivalent of `LspServer` from the core crate. Instead
// of reading from stdin and writing to stdout, it:
//
// - Receives incoming JSON-RPC messages as strings via `handleMessage()`
// - Sends outgoing JSON-RPC messages via a JS callback function
//
// Internally it reuses the same DocumentManager and ParseCache from the
// core crate, and dispatches to the same handler logic.
//
// JavaScript usage:
//
//   import { WasmLspServer, WasmLanguageBridge } from './ls00_wasm.js';
//
//   const bridge = new WasmLanguageBridge(tokenizeFn, parseFn);
//   const server = new WasmLspServer(bridge, (outJson) => {
//     // send outJson to the editor
//     postMessage(outJson);
//   });
//
//   // When a message arrives from the editor:
//   onmessage = (event) => {
//     const response = server.handleMessage(event.data);
//     if (response) {
//       postMessage(response);
//     }
//   };

/// An LSP server that runs in WebAssembly.
///
/// Instead of stdio, this server communicates via:
/// - `handleMessage(json)` -- receives incoming JSON-RPC messages
/// - A callback function -- sends outgoing notifications (diagnostics, etc.)
///
/// The callback is called whenever the server needs to push a message to the
/// client (e.g., `textDocument/publishDiagnostics`). Request responses are
/// returned directly from `handleMessage()`.
#[wasm_bindgen]
pub struct WasmLspServer {
    /// The language bridge (owned, moved in from WasmLanguageBridge).
    bridge: Box<dyn LanguageBridge>,
    /// Tracks open documents and their current contents.
    doc_manager: DocumentManager,
    /// Caches parse results to avoid redundant parsing.
    parse_cache: ParseCache,
    /// JS callback for sending outgoing notifications.
    /// Wrapped in Option so that native tests can create a server without a
    /// real JS function (js_sys::Function panics outside WASM targets).
    send_callback: Option<js_sys::Function>,
    /// Whether the server has been initialized (received "initialize").
    initialized: bool,
    /// Whether the server has received "shutdown".
    shutdown: bool,
}

#[wasm_bindgen]
impl WasmLspServer {
    /// Create a new WASM LSP server.
    ///
    /// - `bridge`: a `WasmLanguageBridge` that provides tokenize/parse logic
    /// - `send_callback`: a JS function `(json: string) -> void` that will be
    ///   called whenever the server needs to send a notification to the client
    ///   (e.g., diagnostics after a file is opened or changed)
    ///
    /// ## Example (JavaScript)
    ///
    /// ```javascript
    /// const bridge = new WasmLanguageBridge(tokenize, parse);
    /// const server = new WasmLspServer(bridge, (json) => {
    ///   // Send to editor via WebSocket, postMessage, etc.
    ///   socket.send(json);
    /// });
    /// ```
    #[wasm_bindgen(constructor)]
    pub fn new(bridge: WasmLanguageBridge, send_callback: js_sys::Function) -> WasmLspServer {
        WasmLspServer {
            bridge: Box::new(bridge),
            doc_manager: DocumentManager::new(),
            parse_cache: ParseCache::new(),
            send_callback: Some(send_callback),
            initialized: false,
            shutdown: false,
        }
    }

    /// Process an incoming JSON-RPC message and return the response (if any).
    ///
    /// The input `json` should be a complete JSON-RPC 2.0 message object as a
    /// string. The return value is:
    /// - A JSON string containing the response (for requests)
    /// - An empty string (for notifications, which don't produce responses)
    ///
    /// Any server-initiated notifications (like `publishDiagnostics`) are sent
    /// via the `send_callback` provided in the constructor, NOT returned here.
    ///
    /// ## Message Flow
    ///
    /// ```text
    /// JS calls handleMessage('{"jsonrpc":"2.0","id":1,"method":"initialize",...}')
    ///   -> returns '{"jsonrpc":"2.0","id":1,"result":{...capabilities...}}'
    ///
    /// JS calls handleMessage('{"jsonrpc":"2.0","method":"textDocument/didOpen",...}')
    ///   -> returns ""  (notification, no response)
    ///   -> send_callback is called with publishDiagnostics notification
    /// ```
    #[wasm_bindgen(js_name = "handleMessage")]
    pub fn handle_message(&mut self, json: &str) -> String {
        // Parse the incoming JSON-RPC message.
        let msg: Value = match serde_json::from_str(json) {
            Ok(v) => v,
            Err(e) => {
                // Return a JSON-RPC parse error response.
                return serde_json::to_string(&json!({
                    "jsonrpc": "2.0",
                    "id": null,
                    "error": {
                        "code": -32700,
                        "message": format!("Parse error: {}", e)
                    }
                }))
                .unwrap_or_default();
            }
        };

        // Determine if this is a request (has "id") or notification (no "id").
        let id = msg.get("id").cloned();
        let method = msg["method"].as_str().unwrap_or("");

        if let Some(id) = id {
            // This is a request -- dispatch and return the response.
            let params = msg.get("params").cloned();
            let result = self.dispatch_request(method, params);

            let response = match result {
                Ok(value) => json!({
                    "jsonrpc": "2.0",
                    "id": id,
                    "result": value
                }),
                Err((code, message)) => json!({
                    "jsonrpc": "2.0",
                    "id": id,
                    "error": {
                        "code": code,
                        "message": message
                    }
                }),
            };

            serde_json::to_string(&response).unwrap_or_default()
        } else {
            // This is a notification -- dispatch and return empty string.
            let params = msg.get("params").cloned();
            self.dispatch_notification(method, params);
            String::new()
        }
    }

    /// Check whether the server has been initialized.
    ///
    /// Returns `true` after the `initialize` request has been processed.
    #[wasm_bindgen(js_name = "isInitialized")]
    pub fn is_initialized(&self) -> bool {
        self.initialized
    }

    /// Check whether the server has received a shutdown request.
    #[wasm_bindgen(js_name = "isShutdown")]
    pub fn is_shutdown(&self) -> bool {
        self.shutdown
    }
}

// ---------------------------------------------------------------------------
// Internal dispatch logic
// ---------------------------------------------------------------------------
//
// These methods are NOT exported to JavaScript (no #[wasm_bindgen]).
// They contain the core message dispatch logic, reusing the same handler
// patterns from the Rust LspServer but adapted for the message-passing API.

impl WasmLspServer {
    /// Dispatch a JSON-RPC request to the appropriate handler.
    ///
    /// Returns `Ok(Value)` for success or `Err((code, message))` for errors.
    /// The error format uses standard JSON-RPC error codes:
    /// - `-32601` for "method not found"
    /// - `-32602` for "invalid params"
    /// - `-32603` for "internal error"
    fn dispatch_request(
        &mut self,
        method: &str,
        params: Option<Value>,
    ) -> Result<Value, (i32, String)> {
        match method {
            "initialize" => self.handle_initialize(params),
            "shutdown" => self.handle_shutdown(),
            "textDocument/hover" => self.handle_hover(params),
            "textDocument/definition" => self.handle_definition(params),
            "textDocument/references" => self.handle_references(params),
            "textDocument/completion" => self.handle_completion(params),
            "textDocument/rename" => self.handle_rename(params),
            "textDocument/documentSymbol" => self.handle_document_symbol(params),
            "textDocument/semanticTokens/full" => self.handle_semantic_tokens_full(params),
            "textDocument/foldingRange" => self.handle_folding_range(params),
            "textDocument/signatureHelp" => self.handle_signature_help(params),
            "textDocument/formatting" => self.handle_formatting(params),
            _ => Err((-32601, format!("Method not found: {}", method))),
        }
    }

    /// Dispatch a JSON-RPC notification.
    ///
    /// Notifications have no response. Side effects (like sending diagnostics)
    /// are delivered via the send_callback.
    fn dispatch_notification(&mut self, method: &str, params: Option<Value>) {
        match method {
            "initialized" => { /* handshake complete, no-op */ }
            "textDocument/didOpen" => self.handle_did_open(params),
            "textDocument/didChange" => self.handle_did_change(params),
            "textDocument/didClose" => self.handle_did_close(params),
            "textDocument/didSave" => self.handle_did_save(params),
            _ => { /* unknown notifications are silently dropped */ }
        }
    }

    // -----------------------------------------------------------------------
    // Lifecycle handlers
    // -----------------------------------------------------------------------

    /// Handle `initialize` -- return server capabilities.
    ///
    /// This builds the capabilities object by inspecting which features the
    /// language bridge supports (hover, completion, etc.) and returns them
    /// to the editor so it knows what requests to send.
    fn handle_initialize(&mut self, _params: Option<Value>) -> Result<Value, (i32, String)> {
        self.initialized = true;

        let caps = build_capabilities(self.bridge.as_ref());

        Ok(json!({
            "capabilities": caps,
            "serverInfo": {
                "name": "ls00-wasm-lsp-server",
                "version": "0.1.0"
            }
        }))
    }

    /// Handle `shutdown` -- mark the server as shut down.
    fn handle_shutdown(&mut self) -> Result<Value, (i32, String)> {
        self.shutdown = true;
        Ok(Value::Null)
    }

    // -----------------------------------------------------------------------
    // Document synchronization handlers
    // -----------------------------------------------------------------------

    /// Handle `textDocument/didOpen` -- open a document and publish diagnostics.
    ///
    /// When the editor opens a file, it sends the full text. We store it in
    /// the document manager, parse it, and push diagnostics to the editor.
    fn handle_did_open(&mut self, params: Option<Value>) {
        let params = match params {
            Some(p) => p,
            None => return,
        };

        let td = &params["textDocument"];
        let uri = td["uri"].as_str().unwrap_or("").to_string();
        let text = td["text"].as_str().unwrap_or("").to_string();
        let version = td["version"].as_i64().unwrap_or(1) as i32;

        if uri.is_empty() {
            return;
        }

        self.doc_manager.open(&uri, &text, version);

        let result = self
            .parse_cache
            .get_or_parse(&uri, version, &text, self.bridge.as_ref());
        let diags = result.diagnostics.clone();
        self.publish_diagnostics(&uri, version, &diags);
    }

    /// Handle `textDocument/didChange` -- apply incremental changes.
    ///
    /// The editor sends only the changed portions of the document. We apply
    /// the changes, re-parse, and push updated diagnostics.
    fn handle_did_change(&mut self, params: Option<Value>) {
        let params = match params {
            Some(p) => p,
            None => return,
        };

        let uri = params["textDocument"]["uri"]
            .as_str()
            .unwrap_or("")
            .to_string();
        if uri.is_empty() {
            return;
        }

        let version = params["textDocument"]["version"]
            .as_i64()
            .unwrap_or(0) as i32;

        let changes_raw = match params["contentChanges"].as_array() {
            Some(arr) => arr.clone(),
            None => return,
        };

        let changes: Vec<coding_adventures_ls00::document_manager::TextChange> = changes_raw
            .iter()
            .map(|change| {
                let new_text = change["text"].as_str().unwrap_or("").to_string();
                let range = if change.get("range").is_some() && !change["range"].is_null() {
                    let r = &change["range"];
                    Some(Range {
                        start: Position {
                            line: r["start"]["line"].as_i64().unwrap_or(0) as i32,
                            character: r["start"]["character"].as_i64().unwrap_or(0) as i32,
                        },
                        end: Position {
                            line: r["end"]["line"].as_i64().unwrap_or(0) as i32,
                            character: r["end"]["character"].as_i64().unwrap_or(0) as i32,
                        },
                    })
                } else {
                    None
                };
                coding_adventures_ls00::document_manager::TextChange { range, new_text }
            })
            .collect();

        if self
            .doc_manager
            .apply_changes(&uri, &changes, version)
            .is_err()
        {
            return;
        }

        let doc = match self.doc_manager.get(&uri) {
            Some(d) => d,
            None => return,
        };

        let doc_text = doc.text.clone();
        let doc_version = doc.version;

        let result = self
            .parse_cache
            .get_or_parse(&uri, doc_version, &doc_text, self.bridge.as_ref());
        let diags = result.diagnostics.clone();
        self.publish_diagnostics(&uri, version, &diags);
    }

    /// Handle `textDocument/didClose` -- remove the document.
    fn handle_did_close(&mut self, params: Option<Value>) {
        let params = match params {
            Some(p) => p,
            None => return,
        };

        let uri = params["textDocument"]["uri"]
            .as_str()
            .unwrap_or("")
            .to_string();
        if uri.is_empty() {
            return;
        }

        self.doc_manager.close(&uri);
        self.parse_cache.evict(&uri);
        self.publish_diagnostics(&uri, 0, &[]);
    }

    /// Handle `textDocument/didSave` -- optionally re-parse with saved content.
    fn handle_did_save(&mut self, params: Option<Value>) {
        let params = match params {
            Some(p) => p,
            None => return,
        };

        let uri = params["textDocument"]["uri"]
            .as_str()
            .unwrap_or("")
            .to_string();
        if uri.is_empty() {
            return;
        }

        if let Some(text) = params["text"].as_str() {
            if !text.is_empty() {
                if let Some(doc) = self.doc_manager.get(&uri) {
                    let version = doc.version;
                    self.doc_manager.close(&uri);
                    self.doc_manager.open(&uri, text, version);
                    let result = self
                        .parse_cache
                        .get_or_parse(&uri, version, text, self.bridge.as_ref());
                    let diags = result.diagnostics.clone();
                    self.publish_diagnostics(&uri, version, &diags);
                }
            }
        }
    }

    // -----------------------------------------------------------------------
    // Feature handlers
    // -----------------------------------------------------------------------
    //
    // Each feature handler follows the same pattern from the core crate:
    // 1. Extract URI and position from params.
    // 2. Check if the bridge supports the feature.
    // 3. Get the parse result.
    // 4. Delegate to the bridge.
    // 5. Convert the result to LSP JSON format.

    /// Handle `textDocument/hover` -- return hover information.
    fn handle_hover(&mut self, params: Option<Value>) -> Result<Value, (i32, String)> {
        let params = params.ok_or((-32602, "Missing params".to_string()))?;
        let uri = extract_uri(&params);
        let pos = extract_position(&params);

        if !self.bridge.supports_hover() {
            return Ok(Value::Null);
        }

        let (ast, _) = self.get_parse_result(&uri)?;
        let ast = match &ast {
            Some(a) => a,
            None => return Ok(Value::Null),
        };

        match self.bridge.hover(ast.as_ref(), pos) {
            None | Some(Err(_)) | Some(Ok(None)) => Ok(Value::Null),
            Some(Ok(Some(hover))) => {
                let mut result = json!({
                    "contents": {
                        "kind": "markdown",
                        "value": hover.contents
                    }
                });
                if let Some(range) = &hover.range {
                    result["range"] = range_to_json(range);
                }
                Ok(result)
            }
        }
    }

    /// Handle `textDocument/definition` -- return definition location.
    fn handle_definition(&mut self, params: Option<Value>) -> Result<Value, (i32, String)> {
        let params = params.ok_or((-32602, "Missing params".to_string()))?;
        let uri = extract_uri(&params);
        let pos = extract_position(&params);

        if !self.bridge.supports_definition() {
            return Ok(Value::Null);
        }

        let (ast, _) = self.get_parse_result(&uri)?;
        let ast = match &ast {
            Some(a) => a,
            None => return Ok(Value::Null),
        };

        match self.bridge.definition(ast.as_ref(), pos, &uri) {
            None | Some(Err(_)) | Some(Ok(None)) => Ok(Value::Null),
            Some(Ok(Some(loc))) => Ok(location_to_json(&loc)),
        }
    }

    /// Handle `textDocument/references` -- return all references.
    fn handle_references(&mut self, params: Option<Value>) -> Result<Value, (i32, String)> {
        let params = params.ok_or((-32602, "Missing params".to_string()))?;
        let uri = extract_uri(&params);
        let pos = extract_position(&params);
        let include_decl = params["context"]["includeDeclaration"]
            .as_bool()
            .unwrap_or(false);

        if !self.bridge.supports_references() {
            return Ok(json!([]));
        }

        let (ast, _) = self.get_parse_result(&uri)?;
        let ast = match &ast {
            Some(a) => a,
            None => return Ok(json!([])),
        };

        match self
            .bridge
            .references(ast.as_ref(), pos, &uri, include_decl)
        {
            None | Some(Err(_)) => Ok(json!([])),
            Some(Ok(locs)) => {
                let result: Vec<Value> = locs.iter().map(location_to_json).collect();
                Ok(Value::Array(result))
            }
        }
    }

    /// Handle `textDocument/completion` -- return autocomplete suggestions.
    fn handle_completion(&mut self, params: Option<Value>) -> Result<Value, (i32, String)> {
        let params = params.ok_or((-32602, "Missing params".to_string()))?;
        let uri = extract_uri(&params);
        let pos = extract_position(&params);

        let empty = json!({"isIncomplete": false, "items": []});

        if !self.bridge.supports_completion() {
            return Ok(empty);
        }

        let (ast, _) = self.get_parse_result(&uri)?;
        let ast = match &ast {
            Some(a) => a,
            None => return Ok(empty),
        };

        match self.bridge.completion(ast.as_ref(), pos) {
            None | Some(Err(_)) => Ok(empty),
            Some(Ok(items)) => {
                let lsp_items: Vec<Value> = items
                    .iter()
                    .map(|item| {
                        let mut ci = json!({"label": item.label});
                        if let Some(kind) = &item.kind {
                            ci["kind"] = json!(*kind as i32);
                        }
                        if let Some(detail) = &item.detail {
                            if !detail.is_empty() {
                                ci["detail"] = json!(detail);
                            }
                        }
                        if let Some(doc) = &item.documentation {
                            if !doc.is_empty() {
                                ci["documentation"] = json!(doc);
                            }
                        }
                        if let Some(text) = &item.insert_text {
                            if !text.is_empty() {
                                ci["insertText"] = json!(text);
                            }
                        }
                        if let Some(fmt) = &item.insert_text_format {
                            if *fmt != 0 {
                                ci["insertTextFormat"] = json!(fmt);
                            }
                        }
                        ci
                    })
                    .collect();

                Ok(json!({"isIncomplete": false, "items": lsp_items}))
            }
        }
    }

    /// Handle `textDocument/rename` -- return workspace edits for renaming.
    fn handle_rename(&mut self, params: Option<Value>) -> Result<Value, (i32, String)> {
        let params = params.ok_or((-32602, "Missing params".to_string()))?;
        let uri = extract_uri(&params);
        let pos = extract_position(&params);
        let new_name = params["newName"].as_str().unwrap_or("");

        if new_name.is_empty() {
            return Err((-32602, "newName is required".to_string()));
        }

        if !self.bridge.supports_rename() {
            return Err((-32803, "Rename not supported".to_string()));
        }

        let (ast, _) = self.get_parse_result(&uri)?;
        let ast = match &ast {
            Some(a) => a,
            None => return Err((-32803, "No AST available".to_string())),
        };

        match self.bridge.rename(ast.as_ref(), pos, new_name) {
            None | Some(Ok(None)) => Err((-32803, "Symbol not found at position".to_string())),
            Some(Err(e)) => Err((-32803, e)),
            Some(Ok(Some(edit))) => {
                let mut lsp_changes = serde_json::Map::new();
                for (edit_uri, edits) in &edit.changes {
                    let lsp_edits: Vec<Value> = edits
                        .iter()
                        .map(|te| {
                            json!({
                                "range": range_to_json(&te.range),
                                "newText": te.new_text
                            })
                        })
                        .collect();
                    lsp_changes.insert(edit_uri.clone(), Value::Array(lsp_edits));
                }
                Ok(json!({"changes": lsp_changes}))
            }
        }
    }

    /// Handle `textDocument/documentSymbol` -- return the document outline.
    fn handle_document_symbol(&mut self, params: Option<Value>) -> Result<Value, (i32, String)> {
        let params = params.ok_or((-32602, "Missing params".to_string()))?;
        let uri = extract_uri(&params);

        if !self.bridge.supports_document_symbols() {
            return Ok(json!([]));
        }

        let (ast, _) = self.get_parse_result(&uri)?;
        let ast = match &ast {
            Some(a) => a,
            None => return Ok(json!([])),
        };

        match self.bridge.document_symbols(ast.as_ref()) {
            None | Some(Err(_)) => Ok(json!([])),
            Some(Ok(symbols)) => Ok(Value::Array(convert_document_symbols(&symbols))),
        }
    }

    /// Handle `textDocument/semanticTokens/full` -- return semantic tokens.
    fn handle_semantic_tokens_full(
        &mut self,
        params: Option<Value>,
    ) -> Result<Value, (i32, String)> {
        let params = params.ok_or((-32602, "Missing params".to_string()))?;
        let uri = extract_uri(&params);
        let empty = json!({"data": []});

        if !self.bridge.supports_semantic_tokens() {
            return Ok(empty);
        }

        let doc = match self.doc_manager.get(&uri) {
            Some(d) => d,
            None => return Ok(empty),
        };
        let doc_text = doc.text.clone();

        let tokens = match self.bridge.tokenize(&doc_text) {
            Ok(t) => t,
            Err(_) => return Ok(empty),
        };

        match self.bridge.semantic_tokens(&doc_text, &tokens) {
            None | Some(Err(_)) => Ok(empty),
            Some(Ok(sem_tokens)) => {
                let data = coding_adventures_ls00::capabilities::encode_semantic_tokens(&sem_tokens);
                Ok(json!({"data": data}))
            }
        }
    }

    /// Handle `textDocument/foldingRange` -- return collapsible regions.
    fn handle_folding_range(&mut self, params: Option<Value>) -> Result<Value, (i32, String)> {
        let params = params.ok_or((-32602, "Missing params".to_string()))?;
        let uri = extract_uri(&params);

        if !self.bridge.supports_folding_ranges() {
            return Ok(json!([]));
        }

        let (ast, _) = self.get_parse_result(&uri)?;
        let ast = match &ast {
            Some(a) => a,
            None => return Ok(json!([])),
        };

        match self.bridge.folding_ranges(ast.as_ref()) {
            None | Some(Err(_)) => Ok(json!([])),
            Some(Ok(ranges)) => {
                let result: Vec<Value> = ranges
                    .iter()
                    .map(|fr| {
                        let mut m = json!({
                            "startLine": fr.start_line,
                            "endLine": fr.end_line
                        });
                        if let Some(kind) = &fr.kind {
                            m["kind"] = json!(kind);
                        }
                        m
                    })
                    .collect();
                Ok(Value::Array(result))
            }
        }
    }

    /// Handle `textDocument/signatureHelp` -- return function signature hints.
    fn handle_signature_help(&mut self, params: Option<Value>) -> Result<Value, (i32, String)> {
        let params = params.ok_or((-32602, "Missing params".to_string()))?;
        let uri = extract_uri(&params);
        let pos = extract_position(&params);

        if !self.bridge.supports_signature_help() {
            return Ok(Value::Null);
        }

        let (ast, _) = self.get_parse_result(&uri)?;
        let ast = match &ast {
            Some(a) => a,
            None => return Ok(Value::Null),
        };

        match self.bridge.signature_help(ast.as_ref(), pos) {
            None | Some(Err(_)) | Some(Ok(None)) => Ok(Value::Null),
            Some(Ok(Some(sig_help))) => {
                let lsp_sigs: Vec<Value> = sig_help
                    .signatures
                    .iter()
                    .map(|sig| {
                        let lsp_params: Vec<Value> = sig
                            .parameters
                            .iter()
                            .map(|p| {
                                let mut pp = json!({"label": p.label});
                                if let Some(doc) = &p.documentation {
                                    if !doc.is_empty() {
                                        pp["documentation"] = json!(doc);
                                    }
                                }
                                pp
                            })
                            .collect();
                        let mut s = json!({
                            "label": sig.label,
                            "parameters": lsp_params
                        });
                        if let Some(doc) = &sig.documentation {
                            if !doc.is_empty() {
                                s["documentation"] = json!(doc);
                            }
                        }
                        s
                    })
                    .collect();

                Ok(json!({
                    "signatures": lsp_sigs,
                    "activeSignature": sig_help.active_signature,
                    "activeParameter": sig_help.active_parameter
                }))
            }
        }
    }

    /// Handle `textDocument/formatting` -- return text edits for formatting.
    fn handle_formatting(&mut self, params: Option<Value>) -> Result<Value, (i32, String)> {
        let params = params.ok_or((-32602, "Missing params".to_string()))?;
        let uri = extract_uri(&params);

        if !self.bridge.supports_format() {
            return Ok(json!([]));
        }

        let doc = match self.doc_manager.get(&uri) {
            Some(d) => d,
            None => return Ok(json!([])),
        };
        let doc_text = doc.text.clone();

        match self.bridge.format(&doc_text) {
            None => Ok(json!([])),
            Some(Err(e)) => Err((-32803, format!("Formatting failed: {}", e))),
            Some(Ok(edits)) => {
                let lsp_edits: Vec<Value> = edits
                    .iter()
                    .map(|edit| {
                        json!({
                            "range": range_to_json(&edit.range),
                            "newText": edit.new_text
                        })
                    })
                    .collect();
                Ok(Value::Array(lsp_edits))
            }
        }
    }

    // -----------------------------------------------------------------------
    // Internal helpers
    // -----------------------------------------------------------------------

    /// Get a fresh parse result for a document.
    ///
    /// Returns the AST and diagnostics. If the document is not open, returns
    /// an error. Uses the parse cache to avoid redundant parsing.
    fn get_parse_result(
        &mut self,
        uri: &str,
    ) -> Result<(Option<Box<dyn Any + Send + Sync>>, Vec<Diagnostic>), (i32, String)> {
        let doc = self.doc_manager.get(uri).ok_or_else(|| {
            (-32803i32, format!("Document not open: {}", uri))
        })?;

        let text = doc.text.clone();
        let version = doc.version;

        // Ensure the cache is populated.
        let cached = self
            .parse_cache
            .get_or_parse(uri, version, &text, self.bridge.as_ref());
        let cached_diags = cached.diagnostics.clone();

        // Re-parse for an owned AST (the cache stores its own copy).
        match self.bridge.parse(&text) {
            Ok((ast, _)) => Ok((Some(ast), cached_diags)),
            Err(_) => Ok((None, cached_diags)),
        }
    }

    /// Send a notification to the client via the JS callback.
    ///
    /// This is the WASM equivalent of writing to stdout. The notification is
    /// serialized as a JSON-RPC notification string and passed to the callback.
    ///
    /// If no callback is registered (e.g., in native tests), the notification
    /// is silently dropped.
    fn send_notification(&self, method: &str, params: Value) {
        let callback = match &self.send_callback {
            Some(cb) => cb,
            None => return, // no callback registered (native test mode)
        };

        let notif = json!({
            "jsonrpc": "2.0",
            "method": method,
            "params": params
        });

        let json_str = serde_json::to_string(&notif).unwrap_or_default();
        let this = JsValue::NULL;
        let _ = callback.call1(&this, &JsValue::from_str(&json_str));
    }

    /// Publish diagnostics to the client.
    ///
    /// Builds the `textDocument/publishDiagnostics` notification and sends it
    /// via the callback. This is called after every document open/change.
    fn publish_diagnostics(&self, uri: &str, version: i32, diagnostics: &[Diagnostic]) {
        let lsp_diags: Vec<Value> = diagnostics
            .iter()
            .map(|d| {
                let mut diag = json!({
                    "range": {
                        "start": {"line": d.range.start.line, "character": d.range.start.character},
                        "end": {"line": d.range.end.line, "character": d.range.end.character}
                    },
                    "severity": d.severity as i32,
                    "message": d.message
                });
                if let Some(code) = &d.code {
                    diag["code"] = json!(code);
                }
                diag
            })
            .collect();

        let mut params = json!({
            "uri": uri,
            "diagnostics": lsp_diags
        });

        if version > 0 {
            params["version"] = json!(version);
        }

        self.send_notification("textDocument/publishDiagnostics", params);
    }
}

// ---------------------------------------------------------------------------
// JSON helpers
// ---------------------------------------------------------------------------
//
// These are utility functions for converting between LSP types and JSON
// values. They mirror the helpers in the core crate's handlers module.

/// Extract the document URI from params that have a `textDocument` field.
fn extract_uri(params: &Value) -> String {
    params["textDocument"]["uri"]
        .as_str()
        .unwrap_or("")
        .to_string()
}

/// Extract a Position from params that have a `position` field.
fn extract_position(params: &Value) -> Position {
    let pos = &params["position"];
    Position {
        line: pos["line"].as_i64().unwrap_or(0) as i32,
        character: pos["character"].as_i64().unwrap_or(0) as i32,
    }
}

/// Convert a Position to a JSON value.
fn position_to_json(p: &Position) -> Value {
    json!({"line": p.line, "character": p.character})
}

/// Convert a Range to a JSON value.
fn range_to_json(r: &Range) -> Value {
    json!({
        "start": position_to_json(&r.start),
        "end": position_to_json(&r.end)
    })
}

/// Convert a Location to a JSON value.
fn location_to_json(l: &Location) -> Value {
    json!({
        "uri": l.uri,
        "range": range_to_json(&l.range)
    })
}

/// Recursively convert DocumentSymbol trees to JSON values.
fn convert_document_symbols(symbols: &[DocumentSymbol]) -> Vec<Value> {
    symbols
        .iter()
        .map(|sym| {
            let mut m = json!({
                "name": sym.name,
                "kind": sym.kind as i32,
                "range": range_to_json(&sym.range),
                "selectionRange": range_to_json(&sym.selection_range)
            });
            if !sym.children.is_empty() {
                m["children"] = Value::Array(convert_document_symbols(&sym.children));
            }
            m
        })
        .collect()
}

// ---------------------------------------------------------------------------
// Native Rust tests
// ---------------------------------------------------------------------------
//
// These tests verify the WASM wrapper's internal logic (JSON parsing,
// diagnostic extraction, message dispatch) without needing WASM tooling.
// They are excluded when compiling for WASM.
//
// Since WasmLanguageBridge requires JS functions (which don't exist in
// native Rust), we test using a mock bridge that implements LanguageBridge
// directly.

#[cfg(not(target_arch = "wasm32"))]
#[cfg(test)]
mod tests {
    use super::*;

    // -----------------------------------------------------------------------
    // Mock bridge for testing
    // -----------------------------------------------------------------------

    /// A minimal bridge that tokenizes by splitting on whitespace and
    /// produces a trivial AST.
    struct MockBridge;

    impl LanguageBridge for MockBridge {
        fn tokenize(&self, source: &str) -> Result<Vec<Token>, String> {
            let mut tokens = Vec::new();
            for (line_idx, line) in source.lines().enumerate() {
                let mut col = 1;
                for word in line.split_whitespace() {
                    tokens.push(Token {
                        token_type: "IDENTIFIER".to_string(),
                        value: word.to_string(),
                        line: (line_idx + 1) as i32,
                        column: col,
                    });
                    col += word.len() as i32 + 1;
                }
            }
            Ok(tokens)
        }

        fn parse(
            &self,
            source: &str,
        ) -> Result<(Box<dyn Any + Send + Sync>, Vec<Diagnostic>), String> {
            // Store the source as the "AST" for simplicity.
            let ast = Box::new(JsAst {
                json: source.to_string(),
            }) as Box<dyn Any + Send + Sync>;
            Ok((ast, Vec::new()))
        }
    }

    /// Create a test server with the MockBridge.
    ///
    /// Since we can't create JS functions in native Rust, we set
    /// `send_callback` to `None`. The server will silently skip sending
    /// notifications (like `publishDiagnostics`) in this mode, which
    /// is fine for testing the dispatch logic.
    fn make_test_server() -> WasmLspServer {
        WasmLspServer {
            bridge: Box::new(MockBridge),
            doc_manager: DocumentManager::new(),
            parse_cache: ParseCache::new(),
            send_callback: None,
            initialized: false,
            shutdown: false,
        }
    }

    // -----------------------------------------------------------------------
    // Diagnostic parsing tests
    // -----------------------------------------------------------------------

    #[test]
    fn test_parse_diagnostics_empty() {
        let diags = parse_diagnostics_from_json(&Value::Null);
        assert!(diags.is_empty(), "Null should produce empty diagnostics");
    }

    #[test]
    fn test_parse_diagnostics_empty_array() {
        let diags = parse_diagnostics_from_json(&json!([]));
        assert!(diags.is_empty(), "Empty array should produce empty diagnostics");
    }

    #[test]
    fn test_parse_diagnostics_single() {
        let input = json!([{
            "range": {
                "start": {"line": 0, "character": 0},
                "end": {"line": 0, "character": 5}
            },
            "severity": 1,
            "message": "unexpected token"
        }]);

        let diags = parse_diagnostics_from_json(&input);
        assert_eq!(diags.len(), 1);
        assert_eq!(diags[0].message, "unexpected token");
        assert_eq!(diags[0].severity, DiagnosticSeverity::Error);
        assert_eq!(diags[0].range.start.line, 0);
        assert_eq!(diags[0].range.start.character, 0);
        assert_eq!(diags[0].range.end.line, 0);
        assert_eq!(diags[0].range.end.character, 5);
    }

    #[test]
    fn test_parse_diagnostics_with_code() {
        let input = json!([{
            "range": {
                "start": {"line": 1, "character": 3},
                "end": {"line": 1, "character": 10}
            },
            "severity": 2,
            "message": "unused variable",
            "code": "W001"
        }]);

        let diags = parse_diagnostics_from_json(&input);
        assert_eq!(diags.len(), 1);
        assert_eq!(diags[0].severity, DiagnosticSeverity::Warning);
        assert_eq!(diags[0].code, Some("W001".to_string()));
    }

    #[test]
    fn test_parse_diagnostics_all_severities() {
        let input = json!([
            {"range": {"start": {"line": 0, "character": 0}, "end": {"line": 0, "character": 1}}, "severity": 1, "message": "error"},
            {"range": {"start": {"line": 0, "character": 0}, "end": {"line": 0, "character": 1}}, "severity": 2, "message": "warning"},
            {"range": {"start": {"line": 0, "character": 0}, "end": {"line": 0, "character": 1}}, "severity": 3, "message": "info"},
            {"range": {"start": {"line": 0, "character": 0}, "end": {"line": 0, "character": 1}}, "severity": 4, "message": "hint"}
        ]);

        let diags = parse_diagnostics_from_json(&input);
        assert_eq!(diags.len(), 4);
        assert_eq!(diags[0].severity, DiagnosticSeverity::Error);
        assert_eq!(diags[1].severity, DiagnosticSeverity::Warning);
        assert_eq!(diags[2].severity, DiagnosticSeverity::Information);
        assert_eq!(diags[3].severity, DiagnosticSeverity::Hint);
    }

    #[test]
    fn test_parse_diagnostics_multiple() {
        let input = json!([
            {
                "range": {"start": {"line": 0, "character": 0}, "end": {"line": 0, "character": 3}},
                "severity": 1,
                "message": "first error"
            },
            {
                "range": {"start": {"line": 2, "character": 5}, "end": {"line": 2, "character": 10}},
                "severity": 2,
                "message": "second warning"
            }
        ]);

        let diags = parse_diagnostics_from_json(&input);
        assert_eq!(diags.len(), 2);
        assert_eq!(diags[0].message, "first error");
        assert_eq!(diags[1].message, "second warning");
        assert_eq!(diags[1].range.start.line, 2);
        assert_eq!(diags[1].range.start.character, 5);
    }

    // -----------------------------------------------------------------------
    // JSON helper tests
    // -----------------------------------------------------------------------

    #[test]
    fn test_extract_uri() {
        let params = json!({
            "textDocument": {"uri": "file:///test.rs"}
        });
        assert_eq!(extract_uri(&params), "file:///test.rs");
    }

    #[test]
    fn test_extract_uri_missing() {
        let params = json!({});
        assert_eq!(extract_uri(&params), "");
    }

    #[test]
    fn test_extract_position() {
        let params = json!({
            "position": {"line": 5, "character": 10}
        });
        let pos = extract_position(&params);
        assert_eq!(pos.line, 5);
        assert_eq!(pos.character, 10);
    }

    #[test]
    fn test_extract_position_defaults() {
        let params = json!({});
        let pos = extract_position(&params);
        assert_eq!(pos.line, 0);
        assert_eq!(pos.character, 0);
    }

    #[test]
    fn test_position_to_json() {
        let pos = Position {
            line: 3,
            character: 7,
        };
        let j = position_to_json(&pos);
        assert_eq!(j["line"], 3);
        assert_eq!(j["character"], 7);
    }

    #[test]
    fn test_range_to_json() {
        let range = Range {
            start: Position {
                line: 1,
                character: 2,
            },
            end: Position {
                line: 3,
                character: 4,
            },
        };
        let j = range_to_json(&range);
        assert_eq!(j["start"]["line"], 1);
        assert_eq!(j["start"]["character"], 2);
        assert_eq!(j["end"]["line"], 3);
        assert_eq!(j["end"]["character"], 4);
    }

    #[test]
    fn test_location_to_json() {
        let loc = Location {
            uri: "file:///test.rs".to_string(),
            range: Range {
                start: Position {
                    line: 0,
                    character: 0,
                },
                end: Position {
                    line: 0,
                    character: 5,
                },
            },
        };
        let j = location_to_json(&loc);
        assert_eq!(j["uri"], "file:///test.rs");
        assert_eq!(j["range"]["start"]["line"], 0);
    }

    #[test]
    fn test_convert_document_symbols_empty() {
        let symbols: Vec<DocumentSymbol> = Vec::new();
        let result = convert_document_symbols(&symbols);
        assert!(result.is_empty());
    }

    #[test]
    fn test_convert_document_symbols_nested() {
        let symbols = vec![DocumentSymbol {
            name: "main".to_string(),
            kind: SymbolKind::Function,
            range: Range {
                start: Position { line: 0, character: 0 },
                end: Position { line: 5, character: 1 },
            },
            selection_range: Range {
                start: Position { line: 0, character: 3 },
                end: Position { line: 0, character: 7 },
            },
            children: vec![DocumentSymbol {
                name: "x".to_string(),
                kind: SymbolKind::Variable,
                range: Range {
                    start: Position { line: 1, character: 4 },
                    end: Position { line: 1, character: 10 },
                },
                selection_range: Range {
                    start: Position { line: 1, character: 8 },
                    end: Position { line: 1, character: 9 },
                },
                children: Vec::new(),
            }],
        }];

        let result = convert_document_symbols(&symbols);
        assert_eq!(result.len(), 1);
        assert_eq!(result[0]["name"], "main");
        assert_eq!(result[0]["kind"], SymbolKind::Function as i32);
        let children = result[0]["children"].as_array().unwrap();
        assert_eq!(children.len(), 1);
        assert_eq!(children[0]["name"], "x");
    }

    // -----------------------------------------------------------------------
    // Dispatch tests (using the mock bridge directly)
    // -----------------------------------------------------------------------
    //
    // We can't test handleMessage directly because it requires a real
    // js_sys::Function. Instead, we test the internal dispatch methods.

    #[test]
    fn test_initialize() {
        let mut server = make_test_server();
        assert!(!server.initialized);

        let result = server.handle_initialize(None);
        assert!(result.is_ok());
        assert!(server.initialized);

        let caps = &result.unwrap();
        assert!(caps["capabilities"].is_object());
        assert_eq!(
            caps["serverInfo"]["name"],
            "ls00-wasm-lsp-server"
        );
    }

    #[test]
    fn test_shutdown() {
        let mut server = make_test_server();
        assert!(!server.shutdown);

        let result = server.handle_shutdown();
        assert!(result.is_ok());
        assert!(server.shutdown);
        assert_eq!(result.unwrap(), Value::Null);
    }

    #[test]
    fn test_dispatch_request_unknown_method() {
        let mut server = make_test_server();
        let result = server.dispatch_request("nonexistent/method", None);
        assert!(result.is_err());
        let (code, msg) = result.unwrap_err();
        assert_eq!(code, -32601);
        assert!(msg.contains("Method not found"));
    }

    #[test]
    fn test_dispatch_notification_unknown_method() {
        let mut server = make_test_server();
        // Unknown notifications should be silently dropped (no panic).
        server.dispatch_notification("nonexistent/notification", None);
    }

    #[test]
    fn test_hover_unsupported() {
        let mut server = make_test_server();
        let params = json!({
            "textDocument": {"uri": "file:///test.rs"},
            "position": {"line": 0, "character": 0}
        });

        let result = server.handle_hover(Some(params));
        assert!(result.is_ok());
        assert_eq!(result.unwrap(), Value::Null);
    }

    #[test]
    fn test_hover_missing_params() {
        let mut server = make_test_server();
        let result = server.handle_hover(None);
        assert!(result.is_err());
        let (code, _) = result.unwrap_err();
        assert_eq!(code, -32602);
    }

    #[test]
    fn test_definition_unsupported() {
        let mut server = make_test_server();
        let params = json!({
            "textDocument": {"uri": "file:///test.rs"},
            "position": {"line": 0, "character": 0}
        });
        let result = server.handle_definition(Some(params));
        assert!(result.is_ok());
        assert_eq!(result.unwrap(), Value::Null);
    }

    #[test]
    fn test_references_unsupported() {
        let mut server = make_test_server();
        let params = json!({
            "textDocument": {"uri": "file:///test.rs"},
            "position": {"line": 0, "character": 0}
        });
        let result = server.handle_references(Some(params));
        assert!(result.is_ok());
        assert_eq!(result.unwrap(), json!([]));
    }

    #[test]
    fn test_completion_unsupported() {
        let mut server = make_test_server();
        let params = json!({
            "textDocument": {"uri": "file:///test.rs"},
            "position": {"line": 0, "character": 0}
        });
        let result = server.handle_completion(Some(params));
        assert!(result.is_ok());
        assert_eq!(result.unwrap(), json!({"isIncomplete": false, "items": []}));
    }

    #[test]
    fn test_rename_unsupported() {
        let mut server = make_test_server();
        let params = json!({
            "textDocument": {"uri": "file:///test.rs"},
            "position": {"line": 0, "character": 0},
            "newName": "foo"
        });
        let result = server.handle_rename(Some(params));
        assert!(result.is_err());
    }

    #[test]
    fn test_rename_missing_new_name() {
        let mut server = make_test_server();
        let params = json!({
            "textDocument": {"uri": "file:///test.rs"},
            "position": {"line": 0, "character": 0}
        });
        let result = server.handle_rename(Some(params));
        assert!(result.is_err());
        let (code, msg) = result.unwrap_err();
        assert_eq!(code, -32602);
        assert!(msg.contains("newName"));
    }

    #[test]
    fn test_document_symbol_unsupported() {
        let mut server = make_test_server();
        let params = json!({"textDocument": {"uri": "file:///test.rs"}});
        let result = server.handle_document_symbol(Some(params));
        assert!(result.is_ok());
        assert_eq!(result.unwrap(), json!([]));
    }

    #[test]
    fn test_semantic_tokens_unsupported() {
        let mut server = make_test_server();
        let params = json!({"textDocument": {"uri": "file:///test.rs"}});
        let result = server.handle_semantic_tokens_full(Some(params));
        assert!(result.is_ok());
        assert_eq!(result.unwrap(), json!({"data": []}));
    }

    #[test]
    fn test_folding_range_unsupported() {
        let mut server = make_test_server();
        let params = json!({"textDocument": {"uri": "file:///test.rs"}});
        let result = server.handle_folding_range(Some(params));
        assert!(result.is_ok());
        assert_eq!(result.unwrap(), json!([]));
    }

    #[test]
    fn test_signature_help_unsupported() {
        let mut server = make_test_server();
        let params = json!({
            "textDocument": {"uri": "file:///test.rs"},
            "position": {"line": 0, "character": 0}
        });
        let result = server.handle_signature_help(Some(params));
        assert!(result.is_ok());
        assert_eq!(result.unwrap(), Value::Null);
    }

    #[test]
    fn test_formatting_unsupported() {
        let mut server = make_test_server();
        let params = json!({"textDocument": {"uri": "file:///test.rs"}});
        let result = server.handle_formatting(Some(params));
        assert!(result.is_ok());
        assert_eq!(result.unwrap(), json!([]));
    }

    // -----------------------------------------------------------------------
    // Document lifecycle tests
    // -----------------------------------------------------------------------

    #[test]
    fn test_did_open_registers_document() {
        let mut server = make_test_server();
        server.handle_did_open(Some(json!({
            "textDocument": {
                "uri": "file:///test.rs",
                "text": "let x = 1;",
                "version": 1
            }
        })));

        // Verify the document is tracked.
        let doc = server.doc_manager.get("file:///test.rs");
        assert!(doc.is_some());
        assert_eq!(doc.unwrap().text, "let x = 1;");
        assert_eq!(doc.unwrap().version, 1);
    }

    #[test]
    fn test_did_open_empty_uri_ignored() {
        let mut server = make_test_server();
        server.handle_did_open(Some(json!({
            "textDocument": {
                "uri": "",
                "text": "let x = 1;",
                "version": 1
            }
        })));

        // No document should be registered.
        assert!(server.doc_manager.get("").is_none());
    }

    #[test]
    fn test_did_open_no_params() {
        let mut server = make_test_server();
        // Should not panic.
        server.handle_did_open(None);
    }

    #[test]
    fn test_did_change_full_replacement() {
        let mut server = make_test_server();

        // Open a document first.
        server.handle_did_open(Some(json!({
            "textDocument": {"uri": "file:///test.rs", "text": "hello", "version": 1}
        })));

        // Full replacement (no range).
        server.handle_did_change(Some(json!({
            "textDocument": {"uri": "file:///test.rs", "version": 2},
            "contentChanges": [{"text": "world"}]
        })));

        let doc = server.doc_manager.get("file:///test.rs").unwrap();
        assert_eq!(doc.text, "world");
        assert_eq!(doc.version, 2);
    }

    #[test]
    fn test_did_close_removes_document() {
        let mut server = make_test_server();

        server.handle_did_open(Some(json!({
            "textDocument": {"uri": "file:///test.rs", "text": "hello", "version": 1}
        })));

        assert!(server.doc_manager.get("file:///test.rs").is_some());

        server.handle_did_close(Some(json!({
            "textDocument": {"uri": "file:///test.rs"}
        })));

        assert!(server.doc_manager.get("file:///test.rs").is_none());
    }

    #[test]
    fn test_did_save_no_params() {
        let mut server = make_test_server();
        // Should not panic.
        server.handle_did_save(None);
    }

    #[test]
    fn test_did_save_with_text() {
        let mut server = make_test_server();

        server.handle_did_open(Some(json!({
            "textDocument": {"uri": "file:///test.rs", "text": "old content", "version": 1}
        })));

        server.handle_did_save(Some(json!({
            "textDocument": {"uri": "file:///test.rs"},
            "text": "saved content"
        })));

        let doc = server.doc_manager.get("file:///test.rs").unwrap();
        assert_eq!(doc.text, "saved content");
    }

    // -----------------------------------------------------------------------
    // Integration tests -- request dispatch after document open
    // -----------------------------------------------------------------------

    #[test]
    fn test_hover_on_open_document() {
        let mut server = make_test_server();

        server.handle_did_open(Some(json!({
            "textDocument": {"uri": "file:///test.rs", "text": "hello world", "version": 1}
        })));

        // MockBridge does not support hover, so this should return null.
        let result = server.handle_hover(Some(json!({
            "textDocument": {"uri": "file:///test.rs"},
            "position": {"line": 0, "character": 0}
        })));

        assert!(result.is_ok());
        assert_eq!(result.unwrap(), Value::Null);
    }

    #[test]
    fn test_hover_on_unopened_document() {
        let mut server = make_test_server();

        // The document is not open, so this should produce an error.
        // But since hover is not supported by MockBridge, it returns Null first.
        let result = server.handle_hover(Some(json!({
            "textDocument": {"uri": "file:///nonexistent.rs"},
            "position": {"line": 0, "character": 0}
        })));

        // MockBridge doesn't support hover, so it returns Null before checking the doc.
        assert!(result.is_ok());
        assert_eq!(result.unwrap(), Value::Null);
    }

    // -----------------------------------------------------------------------
    // Initialize and state tests
    // -----------------------------------------------------------------------

    #[test]
    fn test_initialize_sets_state() {
        let mut server = make_test_server();
        assert!(!server.is_initialized());
        assert!(!server.is_shutdown());

        let _ = server.handle_initialize(None);
        assert!(server.is_initialized());
        assert!(!server.is_shutdown());

        let _ = server.handle_shutdown();
        assert!(server.is_initialized());
        assert!(server.is_shutdown());
    }

    #[test]
    fn test_initialize_returns_capabilities() {
        let mut server = make_test_server();
        let result = server.handle_initialize(None).unwrap();

        // MockBridge supports nothing optional, so capabilities should
        // just have textDocumentSync.
        let caps = &result["capabilities"];
        assert!(caps.is_object());
        // textDocumentSync=2 (incremental) is always set.
        assert_eq!(caps["textDocumentSync"], 2);
    }
}
