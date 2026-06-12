from spice_engine import (
    CompatibilityDeck,
    CompatibilityGoldenValue,
    CompatibilityOracle,
    analyze_deck_controls,
    compatibility_corpus,
    format_compatibility_corpus_table,
    format_release_readiness_report,
    release_readiness_gates,
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
