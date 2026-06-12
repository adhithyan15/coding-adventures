use std::collections::HashMap;

use spice_engine::{
    analyze_deck_controls, compatibility_corpus, format_compatibility_corpus_table,
    format_release_readiness_report, release_readiness_gates, resolve_deck_functions,
    resolve_deck_initial_conditions, resolve_deck_parameters, resolve_deck_sources,
    CompatibilityDeck, CompatibilityGoldenValue, CompatibilityOracle,
};

#[test]
fn compatibility_corpus_release_gates_pass() {
    let corpus = compatibility_corpus();

    assert_eq!(
        corpus
            .iter()
            .map(|deck| deck.id.as_str())
            .collect::<Vec<_>>(),
        vec![
            "dc-op-resistive-divider",
            "dc-sweep-resistive-divider",
            "ac-rc-lowpass",
            "tran-rc-step",
            "tf-resistive-divider"
        ]
    );
    assert!(["op", "dc", "ac", "tran"]
        .iter()
        .all(|analysis| corpus.iter().any(|deck| deck.analysis == *analysis)));
    assert!(corpus
        .iter()
        .all(|deck| deck.netlist.to_ascii_lowercase().contains(".end")));
    assert!(corpus
        .iter()
        .all(|deck| !deck.known_incompatibilities.is_empty()));

    let report = release_readiness_gates(&corpus);

    assert!(report.passed);
    assert_eq!(report.deck_count, 5);
    assert!(report.issues.is_empty());
    assert_eq!(
        format_release_readiness_report(&report).lines().nth(1),
        Some("true\t5\top,dc,ac,tran,tf\t0")
    );
}

#[test]
fn compatibility_corpus_table_is_stable() {
    let corpus = compatibility_corpus();
    let table = format_compatibility_corpus_table(&corpus);

    assert_eq!(
        table.lines().next(),
        Some("id\tanalysis\toracle\tgolden_values\tknown_incompatibilities")
    );
    assert!(table.contains("dc-op-resistive-divider\top\tclosed-form@divider-v1"));
    assert!(table.contains("V(out)=5.000000e+00V"));
}

#[test]
fn release_readiness_gates_report_malformed_decks() {
    let malformed = CompatibilityDeck {
        id: String::new(),
        title: "Missing metadata".to_string(),
        analysis: "noise".to_string(),
        netlist: "V1 in 0 DC 1".to_string(),
        oracle: CompatibilityOracle::new("", "", ""),
        golden_values: vec![CompatibilityGoldenValue::new(
            "V(out)",
            f64::INFINITY,
            "V",
            -1.0,
            0.0,
        )],
        known_incompatibilities: Vec::new(),
    };

    let report = release_readiness_gates(&[malformed]);
    let fields = report
        .issues
        .iter()
        .map(|issue| issue.field.as_str())
        .collect::<std::collections::HashSet<_>>();

    assert!(!report.passed);
    for field in [
        "id",
        "analysis",
        "netlist",
        "oracle.reference",
        "oracle.version",
        "oracle.source",
        "golden_values[0].value",
        "golden_values[0].tolerance",
        "known_incompatibilities",
        "analysis_coverage",
    ] {
        assert!(fields.contains(field), "missing field {field}");
    }
}

#[test]
fn analyze_deck_controls_stops_at_end() {
    let summary = analyze_deck_controls(
        "
* ignored title
V1 in 0 DC 1
.op
.end
.include after-end.lib
.dc V1 0 1 1
",
    );

    assert!(summary.terminated);
    assert_eq!(summary.end_line_number, Some(5));
    assert_eq!(summary.active_lines, vec!["V1 in 0 DC 1", ".op"]);
    assert!(summary.diagnostics.is_empty());
}

#[test]
fn analyze_deck_controls_reports_unsupported_directives() {
    let summary = analyze_deck_controls(
        "
.include models.inc
.LIB vendor.lib TT
.control
run
.endc
.end
",
    );

    assert!(summary.terminated);
    assert_eq!(
        &summary.active_lines[..3],
        &[
            ".include models.inc".to_string(),
            ".LIB vendor.lib TT".to_string(),
            ".control".to_string()
        ]
    );
    let diagnostics = summary
        .diagnostics
        .iter()
        .map(|diagnostic| {
            (
                diagnostic.directive.as_str(),
                diagnostic.line_number,
                diagnostic.severity.as_str(),
            )
        })
        .collect::<Vec<_>>();
    assert_eq!(
        diagnostics,
        vec![
            (".include", 2, "error"),
            (".lib", 3, "error"),
            (".control", 4, "error")
        ]
    );
    assert!(summary
        .diagnostics
        .iter()
        .all(|diagnostic| diagnostic.code == "SPICE_DECK_UNSUPPORTED_DIRECTIVE"));
}

#[test]
fn resolve_deck_sources_expands_include_and_library_section() {
    let mut sources = HashMap::new();
    sources.insert(
        "models.inc".to_string(),
        "
* model include
.model D1 D
Rshim in mid 10
"
        .to_string(),
    );
    sources.insert(
        "vendor.lib".to_string(),
        "
.lib FF
Rfast out 0 1
.endl FF
.lib TT
Rtyp mid out 20
Ctyp out 0 1u
.endl TT
"
        .to_string(),
    );

    let summary = resolve_deck_sources(
        "
V1 in 0 DC 1
.include models.inc
.lib vendor.lib TT
.op
.end
Rafter out 0 1
",
        &sources,
    );

    assert!(summary.terminated);
    assert_eq!(summary.end_line_number, Some(6));
    assert_eq!(
        summary.active_lines,
        vec![
            "V1 in 0 DC 1",
            ".model D1 D",
            "Rshim in mid 10",
            "Rtyp mid out 20",
            "Ctyp out 0 1u",
            ".op",
        ]
    );
    assert_eq!(summary.included_paths, vec!["models.inc"]);
    assert_eq!(summary.library_sections, vec!["vendor.lib:TT"]);
    assert!(summary.diagnostics.is_empty());
}

#[test]
fn resolve_deck_sources_reports_missing_sources_and_cycles() {
    let mut sources = HashMap::new();
    sources.insert(
        "a.inc".to_string(),
        ".include b.inc\nR1 a b 1\n".to_string(),
    );
    sources.insert(
        "b.inc".to_string(),
        ".include a.inc\nR2 b 0 2\n".to_string(),
    );
    sources.insert(
        "vendor.lib".to_string(),
        ".lib TT\nRtyp out 0 20\n.endl TT\n".to_string(),
    );

    let summary = resolve_deck_sources(
        "
.include missing.inc
.include a.inc
.lib vendor.lib SS
.control
.end
",
        &sources,
    );

    assert_eq!(
        summary.active_lines,
        vec!["R2 b 0 2", "R1 a b 1", ".control"]
    );
    assert_eq!(
        summary
            .diagnostics
            .iter()
            .map(|diagnostic| diagnostic.code.as_str())
            .collect::<Vec<_>>(),
        vec![
            "SPICE_DECK_INCLUDE_NOT_FOUND",
            "SPICE_DECK_INCLUDE_CYCLE",
            "SPICE_DECK_LIB_SECTION_NOT_FOUND",
            "SPICE_DECK_UNSUPPORTED_DIRECTIVE"
        ]
    );
    assert_eq!(
        summary
            .diagnostics
            .iter()
            .take(3)
            .map(|diagnostic| {
                (
                    diagnostic.source.as_str(),
                    diagnostic.line_number,
                    diagnostic.target.as_deref(),
                )
            })
            .collect::<Vec<_>>(),
        vec![
            ("<deck>", 2, Some("missing.inc")),
            ("b.inc", 1, Some("a.inc")),
            ("<deck>", 4, Some("vendor.lib:SS")),
        ]
    );
}

#[test]
fn resolve_deck_parameters_rewrites_braced_and_quoted_expressions() {
    let summary = resolve_deck_parameters(
        "
.param RLOAD=2k SCALE=3 TOTAL=RLOAD*SCALE
V1 in 0 DC {scale+1}
R1 in out {total}
C1 out 0 '2u*scale'
.op
.end
Rafter out 0 {total}
",
    );

    assert!(summary.terminated);
    assert_eq!(summary.end_line_number, Some(7));
    assert_eq!(
        summary
            .parameters
            .iter()
            .map(|parameter| (parameter.name.as_str(), parameter.value))
            .collect::<Vec<_>>(),
        vec![("RLOAD", 2000.0), ("SCALE", 3.0), ("TOTAL", 6000.0)]
    );
    assert_eq!(
        summary.active_lines,
        vec!["V1 in 0 DC 4", "R1 in out 6000", "C1 out 0 0.000006", ".op",]
    );
    assert!(summary.diagnostics.is_empty());
}

#[test]
fn resolve_deck_parameters_reports_unresolved_and_unsupported_func() {
    let summary = resolve_deck_parameters(
        "
.param GOOD=1k BAD=missing+1
.func gain(x) {x*2}
R1 in out {bad}
R2 out 0 {good}
.end
",
    );

    assert!(summary.terminated);
    assert_eq!(
        summary.active_lines,
        vec![".func gain(x) {x*2}", "R1 in out {bad}", "R2 out 0 1000"]
    );
    assert_eq!(
        summary
            .diagnostics
            .iter()
            .map(|diagnostic| diagnostic.code.as_str())
            .collect::<Vec<_>>(),
        vec![
            "SPICE_DECK_PARAM_EXPRESSION",
            "SPICE_DECK_UNSUPPORTED_DIRECTIVE",
            "SPICE_DECK_PARAM_UNRESOLVED"
        ]
    );
}

#[test]
fn resolve_deck_initial_conditions_extracts_ic_and_nodeset_hints() {
    let summary = resolve_deck_initial_conditions(
        "
V1 in 0 DC 1
.ic V(out)=1.2 V(mid)='2.5'
.nodeset V(bias)={700m}
.op
.end
.ic V(after)=9
",
    );

    assert!(summary.terminated);
    assert_eq!(summary.end_line_number, Some(6));
    assert_eq!(summary.active_lines, vec!["V1 in 0 DC 1", ".op"]);
    assert_eq!(
        summary
            .initial_conditions
            .iter()
            .map(|condition| (
                condition.directive.as_str(),
                condition.node.as_str(),
                condition.value,
                condition.line_number,
            ))
            .collect::<Vec<_>>(),
        vec![(".ic", "out", 1.2, 3), (".ic", "mid", 2.5, 3)]
    );
    assert_eq!(summary.nodesets.len(), 1);
    assert_eq!(summary.nodesets[0].directive, ".nodeset");
    assert_eq!(summary.nodesets[0].node, "bias");
    assert_eq!(summary.nodesets[0].line_number, 4);
    assert!((summary.nodesets[0].value - 0.7).abs() < 1.0e-12);
    assert!(summary.diagnostics.is_empty());
}

#[test]
fn resolve_deck_initial_conditions_reports_bad_assignments() {
    let summary = resolve_deck_initial_conditions(
        "
.ic out=1 V()=2 V(ok)=bad V(good)=1k
.nodeset
.nodeset I(L1)=2
.end
",
    );

    assert!(summary.terminated);
    assert_eq!(summary.end_line_number, Some(5));
    assert!(summary.active_lines.is_empty());
    assert_eq!(
        summary
            .initial_conditions
            .iter()
            .map(|condition| (
                condition.directive.as_str(),
                condition.node.as_str(),
                condition.value,
                condition.line_number,
            ))
            .collect::<Vec<_>>(),
        vec![(".ic", "good", 1000.0, 2)]
    );
    assert!(summary.nodesets.is_empty());
    assert_eq!(
        summary
            .diagnostics
            .iter()
            .map(|diagnostic| diagnostic.code.as_str())
            .collect::<Vec<_>>(),
        vec![
            "SPICE_DECK_CONDITION_TARGET",
            "SPICE_DECK_CONDITION_TARGET",
            "SPICE_DECK_CONDITION_EXPRESSION",
            "SPICE_DECK_CONDITION_ARGUMENT",
            "SPICE_DECK_CONDITION_TARGET",
        ]
    );
    assert_eq!(
        summary
            .diagnostics
            .iter()
            .map(|diagnostic| diagnostic.directive.as_str())
            .collect::<Vec<_>>(),
        vec![".ic", ".ic", ".ic", ".nodeset", ".nodeset"]
    );
}

#[test]
fn resolve_deck_functions_extracts_function_definitions() {
    let summary = resolve_deck_functions(
        "
R1 in out {gain(vin)}
.func gain(x) {x*2}
.func blend(a,b,weight) 'a*(1-weight)+b*weight'
.op
.end
.func after(x) {x}
",
    );

    assert!(summary.terminated);
    assert_eq!(summary.end_line_number, Some(6));
    assert_eq!(summary.active_lines, vec!["R1 in out {gain(vin)}", ".op"]);
    assert_eq!(
        summary
            .functions
            .iter()
            .map(|function| (
                function.name.as_str(),
                function
                    .arguments
                    .iter()
                    .map(String::as_str)
                    .collect::<Vec<_>>(),
                function.expression.as_str(),
                function.line_number,
            ))
            .collect::<Vec<_>>(),
        vec![
            ("gain", vec!["x"], "x*2", 3),
            (
                "blend",
                vec!["a", "b", "weight"],
                "a*(1-weight)+b*weight",
                4
            )
        ]
    );
    assert!(summary.diagnostics.is_empty());
}

#[test]
fn resolve_deck_functions_reports_bad_definitions() {
    let summary = resolve_deck_functions(
        "
.func
.func 1bad(x) {x}
.func noexpr(x)
.func badarg(1x,x) {x}
.func dup(x,x) {x}
.end
",
    );

    assert!(summary.terminated);
    assert_eq!(summary.end_line_number, Some(7));
    assert!(summary.active_lines.is_empty());
    assert!(summary.functions.is_empty());
    assert_eq!(
        summary
            .diagnostics
            .iter()
            .map(|diagnostic| diagnostic.code.as_str())
            .collect::<Vec<_>>(),
        vec![
            "SPICE_DECK_FUNC_ARGUMENT",
            "SPICE_DECK_FUNC_SIGNATURE",
            "SPICE_DECK_FUNC_EXPRESSION",
            "SPICE_DECK_FUNC_ARGUMENT",
            "SPICE_DECK_FUNC_ARGUMENT",
        ]
    );
    assert_eq!(
        summary
            .diagnostics
            .iter()
            .map(|diagnostic| diagnostic.function_name.as_deref())
            .collect::<Vec<_>>(),
        vec![
            None,
            Some("1bad"),
            Some("noexpr"),
            Some("badarg"),
            Some("dup")
        ]
    );
}
