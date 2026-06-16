//! # spreadsheet-core — headless spreadsheet engine.
//!
//! The Layer-3 engine in the backend-crate-catalog stack. Owns the
//! cell model, the formula AST, the dependency DAG, the recalc
//! algorithm, and the dispatch table that routes formula function
//! names to Layer-1 core implementations (statistics-core, math-core,
//! financial-core, lookup-core, text-core, datetime-core).
//!
//! What it does *not* own: any of the math itself (delegated), any
//! UI concerns (Mosaic / paint-vm / etc.), any I/O (xlsx-io, csv-io
//! live elsewhere).
//!
//! ## Quick start
//!
//! ```rust
//! use spreadsheet_core::{Workbook, CellAddress, CellContent, CellValue};
//!
//! let mut wb = Workbook::new();
//! let sheet = wb.add_sheet("Sheet1");
//! wb.set_value(sheet, CellAddress::new(1, 1), CellValue::Number(2.0));
//! wb.set_value(sheet, CellAddress::new(1, 2), CellValue::Number(3.0));
//! wb.set_formula(sheet, CellAddress::new(1, 3), "=A1+B1").unwrap();
//! wb.recalc_all();
//! let result = wb.get_value(sheet, CellAddress::new(1, 3));
//! assert_eq!(result, Some(CellValue::Number(5.0)));
//! ```
//!
//! ## Portability bar
//!
//! Per `backend-crate-catalog.md` §1: `forbid(unsafe_code)`, no
//! `#[cfg(target_os)]`, no I/O, no globals, WASM-compatible.

#![forbid(unsafe_code)]
#![warn(missing_docs)]

pub mod address;
pub mod ast;
pub mod cell;
pub mod dag;
pub mod dispatch;
pub mod edit;
pub mod errors;
pub mod parser;
pub mod recalc;
pub mod viewport;
pub mod workbook;

pub use address::{column_index_to_letters, column_letters_to_index, CellAddress, CellRange, SheetId};
pub use ast::{BinaryOp, FormulaAst, UnaryOp};
pub use cell::{Cell, CellContent, CellValue};
pub use edit::StructuralEdit;
pub use errors::SpreadsheetError;
pub use viewport::{ChangeSet, UsedRange, Window, CHANGELOG_RETAIN, MAX_WINDOW_CELLS};
pub use workbook::Workbook;
