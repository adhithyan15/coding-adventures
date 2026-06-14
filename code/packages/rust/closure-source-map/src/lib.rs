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
//! # Scope (current)
//!
//! - [`SourceMapBuilder`] accepts `add_mapping` calls and
//!   stores them; [`SourceMapBuilder::build`] now actually
//!   encodes the accumulated `(generated_line, generated_column,
//!   cv_id)` stream as a base64-VLQ `mappings` string per the
//!   source-map v3 spec — resolving each `cv_id` to a
//!   `(source_index, original_line, original_column)` triple
//!   via the supplied [`CVLog`]. Counts of accumulated raw
//!   mappings remain visible via [`SourceMapBuilder::raw_mapping_count`].
//! - The output JSON validates against source-map v3 *shape*:
//!   correct keys, correct types, `version = 3`, and now a
//!   real `mappings` field that devtools can step through.
//! - The `sources` array is populated in first-seen order
//!   from each mapping's resolved `Origin.source`. Identifier
//!   names (`names` array) are still empty — `name_index`
//!   (the 5th VLQ-segment field) needs a per-mapping name hint
//!   that the emitter doesn't currently surface, so all encoded
//!   segments are the 4-field shape.
//! - Mappings whose `cv_id` doesn't resolve to an `Origin`
//!   (CV graph is empty, location string isn't parseable as
//!   `line:col`, etc.) emit the 1-field segment shape:
//!   `[generated_column_delta]` only. Devtools treat these as
//!   "no original location" — correct for tokens the emitter
//!   synthesizes from nothing.
//!
//! # Mapping-resolution policy
//!
//! 1. Look up `cv_id` in the supplied [`CVLog`].
//! 2. If the entry has an `Origin`, use it directly.
//! 3. Otherwise walk the first `parent_ids` chain looking for
//!    an ancestor with an `Origin`. This handles the common
//!    "derive then contribute" pattern where the entry the
//!    emitter knows about is a child of the entry that carries
//!    the source location.
//! 4. Parse `Origin.location` as `"line:column"`. Both halves
//!    are decimal `u32`s. Anything else (free-form strings,
//!    `"row_id:8472"`-style locations from non-source-position
//!    consumers) falls through to the 1-field segment.
//!
//! Origins use whatever line/column convention their producer
//! used. The source-map v3 spec wants 0-based original
//! coordinates; producers feeding mappings into this builder
//! are responsible for matching that convention. We don't
//! adjust the numbers — pass them through verbatim.

mod vlq;
// Re-export so downstream tests / debug tooling can verify their
// expectations against the canonical encoder, without spelling the
// `closure_source_map::vlq::...` path. The full VLQ-encoded
// `mappings` field still belongs to the builder; these are the
// encoding primitives only.
pub use vlq::{encode_vlq_int, encode_vlq_segment};

use coding_adventures_correlation_vector::{CVLog, Origin};
use serde::Serialize;
use std::collections::HashMap;

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
    /// Resolves each accumulated `(generated_line,
    /// generated_column, cv_id)` triple against `cv`, building
    /// up the `sources` list in first-seen order and emitting
    /// the source-map v3 `mappings` string per the encoding
    /// rules described in the module-level docs.
    pub fn build(self, cv: &CVLog) -> SourceMap {
        let SourceMapBuilder {
            file,
            source_root,
            mappings: pending,
        } = self;

        // Fast path: no mappings → empty v3 blob. Matches the
        // pre-encoder behaviour for callers that build a map
        // they don't fill in.
        if pending.is_empty() {
            return SourceMap {
                version: 3,
                file,
                source_root,
                sources: Vec::new(),
                names: Vec::new(),
                mappings: String::new(),
            };
        }

        // -----------------------------------------------------
        // Step 1: resolve each pending mapping's `cv_id` to a
        // `(source_index, orig_line, orig_col)` triple via the
        // CV graph. Unresolvable mappings get `None` and emit
        // the 1-field segment shape downstream.
        // -----------------------------------------------------
        let mut sources: Vec<String> = Vec::new();
        let mut source_index_of: HashMap<String, u32> = HashMap::new();

        let mut resolved: Vec<ResolvedMapping> = Vec::with_capacity(pending.len());
        for m in &pending {
            let origin = resolve_origin(cv, &m.cv_id);
            let triple = origin.and_then(|o| {
                let (line, col) = parse_line_col(&o.location)?;
                let idx = *source_index_of.entry(o.source.clone()).or_insert_with(|| {
                    let i = sources.len() as u32;
                    sources.push(o.source.clone());
                    i
                });
                Some((idx, line, col))
            });
            resolved.push(ResolvedMapping {
                gen_line: m.generated_line,
                gen_col: m.generated_column,
                origin: triple,
            });
        }

        // -----------------------------------------------------
        // Step 2: sort by (generated_line, generated_column).
        // The emitter usually feeds in order, but a defensive
        // sort keeps the encoder honest if anyone batches or
        // reorders.
        // -----------------------------------------------------
        resolved.sort_by_key(|r| (r.gen_line, r.gen_col));

        // -----------------------------------------------------
        // Step 3: VLQ-encode. Format per the v3 spec:
        //
        //     mappings ::= line ( ';' line )*
        //     line     ::= ( segment ( ',' segment )* )?
        //     segment  ::= vlq+   (1 / 4 / 5 fields)
        //
        // Lines beyond the last mapped line don't appear; lines
        // *between* the first and last mapped line do appear,
        // possibly empty (just a `;` separator). The
        // generated_column delta resets to 0 at the start of
        // each line; the other deltas (source_index,
        // original_line, original_column) carry across lines.
        // -----------------------------------------------------
        let mut out = String::new();
        let mut prev_source_idx: i32 = 0;
        let mut prev_orig_line: i32 = 0;
        let mut prev_orig_col: i32 = 0;
        let mut prev_line: u32 = 0;
        let mut prev_gen_col: i32 = 0;

        for (i, r) in resolved.iter().enumerate() {
            // Fill in `;` separators for every line we skipped
            // since the previous mapping. Each `;` ends a line
            // and starts a fresh column-delta context (gen_col
            // resets to 0 inside each line).
            if i == 0 {
                // Lead-in: lines [0, first_mapping.gen_line)
                // each contribute one `;` before we write any
                // segment on the first mapped line.
                for _ in 0..r.gen_line {
                    out.push(';');
                }
            } else {
                let line_gap = r.gen_line - prev_line;
                for _ in 0..line_gap {
                    out.push(';');
                    prev_gen_col = 0;
                }
                if line_gap == 0 {
                    // Same line as previous segment — separate
                    // with `,`.
                    out.push(',');
                }
            }

            // Emit one segment.
            //
            // `as i32` is safe for any realistic source-map
            // input: generated columns are file offsets in
            // characters, and we don't run on >2 GiB outputs.
            // A column ≥ 2^31 would cast to a negative i32 and
            // the subsequent subtraction could overflow in
            // debug builds — pathological-input-only, not a
            // security concern.
            let gen_col = r.gen_col as i32;
            let gen_col_delta = gen_col - prev_gen_col;
            prev_gen_col = gen_col;

            if let Some((src_idx, orig_line, orig_col)) = r.origin {
                let src_idx = src_idx as i32;
                let orig_line = orig_line as i32;
                let orig_col = orig_col as i32;
                let segment = [
                    gen_col_delta,
                    src_idx - prev_source_idx,
                    orig_line - prev_orig_line,
                    orig_col - prev_orig_col,
                ];
                out.push_str(&encode_vlq_segment(&segment));
                prev_source_idx = src_idx;
                prev_orig_line = orig_line;
                prev_orig_col = orig_col;
            } else {
                // Unresolved — 1-field segment.
                out.push_str(&encode_vlq_segment(&[gen_col_delta]));
            }

            prev_line = r.gen_line;
        }

        SourceMap {
            version: 3,
            file,
            source_root,
            sources,
            names: Vec::new(),
            mappings: out,
        }
    }
}

// =====================================================================
// Resolution helpers
// =====================================================================

/// A `PendingMapping` after CV resolution. `origin` is `None`
/// when the `cv_id` couldn't be resolved to a parseable
/// (source, line, col) — e.g. synthetic CV with no upstream
/// position, or a `location` string that isn't `line:col`.
#[derive(Debug, Clone, Copy)]
struct ResolvedMapping {
    gen_line: u32,
    gen_col: u32,
    origin: Option<(u32, u32, u32)>,
}

/// Walk the CV graph from `cv_id` looking for an `Origin`.
///
/// Strategy:
/// 1. If `cv_id` has its own `Origin`, return it.
/// 2. Else walk parents in `parent_ids` order, depth-first,
///    returning the first `Origin` encountered.
/// 3. Cycle guard: track visited ids; bail if we revisit. CV
///    IDs are supposed to be a DAG but a defensive guard keeps
///    `build()` from infinite-looping on malformed input.
fn resolve_origin<'a>(cv: &'a CVLog, cv_id: &str) -> Option<&'a Origin> {
    use std::collections::HashSet;
    let mut stack: Vec<String> = vec![cv_id.to_string()];
    let mut seen: HashSet<String> = HashSet::new();
    while let Some(id) = stack.pop() {
        if !seen.insert(id.clone()) {
            continue;
        }
        let Some(entry) = cv.get(&id) else { continue };
        if let Some(o) = &entry.origin {
            return Some(o);
        }
        // Push parents in reverse so the first parent gets
        // popped first (DFS left-to-right).
        for parent in entry.parent_ids.iter().rev() {
            if !seen.contains(parent) {
                stack.push(parent.clone());
            }
        }
    }
    None
}

/// Parse an `Origin.location` string as `"line:column"` (both
/// decimal `u32`s). Anything else returns `None`.
///
/// The CV spec leaves `location` free-form — `"5:12"`,
/// `"row_id:8472"`, `"byte:4096"` are all valid. The
/// source-map encoder only knows what to do with the
/// `"line:col"` shape, so non-matching strings fall through
/// to the 1-field segment ("no original position").
fn parse_line_col(loc: &str) -> Option<(u32, u32)> {
    let (l, c) = loc.split_once(':')?;
    let line: u32 = l.parse().ok()?;
    let col: u32 = c.parse().ok()?;
    Some((line, col))
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
        // None of these cv_ids are in the empty CVLog so the
        // encoder emits the 1-field segment shape (just the
        // generated_column delta) for each one. The expected
        // string:
        //
        //   line 0: [0, 5]  → segments "A" + "K"
        //   ';'   line break, gen_col_delta resets to 0
        //   line 1: [0]     → segment "A"
        //
        // VLQ digits: 0 → "A", 5 (sign-encoded 10) → "K".
        let m = b.build(&cv());
        assert_eq!(m.mappings, "A,K;A");
        // No origins resolved, so `sources` stays empty.
        assert!(m.sources.is_empty());
        assert!(m.names.is_empty());
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

    // =================================================================
    // Builder-VLQ integration tests (gap-028 step 2).
    //
    // Synthesize a small `CVLog` with `Origin`s the resolver can
    // parse, then drive the builder and pin the exact VLQ output.
    // The earlier `add_mapping_accumulates` test pins the no-origin
    // path; these pin the with-origin path and the delta-encoding
    // arithmetic across multiple sources / lines / columns.
    // =================================================================

    use coding_adventures_correlation_vector::Origin;

    /// Construct an `Origin` whose `location` is `"line:col"` so
    /// the resolver picks it up.
    fn origin(source: &str, line: u32, col: u32) -> Origin {
        Origin {
            source: source.to_string(),
            location: format!("{}:{}", line, col),
            timestamp: None,
            meta: Default::default(),
        }
    }

    #[test]
    fn build_with_single_resolved_mapping_emits_4_field_segment() {
        let mut log = CVLog::new(true);
        let id = log.create(Some(origin("in.js", 0, 0)));
        let mut b = SourceMapBuilder::new();
        b.add_mapping(0, 0, &id);
        let m = b.build(&log);
        // One mapping at (0,0) → "in.js":0:0. All deltas zero.
        // Segment = [0,0,0,0] → "AAAA".
        assert_eq!(m.mappings, "AAAA");
        assert_eq!(m.sources, vec!["in.js".to_string()]);
    }

    #[test]
    fn build_with_two_resolved_mappings_same_line_delta_encodes() {
        let mut log = CVLog::new(true);
        let a = log.create(Some(origin("in.js", 0, 0)));
        let b_id = log.create(Some(origin("in.js", 0, 5)));
        let mut b = SourceMapBuilder::new();
        b.add_mapping(0, 0, &a);
        b.add_mapping(0, 3, &b_id);
        let m = b.build(&log);
        // Seg1: [0,0,0,0] = "AAAA"
        // Seg2: gen_col_delta=3, src_idx_delta=0, orig_line_delta=0,
        //       orig_col_delta=5 → [3,0,0,5] → "G,A,A,K" concatenated.
        //
        // VLQ digits:
        //   3 → sign-encoded 6 → "G"
        //   0 → "A"
        //   0 → "A"
        //   5 → sign-encoded 10 → "K"
        //
        // Segments separated by `,`.
        assert_eq!(m.mappings, "AAAA,GAAK");
        assert_eq!(m.sources, vec!["in.js".to_string()]);
    }

    #[test]
    fn build_with_mappings_on_different_lines_uses_semicolon_separators() {
        let mut log = CVLog::new(true);
        let a = log.create(Some(origin("in.js", 0, 0)));
        let b_id = log.create(Some(origin("in.js", 1, 0)));
        let mut b = SourceMapBuilder::new();
        b.add_mapping(0, 0, &a);
        b.add_mapping(2, 0, &b_id);
        let m = b.build(&log);
        // Line 0: "AAAA" (zero deltas)
        // Lines 1 (empty), then line 2 segment.
        // gen_col resets to 0 each line.
        // Seg2: gen_col_delta=0, src_idx_delta=0, orig_line_delta=1,
        //       orig_col_delta=0 → [0,0,1,0] → "AACA".
        //   1 → sign-encoded 2 → "C"
        assert_eq!(m.mappings, "AAAA;;AACA");
    }

    #[test]
    fn build_with_two_sources_indexes_them_in_first_seen_order() {
        let mut log = CVLog::new(true);
        let a = log.create(Some(origin("first.js", 0, 0)));
        let b_id = log.create(Some(origin("second.js", 0, 0)));
        let mut b = SourceMapBuilder::new();
        b.add_mapping(0, 0, &a);
        b.add_mapping(0, 5, &b_id);
        let m = b.build(&log);
        assert_eq!(
            m.sources,
            vec!["first.js".to_string(), "second.js".to_string()]
        );
        // Seg1: [0,0,0,0] = "AAAA"
        // Seg2: gen_col_delta=5, src_idx_delta=1, orig_line_delta=0,
        //       orig_col_delta=0 → [5,1,0,0] → "K,C,A,A"
        assert_eq!(m.mappings, "AAAA,KCAA");
    }

    #[test]
    fn build_with_unresolvable_cv_emits_1_field_segment() {
        let log = CVLog::new(true);
        let mut b = SourceMapBuilder::new();
        b.add_mapping(0, 7, "missing.cv");
        let m = b.build(&log);
        // gen_col_delta=7 → sign-encoded 14 → "O".
        assert_eq!(m.mappings, "O");
        assert!(m.sources.is_empty());
    }

    #[test]
    fn build_mixes_resolved_and_unresolved_segments() {
        let mut log = CVLog::new(true);
        let a = log.create(Some(origin("in.js", 0, 0)));
        let mut b = SourceMapBuilder::new();
        b.add_mapping(0, 0, &a);
        b.add_mapping(0, 3, "no-such-cv");
        let m = b.build(&log);
        // Seg1: [0,0,0,0] = "AAAA"
        // Seg2 (unresolved): [3] → "G"
        assert_eq!(m.mappings, "AAAA,G");
        assert_eq!(m.sources, vec!["in.js".to_string()]);
    }

    #[test]
    fn build_walks_parents_to_find_origin() {
        // Derived CV inherits its origin via the parent chain.
        // resolve_origin walks parent_ids looking for an Origin.
        let mut log = CVLog::new(true);
        let root = log.create(Some(origin("in.js", 2, 4)));
        let child = log.derive(&root, None);
        let mut b = SourceMapBuilder::new();
        b.add_mapping(0, 0, &child);
        let m = b.build(&log);
        // origin found at parent: (in.js, 2, 4). All gen deltas 0.
        // Segment = [0,0,2,4] → "A","A","E","I".
        //   2 → 4 → "E"
        //   4 → 8 → "I"
        assert_eq!(m.mappings, "AAEI");
        assert_eq!(m.sources, vec!["in.js".to_string()]);
    }

    #[test]
    fn build_sorts_out_of_order_input() {
        // Defensive: feed mappings out of order and confirm
        // the encoder still produces a well-formed delta chain.
        let mut log = CVLog::new(true);
        let a = log.create(Some(origin("in.js", 0, 0)));
        let b_id = log.create(Some(origin("in.js", 0, 1)));
        let mut b = SourceMapBuilder::new();
        b.add_mapping(0, 3, &b_id); // intentionally first
        b.add_mapping(0, 0, &a);
        let m = b.build(&log);
        // Sorted: (0,0,a), (0,3,b). Same as the same-line test
        // shape; second origin is at col 1 not 5.
        // Seg1: [0,0,0,0] = "AAAA"
        // Seg2: [3,0,0,1] = "G","A","A","C"
        assert_eq!(m.mappings, "AAAA,GAAC");
    }

    #[test]
    fn build_skips_unparseable_location_string() {
        // CV Origin.location is free-form; if it's not "L:C" the
        // mapping falls through to the 1-field segment.
        let mut log = CVLog::new(true);
        let weird = Origin {
            source: "rows.csv".to_string(),
            location: "row_id:8472".to_string(), // parses as 0:8472? No — row_id isn't a number.
            timestamp: None,
            meta: Default::default(),
        };
        let id = log.create(Some(weird));
        let mut b = SourceMapBuilder::new();
        b.add_mapping(0, 4, &id);
        let m = b.build(&log);
        // gen_col_delta=4 → 8 → "I".
        assert_eq!(m.mappings, "I");
        assert!(m.sources.is_empty());
    }

    #[test]
    fn build_first_mapping_on_later_line_prefixes_with_semicolons() {
        // If no mappings exist for lines 0..N, the encoder
        // still has to emit N `;`s so devtools can line up
        // column 0 of line N with its segment.
        let mut log = CVLog::new(true);
        let id = log.create(Some(origin("in.js", 0, 0)));
        let mut b = SourceMapBuilder::new();
        b.add_mapping(3, 0, &id);
        let m = b.build(&log);
        // 3 leading `;`s, then [0,0,0,0] = "AAAA".
        assert_eq!(m.mappings, ";;;AAAA");
    }

    #[test]
    fn build_with_empty_mappings_returns_empty_string() {
        let log = CVLog::new(true);
        let b = SourceMapBuilder::new();
        let m = b.build(&log);
        assert_eq!(m.mappings, "");
        assert!(m.sources.is_empty());
    }

    #[test]
    fn build_ignores_self_referential_cycles() {
        // If parent_ids contains a cycle, resolve_origin's
        // visited-set guard keeps build() from looping. Verify
        // by manually constructing such an entry — the public
        // API can't (derive/merge always produce DAGs).
        let mut log = CVLog::new(true);
        let id = log.create(None);
        // Manually inject a self-parent on the entry.
        if let Some(entry) = log.entries.get_mut(&id) {
            entry.parent_ids.push(id.clone());
        }
        let mut b = SourceMapBuilder::new();
        b.add_mapping(0, 0, &id);
        // No Origin found anywhere in the cycle → 1-field segment.
        let m = b.build(&log);
        assert_eq!(m.mappings, "A");
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
