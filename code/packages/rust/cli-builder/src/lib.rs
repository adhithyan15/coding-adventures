//! # CLI Builder
//!
//! A declarative runtime library for CLI argument parsing, driven by
//! directed graphs and state machines.
//!
//! ## The Core Idea
//!
//! Building a CLI tool involves two concerns:
//!
//! 1. **What the tool accepts** — valid syntax: subcommands, flags, argument types.
//! 2. **What the tool does** — the implementation: business logic.
//!
//! Most CLI libraries mix these. CLI Builder separates them cleanly: you write
//! a JSON specification file describing the CLI's structure, and CLI Builder
//! handles all parsing, validation, help generation, and error reporting.
//!
//! ## Architecture: Two Data Structures, One JSON File
//!
//! ```text
//! JSON spec file
//!     │
//!     ├── SpecLoader ──► validates spec ──► G_cmd (command routing graph)
//!     │                                     G_flag (flag dependency graph)
//!     │
//!     └── Parser ──► Phase 1: Routing (DirectedGraph)
//!                ──► Phase 2: Scanning (ModalStateMachine + token DFA)
//!                ──► Phase 3: Validation (constraint checking)
//!                ──► ParseResult / HelpResult / VersionResult
//! ```
//!
//! ## Quick Start
//!
//! ```
//! use cli_builder::spec_loader::load_spec_from_str;
//! use cli_builder::parser::Parser;
//! use cli_builder::types::ParserOutput;
//!
//! let spec = load_spec_from_str(r#"{
//!     "cli_builder_spec_version": "1.0",
//!     "name": "greet",
//!     "description": "Print a greeting",
//!     "flags": [
//!         {"id": "shout", "short": "s", "long": "shout",
//!          "description": "Print in uppercase", "type": "boolean"}
//!     ],
//!     "arguments": [
//!         {"id": "name", "name": "NAME",
//!          "description": "Who to greet", "type": "string",
//!          "required": true}
//!     ]
//! }"#).expect("invalid spec");
//!
//! let parser = Parser::new(spec);
//! let args: Vec<String> = vec!["greet".into(), "--shout".into(), "Alice".into()];
//! match parser.parse(&args).expect("parse failed") {
//!     ParserOutput::Parse(result) => {
//!         assert_eq!(result.flags["shout"], serde_json::json!(true));
//!         assert_eq!(result.arguments["name"], serde_json::json!("Alice"));
//!     }
//!     ParserOutput::Help(h)    => { eprintln!("{}", h.text); }
//!     ParserOutput::Version(v) => { eprintln!("{}", v.version); }
//! }
//! ```
//!
//! ## Modules
//!
//! - [`spec_loader`] — parse and validate JSON spec files
//! - [`parser`] — the three-phase CLI parser
//! - [`types`] — spec schema types and parser output types
//! - [`errors`] — error types for spec validation and argv parsing
//! - [`token_classifier`] — token classification DFA (§5)
//! - [`positional_resolver`] — assign positional tokens to argument slots (§6.4.1)
//! - [`flag_validator`] — flag constraint validation (§6.4.2)
//! - [`help_generator`] — generate help text from the spec (§9)

pub mod errors;
pub mod flag_validator;
pub mod help_generator;
pub mod parser;
pub mod positional_resolver;
pub mod spec_loader;
pub mod token_classifier;
pub mod types;

// Re-export the most commonly used items at the crate root for convenience.
pub use errors::{CliBuilderError, ParseError, ParseErrors};
pub use parser::Parser;
pub use spec_loader::{load_spec_from_file, load_spec_from_str};
pub use types::{CliSpec, HelpResult, ParseResult, ParserOutput, VersionResult};
