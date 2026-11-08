//! # Weakness Registry Parser & Model (VER15)
//!
//! Loads and validates `weakness-registry-v1.json` per VER00 §8.2.

#![deny(unsafe_code)]

use std::collections::HashSet;
use std::fmt;
use std::fs;
use std::path::Path;

use crate::json::JsonValue;

/// A single weakness class in the registry.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct WeaknessClass {
    pub cwe: u32,
    pub name: String,
    pub family: String,
    pub description: String,
    pub applies_to: Vec<String>,
    pub safe_subsets: Vec<String>,
    pub exclusion_rationale: Option<String>,
    pub attack_patterns: Vec<String>,
    pub required_evidence: Vec<String>,
    pub minimum_proof_label: String,
}

/// The repository-wide weakness registry catalog.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct WeaknessRegistry {
    pub version: u32,
    pub last_updated: String,
    pub classes: Vec<WeaknessClass>,
}

#[derive(Debug)]
pub enum RegistryError {
    Io(String),
    JsonParse(String),
    Validation(String),
}

impl fmt::Display for RegistryError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            RegistryError::Io(e) => write!(f, "Registry I/O error: {e}"),
            RegistryError::JsonParse(e) => write!(f, "Registry JSON parse error: {e}"),
            RegistryError::Validation(e) => write!(f, "Registry validation error: {e}"),
        }
    }
}

impl std::error::Error for RegistryError {}

impl WeaknessRegistry {
    /// Load registry from file on disk.
    pub fn load_from_file(path: &Path) -> Result<Self, RegistryError> {
        let content = fs::read_to_string(path)
            .map_err(|e| RegistryError::Io(format!("{}: {e}", path.display())))?;
        Self::load_from_str(&content)
    }

    /// Load and validate registry from JSON string.
    pub fn load_from_str(json_str: &str) -> Result<Self, RegistryError> {
        let val = JsonValue::parse(json_str).map_err(RegistryError::JsonParse)?;
        Self::from_json(&val)
    }

    /// Return the embedded canonical registry v1 (snapshot).
    pub fn embedded() -> Self {
        const EMBEDDED_JSON: &str = include_str!("../../../../specs/data/weakness-registry-v1.json");
        Self::load_from_str(EMBEDDED_JSON)
            .expect("embedded weakness-registry-v1.json must be valid")
    }

    pub fn from_json(val: &JsonValue) -> Result<Self, RegistryError> {
        let obj = val
            .as_object()
            .ok_or_else(|| RegistryError::Validation("root must be a JSON object".into()))?;

        let version = obj
            .get("version")
            .and_then(|v| v.as_u32())
            .ok_or_else(|| RegistryError::Validation("missing or invalid 'version'".into()))?;
        if version != 1 {
            return Err(RegistryError::Validation(format!(
                "unsupported registry version {version}, expected 1"
            )));
        }

        let last_updated = obj
            .get("last_updated")
            .and_then(|v| v.as_str())
            .unwrap_or("2026-11-08")
            .to_string();

        let classes_arr = obj
            .get("classes")
            .and_then(|v| v.as_array())
            .ok_or_else(|| RegistryError::Validation("missing or non-array 'classes'".into()))?;

        let mut classes = Vec::with_capacity(classes_arr.len());
        let mut seen_cwes = HashSet::new();

        for (i, c_val) in classes_arr.iter().enumerate() {
            let c_obj = c_val.as_object().ok_or_else(|| {
                RegistryError::Validation(format!("classes[{i}] must be an object"))
            })?;

            let cwe = c_obj
                .get("cwe")
                .and_then(|v| v.as_u32())
                .ok_or_else(|| RegistryError::Validation(format!("classes[{i}].cwe missing or not u32")))?;

            if !seen_cwes.insert(cwe) {
                return Err(RegistryError::Validation(format!(
                    "duplicate CWE-{cwe} at classes[{i}]"
                )));
            }

            let name = c_obj
                .get("name")
                .and_then(|v| v.as_str())
                .ok_or_else(|| RegistryError::Validation(format!("classes[{i}].name missing")))?
                .to_string();

            let family = c_obj
                .get("family")
                .and_then(|v| v.as_str())
                .ok_or_else(|| RegistryError::Validation(format!("classes[{i}].family missing")))?
                .to_string();

            let description = c_obj
                .get("description")
                .and_then(|v| v.as_str())
                .unwrap_or("")
                .to_string();

            let applies_to = c_obj
                .get("applies_to")
                .and_then(|v| v.as_array())
                .map(|a| a.iter().filter_map(|x| x.as_str().map(String::from)).collect())
                .unwrap_or_default();

            let safe_subsets = c_obj
                .get("safe_subsets")
                .and_then(|v| v.as_array())
                .map(|a| a.iter().filter_map(|x| x.as_str().map(String::from)).collect())
                .unwrap_or_default();

            let exclusion_rationale = c_obj
                .get("exclusion_rationale")
                .and_then(|v| v.as_str())
                .map(String::from);

            let attack_patterns = c_obj
                .get("attack_patterns")
                .and_then(|v| v.as_array())
                .map(|a| a.iter().filter_map(|x| x.as_str().map(String::from)).collect())
                .unwrap_or_default();

            let required_evidence = c_obj
                .get("required_evidence")
                .and_then(|v| v.as_array())
                .map(|a| a.iter().filter_map(|x| x.as_str().map(String::from)).collect())
                .unwrap_or_default();

            let minimum_proof_label = c_obj
                .get("minimum_proof_label")
                .and_then(|v| v.as_str())
                .unwrap_or("proved")
                .to_string();

            classes.push(WeaknessClass {
                cwe,
                name,
                family,
                description,
                applies_to,
                safe_subsets,
                exclusion_rationale,
                attack_patterns,
                required_evidence,
                minimum_proof_label,
            });
        }

        if classes.len() < 20 {
            return Err(RegistryError::Validation(format!(
                "registry contains only {} classes, expected at least 20",
                classes.len()
            )));
        }

        Ok(Self {
            version,
            last_updated,
            classes,
        })
    }

    /// Look up a weakness class by CWE identifier.
    pub fn find_class(&self, cwe: u32) -> Option<&WeaknessClass> {
        self.classes.iter().find(|c| c.cwe == cwe)
    }

    /// Total number of classes in the registry.
    pub fn len(&self) -> usize {
        self.classes.len()
    }

    pub fn is_empty(&self) -> bool {
        self.classes.is_empty()
    }
}
