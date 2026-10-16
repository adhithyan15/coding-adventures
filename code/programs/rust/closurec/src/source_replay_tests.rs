//! D3b trust and replay controls, committed before the implementation.
//! Every forged snapshot must pass original checked import AND D3a shape first.
use super::*;
use crate::run::source_inventory::{record_file, SourceInventoryBudget, SourceInventoryLimits};
use coding_adventures_correlation_vector::{CVLog, GraphLimits};
use lexer::token::{Token, TokenType};
use serde_json::{json, Value};
use std::path::{Path, PathBuf};

const NS: &str = "closurec_source_v1";
struct Fixture {
    log: CVLog,
    roots: Vec<String>,
    paths: Vec<PathBuf>,
    catalog: SourceCatalog,
}
fn fixture(sources: &[&str], edition: EsVersion) -> Fixture {
    let mut log = CVLog::new_compact(true);
    let mut budget = SourceInventoryBudget::new(SourceInventoryLimits::default()).unwrap();
    let mut catalog = SourceCatalog::new(sources.len(), edition, ReplayLimits::default()).unwrap();
    let mut roots = Vec::new();
    let paths: Vec<_> = (0..sources.len())
        .map(|i| PathBuf::from(format!("input-{i}.js")))
        .collect();
    for (source, path) in sources.iter().zip(&paths) {
        let owned = source.to_string();
        let recorded = record_file(&mut log, path, &owned, edition, &mut budget).unwrap();
        catalog
            .retain(
                owned,
                &recorded.id,
                recorded.tokens.as_ref().ok().map(Vec::as_slice),
            )
            .unwrap();
        roots.push(recorded.id);
    }
    Fixture {
        log,
        roots,
        paths,
        catalog,
    }
}
fn wire(log: &CVLog) -> Value {
    serde_json::from_str(&log.to_json_string().unwrap()).unwrap()
}
fn checked(value: &Value) -> CVLog {
    // These are already Value mutations, not duplicate controls. Original raw
    // duplicate bytes continue to go directly to the existing checked importer.
    CVLog::from_checked_json(
        &serde_json::to_string(value).unwrap(),
        GraphLimits::default(),
    )
    .unwrap()
}
fn shaped(log: &CVLog) -> ShapeAdmitted<'_> {
    crate::run::source_domain::admit_publication(
        log,
        crate::run::source_domain::ShapeLimits::default(),
    )
    .unwrap_or_else(|_| panic!("generic AND D3a shape prerequisite must pass"))
}
fn pieces(value: &Value, root: &str) -> Vec<String> {
    value["entries"][root]["contributions"][0]["meta"][NS]["piece_ids"]
        .as_array()
        .unwrap()
        .iter()
        .map(|id| id.as_str().unwrap().to_owned())
        .collect()
}
fn refuse(f: &Fixture, value: &Value, case: &str, reason: &str) {
    let log = checked(value);
    let shape = shaped(&log);
    println!("D3B_SHAPE_ONLY_ACCEPTED {case}");
    let before = log.to_json_string().unwrap();
    let error = f
        .catalog
        .validate(&shape, &f.roots, &f.paths)
        .expect_err(case);
    assert!(error.contains(reason), "{case}: {error}, expected {reason}");
    assert_eq!(log.to_json_string().unwrap(), before);
    println!("D3B_REPLAY_REFUSAL {case}");
}

#[test]
fn d3b_semantic_actual_source_replay_all_editions_and_coordinate_domains() {
    for &edition in EsVersion::ALL {
        let f = fixture(
            &[
                "",
                "/* gap */",
                "1;\r\n2;",
                "1;\r2;",
                "\"😀\\n\";",
                "x+x;",
                "\0",
                "1;",
                "1;",
            ],
            edition,
        );
        let before = f.log.to_json_string().unwrap();
        f.catalog
            .validate(&shaped(&f.log), &f.roots, &f.paths)
            .unwrap();
        assert_eq!(f.log.to_json_string().unwrap(), before);
        println!("D3B_ACTUAL_REPLAY {}", edition.as_str());
    }
}

#[test]
fn d3b_semantic_shape_valid_digest_edition_and_coordinate_forgery_is_not_source_evidence() {
    let f = fixture(&["1;"], EsVersion::Es2025);
    for case in ["digest", "edition", "coordinates"] {
        let mut value = wire(&f.log);
        let root = &f.roots[0];
        match case {
            "digest" => {
                value["entries"][root]["origin"]["meta"][NS]["sha256"] = json!("a".repeat(64));
                value["entries"][root]["origin"]["meta"]["content_sha256"] = json!("a".repeat(64));
            }
            "edition" => value["entries"][root]["origin"]["meta"][NS]["edition"] = json!("es5"),
            "coordinates" => {
                // Change an independent coordinate system consistently across
                // every boundary: D3a continuity still holds, bytes unchanged.
                for id in pieces(&value, root) {
                    for edge in ["start", "end"] {
                        for field in ["scalar_offset", "scalar_column"] {
                            let old = value["entries"][&id]["origin"]["meta"][NS][edge][field]
                                .as_str()
                                .unwrap()
                                .parse::<u64>()
                                .unwrap();
                            value["entries"][&id]["origin"]["meta"][NS][edge][field] =
                                json!((old * 2).to_string());
                        }
                    }
                }
            }
            _ => unreachable!(),
        }
        refuse(&f, &value, case, case);
    }
}

#[test]
fn d3b_semantic_shape_valid_kind_and_legacy_token_forgery_requires_actual_grammar() {
    for case in [
        "trivia-as-token",
        "token-as-trivia",
        "token-kind",
        "legacy-length",
        "legacy-location",
    ] {
        let f = fixture(
            &[if case == "trivia-as-token" {
                "/*x*/"
            } else {
                "1"
            }],
            EsVersion::Es2025,
        );
        let mut value = wire(&f.log);
        let root = &f.roots[0];
        let ids = pieces(&value, root);
        let first = &ids[0];
        let eof = ids.last().unwrap();
        match case {
            "trivia-as-token" => {
                let origin = &mut value["entries"][first]["origin"];
                origin["source"] = json!("lexer_token");
                origin["meta"]["kind"] = json!("name");
                origin["meta"]["token_index"] = json!(0);
                origin["meta"]["lexeme_byte_len"] = json!(5);
                origin["meta"][NS]["kind"] = json!("token");
                origin["meta"][NS]["token_index"] = json!("0");
                origin["meta"][NS]["token_type"] = json!("Name");
                value["entries"][eof]["origin"]["meta"][NS]["token_index"] = json!("1");
                value["entries"][eof]["origin"]["meta"]["token_index"] = json!(1);
                value["entries"][root]["contributions"][0]["meta"][NS]["token_count"] = json!("2");
            }
            "token-as-trivia" => {
                let origin = &mut value["entries"][first]["origin"];
                origin["source"] = json!("lexer_trivia");
                origin["meta"][NS]["kind"] = json!("trivia");
                origin["meta"][NS]
                    .as_object_mut()
                    .unwrap()
                    .remove("token_index");
                origin["meta"][NS]
                    .as_object_mut()
                    .unwrap()
                    .remove("token_type");
                value["entries"][eof]["origin"]["meta"][NS]["token_index"] = json!("0");
                value["entries"][eof]["origin"]["meta"]["token_index"] = json!(0);
                value["entries"][root]["contributions"][0]["meta"][NS]["token_count"] = json!("1");
            }
            "token-kind" => {
                value["entries"][first]["origin"]["meta"][NS]["token_type"] = json!("Name");
                value["entries"][first]["origin"]["meta"]["kind"] = json!("name");
            }
            "legacy-length" => {
                value["entries"][first]["origin"]["meta"]["lexeme_byte_len"] = json!(2)
            }
            "legacy-location" => {
                value["entries"][first]["origin"]["location"] = json!("input-0.js:99:99")
            }
            _ => unreachable!(),
        }
        refuse(
            &f,
            &value,
            case,
            if case.contains("legacy") {
                "legacy"
            } else {
                "kind"
            },
        );
    }
}

#[test]
fn d3b_semantic_shape_valid_available_and_unavailable_declarations_need_actual_lex_result() {
    for (source, replacement, case) in [
        ("\0", "1", "unavailable-valid-source"),
        ("1", "\0", "available-invalid-source"),
    ] {
        let original = fixture(&[source], EsVersion::Es2025);
        let mut value = wire(&original.log);
        let mut f = fixture(&[replacement], EsVersion::Es2025);
        // Reuse the original graph allocation/availability, but bind its input
        // descriptor to the real replacement bytes. Shape remains consistent.
        let hash = coding_adventures_sha256::sha256_hex(replacement.as_bytes());
        value["entries"][&original.roots[0]]["origin"]["meta"][NS]["sha256"] = json!(hash);
        value["entries"][&original.roots[0]]["origin"]["meta"]["content_sha256"] = json!(hash);
        f.roots = original.roots;
        refuse(&f, &value, case, "availability");
    }
}

#[test]
fn d3b_original_token_fingerprint_covers_fields_beyond_wire_lengths() {
    for case in [
        "type",
        "value",
        "line",
        "column",
        "type_name",
        "flags",
        "missing-token",
        "reordered-token",
    ] {
        let mut log = CVLog::new_compact(true);
        let mut budget = SourceInventoryBudget::new(SourceInventoryLimits::default()).unwrap();
        let source = "report;".to_owned();
        let recorded = record_file(
            &mut log,
            Path::new("input-0.js"),
            &source,
            EsVersion::Es2025,
            &mut budget,
        )
        .unwrap();
        let mut tokens = recorded.tokens.unwrap();
        match case {
            "type" => tokens[0].type_ = TokenType::Number,
            "value" => tokens[0].value = "REport".into(),
            "line" => tokens[0].line = 99,
            "column" => tokens[0].column = 99,
            "type_name" => tokens[0].type_name = Some("forged-name".into()),
            "flags" => {
                tokens[0].flags = if tokens[0].flags.is_none() {
                    Some(0)
                } else {
                    None
                }
            }
            "missing-token" => {
                tokens.pop();
            }
            "reordered-token" => tokens.swap(0, 1),
            _ => unreachable!(),
        }
        let mut catalog =
            SourceCatalog::new(1, EsVersion::Es2025, ReplayLimits::default()).unwrap();
        catalog.retain(source, &recorded.id, Some(&tokens)).unwrap();
        let error = catalog
            .validate(
                &shaped(&log),
                &[recorded.id],
                &[PathBuf::from("input-0.js")],
            )
            .unwrap_err();
        assert!(error.contains("fingerprint"), "{case}: {error}");
        println!("D3B_TOKEN_FINGERPRINT_REFUSAL {case}");
    }
}

fn independently_encoded_token(token: &Token) -> Vec<u8> {
    let mut bytes = b"closurec.lexical-token.v1\0".to_vec();
    // Display is a separate implementation from the producer's frozen wire
    // spelling match. Encode complete bytes independently of production helpers.
    let kind = token.type_.to_string();
    for text in [kind.as_str(), token.value.as_str()] {
        bytes.extend_from_slice(&(text.len() as u64).to_le_bytes());
        bytes.extend_from_slice(text.as_bytes());
    }
    bytes.extend_from_slice(&(token.line as u64).to_le_bytes());
    bytes.extend_from_slice(&(token.column as u64).to_le_bytes());
    match &token.type_name {
        None => bytes.push(0),
        Some(name) => {
            bytes.push(1);
            bytes.extend_from_slice(&(name.len() as u64).to_le_bytes());
            bytes.extend_from_slice(name.as_bytes());
        }
    }
    match token.flags {
        None => bytes.push(0),
        Some(flags) => {
            bytes.push(1);
            bytes.extend_from_slice(&flags.to_le_bytes());
        }
    }
    bytes.push(0);
    bytes
}

#[test]
fn d3b_fingerprint_framing_and_work_are_independent_and_complete() {
    let base = Token {
        type_: TokenType::Name,
        value: "abc".into(),
        line: 1,
        column: 2,
        type_name: None,
        flags: None,
        cv: None,
    };
    let mut fingerprints = std::collections::BTreeSet::new();
    for (name, flags) in [
        (None, None),
        (Some(""), None),
        (None, Some(0)),
        (Some(""), Some(0)),
        (Some("x"), Some(7)),
    ] {
        let mut token = base.clone();
        token.type_name = name.map(str::to_owned);
        token.flags = flags;
        let encoded = independently_encoded_token(&token);
        let expected_work = 1 + 2 * encoded.len() + 128;
        let mut work = Work {
            used: 0,
            limit: expected_work,
        };
        let actual = fingerprint(&token, &mut work).unwrap();
        assert_eq!(actual, coding_adventures_sha256::sha256(&encoded));
        assert_eq!(work.used, expected_work);
        assert!(fingerprints.insert(actual));
        assert!(fingerprint(
            &token,
            &mut Work {
                used: 0,
                limit: expected_work - 1
            }
        )
        .is_err());
    }
    for total in [64, 119, 120, 127, 128, 183, 184, 191, 192] {
        let mut token = base.clone();
        token.value = "😀\0".into();
        let prefix = independently_encoded_token(&token).len();
        // Complete framing already exceeds SHA's first 55/56-byte padding
        // boundary; exercise later equivalent residues with UTF8 and NUL.
        if total < prefix {
            continue;
        }
        token.value.push_str(&"x".repeat(total - prefix));
        let encoded = independently_encoded_token(&token);
        assert_eq!(encoded.len(), total);
        assert_ne!(token.value.len(), token.value.chars().count());
        let expected = 1 + 2 * total + 128;
        let mut work = Work {
            used: 0,
            limit: expected,
        };
        assert_eq!(
            fingerprint(&token, &mut work).unwrap(),
            coding_adventures_sha256::sha256(&encoded)
        );
        assert_eq!(work.used, expected);
        assert!(fingerprint(
            &token,
            &mut Work {
                used: 0,
                limit: expected - 1
            }
        )
        .is_err());
    }
    let mut left = base.clone();
    left.value = "a".into();
    left.type_name = Some("bc".into());
    let mut right = base.clone();
    right.value = "ab".into();
    right.type_name = Some("c".into());
    assert_ne!(
        fingerprint(
            &left,
            &mut Work {
                used: 0,
                limit: 10000
            }
        )
        .unwrap(),
        fingerprint(
            &right,
            &mut Work {
                used: 0,
                limit: 10000
            }
        )
        .unwrap()
    );
    let mut stamped = base;
    stamped.cv = Some("cv1.0000000000000001".into());
    assert!(fingerprint(
        &stamped,
        &mut Work {
            used: 0,
            limit: 10000
        }
    )
    .unwrap_err()
    .contains("stamped"));
}

#[test]
fn d3b_moved_consumed_allocation_survives_path_mutation_without_reread() {
    use std::sync::atomic::{AtomicU64, Ordering};
    static SEQUENCE: AtomicU64 = AtomicU64::new(0);
    let path = std::env::temp_dir().join(format!(
        "closurec-d3b-{}-{}.js",
        std::process::id(),
        SEQUENCE.fetch_add(1, Ordering::Relaxed)
    ));
    std::fs::write(&path, "report(1);").unwrap();
    let source = std::fs::read_to_string(&path).unwrap();
    let ptr = source.as_ptr();
    let cap = source.capacity();
    let mut log = CVLog::new_compact(true);
    let mut budget = SourceInventoryBudget::new(SourceInventoryLimits::default()).unwrap();
    let recorded = record_file(&mut log, &path, &source, EsVersion::Es2025, &mut budget).unwrap();
    let mut catalog = SourceCatalog::new(1, EsVersion::Es2025, ReplayLimits::default()).unwrap();
    let retained = catalog
        .retain(
            source,
            &recorded.id,
            Some(recorded.tokens.as_ref().unwrap()),
        )
        .unwrap();
    assert_eq!(retained.as_ptr(), ptr);
    assert_eq!(catalog.inputs[0].source.capacity(), cap);
    std::fs::write(&path, "report(2);").unwrap();
    catalog
        .validate(
            &shaped(&log),
            &[recorded.id.clone()],
            std::slice::from_ref(&path),
        )
        .unwrap();
    let mut wrong = SourceCatalog::new(1, EsVersion::Es2025, ReplayLimits::default()).unwrap();
    wrong
        .retain(
            std::fs::read_to_string(&path).unwrap(),
            &recorded.id,
            Some(recorded.tokens.as_ref().unwrap()),
        )
        .unwrap();
    assert!(wrong
        .validate(&shaped(&log), &[recorded.id], std::slice::from_ref(&path))
        .unwrap_err()
        .contains("digest"));
    std::fs::remove_file(path).unwrap();
}

#[test]
fn d3b_catalog_cardinality_root_order_and_complete_input_census() {
    let f = fixture(&["1;", "1;"], EsVersion::Es2025);
    let shape = shaped(&f.log);
    for roots in [
        vec![f.roots[0].clone()],
        vec![f.roots[1].clone(), f.roots[0].clone()],
        vec![f.roots[0].clone(), f.roots[0].clone()],
    ] {
        assert!(f.catalog.validate(&shape, &roots, &f.paths).is_err());
    }
    assert!(f.catalog.validate(&shape, &f.roots, &f.paths[..1]).is_err());
    let mut wrong_paths = f.paths.clone();
    wrong_paths[0] = PathBuf::from("wrong.js");
    assert!(f
        .catalog
        .validate(&shape, &f.roots, &wrong_paths)
        .unwrap_err()
        .contains("path"));
    let one = fixture(&["1;"], EsVersion::Es2025);
    let error = one
        .catalog
        .validate(&shape, &one.roots, &one.paths)
        .unwrap_err();
    assert!(error.contains("census"), "{error}");
    let mut owner = SourceCatalog::new(1, EsVersion::Es2025, ReplayLimits::default()).unwrap();
    assert!(owner
        .retain("1;".into(), "cv1.0000000000000000", None)
        .is_err());
}

#[test]
fn d3b_original_capture_status_is_independent_of_declared_and_replayed_availability() {
    for (source, original_available) in [("1", false), ("\0", true)] {
        let f = fixture(&[source], EsVersion::Es2025);
        let mut owner = SourceCatalog::new(1, EsVersion::Es2025, ReplayLimits::default()).unwrap();
        owner
            .retain(
                source.into(),
                &f.roots[0],
                if original_available { Some(&[]) } else { None },
            )
            .unwrap();
        assert!(owner
            .validate(&shaped(&f.log), &f.roots, &f.paths)
            .unwrap_err()
            .contains("availability"));
    }
}

#[test]
fn d3b_retention_order_slots_and_fingerprint_failure_preserve_the_committed_prefix() {
    let token = Token {
        type_: TokenType::Name,
        value: "a".into(),
        line: 1,
        column: 1,
        type_name: None,
        flags: None,
        cv: None,
    };
    for (slots, root, stamped, reason) in [
        (2, "cv1.0000000000000002", false, "root"),
        (2, "cv1.0000000000000001", false, "root"),
        (1, "cv1.0000000000000003", false, "slot"),
        (2, "cv1.0000000000000003", true, "stamped"),
    ] {
        let mut owner =
            SourceCatalog::new(slots, EsVersion::Es2025, ReplayLimits::default()).unwrap();
        owner
            .retain(
                "a".into(),
                "cv1.0000000000000002",
                Some(std::slice::from_ref(&token)),
            )
            .unwrap();
        let ledger = owner.used;
        let pointer = owner.inputs[0].source.as_ptr();
        let fingerprints = owner.inputs[0].fingerprints.clone();
        let mut next_token = token.clone();
        if stamped {
            next_token.cv = Some("cv1.0000000000000003".into());
        }
        assert!(owner
            .retain("a".into(), root, Some(&[next_token]))
            .unwrap_err()
            .contains(reason));
        assert_eq!(owner.used, ledger);
        assert_eq!(owner.inputs.len(), 1);
        assert_eq!(owner.inputs[0].source.as_ptr(), pointer);
        assert_eq!(owner.inputs[0].source, "a");
        assert_eq!(owner.inputs[0].root, 2);
        assert_eq!(owner.inputs[0].fingerprints, fingerprints);
    }
}

#[test]
fn d3b_independent_complete_replay_work_schedule_exact_and_one_less() {
    for source in ["\0", "1"] {
        let mut f = fixture(&[source], EsVersion::Es2025);
        let expected = if source == "\0" {
            854
        } else {
            let capture =
                coding_adventures_javascript_lexer::tokenize_javascript_typed_with_ranges(
                    source,
                    EsVersion::Es2025,
                    lexer::lexical::LexicalLimits::default(),
                )
                .unwrap();
            let fingerprints: usize = capture
                .tokens()
                .iter()
                .map(|token| 1 + 2 * independently_encoded_token(token).len() + 128)
                .sum();
            3010 + 2 * fingerprints
        };
        f.catalog.limits.work = expected;
        assert_eq!(
            f.catalog
                .validate_core(&shaped(&f.log), &f.roots, &f.paths)
                .unwrap(),
            expected
        );
        f.catalog.limits.work = expected - 1;
        assert!(f
            .catalog
            .validate(&shaped(&f.log), &f.roots, &f.paths)
            .unwrap_err()
            .contains("work"));
    }
}

#[test]
fn d3b_catalog_resource_boundaries_charge_capacity_and_preserve_prefix() {
    assert!(catalog_storage(usize::MAX).is_err());
    assert!(fingerprint_storage(usize::MAX).is_err());
    assert_eq!(catalog_storage(2).unwrap(), 2 * size_of::<TrustedInput>());
    assert_eq!(fingerprint_storage(3).unwrap(), 96);
    assert!(SourceCatalog::new(usize::MAX, EsVersion::Es2025, ReplayLimits::default()).is_err());
    let mut limits = ReplayLimits::default();
    limits.index_bytes = catalog_storage(1).unwrap();
    SourceCatalog::new(1, EsVersion::Es2025, limits).unwrap();
    limits.index_bytes -= 1;
    assert!(SourceCatalog::new(1, EsVersion::Es2025, limits).is_err());
    for (source_capacity, index_bytes, fingerprint_bytes, work) in [
        (256 * 1024 * 1024 + 1, 0, 0, 0),
        (0, 64 * 1024 * 1024 + 1, 0, 0),
        (0, 0, 64 * 1024 * 1024 + 1, 0),
        (0, 0, 0, 512_000_001),
    ] {
        assert!(SourceCatalog::new(
            0,
            EsVersion::Es2025,
            ReplayLimits {
                source_capacity,
                index_bytes,
                fingerprint_bytes,
                work
            }
        )
        .is_err());
    }
    let mut limits = ReplayLimits::default();
    limits.source_capacity = 16;
    let mut owner = SourceCatalog::new(2, EsVersion::Es2025, limits).unwrap();
    owner
        .retain("1".into(), "cv1.0000000000000001", None)
        .unwrap();
    let before = owner.used;
    let mut spare = String::with_capacity(128);
    spare.push('1');
    assert!(owner
        .retain(spare, "cv1.0000000000000002", None)
        .unwrap_err()
        .contains("capacity"));
    assert_eq!(owner.used, before);
    assert_eq!(owner.inputs.len(), 1);
    assert_eq!(owner.inputs[0].source, "1");
    let f = fixture(&["1;"], EsVersion::Es2025);
    let work = f
        .catalog
        .validate_core(&shaped(&f.log), &f.roots, &f.paths)
        .unwrap();
    let mut limited = f.catalog;
    limited.limits.work = work;
    limited
        .validate(&shaped(&f.log), &f.roots, &f.paths)
        .unwrap();
    limited.limits.work -= 1;
    assert!(limited
        .validate(&shaped(&f.log), &f.roots, &f.paths)
        .unwrap_err()
        .contains("work"));
    let empty = CVLog::new_compact(true);
    let zero = SourceCatalog::new(
        0,
        EsVersion::Es2025,
        ReplayLimits {
            source_capacity: 0,
            index_bytes: 0,
            fingerprint_bytes: 0,
            work: 0,
        },
    )
    .unwrap();
    zero.validate(&shaped(&empty), &[], &[]).unwrap();
}

#[test]
fn d3b_exact_source_and_fingerprint_retention_refusals_are_atomic() {
    let token = Token {
        type_: TokenType::Name,
        value: "a".into(),
        line: 1,
        column: 1,
        type_name: None,
        flags: None,
        cv: None,
    };
    for admitted in [true, false] {
        let source = "a".to_owned();
        let capacity = source.capacity();
        let mut limits = ReplayLimits::default();
        limits.source_capacity = capacity;
        limits.fingerprint_bytes = if admitted { 32 } else { 31 };
        let mut owner = SourceCatalog::new(1, EsVersion::Es2025, limits).unwrap();
        let before = owner.used;
        let result = owner.retain(
            source,
            "cv1.0000000000000001",
            Some(std::slice::from_ref(&token)),
        );
        if admitted {
            result.unwrap();
            assert_eq!(owner.used.source_capacity, capacity);
            assert_eq!(owner.used.fingerprint_bytes, 32);
        } else {
            assert!(result.unwrap_err().contains("fingerprint storage"));
            assert_eq!(owner.used, before);
            assert!(owner.inputs.is_empty());
        }
    }
    let source = "a".to_owned();
    let mut limits = ReplayLimits::default();
    limits.source_capacity = source.capacity() - 1;
    assert!(SourceCatalog::new(1, EsVersion::Es2025, limits)
        .unwrap()
        .retain(source, "cv1.0000000000000001", None)
        .is_err());
    let mut limits = ReplayLimits::default();
    limits.work = 0;
    assert!(SourceCatalog::new(1, EsVersion::Es2025, limits).is_err());
}

#[test]
fn d3b_actual_production_capture_limit_is_never_unavailable_success() {
    let f = fixture(&["\0"], EsVersion::Es2025);
    let source = "x".repeat(lexer::lexical::LexicalLimits::default().max_source_bytes + 1);
    assert!(matches!(
        coding_adventures_javascript_lexer::tokenize_javascript_typed_with_ranges(
            &source,
            EsVersion::Es2025,
            lexer::lexical::LexicalLimits::default()
        ),
        Err(lexer::lexical::LexicalError::Limit("source bytes"))
    ));
    let mut value = wire(&f.log);
    let root = &f.roots[0];
    let mut hasher = coding_adventures_sha256::Sha256Hasher::new();
    for block in source.as_bytes().chunks(64) {
        hasher.update(block);
    }
    let hash = hasher.hex_digest();
    value["entries"][root]["origin"]["meta"][NS]["sha256"] = json!(hash);
    value["entries"][root]["origin"]["meta"]["content_sha256"] = json!(hash);
    value["entries"][root]["origin"]["meta"][NS]["byte_length"] = json!(source.len().to_string());
    value["entries"][root]["origin"]["meta"]["byte_len"] = json!(source.len());
    let mut owner = SourceCatalog::new(1, EsVersion::Es2025, ReplayLimits::default()).unwrap();
    owner.retain(source, root, None).unwrap();
    let log = checked(&value);
    assert!(owner
        .validate(&shaped(&log), &f.roots, &f.paths)
        .unwrap_err()
        .contains("capture refused"));
}

#[test]
fn d3b_actual_compiler_early_retention_and_final_replay_refusals_preserve_all_destination_bytes() {
    use crate::config::{CompilationLevel, CompilerConfig, CorrelationVectorFormat};
    use crate::run::{run_compiler_with_replay_limits, CompilerError};
    use std::sync::atomic::{AtomicU64, Ordering};
    static SEQUENCE: AtomicU64 = AtomicU64::new(0);
    let dir = std::env::temp_dir().join(format!(
        "closurec-d3b-pub-{}-{}",
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
                for early in [true, false] {
                    for path in &paths {
                        std::fs::write(path, b"prior\0source replay sentinel").unwrap();
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
                            vec!["hide-all-input-rows".into()];
                    }
                    let mut limits = ReplayLimits::default();
                    let reason = if early {
                        limits.source_capacity = 0;
                        "source capacity"
                    } else {
                        // Independently pay exactly the entire retain phase. The
                        // first final-replay census visit must exhaust this budget;
                        // bypassing final replay would instead publish sentinels.
                        let edition = crate::run::map_language_in_to_es_version(&config);
                        let capture = coding_adventures_javascript_lexer::tokenize_javascript_typed_with_ranges("report(1+1);", edition, lexer::lexical::LexicalLimits::default()).unwrap();
                        let retention = 43
                            + capture
                                .tokens()
                                .iter()
                                .map(|token| 1 + 2 * independently_encoded_token(token).len() + 128)
                                .sum::<usize>();
                        limits.work = retention;
                        let mut control = SourceCatalog::new(1, edition, limits).unwrap();
                        control
                            .retain(
                                "report(1+1);".into(),
                                "cv1.0000000000000001",
                                Some(capture.tokens()),
                            )
                            .unwrap();
                        assert_eq!(control.used.work, retention);
                        "source replay work limit"
                    };
                    let error = run_compiler_with_replay_limits(&config, limits).unwrap_err();
                    assert!(
                        matches!(error,CompilerError::Provenance{ref stage,ref message} if stage=="source_replay" && message.contains(reason)),
                        "{error:?}"
                    );
                    for path in &paths {
                        assert_eq!(
                            std::fs::read(path).unwrap(),
                            b"prior\0source replay sentinel"
                        );
                    }
                    assert_eq!(std::fs::read_dir(&dir).unwrap().count(), 5);
                    println!("D3B_PUBLICATION_REFUSAL {level:?} {format:?} filtered={filtered} early={early}");
                }
            }
        }
    }
    for path in paths.iter().chain(std::iter::once(&input)) {
        std::fs::remove_file(path).unwrap();
    }
    std::fs::remove_dir(dir).unwrap();
}
