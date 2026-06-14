//! # HDL Elaboration
//!
//! Converts Verilog HDL source code into HIR (Hardware Intermediate
//! Representation) through a three-pass process:
//!
//! | Pass | Name    | What it does |
//! |------|---------|--------------|
//! | 1    | Collect | Walk the AST, register every `module_declaration` in a symbol table |
//! | 2    | Bind    | For each collected module, elaborate ports and continuous assignments |
//! | 3    | Unroll  | Resolve references, validate, and produce a final `Hir` document |
//!
//! ## v0.1.0 scope
//!
//! Structural Verilog (ANSI 2001 style): modules, ports, and continuous
//! assignments (`assign lhs = expr;`). Behavioral constructs (`always`,
//! `initial`, functions, tasks) are v0.2.0 work.
//!
//! ## Usage
//!
//! ```rust
//! use hdl_elaboration::elaborate_verilog;
//!
//! let src = r#"
//!   module adder(input [3:0] a, input [3:0] b, output [4:0] sum);
//!     assign sum = a + b;
//!   endmodule
//! "#;
//! let hir = elaborate_verilog(src).unwrap();
//! assert_eq!(hir.top, "adder");
//! let m = &hir.modules["adder"];
//! assert_eq!(m.ports.len(), 3);
//! assert_eq!(m.cont_assigns.len(), 1);
//! ```

mod ast;
mod expr;
mod module;

use std::collections::HashMap;

use hdl_ir::{Hir, Module};
use parser::grammar_parser::GrammarASTNode;

use crate::ast::find_all;
use crate::module::elaborate_module;

// ---------------------------------------------------------------------------
// Error type
// ---------------------------------------------------------------------------

/// Errors produced during HDL elaboration.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum ElaborationError {
    /// The Verilog source could not be parsed.
    ParseError(String),
    /// The named top module was not found in the parsed source.
    TopModuleNotFound(String),
    /// A module declaration was malformed (e.g., missing name).
    InvalidModule(String),
    /// A port declaration was malformed.
    InvalidPort(String),
    /// An expression could not be elaborated.
    InvalidExpr(String),
}

impl std::fmt::Display for ElaborationError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::ParseError(m) => write!(f, "parse error: {m}"),
            Self::TopModuleNotFound(m) => write!(f, "top module '{m}' not found"),
            Self::InvalidModule(m) => write!(f, "invalid module: {m}"),
            Self::InvalidPort(m) => write!(f, "invalid port: {m}"),
            Self::InvalidExpr(m) => write!(f, "invalid expression: {m}"),
        }
    }
}

// ---------------------------------------------------------------------------
// Public API
// ---------------------------------------------------------------------------

/// Parse `source` as Verilog and elaborate all modules.
///
/// The top module is the first module declared in `source`.
pub fn elaborate_verilog(source: &str) -> Result<Hir, ElaborationError> {
    let ast = coding_adventures_verilog_parser::parse_verilog(source);
    elaborate_ast(&ast, None)
}

/// Parse `source` as Verilog and elaborate all modules, using `top` as the
/// name of the top-level module.
pub fn elaborate_verilog_with_top(source: &str, top: &str) -> Result<Hir, ElaborationError> {
    let ast = coding_adventures_verilog_parser::parse_verilog(source);
    elaborate_ast(&ast, Some(top))
}

/// Elaborate a pre-parsed Verilog AST into HIR.
///
/// `top_override` selects the top module; if `None`, the first module
/// declaration in the AST is used.
pub fn elaborate(ast: &GrammarASTNode, top_override: Option<&str>) -> Result<Hir, ElaborationError> {
    elaborate_ast(ast, top_override)
}

// ---------------------------------------------------------------------------
// Core elaboration
// ---------------------------------------------------------------------------

fn elaborate_ast(ast: &GrammarASTNode, top_override: Option<&str>) -> Result<Hir, ElaborationError> {
    // Pass 1: Collect all `module_declaration` nodes.
    let decls = find_all(ast, "module_declaration");
    if decls.is_empty() {
        let top = top_override.unwrap_or("unknown").to_string();
        return Ok(Hir::new(top));
    }

    // Pass 2: Elaborate each module declaration.
    let mut modules: HashMap<String, Module> = HashMap::new();
    let mut first_name: Option<String> = None;
    for decl in decls {
        let m = elaborate_module(decl)?;
        if first_name.is_none() { first_name = Some(m.name.clone()); }
        modules.insert(m.name.clone(), m);
    }

    // Pass 3: Determine the top module name.
    let top = if let Some(t) = top_override {
        if !modules.contains_key(t) {
            return Err(ElaborationError::TopModuleNotFound(t.to_string()));
        }
        t.to_string()
    } else {
        first_name.unwrap()
    };

    let mut hir = Hir::new(top);
    hir.modules = modules;
    Ok(hir)
}
