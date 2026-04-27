//! # ls00 -- Generic Language Server Protocol (LSP) Framework
//!
//! This crate implements a generic LSP server framework that language-specific
//! "bridges" plug into. It handles all the protocol boilerplate -- JSON-RPC
//! message dispatch, document synchronization, capability advertisement,
//! semantic token encoding -- so that a language author only needs to implement
//! the `LanguageBridge` trait.
//!
//! ## Architecture
//!
//! ```text
//! Lexer -> Parser -> [LanguageBridge] -> [LspServer] -> VS Code / Neovim / Emacs
//! ```
//!
//! ## How to use this crate
//!
//! 1. Implement the `LanguageBridge` trait for your language.
//! 2. Call `LspServer::new(bridge, reader, writer)`.
//! 3. Call `server.serve()` -- it blocks until the editor closes the connection.
//!
//! ## Module Map
//!
//! | Module             | Role                                          |
//! |--------------------|-----------------------------------------------|
//! | `types`            | All shared LSP data types                     |
//! | `language_bridge`  | The `LanguageBridge` trait                     |
//! | `document_manager` | Tracks open files, applies incremental edits  |
//! | `parse_cache`      | Caches parse results keyed by (uri, version)  |
//! | `capabilities`     | Builds capabilities + semantic token encoding |
//! | `lsp_errors`       | LSP-specific error code constants             |
//! | `server`           | `LspServer` -- the main coordinator           |
//! | `handlers`         | All LSP handler implementations               |

pub mod capabilities;
pub mod document_manager;
pub mod handlers;
pub mod language_bridge;
pub mod lsp_errors;
pub mod parse_cache;
pub mod server;
pub mod types;
