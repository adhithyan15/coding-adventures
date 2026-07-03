//! HIR type system.
//!
//! Hardware types are simpler than software types: almost everything is
//! either a single bit, a vector of bits, or a bounded integer. The type
//! system here is deliberately minimal — just enough for width inference
//! during synthesis, not a full type checker.
//!
//! ```text
//! Ty::Bit          — one bit (Verilog `wire`, VHDL `std_logic`)
//! Ty::Vec(n)       — n-bit vector (Verilog [n-1:0], VHDL std_logic_vector)
//! Ty::Int          — unbounded integer (for parameters/generics)
//! Ty::Bool         — boolean (VHDL `boolean`)
//! Ty::Real         — IEEE double (for testbench math)
//! Ty::Array(n, ty) — n-element array of ty (multi-dim vectors, memories)
//! Ty::Record(fields) — VHDL record / Verilog struct
//! Ty::Unknown      — placeholder when elaboration hasn't resolved the type yet
//! ```

use serde::{Deserialize, Serialize};

/// A field in a Record type.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct RecordField {
    pub name: String,
    pub ty: Box<Ty>,
}

/// Hardware type.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(tag = "kind", rename_all = "snake_case")]
pub enum Ty {
    Bit,
    Vec { width: u32 },
    Int,
    Bool,
    Real,
    Array { size: u32, element: Box<Ty> },
    Record { fields: Vec<RecordField> },
    Unknown,
}

impl Ty {
    /// Return the bit-width of this type, if statically known.
    /// Returns `None` for Int, Real, Record, and Unknown.
    pub fn width(&self) -> Option<u32> {
        match self {
            Ty::Bit => Some(1),
            Ty::Vec { width } => Some(*width),
            Ty::Bool => Some(1),
            Ty::Array { size, element } => {
                element.width().map(|ew| size * ew)
            }
            _ => None,
        }
    }

    pub fn bit() -> Self {
        Ty::Bit
    }

    pub fn vec(width: u32) -> Self {
        Ty::Vec { width }
    }
}
