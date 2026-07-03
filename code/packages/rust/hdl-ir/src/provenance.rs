//! Source-language provenance — every HIR node knows where it came from.
//!
//! Provenance is lightweight: a language tag plus an optional file/line/column.
//! Attaching it to every node has negligible runtime cost and is invaluable
//! for error messages that cite the original HDL source.

use serde::{Deserialize, Serialize};

/// Which front-end produced this node.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum SourceLang {
    Verilog,
    Vhdl,
    RubyDsl,
    Unknown,
}

/// A point in a source file. Line and column are 1-indexed per IEEE convention.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct SourceLocation {
    pub file: String,
    pub line: u32,
    pub column: u32,
}

impl SourceLocation {
    pub fn new(file: impl Into<String>, line: u32, column: u32) -> Self {
        assert!(line >= 1, "line must be >= 1");
        assert!(column >= 1, "column must be >= 1");
        Self { file: file.into(), line, column }
    }
}

/// Full provenance: language + optional location.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct Provenance {
    pub lang: SourceLang,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub location: Option<SourceLocation>,
}

impl Provenance {
    pub fn verilog(file: impl Into<String>, line: u32, col: u32) -> Self {
        Self {
            lang: SourceLang::Verilog,
            location: Some(SourceLocation::new(file, line, col)),
        }
    }

    pub fn vhdl(file: impl Into<String>, line: u32, col: u32) -> Self {
        Self {
            lang: SourceLang::Vhdl,
            location: Some(SourceLocation::new(file, line, col)),
        }
    }

    pub fn unknown() -> Self {
        Self { lang: SourceLang::Unknown, location: None }
    }
}
