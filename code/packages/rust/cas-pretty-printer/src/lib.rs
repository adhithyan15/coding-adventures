//! Pretty-print symbolic IR back to source text.
//!
//! The package is dialect-aware: the same walker emits MACSYMA,
//! Mathematica, Maple, or Lisp by swapping a small [`Dialect`] object.
//!
//! # Quick start
//!
//! ```rust
//! use cas_pretty_printer::{pretty, MacsymaDialect};
//! use symbolic_ir::{apply, int, sym, ADD, POW};
//!
//! let x = sym("x");
//! let expr = apply(sym(ADD), vec![
//!     apply(sym(POW), vec![x.clone(), int(2)]),
//!     int(1),
//! ]);
//! assert_eq!(pretty(&expr, &MacsymaDialect), "x^2 + 1");
//! ```
//!
//! Use [`pretty_2d`] for multi-line box layout:
//!
//! ```rust
//! use cas_pretty_printer::{pretty_2d, MacsymaDialect};
//! use symbolic_ir::{apply, int, sym, DIV};
//!
//! let expr = apply(sym(DIV), vec![int(1), int(2)]);
//! assert_eq!(pretty_2d(&expr, &MacsymaDialect), " 1 \n───\n 2 ");
//! ```
//!
//! For the always-prefix Lisp form, use [`format_lisp`] (which bypasses the
//! walker entirely and ignores any registered head formatters):
//!
//! ```rust
//! use cas_pretty_printer::format_lisp;
//! use symbolic_ir::{apply, int, sym, ADD, POW};
//!
//! let x = sym("x");
//! let expr = apply(sym(ADD), vec![
//!     apply(sym(POW), vec![x.clone(), int(2)]),
//!     int(1),
//! ]);
//! assert_eq!(format_lisp(&expr), "(Add (Pow x 2) 1)");
//! ```
//!
//! # Extensibility
//!
//! Downstream crates can register formatters for new IR heads:
//!
//! ```rust
//! use cas_pretty_printer::{pretty, register_head_formatter, unregister_head_formatter, MacsymaDialect};
//! use symbolic_ir::{apply, int, sym, IRNode};
//!
//! register_head_formatter("Matrix", |node, _dialect, fmt| {
//!     let rows: Vec<String> = node.args.iter()
//!         .map(|row| {
//!             if let IRNode::Apply(a) = row {
//!                 let cells: Vec<String> = a.args.iter().map(|c| fmt(c)).collect();
//!                 format!("[{}]", cells.join(", "))
//!             } else {
//!                 fmt(row)
//!             }
//!         })
//!         .collect();
//!     format!("matrix({})", rows.join(", "))
//! });
//!
//! // ... use the formatter ...
//!
//! unregister_head_formatter("Matrix");  // clean up in tests
//! ```
//!
//! # Stack position
//!
//! ```text
//! symbolic-ir  ←  cas-pretty-printer
//! ```

pub mod box_layout;
pub mod dialect;
pub mod lisp;
pub mod macsyma;
pub mod maple;
pub mod mathematica;
pub mod walker;

// Re-export the public surface API.
pub use box_layout::{atom, hbox, pretty_2d, vbox, Align, Box as LayoutBox};
pub use dialect::{
    Dialect, PREC_ADD, PREC_AND, PREC_ATOM, PREC_CALL, PREC_CMP, PREC_MUL, PREC_NEG, PREC_NOT,
    PREC_OR, PREC_POW,
};
pub use lisp::{format_lisp, LispDialect};
pub use macsyma::MacsymaDialect;
pub use maple::MapleDialect;
pub use mathematica::MathematicaDialect;
pub use walker::{pretty, register_head_formatter, unregister_head_formatter};
