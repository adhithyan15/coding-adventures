use spice_engine::{
    compatibility_corpus, format_compatibility_corpus_table, format_release_readiness_report,
    release_readiness_gates, CompatibilityDeck, CompatibilityGoldenValue, CompatibilityOracle,
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
