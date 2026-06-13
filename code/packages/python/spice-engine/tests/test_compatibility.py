from spice_engine import (
    CompatibilityDeck,
    CompatibilityGoldenValue,
    CompatibilityOracle,
    analyze_deck_controls,
    compatibility_corpus,
    format_compatibility_corpus_table,
    format_release_readiness_report,
    release_readiness_gates,
    resolve_deck_functions,
    resolve_deck_initial_conditions,
    resolve_deck_measurements,
    resolve_deck_parameters,
    resolve_deck_sources,
)


def test_compatibility_corpus_release_gates_pass() -> None:
    corpus = compatibility_corpus()

    assert [deck.id for deck in corpus] == [
        "dc-op-resistive-divider",
        "dc-sweep-resistive-divider",
        "ac-rc-lowpass",
        "tran-rc-step",
        "tf-resistive-divider",
    ]
    assert {deck.analysis for deck in corpus} >= {"op", "dc", "ac", "tran"}
    assert all(deck.known_incompatibilities for deck in corpus)
    assert all(".end" in deck.netlist.lower() for deck in corpus)

    report = release_readiness_gates(corpus)

    assert report.passed is True
    assert report.deck_count == 5
    assert report.issues == ()
    assert format_release_readiness_report(report).splitlines()[1] == (
        "true\t5\top,dc,ac,tran,tf\t0"
    )


def test_compatibility_corpus_table_is_stable() -> None:
    table = format_compatibility_corpus_table()

    assert table.splitlines()[0] == (
        "id\tanalysis\toracle\tgolden_values\tknown_incompatibilities"
    )
    assert "dc-op-resistive-divider\top\tclosed-form@divider-v1" in table
    assert "V(out)=5.000000e+00V" in table


def test_release_readiness_gates_report_malformed_decks() -> None:
    malformed = CompatibilityDeck(
        id="",
        title="Missing metadata",
        analysis="noise",
        netlist="V1 in 0 DC 1",
        oracle=CompatibilityOracle(reference="", version="", source=""),
        golden_values=(
            CompatibilityGoldenValue("V(out)", float("inf"), "V", -1.0, 0.0),
        ),
        known_incompatibilities=(),
    )

    report = release_readiness_gates([malformed])

    assert report.passed is False
    fields = {issue.field for issue in report.issues}
    assert {
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
    }.issubset(fields)


def test_analyze_deck_controls_stops_at_end() -> None:
    summary = analyze_deck_controls(
        """
* ignored title
V1 in 0 DC 1
.op
.end
.include after-end.lib
.dc V1 0 1 1
"""
    )

    assert summary.terminated is True
    assert summary.end_line_number == 5
    assert summary.active_lines == ("V1 in 0 DC 1", ".op")
    assert summary.diagnostics == ()


def test_analyze_deck_controls_reports_unsupported_directives() -> None:
    summary = analyze_deck_controls(
        """
.include models.inc
.LIB vendor.lib TT
.control
run
.endc
.end
"""
    )

    assert summary.terminated is True
    assert summary.active_lines[:3] == (
        ".include models.inc",
        ".LIB vendor.lib TT",
        ".control",
    )
    assert [(diag.directive, diag.line_number, diag.severity) for diag in summary.diagnostics] == [
        (".include", 2, "error"),
        (".lib", 3, "error"),
        (".control", 4, "error"),
    ]
    assert all(diag.code == "SPICE_DECK_UNSUPPORTED_DIRECTIVE" for diag in summary.diagnostics)


def test_resolve_deck_sources_expands_include_and_library_section() -> None:
    summary = resolve_deck_sources(
        """
V1 in 0 DC 1
.include models.inc
.lib vendor.lib TT
.op
.end
Rafter out 0 1
""",
        {
            "models.inc": """
* model include
.model D1 D
Rshim in mid 10
""",
            "vendor.lib": """
.lib FF
Rfast out 0 1
.endl FF
.lib TT
Rtyp mid out 20
Ctyp out 0 1u
.endl TT
""",
        },
    )

    assert summary.terminated is True
    assert summary.end_line_number == 6
    assert summary.active_lines == (
        "V1 in 0 DC 1",
        ".model D1 D",
        "Rshim in mid 10",
        "Rtyp mid out 20",
        "Ctyp out 0 1u",
        ".op",
    )
    assert summary.included_paths == ("models.inc",)
    assert summary.library_sections == ("vendor.lib:TT",)
    assert summary.diagnostics == ()


def test_resolve_deck_sources_reports_missing_sources_and_cycles() -> None:
    summary = resolve_deck_sources(
        """
.include missing.inc
.include a.inc
.lib vendor.lib SS
.control
.end
""",
        {
            "a.inc": ".include b.inc\nR1 a b 1\n",
            "b.inc": ".include a.inc\nR2 b 0 2\n",
            "vendor.lib": ".lib TT\nRtyp out 0 20\n.endl TT\n",
        },
    )

    assert summary.active_lines == ("R2 b 0 2", "R1 a b 1", ".control")
    assert [diag.code for diag in summary.diagnostics] == [
        "SPICE_DECK_INCLUDE_NOT_FOUND",
        "SPICE_DECK_INCLUDE_CYCLE",
        "SPICE_DECK_LIB_SECTION_NOT_FOUND",
        "SPICE_DECK_UNSUPPORTED_DIRECTIVE",
    ]
    assert [(diag.source, diag.line_number, diag.target) for diag in summary.diagnostics[:3]] == [
        ("<deck>", 2, "missing.inc"),
        ("b.inc", 1, "a.inc"),
        ("<deck>", 4, "vendor.lib:SS"),
    ]


def test_resolve_deck_parameters_rewrites_braced_and_quoted_expressions() -> None:
    summary = resolve_deck_parameters(
        """
.param RLOAD=2k SCALE=3 TOTAL=RLOAD*SCALE
V1 in 0 DC {scale+1}
R1 in out {total}
C1 out 0 '2u*scale'
.op
.end
Rafter out 0 {total}
"""
    )

    assert summary.terminated is True
    assert summary.end_line_number == 7
    assert [(param.name, param.value) for param in summary.parameters] == [
        ("RLOAD", 2000.0),
        ("SCALE", 3.0),
        ("TOTAL", 6000.0),
    ]
    assert summary.active_lines == (
        "V1 in 0 DC 4",
        "R1 in out 6000",
        "C1 out 0 0.000006",
        ".op",
    )
    assert summary.diagnostics == ()


def test_resolve_deck_parameters_evaluates_func_calls() -> None:
    summary = resolve_deck_parameters(
        """
.func gain(x) {x*2}
.param BASE=2 SCALE=3 SHIFT=1 TOTAL=blend(base,scale,shift)
.func blend(a,b,c) 'gain(a)+b+c'
R1 in out {gain(total)}
B1 out 0 V='blend(1,2,3)'
.op
.end
"""
    )

    assert summary.terminated is True
    assert summary.end_line_number == 8
    assert summary.active_lines == (
        "R1 in out 16",
        "B1 out 0 V=7",
        ".op",
    )
    assert [(param.name, param.value) for param in summary.parameters] == [
        ("BASE", 2.0),
        ("SCALE", 3.0),
        ("SHIFT", 1.0),
        ("TOTAL", 8.0),
    ]
    assert summary.diagnostics == ()


def test_resolve_deck_parameters_reports_bad_func_calls() -> None:
    summary = resolve_deck_parameters(
        """
.func one(x) {x+1}
.func loop(x) {loop(x)}
.param GOOD=one(1) BAD=unknown(1) ARITY=one(1,2) RECUR=loop(1)
R1 in out {bad}
R2 out 0 {good}
.end
"""
    )

    assert summary.active_lines == (
        "R1 in out {bad}",
        "R2 out 0 2",
    )
    assert [(param.name, param.value) for param in summary.parameters] == [("GOOD", 2.0)]
    assert [diag.code for diag in summary.diagnostics] == [
        "SPICE_DECK_PARAM_EXPRESSION",
        "SPICE_DECK_PARAM_EXPRESSION",
        "SPICE_DECK_PARAM_EXPRESSION",
        "SPICE_DECK_PARAM_UNRESOLVED",
    ]
    assert [diag.parameter for diag in summary.diagnostics[:3]] == ["BAD", "ARITY", "RECUR"]
    assert [diag.expression for diag in summary.diagnostics] == [
        "unknown(1)",
        "one(1,2)",
        "loop(1)",
        "bad",
    ]


def test_resolve_deck_initial_conditions_extracts_ic_and_nodeset_hints() -> None:
    summary = resolve_deck_initial_conditions(
        """
V1 in 0 DC 1
.ic V(out)=1.2 V(mid)='2.5'
.nodeset V(bias)={700m}
.op
.end
.ic V(after)=9
"""
    )

    assert summary.terminated is True
    assert summary.end_line_number == 6
    assert summary.active_lines == ("V1 in 0 DC 1", ".op")
    assert [
        (condition.directive, condition.node, round(condition.value, 12), condition.line_number)
        for condition in summary.initial_conditions
    ] == [
        (".ic", "out", 1.2, 3),
        (".ic", "mid", 2.5, 3),
    ]
    assert [
        (condition.directive, condition.node, round(condition.value, 12), condition.line_number)
        for condition in summary.nodesets
    ] == [(".nodeset", "bias", 0.7, 4)]
    assert summary.diagnostics == ()


def test_resolve_deck_initial_conditions_reports_bad_assignments() -> None:
    summary = resolve_deck_initial_conditions(
        """
.ic out=1 V()=2 V(ok)=bad V(good)=1k
.nodeset
.nodeset I(L1)=2
.end
"""
    )

    assert summary.terminated is True
    assert summary.end_line_number == 5
    assert summary.active_lines == ()
    assert [
        (condition.directive, condition.node, condition.value, condition.line_number)
        for condition in summary.initial_conditions
    ] == [(".ic", "good", 1000.0, 2)]
    assert summary.nodesets == ()
    assert [diagnostic.code for diagnostic in summary.diagnostics] == [
        "SPICE_DECK_CONDITION_TARGET",
        "SPICE_DECK_CONDITION_TARGET",
        "SPICE_DECK_CONDITION_EXPRESSION",
        "SPICE_DECK_CONDITION_ARGUMENT",
        "SPICE_DECK_CONDITION_TARGET",
    ]
    assert [diagnostic.directive for diagnostic in summary.diagnostics] == [
        ".ic",
        ".ic",
        ".ic",
        ".nodeset",
        ".nodeset",
    ]


def test_resolve_deck_functions_extracts_function_definitions() -> None:
    summary = resolve_deck_functions(
        """
R1 in out {gain(vin)}
.func gain(x) {x*2}
.func blend(a,b,weight) 'a*(1-weight)+b*weight'
.op
.end
.func after(x) {x}
"""
    )

    assert summary.terminated is True
    assert summary.end_line_number == 6
    assert summary.active_lines == ("R1 in out {gain(vin)}", ".op")
    assert [
        (function.name, function.arguments, function.expression, function.line_number)
        for function in summary.functions
    ] == [
        ("gain", ("x",), "x*2", 3),
        ("blend", ("a", "b", "weight"), "a*(1-weight)+b*weight", 4),
    ]
    assert summary.diagnostics == ()


def test_resolve_deck_functions_reports_bad_definitions() -> None:
    summary = resolve_deck_functions(
        """
.func
.func 1bad(x) {x}
.func noexpr(x)
.func badarg(1x,x) {x}
.func dup(x,x) {x}
.end
"""
    )

    assert summary.terminated is True
    assert summary.end_line_number == 7
    assert summary.active_lines == ()
    assert summary.functions == ()
    assert [diagnostic.code for diagnostic in summary.diagnostics] == [
        "SPICE_DECK_FUNC_ARGUMENT",
        "SPICE_DECK_FUNC_SIGNATURE",
        "SPICE_DECK_FUNC_EXPRESSION",
        "SPICE_DECK_FUNC_ARGUMENT",
        "SPICE_DECK_FUNC_ARGUMENT",
    ]
    assert [diagnostic.function_name for diagnostic in summary.diagnostics] == [
        None,
        "1bad",
        "noexpr",
        "badarg",
        "dup",
    ]


def test_resolve_deck_measurements_extracts_transient_cards() -> None:
    summary = resolve_deck_measurements(
        """
V1 in 0 DC 1
.measure tran swing peak-to-peak V(out) FROM=1m TO={3m}
.meas transient settled FINAL V(out)
.end
.measure tran ignored MAX V(out)
"""
    )

    assert summary.active_lines == ("V1 in 0 DC 1",)
    assert summary.terminated is True
    assert summary.end_line_number == 5
    assert summary.diagnostics == ()
    assert [(card.name, card.analysis, card.mode, card.probe) for card in summary.measurements] == [
        ("swing", "tran", "pp", "V(out)"),
        ("settled", "transient", "last", "V(out)"),
    ]
    assert summary.measurements[0].from_value == 1.0e-3
    assert summary.measurements[0].to_value == 3.0e-3


def test_resolve_deck_measurements_reports_unsupported_subset() -> None:
    summary = resolve_deck_measurements(
        """
.measure ac gain MAX V(out)
.measure tran badmode MEDIAN V(out)
.measure tran badwindow MAX V(out) FROM=3m TO=1m
.measure tran badoption MAX V(out) RISE=1
.measure tran badvalue MAX V(out) FROM={unknown}
.end
"""
    )

    assert summary.measurements == ()
    assert {diagnostic.code for diagnostic in summary.diagnostics} == {
        "SPICE_DECK_MEASURE_ANALYSIS",
        "SPICE_DECK_MEASURE_MODE",
        "SPICE_DECK_MEASURE_WINDOW",
        "SPICE_DECK_MEASURE_ARGUMENT",
        "SPICE_DECK_MEASURE_EXPRESSION",
    }
