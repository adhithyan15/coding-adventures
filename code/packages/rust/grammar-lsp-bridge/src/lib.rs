//! # `grammar-lsp-bridge` — Generic grammar-driven LSP bridge.
//!
//! **LS02 PR A** — Implements [`coding_adventures_ls00::language_bridge::LanguageBridge`]
//! automatically from a pair of `.tokens` + `.grammar` files plus a small
//! declarative [`LanguageSpec`] config.  Any language author who has those two
//! files gets a fully-functional LSP server with diagnostics, semantic tokens,
//! document symbols, folding ranges, hover, completion, and formatting —
//! without writing bespoke Rust.
//!
//! ## Quick start
//!
//! ```rust,ignore
//! use grammar_lsp_bridge::{GrammarLanguageBridge, LanguageSpec, LspSemanticTokenType};
//!
//! static MY_SPEC: LanguageSpec = LanguageSpec {
//!     language_name:     "my-lang",
//!     file_extensions:   &["ml"],
//!     tokens_source:     include_str!("my-lang.tokens"),
//!     grammar_source:    include_str!("my-lang.grammar"),
//!     token_kind_map:    &[
//!         ("KEYWORD", LspSemanticTokenType::Keyword),
//!         ("NAME",    LspSemanticTokenType::Variable),
//!         ("INTEGER", LspSemanticTokenType::Number),
//!     ],
//!     declaration_rules: &["function_def", "let_binding"],
//!     keyword_names:     &["define", "let", "if", "else"],
//!     format_fn:         None,
//! };
//!
//! fn main() {
//!     let bridge = GrammarLanguageBridge::new(&MY_SPEC);
//!     coding_adventures_ls00::serve_stdio(bridge).expect("LSP error");
//! }
//! ```
//!
//! ## Architecture
//!
//! ```text
//! .tokens + .grammar
//!     │
//!     ▼  grammar-tools parses the grammar specs
//! TokenGrammar + ParserGrammar   (stored in GrammarLanguageBridge)
//!     │
//!     ▼  lexer::grammar_lexer::GrammarLexer  (runtime tokenisation)
//! Vec<lexer::token::Token>
//!     │
//!     ▼  parser::grammar_parser::GrammarParser (runtime parsing)
//! GrammarASTNode tree  (stored as Box<dyn Any + Send + Sync> in ls00 cache)
//!     │
//!     ▼  GrammarLanguageBridge (implements ls00::LanguageBridge)
//! Editor (VS Code, Neovim, …)
//! ```
//!
//! ## Crate layout
//!
//! | Module  | Contents                                            |
//! |---------|-----------------------------------------------------|
//! | `spec`  | [`LanguageSpec`] + [`LspSemanticTokenType`]         |
//! | `bridge`| [`GrammarLanguageBridge`] — full `LanguageBridge`   |

#![warn(missing_docs)]
#![warn(rust_2018_idioms)]

pub mod bridge;
pub mod spec;

pub use bridge::GrammarLanguageBridge;
pub use spec::{LanguageSpec, LspSemanticTokenType};
