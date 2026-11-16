//! Error types for SWF shape parsing, reconstruction, and lowering.

#![deny(unsafe_code)]

use std::fmt;
use swf_container::SwfError;

/// Errors produced during SWF shape parsing and lowering.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum ShapeError {
    /// Tag is not a supported shape tag (must be 2, 22, 32, or 83).
    InvalidTag,
    /// Binary input ended unexpectedly.
    UnexpectedEof,
    /// Bit field length or value is invalid.
    InvalidBitField,
    /// Number of fill or line styles exceeded security budget (CWE-400).
    ExcessiveStyles,
    /// Number of shape records or edges exceeded security budget (CWE-400).
    ExcessiveEdges,
    /// Style index references a non-existent fill or line style (CWE-125).
    InvalidStyleIndex,
    /// Winding or contour reconstruction topology error.
    WindingError,
    /// Record structure is corrupted or invalid.
    CorruptedRecord,
    /// Arithmetic overflow in coordinate or delta calculations (CWE-190).
    ArithmeticOverflow,
    /// Underlying container bit reader error.
    ContainerError(SwfError),
}

impl fmt::Display for ShapeError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::InvalidTag => write!(f, "tag is not a valid shape tag"),
            Self::UnexpectedEof => write!(f, "unexpected end of binary stream"),
            Self::InvalidBitField => write!(f, "invalid bit field width or value"),
            Self::ExcessiveStyles => write!(f, "style count exceeds maximum security limit"),
            Self::ExcessiveEdges => write!(f, "edge count exceeds maximum security limit"),
            Self::InvalidStyleIndex => write!(f, "style index out of bounds"),
            Self::WindingError => write!(f, "edge contour winding reconstruction error"),
            Self::CorruptedRecord => write!(f, "corrupted shape record"),
            Self::ArithmeticOverflow => write!(f, "arithmetic overflow in coordinates"),
            Self::ContainerError(e) => write!(f, "container error: {e}"),
        }
    }
}

impl std::error::Error for ShapeError {}

impl From<SwfError> for ShapeError {
    fn from(e: SwfError) -> Self {
        Self::ContainerError(e)
    }
}
