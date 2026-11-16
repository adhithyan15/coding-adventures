//! Error types for Flash display-object hierarchy and scene diffing (`SWF05`).

use paint_palette_blend::PaletteBlendError;
use std::fmt;
use swf_container::SwfError;

/// Core error type representing failures in display list operations,
/// timeline navigation, depth placement, and paint scene lowering.
#[derive(Debug, Clone, PartialEq)]
pub enum DisplayListError {
    /// MovieClip hierarchy nesting exceeded the maximum allowed depth (`MAX_HIERARCHY_DEPTH = 64`).
    HierarchyDepthExceeded,
    /// Specified display depth is out of supported bounds (`-16384..=65535`).
    InvalidDepth,
    /// Clip depth must be strictly greater than mask object depth.
    InvalidClipDepth,
    /// Referenced timeline frame number or label does not exist.
    InvalidFrame,
    /// Specified depth slot is already occupied.
    SlotOccupied,
    /// Specified depth slot was not found.
    SlotNotFound,
    /// Morph shape ratio must be in `0..=65535`.
    InvalidRatio,
    /// Affine transformation or coordinate arithmetic overflowed.
    TransformOverflow,
    /// Tag payload length is invalid.
    InvalidTagLength,
    /// Bit field contains invalid or unsupported flags.
    InvalidBitField,
    /// Unexpected end of input buffer while parsing tag payload.
    UnexpectedEof,
    /// Tag data or shape record is corrupted.
    CorruptedData,
    /// Retained-mode paint scene diffing failure from `paint-palette-blend`.
    PaletteBlend(PaletteBlendError),
}

impl fmt::Display for DisplayListError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::HierarchyDepthExceeded => {
                write!(f, "display hierarchy depth limit exceeded (max 64)")
            }
            Self::InvalidDepth => write!(f, "display depth out of valid bounds (-16384..=65535)"),
            Self::InvalidClipDepth => {
                write!(f, "clip depth must be strictly greater than mask depth")
            }
            Self::InvalidFrame => write!(f, "invalid timeline frame index or label"),
            Self::SlotOccupied => write!(f, "depth slot is already occupied"),
            Self::SlotNotFound => write!(f, "depth slot not found"),
            Self::InvalidRatio => write!(f, "morph ratio out of range (0..=65535)"),
            Self::TransformOverflow => write!(f, "transform coordinate overflow"),
            Self::InvalidTagLength => write!(f, "invalid display list tag length"),
            Self::InvalidBitField => write!(f, "invalid bit field in tag flags"),
            Self::UnexpectedEof => write!(f, "unexpected end of tag data"),
            Self::CorruptedData => write!(f, "corrupted display tag data"),
            Self::PaletteBlend(e) => write!(f, "paint palette blend error: {e}"),
        }
    }
}

impl std::error::Error for DisplayListError {}

impl From<SwfError> for DisplayListError {
    fn from(err: SwfError) -> Self {
        match err {
            SwfError::UnexpectedEof => Self::UnexpectedEof,
            SwfError::InvalidTagLength => Self::InvalidTagLength,
            SwfError::InvalidBitField => Self::InvalidBitField,
            SwfError::CorruptedData => Self::CorruptedData,
            _ => Self::CorruptedData,
        }
    }
}

impl From<PaletteBlendError> for DisplayListError {
    fn from(err: PaletteBlendError) -> Self {
        Self::PaletteBlend(err)
    }
}
