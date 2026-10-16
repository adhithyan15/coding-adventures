//! D3a tests precede the private shape gate. Generic import is an independent
//! prerequisite in every mutation cell; compile failure is never semantic red.
use super::*;
use crate::run::source_inventory::{record_file, SourceInventoryBudget, SourceInventoryLimits};
use coding_adventures_correlation_vector::{GraphLimits, Origin};
use serde_json::json;
use std::collections::HashMap;
use std::path::Path;

fn fixture(sources: &[&str], edition: EsVersion) -> (CVLog, Vec<String>) {
    let mut log = CVLog::new_compact(true);
    let mut budget = SourceInventoryBudget::new(SourceInventoryLimits::default()).unwrap();
    let roots = sources
        .iter()
        .enumerate()
        .map(|(i, source)| {
            record_file(
                &mut log,
                Path::new(&format!("input-{i}.js")),
                source,
                edition,
                &mut budget,
            )
            .unwrap()
            .id
        })
        .collect();
    (log, roots)
}
fn wire(log: &CVLog) -> Value {
    serde_json::from_str(&log.to_json_string().unwrap()).unwrap()
}
fn checked(value: &Value) -> CVLog {
    // These mutations are already Values because duplicates are NOT this test's
    // subject. Original-raw duplicate controls live in cv_source_shape_baseline.
    CVLog::from_checked_json(
        &serde_json::to_string(value).unwrap(),
        GraphLimits::default(),
    )
    .unwrap()
}
fn ids(value: &Value, root: &str) -> Vec<String> {
    value["entries"][root]["contributions"][0]["meta"][NS]["piece_ids"]
        .as_array()
        .unwrap()
        .iter()
        .map(|v| v.as_str().unwrap().to_owned())
        .collect()
}
fn refused(value: &Value, case: &str, expected: &str) {
    let log = checked(value);
    let before = log.to_json_string().unwrap();
    let error = validate_shape(&log, ShapeLimits::default()).expect_err(case);
    assert!(error.contains(expected), "{case}: {error} lacks {expected}");
    assert_eq!(
        log.to_json_string().unwrap(),
        before,
        "read-only refusal: {case}"
    );
    println!("D3A_SHAPE_REFUSAL {case}");
}

#[test]
fn d3a_actual_producer_shapes_all_editions_and_legacy_coordinate_domains() {
    for &edition in EsVersion::ALL {
        let (log, _) = fixture(
            &[
                "",
                "/*x*/",
                "1; /* gap */\r\n2;",
                "1;\r2;",
                "\"😀\\n\";",
                "\0",
            ],
            edition,
        );
        let before = log.to_json_string().unwrap();
        validate_shape(&checked(&wire(&log)), ShapeLimits::default()).unwrap();
        assert_eq!(log.to_json_string().unwrap(), before);
    }
    // This is explicitly shape-only: a self-consistent declared digest is not
    // bound to a trusted source buffer until D3b replay is implemented.
    let (log, roots) = fixture(&[""], EsVersion::Es2025);
    let mut value = wire(&log);
    value["entries"][&roots[0]]["origin"]["meta"][NS]["sha256"] = json!("a".repeat(64));
    value["entries"][&roots[0]]["origin"]["meta"]["content_sha256"] = json!("a".repeat(64));
    validate_shape(&checked(&value), ShapeLimits::default()).unwrap();
}

#[test]
fn d3a_closed_input_fields_literals_numbers_and_legacy_agreement() {
    let (log, roots) = fixture(&["1;"], EsVersion::Es2025);
    for (case, field, replacement) in [
        ("unknown-field", "unknown", json!(true)),
        ("schema", "schema", json!("2")),
        ("role", "role", json!("piece")),
        ("domain", "buffer_domain", json!("caller_guess")),
        ("encoding", "encoding", json!("utf16")),
        ("grammar", "grammar", json!("other")),
        ("edition", "edition", json!("es2099")),
        ("hash", "sha256", json!("A".repeat(64))),
        ("zero-padding", "byte_length", json!("02")),
        ("negative", "byte_length", json!("-1")),
        ("numeric-type", "byte_length", json!(2)),
        ("overflow", "byte_length", json!("18446744073709551616")),
        ("long-decimal", "byte_length", json!("1".repeat(21))),
    ] {
        let mut value = wire(&log);
        value["entries"][&roots[0]]["origin"]["meta"][NS][field] = replacement;
        refused(&value, case, "source shape");
    }
    for case in [
        "missing-key",
        "missing-namespace",
        "timestamp",
        "legacy-length",
        "legacy-hash",
        "wrong-origin",
    ] {
        let mut value = wire(&log);
        let origin = &mut value["entries"][&roots[0]]["origin"];
        match case {
            "missing-key" => {
                origin["meta"][NS]
                    .as_object_mut()
                    .unwrap()
                    .remove("encoding");
            }
            "missing-namespace" => {
                origin["meta"].as_object_mut().unwrap().remove(NS);
            }
            "timestamp" => origin["timestamp"] = json!("forged"),
            "legacy-length" => origin["meta"]["byte_len"] = json!(3),
            "legacy-hash" => origin["meta"]["content_sha256"] = json!("b".repeat(64)),
            "wrong-origin" => origin["source"] = json!("other"),
            _ => unreachable!(),
        }
        refused(&value, case, "source shape");
    }
}

#[test]
fn d3a_inventory_placement_status_counts_and_census() {
    let (log, roots) = fixture(&["1;", "2;"], EsVersion::Es2025);
    for case in [
        "missing-inventory",
        "missing-namespace",
        "wrong-tag",
        "unknown-key",
        "unknown-status",
        "wrong-count",
        "wrong-token-count",
        "omitted-piece",
        "duplicate-piece",
        "cross-root-piece",
        "unavailable-child",
        "wrong-owner",
        "reserved-contribution",
        "reserved-deletion",
        "namespace-free-child",
    ] {
        let mut value = wire(&log);
        let pieces = ids(&value, &roots[0]);
        let other = ids(&value, &roots[1]);
        let entries = value["entries"].as_object_mut().unwrap();
        let root = &mut entries[&roots[0]];
        let inv = &mut root["contributions"][0]["meta"][NS];
        match case {
            "missing-inventory" => root["contributions"] = json!([]),
            "missing-namespace" => root["contributions"][0]["meta"] = json!({}),
            "wrong-tag" => root["contributions"][0]["tag"] = json!("other"),
            "unknown-key" => inv["extra"] = json!(true),
            "unknown-status" => inv["status"] = json!("caller_guess"),
            "wrong-count" => inv["piece_count"] = json!("99"),
            "wrong-token-count" => inv["token_count"] = json!("99"),
            "omitted-piece" => {
                inv["piece_ids"].as_array_mut().unwrap().pop();
                inv["piece_count"] = json!("2");
            }
            "duplicate-piece" => inv["piece_ids"][1] = json!(pieces[0]),
            "cross-root-piece" => inv["piece_ids"][0] = json!(other[0]),
            "unavailable-child" => {
                *inv = json!({"schema":"1","role":"inventory","status":"unavailable","reason":"lexical_error","piece_count":"0","token_count":"0","piece_ids":[]})
            }
            "wrong-owner" => {
                let contribution = root["contributions"][0].clone();
                root["contributions"] = json!([]);
                entries[&pieces[0]]["contributions"] = json!([contribution]);
            }
            "reserved-contribution" => {
                root["contributions"][0]["source"] = json!("other");
            }
            "reserved-deletion" => {
                entries[&pieces[0]]["deleted"] =
                    json!({"source":"other","reason":"gone","meta":{NS:{"role":"inventory"}}});
            }
            "namespace-free-child" => {
                entries[&pieces[0]]["origin"]["meta"]
                    .as_object_mut()
                    .unwrap()
                    .remove(NS);
            }
            _ => unreachable!(),
        }
        // Retain the exact generic event-source census. Removing one root's
        // inventory does not remove the other root's stage declaration.
        let mut stages = std::collections::BTreeSet::new();
        for entry in value["entries"].as_object().unwrap().values() {
            for event in entry["contributions"].as_array().unwrap() {
                stages.insert(event["source"].as_str().unwrap().to_owned());
            }
            if let Some(source) = entry["deleted"]["source"].as_str() {
                stages.insert(source.to_owned());
            }
        }
        value["pass_order"] = json!(stages);
        refused(&value, case, "source shape");
    }
    let (log, roots) = fixture(&["\0"], EsVersion::Es2025);
    for (field, replacement) in [
        ("reason", json!("resource_limit")),
        ("piece_count", json!("1")),
        ("token_count", json!("1")),
        ("extra", json!(false)),
    ] {
        let mut value = wire(&log);
        value["entries"][&roots[0]]["contributions"][0]["meta"][NS][field] = replacement;
        refused(&value, field, "source shape");
    }
}

#[test]
fn d3a_piece_ordinals_parents_eof_coordinates_and_legacy_fields() {
    let (log, roots) = fixture(&["1; /*x*/\r\n2;", "3;"], EsVersion::Es2025);
    for case in [
        "piece-key",
        "piece-schema",
        "piece-role",
        "piece-input",
        "piece-ordinal",
        "token-ordinal",
        "token-type",
        "missing-token-field",
        "extra-trivia-field",
        "legacy-kind",
        "legacy-index",
        "legacy-length",
        "legacy-location",
        "wrong-piece-source",
        "parents",
        "position-key",
        "position-number",
        "nonzero-start",
        "gap",
        "backward",
        "zero-token",
        "early-eof",
        "wide-eof",
        "final-length",
    ] {
        let mut value = wire(&log);
        let pieces = ids(&value, &roots[0]);
        let trivia = pieces
            .iter()
            .find(|id| value["entries"][*id]["origin"]["source"] == "lexer_trivia")
            .unwrap()
            .clone();
        let first = &pieces[0];
        let last = pieces.last().unwrap();
        let later_piece = ids(&value, &roots[1])[0].clone();
        let entries = value["entries"].as_object_mut().unwrap();
        let origin = &mut entries[first]["origin"];
        let piece = &mut origin["meta"][NS];
        match case {
            "piece-key" => piece["extra"] = json!(true),
            "piece-schema" => piece["schema"] = json!("0"),
            "piece-role" => piece["role"] = json!("input"),
            "piece-input" => piece["input_id"] = json!(roots[1]),
            "piece-ordinal" => piece["piece_index"] = json!("1"),
            "token-ordinal" => piece["token_index"] = json!("1"),
            "token-type" => piece["token_type"] = json!("Arbitrary"),
            "missing-token-field" => {
                piece.as_object_mut().unwrap().remove("token_index");
            }
            "extra-trivia-field" => {
                entries[&trivia]["origin"]["meta"][NS]["token_index"] = json!("0")
            }
            "legacy-kind" => origin["meta"]["kind"] = json!("name"),
            "legacy-index" => origin["meta"]["token_index"] = json!(99),
            "legacy-length" => origin["meta"]["lexeme_byte_len"] = json!("1"),
            "legacy-location" => origin["location"] = json!("input-0.js:0:1"),
            "wrong-piece-source" => origin["source"] = json!("other"),
            "parents" => entries[&later_piece]["parent_ids"] = json!([roots[0]]),
            "position-key" => piece["start"]["extra"] = json!("0"),
            "position-number" => piece["start"]["scalar_offset"] = json!("00"),
            "nonzero-start" => piece["start"]["byte_offset"] = json!("1"),
            "gap" => entries[&pieces[1]]["origin"]["meta"][NS]["start"]["byte_offset"] = json!("2"),
            "backward" => {
                entries[&pieces[1]]["origin"]["meta"][NS]["end"]["utf16_offset"] = json!("0")
            }
            "zero-token" => piece["end"] = piece["start"].clone(),
            "early-eof" => piece["kind"] = json!("eof"),
            "wide-eof" => entries[last]["origin"]["meta"][NS]["end"]["byte_offset"] = json!("99"),
            "final-length" => {
                entries[&roots[0]]["origin"]["meta"][NS]["byte_length"] = json!("99");
                entries[&roots[0]]["origin"]["meta"]["byte_len"] = json!(99);
            }
            _ => unreachable!(),
        }
        refused(&value, case, "source shape");
    }
}

#[test]
fn d3a_compact_preflight_and_generic_evidence_remain_separate() {
    let mut hierarchical = CVLog::new(true);
    hierarchical.try_create(None).unwrap();
    hierarchical.validate_graph().unwrap();
    assert!(validate_shape(&hierarchical, ShapeLimits::default())
        .unwrap_err()
        .contains("compact"));
    for id in [
        "cv1.0000000000000000",
        "cv1.0000000000000002",
        "cv1.ffffffffffffffff",
        "cv1.000000000000000A",
        "a.1",
    ] {
        assert!(compact_index(id, 1).is_err(), "{id}");
    }
    assert_eq!(compact_index("cv1.0000000000000001", 1).unwrap(), 0);
    let mut disabled = CVLog::new_compact(false);
    disabled.try_create(None).unwrap();
    assert!(validate_shape(&disabled, ShapeLimits::default()).is_err());
    let (log, _) = fixture(&[""], EsVersion::Es2025);
    let allocator_only = CVLog::from_json_string(&log.to_json_string().unwrap()).unwrap();
    assert!(validate_shape(&allocator_only, ShapeLimits::default()).is_err());
}

#[test]
fn d3a_exact_zero_one_less_ceiling_and_overflow_limits_are_read_only() {
    let (log, _) = fixture(&["1; /*x*/"], EsVersion::Es2025);
    let before = log.to_json_string().unwrap();
    let (work, bytes) = validate_shape_core(&log, ShapeLimits::default()).unwrap();
    assert!(work > 0 && bytes > 0);
    validate_shape(
        &log,
        ShapeLimits {
            work,
            index_bytes: bytes,
        },
    )
    .unwrap();
    for limits in [
        ShapeLimits {
            work: work - 1,
            index_bytes: bytes,
        },
        ShapeLimits {
            work,
            index_bytes: bytes - 1,
        },
        ShapeLimits {
            work: 0,
            index_bytes: bytes,
        },
        ShapeLimits {
            work,
            index_bytes: 0,
        },
        ShapeLimits {
            work: 512_000_001,
            index_bytes: bytes,
        },
        ShapeLimits {
            work,
            index_bytes: 256 * 1024 * 1024 + 1,
        },
    ] {
        assert!(validate_shape(&log, limits).is_err());
        assert_eq!(log.to_json_string().unwrap(), before);
    }
    assert!(index_storage(usize::MAX).is_err());
    let empty = CVLog::new_compact(true);
    validate_shape(
        &empty,
        ShapeLimits {
            work: 0,
            index_bytes: 0,
        },
    )
    .unwrap();
    // Unrelated metadata and synthetic nodes are generic evidence, not physical
    // pieces. Their presence must neither certify source nor imply an orphan.
    let mut generic = CVLog::new_compact(true);
    generic
        .try_create(Some(Origin {
            source: "other".into(),
            location: "".into(),
            timestamp: None,
            meta: HashMap::from([("ordinary".into(), json!({"caller":"data"}))]),
        }))
        .unwrap();
    validate_shape(&generic, ShapeLimits::default()).unwrap();
}

#[test]
fn d3a_independent_extra_physical_census_and_reserved_placement() {
    for legacy in [false, true] {
        let (mut log, roots) = fixture(&["1;"], EsVersion::Es2025);
        let pieces = ids(&wire(&log), &roots[0]);
        let mut origin = log.get(&pieces[0]).unwrap().origin.clone().unwrap();
        if legacy {
            origin.meta.remove(NS);
        }
        log.try_derive(&roots[0], Some(origin)).unwrap();
        refused(
            &wire(&log),
            if legacy {
                "extra-legacy-child"
            } else {
                "extra-closed-child"
            },
            if legacy { "namespace" } else { "census" },
        );
    }
    for case in [
        "extra-inventory",
        "extra-reserved-contribution",
        "extra-reserved-deletion",
        "input-parent",
    ] {
        let (mut log, roots) = fixture(&["1;", "2;"], EsVersion::Es2025);
        let mut value = wire(&log);
        let root = &roots[0];
        let pieces = ids(&value, root);
        match case {
            "extra-inventory" => {
                let meta = log.get(root).unwrap().contributions[0].meta.clone();
                log.contribute(root, "source.inventory", "finalized", meta)
                    .unwrap();
                value = wire(&log);
            }
            "extra-reserved-contribution" => {
                log.contribute(
                    &pieces[0],
                    "source.inventory",
                    "finalized",
                    HashMap::from([(NS.into(), json!({"role":"inventory"}))]),
                )
                .unwrap();
                value = wire(&log);
            }
            "extra-reserved-deletion" => {
                value["entries"][&pieces[0]]["deleted"] = json!({"source":"source.inventory","reason":"gone","meta":{NS:{"role":"inventory"}}});
            }
            "input-parent" => value["entries"][&roots[1]]["parent_ids"] = json!([root]),
            _ => unreachable!(),
        }
        refused(&value, case, "source shape");
    }
}

#[test]
fn d3a_small_graph_storage_and_work_have_an_independent_schedule() {
    // No metadata/events: this schedule is derivable without asking the gate
    // for a threshold. For each 20-byte compact identity:
    // preflight = entry1 + equality20 + (index1 + two20-byte scans) = 62;
    // reserve initialization = 1; pass A = 1+41; pass B = 1+41 => 147.
    for count in [0, 1, 2, 17] {
        let mut log = CVLog::new_compact(true);
        for _ in 0..count {
            log.try_create(None).unwrap();
        }
        let expected_work = count * 147;
        let expected_bytes = count * size_of::<Option<&Value>>();
        assert_eq!(
            validate_shape_core(&log, ShapeLimits::default()).unwrap(),
            (expected_work, expected_bytes)
        );
        validate_shape(
            &log,
            ShapeLimits {
                work: expected_work,
                index_bytes: expected_bytes,
            },
        )
        .unwrap();
        if count > 0 {
            assert!(validate_shape(
                &log,
                ShapeLimits {
                    work: expected_work - 1,
                    index_bytes: expected_bytes
                }
            )
            .unwrap_err()
            .contains("work limit"));
            assert!(validate_shape(
                &log,
                ShapeLimits {
                    work: expected_work,
                    index_bytes: expected_bytes - 1
                }
            )
            .unwrap_err()
            .contains("index storage"));
        }
    }
}

#[test]
fn d3a_prepublication_refusal_preserves_every_destination_in_every_presentation() {
    use crate::config::{CompilationLevel, CompilerConfig, CorrelationVectorFormat};
    use crate::run::{run_compiler_with_shape_limits, CompilerError};
    use std::sync::atomic::{AtomicU64, Ordering};
    static SEQUENCE: AtomicU64 = AtomicU64::new(0);
    let dir = std::env::temp_dir().join(format!(
        "closurec-d3a-{}-{}",
        std::process::id(),
        SEQUENCE.fetch_add(1, Ordering::Relaxed)
    ));
    std::fs::create_dir(&dir).unwrap();
    let input = dir.join("input.js");
    std::fs::write(&input, "report(1+1);").unwrap();
    let paths = ["out.js", "out.map", "manifest.txt", "trace.json"].map(|p| dir.join(p));
    for level in [
        CompilationLevel::Bundle,
        CompilationLevel::WhitespaceOnly,
        CompilationLevel::Simple,
        CompilationLevel::TranspileOnly,
        CompilationLevel::Advanced,
    ] {
        for format in [
            CorrelationVectorFormat::Json,
            CorrelationVectorFormat::Ndjson,
            CorrelationVectorFormat::None,
        ] {
            for filtered in [false, true] {
                for path in &paths {
                    std::fs::write(path, b"sentinel\0prior bytes").unwrap();
                }
                let mut config = CompilerConfig::default();
                config.compilation.level = level;
                config.io.js_patterns = vec![input.to_string_lossy().into_owned()];
                config.io.js_output_file = Some(paths[0].clone());
                config.source_map.path_template = paths[1].to_string_lossy().into_owned();
                config.chunks.output_manifest_file = Some(paths[2].clone());
                config.special_modes.correlation_vector = true;
                config.special_modes.correlation_vector_format = format;
                config.special_modes.correlation_vector_output = Some(paths[3].clone());
                config.special_modes.correlation_vector_summary = true;
                if filtered {
                    config.special_modes.correlation_vector_filter =
                        vec!["hide-all-source-rows".into()];
                }
                let error = run_compiler_with_shape_limits(
                    &config,
                    ShapeLimits {
                        work: 0,
                        index_bytes: 64 * 1024 * 1024,
                    },
                )
                .unwrap_err();
                assert!(
                    matches!(error, CompilerError::Provenance { ref stage, ref message } if stage=="source_domain" && message=="source shape work limit"),
                    "{error:?}"
                );
                for path in &paths {
                    assert_eq!(std::fs::read(path).unwrap(), b"sentinel\0prior bytes");
                }
                assert_eq!(
                    std::fs::read_dir(&dir).unwrap().count(),
                    5,
                    "no stage/backup files on refusal"
                );
                println!("D3A_PUBLICATION_REFUSAL {level:?} {format:?} filtered={filtered}");
            }
        }
    }
    // Only explicitly owned files are removed; no recursive or computed parent
    // deletion is needed for this failure-before-publication fixture.
    for path in paths.iter().chain(std::iter::once(&input)) {
        std::fs::remove_file(path).unwrap();
    }
    std::fs::remove_dir(dir).unwrap();
}

#[test]
fn d3a_private_error_classification_preserves_configuration_then_graph_then_shape() {
    let disabled = CVLog::new_compact(false);
    assert!(
        matches!(admit_publication(&disabled, ShapeLimits { work: 512_000_001, index_bytes: 0 }), Err(AdmissionError::Shape(message)) if message=="source shape configuration ceiling")
    );
    assert!(matches!(
        admit_publication(&disabled, ShapeLimits::default()),
        Err(AdmissionError::Graph(_))
    ));
    let (log, roots) = fixture(&[""], EsVersion::Es2025);
    let mut value = wire(&log);
    value["entries"][&roots[0]]["origin"]["meta"][NS]["schema"] = json!("2");
    assert!(
        matches!(admit_publication(&checked(&value), ShapeLimits::default()), Err(AdmissionError::Shape(message)) if message=="source shape literal mismatch")
    );
}
