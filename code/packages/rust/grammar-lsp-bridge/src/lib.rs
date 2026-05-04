//! # `grammar-lsp-bridge` — Generic grammar-driven LSP bridge.
//!
//! **LS02 PR A** — Implements [`ls00::LanguageBridge`] automatically from a
//! pair of `.tokens` + `.grammar` files plus a small declarative
//! [`LanguageSpec`] config.  Any language author who has those two files
//! gets a fully-functional LSP server (diagnostics, semantic tokens, symbols,
//! folding, hover, completion, formatting) without writing bespoke Rust.
//!
//! ## Quick start
//!
//! ```rust,ignore
//! use grammar_lsp_bridge::{GrammarLanguageBridge, LanguageSpec, LspSemanticTokenType};
//!
//! static MY_SPEC: LanguageSpec = LanguageSpec {
//!     name: "my-lang",
//!     file_extensions: &["ml"],
//!     tokens_source: include_str!("../my-lang.tokens"),
//!     grammar_source: include_str!("../my-lang.grammar"),
//!     token_kind_map: &[
//!         ("KEYWORD",  LspSemanticTokenType::Keyword),
//!         ("NAME",     LspSemanticTokenType::Variable),
//!         ("INTEGER",  LspSemanticTokenType::Number),
//!     ],
//!     declaration_rules: &["define", "let"],
//!     keyword_names: &["define", "let", "if", "else"],
//!     format_fn: None,
//! };
//!
//! // Then pass GrammarLanguageBridge::new(&MY_SPEC) to ls00::serve().
//! ```
//!
//! ## Architecture
//!
//! ```text
//! .tokens + .grammar
//!     │
//!     ▼ grammar-tools (runtime: GrammarLexer + GrammarParser)
//! GrammarASTNode tree + token stream
//!     │
//!     ▼ LanguageSpec (per-language config — ~20 lines)
//! GrammarLanguageBridge   implements ls00::LanguageBridge
//!     │
//!     ▼ ls00::LspServer
//! Editor (VS Code, Neovim, …)
//! ```
//!
//! ## Status — SKELETON (LS02 PR A in progress)
//!
//! Types and module structure are defined.  Implementations are TODO stubs.
//! See each module for the detailed implementation plan.

#![warn(missing_docs)]
#![warn(rust_2018_idioms)]

pub mod spec;
pub mod bridge;
pub mod tokenize;
pub mod parse;
pub mod semantic_tokens;
pub mod symbols;
pub mod folding;
pub mod hover;
pub mod completion;
pub mod format;

pub use spec::{LanguageSpec, LspSemanticTokenType};
pub use bridge::GrammarLanguageBridge;
