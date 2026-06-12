from spice_engine import (
    CompatibilityDeck,
    CompatibilityGoldenValue,
    CompatibilityOracle,
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
