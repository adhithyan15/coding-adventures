//! All LSP handler implementations.
//!
//! Each function here processes one LSP request or notification method. The
//! server's `register_handlers` method wires each method name to its handler.
//!
//! # Handler Patterns
//!
//! - **Request handlers** receive `(id, params)` and return `Result<Value, ResponseError>`.
//! - **Notification handlers** receive `params` and return nothing.
//!
//! # Lifecycle Handlers
//!
//! - `initialize` -- first message, returns capabilities
//! - `initialized` -- handshake complete, no-op
//! - `shutdown` -- set shutdown flag, return null
//! - `exit` -- terminate the process
//!
//! # Document Sync Handlers
//!
//! - `didOpen` -- open doc, parse, push diagnostics
//! - `didChange` -- apply changes, re-parse, push diagnostics
//! - `didClose` -- remove doc, evict cache
//! - `didSave` -- optionally re-parse with saved content
//!
//! # Feature Handlers
//!
//! Each feature handler follows the same pattern:
//! 1. Extract URI and position from params.
//! 2. Check if the bridge supports the feature.
//! 3. Get the parse result from cache.
//! 4. Delegate to the bridge.
//! 5. Convert the result to LSP JSON format.

use crate::capabilities::build_capabilities;
use crate::lsp_errors::REQUEST_FAILED;
use crate::server::LspServer;
use crate::types::*;
use coding_adventures_json_rpc::ResponseError;
use serde_json::{json, Value};
use std::io::{BufRead, Write};

// Note: The handler methods live on LspServer<R, W> so they can access
// the bridge, document manager, parse cache, and writer. They are called
// from the `serve()` dispatch loop in server.rs.

// ---------------------------------------------------------------------------
// JSON param extraction helpers
// ---------------------------------------------------------------------------

/// Extract a `Position` from a JSON params object.
/// The LSP sends positions as `{"position": {"line": N, "character": N}}`.
fn parse_position(params: &Value) -> Position {
    let pos = &params["position"];
    Position {
        line: pos["line"].as_i64().unwrap_or(0) as i32,
        character: pos["character"].as_i64().unwrap_or(0) as i32,
    }
}

/// Extract the document URI from params that have a `textDocument` field.
fn parse_uri(params: &Value) -> String {
    params["textDocument"]["uri"]
        .as_str()
        .unwrap_or("")
        .to_string()
}

/// Convert a `Position` to a JSON value.
fn position_to_json(p: &Position) -> Value {
    json!({"line": p.line, "character": p.character})
}

/// Convert a `Range` to a JSON value.
fn range_to_json(r: &Range) -> Value {
    json!({
        "start": position_to_json(&r.start),
        "end": position_to_json(&r.end)
    })
}

/// Convert a `Location` to a JSON value.
fn location_to_json(l: &Location) -> Value {
    json!({
        "uri": l.uri,
        "range": range_to_json(&l.range)
    })
}

/// Parse an LSP range from a JSON value.
fn parse_lsp_range(raw: &Value) -> Range {
    Range {
        start: Position {
            line: raw["start"]["line"].as_i64().unwrap_or(0) as i32,
            character: raw["start"]["character"].as_i64().unwrap_or(0) as i32,
        },
        end: Position {
            line: raw["end"]["line"].as_i64().unwrap_or(0) as i32,
            character: raw["end"]["character"].as_i64().unwrap_or(0) as i32,
        },
    }
}

// ---------------------------------------------------------------------------
// Lifecycle handlers
// ---------------------------------------------------------------------------

impl<R: BufRead, W: Write> LspServer<R, W> {
    /// Process the LSP `initialize` request.
    ///
    /// This is the server's first message. We store the client info (for
    /// logging) and return our capabilities built from the bridge.
    pub(crate) fn handle_initialize(
        &mut self,
        _id: Value,
        _params: Option<Value>,
    ) -> Result<Value, ResponseError> {
        self.initialized = true;

        let caps = build_capabilities(self.bridge.as_ref());

        Ok(json!({
            "capabilities": caps,
            "serverInfo": {
                "name": "ls00-generic-lsp-server",
                "version": "0.1.0"
            }
        }))
    }

    /// Process the `initialized` notification.
    ///
    /// The editor's acknowledgment that it received our capabilities.
    /// No-op -- normal operation begins now.
    pub(crate) fn handle_initialized(&mut self, _params: Option<Value>) {
        // No-op: the handshake is complete.
    }

    /// Process the LSP `shutdown` request.
    ///
    /// After receiving shutdown, the server should stop processing new requests
    /// and return null.
    pub(crate) fn handle_shutdown(
        &mut self,
        _id: Value,
        _params: Option<Value>,
    ) -> Result<Value, ResponseError> {
        self.shutdown = true;
        Ok(Value::Null)
    }

    /// Process the `exit` notification.
    ///
    /// Exit code semantics (from the LSP spec):
    /// - 0: shutdown was received before exit -> clean shutdown
    /// - 1: shutdown was NOT received -> abnormal termination
    pub(crate) fn handle_exit(&mut self, _params: Option<Value>) {
        if self.shutdown {
            std::process::exit(0);
        } else {
            std::process::exit(1);
        }
    }

    // -----------------------------------------------------------------------
    // Text document synchronization
    // -----------------------------------------------------------------------

    /// Process `textDocument/didOpen`.
    ///
    /// Records the opened file and immediately parses + pushes diagnostics.
    pub(crate) fn handle_did_open(&mut self, params: Option<Value>) {
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

    /// Process `textDocument/didChange`.
    ///
    /// Applies incremental changes, re-parses, and pushes diagnostics.
    pub(crate) fn handle_did_change(&mut self, params: Option<Value>) {
        let params = match params {
            Some(p) => p,
            None => return,
        };

        let uri = parse_uri(&params);
        if uri.is_empty() {
            return;
        }

        let version = params["textDocument"]["version"]
            .as_i64()
            .unwrap_or(0) as i32;

        // Parse the content changes array.
        let changes_raw = match params["contentChanges"].as_array() {
            Some(arr) => arr,
            None => return,
        };

        let changes: Vec<crate::document_manager::TextChange> = changes_raw
            .iter()
            .map(|change| {
                let new_text = change["text"].as_str().unwrap_or("").to_string();
                let range = if change.get("range").is_some() && !change["range"].is_null() {
                    Some(parse_lsp_range(&change["range"]))
                } else {
                    None
                };
                crate::document_manager::TextChange { range, new_text }
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

    /// Process `textDocument/didClose`.
    ///
    /// Removes the document and clears its diagnostics.
    pub(crate) fn handle_did_close(&mut self, params: Option<Value>) {
        let params = match params {
            Some(p) => p,
            None => return,
        };

        let uri = parse_uri(&params);
        if uri.is_empty() {
            return;
        }

        self.doc_manager.close(&uri);
        self.parse_cache.evict(&uri);

        // Clear diagnostics for the closed file.
        self.publish_diagnostics(&uri, 0, &[]);
    }

    /// Process `textDocument/didSave`.
    ///
    /// If the client sends full text in didSave, apply it and re-parse.
    pub(crate) fn handle_did_save(&mut self, params: Option<Value>) {
        let params = match params {
            Some(p) => p,
            None => return,
        };

        let uri = parse_uri(&params);
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

    /// Process `textDocument/hover`.
    pub(crate) fn handle_hover(
        &mut self,
        _id: Value,
        params: Option<Value>,
    ) -> Result<Value, ResponseError> {
        let params = params.ok_or_else(|| ResponseError::invalid_params(None))?;
        let uri = parse_uri(&params);
        let pos = parse_position(&params);

        if !self.bridge.supports_hover() {
            return Ok(Value::Null);
        }

        let (parse_result_ast, _) = self.get_parse_result_cloned(&uri)?;

        let ast = match &parse_result_ast {
            Some(a) => a,
            None => return Ok(Value::Null),
        };

        let hover_result = match self.bridge.hover(ast.as_ref(), pos) {
            None | Some(Err(_)) => return Ok(Value::Null),
            Some(Ok(None)) => return Ok(Value::Null),
            Some(Ok(Some(r))) => r,
        };

        let mut result = json!({
            "contents": {
                "kind": "markdown",
                "value": hover_result.contents
            }
        });

        if let Some(range) = &hover_result.range {
            result["range"] = range_to_json(range);
        }

        Ok(result)
    }

    /// Process `textDocument/definition`.
    pub(crate) fn handle_definition(
        &mut self,
        _id: Value,
        params: Option<Value>,
    ) -> Result<Value, ResponseError> {
        let params = params.ok_or_else(|| ResponseError::invalid_params(None))?;
        let uri = parse_uri(&params);
        let pos = parse_position(&params);

        if !self.bridge.supports_definition() {
            return Ok(Value::Null);
        }

        let (parse_result_ast, _) = self.get_parse_result_cloned(&uri)?;
        let ast = match &parse_result_ast {
            Some(a) => a,
            None => return Ok(Value::Null),
        };

        match self.bridge.definition(ast.as_ref(), pos, &uri) {
            None | Some(Err(_)) | Some(Ok(None)) => Ok(Value::Null),
            Some(Ok(Some(loc))) => Ok(location_to_json(&loc)),
        }
    }

    /// Process `textDocument/references`.
    pub(crate) fn handle_references(
        &mut self,
        _id: Value,
        params: Option<Value>,
    ) -> Result<Value, ResponseError> {
        let params = params.ok_or_else(|| ResponseError::invalid_params(None))?;
        let uri = parse_uri(&params);
        let pos = parse_position(&params);

        let include_decl = params["context"]["includeDeclaration"]
            .as_bool()
            .unwrap_or(false);

        if !self.bridge.supports_references() {
            return Ok(json!([]));
        }

        let (parse_result_ast, _) = self.get_parse_result_cloned(&uri)?;
        let ast = match &parse_result_ast {
            Some(a) => a,
            None => return Ok(json!([])),
        };

        match self
            .bridge
            .references(ast.as_ref(), pos, &uri, include_decl)
        {
            None | Some(Err(_)) => Ok(json!([])),
            Some(Ok(locations)) => {
                let result: Vec<Value> = locations.iter().map(location_to_json).collect();
                Ok(Value::Array(result))
            }
        }
    }

    /// Process `textDocument/completion`.
    pub(crate) fn handle_completion(
        &mut self,
        _id: Value,
        params: Option<Value>,
    ) -> Result<Value, ResponseError> {
        let params = params.ok_or_else(|| ResponseError::invalid_params(None))?;
        let uri = parse_uri(&params);
        let pos = parse_position(&params);

        let empty_result = json!({"isIncomplete": false, "items": []});

        if !self.bridge.supports_completion() {
            return Ok(empty_result);
        }

        let (parse_result_ast, _) = self.get_parse_result_cloned(&uri)?;
        let ast = match &parse_result_ast {
            Some(a) => a,
            None => return Ok(empty_result),
        };

        match self.bridge.completion(ast.as_ref(), pos) {
            None | Some(Err(_)) => Ok(empty_result),
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

    /// Process `textDocument/rename`.
    pub(crate) fn handle_rename(
        &mut self,
        _id: Value,
        params: Option<Value>,
    ) -> Result<Value, ResponseError> {
        let params = params.ok_or_else(|| ResponseError::invalid_params(None))?;
        let uri = parse_uri(&params);
        let pos = parse_position(&params);
        let new_name = params["newName"].as_str().unwrap_or("");

        if new_name.is_empty() {
            return Err(ResponseError::invalid_params(Some(json!(
                "newName is required"
            ))));
        }

        if !self.bridge.supports_rename() {
            return Err(ResponseError {
                code: REQUEST_FAILED,
                message: "rename not supported".to_string(),
                data: None,
            });
        }

        let (parse_result_ast, _) = self.get_parse_result_cloned(&uri)?;
        let ast = match &parse_result_ast {
            Some(a) => a,
            None => {
                return Err(ResponseError {
                    code: REQUEST_FAILED,
                    message: "no AST available".to_string(),
                    data: None,
                })
            }
        };

        match self.bridge.rename(ast.as_ref(), pos, new_name) {
            None | Some(Ok(None)) => Err(ResponseError {
                code: REQUEST_FAILED,
                message: "symbol not found at position".to_string(),
                data: None,
            }),
            Some(Err(e)) => Err(ResponseError {
                code: REQUEST_FAILED,
                message: e,
                data: None,
            }),
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

    /// Process `textDocument/documentSymbol`.
    pub(crate) fn handle_document_symbol(
        &mut self,
        _id: Value,
        params: Option<Value>,
    ) -> Result<Value, ResponseError> {
        let params = params.ok_or_else(|| ResponseError::invalid_params(None))?;
        let uri = parse_uri(&params);

        if !self.bridge.supports_document_symbols() {
            return Ok(json!([]));
        }

        let (parse_result_ast, _) = self.get_parse_result_cloned(&uri)?;
        let ast = match &parse_result_ast {
            Some(a) => a,
            None => return Ok(json!([])),
        };

        match self.bridge.document_symbols(ast.as_ref()) {
            None | Some(Err(_)) => Ok(json!([])),
            Some(Ok(symbols)) => Ok(Value::Array(convert_document_symbols(&symbols))),
        }
    }

    /// Process `textDocument/semanticTokens/full`.
    pub(crate) fn handle_semantic_tokens_full(
        &mut self,
        _id: Value,
        params: Option<Value>,
    ) -> Result<Value, ResponseError> {
        let params = params.ok_or_else(|| ResponseError::invalid_params(None))?;
        let uri = parse_uri(&params);

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
                let data = crate::capabilities::encode_semantic_tokens(&sem_tokens);
                Ok(json!({"data": data}))
            }
        }
    }

    /// Process `textDocument/foldingRange`.
    pub(crate) fn handle_folding_range(
        &mut self,
        _id: Value,
        params: Option<Value>,
    ) -> Result<Value, ResponseError> {
        let params = params.ok_or_else(|| ResponseError::invalid_params(None))?;
        let uri = parse_uri(&params);

        if !self.bridge.supports_folding_ranges() {
            return Ok(json!([]));
        }

        let (parse_result_ast, _) = self.get_parse_result_cloned(&uri)?;
        let ast = match &parse_result_ast {
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

    /// Process `textDocument/signatureHelp`.
    pub(crate) fn handle_signature_help(
        &mut self,
        _id: Value,
        params: Option<Value>,
    ) -> Result<Value, ResponseError> {
        let params = params.ok_or_else(|| ResponseError::invalid_params(None))?;
        let uri = parse_uri(&params);
        let pos = parse_position(&params);

        if !self.bridge.supports_signature_help() {
            return Ok(Value::Null);
        }

        let (parse_result_ast, _) = self.get_parse_result_cloned(&uri)?;
        let ast = match &parse_result_ast {
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

    /// Process `textDocument/formatting`.
    pub(crate) fn handle_formatting(
        &mut self,
        _id: Value,
        params: Option<Value>,
    ) -> Result<Value, ResponseError> {
        let params = params.ok_or_else(|| ResponseError::invalid_params(None))?;
        let uri = parse_uri(&params);

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
            Some(Err(e)) => Err(ResponseError {
                code: REQUEST_FAILED,
                message: format!("formatting failed: {}", e),
                data: None,
            }),
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
}

// ---------------------------------------------------------------------------
// Helper: convert DocumentSymbol tree to JSON
// ---------------------------------------------------------------------------

/// Recursively convert `DocumentSymbol` slices to JSON values for the LSP
/// response.
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
