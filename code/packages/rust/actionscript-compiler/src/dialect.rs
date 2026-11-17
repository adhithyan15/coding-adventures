//! ActionScript language dialects supported by SWF03.

#![deny(unsafe_code)]

use crate::error::AsCompilerError;

/// ActionScript language dialect.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum AsDialect {
    /// ActionScript 1.0: Dynamic prototype-based ECMAScript 3 subset with Flash timeline verbs.
    As1,
    /// ActionScript 2.0: Classes, interfaces, type annotations lowered to prototypes.
    As2,
    /// ActionScript 3.0: ECMAScript 4 dialect with packages, namespaces, Vector.<T>, try/catch/finally, is/as.
    As3,
}

impl AsDialect {
    /// Parse dialect from an integer (1, 2, or 3).
    pub fn dialect_from_u8(v: u8) -> Result<Self, AsCompilerError> {
        match v {
            1 => Ok(Self::As1),
            2 => Ok(Self::As2),
            3 => Ok(Self::As3),
            _ => Err(AsCompilerError::UnsupportedDialect),
        }
    }

    /// Return dialect identifier string.
    pub fn dialect_name(&self) -> &'static str {
        match self {
            Self::As1 => "as1.0",
            Self::As2 => "as2.0",
            Self::As3 => "as3.0",
        }
    }

    /// Returns true if this dialect supports AS2+ classes and interfaces.
    pub fn dialect_supports_classes(&self) -> bool {
        matches!(self, Self::As2 | Self::As3)
    }

    /// Returns true if this dialect supports AS3 packages and namespaces.
    pub fn dialect_supports_packages(&self) -> bool {
        matches!(self, Self::As3)
    }
}
