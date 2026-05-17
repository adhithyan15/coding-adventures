//! # mosaic-formula-engine
//!
//! A standalone VisiCalc-style formula evaluator for spreadsheet cells.
//!
//! ## Overview
//!
//! This crate provides an in-memory spreadsheet engine with:
//!
//! - **Cell storage** — a 26-column (A–Z) × 99-row grid.
//! - **Formula evaluation** — arithmetic, cell references, ranges, and
//!   six built-in functions (`SUM`, `AVG`, `COUNT`, `MAX`, `MIN`, `IF`).
//! - **Dependency tracking** — automatically marks dependent cells dirty
//!   when a cell changes, and recalculates in topological order.
//! - **Cycle detection** — circular dependencies produce `#CIRC` errors.
//!
//! ## Quick start
//!
//! ```rust
//! use mosaic_formula_engine::{FormulaEngine, CellAddr};
//!
//! let mut engine = FormulaEngine::new();
//!
//! let a1 = CellAddr::parse("A1").unwrap();
//! let b1 = CellAddr::parse("B1").unwrap();
//!
//! engine.set_raw(a1.clone(), "5".to_string());
//! engine.set_raw(b1.clone(), "=A1*2".to_string());
//! engine.recalculate();
//!
//! assert_eq!(engine.get_display(&b1), "10");
//! ```
//!
//! ## Module map
//!
//! | Module | Responsibility |
//! |--------|---------------|
//! | `addr` | `CellAddr` — parsing and formatting cell addresses |
//! | `value` | `CellValue`, `FormulaError` — value types and error codes |
//! | `lexer` | Tokenise a formula string into `Vec<Token>` |
//! | `parser` | Parse tokens into an `Expr` AST |
//! | `eval` | Walk the AST to produce a `CellValue` |
//! | `builtins` | SUM, AVG, COUNT, MAX, MIN, IF implementations |
//! | `graph` | Dependency graph, topological sort, cycle detection |
//! | `engine` | `FormulaEngine` — the main public API |
//!
//! ## Design principles
//!
//! - **No external dependencies** — only `std`.
//! - **No unsafe** — zero `unsafe` blocks.
//! - **Literate code** — every module explains its algorithms with analogies
//!   and examples so that a newcomer to Rust can learn by reading.

pub mod addr;
pub mod builtins;
pub mod engine;
pub mod eval;
pub mod graph;
pub mod lexer;
pub mod parser;
pub mod value;

// Re-export the main public types at the crate root so users don't need to
// know which module they live in.
pub use addr::CellAddr;
pub use engine::FormulaEngine;
pub use value::{CellValue, FormulaError};
