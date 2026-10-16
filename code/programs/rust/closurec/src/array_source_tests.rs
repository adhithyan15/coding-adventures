//! CLOC33 private trust/limit controls, committed before implementation.
use super::*;
use crate::run::source_inventory::{
    record_file, stamp_ingestion_ids, SourceInventoryBudget, SourceInventoryLimits,
};
use crate::run::source_replay::{ReplayLimits, SourceCatalog};
use coding_adventures_correlation_vector::GraphLimits;
use coding_adventures_javascript_parser::{
    asi::parse_with_asi, bridge::grammar_to_program_with_array_observer, DEFAULT_ES_VERSION,
};
use serde_json::json;
use std::path::{Path, PathBuf};

struct Fixture {
    log: CVLog,
    catalog: SourceCatalog,
    roots: Vec<String>,
    paths: Vec<PathBuf>,
    ledger: ArrayLedger,
}
fn fixture_with(source: &str, limits: ArrayLimits, graph: GraphLimits) -> Result<Fixture, String> {
    let mut log = CVLog::new_checked_chronology(graph)?;
    let mut budget = SourceInventoryBudget::new(SourceInventoryLimits::default())?;
    let mut catalog = SourceCatalog::new(1, DEFAULT_ES_VERSION, ReplayLimits::default())?;
    let path = Path::new("array-input.js");
    let owned = source.to_owned();
    let recorded = record_file(&mut log, path, &owned, DEFAULT_ES_VERSION, &mut budget)?;
    catalog.retain(
        owned,
        &recorded.id,
        recorded.tokens.as_ref().ok().map(Vec::as_slice),
    )?;
    let mut tokens = recorded.tokens?;
    log.contribute(
        &recorded.id,
        "lex",
        "tokens_emitted",
        std::collections::HashMap::from([("token_count".into(), json!(tokens.len()))]),
    )?;
    stamp_ingestion_ids(&mut tokens, recorded.token_ids)?;
    let node = parse_with_asi(tokens, DEFAULT_ES_VERSION).map_err(|e| e.to_string())?;
    let mut ledger = ArrayLedger::new(limits)?;
    let program = {
        let mut observer = ledger.observer(&mut log, &recorded.id)?;
        grammar_to_program_with_array_observer(&node, DEFAULT_ES_VERSION, &mut observer)
            .map_err(|e| e.to_string())?
    };
    crate::run::run_typed_pipeline(program, None, Some(&mut log)).map_err(|e| format!("{e:?}"))?;
    Ok(Fixture {
        log,
        catalog,
        roots: vec![recorded.id],
        paths: vec![path.into()],
        ledger,
    })
}
fn fixture() -> Fixture {
    fixture_with(
        "report([].length);",
        ArrayLimits::default(),
        GraphLimits::default(),
    )
    .unwrap()
}
fn validate(f: &Fixture, log: &CVLog) -> Result<(), String> {
    let shape = crate::run::source_domain::admit_publication(
        log,
        crate::run::source_domain::ShapeLimits::default(),
    )
    .map_err(|_| "test generic/source shape prerequisite")?;
    let source = f.catalog.admit(&shape, &f.roots, &f.paths)?;
    f.ledger.validate(&source)
}
fn wire(log: &CVLog) -> Value {
    serde_json::from_str(&log.to_json_string().unwrap()).unwrap()
}
fn checked(value: &Value) -> CVLog {
    CVLog::from_checked_json(
        &serde_json::to_string(value).unwrap(),
        GraphLimits::default(),
    )
    .unwrap()
}
fn array_id(f: &Fixture) -> String {
    f.log
        .entries()
        .iter()
        .find(|(_, e)| e.origin.as_ref().is_some_and(|o| o.source == "typed_array"))
        .unwrap()
        .0
        .clone()
}

#[test]
fn c33_final_source_premise_accepts_actual_construction_after_pipeline() {
    let mut f = fixture();
    assert!(!f.ledger.receipts.is_empty());
    validate(&f, &f.log).unwrap();
    let array = array_id(&f);
    f.log
        .contribute(&array, "later-test", "ordinary-history", Default::default())
        .unwrap();
    validate(&f, &f.log).unwrap();
}
#[test]
fn c33_source_valid_graph_cannot_forge_array_origin_or_ordered_parents() {
    let f = fixture();
    let id = array_id(&f);
    let original = wire(&f.log);
    for case in [
        "missing-namespace",
        "extra-field",
        "range",
        "root",
        "boundary",
        "parents",
        "wrong-source",
        "location",
    ] {
        let mut v = original.clone();
        let entry = &mut v["entries"][&id];
        match case {
            "missing-namespace" => {
                entry["origin"]["meta"].as_object_mut().unwrap().remove(NS);
            }
            "extra-field" => entry["origin"]["meta"][NS]["forged"] = json!(true),
            "range" => entry["origin"]["meta"][NS]["start"]["byte_offset"] = json!("0"),
            "root" => entry["origin"]["meta"][NS]["input_id"] = json!(id),
            "boundary" => {
                entry["origin"]["meta"][NS]["first_piece_id"] =
                    entry["origin"]["meta"][NS]["last_piece_id"].clone()
            }
            "parents" => entry["parent_ids"].as_array_mut().unwrap().reverse(),
            "wrong-source" => entry["origin"]["source"] = json!("ordinary"),
            "location" => entry["origin"]["location"] = json!("invented"),
            _ => unreachable!(),
        }
        let log = checked(&v);
        assert!(validate(&f, &log).is_err(), "accepted {case}");
    }
}
#[test]
fn c33_final_census_rejects_late_unreceipted_and_misplaced_reserved_facts() {
    for case in ["unreceipted", "contribution", "deletion"] {
        let mut f = fixture();
        let id = array_id(&f);
        let meta = std::collections::HashMap::from([(NS.to_owned(), json!({"schema":"1"}))]);
        match case {
            "unreceipted" => {
                let origin = f.log.get(&id).unwrap().origin.clone().unwrap();
                f.log.try_create(Some(origin)).unwrap();
            }
            "contribution" => {
                f.log.contribute(&id, "late-pass", "tamper", meta).unwrap();
            }
            "deletion" => {
                f.log.try_delete(&id, "late-pass", "tamper", meta).unwrap();
            }
            _ => unreachable!(),
        }
        let before = f.log.to_json_string().unwrap();
        assert!(validate(&f, &f.log).is_err(), "accepted {case}");
        assert_eq!(
            f.log.to_json_string().unwrap(),
            before,
            "admission must be read-only"
        );
    }
}
#[test]
fn c33_private_receipts_bind_exact_merge_clock_and_actual_occurrence_census() {
    for case in ["missing", "sequence", "entity", "duplicate"] {
        let mut f = fixture();
        match case {
            "missing" => {
                f.ledger.receipts.clear();
            }
            "sequence" => f.ledger.receipts[0].merge_sequence += 1,
            "entity" => f.ledger.receipts[0].id += 1,
            "duplicate" => {
                let row = f.ledger.receipts[0];
                f.ledger.receipts.push(row);
            }
            _ => unreachable!(),
        }
        assert!(validate(&f, &f.log).is_err(), "accepted {case}");
    }
}
#[test]
fn c33_all_new_limits_and_original_checked_graph_limits_refuse() {
    for case in ["work", "buffer", "ledger", "metadata"] {
        let mut limits = ArrayLimits::default();
        match case {
            "work" => limits.work = 0,
            "buffer" => limits.buffer_bytes = 0,
            "ledger" => limits.ledger_bytes = 0,
            "metadata" => limits.metadata_bytes = 0,
            _ => unreachable!(),
        }
        assert!(
            fixture_with("report([].length);", limits, GraphLimits::default()).is_err(),
            "accepted {case}"
        );
    }
    let mut f = fixture();
    f.ledger.limits.work = f.ledger.used_work;
    assert!(
        validate(&f, &f.log).is_err(),
        "final census cannot run on an exhausted budget"
    );
    let capture = coding_adventures_javascript_lexer::tokenize_javascript_typed_with_ranges(
        "[];",
        DEFAULT_ES_VERSION,
        lexer::lexical::LexicalLimits::default(),
    )
    .unwrap();
    let graph = GraphLimits {
        max_nodes: capture.pieces().len() + 1,
        ..GraphLimits::default()
    };
    assert!(
        fixture_with("[];", ArrayLimits::default(), graph).is_err(),
        "array checked allocation must honor graph node limit"
    );
    for limits in [
        ArrayLimits {
            work: 512_000_001,
            ..ArrayLimits::default()
        },
        ArrayLimits {
            buffer_bytes: 128 * 1024 * 1024 + 1,
            ..ArrayLimits::default()
        },
        ArrayLimits {
            ledger_bytes: 64 * 1024 * 1024 + 1,
            ..ArrayLimits::default()
        },
        ArrayLimits {
            metadata_bytes: 64 * 1024 * 1024 + 1,
            ..ArrayLimits::default()
        },
    ] {
        assert!(ArrayLedger::new(limits).is_err());
    }
}
#[test]
fn c33_actual_compiler_refusals_keep_every_preexisting_destination() {
    use crate::config::{CompilationLevel, CompilerConfig, CorrelationVectorFormat};
    use crate::run::{run_compiler_with_array_limits, CompilerError};
    use std::sync::atomic::{AtomicU64, Ordering};
    static SEQUENCE: AtomicU64 = AtomicU64::new(0);
    let dir = std::env::temp_dir().join(format!(
        "closurec-c33-pub-{}-{}",
        std::process::id(),
        SEQUENCE.fetch_add(1, Ordering::Relaxed)
    ));
    std::fs::create_dir(&dir).unwrap();
    let input = dir.join("input.js");
    std::fs::write(&input, "report([].length);").unwrap();
    let paths = ["out.js", "out.map", "manifest.txt", "trace.json"].map(|p| dir.join(p));
    for level in [CompilationLevel::Simple, CompilationLevel::Advanced] {
        for format in [
            CorrelationVectorFormat::Json,
            CorrelationVectorFormat::Ndjson,
            CorrelationVectorFormat::None,
        ] {
            for filtered in [false, true] {
                for case in ["work", "buffer", "ledger", "metadata", "final-work"] {
                    for p in &paths {
                        std::fs::write(p, b"prior\0array source sentinel").unwrap();
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
                            vec!["hide-array-rows".into()];
                    }
                    let mut limits = ArrayLimits::default();
                    match case {
                        "work" => limits.work = 0,
                        "buffer" => limits.buffer_bytes = 0,
                        "ledger" => limits.ledger_bytes = 0,
                        "metadata" => limits.metadata_bytes = 0,
                        "final-work" => limits.work = fixture().ledger.used_work,
                        _ => unreachable!(),
                    }
                    let error = run_compiler_with_array_limits(&config, limits).unwrap_err();
                    assert!(
                        matches!(error,CompilerError::Provenance{ref stage,..} if stage==if case=="final-work" {"array_source_admission"} else {"array_source"}),
                        "{error:?}"
                    );
                    for p in &paths {
                        assert_eq!(std::fs::read(p).unwrap(), b"prior\0array source sentinel");
                    }
                    assert_eq!(std::fs::read_dir(&dir).unwrap().count(), 5);
                }
            }
        }
    }
    for p in paths.iter().chain(std::iter::once(&input)) {
        std::fs::remove_file(p).unwrap();
    }
    std::fs::remove_dir(dir).unwrap();
}

#[test]
fn c33_final_validation_obeys_transient_buffer_budget() {
    let mut f = fixture();
    f.ledger.limits.buffer_bytes = 0;
    assert!(
        validate(&f, &f.log).is_err(),
        "final identity preparation cannot allocate outside the buffer limit"
    );
    f.ledger.limits.buffer_bytes = ArrayLimits::default().buffer_bytes;
    f.ledger.limits.work = f.ledger.used_work + 1;
    assert!(
        validate(&f, &f.log).is_err(),
        "one row visit does not buy identity encoding/scans"
    );
}
#[test]
fn c33_missing_foreign_duplicate_reversed_and_wrong_kind_boundaries_never_mutate_graph() {
    fn find(node: &mut GrammarASTNode) -> &mut GrammarASTNode {
        if node.rule_name == "array_literal" {
            return node;
        }
        for child in &mut node.children {
            if let ASTNodeOrToken::Node(n) = child {
                if contains(n) {
                    return find(n);
                }
            }
        }
        panic!("array node");
    }
    fn contains(node: &GrammarASTNode) -> bool {
        node.rule_name == "array_literal"
            || node
                .children
                .iter()
                .any(|c| matches!(c,ASTNodeOrToken::Node(n) if contains(n)))
    }
    for case in [
        "missing",
        "foreign",
        "duplicate",
        "reversed",
        "wrong-kind",
        "noncompact",
    ] {
        let mut log = CVLog::new_checked_chronology(GraphLimits::default()).unwrap();
        let mut budget = SourceInventoryBudget::new(SourceInventoryLimits::default()).unwrap();
        let recorded = record_file(
            &mut log,
            Path::new("first.js"),
            "[];",
            DEFAULT_ES_VERSION,
            &mut budget,
        )
        .unwrap();
        let foreign = record_file(
            &mut log,
            Path::new("other.js"),
            "[];",
            DEFAULT_ES_VERSION,
            &mut budget,
        )
        .unwrap();
        let mut tokens = recorded.tokens.unwrap();
        stamp_ingestion_ids(&mut tokens, recorded.token_ids).unwrap();
        let mut node = parse_with_asi(tokens, DEFAULT_ES_VERSION).unwrap();
        let array = find(&mut node);
        let len = array.children.len();
        let original_first = match &array.children[0] {
            ASTNodeOrToken::Token(t) => t.cv.clone().unwrap(),
            _ => panic!("open"),
        };
        let original_last = match &array.children[len - 1] {
            ASTNodeOrToken::Token(t) => t.cv.clone().unwrap(),
            _ => panic!("close"),
        };
        match case {
            "missing" => {
                if let ASTNodeOrToken::Token(t) = &mut array.children[0] {
                    t.cv = None;
                }
            }
            "foreign" => {
                if let ASTNodeOrToken::Token(t) = &mut array.children[0] {
                    t.cv = Some(foreign.token_ids[0].clone());
                }
            }
            "duplicate" => {
                if let ASTNodeOrToken::Token(t) = &mut array.children[len - 1] {
                    t.cv = Some(original_first.clone());
                }
            }
            "reversed" => {
                if let ASTNodeOrToken::Token(t) = &mut array.children[0] {
                    t.cv = Some(original_last);
                }
                if let ASTNodeOrToken::Token(t) = &mut array.children[len - 1] {
                    t.cv = Some(original_first);
                }
            }
            "wrong-kind" => {
                if let ASTNodeOrToken::Token(t) = &mut array.children[0] {
                    t.type_ = TokenType::Name;
                }
            }
            "noncompact" => {
                if let ASTNodeOrToken::Token(t) = &mut array.children[0] {
                    t.cv = Some("not-an-id".into());
                }
            }
            _ => unreachable!(),
        }
        let before = log.to_json_string().unwrap();
        let mut ledger = ArrayLedger::new(ArrayLimits::default()).unwrap();
        {
            let mut observer = ledger.observer(&mut log, &recorded.id).unwrap();
            assert!(
                observer.array_constructed(array).is_err(),
                "accepted {case}"
            );
        }
        assert_eq!(
            log.to_json_string().unwrap(),
            before,
            "{case} mutated graph"
        );
        assert!(ledger.receipts.is_empty());
    }
}
#[test]
fn c33_exact_and_one_less_construction_limits_and_nonpower_receipt_growth() {
    for source in ["[];", "[];[];[];", "[];[];[];[];[];"] {
        let f = fixture_with(source, ArrayLimits::default(), GraphLimits::default()).unwrap();
        let ledger_bytes = f.ledger.receipts.capacity() * size_of::<Receipt>();
        let parent_bytes = 2 * (size_of::<String>() + 64 + size_of::<&str>());
        let peak = ledger_bytes + parent_bytes + 4096;
        let exact = ArrayLimits {
            work: f.ledger.used_work,
            buffer_bytes: peak,
            ledger_bytes,
            metadata_bytes: f.ledger.receipts.len() * 4096,
        };
        fixture_with(source, exact, GraphLimits::default()).unwrap();
        for case in ["work", "buffer", "ledger", "metadata"] {
            let mut less = exact;
            match case {
                "work" => less.work -= 1,
                "buffer" => less.buffer_bytes -= 1,
                "ledger" => less.ledger_bytes -= 1,
                "metadata" => less.metadata_bytes -= 1,
                _ => unreachable!(),
            }
            assert!(
                fixture_with(source, less, GraphLimits::default()).is_err(),
                "accepted one-less {case} for {source}"
            );
        }
        // Final admission includes all late graph rows and the non-power-of-two
        // owned-node census; buy its complete measured work, then one less.
        let shape = crate::run::source_domain::admit_publication(
            &f.log,
            crate::run::source_domain::ShapeLimits::default(),
        )
        .unwrap_or_else(|_| panic!("shape"));
        let admitted = f.catalog.admit(&shape, &f.roots, &f.paths).unwrap();
        let total = f.ledger.validate_core(&admitted).unwrap();
        let mut f = f;
        f.ledger.limits.work = total;
        validate(&f, &f.log).unwrap();
        f.ledger.limits.work -= 1;
        assert!(validate(&f, &f.log).is_err());
    }
}
