//! Source-map v3 generator for the Closure Compiler clone.
//!
//! Per [CLOC07](../../../specs/CLOC07-emit-and-source-map.md)
//! Phase 2. Companion to `closure-emitter`: receives per-token
//! mappings (generated `(line, column)` ↔ CV id) from the
//! emitter and produces a source-map v3 JSON blob the browser's
//! devtools can consume.
//!
//! # The shape of a source-map v3 blob
//!
//! ```json
//! {
//!   "version": 3,
//!   "file": "out.js",
//!   "sourceRoot": "",
//!   "sources": ["in.js"],
//!   "names": ["userName"],
//!   "mappings": "AAAA,SAASA,QAAQ;..."
//! }
//! ```
//!
//! - **`version`** is always `3`. v1 and v2 are obsolete.
//! - **`file`** is the name of the *generated* file (the
//!   browser shows this in the file picker).
//! - **`sourceRoot`** is prepended to each entry in `sources`.
//!   Useful for serving sources from a CDN without rewriting
//!   the map.
//! - **`sources`** is the list of *input* files.
//! - **`names`** is a dictionary of original identifier names —
//!   what the rename pass took away, so devtools can show them
//!   again.
//! - **`mappings`** is a VLQ-encoded string of generated-to-
//!   original position triples (or quintuples when a name is
//!   involved). The most-decoded piece, but it's a separate
//!   layer — v1 emits the empty string.
//!
//! # Why a separate crate from the emitter?
//!
//! 1. **Different audiences.** The emitter cares about valid
//!    JavaScript bytes; this crate cares about a precisely
//!    specified JSON wire format. Different libraries to debug,
//!    different test fixtures.
//! 2. **Different reuse story.** Anything that emits source
//!    text + position metadata can use this builder — not just
//!    the JS emitter. Future Lispy / Prolog backends in the
//!    same monorepo will produce maps with the same crate.
//! 3. **Pure data transform.** No AST, no sidecar, no CV log
//!    mutation — just (line, col, cv_id) entries in, JSON
//!    blob out. Independent dependency graph from the rest of
//!    the closure compiler stack.
//!
//! # Why does the CV id play the role usually played by
//! original-source positions?
//!
//! Per CLOC02, the AST doesn't carry source ranges — it carries
//! CV ids. The CV graph maps each id back to the bytes it
//! traces to (possibly across multiple optimization passes).
//! Storing the CV id in the source map's intermediate form
//! lets us defer the lookup until we serialize. Each `cv_id`
//! gets converted to a `(source_file, line, column)` triple
//! at `build()` time — the conversion happens by walking the
//! CV graph and producing the index lists `sources` / `names`
//! need.
//!
//! That conversion is the actual VLQ encoder. v1 doesn't run
//! it (no mappings collected anywhere yet), so the body is
//! a placeholder that produces a valid empty v3 blob.
//!
//! # Scope (v1)
//!
//! - [`SourceMapBuilder`] accepts `add_mapping` calls and
//!   stores them, but [`SourceMapBuilder::build`] doesn't
//!   actually encode them yet — the v1 build always produces
//!   an empty `mappings` string. Counts of accumulated raw
//!   mappings are visible via [`SourceMapBuilder::raw_mapping_count`]
//!   for testing.
//! - The output JSON validates against source-map v3 *shape*:
//!   correct keys, correct types, `version = 3`. Browsers will
//!   load it without complaint; they'll just have no mappings
//!   to display.
//! - Real VLQ encoding of the `(line, col, cv_id)` stream
//!   lands in v2 alongside a `SourceMapBuilder::file` /
//!   `source_root` getter pair and the integration with the
//!   future non-identity `closure-emitter`.

mod vlq;
// Re-export so downstream tests / debug tooling can verify their
// expectations against the canonical encoder, without spelling the
// `closure_source_map::vlq::...` path. The full VLQ-encoded
// `mappings` field still belongs to the builder; these are the
// encoding primitives only.
pub use vlq::{encode_vlq_int, encode_vlq_segment};

use coding_adventures_correlation_vector::CVLog;
use serde::Serialize;

/// One pending mapping entry. Stored in [`SourceMapBuilder`]
/// until [`build`] resolves them via the CV graph into
/// `(source_index, original_line, original_column, name_index)`
/// quadruples and VLQ-encodes the result.
///
/// [`build`]: SourceMapBuilder::build
#[derive(Debug, Clone, PartialEq, Eq)]
struct PendingMapping {
    /// 0-based line in the generated output.
    generated_line: u32,
    /// 0-based column in the generated output (UTF-16 code units
    /// per the v3 spec).
    generated_column: u32,
    /// CV id pointing to the original source the generated
    /// token came from.
    cv_id: String,
}

/// Builder for a source-map v3 blob.
///
/// The emitter feeds mappings in via [`Self::add_mapping`] as
/// it walks the AST and emits tokens. When emit completes, call
/// [`Self::build`] to get a [`SourceMap`] that knows how to
/// serialize to JSON.
///
/// Builders are reusable: build once, throw the builder away.
/// They don't cache state across multiple `build()` calls in
/// v1 (the future VLQ encoder may want to memoize).
#[derive(Debug, Clone, Default)]
pub struct SourceMapBuilder {
    /// Name of the generated file. Defaults to empty.
    file: String,
    /// Prefix prepended to each `sources` entry.
    source_root: String,
    /// Raw (line, column, cv_id) entries, in the order added.
    mappings: Vec<PendingMapping>,
}

impl SourceMapBuilder {
    /// Create an empty builder with default `file`,
    /// `source_root`, and no mappings.
    pub fn new() -> Self {
        Self::default()
    }

    /// Set the `file` field — the name of the generated file
    /// the source map describes. Shown to users in devtools.
    pub fn set_file(&mut self, name: String) -> &mut Self {
        self.file = name;
        self
    }

    /// Set the `sourceRoot` field — prepended to each `sources`
    /// entry. Useful for serving sources from a CDN without
    /// rewriting the map.
    pub fn set_source_root(&mut self, root: String) -> &mut Self {
        self.source_root = root;
        self
    }

    /// Record that the token at generated `(line, column)`
    /// originated from the CV node `cv_id`. The emitter calls
    /// this once per emitted token (well, once per token whose
    /// origin is traceable; tokens the emitter synthesizes
    /// from nothing don't get mappings).
    ///
    /// Coordinates are 0-based per the source-map v3 spec.
    pub fn add_mapping(
        &mut self,
        generated_line: u32,
        generated_column: u32,
        cv_id: &str,
    ) -> &mut Self {
        self.mappings.push(PendingMapping {
            generated_line,
            generated_column,
            cv_id: cv_id.to_string(),
        });
        self
    }

    /// Count of raw (pre-encoding) mappings accumulated.
    /// Exposed for tests and debug tooling; the public output
    /// of the builder is [`SourceMap`].
    pub fn raw_mapping_count(&self) -> usize {
        self.mappings.len()
    }

    /// Finalize the builder into a [`SourceMap`] ready to
    /// serialize.
    ///
    /// `_cv` is reserved for v2: the VLQ encoder needs the CV
    /// graph to translate each pending `cv_id` into the
    /// `(source_index, original_line, original_column)` triple
    /// the v3 mappings field encodes. v1 doesn't run the
    /// encoder, so `_cv` is unused — but the parameter is in
    /// the signature so v2 doesn't break callers.
    pub fn build(self, _cv: &CVLog) -> SourceMap {
        // v1: produce a valid empty v3 blob. Real VLQ encoding
        // of self.mappings lands in v2.
        SourceMap {
            version: 3,
            file: self.file,
            source_root: self.source_root,
            sources: Vec::new(),
            names: Vec::new(),
            mappings: String::new(),
        }
    }
}

/// A finalized source-map v3 blob.
///
/// Fields match the v3 spec key names exactly when serialized
/// (camelCase for `sourceRoot`). Use [`Self::to_json`] to get
/// the wire string the browser consumes.
#[derive(Debug, Clone, Serialize)]
pub struct SourceMap {
    /// Always `3`. v1 and v2 are obsolete.
    pub version: u32,

    /// Name of the generated file. Empty string means
    /// "unspecified."
    pub file: String,

    /// Prefix prepended to each `sources` entry.
    #[serde(rename = "sourceRoot")]
    pub source_root: String,

    /// Original input file names.
    pub sources: Vec<String>,

    /// Original identifier names recoverable for devtools.
    pub names: Vec<String>,

    /// VLQ-encoded mapping string. Empty in v1 (real encoding
    /// lands in v2).
    pub mappings: String,
}

impl SourceMap {
    /// Serialize to a compact source-map v3 JSON blob. The
    /// browser's devtools consume this verbatim.
    ///
    /// Returns a `String` rather than writing to a path: the
    /// emitter is in charge of deciding where the blob lives
    /// (inline `//# sourceMappingURL=data:...` vs. a sibling
    /// `.js.map` file).
    pub fn to_json(&self) -> String {
        // `Serialize` is total over `SourceMap` so this can't
        // fail in practice; serde_json::to_string only fails
        // on serializer-level issues (custom Serialize impls
        // that return errors), and we don't have any.
        serde_json::to_string(self).expect("SourceMap is total over serde_json::Serialize")
    }
}

#[cfg(test)]
mod tests {
    //! Tests pin the public surface: builder defaults, fluent
    //! setter chaining, mapping accumulation, the shape of the
    //! finalized JSON.
    use super::*;
    use coding_adventures_correlation_vector::CVLog;

    fn cv() -> CVLog {
        CVLog::new(true)
    }

    #[test]
    fn builder_new_is_empty() {
        let b = SourceMapBuilder::new();
        assert_eq!(b.raw_mapping_count(), 0);
        let m = b.build(&cv());
        assert_eq!(m.version, 3);
        assert_eq!(m.file, "");
        assert_eq!(m.source_root, "");
        assert!(m.sources.is_empty());
        assert!(m.names.is_empty());
        assert_eq!(m.mappings, "");
    }

    #[test]
    fn builder_default_equals_new() {
        // Both paths should produce identical state.
        let a = SourceMapBuilder::new();
        let b = SourceMapBuilder::default();
        assert_eq!(a.raw_mapping_count(), b.raw_mapping_count());
        let ja = a.build(&cv()).to_json();
        let jb = b.build(&cv()).to_json();
        assert_eq!(ja, jb);
    }

    #[test]
    fn add_mapping_accumulates() {
        let mut b = SourceMapBuilder::new();
        b.add_mapping(0, 0, "node.1");
        b.add_mapping(0, 5, "node.2");
        b.add_mapping(1, 0, "node.3");
        assert_eq!(b.raw_mapping_count(), 3);
        // Mappings are stored in insertion order so the v2
        // encoder can rely on it; v1 doesn't read them out so
        // we just verify the count and that build() still
        // produces a valid empty mappings string.
        let m = b.build(&cv());
        assert_eq!(m.mappings, "");
    }

    #[test]
    fn set_file_updates_output() {
        let mut b = SourceMapBuilder::new();
        b.set_file("out.js".to_string());
        let m = b.build(&cv());
        assert_eq!(m.file, "out.js");
    }

    #[test]
    fn set_source_root_updates_output() {
        let mut b = SourceMapBuilder::new();
        b.set_source_root("https://cdn.example.com/src/".to_string());
        let m = b.build(&cv());
        assert_eq!(m.source_root, "https://cdn.example.com/src/");
    }

    #[test]
    fn fluent_chaining() {
        // The `&mut Self` returns let callers chain. CLI code
        // benefits.
        let mut b = SourceMapBuilder::new();
        b.set_file("out.js".to_string())
            .set_source_root("/src/".to_string())
            .add_mapping(0, 0, "x")
            .add_mapping(0, 1, "y");
        assert_eq!(b.raw_mapping_count(), 2);
        let m = b.build(&cv());
        assert_eq!(m.file, "out.js");
        assert_eq!(m.source_root, "/src/");
    }

    #[test]
    fn to_json_emits_v3_shape() {
        // Validate that to_json produces a JSON object with
        // exactly the v3-spec keys, and that `sourceRoot` is
        // camelCase (not `source_root`).
        let b = SourceMapBuilder::new();
        let m = b.build(&cv());
        let j: serde_json::Value = serde_json::from_str(&m.to_json())
            .expect("to_json output must be valid JSON");
        assert_eq!(j["version"], 3);
        assert_eq!(j["file"], "");
        assert_eq!(j["sourceRoot"], ""); // camelCase, not source_root
        assert!(j["sources"].is_array() && j["sources"].as_array().unwrap().is_empty());
        assert!(j["names"].is_array() && j["names"].as_array().unwrap().is_empty());
        assert_eq!(j["mappings"], "");
        // No unexpected keys (lock the shape).
        let obj = j.as_object().expect("top-level must be an object");
        let keys: std::collections::HashSet<&str> =
            obj.keys().map(String::as_str).collect();
        let expected: std::collections::HashSet<&str> =
            ["version", "file", "sourceRoot", "sources", "names", "mappings"]
                .iter()
                .copied()
                .collect();
        assert_eq!(keys, expected, "unexpected v3 keys: {:?}", keys);
    }

    #[test]
    fn to_json_round_trips_set_values() {
        let mut b = SourceMapBuilder::new();
        b.set_file("output.js".to_string())
            .set_source_root("/root/".to_string());
        let m = b.build(&cv());
        let j: serde_json::Value =
            serde_json::from_str(&m.to_json()).expect("valid JSON");
        assert_eq!(j["file"], "output.js");
        assert_eq!(j["sourceRoot"], "/root/");
    }

    #[test]
    fn source_map_is_clone_and_debug() {
        let b = SourceMapBuilder::new();
        let m = b.build(&cv());
        let m2 = m.clone();
        let _dbg = format!("{:?}", m2);
        assert_eq!(m.version, m2.version);
    }

    #[test]
    fn builder_is_clone_default_debug() {
        let mut a = SourceMapBuilder::new();
        a.add_mapping(0, 0, "x");
        let b = a.clone();
        assert_eq!(a.raw_mapping_count(), b.raw_mapping_count());
        let _dbg = format!("{:?}", b);
        // Default goes through the same code path as ::new().
        let c: SourceMapBuilder = Default::default();
        assert_eq!(c.raw_mapping_count(), 0);
    }
}
