//! `LspServer` -- the main coordinator.
//!
//! `LspServer` wires together:
//! - The `LanguageBridge` (language-specific logic)
//! - The `DocumentManager` (tracks open file contents)
//! - The `ParseCache` (avoids redundant parses)
//! - The JSON-RPC `MessageReader` and `MessageWriter` (protocol layer)
//!
//! Instead of using the JSON-RPC `Server`'s dispatch loop (which requires
//! closures that are `Send + 'static`), we read and dispatch messages directly.
//! This is simpler and avoids lifetime gymnastics with self-referential closures.
//!
//! # Server Lifecycle
//!
//! ```text
//! Client (editor)              Server (us)
//!   |                               |
//!   |--initialize-------------->    |  store clientInfo, return capabilities
//!   | <-----------------result-     |
//!   |                               |
//!   |--initialized (notif)----->    |  no-op (handshake complete)
//!   |                               |
//!   |--textDocument/didOpen---->    |  open doc, parse, push diagnostics
//!   |--textDocument/hover------>    |  get parse result, call bridge.Hover
//!   | <-----------------result-     |
//!   |                               |
//!   |--shutdown---------------->    |  set shutdown flag, return null
//!   |--exit (notif)------------>    |  exit process
//! ```

use crate::document_manager::DocumentManager;
use crate::language_bridge::LanguageBridge;
use crate::lsp_errors::REQUEST_FAILED;
use crate::parse_cache::ParseCache;
use crate::types::Diagnostic;
use coding_adventures_json_rpc::message::{Message, Response};
use coding_adventures_json_rpc::{MessageReader, MessageWriter, ResponseError};
use serde_json::{json, Value};
use std::any::Any;
use std::io::{BufRead, Write};

/// The main LSP server.
///
/// Create it with `LspServer::new()`, then call `serve()` to start serving.
/// It is designed to be used once per process -- start it, it blocks, it exits.
pub struct LspServer<R: BufRead, W: Write> {
    pub(crate) bridge: Box<dyn LanguageBridge>,
    pub(crate) doc_manager: DocumentManager,
    pub(crate) parse_cache: ParseCache,
    reader: MessageReader<R>,
    /// The writer for sending messages back to the client.
    pub writer: MessageWriter<W>,

    /// Whether the editor has sent "shutdown".
    pub(crate) shutdown: bool,

    /// Whether the initialize handshake is complete.
    pub(crate) initialized: bool,
}

impl<R: BufRead, W: Write> LspServer<R, W> {
    /// Create an `LspServer` wired to read from `reader` and write to `writer`.
    pub fn new(bridge: Box<dyn LanguageBridge>, reader: R, writer: W) -> Self {
        Self {
            bridge,
            doc_manager: DocumentManager::new(),
            parse_cache: ParseCache::new(),
            reader: MessageReader::new(reader),
            writer: MessageWriter::new(writer),
            shutdown: false,
            initialized: false,
        }
    }

    /// Start the blocking read-dispatch-write loop.
    ///
    /// This call blocks until the editor closes the connection (EOF on stdin).
    /// All LSP messages are handled synchronously in this loop.
    pub fn serve(&mut self) {
        loop {
            match self.reader.read_message() {
                None => {
                    // EOF -- clean shutdown.
                    break;
                }

                Some(Ok(Message::Request(req))) => {
                    let id = req.id.clone();
                    let method = req.method.as_str();
                    let params = req.params.clone();

                    let result = match method {
                        "initialize" => self.handle_initialize(id.clone(), params),
                        "shutdown" => self.handle_shutdown(id.clone(), params),
                        "textDocument/hover" => self.handle_hover(id.clone(), params),
                        "textDocument/definition" => self.handle_definition(id.clone(), params),
                        "textDocument/references" => self.handle_references(id.clone(), params),
                        "textDocument/completion" => self.handle_completion(id.clone(), params),
                        "textDocument/rename" => self.handle_rename(id.clone(), params),
                        "textDocument/documentSymbol" => {
                            self.handle_document_symbol(id.clone(), params)
                        }
                        "textDocument/semanticTokens/full" => {
                            self.handle_semantic_tokens_full(id.clone(), params)
                        }
                        "textDocument/foldingRange" => {
                            self.handle_folding_range(id.clone(), params)
                        }
                        "textDocument/signatureHelp" => {
                            self.handle_signature_help(id.clone(), params)
                        }
                        "textDocument/formatting" => {
                            self.handle_formatting(id.clone(), params)
                        }
                        _ => Err(ResponseError::method_not_found(method)),
                    };

                    let response = match result {
                        Ok(value) => Response {
                            id,
                            result: Some(value),
                            error: None,
                        },
                        Err(e) => Response {
                            id,
                            result: None,
                            error: Some(e),
                        },
                    };

                    let _ = self.writer.write_message(&Message::Response(response));
                }

                Some(Ok(Message::Notification(notif))) => {
                    let method = notif.method.as_str();
                    let params = notif.params.clone();

                    match method {
                        "initialized" => self.handle_initialized(params),
                        "exit" => self.handle_exit(params),
                        "textDocument/didOpen" => self.handle_did_open(params),
                        "textDocument/didChange" => self.handle_did_change(params),
                        "textDocument/didClose" => self.handle_did_close(params),
                        "textDocument/didSave" => self.handle_did_save(params),
                        _ => {} // unknown notifications are silently dropped
                    }
                }

                Some(Ok(Message::Response(_))) => {
                    // Responses are for client-side use. Ignored.
                }

                Some(Err(e)) => {
                    let response = Response {
                        id: Value::Null,
                        result: None,
                        error: Some(e),
                    };
                    let _ = self.writer.write_message(&Message::Response(response));
                }
            }
        }
    }

    /// Send a server-initiated notification to the editor.
    fn send_notification(&mut self, method: &str, params: Value) {
        let notif = Message::Notification(coding_adventures_json_rpc::Notification {
            method: method.to_string(),
            params: Some(params),
        });
        let _ = self.writer.write_message(&notif);
    }

    /// Publish diagnostics to the editor.
    pub(crate) fn publish_diagnostics(
        &mut self,
        uri: &str,
        version: i32,
        diagnostics: &[Diagnostic],
    ) {
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

    /// Get the current parse result for a document, returning an owned AST.
    ///
    /// We call `bridge.parse()` directly to get an owned AST box, but the
    /// parse cache ensures we don't do redundant work for diagnostics.
    pub(crate) fn get_parse_result_cloned(
        &mut self,
        uri: &str,
    ) -> Result<(Option<Box<dyn Any + Send + Sync>>, Vec<Diagnostic>), ResponseError> {
        let doc = self.doc_manager.get(uri).ok_or_else(|| ResponseError {
            code: REQUEST_FAILED,
            message: format!("document not open: {}", uri),
            data: None,
        })?;

        let text = doc.text.clone();
        let version = doc.version;

        // Ensure diagnostics are cached.
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
}
