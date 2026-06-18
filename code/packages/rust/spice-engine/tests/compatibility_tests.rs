use std::collections::HashMap;

use spice_engine::{
    analyze_deck_controls, compatibility_corpus, format_compatibility_corpus_table,
    format_release_readiness_report, release_readiness_gates, resolve_deck_analyses,
    resolve_deck_fourier, resolve_deck_functions, resolve_deck_initial_conditions,
    resolve_deck_measurements, resolve_deck_outputs, resolve_deck_parameters, resolve_deck_sources,
    select_deck_analysis_plan, select_deck_output_probes, CompatibilityDeck,
    CompatibilityGoldenValue, CompatibilityOracle,
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
op
save V(in)
probe V(out)
print op V(in)
measure tran vmax MAX V(out)
meas dc imax MAX I(V1)
fourier 1k V(out)
four 2k V(in)
reset
set noaskquit
set filetype=ascii
set wr_vecnames
set wr_singlescale
set appendwrite
set filetype=binary
write out.raw V(out)
wrdata out.dat V(out)
wrdata empty.dat
display all
listing physical
run
quit
.endc
.end
",
    );

    assert!(summary.terminated);
    assert_eq!(
        summary.active_lines,
        &[
            ".include models.inc".to_string(),
            ".LIB vendor.lib TT".to_string(),
            ".op".to_string(),
            ".save V(in)".to_string(),
            ".probe V(out)".to_string(),
            ".print op V(in)".to_string(),
            ".measure tran vmax MAX V(out)".to_string(),
            ".meas dc imax MAX I(V1)".to_string(),
            ".four 1k V(out)".to_string(),
            ".four 2k V(in)".to_string(),
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
            (".control", 4, "error"),
            (".control", 19, "error"),
            (".control", 22, "error")
        ]
    );
    assert_eq!(
        summary
            .diagnostics
            .iter()
            .map(|diagnostic| diagnostic.code.as_str())
            .collect::<Vec<_>>(),
        vec![
            "SPICE_DECK_UNSUPPORTED_DIRECTIVE",
            "SPICE_DECK_UNSUPPORTED_DIRECTIVE",
            "SPICE_DECK_UNSUPPORTED_DIRECTIVE",
            "SPICE_DECK_CONTROL_COMMAND",
            "SPICE_DECK_CONTROL_COMMAND"
        ]
    );
    let measurement_deck = format!("{}\n.end", summary.active_lines.join("\n"));
    let measurement_summary = resolve_deck_measurements(&measurement_deck);
    assert_eq!(
        measurement_summary
            .measurements
            .iter()
            .map(|card| (
                card.directive.as_str(),
                card.analysis.as_str(),
                card.name.as_str(),
                card.mode.as_str(),
                card.probe.as_str()
            ))
            .collect::<Vec<_>>(),
        vec![
            (".measure", "tran", "vmax", "max", "V(out)"),
            (".meas", "dc", "imax", "max", "I(V1)")
        ]
    );
    let fourier_deck = format!("{}\n.end", summary.active_lines.join("\n"));
    let fourier_summary = resolve_deck_fourier(&fourier_deck);
    assert_eq!(
        fourier_summary
            .fourier
            .iter()
            .map(|card| (
                card.directive.as_str(),
                card.fundamental_frequency_hz,
                card.probes
                    .iter()
                    .map(|probe| probe.as_str())
                    .collect::<Vec<_>>()
            ))
            .collect::<Vec<_>>(),
        vec![
            (".four", 1000.0, vec!["V(out)"]),
            (".four", 2000.0, vec!["V(in)"])
        ]
    );
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
op
save V(a)
probe V(b)
print op V(a)
measure tran vmax MAX V(a)
meas dc imax MAX I(V1)
fourier 1k V(a)
four 2k V(b)
.reset
.set noaskquit
.set filetype=ascii
.set wr_vecnames
.set wr_singlescale
.set appendwrite
.write out.raw V(a)
.wrdata out.dat V(a)
.display all
.listing deck
run
.quit
.endc
.end
",
        &sources,
    );

    assert!(summary.terminated);
    assert_eq!(
        summary.active_lines,
        vec![
            "R2 b 0 2",
            "R1 a b 1",
            ".op",
            ".save V(a)",
            ".probe V(b)",
            ".print op V(a)",
            ".measure tran vmax MAX V(a)",
            ".meas dc imax MAX I(V1)",
            ".four 1k V(a)",
            ".four 2k V(b)"
        ]
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
            .skip(3)
            .map(|diagnostic| (diagnostic.directive.as_str(), diagnostic.line_number))
            .collect::<Vec<_>>(),
        vec![(".control", 5)]
    );
    let measurement_deck = format!("{}\n.end", summary.active_lines.join("\n"));
    let measurement_summary = resolve_deck_measurements(&measurement_deck);
    assert_eq!(
        measurement_summary
            .measurements
            .iter()
            .map(|card| (
                card.directive.as_str(),
                card.analysis.as_str(),
                card.name.as_str(),
                card.mode.as_str(),
                card.probe.as_str()
            ))
            .collect::<Vec<_>>(),
        vec![
            (".measure", "tran", "vmax", "max", "V(a)"),
            (".meas", "dc", "imax", "max", "I(V1)")
        ]
    );
    let fourier_deck = format!("{}\n.end", summary.active_lines.join("\n"));
    let fourier_summary = resolve_deck_fourier(&fourier_deck);
    assert_eq!(
        fourier_summary
            .fourier
            .iter()
            .map(|card| (
                card.directive.as_str(),
                card.fundamental_frequency_hz,
                card.probes
                    .iter()
                    .map(|probe| probe.as_str())
                    .collect::<Vec<_>>()
            ))
            .collect::<Vec<_>>(),
        vec![
            (".four", 1000.0, vec!["V(a)"]),
            (".four", 2000.0, vec!["V(b)"])
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
fn resolve_deck_parameters_evaluates_func_calls() {
    let summary = resolve_deck_parameters(
        "
.func gain(x) {x*2}
.param BASE=2 SCALE=3 SHIFT=1 TOTAL=blend(base,scale,shift)
.func blend(a,b,c) 'gain(a)+b+c'
R1 in out {gain(total)}
B1 out 0 V='blend(1,2,3)'
.op
.end
",
    );

    assert!(summary.terminated);
    assert_eq!(summary.end_line_number, Some(8));
    assert_eq!(
        summary.active_lines,
        vec!["R1 in out 16", "B1 out 0 V=7", ".op"]
    );
    assert_eq!(
        summary
            .parameters
            .iter()
            .map(|parameter| (parameter.name.as_str(), parameter.value))
            .collect::<Vec<_>>(),
        vec![
            ("BASE", 2.0),
            ("SCALE", 3.0),
            ("SHIFT", 1.0),
            ("TOTAL", 8.0)
        ]
    );
    assert!(summary.diagnostics.is_empty());
}

#[test]
fn resolve_deck_parameters_reports_bad_func_calls() {
    let summary = resolve_deck_parameters(
        "
.func one(x) {x+1}
.func loop(x) {loop(x)}
.param GOOD=one(1) BAD=unknown(1) ARITY=one(1,2) RECUR=loop(1)
R1 in out {bad}
R2 out 0 {good}
.end
",
    );

    assert_eq!(summary.active_lines, vec!["R1 in out {bad}", "R2 out 0 2"]);
    assert_eq!(
        summary
            .parameters
            .iter()
            .map(|parameter| (parameter.name.as_str(), parameter.value))
            .collect::<Vec<_>>(),
        vec![("GOOD", 2.0)]
    );
    assert_eq!(
        summary
            .diagnostics
            .iter()
            .map(|diagnostic| diagnostic.code.as_str())
            .collect::<Vec<_>>(),
        vec![
            "SPICE_DECK_PARAM_EXPRESSION",
            "SPICE_DECK_PARAM_EXPRESSION",
            "SPICE_DECK_PARAM_EXPRESSION",
            "SPICE_DECK_PARAM_UNRESOLVED"
        ]
    );
    assert_eq!(
        summary
            .diagnostics
            .iter()
            .take(3)
            .map(|diagnostic| diagnostic.parameter.as_deref())
            .collect::<Vec<_>>(),
        vec![Some("BAD"), Some("ARITY"), Some("RECUR")]
    );
    assert_eq!(
        summary
            .diagnostics
            .iter()
            .map(|diagnostic| diagnostic.expression.as_deref())
            .collect::<Vec<_>>(),
        vec![
            Some("unknown(1)"),
            Some("one(1,2)"),
            Some("loop(1)"),
            Some("bad")
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

#[test]
fn resolve_deck_measurements_extracts_transient_cards() {
    let summary = resolve_deck_measurements(
        "
V1 in 0 DC 1
.measure tran swing peak-to-peak V(out) FROM=1m TO={3m}
.meas transient settled FINAL V(out)
.measure tran sample FIND V(out) AT={1.5m}
.measure tran crossing WHEN V(out)=0.5 FROM=1m TO=3m RISE=1
.measure tran prop_delay TRIG V(in) VAL=0.5 RISE=1 TARG V(out) VAL=0.5 FALL=1 FROM=0 TO=4m
.measure dc dcmax MAX V(out) FROM=1 TO=3
.measure ac acmax MAX V(out) FROM=1k TO=10k
.end
.measure tran ignored MAX V(out)
",
    );

    assert_eq!(summary.active_lines, vec!["V1 in 0 DC 1"]);
    assert!(summary.terminated);
    assert_eq!(summary.end_line_number, Some(10));
    assert!(summary.diagnostics.is_empty());
    assert_eq!(
        summary
            .measurements
            .iter()
            .map(|card| (
                card.name.as_str(),
                card.analysis.as_str(),
                card.mode.as_str(),
                card.probe.as_str()
            ))
            .collect::<Vec<_>>(),
        vec![
            ("swing", "tran", "pp", "V(out)"),
            ("settled", "transient", "last", "V(out)"),
            ("sample", "tran", "find", "V(out)"),
            ("crossing", "tran", "when", "V(out)"),
            ("prop_delay", "tran", "delay", "V(out)"),
            ("dcmax", "dc", "max", "V(out)"),
            ("acmax", "ac", "max", "V(out)")
        ]
    );
    assert!((summary.measurements[0].from_value.unwrap() - 1.0e-3).abs() < 1.0e-12);
    assert!((summary.measurements[0].to_value.unwrap() - 3.0e-3).abs() < 1.0e-12);
    assert!((summary.measurements[2].at_value.unwrap() - 1.5e-3).abs() < 1.0e-12);
    assert!((summary.measurements[3].target_value.unwrap() - 0.5).abs() < 1.0e-12);
    assert!((summary.measurements[3].from_value.unwrap() - 1.0e-3).abs() < 1.0e-12);
    assert!((summary.measurements[3].to_value.unwrap() - 3.0e-3).abs() < 1.0e-12);
    assert_eq!(
        summary.measurements[3].crossing_kind.as_deref(),
        Some("rise")
    );
    assert_eq!(summary.measurements[3].crossing_count, Some(1));
    assert!((summary.measurements[4].target_value.unwrap() - 0.5).abs() < 1.0e-12);
    assert_eq!(
        summary.measurements[4].crossing_kind.as_deref(),
        Some("fall")
    );
    assert_eq!(summary.measurements[4].crossing_count, Some(1));
    assert_eq!(
        summary.measurements[4].trigger_probe.as_deref(),
        Some("V(in)")
    );
    assert!((summary.measurements[4].trigger_value.unwrap() - 0.5).abs() < 1.0e-12);
    assert_eq!(
        summary.measurements[4].trigger_crossing_kind.as_deref(),
        Some("rise")
    );
    assert_eq!(summary.measurements[4].trigger_crossing_count, Some(1));
    assert!((summary.measurements[4].from_value.unwrap() - 0.0).abs() < 1.0e-12);
    assert!((summary.measurements[4].to_value.unwrap() - 4.0e-3).abs() < 1.0e-12);
    assert!((summary.measurements[5].from_value.unwrap() - 1.0).abs() < 1.0e-12);
    assert!((summary.measurements[5].to_value.unwrap() - 3.0).abs() < 1.0e-12);
    assert!((summary.measurements[6].from_value.unwrap() - 1.0e3).abs() < 1.0e-9);
    assert!((summary.measurements[6].to_value.unwrap() - 1.0e4).abs() < 1.0e-9);
}

#[test]
fn resolve_deck_measurements_reports_unsupported_subset() {
    let summary = resolve_deck_measurements(
        "
.measure tf gain MAX V(out)
.measure tran badmode MEDIAN V(out)
.measure tran badwindow MAX V(out) FROM=3m TO=1m
.measure tran badoption MAX V(out) RISE=1
.measure tran badvalue MAX V(out) FROM={unknown}
.end
",
    );

    assert!(summary.measurements.is_empty());
    let mut codes = summary
        .diagnostics
        .iter()
        .map(|diagnostic| diagnostic.code.as_str())
        .collect::<Vec<_>>();
    codes.sort_unstable();
    assert_eq!(
        codes,
        vec![
            "SPICE_DECK_MEASURE_ANALYSIS",
            "SPICE_DECK_MEASURE_ARGUMENT",
            "SPICE_DECK_MEASURE_EXPRESSION",
            "SPICE_DECK_MEASURE_MODE",
            "SPICE_DECK_MEASURE_WINDOW",
        ]
    );
}

#[test]
fn resolve_deck_fourier_extracts_transient_cards() {
    let summary = resolve_deck_fourier(
        "
V1 in 0 SIN(0 1 1k)
.tran 1u 2m
.four {1k} V(in) V(out) HARMONICS=5 FROM=1m
.four 2k \"I(V1)\"
.end
.four 3k V(ignored)
",
    );

    assert_eq!(
        summary.active_lines,
        vec!["V1 in 0 SIN(0 1 1k)", ".tran 1u 2m"]
    );
    assert!(summary.terminated);
    assert_eq!(summary.end_line_number, Some(6));
    assert!(summary.diagnostics.is_empty());
    assert_eq!(summary.fourier.len(), 2);
    assert!((summary.fourier[0].fundamental_frequency_hz - 1000.0).abs() < 1.0e-12);
    assert_eq!(summary.fourier[0].probes, vec!["V(in)", "V(out)"]);
    assert_eq!(summary.fourier[0].harmonics, Some(5));
    assert!((summary.fourier[0].from_value.unwrap() - 1.0e-3).abs() < 1.0e-12);
    assert_eq!(summary.fourier[1].probes, vec!["I(V1)"]);
    assert_eq!(summary.fourier[1].harmonics, None);
}

#[test]
fn resolve_deck_fourier_reports_unsupported_subset() {
    let summary = resolve_deck_fourier(
        "
.four 0 V(out)
.four 1k
.four 1k V(out) HARMONICS=1.5
.four 1k V(out) TO=2m
.four 1k \"\"
.end
",
    );

    assert!(summary.fourier.is_empty());
    let mut codes = summary
        .diagnostics
        .iter()
        .map(|diagnostic| diagnostic.code.as_str())
        .collect::<Vec<_>>();
    codes.sort_unstable();
    assert_eq!(
        codes,
        vec![
            "SPICE_DECK_FOURIER_ARGUMENT",
            "SPICE_DECK_FOURIER_ARGUMENT",
            "SPICE_DECK_FOURIER_ARGUMENT",
            "SPICE_DECK_FOURIER_FREQUENCY",
            "SPICE_DECK_FOURIER_PROBE",
        ]
    );
}

#[test]
fn resolve_deck_outputs_extracts_save_probe_print_and_plot_cards() {
    let summary = resolve_deck_outputs(
        "
V1 in 0 DC 1
.save V(out) i(V1)
.probe tran V(clk)
.probe AC V(out)
.print dc V(load) I(V2)
.plot ac I(V3)
.end
.save V(ignored)
",
    );

    assert_eq!(summary.active_lines, vec!["V1 in 0 DC 1"]);
    assert!(summary.terminated);
    assert_eq!(summary.end_line_number, Some(8));
    assert!(summary.diagnostics.is_empty());
    assert_eq!(
        summary
            .selections
            .iter()
            .map(|selection| (
                selection.directive.as_str(),
                selection.analysis.as_deref(),
                selection.probes.as_slice()
            ))
            .collect::<Vec<_>>(),
        vec![
            (
                ".save",
                None,
                &["V(out)".to_string(), "I(V1)".to_string()][..]
            ),
            (".probe", Some("tran"), &["V(clk)".to_string()][..]),
            (".probe", Some("ac"), &["V(out)".to_string()][..]),
            (
                ".print",
                Some("dc"),
                &["V(load)".to_string(), "I(V2)".to_string()][..]
            ),
            (".plot", Some("ac"), &["I(V3)".to_string()][..]),
        ]
    );

    assert_eq!(
        select_deck_output_probes(
            "
.save V(out) I(V1)
.probe tran V(out) V(clk)
.print tran I(V2)
.plot tran V(extra)
.probe ac V(freq)
.end
",
            "transient",
        )
        .unwrap(),
        vec!["V(out)", "I(V1)", "V(clk)", "I(V2)", "V(extra)"]
    );
}

#[test]
fn resolve_deck_analyses_extracts_supported_cards() {
    let summary = resolve_deck_analyses(
        "
V1 in 0 DC 0
R1 in out 1k
.op
.dc V1 0 5 1
.ac dec 10 1k 1Meg
.tran 1u 2m 0 10u uic
.end
.tran 1u 1m
",
    );

    assert_eq!(
        summary.active_lines,
        vec!["V1 in 0 DC 0".to_string(), "R1 in out 1k".to_string()]
    );
    assert!(summary.terminated);
    assert_eq!(summary.end_line_number, Some(8));
    assert!(summary.diagnostics.is_empty());
    assert_eq!(
        summary
            .analyses
            .iter()
            .map(|analysis| analysis.analysis.as_str())
            .collect::<Vec<_>>(),
        vec!["op", "dc", "ac", "tran"]
    );

    let dc = &summary.analyses[1];
    assert_eq!(dc.directive, ".dc");
    assert_eq!(dc.source_name.as_deref(), Some("V1"));
    assert!((dc.start_value.unwrap() - 0.0).abs() < 1.0e-12);
    assert!((dc.stop_value.unwrap() - 5.0).abs() < 1.0e-12);
    assert!((dc.step_value.unwrap() - 1.0).abs() < 1.0e-12);

    let ac = &summary.analyses[2];
    assert_eq!(ac.directive, ".ac");
    assert_eq!(ac.sweep_kind.as_deref(), Some("dec"));
    assert_eq!(ac.point_count, Some(10));
    assert!((ac.start_frequency_hz.unwrap() - 1.0e3).abs() < 1.0e-9);
    assert!((ac.stop_frequency_hz.unwrap() - 1.0e6).abs() < 1.0e-6);

    let tran = &summary.analyses[3];
    assert_eq!(tran.directive, ".tran");
    assert!((tran.step_time.unwrap() - 1.0e-6).abs() < 1.0e-12);
    assert!((tran.stop_time.unwrap() - 2.0e-3).abs() < 1.0e-12);
    assert!((tran.start_time.unwrap() - 0.0).abs() < 1.0e-12);
    assert!((tran.max_step.unwrap() - 1.0e-5).abs() < 1.0e-12);
    assert!(tran.use_initial_conditions);
}

#[test]
fn resolve_deck_analyses_reports_invalid_cards() {
    let summary = resolve_deck_analyses(
        "
.op extra
.dc V1 0 1 0
.dc V1 1 0 1
.ac decade 10 1 10
.ac lin 0 1 10
.tran 0 1m
.tran 1u 2m 0 1u extra
.end
",
    );

    assert!(summary.analyses.is_empty());
    let mut codes = summary
        .diagnostics
        .iter()
        .map(|diagnostic| diagnostic.code.as_str())
        .collect::<Vec<_>>();
    codes.sort_unstable();
    assert_eq!(
        codes,
        vec![
            "SPICE_DECK_ANALYSIS_ARGUMENT",
            "SPICE_DECK_ANALYSIS_ARGUMENT",
            "SPICE_DECK_ANALYSIS_INTERVAL",
            "SPICE_DECK_ANALYSIS_MODE",
            "SPICE_DECK_ANALYSIS_SWEEP",
            "SPICE_DECK_ANALYSIS_SWEEP",
            "SPICE_DECK_ANALYSIS_SWEEP",
        ]
    );
}

#[test]
fn select_deck_analysis_plan_defaults_and_selects() {
    let implicit = select_deck_analysis_plan(
        "
V1 in 0 DC 1
R1 in 0 1k
.end
",
        None,
    )
    .unwrap();
    assert_eq!(implicit.directive, ".op");
    assert_eq!(implicit.analysis, "op");
    assert_eq!(implicit.line_number, 0);

    let selected = select_deck_analysis_plan(
        "
V1 in 0 DC 0
.dc V1 0 5 1
.tran 1u 2m
.end
",
        Some("transient"),
    )
    .unwrap();
    assert_eq!(selected.directive, ".tran");
    assert_eq!(selected.analysis, "tran");
    assert_eq!(selected.line_number, 4);
    assert!((selected.stop_time.unwrap() - 2.0e-3).abs() < 1.0e-12);
}

#[test]
fn select_deck_analysis_plan_reports_ambiguous_or_invalid_selection() {
    let error = select_deck_analysis_plan(
        "
.dc V1 0 5 1
.tran 1u 2m
.end
",
        None,
    )
    .unwrap_err()
    .to_string();
    assert!(error.contains("multiple analysis cards"));

    let error = select_deck_analysis_plan(
        "
.tran 1u 2m
.tran 2u 4m
.end
",
        Some(".tran"),
    )
    .unwrap_err()
    .to_string();
    assert!(error.contains("multiple .tran analysis cards"));

    let error = select_deck_analysis_plan(".op\n.end\n", Some("noise"))
        .unwrap_err()
        .to_string();
    assert!(error.contains("unsupported analysis"));

    let error = select_deck_analysis_plan(
        "
.dc V1 0 1 0
.end
",
        None,
    )
    .unwrap_err()
    .to_string();
    assert!(error.contains("line 2: .dc step value must be non-zero"));
}

#[test]
fn resolve_deck_outputs_reports_invalid_cards() {
    let summary = resolve_deck_outputs(
        "
.save
.probe tran
.print tran
.print foo V(out)
.plot tran
.plot foo V(out)
.save P(out)
.probe dc V(out) bad-token
.print dc bad-token
.plot dc bad-token
.end
",
    );

    let mut codes = summary
        .diagnostics
        .iter()
        .map(|diagnostic| diagnostic.code.as_str())
        .collect::<Vec<_>>();
    codes.sort_unstable();
    assert_eq!(
        codes,
        vec![
            "SPICE_DECK_OUTPUT_ANALYSIS",
            "SPICE_DECK_OUTPUT_ANALYSIS",
            "SPICE_DECK_OUTPUT_ARGUMENT",
            "SPICE_DECK_OUTPUT_ARGUMENT",
            "SPICE_DECK_OUTPUT_ARGUMENT",
            "SPICE_DECK_OUTPUT_ARGUMENT",
            "SPICE_DECK_OUTPUT_PROBE",
            "SPICE_DECK_OUTPUT_PROBE",
            "SPICE_DECK_OUTPUT_PROBE",
            "SPICE_DECK_OUTPUT_PROBE",
        ]
    );
    assert_eq!(summary.selections[0].probes, vec!["V(out)".to_string()]);
}
