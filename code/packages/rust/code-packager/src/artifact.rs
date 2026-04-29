//! The `CodeArtifact` — the packager's input.
//!
//! A `CodeArtifact` carries everything the packager needs to wrap native bytes
//! into an OS-specific binary:
//!
//! - **`native_bytes`** — the raw machine code produced by a code generator.
//! - **`entry_point`** — byte offset into `native_bytes` where execution starts.
//! - **`target`** — which CPU and OS the bytes are meant for.
//! - **`symbol_table`** — optional map from symbol names to byte offsets.
//! - **`metadata`** — open-ended key/value store for packager-specific hints
//!   (e.g. `"load_address"`, `"origin"`, `"exports"`).
//!
//! ## Metadata values
//!
//! Three value kinds cover the packager's needs:
//!
//! ```text
//! MetadataValue
//! ├── Int(i64)          — numeric hint, e.g. load_address = 0x400000
//! ├── Str(String)       — string hint, e.g. subsystem = "console"
//! └── List(Vec<String>) — list hint, e.g. exports = ["main", "add"]
//! ```
//!
//! ## Builder pattern
//!
//! ```rust
//! use code_packager::{CodeArtifact, MetadataValue, Target};
//! use std::collections::HashMap;
//!
//! let artifact = CodeArtifact::new(vec![0x90], 0, Target::linux_x64())
//!     .with_metadata({
//!         let mut m = HashMap::new();
//!         m.insert("load_address".into(), MetadataValue::Int(0x600000));
//!         m
//!     });
//! assert_eq!(artifact.metadata_int("load_address", 0x400000), 0x600000);
//! ```

use std::collections::HashMap;

use crate::target::Target;

// ── MetadataValue ─────────────────────────────────────────────────────────────

/// A typed metadata value attached to a `CodeArtifact`.
///
/// Metadata is used to pass packager-specific hints that are not part of the
/// core artifact (e.g. a custom load address, a list of function exports, or
/// a subsystem identifier for Windows PE files).
#[derive(Debug, Clone, PartialEq)]
pub enum MetadataValue {
    /// A signed 64-bit integer (covers all address-sized values).
    Int(i64),
    /// A UTF-8 string value.
    Str(String),
    /// A list of UTF-8 strings (e.g. WASM export names).
    List(Vec<String>),
}

// ── CodeArtifact ──────────────────────────────────────────────────────────────

/// The input to the packager: native bytes plus everything needed to wrap them.
///
/// ```text
/// CodeArtifact
/// │
/// ├── native_bytes  ─ the raw machine code
/// │                   e.g. [0x55, 0x48, 0x89, 0xE5, ...]  (x86-64 prologue)
/// │
/// ├── entry_point   ─ byte offset into native_bytes where execution begins
/// │                   (0 = first byte is the entry point)
/// │
/// ├── target        ─ Target { arch, os, binary_format }
/// │
/// ├── symbol_table  ─ { "main" → 0, "add" → 48, ... }
/// │
/// └── metadata      ─ { "load_address" → Int(0x400000), "exports" → List([...]) }
/// ```
pub struct CodeArtifact {
    /// Raw machine-code bytes produced by the code generator.
    pub native_bytes: Vec<u8>,
    /// Byte offset (0-based) into `native_bytes` where the CPU starts executing.
    pub entry_point: usize,
    /// Which CPU and OS this code targets.
    pub target: Target,
    /// Maps symbol names to byte offsets within `native_bytes`.
    pub symbol_table: HashMap<String, usize>,
    /// Open-ended packager hints, keyed by string.
    pub metadata: HashMap<String, MetadataValue>,
}

impl CodeArtifact {
    // ── Constructors ──────────────────────────────────────────────────────────

    /// Create a minimal artifact with empty `symbol_table` and `metadata`.
    ///
    /// # Arguments
    ///
    /// * `native_bytes` — the raw machine code.
    /// * `entry_point` — byte offset of the first instruction.
    /// * `target` — the compilation target.
    pub fn new(native_bytes: Vec<u8>, entry_point: usize, target: Target) -> Self {
        Self {
            native_bytes,
            entry_point,
            target,
            symbol_table: HashMap::new(),
            metadata: HashMap::new(),
        }
    }

    // ── Builder methods ───────────────────────────────────────────────────────

    /// Attach a symbol table and return `self` (builder style).
    ///
    /// # Example
    ///
    /// ```rust
    /// use code_packager::{CodeArtifact, Target};
    /// use std::collections::HashMap;
    ///
    /// let mut st = HashMap::new();
    /// st.insert("main".into(), 0usize);
    /// let artifact = CodeArtifact::new(vec![], 0, Target::linux_x64())
    ///     .with_symbol_table(st);
    /// assert_eq!(artifact.symbol_table["main"], 0);
    /// ```
    pub fn with_symbol_table(mut self, st: HashMap<String, usize>) -> Self {
        self.symbol_table = st;
        self
    }

    /// Attach metadata and return `self` (builder style).
    pub fn with_metadata(mut self, md: HashMap<String, MetadataValue>) -> Self {
        self.metadata = md;
        self
    }

    // ── Metadata accessors ────────────────────────────────────────────────────

    /// Read an `Int` metadata value, falling back to `default` if absent or wrong type.
    ///
    /// # Example
    ///
    /// ```rust
    /// use code_packager::{CodeArtifact, MetadataValue, Target};
    /// use std::collections::HashMap;
    ///
    /// let mut m = HashMap::new();
    /// m.insert("load_address".into(), MetadataValue::Int(0x600000));
    /// let artifact = CodeArtifact::new(vec![], 0, Target::raw("x86_64"))
    ///     .with_metadata(m);
    ///
    /// assert_eq!(artifact.metadata_int("load_address", 0), 0x600000);
    /// assert_eq!(artifact.metadata_int("missing", 42),    42);
    /// ```
    pub fn metadata_int(&self, key: &str, default: i64) -> i64 {
        match self.metadata.get(key) {
            Some(MetadataValue::Int(n)) => *n,
            _ => default,
        }
    }

    /// Read a `Str` metadata value, falling back to `default` if absent or wrong type.
    ///
    /// Returns an owned `String` so callers do not need to worry about lifetimes.
    pub fn metadata_str(&self, key: &str, default: &str) -> String {
        match self.metadata.get(key) {
            Some(MetadataValue::Str(s)) => s.clone(),
            _ => default.to_string(),
        }
    }

    /// Read a `List` metadata value. Returns an empty `Vec` if absent or wrong type.
    pub fn metadata_list(&self, key: &str) -> Vec<String> {
        match self.metadata.get(key) {
            Some(MetadataValue::List(v)) => v.clone(),
            _ => Vec::new(),
        }
    }
}

// ── Tests ─────────────────────────────────────────────────────────────────────

#[cfg(test)]
mod tests {
    use super::*;

    // Test 1: new() produces empty symbol_table and metadata
    #[test]
    fn new_has_empty_symbol_table_and_metadata() {
        let art = CodeArtifact::new(vec![0x90], 0, Target::linux_x64());
        assert!(art.symbol_table.is_empty());
        assert!(art.metadata.is_empty());
        assert_eq!(art.native_bytes, vec![0x90]);
        assert_eq!(art.entry_point, 0);
    }

    // Test 2: with_symbol_table builder
    #[test]
    fn with_symbol_table_builder() {
        let mut st = HashMap::new();
        st.insert("main".into(), 0usize);
        st.insert("add".into(), 16usize);
        let art = CodeArtifact::new(vec![], 0, Target::linux_x64())
            .with_symbol_table(st);
        assert_eq!(art.symbol_table["main"], 0);
        assert_eq!(art.symbol_table["add"], 16);
    }

    // Test 3: metadata_int default when absent
    #[test]
    fn metadata_int_default_when_absent() {
        let art = CodeArtifact::new(vec![], 0, Target::linux_x64());
        assert_eq!(art.metadata_int("load_address", 0x400000), 0x400000);
    }

    // Test 4: metadata_int returns stored value
    #[test]
    fn metadata_int_stored_value() {
        let mut m = HashMap::new();
        m.insert("load_address".into(), MetadataValue::Int(0x600000));
        let art = CodeArtifact::new(vec![], 0, Target::linux_x64()).with_metadata(m);
        assert_eq!(art.metadata_int("load_address", 0), 0x600000);
    }

    // Test 5: metadata_int falls back when type mismatch
    #[test]
    fn metadata_int_type_mismatch_falls_back() {
        let mut m = HashMap::new();
        m.insert("key".into(), MetadataValue::Str("not a number".into()));
        let art = CodeArtifact::new(vec![], 0, Target::linux_x64()).with_metadata(m);
        assert_eq!(art.metadata_int("key", 99), 99);
    }

    // Test 6: metadata_str default when absent
    #[test]
    fn metadata_str_default_when_absent() {
        let art = CodeArtifact::new(vec![], 0, Target::linux_x64());
        assert_eq!(art.metadata_str("subsystem", "console"), "console");
    }

    // Test 7: metadata_str stored value
    #[test]
    fn metadata_str_stored_value() {
        let mut m = HashMap::new();
        m.insert("subsystem".into(), MetadataValue::Str("windows".into()));
        let art = CodeArtifact::new(vec![], 0, Target::windows_x64()).with_metadata(m);
        assert_eq!(art.metadata_str("subsystem", "console"), "windows");
    }

    // Test 8: metadata_list default when absent
    #[test]
    fn metadata_list_default_when_absent() {
        let art = CodeArtifact::new(vec![], 0, Target::wasm());
        assert!(art.metadata_list("exports").is_empty());
    }

    // Test 9: metadata_list stored value
    #[test]
    fn metadata_list_stored_value() {
        let mut m = HashMap::new();
        m.insert(
            "exports".into(),
            MetadataValue::List(vec!["main".into(), "add".into()]),
        );
        let art = CodeArtifact::new(vec![], 0, Target::wasm()).with_metadata(m);
        let list = art.metadata_list("exports");
        assert_eq!(list, vec!["main", "add"]);
    }

    // Test 10: with_metadata replaces all metadata
    #[test]
    fn with_metadata_replaces() {
        let mut m = HashMap::new();
        m.insert("origin".into(), MetadataValue::Int(0x1000));
        let art = CodeArtifact::new(vec![], 0, Target::intel_4004()).with_metadata(m);
        assert_eq!(art.metadata_int("origin", 0), 0x1000);
        assert_eq!(art.metadata_int("load_address", 42), 42); // absent
    }
}
