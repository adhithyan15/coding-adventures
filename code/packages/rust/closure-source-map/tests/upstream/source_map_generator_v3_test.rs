//! Ported from `SourceMapGeneratorV3Test.java` in
//! `google/closure-compiler`, Apache-2.0. Upstream SHA: see
//! `tests/upstream/UPSTREAM_SHA`.
//!
//! Translation policy: see
//! `code/specs/CLOC12-upstream-test-suite-port.md`.
//!
//! Fifth port under CLOC12 — first one targeting `closure-source-map`.
//! Most upstream `@Test` methods drive the full Closure-compiler
//! pipeline (`compileAndCheck`) and assert VLQ-encoded `mappings`
//! strings like `"A,aAAAA,QAASA,UAAS,EAAG;"`. Our crate is currently
//! v0.1.0 with no VLQ encoder — the builder accumulates raw
//! `(line, column, cv_id)` mappings and the finalized `mappings`
//! field is always the empty string pending Phase 2 v2 work.
//!
//! So the bulk of this file is `#[ignore = "blocked on gap-028"]`
//! placeholders. What we *can* assert today is the JSON-shape
//! contract: `version: 3`, `file` reflects `set_file`, `sourceRoot`
//! reflects `set_source_root`, `sources`/`names` are JSON arrays,
//! `mappings` is the empty string. Those passing tests pin the
//! shape so the future VLQ slice has a stable baseline to extend.

use coding_adventures_closure_source_map::SourceMapBuilder;
use coding_adventures_correlation_vector::CVLog;

fn cv() -> CVLog {
    CVLog::new(true)
}

/// Helper — build an empty SourceMap with no mappings and return its
/// JSON serialization for shape assertions. Mirrors what upstream's
/// `compileAndCheck` does at the very end (call `getSourceMap()` and
/// inspect its JSON), but skips the lex/parse/emit upstream uses to
/// produce the mappings string.
fn empty_map_json() -> String {
    SourceMapBuilder::new().build(&cv()).to_json()
}

// =====================================================================
// Ported tests
// =====================================================================

/// Upstream `testBasicMapping1`:
///
///   compileAndCheck("function __BASIC__() { }");
///
/// Asserts that compiling a one-function source produces a valid v3
/// map. Our crate isn't wired into compilation; this test belongs in a
/// closurec-level end-to-end port once the source-map sidecar emits
/// real VLQ mappings.
#[test]
#[ignore = "gap-028 encoder shipped (CLOC12.31); remaining blocker is full-pipeline compileAndCheck harness + upstream golden capture (CLOC14.1)"]
fn test_basic_mapping_1() {
    // Would compile `function __BASIC__() {}` and check the generated
    // mappings string matches upstream's golden output.
}

/// Upstream `testBasicMappingGoldenOutput`:
///
///   checkSourceMap("function __BASIC__() { }", TestJsonBuilder...
///       .setMappings("A,aAAAA,QAASA,UAAS,EAAG;").build());
///
/// Pin the exact VLQ string. Blocked on gap-028.
#[test]
#[ignore = "gap-028 encoder shipped (CLOC12.31); remaining blocker is closurec full-pipeline harness + upstream Closure golden VLQ capture (CLOC14.1)"]
fn test_basic_mapping_golden_output() {}

/// Upstream `testLiteralMappings`:
///
///   compileAndCheck("function __BASIC__(__PARAM1__, __PARAM2__) {
///     var __VAR__ = '__STR__'; }");
///
/// Same shape, more identifiers.
#[test]
#[ignore = "gap-028 encoder shipped (CLOC12.31); remaining blocker is closurec full-pipeline harness + upstream Closure golden VLQ capture (CLOC14.1)"]
fn test_literal_mappings() {}

/// Upstream `testLiteralMappingsGoldenOutput`:
///
///   checkSourceMap(...) with VLQ
///   `"A,aAAAA,QAASA,UAAS,CAACC,UAAD,CAAaC,UAAb,CAAyB,CAAE,IAAIC,QAAU,SAAhB;"`
#[test]
#[ignore = "gap-028 encoder shipped (CLOC12.31); remaining blocker is closurec full-pipeline harness + upstream Closure golden VLQ capture (CLOC14.1)"]
fn test_literal_mappings_golden_output() {}

/// Upstream `testMultilineMapping`:
///
///   compileAndCheck("function __BASIC__() {\n    var x = 1;\n}");
///
/// Multi-line case — VLQ deltas track line advances. Blocked on gap-028.
#[test]
#[ignore = "gap-028 encoder shipped (CLOC12.31); remaining blocker is closurec full-pipeline harness + upstream Closure golden VLQ capture (CLOC14.1)"]
fn test_multiline_mapping() {}

/// Upstream `testMultiFunctionMapping`:
///
///   compileAndCheck("function __BASIC__() {} function __OTHER__() {}");
#[test]
#[ignore = "gap-028 encoder shipped (CLOC12.31); remaining blocker is closurec full-pipeline harness + upstream Closure golden VLQ capture (CLOC14.1)"]
fn test_multi_function_mapping() {}

/// Upstream `testGoldenOutput0`:
///
///   checkSourceMap with a specific function source and expected JSON.
///   Tests the full JSON shape plus the VLQ mappings field.
#[test]
#[ignore = "gap-028 encoder shipped (CLOC12.31); remaining blocker is closurec full-pipeline harness + upstream Closure golden VLQ capture (CLOC14.1)"]
fn test_golden_output_0() {}

// =====================================================================
// Shape-only tests we CAN pass today.
//
// These pin the JSON shape produced by the empty builder, which is
// independent of VLQ encoding. They mirror the spirit of upstream's
// `TestJsonBuilder` golden-output tests at the "is the document
// well-formed v3" level. Each one corresponds to a specific
// `TestJsonBuilder.set*` setter — when VLQ encoding lands, these
// assertions still hold and the `mappings` field will gain content.
// =====================================================================

/// Upstream golden-output tests all start by asserting `version: 3`.
/// We pin that invariant here in isolation.
#[test]
fn empty_builder_emits_version_3() {
    let s = empty_map_json();
    let v: serde_json::Value = serde_json::from_str(&s).expect("valid JSON");
    assert_eq!(v["version"], 3, "expected version=3, got {}", s);
}

/// Upstream `TestJsonBuilder.setFile("testcode")` produces
/// `"file": "testcode"` in the JSON. Our builder defaults to empty
/// string, which is what the empty-map serialization produces.
#[test]
fn empty_builder_emits_empty_file_field() {
    let s = empty_map_json();
    let v: serde_json::Value = serde_json::from_str(&s).expect("valid JSON");
    assert_eq!(v["file"], "", "expected file=\"\", got {}", s);
}

/// `set_file` reflects in the JSON output.
#[test]
fn set_file_reflects_in_json() {
    let mut b = SourceMapBuilder::new();
    b.set_file("out.js".to_string());
    let s = b.build(&cv()).to_json();
    let v: serde_json::Value = serde_json::from_str(&s).expect("valid JSON");
    assert_eq!(v["file"], "out.js");
}

/// Upstream JSON uses `sourceRoot` (camelCase) as the key. Our serde
/// rename ensures the wire format matches.
#[test]
fn set_source_root_serializes_as_camelcase_sourceroot() {
    let mut b = SourceMapBuilder::new();
    b.set_source_root("/src/".to_string());
    let s = b.build(&cv()).to_json();
    let v: serde_json::Value = serde_json::from_str(&s).expect("valid JSON");
    assert_eq!(v["sourceRoot"], "/src/", "got {}", s);
}

/// `sources` is always a JSON array. Empty by default in v0.1.0.
#[test]
fn empty_builder_emits_sources_as_empty_array() {
    let s = empty_map_json();
    let v: serde_json::Value = serde_json::from_str(&s).expect("valid JSON");
    assert!(v["sources"].is_array(), "sources must be a JSON array; got {}", s);
    assert_eq!(v["sources"].as_array().unwrap().len(), 0);
}

/// `names` is always a JSON array. Empty by default in v0.1.0.
#[test]
fn empty_builder_emits_names_as_empty_array() {
    let s = empty_map_json();
    let v: serde_json::Value = serde_json::from_str(&s).expect("valid JSON");
    assert!(v["names"].is_array(), "names must be a JSON array; got {}", s);
    assert_eq!(v["names"].as_array().unwrap().len(), 0);
}

/// `mappings` is the empty string in v0.1.0 (VLQ encoding pending
/// gap-028). When that gap closes, this assertion will flip to a
/// VLQ-content check.
#[test]
fn empty_builder_emits_mappings_as_empty_string() {
    let s = empty_map_json();
    let v: serde_json::Value = serde_json::from_str(&s).expect("valid JSON");
    assert_eq!(v["mappings"], "", "expected empty mappings, got {}", s);
}

/// `raw_mapping_count` reflects accumulated `add_mapping` calls even
/// when the encoded `mappings` field is empty. Pins that we're at
/// least *tracking* mappings; encoding them is gap-028.
#[test]
fn add_mapping_accumulates_raw_count() {
    let mut b = SourceMapBuilder::new();
    assert_eq!(b.raw_mapping_count(), 0);
    b.add_mapping(0, 0, "cv.1");
    b.add_mapping(0, 5, "cv.2");
    b.add_mapping(1, 0, "cv.3");
    assert_eq!(b.raw_mapping_count(), 3);
}
