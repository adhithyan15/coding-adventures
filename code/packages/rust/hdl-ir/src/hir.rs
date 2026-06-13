//! Top-level HIR document with JSON round-trip.
//!
//! The `Hir` struct is the root of the hardware IR. It names the top-level
//! module and provides a flat map of all module definitions. Libraries
//! (VHDL multi-library designs) are kept in a parallel map.
//!
//! ## JSON schema (`format: "HIR"`, `version: "0.1.0"`)
//!
//! ```json
//! {
//!   "format": "HIR",
//!   "version": "0.1.0",
//!   "top": "adder4",
//!   "modules": {
//!     "adder4": { "ports": [...], "cont_assigns": [...], ... }
//!   }
//! }
//! ```
//!
//! ## Version policy
//!
//! Only the major version (first `.`-segment) is checked. A file written by
//! v0.9.0 is accepted by a v0.1.0 reader; a file written by v1.0.0 is rejected.

use std::collections::HashMap;

use serde::{Deserialize, Serialize};

use crate::module::{Library, Module};
use crate::provenance::Provenance;

pub const SCHEMA_VERSION: &str = "0.1.0";

// ---------------------------------------------------------------------------
// HIR document
// ---------------------------------------------------------------------------

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct Hir {
    #[serde(rename = "format", default = "default_format")]
    pub format: String,
    #[serde(default = "default_version")]
    pub version: String,
    pub top: String,
    #[serde(default)]
    pub modules: HashMap<String, Module>,
    #[serde(default)]
    pub libraries: HashMap<String, Library>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub provenance: Option<Provenance>,
}

fn default_format() -> String { "HIR".to_string() }
fn default_version() -> String { SCHEMA_VERSION.to_string() }

impl Hir {
    pub fn new(top: impl Into<String>) -> Self {
        Self {
            format: "HIR".to_string(),
            version: SCHEMA_VERSION.to_string(),
            top: top.into(),
            modules: HashMap::new(),
            libraries: HashMap::new(),
            provenance: None,
        }
    }

    /// Serialize to compact JSON string.
    pub fn to_json(&self) -> Result<String, serde_json::Error> {
        serde_json::to_string(self)
    }

    /// Serialize to pretty JSON string.
    pub fn to_json_pretty(&self) -> Result<String, serde_json::Error> {
        serde_json::to_string_pretty(self)
    }

    /// Deserialize from JSON string. Validates `format` and major version.
    pub fn from_json(s: &str) -> Result<Self, HirError> {
        let hir: Self = serde_json::from_str(s).map_err(HirError::Json)?;
        if hir.format != "HIR" {
            return Err(HirError::BadFormat(hir.format));
        }
        let file_major = hir.version.split('.').next().unwrap_or("0");
        let lib_major = SCHEMA_VERSION.split('.').next().unwrap_or("0");
        if file_major != lib_major {
            return Err(HirError::VersionMismatch {
                file: hir.version.clone(),
                lib: SCHEMA_VERSION.to_string(),
            });
        }
        Ok(hir)
    }

    /// Summary statistics.
    pub fn stats(&self) -> HirStats {
        HirStats {
            module_count: self.modules.len(),
            instance_count: self.modules.values().map(|m| m.instances.len()).sum(),
            process_count: self.modules.values().map(|m| m.processes.len()).sum(),
            cont_assign_count: self.modules.values().map(|m| m.cont_assigns.len()).sum(),
            net_count: self.modules.values().map(|m| m.nets.len()).sum(),
        }
    }
}

// ---------------------------------------------------------------------------
// Stats + Error
// ---------------------------------------------------------------------------

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct HirStats {
    pub module_count: usize,
    pub instance_count: usize,
    pub process_count: usize,
    pub cont_assign_count: usize,
    pub net_count: usize,
}

#[derive(Debug)]
pub enum HirError {
    Json(serde_json::Error),
    BadFormat(String),
    VersionMismatch { file: String, lib: String },
}

impl std::fmt::Display for HirError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            HirError::Json(e) => write!(f, "JSON error: {e}"),
            HirError::BadFormat(fmt) => write!(f, "not an HIR document (format={fmt:?})"),
            HirError::VersionMismatch { file, lib } => {
                write!(f, "HIR major version mismatch: file={file}, lib={lib}")
            }
        }
    }
}

impl std::error::Error for HirError {}
