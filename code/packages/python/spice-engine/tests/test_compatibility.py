import pytest

from spice_engine import (
    CompatibilityDeck,
    CompatibilityGoldenValue,
    CompatibilityOracle,
    analyze_deck_controls,
    compatibility_corpus,
    format_compatibility_corpus_table,
    format_release_readiness_report,
    release_readiness_gates,
    resolve_deck_analyses,
    resolve_deck_fourier,
    resolve_deck_functions,
    resolve_deck_initial_conditions,
    resolve_deck_measurements,
    resolve_deck_outputs,
    resolve_deck_parameters,
    resolve_deck_sources,
    select_deck_analysis_plan,
    select_deck_output_probes,
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
.measure tran sample FIND V(out) AT={1.5m}
.measure tran crossing WHEN V(out)=0.5 FROM=1m TO=3m RISE=1
.measure tran prop_delay TRIG V(in) VAL=0.5 RISE=1 TARG V(out) VAL=0.5 FALL=1 FROM=0 TO=4m
.measure dc dcmax MAX V(out) FROM=1 TO=3
.measure ac acmax MAX V(out) FROM=1k TO=10k
.end
.measure tran ignored MAX V(out)
"""
    )

    assert summary.active_lines == ("V1 in 0 DC 1",)
    assert summary.terminated is True
    assert summary.end_line_number == 10
    assert summary.diagnostics == ()
    assert [(card.name, card.analysis, card.mode, card.probe) for card in summary.measurements] == [
        ("swing", "tran", "pp", "V(out)"),
        ("settled", "transient", "last", "V(out)"),
        ("sample", "tran", "find", "V(out)"),
        ("crossing", "tran", "when", "V(out)"),
        ("prop_delay", "tran", "delay", "V(out)"),
        ("dcmax", "dc", "max", "V(out)"),
        ("acmax", "ac", "max", "V(out)"),
    ]
    assert summary.measurements[0].from_value == 1.0e-3
    assert summary.measurements[0].to_value == 3.0e-3
    assert summary.measurements[2].at_value == 1.5e-3
    assert summary.measurements[3].target_value == 0.5
    assert summary.measurements[3].from_value == 1.0e-3
    assert summary.measurements[3].to_value == 3.0e-3
    assert summary.measurements[3].crossing_kind == "rise"
    assert summary.measurements[3].crossing_count == 1
    assert summary.measurements[4].target_value == 0.5
    assert summary.measurements[4].crossing_kind == "fall"
    assert summary.measurements[4].crossing_count == 1
    assert summary.measurements[4].trigger_probe == "V(in)"
    assert summary.measurements[4].trigger_value == 0.5
    assert summary.measurements[4].trigger_crossing_kind == "rise"
    assert summary.measurements[4].trigger_crossing_count == 1
    assert summary.measurements[4].from_value == 0.0
    assert summary.measurements[4].to_value == 4.0e-3
    assert summary.measurements[5].from_value == 1.0
    assert summary.measurements[5].to_value == 3.0
    assert summary.measurements[6].from_value == 1.0e3
    assert summary.measurements[6].to_value == 1.0e4


def test_resolve_deck_measurements_reports_unsupported_subset() -> None:
    summary = resolve_deck_measurements(
        """
.measure tf gain MAX V(out)
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


def test_resolve_deck_fourier_extracts_transient_cards() -> None:
    summary = resolve_deck_fourier(
        """
V1 in 0 SIN(0 1 1k)
.tran 1u 2m
.four {1k} V(in) V(out) HARMONICS=5 FROM=1m
.four 2k "I(V1)"
.end
.four 3k V(ignored)
"""
    )

    assert summary.active_lines == ("V1 in 0 SIN(0 1 1k)", ".tran 1u 2m")
    assert summary.terminated is True
    assert summary.end_line_number == 6
    assert summary.diagnostics == ()
    assert len(summary.fourier) == 2
    assert summary.fourier[0].fundamental_frequency == pytest.approx(1000.0)
    assert summary.fourier[0].probes == ("V(in)", "V(out)")
    assert summary.fourier[0].harmonics == 5
    assert summary.fourier[0].from_value == pytest.approx(1.0e-3)
    assert summary.fourier[1].probes == ("I(V1)",)
    assert summary.fourier[1].harmonics is None


def test_resolve_deck_fourier_reports_unsupported_subset() -> None:
    summary = resolve_deck_fourier(
        """
.four 0 V(out)
.four 1k
.four 1k V(out) HARMONICS=1.5
.four 1k V(out) TO=2m
.four 1k ""
.end
"""
    )

    assert summary.fourier == ()
    assert {diagnostic.code for diagnostic in summary.diagnostics} == {
        "SPICE_DECK_FOURIER_ARGUMENT",
        "SPICE_DECK_FOURIER_FREQUENCY",
        "SPICE_DECK_FOURIER_PROBE",
    }


def test_resolve_deck_outputs_extracts_save_probe_and_print_cards() -> None:
    summary = resolve_deck_outputs(
        """
V1 in 0 DC 1
.save V(out) i(V1)
.probe tran V(clk)
.probe AC V(out)
.print dc V(load) I(V2)
.end
.save V(ignored)
"""
    )

    assert summary.active_lines == ("V1 in 0 DC 1",)
    assert summary.terminated is True
    assert summary.end_line_number == 7
    assert summary.diagnostics == ()
    assert [
        (selection.directive, selection.analysis, selection.probes)
        for selection in summary.selections
    ] == [
        (".save", None, ("V(out)", "I(V1)")),
        (".probe", "tran", ("V(clk)",)),
        (".probe", "ac", ("V(out)",)),
        (".print", "dc", ("V(load)", "I(V2)")),
    ]

    assert select_deck_output_probes(
        """
.save V(out) I(V1)
.probe tran V(out) V(clk)
.print tran I(V2)
.probe ac V(freq)
.end
""",
        "transient",
    ) == ["V(out)", "I(V1)", "V(clk)", "I(V2)"]


def test_resolve_deck_analyses_extracts_supported_cards() -> None:
    summary = resolve_deck_analyses(
        """
V1 in 0 DC 0
R1 in out 1k
.op
.dc V1 0 5 1
.ac dec 10 1k 1Meg
.tran 1u 2m 0 10u uic
.end
.tran 1u 1m
"""
    )

    assert summary.active_lines == ("V1 in 0 DC 0", "R1 in out 1k")
    assert summary.terminated is True
    assert summary.end_line_number == 8
    assert summary.diagnostics == ()
    assert [analysis.analysis for analysis in summary.analyses] == [
        "op",
        "dc",
        "ac",
        "tran",
    ]

    dc = summary.analyses[1]
    assert dc.directive == ".dc"
    assert dc.source_name == "V1"
    assert dc.start_value == pytest.approx(0.0)
    assert dc.stop_value == pytest.approx(5.0)
    assert dc.step_value == pytest.approx(1.0)

    ac = summary.analyses[2]
    assert ac.directive == ".ac"
    assert ac.sweep_kind == "dec"
    assert ac.point_count == 10
    assert ac.start_frequency == pytest.approx(1.0e3)
    assert ac.stop_frequency == pytest.approx(1.0e6)

    tran = summary.analyses[3]
    assert tran.directive == ".tran"
    assert tran.step_time == pytest.approx(1.0e-6)
    assert tran.stop_time == pytest.approx(2.0e-3)
    assert tran.start_time == pytest.approx(0.0)
    assert tran.max_step == pytest.approx(1.0e-5)
    assert tran.use_initial_conditions is True


def test_resolve_deck_analyses_reports_invalid_cards() -> None:
    summary = resolve_deck_analyses(
        """
.op extra
.dc V1 0 1 0
.dc V1 1 0 1
.ac decade 10 1 10
.ac lin 0 1 10
.tran 0 1m
.tran 1u 2m 0 1u extra
.end
"""
    )

    assert summary.analyses == ()
    assert sorted(diagnostic.code for diagnostic in summary.diagnostics) == [
        "SPICE_DECK_ANALYSIS_ARGUMENT",
        "SPICE_DECK_ANALYSIS_ARGUMENT",
        "SPICE_DECK_ANALYSIS_INTERVAL",
        "SPICE_DECK_ANALYSIS_MODE",
        "SPICE_DECK_ANALYSIS_SWEEP",
        "SPICE_DECK_ANALYSIS_SWEEP",
        "SPICE_DECK_ANALYSIS_SWEEP",
    ]


def test_select_deck_analysis_plan_defaults_and_selects() -> None:
    implicit = select_deck_analysis_plan(
        """
V1 in 0 DC 1
R1 in 0 1k
.end
"""
    )
    assert implicit.directive == ".op"
    assert implicit.analysis == "op"
    assert implicit.line_number == 0

    selected = select_deck_analysis_plan(
        """
V1 in 0 DC 0
.dc V1 0 5 1
.tran 1u 2m
.end
""",
        "transient",
    )
    assert selected.directive == ".tran"
    assert selected.analysis == "tran"
    assert selected.line_number == 4
    assert selected.stop_time == pytest.approx(2.0e-3)


def test_select_deck_analysis_plan_reports_ambiguous_or_invalid_selection() -> None:
    with pytest.raises(ValueError, match="multiple analysis cards"):
        select_deck_analysis_plan(
            """
.dc V1 0 5 1
.tran 1u 2m
.end
"""
        )

    with pytest.raises(ValueError, match=r"multiple \.tran analysis cards"):
        select_deck_analysis_plan(
            """
.tran 1u 2m
.tran 2u 4m
.end
""",
            ".tran",
        )

    with pytest.raises(ValueError, match="unsupported analysis"):
        select_deck_analysis_plan(".op\n.end\n", "noise")

    with pytest.raises(ValueError, match=r"line 2: \.dc step value must be non-zero"):
        select_deck_analysis_plan(
            """
.dc V1 0 1 0
.end
"""
        )


def test_resolve_deck_outputs_reports_invalid_cards() -> None:
    summary = resolve_deck_outputs(
        """
.save
.probe tran
.print tran
.print foo V(out)
.save P(out)
.probe dc V(out) bad-token
.print dc bad-token
.end
"""
    )

    assert sorted(diagnostic.code for diagnostic in summary.diagnostics) == [
        "SPICE_DECK_OUTPUT_ANALYSIS",
        "SPICE_DECK_OUTPUT_ARGUMENT",
        "SPICE_DECK_OUTPUT_ARGUMENT",
        "SPICE_DECK_OUTPUT_ARGUMENT",
        "SPICE_DECK_OUTPUT_PROBE",
        "SPICE_DECK_OUTPUT_PROBE",
        "SPICE_DECK_OUTPUT_PROBE",
    ]
    assert summary.selections[0].probes == ("V(out)",)

    with pytest.raises(ValueError, match=r"line 2: \.save requires"):
        select_deck_output_probes(
            """
.save
.end
""",
            "dc",
        )
