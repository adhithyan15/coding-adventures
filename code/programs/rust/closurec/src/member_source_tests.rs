//! CLOC35 private tests committed before implementation; wired with the adapter.
use super::*;
use crate::run::source_inventory::{
    record_file, stamp_ingestion_ids, SourceInventoryBudget, SourceInventoryLimits,
};
use crate::run::source_replay::{ReplayLimits, SourceCatalog};
use coding_adventures_correlation_vector::GraphLimits;
use coding_adventures_javascript_ast::{
    expression::Expression, statement::TaggedStatement, Program, ProgramItem, Statement,
};
use coding_adventures_javascript_parser::{
    asi::parse_with_asi,
    bridge::{grammar_to_program_with_composite_observer, CompositeOccurrenceObserver},
    DEFAULT_ES_VERSION,
};
use std::path::{Path, PathBuf};
struct Prepared {
    log: CVLog,
    catalog: SourceCatalog,
    root: String,
    node: GrammarASTNode,
}
fn prepare(source: &str, graph: GraphLimits) -> Result<Prepared, String> {
    let mut log = CVLog::new_checked_chronology(graph)?;
    let mut budget = SourceInventoryBudget::new(SourceInventoryLimits::default())?;
    let mut catalog = SourceCatalog::new(1, DEFAULT_ES_VERSION, ReplayLimits::default())?;
    let owned = source.to_owned();
    let input = record_file(
        &mut log,
        Path::new("member.js"),
        &owned,
        DEFAULT_ES_VERSION,
        &mut budget,
    )?;
    catalog.retain(
        owned,
        &input.id,
        input.tokens.as_ref().ok().map(Vec::as_slice),
    )?;
    let mut tokens = input.tokens?;
    log.contribute(
        &input.id,
        "lex",
        "tokens_emitted",
        HashMap::from([("token_count".into(), json!(tokens.len()))]),
    )?;
    stamp_ingestion_ids(&mut tokens, input.token_ids)?;
    let node = parse_with_asi(tokens, DEFAULT_ES_VERSION).map_err(|e| e.to_string())?;
    Ok(Prepared {
        log,
        catalog,
        root: input.id,
        node,
    })
}
struct Fixture {
    p: Prepared,
    ledger: ArrayLedger,
    program: Program,
}
fn fixture(source: &str, limits: ArrayLimits) -> Result<Fixture, String> {
    let mut p = prepare(source, GraphLimits::default())?;
    let mut ledger = ArrayLedger::new(limits)?;
    let mut observer = ledger.observer(&mut p.log, &p.root)?;
    let program =
        grammar_to_program_with_composite_observer(&p.node, DEFAULT_ES_VERSION, &mut observer)
            .map_err(|e| e.to_string())?;
    let mut pipeline = coding_adventures_closure_pass_pipeline::PassPipeline::new();
    pipeline.add(Box::new(
        coding_adventures_closure_pass_constant_fold::ConstantFoldPass::new(),
    ));
    let program = pipeline
        .run(
            program,
            &coding_adventures_type_sidecar::Sidecar::new(),
            &mut p.log,
        )
        .map_err(|e| e.to_string())?
        .program;
    Ok(Fixture { p, ledger, program })
}
fn validate(f: &Fixture, log: &CVLog) -> Result<usize, String> {
    let shape = crate::run::source_domain::admit_publication(
        log,
        crate::run::source_domain::ShapeLimits::default(),
    )
    .map_err(|_| "shape")?;
    let roots = [f.p.root.clone()];
    let paths = [PathBuf::from("member.js")];
    let admitted = f.p.catalog.admit(&shape, &roots, &paths)?;
    f.ledger.validate_core(&admitted)
}
fn members(f: &Fixture) -> Vec<String> {
    f.p.log
        .entries()
        .iter()
        .filter(|(_, e)| {
            e.origin
                .as_ref()
                .is_some_and(|o| o.source == "typed_member")
        })
        .map(|(id, _)| id.clone())
        .collect()
}
fn reachable(log: &CVLog, id: &str) -> std::collections::BTreeSet<String> {
    fn visit(log: &CVLog, id: &str, out: &mut std::collections::BTreeSet<String>) {
        if !out.insert(id.into()) {
            return;
        }
        for p in &log.get(id).unwrap().parent_ids {
            visit(log, p, out);
        }
    }
    let mut out = Default::default();
    visit(log, id, &mut out);
    out
}
#[test]
fn c35_actual_typed_stamped_replacements_own_their_fold_and_member_ancestry() {
    for (source, after) in [
        ("report([].length);", "0"),
        ("report([,,][\"length\"]);", "2"),
        ("report(\"😀\".length);", "2"),
        ("report([,][0]);", "void 0"),
        ("report([][4]);", "void 0"),
    ] {
        let f = fixture(source, ArrayLimits::default()).unwrap();
        validate(&f, &f.p.log).unwrap();
        let ProgramItem::Statement(Statement::Tagged(TaggedStatement::ExpressionStatement(s))) =
            &f.program.body[0]
        else {
            panic!("statement");
        };
        let Expression::CallExpression(call) = &s.expression else {
            panic!("call");
        };
        let id = match &call.arguments[0] {
            Expression::NumericLiteral(n) => n.cv.as_deref().unwrap(),
            Expression::UndefinedLiteral(n) => n.cv.as_deref().unwrap(),
            other => panic!("replacement {other:?}"),
        };
        let result = f.p.log.get(id).unwrap();
        assert!(result
            .contributions
            .iter()
            .any(|c| c.source == "constant-fold"
                && c.tag == "folded"
                && c.meta.get("new_cv") == Some(&json!(id))
                && c.meta.get("after") == Some(&json!(after))));
        let ids = members(&f);
        assert_eq!(ids.len(), 1);
        assert!(reachable(&f.p.log, id).contains(&ids[0]));
    }
}
#[test]
fn c35_closed_member_kinds_cannot_forge_namespace_origin_ranges_or_parent_order() {
    for source in ["a.b;", "a?.[[]];"] {
        let f = fixture(source, ArrayLimits::default()).unwrap();
        validate(&f, &f.p.log).unwrap();
        let id = members(&f).remove(0);
        let original: Value = serde_json::from_str(&f.p.log.to_json_string().unwrap()).unwrap();
        for case in [
            "namespace",
            "extra",
            "kind",
            "root",
            "first",
            "range",
            "parents",
            "origin",
            "location",
        ] {
            let mut value = original.clone();
            let e = &mut value["entries"][&id];
            match case {
                "namespace" => {
                    e["origin"]["meta"].as_object_mut().unwrap().remove(NS);
                }
                "extra" => e["origin"]["meta"][NS]["extra"] = json!(true),
                "kind" => e["origin"]["meta"][NS]["kind"] = json!("ArrayExpression"),
                "root" => e["origin"]["meta"][NS]["input_id"] = json!(id),
                "first" => {
                    e["origin"]["meta"][NS]["first_piece_id"] =
                        e["origin"]["meta"][NS]["last_piece_id"].clone()
                }
                "range" => {
                    let start = &mut e["origin"]["meta"][NS]["start"]["byte_offset"];
                    assert_ne!(
                        start,
                        &json!("1"),
                        "mutant must differ from the fixture baseline"
                    );
                    *start = json!("1");
                }
                "parents" => e["parent_ids"].as_array_mut().unwrap().reverse(),
                "origin" => e["origin"]["source"] = json!("ordinary"),
                "location" => e["origin"]["location"] = json!("invented"),
                _ => unreachable!(),
            }
            let log = CVLog::from_checked_json(
                &serde_json::to_string(&value).unwrap(),
                GraphLimits::default(),
            )
            .unwrap();
            assert!(validate(&f, &log).is_err(), "accepted {case} for {source}");
        }
    }
}
#[test]
fn c35_final_census_and_private_receipts_bind_actual_member_construction() {
    for source in ["a.b;", "a?.b;"] {
        for case in [
            "unreceipted",
            "bare-owned",
            "contribution",
            "deletion",
            "clock",
            "entity",
            "kind",
            "boundary-kind",
            "missing",
        ] {
            let mut f = fixture(source, ArrayLimits::default()).unwrap();
            let id = members(&f).remove(0);
            let row = f
                .ledger
                .receipts
                .iter()
                .position(|r| {
                    r.id == compact(
                        &id,
                        &mut Work {
                            used: 0,
                            limit: usize::MAX,
                        },
                    )
                    .unwrap()
                })
                .unwrap();
            match case {
                "unreceipted" | "bare-owned" => {
                    let mut origin = f.p.log.get(&id).unwrap().origin.clone().unwrap();
                    if case == "bare-owned" {
                        origin.meta.clear();
                    }
                    f.p.log.try_create(Some(origin)).unwrap();
                }
                "contribution" => {
                    f.p.log
                        .contribute(
                            &id,
                            "later",
                            "tamper",
                            HashMap::from([(NS.into(), json!({"schema":"1"}))]),
                        )
                        .unwrap();
                }
                "deletion" => {
                    f.p.log
                        .try_delete(
                            &id,
                            "later",
                            "tamper",
                            HashMap::from([(NS.into(), json!({"schema":"1"}))]),
                        )
                        .unwrap();
                }
                "clock" => f.ledger.receipts[row].merge_sequence += 1,
                "entity" => f.ledger.receipts[row].id += 1,
                "kind" => f.ledger.receipts[row].kind = NodeKind::Array,
                "boundary-kind" => f.ledger.receipts[row].first_kind = TokenType::LBracket,
                "missing" => {
                    f.ledger.receipts.remove(row);
                }
                _ => unreachable!(),
            }
            let before = f.p.log.to_json_string().unwrap();
            assert!(validate(&f, &f.p.log).is_err(), "accepted {case}");
            assert_eq!(f.p.log.to_json_string().unwrap(), before);
        }
    }
}
fn owner(node: &GrammarASTNode) -> Option<&GrammarASTNode> {
    if [
        "member_expression",
        "call_expression",
        "optional_chain_expression",
    ]
    .contains(&node.rule_name.as_str())
        && node.children.len() > 1
    {
        return Some(node);
    }
    node.children.iter().find_map(|c| match c {
        ASTNodeOrToken::Node(n) => owner(n),
        _ => None,
    })
}
fn first(node: &mut GrammarASTNode) -> &mut Token {
    match &mut node.children[0] {
        ASTNodeOrToken::Node(n) => first(n),
        ASTNodeOrToken::Token(t) => t,
    }
}
#[test]
fn c35_bad_private_member_evidence_refuses_before_any_graph_mutation() {
    for (source, computed, optional) in [("a.b;", false, false), ("a?.[b];", true, true)] {
        for case in [
            "missing",
            "foreign",
            "noncompact",
            "duplicate",
            "reversed",
            "kind",
            "cursor",
            "suffix",
            "flag",
            "owner",
            "empty-spine",
        ] {
            let mut p = prepare(source, GraphLimits::default()).unwrap();
            let mut node = owner(&p.node).unwrap().clone();
            let mut end = node.children.len() - 1;
            let start_cv = first(&mut node).cv.clone();
            let end_cv = match &node.children[end] {
                ASTNodeOrToken::Token(t) => t.cv.clone(),
                _ => panic!("end"),
            };
            let mut budget = SourceInventoryBudget::new(SourceInventoryLimits::default()).unwrap();
            let foreign = record_file(
                &mut p.log,
                Path::new("foreign.js"),
                "a.b;",
                DEFAULT_ES_VERSION,
                &mut budget,
            )
            .unwrap();
            match case {
                "missing" => first(&mut node).cv = None,
                "foreign" => first(&mut node).cv = Some(foreign.token_ids[0].clone()),
                "noncompact" => first(&mut node).cv = Some("not-compact".into()),
                "duplicate" => {
                    if let ASTNodeOrToken::Token(t) = &mut node.children[end] {
                        t.cv = start_cv;
                    }
                }
                "reversed" => {
                    first(&mut node).cv = end_cv;
                    if let ASTNodeOrToken::Token(t) = &mut node.children[end] {
                        t.cv = start_cv;
                    }
                }
                "kind" => first(&mut node).type_ = TokenType::LBrace,
                "cursor" => end = usize::MAX,
                "suffix" => {
                    if let ASTNodeOrToken::Token(t) = &mut node.children[1] {
                        t.value = "+".into();
                        t.type_ = TokenType::Plus;
                    }
                }
                "owner" => node.rule_name = "array_literal".into(),
                "empty-spine" => {
                    if let ASTNodeOrToken::Node(n) = &mut node.children[0] {
                        n.children.clear();
                    }
                }
                "flag" => {}
                _ => unreachable!(),
            }
            let before = p.log.to_json_string().unwrap();
            let mut ledger = ArrayLedger::new(ArrayLimits::default()).unwrap();
            let mut sink = ledger.observer(&mut p.log, &p.root).unwrap();
            assert!(
                sink.member_constructed(
                    &node,
                    end,
                    computed,
                    if case == "flag" { !optional } else { optional }
                )
                .is_err(),
                "accepted {case} {source}"
            );
            assert_eq!(p.log.to_json_string().unwrap(), before);
            assert!(ledger.receipts.is_empty());
        }
    }
}
#[test]
fn c35_optional_marker_kind_and_grammar_name_are_required_before_graph_mutation() {
    for (source, computed) in [("a?.b;", false), ("a?.[b];", true)] {
        for case in ["kind", "grammar-name", "missing-name"] {
            let mut p = prepare(source, GraphLimits::default()).unwrap();
            let mut node = owner(&p.node).unwrap().clone();
            let end = node.children.len() - 1;
            let ASTNodeOrToken::Token(marker) = &mut node.children[1] else {
                panic!("optional marker");
            };
            assert_eq!(marker.value, "?.");
            assert_eq!(marker.type_name.as_deref(), Some("OPTIONAL_CHAIN"));
            match case {
                "kind" => marker.type_ = TokenType::Number,
                "grammar-name" => marker.type_name = Some("NUMBER".into()),
                "missing-name" => marker.type_name = None,
                _ => unreachable!(),
            }
            let before = p.log.to_json_string().unwrap();
            let mut ledger = ArrayLedger::new(ArrayLimits::default()).unwrap();
            let mut sink = ledger.observer(&mut p.log, &p.root).unwrap();
            assert!(
                sink.member_constructed(&node, end, computed, true).is_err(),
                "accepted {case} {source}"
            );
            assert_eq!(p.log.to_json_string().unwrap(), before);
            assert!(ledger.receipts.is_empty());
        }
    }
}
#[test]
fn c35_exact_and_one_less_limits_include_mixed_nonpower_growth_and_final_census() {
    for source in ["a.b;", "[];a.b;a?.b;", "[];a.b;[];a?.[x];a[b];"] {
        let f = fixture(source, ArrayLimits::default()).unwrap();
        validate(&f, &f.p.log).unwrap();
        let ledger_bytes = f.ledger.receipts.capacity() * size_of::<Receipt>();
        // Capacity and interval maxima may occur in different callbacks. Buy
        // only the maximum simultaneous phase, including old/new ledger growth.
        let mut capacity = 0usize;
        let mut peak = 0usize;
        for (index, row) in f.ledger.receipts.iter().enumerate() {
            if index == capacity {
                let old = capacity;
                capacity = if old == 0 { 1 } else { old * 2 };
                peak = peak.max((old + capacity) * size_of::<Receipt>());
            }
            let count = row.last_index - row.first_index + 1;
            peak = peak.max(
                capacity * size_of::<Receipt>()
                    + count * (size_of::<String>() + 64 + size_of::<&str>())
                    + META_BYTES,
            );
        }
        assert_eq!(capacity, f.ledger.receipts.capacity());
        let exact = ArrayLimits {
            work: f.ledger.used_work,
            buffer_bytes: peak,
            ledger_bytes,
            metadata_bytes: f.ledger.receipts.len() * META_BYTES,
        };
        fixture(source, exact).unwrap();
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
                fixture(source, less).is_err(),
                "accepted one-less {case} {source}"
            );
        }
        let total = validate(&f, &f.p.log).unwrap();
        let mut f = f;
        f.ledger.limits.work = total;
        validate(&f, &f.p.log).unwrap();
        f.ledger.limits.work -= 1;
        assert!(validate(&f, &f.p.log).is_err());
        f.ledger.limits.work = f.ledger.used_work + 1;
        assert!(validate(&f, &f.p.log).is_err());
        f.ledger.limits.work = ArrayLimits::default().work;
        f.ledger.limits.buffer_bytes = 0;
        assert!(validate(&f, &f.p.log).is_err());
    }
}
#[test]
fn c35_original_checked_graph_node_and_edge_exhaustion_refuses_member_construction() {
    let prepared = prepare("a.b;", GraphLimits::default()).unwrap();
    let nodes = prepared.log.entries().len();
    let edges = prepared
        .log
        .entries()
        .values()
        .map(|e| e.parent_ids.len())
        .sum();
    for limits in [
        GraphLimits {
            max_nodes: nodes,
            ..Default::default()
        },
        GraphLimits {
            max_edges: edges,
            ..Default::default()
        },
    ] {
        let mut p = prepare("a.b;", limits).unwrap();
        let before = p.log.to_json_string().unwrap();
        let mut ledger = ArrayLedger::new(ArrayLimits::default()).unwrap();
        let mut sink = ledger.observer(&mut p.log, &p.root).unwrap();
        assert!(
            matches!(grammar_to_program_with_composite_observer(&p.node,DEFAULT_ES_VERSION,&mut sink),Err(coding_adventures_javascript_parser::bridge::ObservedBridgeError::Observer(s)) if s.contains("limit"))
        );
        assert_eq!(p.log.to_json_string().unwrap(), before);
        assert!(ledger.receipts.is_empty());
    }
}
#[test]
fn c35_member_refusals_preserve_all_preexisting_publication_destinations() {
    use crate::config::{CompilationLevel, CompilerConfig, CorrelationVectorFormat};
    let dir = std::env::temp_dir().join(format!("closurec-c35-pub-{}", std::process::id()));
    std::fs::create_dir(&dir).unwrap();
    let input = dir.join("input.js");
    std::fs::write(&input, "a.b;").unwrap();
    let paths = ["out.js", "out.map", "manifest.txt", "trace.json"].map(|p| dir.join(p));
    let spent = fixture("a.b;", ArrayLimits::default())
        .unwrap()
        .ledger
        .used_work;
    for level in [CompilationLevel::Simple, CompilationLevel::Advanced] {
        for format in [
            CorrelationVectorFormat::Json,
            CorrelationVectorFormat::Ndjson,
            CorrelationVectorFormat::None,
        ] {
            for filtered in [false, true] {
                for case in ["work", "buffer", "ledger", "metadata", "final"] {
                    for p in &paths {
                        std::fs::write(p, b"prior\0member sentinel").unwrap();
                    }
                    let mut c = CompilerConfig::default();
                    c.compilation.level = level;
                    c.io.js_patterns = vec![input.to_string_lossy().into_owned()];
                    c.io.js_output_file = Some(paths[0].clone());
                    c.source_map.path_template = paths[1].to_string_lossy().into_owned();
                    c.chunks.output_manifest_file = Some(paths[2].clone());
                    c.special_modes.correlation_vector = true;
                    c.special_modes.correlation_vector_format = format;
                    c.special_modes.correlation_vector_output = Some(paths[3].clone());
                    c.special_modes.correlation_vector_summary = true;
                    if filtered {
                        c.special_modes.correlation_vector_filter = vec!["hide-member".into()];
                    }
                    let mut limits = ArrayLimits::default();
                    match case {
                        "work" => limits.work = 0,
                        "buffer" => limits.buffer_bytes = 0,
                        "ledger" => limits.ledger_bytes = 0,
                        "metadata" => limits.metadata_bytes = 0,
                        "final" => limits.work = spent,
                        _ => unreachable!(),
                    }
                    let error = crate::run::run_compiler_with_array_limits(&c, limits).unwrap_err();
                    assert!(
                        matches!(error,crate::run::CompilerError::Provenance{ref stage,..} if stage==if case=="final" {"array_source_admission"} else {"array_source"}),
                        "{error:?}"
                    );
                    for p in &paths {
                        assert_eq!(std::fs::read(p).unwrap(), b"prior\0member sentinel");
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
fn c36_selected_merge_and_derive_refusals_preserve_all_publication_destinations() {
    use crate::config::{CompilationLevel, CompilerConfig, CorrelationVectorFormat};
    let dir = std::env::temp_dir().join(format!("closurec-c36-pub-{}", std::process::id()));
    std::fs::create_dir(&dir).unwrap();
    let input = dir.join("input.js");
    std::fs::write(&input, "report([7,9][0]);").unwrap();
    let paths = ["out.js", "out.map", "manifest.txt", "trace.json"].map(|name| dir.join(name));
    for level in [CompilationLevel::Simple, CompilationLevel::Advanced] {
        let mut c = CompilerConfig::default();
        c.compilation.level = level;
        c.io.js_patterns = vec![input.to_string_lossy().into_owned()];
        c.io.js_output_file = Some(paths[0].clone());
        c.source_map.path_template = paths[1].to_string_lossy().into_owned();
        c.chunks.output_manifest_file = Some(paths[2].clone());
        c.special_modes.correlation_vector = true;
        c.special_modes.correlation_vector_format = CorrelationVectorFormat::Json;
        c.special_modes.correlation_vector_output = Some(paths[3].clone());
        crate::run::run_compiler(&c).unwrap();
        let j: serde_json::Value =
            serde_json::from_slice(&std::fs::read(&paths[3]).unwrap()).unwrap();
        let entries = j["entries"].as_object().unwrap();
        let selected: Vec<_> = entries
            .iter()
            .filter(|(id, e)| {
                e["contributions"].as_array().unwrap().iter().any(|v| {
                    v["source"] == "constant-fold"
                        && v["tag"] == "folded"
                        && v["meta"]["before"] == "array-literal[2][0]"
                        && v["meta"]["after"] == "element[0]"
                        && v["meta"]["new_cv"] == id.as_str()
                })
            })
            .collect();
        assert_eq!(selected.len(), 1);
        let merge_id = selected[0].1["parent_ids"][0].as_str().unwrap();
        let events = j["journal"]["events"].as_array().unwrap();
        let index = events
            .iter()
            .position(|e| e["event"]["kind"] == "merge" && e["event"]["entity"] == merge_id)
            .unwrap();
        assert!(
            !events[index]["context"].is_null(),
            "selection's own pass merge"
        );
        let before_merge = events[..index]
            .iter()
            .filter(|e| {
                matches!(
                    e["event"]["kind"].as_str(),
                    Some("create" | "derive" | "merge")
                )
            })
            .count();
        for cap in [before_merge, before_merge + 1] {
            for format in [
                CorrelationVectorFormat::Json,
                CorrelationVectorFormat::Ndjson,
                CorrelationVectorFormat::None,
            ] {
                for filtered in [false, true] {
                    for p in &paths {
                        std::fs::write(p, b"prior\0selection sentinel").unwrap();
                    }
                    let mut refused = c.clone();
                    refused.special_modes.correlation_vector_limits.max_nodes = cap;
                    refused.special_modes.correlation_vector_format = format;
                    if filtered {
                        refused.special_modes.correlation_vector_filter =
                            vec!["hide-selection".into()];
                    }
                    let error = crate::run::run_compiler(&refused).unwrap_err();
                    assert!(
                        matches!(error, crate::run::CompilerError::TypedPipeline { .. }),
                        "{error:?}"
                    );
                    assert!(error.to_string().contains("constant-fold"), "{error:?}");
                    for p in &paths {
                        assert_eq!(std::fs::read(p).unwrap(), b"prior\0selection sentinel");
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
