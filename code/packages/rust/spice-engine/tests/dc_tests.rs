use spice_engine::{
    analyze_custom_model_source, bjt_at_temperature, bjt_from_model_card, circuit_at_temperature,
    dc_corners, dc_corners_parallel, dc_initial_vector_from_conditions, dc_op,
    dc_op_with_initial_conditions, dc_op_with_options, dc_sweep, dc_sweep_corners,
    dc_sweep_corners_parallel, dc_temperature_sweep, dc_temperature_sweep_corners,
    device_model_audit_fixtures, device_model_behavior_audit_fixtures,
    device_model_reference_deck_audit_analysis_summary,
    device_model_reference_deck_audit_analysis_summary_records,
    device_model_reference_deck_audit_fixtures, device_model_reference_deck_audit_gate,
    device_model_reference_deck_audit_gate_coverage_digest,
    device_model_reference_deck_audit_gate_coverage_digest_records,
    device_model_reference_deck_audit_gate_issue_records,
    device_model_reference_deck_audit_gate_issue_summary,
    device_model_reference_deck_audit_gate_issue_summary_records,
    device_model_reference_deck_audit_matrix, device_model_reference_deck_audit_matrix_records,
    device_model_reference_deck_audit_records, device_model_reference_deck_audit_summary,
    device_model_reference_deck_audit_summary_records, device_model_temperature_audit_fixtures,
    diode_at_temperature, diode_from_model_card, format_corner_dc_sweep_table,
    format_corner_dc_table, format_corner_temperature_dc_table, format_dc_sweep_table,
    format_device_model_reference_deck_audit_analysis_summary_csv,
    format_device_model_reference_deck_audit_analysis_summary_json,
    format_device_model_reference_deck_audit_analysis_summary_table,
    format_device_model_reference_deck_audit_csv,
    format_device_model_reference_deck_audit_gate_coverage_digest_csv,
    format_device_model_reference_deck_audit_gate_coverage_digest_json,
    format_device_model_reference_deck_audit_gate_coverage_digest_table,
    format_device_model_reference_deck_audit_gate_issue_csv,
    format_device_model_reference_deck_audit_gate_issue_json,
    format_device_model_reference_deck_audit_gate_issue_summary_csv,
    format_device_model_reference_deck_audit_gate_issue_summary_json,
    format_device_model_reference_deck_audit_gate_issue_summary_table,
    format_device_model_reference_deck_audit_gate_issue_table,
    format_device_model_reference_deck_audit_gate_report,
    format_device_model_reference_deck_audit_json,
    format_device_model_reference_deck_audit_matrix_csv,
    format_device_model_reference_deck_audit_matrix_json,
    format_device_model_reference_deck_audit_matrix_table,
    format_device_model_reference_deck_audit_summary_csv,
    format_device_model_reference_deck_audit_summary_json,
    format_device_model_reference_deck_audit_summary_table,
    format_device_model_reference_deck_audit_table, format_measurement_table,
    format_model_card_supported_parameter_coverage_csv,
    format_model_card_supported_parameter_coverage_dashboard_csv,
    format_model_card_supported_parameter_coverage_dashboard_json,
    format_model_card_supported_parameter_coverage_dashboard_table,
    format_model_card_supported_parameter_coverage_gate_issue_csv,
    format_model_card_supported_parameter_coverage_gate_issue_json,
    format_model_card_supported_parameter_coverage_gate_issue_table,
    format_model_card_supported_parameter_coverage_gate_report,
    format_model_card_supported_parameter_coverage_json,
    format_model_card_supported_parameter_coverage_summary_csv,
    format_model_card_supported_parameter_coverage_summary_json,
    format_model_card_supported_parameter_coverage_summary_table,
    format_model_card_supported_parameter_coverage_table, format_temperature_dc_table,
    jfet_at_temperature, jfet_from_model_card, measure_dc_sweep_deck, measure_dc_sweep_probe,
    model_card_supported_parameter_coverage, model_card_supported_parameter_coverage_dashboard,
    model_card_supported_parameter_coverage_dashboard_records,
    model_card_supported_parameter_coverage_gate,
    model_card_supported_parameter_coverage_gate_issue_records,
    model_card_supported_parameter_coverage_records,
    model_card_supported_parameter_coverage_summary,
    model_card_supported_parameter_coverage_summary_records, mosfet_at_temperature,
    mosfet_from_model_card, normalize_model_card, normalize_model_card_type,
    resolve_deck_initial_conditions, BSource, Bjt, BjtPolarity, Cccs, Ccvs, Circuit,
    CornerOverride, CornerSpec, CornerTemperatureDcResult, CurrentSource, CustomModel,
    DcConvergenceAid, DcOpOptions, Diode, Element, Inductor, Jfet, JfetPolarity, ModelCardKind,
    Mosfet, MosfetLevel1Params, MosfetType, Resistor, SinWaveform, SpiceError,
    SubcircuitDefinition, SubcircuitElement, TemperatureDcResult, Vccs, Vcvs, VoltageSource,
    Waveform, XInstance,
};
use std::collections::{BTreeMap, BTreeSet};

fn assert_close(actual: f64, expected: f64) {
    assert!(
        (actual - expected).abs() < 1.0e-9,
        "expected {expected}, got {actual}"
    );
}

#[test]
fn model_card_type_aliases_are_normalized() {
    assert_eq!(
        normalize_model_card_type("diode").unwrap(),
        ModelCardKind::Diode
    );
    assert_eq!(
        normalize_model_card_type("n-jfet").unwrap(),
        ModelCardKind::Njf
    );
    assert_eq!(
        normalize_model_card_type("pch").unwrap(),
        ModelCardKind::Pmos
    );
}

#[test]
fn model_card_supported_parameter_coverage_exports_are_stable() {
    let coverage = model_card_supported_parameter_coverage();
    assert_eq!(coverage.len(), 207);
    assert_eq!(coverage[0].kind, ModelCardKind::Diode);
    assert_eq!(coverage[0].canonical_parameter, "IS");
    assert_eq!(coverage[0].accepted_names, vec!["IS", "JS"]);
    assert_eq!(coverage[0].alias_count, 2);
    assert_eq!(coverage.last().unwrap().kind, ModelCardKind::Pmos);
    assert_eq!(coverage.last().unwrap().canonical_parameter, "AF");
    assert_eq!(coverage.last().unwrap().accepted_names, vec!["AF"]);

    let table = format_model_card_supported_parameter_coverage_table();
    let lines = table.lines().collect::<Vec<_>>();
    assert_eq!(
        lines[0],
        "kind\tcanonical_parameter\taccepted_names\talias_count"
    );
    assert_eq!(lines[1], "D\tIS\tIS|JS\t2");
    assert!(table.contains("NMOS\tVT0\tVT0|VTO|VTH\t3"));
    assert_eq!(lines.last().unwrap(), &"PMOS\tAF\tAF\t1");
    let records = model_card_supported_parameter_coverage_records();
    assert_eq!(records.len(), 207);
    assert_eq!(records[0]["kind"], "D");
    assert_eq!(records[0]["canonical_parameter"], "IS");
    assert_eq!(records[0]["accepted_names"], "IS|JS");
    assert_eq!(records[0]["alias_count"], "2");
    assert!(format_model_card_supported_parameter_coverage_csv()
        .starts_with("kind,canonical_parameter,accepted_names,alias_count\nD,IS,IS|JS,2\n"));
    let json = format_model_card_supported_parameter_coverage_json();
    assert!(json.starts_with(
        "[{\"kind\":\"D\",\"canonical_parameter\":\"IS\",\"accepted_names\":\"IS|JS\",\"alias_count\":\"2\"}"
    ));
    assert!(json.contains(
        "{\"kind\":\"NMOS\",\"canonical_parameter\":\"VT0\",\"accepted_names\":\"VT0|VTO|VTH\",\"alias_count\":\"3\"}"
    ));
    assert!(json.ends_with(
        "{\"kind\":\"PMOS\",\"canonical_parameter\":\"AF\",\"accepted_names\":\"AF\",\"alias_count\":\"1\"}]\n"
    ));
}

#[test]
fn model_card_supported_parameter_coverage_summary_exports_are_stable() {
    let summary = model_card_supported_parameter_coverage_summary();
    assert_eq!(summary.len(), 7);
    assert_eq!(summary[0].kind, ModelCardKind::Diode);
    assert_eq!(summary[0].canonical_parameter_count, 15);
    assert_eq!(summary[0].accepted_name_count, 21);
    assert_eq!(summary[0].aliased_parameter_count, 5);
    assert_eq!(summary[0].max_alias_count, 3);
    assert_eq!(
        summary[0].aliased_parameters,
        vec!["IS", "VT", "CJO", "VJ", "M"]
    );
    assert_eq!(summary[5].kind, ModelCardKind::Nmos);
    assert_eq!(summary[5].canonical_parameter_count, 33);
    assert_eq!(summary[5].accepted_name_count, 41);
    assert_eq!(summary[5].aliased_parameter_count, 7);
    assert_eq!(summary[5].max_alias_count, 3);
    assert_eq!(
        summary[5].aliased_parameters,
        vec!["VT0", "LAMBDA", "U0", "N_SUB", "T_NOM", "CBS", "CBD"]
    );
    assert_eq!(summary.last().unwrap().kind, ModelCardKind::Pmos);

    let table = format_model_card_supported_parameter_coverage_summary_table();
    let lines = table.lines().collect::<Vec<_>>();
    assert_eq!(
        lines[0],
        "kind\tcanonical_parameter_count\taccepted_name_count\taliased_parameter_count\tmax_alias_count\taliased_parameters"
    );
    assert_eq!(lines[1], "D\t15\t21\t5\t3\tIS|VT|CJO|VJ|M");
    assert_eq!(
        lines.last().unwrap(),
        &"PMOS\t33\t41\t7\t3\tVT0|LAMBDA|U0|N_SUB|T_NOM|CBS|CBD"
    );
    let records = model_card_supported_parameter_coverage_summary_records();
    assert_eq!(records.len(), 7);
    assert_eq!(records[0]["kind"], "D");
    assert_eq!(records[0]["canonical_parameter_count"], "15");
    assert_eq!(records[0]["accepted_name_count"], "21");
    assert_eq!(records[0]["aliased_parameter_count"], "5");
    assert_eq!(records[0]["max_alias_count"], "3");
    assert_eq!(records[0]["aliased_parameters"], "IS|VT|CJO|VJ|M");
    assert!(format_model_card_supported_parameter_coverage_summary_csv().starts_with(
        "kind,canonical_parameter_count,accepted_name_count,aliased_parameter_count,max_alias_count,aliased_parameters\nD,15,21,5,3,IS|VT|CJO|VJ|M\n"
    ));
    let json = format_model_card_supported_parameter_coverage_summary_json();
    assert!(json.starts_with(
        "[{\"kind\":\"D\",\"canonical_parameter_count\":\"15\",\"accepted_name_count\":\"21\",\"aliased_parameter_count\":\"5\",\"max_alias_count\":\"3\",\"aliased_parameters\":\"IS|VT|CJO|VJ|M\"}"
    ));
    assert!(json.ends_with(
        "{\"kind\":\"PMOS\",\"canonical_parameter_count\":\"33\",\"accepted_name_count\":\"41\",\"aliased_parameter_count\":\"7\",\"max_alias_count\":\"3\",\"aliased_parameters\":\"VT0|LAMBDA|U0|N_SUB|T_NOM|CBS|CBD\"}]\n"
    ));
}

#[test]
fn model_card_supported_parameter_coverage_gate_passes_current_catalog() {
    let coverage = model_card_supported_parameter_coverage();
    let report = model_card_supported_parameter_coverage_gate(&coverage);
    assert!(report.passed);
    assert_eq!(report.kind_count, 7);
    assert_eq!(report.expected_kind_count, 7);
    assert_eq!(report.canonical_parameter_count, 207);
    assert_eq!(report.expected_canonical_parameter_count, 207);
    assert_eq!(report.accepted_name_count, 279);
    assert_eq!(report.aliased_parameter_count, 59);
    assert_eq!(report.max_alias_count, 4);
    assert!(report.issues.is_empty());
    assert_eq!(
        format_model_card_supported_parameter_coverage_gate_report(&report),
        "passed\tkind_count\texpected_kind_count\tcanonical_parameter_count\texpected_canonical_parameter_count\taccepted_name_count\taliased_parameter_count\tmax_alias_count\tissue_count\ntrue\t7\t7\t207\t207\t279\t59\t4\t0"
    );
    assert_eq!(
        format_model_card_supported_parameter_coverage_gate_issue_table(&report),
        "kind\tfield\tmessage"
    );
    assert!(model_card_supported_parameter_coverage_gate_issue_records(&report).is_empty());
    assert_eq!(
        format_model_card_supported_parameter_coverage_gate_issue_csv(&report),
        "kind,field,message\n"
    );
    assert_eq!(
        format_model_card_supported_parameter_coverage_gate_issue_json(&report),
        "[]\n"
    );
}

#[test]
fn model_card_supported_parameter_coverage_gate_reports_missing_alias_family() {
    let coverage = model_card_supported_parameter_coverage()
        .into_iter()
        .filter(|row| !(row.kind == ModelCardKind::Nmos && row.canonical_parameter == "VT0"))
        .collect::<Vec<_>>();
    let report = model_card_supported_parameter_coverage_gate(&coverage);

    assert!(!report.passed);
    assert_eq!(report.kind_count, 7);
    assert_eq!(report.canonical_parameter_count, 206);
    assert_eq!(report.accepted_name_count, 276);
    assert_eq!(report.aliased_parameter_count, 58);
    assert_eq!(report.max_alias_count, 4);
    assert_eq!(report.issues.len(), 4);
    assert_eq!(report.issues[0].kind, "NMOS");
    assert_eq!(report.issues[0].field, "canonical_parameter_count");
    assert_eq!(
        report.issues[0].message,
        "expected NMOS to expose 33 canonical supported parameters, found 32"
    );
    assert_eq!(report.issues.last().unwrap().field, "max_alias_count");
    assert_eq!(
        report.issues.last().unwrap().message,
        "expected NMOS max alias count 3, found 2"
    );
    assert_eq!(
        format_model_card_supported_parameter_coverage_gate_report(&report),
        "passed\tkind_count\texpected_kind_count\tcanonical_parameter_count\texpected_canonical_parameter_count\taccepted_name_count\taliased_parameter_count\tmax_alias_count\tissue_count\nfalse\t7\t7\t206\t207\t276\t58\t4\t4\nkind\tfield\tmessage\nNMOS\tcanonical_parameter_count\texpected NMOS to expose 33 canonical supported parameters, found 32\nNMOS\taccepted_name_count\texpected NMOS to expose 41 accepted model-card names, found 38\nNMOS\taliased_parameter_count\texpected NMOS to expose 7 alias-bearing parameters, found 6\nNMOS\tmax_alias_count\texpected NMOS max alias count 3, found 2"
    );
    let records = model_card_supported_parameter_coverage_gate_issue_records(&report);
    assert_eq!(records[0]["kind"], "NMOS");
    assert_eq!(records[0]["field"], "canonical_parameter_count");
    assert_eq!(
        records[0]["message"],
        "expected NMOS to expose 33 canonical supported parameters, found 32"
    );
    assert!(format_model_card_supported_parameter_coverage_gate_issue_csv(&report).starts_with(
        "kind,field,message\nNMOS,canonical_parameter_count,\"expected NMOS to expose 33 canonical supported parameters, found 32\"\n"
    ));
    assert!(format_model_card_supported_parameter_coverage_gate_issue_json(&report).starts_with(
        "[{\"kind\":\"NMOS\",\"field\":\"canonical_parameter_count\",\"message\":\"expected NMOS to expose 33 canonical supported parameters, found 32\"}"
    ));
}

#[test]
fn model_card_supported_parameter_coverage_dashboard_exports_are_stable() {
    let coverage = model_card_supported_parameter_coverage();
    let dashboard = model_card_supported_parameter_coverage_dashboard(&coverage);
    assert_eq!(dashboard.len(), 7);
    assert_eq!(dashboard[0].kind, ModelCardKind::Diode);
    assert!(dashboard[0].passed);
    assert_eq!(dashboard[0].canonical_parameter_count, 15);
    assert_eq!(dashboard[0].expected_canonical_parameter_count, 15);
    assert_eq!(dashboard[0].accepted_name_count, 21);
    assert_eq!(dashboard[0].expected_accepted_name_count, 21);
    assert_eq!(dashboard[0].aliased_parameter_count, 5);
    assert_eq!(dashboard[0].expected_aliased_parameter_count, 5);
    assert_eq!(dashboard[0].max_alias_count, 3);
    assert_eq!(dashboard[0].expected_max_alias_count, 3);
    assert_eq!(dashboard[0].issue_count, 0);
    assert!(dashboard[0].issue_fields.is_empty());
    assert_eq!(dashboard[5].kind, ModelCardKind::Nmos);
    assert_eq!(dashboard[5].canonical_parameter_count, 33);
    assert_eq!(dashboard[5].accepted_name_count, 41);
    assert_eq!(dashboard[5].issue_count, 0);

    let table = format_model_card_supported_parameter_coverage_dashboard_table(&coverage);
    let lines = table.lines().collect::<Vec<_>>();
    assert_eq!(
        lines[0],
        "kind\tpassed\tcanonical_parameter_count\texpected_canonical_parameter_count\taccepted_name_count\texpected_accepted_name_count\taliased_parameter_count\texpected_aliased_parameter_count\tmax_alias_count\texpected_max_alias_count\tissue_count\tissue_fields"
    );
    assert_eq!(lines[1], "D\ttrue\t15\t15\t21\t21\t5\t5\t3\t3\t0\t");
    assert_eq!(
        lines.last().unwrap(),
        &"PMOS\ttrue\t33\t33\t41\t41\t7\t7\t3\t3\t0\t"
    );
    let records = model_card_supported_parameter_coverage_dashboard_records(&coverage);
    assert_eq!(records.len(), 7);
    assert_eq!(records[0]["kind"], "D");
    assert_eq!(records[0]["passed"], "true");
    assert_eq!(records[0]["canonical_parameter_count"], "15");
    assert_eq!(records[0]["expected_canonical_parameter_count"], "15");
    assert_eq!(records[0]["issue_count"], "0");
    assert_eq!(records[0]["issue_fields"], "");
    assert!(format_model_card_supported_parameter_coverage_dashboard_csv(&coverage).starts_with(
        "kind,passed,canonical_parameter_count,expected_canonical_parameter_count,accepted_name_count,expected_accepted_name_count,aliased_parameter_count,expected_aliased_parameter_count,max_alias_count,expected_max_alias_count,issue_count,issue_fields\nD,true,15,15,21,21,5,5,3,3,0,\n"
    ));
    assert!(format_model_card_supported_parameter_coverage_dashboard_json(&coverage).starts_with(
        "[{\"kind\":\"D\",\"passed\":\"true\",\"canonical_parameter_count\":\"15\",\"expected_canonical_parameter_count\":\"15\""
    ));
}

#[test]
fn model_card_supported_parameter_coverage_dashboard_reports_missing_alias_family() {
    let coverage = model_card_supported_parameter_coverage()
        .into_iter()
        .filter(|row| !(row.kind == ModelCardKind::Nmos && row.canonical_parameter == "VT0"))
        .collect::<Vec<_>>();

    let dashboard = model_card_supported_parameter_coverage_dashboard(&coverage);
    let nmos = dashboard
        .iter()
        .find(|row| row.kind == ModelCardKind::Nmos)
        .unwrap();

    assert!(!nmos.passed);
    assert_eq!(nmos.canonical_parameter_count, 32);
    assert_eq!(nmos.expected_canonical_parameter_count, 33);
    assert_eq!(nmos.accepted_name_count, 38);
    assert_eq!(nmos.expected_accepted_name_count, 41);
    assert_eq!(nmos.aliased_parameter_count, 6);
    assert_eq!(nmos.expected_aliased_parameter_count, 7);
    assert_eq!(nmos.max_alias_count, 2);
    assert_eq!(nmos.expected_max_alias_count, 3);
    assert_eq!(nmos.issue_count, 4);
    assert_eq!(
        nmos.issue_fields,
        vec![
            "canonical_parameter_count".to_string(),
            "accepted_name_count".to_string(),
            "aliased_parameter_count".to_string(),
            "max_alias_count".to_string()
        ]
    );
    assert!(format_model_card_supported_parameter_coverage_dashboard_table(&coverage).contains(
        "NMOS\tfalse\t32\t33\t38\t41\t6\t7\t2\t3\t4\tcanonical_parameter_count|accepted_name_count|aliased_parameter_count|max_alias_count"
    ));
}

#[test]
fn model_card_aliases_build_device_instances() {
    let diode_card = normalize_model_card(
        "Dfast",
        "diode",
        &[
            ("JS", 2.0e-14),
            ("CJ", 1.5e-12),
            ("TT", 4.0e-9),
            ("PB", 0.8),
            ("MJ", 0.4),
            ("FC", 0.35),
            ("XTI", 2.2),
            ("EG", 1.05),
            ("RS", 10.0),
            ("KF", 1.0e-12),
            ("AF", 1.3),
        ],
    )
    .unwrap();
    let diode_model = diode_from_model_card("D1", "a", "k", &diode_card).unwrap();
    assert_close(*diode_card.parameters.get("IS").unwrap(), 2.0e-14);
    assert_close(*diode_card.parameters.get("CJO").unwrap(), 1.5e-12);
    assert_close(*diode_card.parameters.get("VJ").unwrap(), 0.8);
    assert_close(*diode_card.parameters.get("M").unwrap(), 0.4);
    assert_close(*diode_card.parameters.get("FC").unwrap(), 0.35);
    assert_close(*diode_card.parameters.get("XTI").unwrap(), 2.2);
    assert_close(*diode_card.parameters.get("EG").unwrap(), 1.05);
    assert_close(*diode_card.parameters.get("RS").unwrap(), 10.0);
    assert_close(*diode_card.parameters.get("KF").unwrap(), 1.0e-12);
    assert_close(*diode_card.parameters.get("AF").unwrap(), 1.3);
    assert!(diode_card.unsupported_parameters.is_empty());
    assert_close(diode_model.saturation_current, 2.0e-14);
    assert_close(diode_model.junction_capacitance, 1.5e-12);
    assert_close(diode_model.transit_time, 4.0e-9);
    assert_close(diode_model.junction_potential, 0.8);
    assert_close(diode_model.grading_coefficient, 0.4);
    assert_close(diode_model.forward_bias_depletion_coefficient, 0.35);
    assert_close(diode_model.saturation_current_temperature_exponent, 2.2);
    assert_close(diode_model.energy_gap_electron_volts, 1.05);
    assert_close(diode_model.series_resistance, 10.0);
    assert_close(diode_model.flicker_noise_coefficient, 1.0e-12);
    assert_close(diode_model.flicker_noise_exponent, 1.3);

    let bjt_card = normalize_model_card(
        "Qsmall",
        "npn",
        &[
            ("BETA", 125.0),
            ("CBE", 2.0e-12),
            ("XTI", 2.4),
            ("XTB", 1.5),
            ("BETA_R", 0.25),
            ("EG", 1.05),
            ("VA", 80.0),
            ("VB", 120.0),
            ("IK", 2.0e-3),
            ("IKR", 3.0e-3),
            ("T_NOM", 50.0),
            ("KF", 1.0e-12),
            ("AF", 1.3),
            ("PTF", 30.0),
            ("XTF", 2.0),
            ("ITF", 4.0e-3),
            ("VTF", 0.6),
            ("RE", 12.0),
            ("RC", 13.0),
            ("ISE", 3.0e-13),
            ("NE", 1.7),
            ("ISC", 4.0e-13),
            ("NC", 1.8),
            ("NF", 1.2),
            ("NR", 1.3),
            ("PE", 0.8),
            ("ME", 0.4),
            ("PC", 0.7),
            ("MC", 0.45),
            ("FC", 0.4),
            ("RB", 14.0),
            ("RBM", 2.0),
            ("IRB", 5.0e-6),
            ("XCJC", 0.4),
        ],
    )
    .unwrap();
    let bjt_model = bjt_from_model_card("Q1", "c", "b", "e", &bjt_card).unwrap();
    assert_close(*bjt_card.parameters.get("BF").unwrap(), 125.0);
    assert_close(*bjt_card.parameters.get("CJE").unwrap(), 2.0e-12);
    assert_close(*bjt_card.parameters.get("XTI").unwrap(), 2.4);
    assert_close(*bjt_card.parameters.get("XTB").unwrap(), 1.5);
    assert_close(*bjt_card.parameters.get("BR").unwrap(), 0.25);
    assert_close(*bjt_card.parameters.get("EG").unwrap(), 1.05);
    assert_close(*bjt_card.parameters.get("VAF").unwrap(), 80.0);
    assert_close(*bjt_card.parameters.get("VAR").unwrap(), 120.0);
    assert_close(*bjt_card.parameters.get("IKF").unwrap(), 2.0e-3);
    assert_close(*bjt_card.parameters.get("IKR").unwrap(), 3.0e-3);
    assert_close(*bjt_card.parameters.get("TNOM").unwrap(), 50.0);
    assert_close(*bjt_card.parameters.get("KF").unwrap(), 1.0e-12);
    assert_close(*bjt_card.parameters.get("AF").unwrap(), 1.3);
    assert_close(*bjt_card.parameters.get("PTF").unwrap(), 30.0);
    assert_close(*bjt_card.parameters.get("XTF").unwrap(), 2.0);
    assert_close(*bjt_card.parameters.get("ITF").unwrap(), 4.0e-3);
    assert_close(*bjt_card.parameters.get("VTF").unwrap(), 0.6);
    assert_close(*bjt_card.parameters.get("RE").unwrap(), 12.0);
    assert_close(*bjt_card.parameters.get("RC").unwrap(), 13.0);
    assert_close(*bjt_card.parameters.get("RB").unwrap(), 14.0);
    assert_close(*bjt_card.parameters.get("RBM").unwrap(), 2.0);
    assert_close(*bjt_card.parameters.get("IRB").unwrap(), 5.0e-6);
    assert_close(*bjt_card.parameters.get("ISE").unwrap(), 3.0e-13);
    assert_close(*bjt_card.parameters.get("NE").unwrap(), 1.7);
    assert_close(*bjt_card.parameters.get("ISC").unwrap(), 4.0e-13);
    assert_close(*bjt_card.parameters.get("NC").unwrap(), 1.8);
    assert_close(*bjt_card.parameters.get("NF").unwrap(), 1.2);
    assert_close(*bjt_card.parameters.get("NR").unwrap(), 1.3);
    assert_close(*bjt_card.parameters.get("VJE").unwrap(), 0.8);
    assert_close(*bjt_card.parameters.get("MJE").unwrap(), 0.4);
    assert_close(*bjt_card.parameters.get("VJC").unwrap(), 0.7);
    assert_close(*bjt_card.parameters.get("MJC").unwrap(), 0.45);
    assert_close(*bjt_card.parameters.get("FC").unwrap(), 0.4);
    assert_eq!(bjt_model.polarity, BjtPolarity::Npn);
    assert_close(bjt_model.forward_beta, 125.0);
    assert_close(bjt_model.base_emitter_capacitance, 2.0e-12);
    assert_close(bjt_model.saturation_current_temperature_exponent, 2.4);
    assert_close(bjt_model.forward_beta_temperature_exponent, 1.5);
    assert_close(bjt_model.reverse_beta, 0.25);
    assert_close(bjt_model.energy_gap_electron_volts, 1.05);
    assert_close(bjt_model.forward_early_voltage, 80.0);
    assert_close(bjt_model.reverse_early_voltage, 120.0);
    assert_close(bjt_model.forward_beta_rolloff_current, 2.0e-3);
    assert_close(bjt_model.reverse_beta_rolloff_current, 3.0e-3);
    assert_close(bjt_model.nominal_temperature_kelvin.unwrap(), 323.15);
    assert_close(bjt_model.flicker_noise_coefficient, 1.0e-12);
    assert_close(bjt_model.flicker_noise_exponent, 1.3);
    assert_close(bjt_model.forward_excess_phase_degrees, 30.0);
    assert_close(bjt_model.forward_transit_time_bias_coefficient, 2.0);
    assert_close(bjt_model.forward_transit_time_current, 4.0e-3);
    assert_close(bjt_model.forward_transit_time_voltage, 0.6);
    assert_close(bjt_model.emitter_resistance, 12.0);
    assert_close(bjt_model.collector_resistance, 13.0);
    assert_close(bjt_model.base_resistance, 14.0);
    assert_close(bjt_model.minimum_base_resistance.unwrap(), 2.0);
    assert_close(bjt_model.base_resistance_half_current, 5.0e-6);
    assert_close(bjt_model.base_collector_capacitance_fraction, 0.4);
    assert_close(bjt_model.base_emitter_leakage_saturation_current, 3.0e-13);
    assert_close(bjt_model.base_emitter_leakage_emission_coefficient, 1.7);
    assert_close(bjt_model.base_collector_leakage_saturation_current, 4.0e-13);
    assert_close(bjt_model.base_collector_leakage_emission_coefficient, 1.8);
    assert_close(bjt_model.forward_emission_coefficient, 1.2);
    assert_close(bjt_model.reverse_emission_coefficient, 1.3);
    assert_close(bjt_model.base_emitter_junction_potential, 0.8);
    assert_close(bjt_model.base_emitter_grading_coefficient, 0.4);
    assert_close(bjt_model.base_collector_junction_potential, 0.7);
    assert_close(bjt_model.base_collector_grading_coefficient, 0.45);
    assert_close(bjt_model.forward_bias_depletion_coefficient, 0.4);

    let jfet_card = normalize_model_card(
        "Jn",
        "njfet",
        &[
            ("BET", 9.0e-4),
            ("VT0", -1.8),
            ("LAM", 0.02),
            ("KF", 1.0e-12),
            ("AF", 1.3),
            ("VJ", 0.8),
            ("FC", 0.35),
            ("IS", 2.0e-13),
            ("XTI", 2.5),
            ("EG", 1.05),
            ("B", 1.1),
            ("NLEV", 3.0),
            ("GDSNOI", 1.25),
            ("RD", 125.0),
            ("RS", 75.0),
            ("T_NOM", 50.0),
            ("TCV", 0.01),
            ("VTOTC", -0.0025),
            ("BEX", 1.5),
            ("BETATCE", -0.5),
        ],
    )
    .unwrap();
    let jfet_model = jfet_from_model_card("J1", "d", "g", "s", &jfet_card).unwrap();
    assert_close(*jfet_card.parameters.get("BETA").unwrap(), 9.0e-4);
    assert_close(*jfet_card.parameters.get("VTO").unwrap(), -1.8);
    assert_close(*jfet_card.parameters.get("LAMBDA").unwrap(), 0.02);
    assert_close(*jfet_card.parameters.get("KF").unwrap(), 1.0e-12);
    assert_close(*jfet_card.parameters.get("AF").unwrap(), 1.3);
    assert_close(*jfet_card.parameters.get("PB").unwrap(), 0.8);
    assert_close(*jfet_card.parameters.get("FC").unwrap(), 0.35);
    assert_close(*jfet_card.parameters.get("IS").unwrap(), 2.0e-13);
    assert_close(*jfet_card.parameters.get("XTI").unwrap(), 2.5);
    assert_close(*jfet_card.parameters.get("EG").unwrap(), 1.05);
    assert_close(*jfet_card.parameters.get("B").unwrap(), 1.1);
    assert_close(*jfet_card.parameters.get("NLEV").unwrap(), 3.0);
    assert_close(*jfet_card.parameters.get("GDSNOI").unwrap(), 1.25);
    assert_close(*jfet_card.parameters.get("RD").unwrap(), 125.0);
    assert_close(*jfet_card.parameters.get("RS").unwrap(), 75.0);
    assert_close(*jfet_card.parameters.get("TNOM").unwrap(), 50.0);
    assert_close(*jfet_card.parameters.get("TCV").unwrap(), 0.01);
    assert_close(*jfet_card.parameters.get("VTOTC").unwrap(), -0.0025);
    assert_close(*jfet_card.parameters.get("BEX").unwrap(), 1.5);
    assert_close(*jfet_card.parameters.get("BETATCE").unwrap(), -0.5);
    assert_eq!(jfet_model.polarity, JfetPolarity::Njf);
    assert_close(jfet_model.beta, 9.0e-4);
    assert_close(jfet_model.threshold_voltage, -1.8);
    assert_close(jfet_model.channel_length_modulation, 0.02);
    assert_close(jfet_model.flicker_noise_coefficient, 1.0e-12);
    assert_close(jfet_model.flicker_noise_exponent, 1.3);
    assert_close(jfet_model.junction_potential, 0.8);
    assert_close(jfet_model.forward_bias_depletion_coefficient, 0.35);
    assert_close(jfet_model.gate_saturation_current, 2.0e-13);
    assert_close(jfet_model.gate_saturation_current_temperature_exponent, 2.5);
    assert_close(jfet_model.bandgap_voltage, 1.05);
    assert_close(jfet_model.doping_tail_parameter, 1.1);
    assert_close(jfet_model.noise_equation_level, 3.0);
    assert_close(jfet_model.channel_noise_coefficient, 1.25);
    assert_close(jfet_model.nominal_temperature_kelvin.unwrap(), 323.15);
    assert_close(jfet_model.threshold_voltage_temperature_coefficient, 0.01);
    assert_close(
        jfet_model
            .alternative_threshold_voltage_temperature_coefficient
            .unwrap(),
        -0.0025,
    );
    assert_close(jfet_model.drain_resistance, 125.0);
    assert_close(jfet_model.source_resistance, 75.0);
    assert_close(jfet_model.mobility_temperature_exponent, 1.5);
    assert_close(jfet_model.mobility_temperature_coefficient.unwrap(), -0.5);

    let mos_card = normalize_model_card(
        "Mn",
        "nmos",
        &[
            ("LEVEL", 1.0),
            ("VTO", 0.55),
            ("LAM", 0.04),
            ("NSUB", 1.6e16),
            ("CJD", 3.0e-13),
            ("PB", 0.9),
            ("MJ", 0.45),
            ("MJSW", 0.25),
            ("FC", 0.4),
            ("LD", 50.0e-9),
            ("RD", 125.0),
            ("RS", 75.0),
            ("RSH", 50.0),
            ("IS", 4.0e-15),
            ("JS", 2.0e-3),
            ("CJ", 2.0e-3),
            ("TOX", 25.0e-9),
            ("UO", 500.0),
            ("KF", 2.0e-24),
            ("AF", 1.4),
        ],
    )
    .unwrap();
    let mos_model = mosfet_from_model_card("M1", "d", "g", "s", "b", &mos_card).unwrap();
    assert_close(*mos_card.parameters.get("VT0").unwrap(), 0.55);
    assert_close(*mos_card.parameters.get("LAMBDA").unwrap(), 0.04);
    assert_close(*mos_card.parameters.get("N_SUB").unwrap(), 1.6e16);
    assert_close(*mos_card.parameters.get("CBD").unwrap(), 3.0e-13);
    assert_close(*mos_card.parameters.get("PB").unwrap(), 0.9);
    assert_close(*mos_card.parameters.get("MJ").unwrap(), 0.45);
    assert_close(*mos_card.parameters.get("MJSW").unwrap(), 0.25);
    assert_close(*mos_card.parameters.get("FC").unwrap(), 0.4);
    assert_close(*mos_card.parameters.get("LD").unwrap(), 50.0e-9);
    assert_close(*mos_card.parameters.get("RD").unwrap(), 125.0);
    assert_close(*mos_card.parameters.get("RS").unwrap(), 75.0);
    assert_close(*mos_card.parameters.get("RSH").unwrap(), 50.0);
    assert_close(*mos_card.parameters.get("IS").unwrap(), 4.0e-15);
    assert_close(*mos_card.parameters.get("JS").unwrap(), 2.0e-3);
    assert_close(*mos_card.parameters.get("CJ").unwrap(), 2.0e-3);
    assert_close(*mos_card.parameters.get("TOX").unwrap(), 25.0e-9);
    assert_close(*mos_card.parameters.get("U0").unwrap(), 500.0);
    assert_close(*mos_card.parameters.get("KF").unwrap(), 2.0e-24);
    assert_close(*mos_card.parameters.get("AF").unwrap(), 1.4);
    assert_eq!(mos_model.mosfet_type, MosfetType::Nmos);
    assert_close(mos_model.params.vt0, 0.55);
    assert_close(mos_model.params.lambda, 0.04);
    assert_close(mos_model.params.n_sub, 1.6e16);
    assert_close(mos_model.params.drain_bulk_capacitance, 3.0e-13);
    assert_close(mos_model.params.bulk_junction_potential, 0.9);
    assert_close(mos_model.params.bulk_junction_grading_coefficient, 0.45);
    assert_close(mos_model.params.sidewall_junction_grading_coefficient, 0.25);
    assert_close(mos_model.params.forward_bias_depletion_coefficient, 0.4);
    assert_close(mos_model.params.lateral_diffusion_length, 50.0e-9);
    assert_close(mos_model.params.drain_resistance, 125.0);
    assert_close(mos_model.params.saturation_current, 4.0e-15);
    assert_close(mos_model.params.saturation_current_density, 2.0e-3);
    assert_close(mos_model.params.source_resistance, 75.0);
    assert_close(mos_model.params.sheet_resistance, 50.0);
    assert_close(mos_model.params.drain_squares, 1.0);
    assert_close(mos_model.params.source_squares, 1.0);
    assert_close(mos_model.params.bottom_junction_capacitance, 2.0e-3);
    assert_close(mos_model.params.oxide_thickness, 25.0e-9);
    assert_close(mos_model.params.surface_mobility, 500.0);
    assert_close(
        mos_model.params.kp,
        500.0 * 1.0e-4 * 3.453_133e-11 / 25.0e-9,
    );
    assert_close(mos_model.params.flicker_noise_coefficient, 2.0e-24);
    assert_close(mos_model.params.flicker_noise_exponent, 1.4);
}

#[test]
fn mos_model_card_surface_mobility_derives_kp_with_explicit_precedence() {
    let default_mobility = normalize_model_card("Mdefault", "nmos", &[("TOX", 100.0e-9)]).unwrap();
    let derived = mosfet_from_model_card("M1", "d", "g", "s", "b", &default_mobility).unwrap();
    assert_close(derived.params.surface_mobility, 600.0);
    assert_close(derived.params.kp, 600.0 * 1.0e-4 * 3.453_133e-11 / 100.0e-9);

    let explicit = normalize_model_card(
        "Mexplicit",
        "nmos",
        &[("TOX", 100.0e-9), ("U0", 500.0), ("KP", 250.0e-6)],
    )
    .unwrap();
    let explicit = mosfet_from_model_card("M2", "d", "g", "s", "b", &explicit).unwrap();
    assert_close(explicit.params.surface_mobility, 500.0);
    assert_close(explicit.params.kp, 250.0e-6);
}

#[test]
fn mos_model_card_substrate_doping_derives_electrostatics_with_explicit_precedence() {
    let derived_card = normalize_model_card(
        "Mderived",
        "nmos",
        &[("NSUB", 4.0e15), ("TOX", 100.0e-9), ("NSS", 1.0e10)],
    )
    .unwrap();
    let derived = mosfet_from_model_card("M1", "d", "g", "s", "b", &derived_card).unwrap();
    let thermal_voltage = 1.380_649e-23 * 300.15 / 1.602_176_634e-19;
    let expected_phi = (2.0 * thermal_voltage * (4.0e21_f64 / 1.45e16).ln()).max(0.1);
    let expected_gamma = (2.0_f64 * (11.70 * 8.854_214_871e-12) * 1.602_176_634e-19 * 4.0e21)
        .sqrt()
        / (3.453_133e-11 / 100.0e-9);
    let expected_band_gap = 1.16 - 7.02e-4 * 300.15 * 300.15 / (300.15 + 1108.0);
    let process_vt0 =
        expected_gamma * expected_phi.sqrt() + 0.5 * (expected_phi - expected_band_gap);
    let surface_state_shift = 1.0e10 * 1.0e4 * 1.602_176_634e-19 / (3.453_133e-11 / 100.0e-9);
    let expected_vt0 = process_vt0 - surface_state_shift;
    assert_close(derived.params.phi, expected_phi);
    assert_close(derived.params.gamma, expected_gamma);
    assert_close(derived.params.vt0, expected_vt0);

    let pmos_card = normalize_model_card(
        "Mp",
        "pmos",
        &[("NSUB", 4.0e15), ("TOX", 100.0e-9), ("NSS", 1.0e10)],
    )
    .unwrap();
    let pmos = mosfet_from_model_card("M2", "d", "g", "s", "b", &pmos_card).unwrap();
    assert_close(pmos.params.vt0, -process_vt0 - surface_state_shift);

    let explicit_card = normalize_model_card(
        "Mexplicit",
        "nmos",
        &[
            ("NSUB", 4.0e15),
            ("TOX", 100.0e-9),
            ("PHI", 0.72),
            ("GAMMA", 0.41),
            ("NSS", 1.0e10),
            ("VTO", 0.63),
        ],
    )
    .unwrap();
    let explicit = mosfet_from_model_card("M3", "d", "g", "s", "b", &explicit_card).unwrap();
    assert_close(explicit.params.phi, 0.72);
    assert_close(explicit.params.gamma, 0.41);
    assert_close(explicit.params.vt0, 0.63);

    let invalid_card =
        normalize_model_card("Minvalid", "nmos", &[("NSUB", 1.0e10), ("TOX", 1.0e-7)]).unwrap();
    assert!(matches!(
        mosfet_from_model_card("M4", "d", "g", "s", "b", &invalid_card),
        Err(SpiceError::InvalidElement { reason, .. })
            if reason == "MOSFET NSUB must exceed the intrinsic carrier density"
    ));
}

#[test]
fn mos_model_card_gate_material_shifts_process_derived_threshold() {
    let process = [("NSUB", 4.0e15), ("TOX", 100.0e-9)];
    let default_nmos = normalize_model_card("Mdefault", "nmos", &process).unwrap();
    let default_nmos = mosfet_from_model_card("M1", "d", "g", "s", "b", &default_nmos).unwrap();
    let band_gap = 1.16 - 7.02e-4 * 300.15 * 300.15 / (300.15 + 1108.0);

    let opposite_nmos = normalize_model_card(
        "Mopposite",
        "nmos",
        &[process[0], process[1], ("TPG", -1.0)],
    )
    .unwrap();
    let opposite_nmos = mosfet_from_model_card("M2", "d", "g", "s", "b", &opposite_nmos).unwrap();
    assert_close(opposite_nmos.params.vt0, default_nmos.params.vt0 + band_gap);

    let metal_nmos =
        normalize_model_card("Mmetal", "nmos", &[process[0], process[1], ("TPG", 0.0)]).unwrap();
    let metal_nmos = mosfet_from_model_card("M3", "d", "g", "s", "b", &metal_nmos).unwrap();
    assert_close(metal_nmos.params.vt0, default_nmos.params.vt0 - 0.05);

    let default_pmos = normalize_model_card("Mpdefault", "pmos", &process).unwrap();
    let default_pmos = mosfet_from_model_card("M4", "d", "g", "s", "b", &default_pmos).unwrap();
    let opposite_pmos = normalize_model_card(
        "Mpopposite",
        "pmos",
        &[process[0], process[1], ("TPG", -1.0)],
    )
    .unwrap();
    let opposite_pmos = mosfet_from_model_card("M5", "d", "g", "s", "b", &opposite_pmos).unwrap();
    assert_close(opposite_pmos.params.vt0, default_pmos.params.vt0 - band_gap);

    let explicit = normalize_model_card(
        "Mexplicit",
        "nmos",
        &[process[0], process[1], ("TPG", -1.0), ("VTO", 0.63)],
    )
    .unwrap();
    let explicit = mosfet_from_model_card("M6", "d", "g", "s", "b", &explicit).unwrap();
    assert_close(explicit.params.vt0, 0.63);

    assert!(matches!(
        normalize_model_card("Minvalid", "nmos", &[("TPG", 0.5)]),
        Err(SpiceError::InvalidElement { reason, .. })
            if reason == "MOSFET TPG must be -1, 0, or 1"
    ));

    for invalid_nss in [-1.0, f64::INFINITY, f64::NAN] {
        assert!(matches!(
            normalize_model_card("Minvalid", "nmos", &[("NSS", invalid_nss)]),
            Err(SpiceError::InvalidElement { reason, .. })
                if reason == "MOSFET NSS must be finite and non-negative"
        ));
    }
}

#[test]
fn mos_model_card_rejects_invalid_nominal_temperature() {
    let valid = normalize_model_card("Mvalid", "nmos", &[("TNOM", 325.0)]).unwrap();
    assert_close(*valid.parameters.get("T_NOM").unwrap(), 325.0);

    for invalid_temperature in [0.0, -1.0, f64::INFINITY, f64::NAN] {
        assert!(matches!(
            normalize_model_card("Minvalid", "nmos", &[("TNOM", invalid_temperature)]),
            Err(SpiceError::InvalidElement { reason, .. })
                if reason == "MOSFET TNOM must be finite and positive"
        ));
    }
}

#[test]
fn mos_model_card_rejects_invalid_substrate_doping() {
    let valid = normalize_model_card("Mvalid", "nmos", &[("NSUB", 4.0e15)]).unwrap();
    assert_close(*valid.parameters.get("N_SUB").unwrap(), 4.0e15);

    for invalid_doping in [0.0, -1.0, f64::INFINITY, f64::NAN] {
        assert!(matches!(
            normalize_model_card("Minvalid", "nmos", &[("NSUB", invalid_doping)]),
            Err(SpiceError::InvalidElement { reason, .. })
                if reason == "MOSFET NSUB must be finite and positive"
        ));
    }
}

#[test]
fn mos_model_card_rejects_invalid_oxide_thickness() {
    let valid = normalize_model_card("Mvalid", "nmos", &[("TOX", 100.0e-9)]).unwrap();
    assert_close(*valid.parameters.get("TOX").unwrap(), 100.0e-9);

    for invalid_thickness in [0.0, -1.0, f64::INFINITY, f64::NAN] {
        assert!(matches!(
            normalize_model_card("Minvalid", "nmos", &[("TOX", invalid_thickness)]),
            Err(SpiceError::InvalidElement { reason, .. })
                if reason == "MOSFET TOX must be finite and positive"
        ));
    }
}

#[test]
fn mos_model_card_rejects_invalid_surface_mobility() {
    for (name, value) in [("U0", 0.0), ("UO", 600.0)] {
        let valid = normalize_model_card("Mvalid", "nmos", &[(name, value)]).unwrap();
        assert_close(*valid.parameters.get("U0").unwrap(), value);
    }

    for invalid_mobility in [-1.0, f64::INFINITY, f64::NAN] {
        assert!(matches!(
            normalize_model_card("Minvalid", "nmos", &[("U0", invalid_mobility)]),
            Err(SpiceError::InvalidElement { reason, .. })
                if reason == "MOSFET U0 must be finite and non-negative"
        ));
    }
}

#[test]
fn mos_model_card_rejects_invalid_transconductance() {
    let valid = normalize_model_card("Mvalid", "nmos", &[("KP", 200.0e-6)]).unwrap();
    assert_close(*valid.parameters.get("KP").unwrap(), 200.0e-6);

    for invalid_transconductance in [0.0, -1.0, f64::INFINITY, f64::NAN] {
        assert!(matches!(
            normalize_model_card("Minvalid", "nmos", &[("KP", invalid_transconductance)]),
            Err(SpiceError::InvalidElement { reason, .. })
                if reason == "MOSFET KP must be finite and positive"
        ));
    }
}

#[test]
fn mos_model_card_rejects_non_finite_threshold_voltage() {
    for (name, value) in [("VT0", -0.7), ("VTO", 0.0), ("VTH", 0.7)] {
        let valid = normalize_model_card("Mvalid", "nmos", &[(name, value)]).unwrap();
        assert_close(*valid.parameters.get("VT0").unwrap(), value);
    }

    for invalid_threshold in [f64::NEG_INFINITY, f64::INFINITY, f64::NAN] {
        assert!(matches!(
            normalize_model_card("Minvalid", "nmos", &[("VTO", invalid_threshold)]),
            Err(SpiceError::InvalidElement { reason, .. })
                if reason == "MOSFET VT0 must be finite"
        ));
    }
}

#[test]
fn mos_model_card_rejects_non_finite_channel_modulation() {
    for (name, value) in [("LAMBDA", -0.01), ("LAM", 0.0), ("LAMBDA", 0.01)] {
        let valid = normalize_model_card("Mvalid", "nmos", &[(name, value)]).unwrap();
        assert_close(*valid.parameters.get("LAMBDA").unwrap(), value);
    }

    for invalid_modulation in [f64::NEG_INFINITY, f64::INFINITY, f64::NAN] {
        assert!(matches!(
            normalize_model_card("Minvalid", "nmos", &[("LAM", invalid_modulation)]),
            Err(SpiceError::InvalidElement { reason, .. })
                if reason == "MOSFET LAMBDA must be finite"
        ));
    }
}

#[test]
fn bjt_legacy_leakage_ratios_derive_currents_with_explicit_precedence() {
    let legacy_card = normalize_model_card(
        "Qlegacy",
        "npn",
        &[("IS", 2.0e-14), ("C2", 15.0), ("C4", 20.0)],
    )
    .unwrap();
    let legacy = bjt_from_model_card("Q1", "c", "b", "e", &legacy_card).unwrap();

    assert_close(*legacy_card.parameters.get("C2").unwrap(), 15.0);
    assert_close(*legacy_card.parameters.get("C4").unwrap(), 20.0);
    assert_close(legacy.base_emitter_leakage_saturation_current, 3.0e-13);
    assert_close(legacy.base_collector_leakage_saturation_current, 4.0e-13);

    let explicit_card = normalize_model_card(
        "Qexplicit",
        "pnp",
        &[
            ("IS", 2.0e-14),
            ("C2", 15.0),
            ("ISE", 5.0e-13),
            ("C4", 20.0),
            ("ISC", 6.0e-13),
        ],
    )
    .unwrap();
    let explicit = bjt_from_model_card("Q2", "c", "b", "e", &explicit_card).unwrap();
    assert_close(explicit.base_emitter_leakage_saturation_current, 5.0e-13);
    assert_close(explicit.base_collector_leakage_saturation_current, 6.0e-13);
}

#[test]
fn model_card_audit_fixtures_cover_supported_device_families() {
    let fixtures = device_model_audit_fixtures().unwrap();
    assert_eq!(
        fixtures
            .iter()
            .map(|fixture| fixture.kind)
            .collect::<Vec<_>>(),
        vec![
            ModelCardKind::Diode,
            ModelCardKind::Npn,
            ModelCardKind::Njf,
            ModelCardKind::Nmos
        ]
    );
    assert_close(*fixtures[0].parameters.get("IS").unwrap(), 2.0e-14);
    assert_close(*fixtures[1].parameters.get("BF").unwrap(), 125.0);
    assert_close(*fixtures[2].parameters.get("VTO").unwrap(), -1.8);
    assert_close(*fixtures[3].parameters.get("VT0").unwrap(), 0.55);
}

#[test]
fn device_model_behavior_audit_fixtures_run_reference_bias_points() {
    let fixtures = device_model_behavior_audit_fixtures().unwrap();
    assert_eq!(
        fixtures
            .iter()
            .map(|fixture| fixture.name.as_str())
            .collect::<Vec<_>>(),
        vec![
            "diode-forward-bias",
            "bjt-emitter-follower",
            "jfet-source-bias",
            "mos-level1-common-source"
        ]
    );

    for fixture in fixtures {
        let result = dc_op(&fixture.circuit).unwrap();
        let value = *result
            .node_voltages
            .get(&fixture.probe_node)
            .expect("fixture probe node should be present");
        assert!(result.converged);
        assert!(
            value >= fixture.expected_min && value <= fixture.expected_max,
            "{} expected {} <= {} <= {}",
            fixture.name,
            fixture.expected_min,
            value,
            fixture.expected_max
        );
        assert!(fixture.deck_lines[0].starts_with("* device-model behavior fixture:"));
        assert!(fixture.deck_lines.iter().any(|line| line == ".op"));
        assert!(fixture
            .deck_lines
            .iter()
            .any(|line| line.starts_with(".model ")));
    }
}

#[test]
fn device_model_temperature_audit_fixtures_run_reference_sweeps() {
    let fixtures = device_model_temperature_audit_fixtures().unwrap();
    assert_eq!(
        fixtures
            .iter()
            .map(|fixture| fixture.name.as_str())
            .collect::<Vec<_>>(),
        vec![
            "diode-forward-bias",
            "bjt-emitter-follower",
            "jfet-source-bias",
            "mos-level1-common-source"
        ]
    );

    for fixture in &fixtures {
        let temperatures = fixture
            .temperature_points
            .iter()
            .map(|point| point.temperature_kelvin)
            .collect::<Vec<_>>();
        let result = dc_temperature_sweep(
            &fixture.circuit,
            &temperatures,
            fixture.nominal_temperature_kelvin,
            fixture.energy_gap_electron_volts,
            DcOpOptions::default(),
        )
        .unwrap();
        assert!(fixture
            .deck_lines
            .iter()
            .any(|line| line == ".temp 260.15 300.15 340.15"));
        assert!(fixture.deck_lines[0].starts_with("* device-model temperature fixture:"));
        assert_eq!(result.points.len(), fixture.temperature_points.len());
        for (actual, expected) in result.points.iter().zip(&fixture.temperature_points) {
            let value = *actual
                .result
                .node_voltages
                .get(&fixture.probe_node)
                .expect("fixture probe node should be present");
            assert!(actual.result.converged);
            assert_close(actual.temperature_kelvin, expected.temperature_kelvin);
            assert!(
                value >= expected.expected_min && value <= expected.expected_max,
                "{} expected {} <= {} <= {} at {} K",
                fixture.name,
                expected.expected_min,
                value,
                expected.expected_max,
                expected.temperature_kelvin
            );
        }
    }

    let jfet_fixture = fixtures
        .iter()
        .find(|fixture| fixture.kind == ModelCardKind::Njf)
        .expect("NJF fixture should exist");
    assert!(jfet_fixture
        .temperature_behavior
        .starts_with("JFET temperature scaling defaults"));
}

#[test]
fn device_model_reference_deck_audit_fixtures_cover_model_depth_matrix() {
    let fixtures = device_model_reference_deck_audit_fixtures().unwrap();
    assert_eq!(fixtures.len(), 20);
    assert_eq!(fixtures.first().unwrap().name, "diode-forward-bias:op");
    assert_eq!(
        fixtures.last().unwrap().name,
        "mos-level1-storage-charge:tran"
    );

    let expected_analyses = BTreeSet::from([
        "ac".to_string(),
        "noise".to_string(),
        "op".to_string(),
        "temperature".to_string(),
        "tran".to_string(),
    ]);
    let mut by_kind: BTreeMap<ModelCardKind, BTreeSet<String>> = BTreeMap::new();
    for fixture in &fixtures {
        by_kind
            .entry(fixture.kind)
            .or_default()
            .insert(fixture.analysis.clone());
        assert_eq!(
            fixture.reference,
            "SPICE2/SPICE3-style local model-depth fixture"
        );
        assert!(!fixture.expected_behavior.is_empty());
        assert!(fixture.deck_lines[0].starts_with("* device-model "));
        assert!(fixture
            .deck_lines
            .iter()
            .any(|line| line.starts_with(".model ")));
        assert_eq!(fixture.deck_lines.last().unwrap(), ".end");
    }

    assert_eq!(
        by_kind.keys().copied().collect::<BTreeSet<_>>(),
        BTreeSet::from([
            ModelCardKind::Diode,
            ModelCardKind::Npn,
            ModelCardKind::Njf,
            ModelCardKind::Nmos,
        ])
    );
    for analyses in by_kind.values() {
        assert_eq!(analyses, &expected_analyses);
    }
}

#[test]
fn device_model_reference_deck_audit_table_is_stable() {
    let fixtures = device_model_reference_deck_audit_fixtures().unwrap();
    let table = format_device_model_reference_deck_audit_table(&fixtures);
    let lines = table.lines().collect::<Vec<_>>();
    assert_eq!(lines.len(), 21);
    assert_eq!(
        lines[0],
        "name\tkind\tanalysis\tmodel\treference\texpected_behavior\tdeck_lines"
    );
    assert_eq!(
        lines[1],
        "diode-forward-bias:op\tD\top\tDfast\tSPICE2/SPICE3-style local model-depth fixture\tDC probe out remains in [0.55, 0.65] V\t8"
    );
    assert_eq!(
        lines.last().copied().unwrap(),
        "mos-level1-storage-charge:tran\tNMOS\ttran\tMn\tSPICE2/SPICE3-style local model-depth fixture\tLevel-1 MOS CGSO/CGDO/CGBO plus CBS/CBD contribute transient gate-overlap and depletion-shaped bulk-junction storage; explicit Cstore keeps the fixture comparable with other charge audits\t10"
    );
}

#[test]
fn device_model_reference_deck_audit_record_exports_are_stable() {
    let fixtures = device_model_reference_deck_audit_fixtures().unwrap();
    let records = device_model_reference_deck_audit_records(&fixtures);

    assert_eq!(records.len(), 20);
    assert_eq!(
        records[0].get("name").map(String::as_str),
        Some("diode-forward-bias:op")
    );
    assert_eq!(records[0].get("kind").map(String::as_str), Some("D"));
    assert_eq!(records[0].get("analysis").map(String::as_str), Some("op"));
    assert_eq!(records[0].get("model").map(String::as_str), Some("Dfast"));
    assert_eq!(
        records[0].get("reference").map(String::as_str),
        Some("SPICE2/SPICE3-style local model-depth fixture")
    );
    assert_eq!(
        records[0].get("expected_behavior").map(String::as_str),
        Some("DC probe out remains in [0.55, 0.65] V")
    );
    assert_eq!(records[0].get("deck_lines").map(String::as_str), Some("8"));
    assert_eq!(
        records.last().unwrap().get("name").map(String::as_str),
        Some("mos-level1-storage-charge:tran")
    );
    assert_eq!(
        records
            .last()
            .unwrap()
            .get("deck_lines")
            .map(String::as_str),
        Some("10")
    );

    let csv = format_device_model_reference_deck_audit_csv(&fixtures);
    let csv_lines = csv.lines().collect::<Vec<_>>();
    assert_eq!(
        csv_lines[0],
        "name,kind,analysis,model,reference,expected_behavior,deck_lines"
    );
    assert_eq!(
        csv_lines[1],
        "diode-forward-bias:op,D,op,Dfast,SPICE2/SPICE3-style local model-depth fixture,\"DC probe out remains in [0.55, 0.65] V\",8"
    );

    let json = format_device_model_reference_deck_audit_json(&fixtures);
    assert!(json.starts_with("[{\"name\":\"diode-forward-bias:op\""));
    assert!(json.contains("\"deck_lines\":\"10\""));
    assert!(json.ends_with("]\n"));
}

#[test]
fn device_model_reference_deck_audit_summary_exports_are_stable() {
    let fixtures = device_model_reference_deck_audit_fixtures().unwrap();
    let summary = device_model_reference_deck_audit_summary(&fixtures);

    assert_eq!(summary.len(), 4);
    assert_eq!(summary[0].kind, "D");
    assert_eq!(summary[0].fixture_count, 5);
    assert_eq!(
        summary[0].analyses,
        vec!["op", "temperature", "ac", "noise", "tran"]
    );
    assert!(summary[0].missing_analyses.is_empty());
    assert_eq!(summary[0].deck_line_count, 42);
    assert_eq!(
        summary[0].references,
        vec!["SPICE2/SPICE3-style local model-depth fixture"]
    );

    let table = format_device_model_reference_deck_audit_summary_table(&fixtures);
    assert_eq!(
        table,
        concat!(
            "kind\tfixture_count\tanalyses\tmissing_analyses\tdeck_lines\treferences\n",
            "D\t5\top,temperature,ac,noise,tran\t\t42\tSPICE2/SPICE3-style local model-depth fixture\n",
            "NPN\t5\top,temperature,ac,noise,tran\t\t47\tSPICE2/SPICE3-style local model-depth fixture\n",
            "NJF\t5\top,temperature,ac,noise,tran\t\t52\tSPICE2/SPICE3-style local model-depth fixture\n",
            "NMOS\t5\top,temperature,ac,noise,tran\t\t47\tSPICE2/SPICE3-style local model-depth fixture"
        )
    );

    let records = device_model_reference_deck_audit_summary_records(&fixtures);
    assert_eq!(records[0].get("kind").map(String::as_str), Some("D"));
    assert_eq!(
        records[0].get("fixture_count").map(String::as_str),
        Some("5")
    );
    assert_eq!(
        records[0].get("analyses").map(String::as_str),
        Some("op,temperature,ac,noise,tran")
    );
    assert_eq!(
        records[0].get("missing_analyses").map(String::as_str),
        Some("")
    );
    assert_eq!(records[0].get("deck_lines").map(String::as_str), Some("42"));
    assert_eq!(
        format_device_model_reference_deck_audit_summary_csv(&fixtures)
            .lines()
            .nth(1),
        Some("D,5,\"op,temperature,ac,noise,tran\",,42,SPICE2/SPICE3-style local model-depth fixture")
    );
    let json = format_device_model_reference_deck_audit_summary_json(&fixtures);
    assert!(json.starts_with("[{\"kind\":\"D\""));
    assert!(json.contains("\"deck_lines\":\"47\""));
    assert!(json.ends_with("]\n"));
}

#[test]
fn device_model_reference_deck_audit_summary_reports_missing_analysis() {
    let fixtures = device_model_reference_deck_audit_fixtures()
        .unwrap()
        .into_iter()
        .filter(|fixture| !(fixture.kind == ModelCardKind::Nmos && fixture.analysis == "tran"))
        .collect::<Vec<_>>();

    let summary = device_model_reference_deck_audit_summary(&fixtures);
    let nmos = summary
        .iter()
        .find(|row| row.kind == "NMOS")
        .expect("NMOS summary row should exist");

    assert_eq!(nmos.fixture_count, 4);
    assert_eq!(nmos.analyses, vec!["op", "temperature", "ac", "noise"]);
    assert_eq!(nmos.missing_analyses, vec!["tran"]);
    assert_eq!(nmos.deck_line_count, 37);
    assert!(format_device_model_reference_deck_audit_summary_table(&fixtures).contains(
        "NMOS\t4\top,temperature,ac,noise\ttran\t37\tSPICE2/SPICE3-style local model-depth fixture"
    ));
}

#[test]
fn device_model_reference_deck_audit_analysis_summary_exports_are_stable() {
    let fixtures = device_model_reference_deck_audit_fixtures().unwrap();
    let summary = device_model_reference_deck_audit_analysis_summary(&fixtures);

    assert_eq!(summary.len(), 5);
    assert_eq!(summary[0].analysis, "op");
    assert_eq!(summary[0].fixture_count, 4);
    assert_eq!(summary[0].kinds, vec!["D", "NPN", "NJF", "NMOS"]);
    assert!(summary[0].missing_kinds.is_empty());
    assert_eq!(summary[0].deck_line_count, 36);
    assert_eq!(
        summary[0].references,
        vec!["SPICE2/SPICE3-style local model-depth fixture"]
    );

    let table = format_device_model_reference_deck_audit_analysis_summary_table(&fixtures);
    assert_eq!(
        table,
        concat!(
            "analysis\tfixture_count\tkinds\tmissing_kinds\tdeck_lines\treferences\n",
            "op\t4\tD,NPN,NJF,NMOS\t\t36\tSPICE2/SPICE3-style local model-depth fixture\n",
            "temperature\t4\tD,NPN,NJF,NMOS\t\t40\tSPICE2/SPICE3-style local model-depth fixture\n",
            "ac\t4\tD,NPN,NJF,NMOS\t\t36\tSPICE2/SPICE3-style local model-depth fixture\n",
            "noise\t4\tD,NPN,NJF,NMOS\t\t36\tSPICE2/SPICE3-style local model-depth fixture\n",
            "tran\t4\tD,NPN,NJF,NMOS\t\t40\tSPICE2/SPICE3-style local model-depth fixture"
        )
    );

    let records = device_model_reference_deck_audit_analysis_summary_records(&fixtures);
    assert_eq!(records[0].get("analysis").map(String::as_str), Some("op"));
    assert_eq!(
        records[0].get("fixture_count").map(String::as_str),
        Some("4")
    );
    assert_eq!(
        records[0].get("kinds").map(String::as_str),
        Some("D,NPN,NJF,NMOS")
    );
    assert_eq!(
        records[0].get("missing_kinds").map(String::as_str),
        Some("")
    );
    assert_eq!(records[0].get("deck_lines").map(String::as_str), Some("36"));
    assert_eq!(
        format_device_model_reference_deck_audit_analysis_summary_csv(&fixtures)
            .lines()
            .nth(1),
        Some("op,4,\"D,NPN,NJF,NMOS\",,36,SPICE2/SPICE3-style local model-depth fixture")
    );
    let json = format_device_model_reference_deck_audit_analysis_summary_json(&fixtures);
    assert!(json.starts_with("[{\"analysis\":\"op\""));
    assert!(json.contains("\"analysis\":\"tran\""));
    assert!(json.ends_with("]\n"));
}

#[test]
fn device_model_reference_deck_audit_analysis_summary_reports_missing_kind() {
    let fixtures = device_model_reference_deck_audit_fixtures()
        .unwrap()
        .into_iter()
        .filter(|fixture| !(fixture.kind == ModelCardKind::Nmos && fixture.analysis == "tran"))
        .collect::<Vec<_>>();

    let summary = device_model_reference_deck_audit_analysis_summary(&fixtures);
    let tran = summary
        .iter()
        .find(|row| row.analysis == "tran")
        .expect("transient summary row should exist");

    assert_eq!(tran.fixture_count, 3);
    assert_eq!(tran.kinds, vec!["D", "NPN", "NJF"]);
    assert_eq!(tran.missing_kinds, vec!["NMOS"]);
    assert_eq!(tran.deck_line_count, 30);
    assert!(
        format_device_model_reference_deck_audit_analysis_summary_table(&fixtures).contains(
            "tran\t3\tD,NPN,NJF\tNMOS\t30\tSPICE2/SPICE3-style local model-depth fixture"
        )
    );
}

#[test]
fn device_model_reference_deck_audit_matrix_exports_are_stable() {
    let fixtures = device_model_reference_deck_audit_fixtures().unwrap();
    let matrix = device_model_reference_deck_audit_matrix(&fixtures);

    assert_eq!(matrix.len(), 4);
    assert_eq!(matrix[0].kind, "D");
    assert_eq!(matrix[0].fixture_count, 5);
    assert_eq!(matrix[0].op, "diode-forward-bias:op");
    assert_eq!(matrix[0].temperature, "diode-forward-bias:temperature");
    assert_eq!(matrix[0].ac, "diode-capacitance-ac:ac");
    assert_eq!(matrix[0].noise, "diode-shot-noise:noise");
    assert_eq!(matrix[0].tran, "diode-storage-charge:tran");
    assert!(matrix[0].missing_analyses.is_empty());
    assert!(matrix[0].extra_analyses.is_empty());
    assert_eq!(matrix[0].deck_line_count, 42);

    let table = format_device_model_reference_deck_audit_matrix_table(&fixtures);
    assert_eq!(
        table,
        concat!(
            "kind\tfixture_count\top\ttemperature\tac\tnoise\ttran\tmissing_analyses\textra_analyses\tdeck_lines\n",
            "D\t5\tdiode-forward-bias:op\tdiode-forward-bias:temperature\tdiode-capacitance-ac:ac\tdiode-shot-noise:noise\tdiode-storage-charge:tran\t\t\t42\n",
            "NPN\t5\tbjt-emitter-follower:op\tbjt-emitter-follower:temperature\tbjt-capacitance-ac:ac\tbjt-shot-noise:noise\tbjt-storage-charge:tran\t\t\t47\n",
            "NJF\t5\tjfet-source-bias:op\tjfet-source-bias:temperature\tjfet-capacitance-ac:ac\tjfet-channel-noise:noise\tjfet-storage-charge:tran\t\t\t52\n",
            "NMOS\t5\tmos-level1-common-source:op\tmos-level1-common-source:temperature\tmos-level1-capacitance-ac:ac\tmos-level1-channel-noise:noise\tmos-level1-storage-charge:tran\t\t\t47"
        )
    );

    let records = device_model_reference_deck_audit_matrix_records(&fixtures);
    assert_eq!(records[0].get("kind").map(String::as_str), Some("D"));
    assert_eq!(
        records[0].get("fixture_count").map(String::as_str),
        Some("5")
    );
    assert_eq!(
        records[0].get("op").map(String::as_str),
        Some("diode-forward-bias:op")
    );
    assert_eq!(
        records[0].get("missing_analyses").map(String::as_str),
        Some("")
    );
    assert_eq!(
        format_device_model_reference_deck_audit_matrix_csv(&fixtures)
            .lines()
            .nth(1),
        Some("D,5,diode-forward-bias:op,diode-forward-bias:temperature,diode-capacitance-ac:ac,diode-shot-noise:noise,diode-storage-charge:tran,,,42")
    );
    let json = format_device_model_reference_deck_audit_matrix_json(&fixtures);
    assert!(json.starts_with("[{\"kind\":\"D\""));
    assert!(json.contains("\"tran\":\"mos-level1-storage-charge:tran\""));
    assert!(json.ends_with("]\n"));
}

#[test]
fn device_model_reference_deck_audit_matrix_reports_missing_analysis() {
    let fixtures = device_model_reference_deck_audit_fixtures()
        .unwrap()
        .into_iter()
        .filter(|fixture| !(fixture.kind == ModelCardKind::Nmos && fixture.analysis == "tran"))
        .collect::<Vec<_>>();

    let matrix = device_model_reference_deck_audit_matrix(&fixtures);
    let nmos = matrix
        .iter()
        .find(|row| row.kind == "NMOS")
        .expect("NMOS matrix row should exist");

    assert_eq!(nmos.fixture_count, 4);
    assert_eq!(nmos.tran, "");
    assert_eq!(nmos.missing_analyses, vec!["tran"]);
    assert_eq!(nmos.deck_line_count, 37);
    assert!(format_device_model_reference_deck_audit_matrix_table(&fixtures).contains(
        "NMOS\t4\tmos-level1-common-source:op\tmos-level1-common-source:temperature\tmos-level1-capacitance-ac:ac\tmos-level1-channel-noise:noise\t\ttran\t\t37"
    ));
}

#[test]
fn device_model_reference_deck_audit_gate_report_is_stable() {
    let fixtures = device_model_reference_deck_audit_fixtures().unwrap();
    let report = device_model_reference_deck_audit_gate(&fixtures);

    assert!(report.passed);
    assert_eq!(report.fixture_count, 20);
    assert_eq!(report.expected_kinds, vec!["D", "NPN", "NJF", "NMOS"]);
    assert_eq!(
        report.expected_analyses,
        vec!["op", "temperature", "ac", "noise", "tran"]
    );
    assert!(report.issues.is_empty());
    assert_eq!(
        format_device_model_reference_deck_audit_gate_report(&report),
        "passed\tfixture_count\texpected_kinds\texpected_analyses\tissue_count\ntrue\t20\tD,NPN,NJF,NMOS\top,temperature,ac,noise,tran\t0"
    );
    let digest = device_model_reference_deck_audit_gate_coverage_digest(&report);
    assert!(digest.passed);
    assert_eq!(digest.fixture_count, 20);
    assert_eq!(digest.expected_pair_count, 20);
    assert_eq!(digest.covered_pair_count, 20);
    assert_eq!(digest.missing_pair_count, 0);
    assert_eq!(digest.issue_count, 0);
    assert!(digest.issue_fields.is_empty());
    assert_eq!(
        format_device_model_reference_deck_audit_gate_coverage_digest_table(&report),
        "passed\tfixture_count\texpected_pair_count\tcovered_pair_count\tmissing_pair_count\tissue_count\tissue_fields\ntrue\t20\t20\t20\t0\t0\t"
    );
}

#[test]
fn device_model_reference_deck_audit_gate_reports_missing_coverage() {
    let fixtures = device_model_reference_deck_audit_fixtures()
        .unwrap()
        .into_iter()
        .filter(|fixture| !(fixture.kind == ModelCardKind::Nmos && fixture.analysis == "tran"))
        .collect::<Vec<_>>();

    let report = device_model_reference_deck_audit_gate(&fixtures);
    let table = format_device_model_reference_deck_audit_gate_report(&report);

    assert!(!report.passed);
    assert!(report
        .issues
        .iter()
        .any(|issue| issue.fixture_name == "NMOS:tran" && issue.field == "coverage"));
    assert!(table.contains("fixture_name\tfield\tmessage"));
    assert!(
        table.contains("NMOS:tran\tcoverage\tmissing required NMOS tran reference-deck audit row")
    );

    assert_eq!(
        format_device_model_reference_deck_audit_gate_issue_table(&report),
        "fixture_name\tfield\tmessage\nNMOS:tran\tcoverage\tmissing required NMOS tran reference-deck audit row"
    );
    let records = device_model_reference_deck_audit_gate_issue_records(&report);
    assert_eq!(records.len(), 1);
    assert_eq!(records[0]["fixture_name"], "NMOS:tran");
    assert_eq!(records[0]["field"], "coverage");
    assert_eq!(
        records[0]["message"],
        "missing required NMOS tran reference-deck audit row"
    );
    assert_eq!(
        format_device_model_reference_deck_audit_gate_issue_csv(&report),
        "fixture_name,field,message\nNMOS:tran,coverage,missing required NMOS tran reference-deck audit row\n"
    );
    assert_eq!(
        format_device_model_reference_deck_audit_gate_issue_json(&report),
        "[{\"fixture_name\":\"NMOS:tran\",\"field\":\"coverage\",\"message\":\"missing required NMOS tran reference-deck audit row\"}]\n"
    );
    let summary = device_model_reference_deck_audit_gate_issue_summary(&report);
    assert_eq!(summary.len(), 1);
    assert_eq!(summary[0].field, "coverage");
    assert_eq!(summary[0].issue_count, 1);
    assert_eq!(summary[0].fixture_names, vec!["NMOS:tran"]);
    assert_eq!(
        summary[0].messages,
        vec!["missing required NMOS tran reference-deck audit row"]
    );
    assert_eq!(
        format_device_model_reference_deck_audit_gate_issue_summary_table(&report),
        "field\tissue_count\tfixture_names\tmessages\ncoverage\t1\tNMOS:tran\tmissing required NMOS tran reference-deck audit row"
    );
    let summary_records = device_model_reference_deck_audit_gate_issue_summary_records(&report);
    assert_eq!(summary_records.len(), 1);
    assert_eq!(summary_records[0]["field"], "coverage");
    assert_eq!(summary_records[0]["issue_count"], "1");
    assert_eq!(summary_records[0]["fixture_names"], "NMOS:tran");
    assert_eq!(
        summary_records[0]["messages"],
        "missing required NMOS tran reference-deck audit row"
    );
    assert_eq!(
        format_device_model_reference_deck_audit_gate_issue_summary_csv(&report),
        "field,issue_count,fixture_names,messages\ncoverage,1,NMOS:tran,missing required NMOS tran reference-deck audit row\n"
    );
    assert_eq!(
        format_device_model_reference_deck_audit_gate_issue_summary_json(&report),
        "[{\"field\":\"coverage\",\"issue_count\":\"1\",\"fixture_names\":\"NMOS:tran\",\"messages\":\"missing required NMOS tran reference-deck audit row\"}]\n"
    );
    let digest = device_model_reference_deck_audit_gate_coverage_digest(&report);
    assert!(!digest.passed);
    assert_eq!(digest.fixture_count, 19);
    assert_eq!(digest.expected_pair_count, 20);
    assert_eq!(digest.covered_pair_count, 19);
    assert_eq!(digest.missing_pair_count, 1);
    assert_eq!(digest.issue_count, 1);
    assert_eq!(digest.issue_fields, vec!["coverage"]);
    let digest_records = device_model_reference_deck_audit_gate_coverage_digest_records(&report);
    assert_eq!(digest_records.len(), 1);
    assert_eq!(digest_records[0]["passed"], "false");
    assert_eq!(digest_records[0]["fixture_count"], "19");
    assert_eq!(digest_records[0]["expected_pair_count"], "20");
    assert_eq!(digest_records[0]["covered_pair_count"], "19");
    assert_eq!(digest_records[0]["missing_pair_count"], "1");
    assert_eq!(digest_records[0]["issue_count"], "1");
    assert_eq!(digest_records[0]["issue_fields"], "coverage");
    assert_eq!(
        format_device_model_reference_deck_audit_gate_coverage_digest_table(&report),
        "passed\tfixture_count\texpected_pair_count\tcovered_pair_count\tmissing_pair_count\tissue_count\tissue_fields\nfalse\t19\t20\t19\t1\t1\tcoverage"
    );
    assert_eq!(
        format_device_model_reference_deck_audit_gate_coverage_digest_csv(&report),
        "passed,fixture_count,expected_pair_count,covered_pair_count,missing_pair_count,issue_count,issue_fields\nfalse,19,20,19,1,1,coverage\n"
    );
    assert_eq!(
        format_device_model_reference_deck_audit_gate_coverage_digest_json(&report),
        "[{\"passed\":\"false\",\"fixture_count\":\"19\",\"expected_pair_count\":\"20\",\"covered_pair_count\":\"19\",\"missing_pair_count\":\"1\",\"issue_count\":\"1\",\"issue_fields\":\"coverage\"}]\n"
    );
}

#[test]
fn non_level_one_mos_model_cards_are_explicitly_rejected() {
    let error = normalize_model_card("Mbad", "nmos", &[("LEVEL", 2.0)]).unwrap_err();
    assert!(error
        .to_string()
        .contains("only MOS LEVEL=1 model cards are supported"));
}

#[test]
fn custom_model_linear_conductance_fast_path_stamps_dc_current() {
    let mut circuit = Circuit::new();
    circuit.add(Element::VoltageSource(VoltageSource::new(
        "V1", "in", "0", 1.0,
    )));
    circuit.add(Element::CustomModel(CustomModel::linear_conductance(
        "XG", "in", "0", 2.0e-3,
    )));

    let result = dc_op(&circuit).unwrap();

    assert_close(result.voltage("in").unwrap(), 1.0);
    assert_close(result.branch_current("I(V1)").unwrap(), -2.0e-3);
}

#[test]
fn custom_model_source_analyzer_accepts_subset_and_rejects_dynamic_constructs() {
    let accepted = analyze_custom_model_source(
        "module rlim(p, n); analog begin I(p,n) <+ g * V(p,n); end endmodule",
    );
    let rejected = analyze_custom_model_source(
        "module cap(p, n); analog begin I(p,n) <+ ddt(C * V(p,n)); end endmodule",
    );

    assert!(accepted.accepted);
    assert_eq!(accepted.module_name.as_deref(), Some("rlim"));
    assert_eq!(accepted.terminals, vec!["p".to_string(), "n".to_string()]);
    assert_eq!(
        accepted.contribution,
        Some(("p".to_string(), "n".to_string()))
    );
    assert!(!rejected.accepted);
    assert!(rejected
        .diagnostics
        .iter()
        .any(|diagnostic| diagnostic.code == "CUSTOM_MODEL_FORBIDDEN_CONSTRUCT"));
}

#[test]
fn dc_voltage_divider_solves_midpoint_voltage() {
    let mut circuit = Circuit::new();
    circuit.add(Element::VoltageSource(VoltageSource::new(
        "V1", "vin", "0", 10.0,
    )));
    circuit.add(Element::Resistor(Resistor::new(
        "R1", "vin", "mid", 1_000.0,
    )));
    circuit.add(Element::Resistor(Resistor::new("R2", "mid", "0", 1_000.0)));

    let result = dc_op(&circuit).unwrap();

    assert_close(result.voltage("vin").unwrap(), 10.0);
    assert_close(result.voltage("mid").unwrap(), 5.0);
    assert_close(result.voltage("0").unwrap(), 0.0);
    assert!(result.converged);
    assert_eq!(result.convergence_aid, DcConvergenceAid::Newton);
    assert_eq!(result.iterations, 1);
}

#[test]
fn dc_initial_conditions_seed_operating_point_vector() {
    let mut circuit = Circuit::new();
    circuit.add(Element::VoltageSource(VoltageSource::new(
        "V1", "vin", "0", 10.0,
    )));
    circuit.add(Element::Resistor(Resistor::new(
        "R1", "vin", "mid", 1_000.0,
    )));
    circuit.add(Element::Resistor(Resistor::new("R2", "mid", "0", 1_000.0)));
    let summary = resolve_deck_initial_conditions(
        "
.nodeset V(vin)=10 V(mid)=1
.ic V(mid)=4
.end
",
    );

    let vector =
        dc_initial_vector_from_conditions(&circuit, &summary.initial_conditions, &summary.nodesets)
            .unwrap();
    assert_eq!(vector, vec![4.0, 10.0, 0.0]);

    let result = dc_op_with_initial_conditions(&circuit, &summary, DcOpOptions::default()).unwrap();

    assert!(result.converged);
    assert_close(result.voltage("vin").unwrap(), 10.0);
    assert_close(result.voltage("mid").unwrap(), 5.0);
}

#[test]
fn dc_large_resistor_ladder_uses_sparse_real_solver_path() {
    let mut circuit = Circuit::new();
    circuit.add(Element::VoltageSource(VoltageSource::new(
        "V1", "n0", "0", 10.0,
    )));
    for index in 0..34 {
        circuit.add(Element::Resistor(Resistor::new(
            format!("R{index}"),
            format!("n{index}"),
            format!("n{}", index + 1),
            1_000.0,
        )));
    }
    circuit.add(Element::Resistor(Resistor::new("R34", "n34", "0", 1_000.0)));

    let result = dc_op(&circuit).unwrap();

    assert!(result.converged);
    assert_close(result.voltage("n34").unwrap(), 10.0 / 35.0);
    assert_eq!(result.diagnostics.matrix_size, 36);
    assert_eq!(result.diagnostics.solver, "sparse_real");
    assert_eq!(result.diagnostics.convergence_aid, DcConvergenceAid::Newton);
    assert_close(result.diagnostics.tolerance, 1.0e-9);
    assert!(result.diagnostics.max_delta.is_finite());
    assert_eq!(result.diagnostics.newton_step_limit, None);
    assert_eq!(result.diagnostics.limited_newton_steps, 0);
    assert_close(result.diagnostics.minimum_damping_factor, 1.0);
    assert_eq!(result.diagnostics.solver_profile.matrix_size, 36);
    assert_eq!(result.diagnostics.solver_profile.solver, "sparse_real");
    assert_eq!(
        result.diagnostics.solver_profile.backend,
        "native_sparse_gaussian"
    );
    assert!(result.diagnostics.solver_profile.structural_nonzeros > 0);
    assert!(result.diagnostics.solver_profile.density > 0.0);
    assert!(result.diagnostics.solver_profile.density < 0.1);
    assert!(result.diagnostics.solver_profile.fill_in_nonzeros <= 36 * 36);
    assert!(result.diagnostics.solver_profile.fallback_reason.is_none());
}

#[test]
fn dc_subcircuit_instance_expands_resistor_divider() {
    let mut circuit = Circuit::new();
    circuit
        .define_subcircuit(SubcircuitDefinition::new(
            "atten2",
            vec!["in".to_string(), "out".to_string()],
            vec![
                SubcircuitElement::from(Element::Resistor(Resistor::new(
                    "Rtop", "in", "out", 1_000.0,
                ))),
                SubcircuitElement::from(Element::Resistor(Resistor::new(
                    "Rbot", "out", "0", 1_000.0,
                ))),
            ],
        ))
        .unwrap();
    circuit.add(Element::VoltageSource(VoltageSource::new(
        "V1", "vin", "0", 10.0,
    )));
    circuit
        .instantiate(XInstance::new(
            "X1",
            vec!["vin".to_string(), "vout".to_string()],
            "atten2",
        ))
        .unwrap();

    let result = dc_op(&circuit).unwrap();

    assert_close(result.voltage("vout").unwrap(), 5.0);
    let names: Vec<&str> = circuit
        .elements()
        .iter()
        .filter_map(|element| match element {
            Element::Resistor(resistor) => Some(resistor.name.as_str()),
            _ => None,
        })
        .collect();
    assert_eq!(names, vec!["X1.Rtop", "X1.Rbot"]);
}

#[test]
fn subcircuit_expansion_preserves_complete_diode_model() {
    let mut diode = Diode::with_model_and_temperature_parameters(
        "Dcell",
        "in",
        "0",
        2.0e-14,
        0.026,
        1.2,
        Some(6.0),
        2.0e-6,
        1.5e-12,
        4.0e-9,
        0.8,
        0.4,
        0.35,
        2.2,
        1.05,
    );
    diode.series_resistance = 10.0;
    diode.flicker_noise_coefficient = 1.0e-12;
    diode.flicker_noise_exponent = 1.3;
    let mut circuit = Circuit::new();
    circuit
        .define_subcircuit(SubcircuitDefinition::new(
            "diode-cell",
            vec!["in".to_string()],
            vec![SubcircuitElement::from(Element::Diode(diode))],
        ))
        .unwrap();
    circuit
        .instantiate(XInstance::new("X1", vec!["a".to_string()], "diode-cell"))
        .unwrap();

    let expanded = circuit
        .elements()
        .iter()
        .find_map(|element| match element {
            Element::Diode(diode) => Some(diode),
            _ => None,
        })
        .unwrap();
    assert_close(expanded.junction_potential, 0.8);
    assert_close(expanded.grading_coefficient, 0.4);
    assert_close(expanded.forward_bias_depletion_coefficient, 0.35);
    assert_close(expanded.saturation_current_temperature_exponent, 2.2);
    assert_close(expanded.energy_gap_electron_volts, 1.05);
    assert_close(expanded.series_resistance, 10.0);
    assert_close(expanded.flicker_noise_coefficient, 1.0e-12);
    assert_close(expanded.flicker_noise_exponent, 1.3);
}

#[test]
fn subcircuit_expansion_preserves_complete_jfet_model() {
    let mut jfet = Jfet::new("Jcell", "d", "g", "s");
    jfet.flicker_noise_coefficient = 1.0e-12;
    jfet.flicker_noise_exponent = 1.3;
    jfet.junction_potential = 0.8;
    jfet.forward_bias_depletion_coefficient = 0.35;
    jfet.gate_saturation_current = 2.0e-13;
    jfet.gate_saturation_current_temperature_exponent = 2.5;
    jfet.bandgap_voltage = 1.05;
    jfet.doping_tail_parameter = 1.1;
    jfet.noise_equation_level = 3.0;
    jfet.channel_noise_coefficient = 1.25;
    jfet.drain_resistance = 125.0;
    jfet.source_resistance = 75.0;
    jfet.threshold_voltage_temperature_coefficient = 0.01;
    jfet.alternative_threshold_voltage_temperature_coefficient = Some(-0.0025);
    jfet.nominal_temperature_kelvin = Some(323.15);
    jfet.mobility_temperature_exponent = 1.5;
    jfet.mobility_temperature_coefficient = Some(-0.5);
    let mut circuit = Circuit::new();
    circuit
        .define_subcircuit(SubcircuitDefinition::new(
            "jfet-cell",
            vec!["d".to_string(), "g".to_string(), "s".to_string()],
            vec![SubcircuitElement::from(Element::Jfet(jfet))],
        ))
        .unwrap();
    circuit
        .instantiate(XInstance::new(
            "X1",
            vec!["d1".to_string(), "g1".to_string(), "0".to_string()],
            "jfet-cell",
        ))
        .unwrap();

    let expanded = circuit
        .elements()
        .iter()
        .find_map(|element| match element {
            Element::Jfet(jfet) => Some(jfet),
            _ => None,
        })
        .unwrap();
    assert_close(expanded.flicker_noise_coefficient, 1.0e-12);
    assert_close(expanded.flicker_noise_exponent, 1.3);
    assert_close(expanded.junction_potential, 0.8);
    assert_close(expanded.forward_bias_depletion_coefficient, 0.35);
    assert_close(expanded.gate_saturation_current, 2.0e-13);
    assert_close(expanded.gate_saturation_current_temperature_exponent, 2.5);
    assert_close(expanded.bandgap_voltage, 1.05);
    assert_close(expanded.doping_tail_parameter, 1.1);
    assert_close(expanded.noise_equation_level, 3.0);
    assert_close(expanded.channel_noise_coefficient, 1.25);
    assert_close(expanded.drain_resistance, 125.0);
    assert_close(expanded.source_resistance, 75.0);
    assert_close(expanded.threshold_voltage_temperature_coefficient, 0.01);
    assert_close(
        expanded
            .alternative_threshold_voltage_temperature_coefficient
            .unwrap(),
        -0.0025,
    );
    assert_close(expanded.nominal_temperature_kelvin.unwrap(), 323.15);
    assert_close(expanded.mobility_temperature_exponent, 1.5);
    assert_close(expanded.mobility_temperature_coefficient.unwrap(), -0.5);
}

#[test]
fn dc_jfet_drain_resistance_drops_intrinsic_drain_voltage() {
    let mut circuit = Circuit::new();
    circuit.add(Element::VoltageSource(VoltageSource::new(
        "Vdrain", "drain", "0", 5.0,
    )));
    circuit.add(Element::VoltageSource(VoltageSource::new(
        "Vgate", "gate", "0", 0.0,
    )));
    let mut jfet = Jfet::new("J1", "drain", "gate", "0");
    jfet.beta = 1.0e-3;
    jfet.drain_resistance = 1_000.0;
    circuit.add(Element::Jfet(jfet));

    let result = dc_op(&circuit).unwrap();
    assert_close(result.node_voltages["drain"], 5.0);
    assert!(result.node_voltages["__spice_J1_drain"] < 5.0);
}

#[test]
fn dc_mosfet_drain_resistance_drops_intrinsic_drain_voltage() {
    let mut circuit = Circuit::new();
    circuit.add(Element::VoltageSource(VoltageSource::new(
        "Vdrain", "drain", "0", 5.0,
    )));
    circuit.add(Element::VoltageSource(VoltageSource::new(
        "Vgate", "gate", "0", 3.0,
    )));
    circuit.add(Element::Mosfet(Mosfet::with_model(
        "M1",
        "drain",
        "gate",
        "0",
        "0",
        MosfetType::Nmos,
        MosfetLevel1Params {
            drain_resistance: 1_000.0,
            ..MosfetLevel1Params::default()
        },
    )));

    let result = dc_op(&circuit).unwrap();
    assert_close(result.node_voltages["drain"], 5.0);
    assert!(result.node_voltages["__spice_M1_drain"] < 5.0);
}

#[test]
fn dc_mosfet_bulk_junction_saturation_current_sets_reverse_leakage() {
    let bias_current = |mosfet_type: MosfetType, bias_voltage: f64, saturation_current: f64| {
        let mut circuit = Circuit::new();
        circuit.add(Element::VoltageSource(VoltageSource::new(
            "Vbias",
            "body",
            "0",
            bias_voltage,
        )));
        circuit.add(Element::Mosfet(Mosfet::with_model(
            "M1",
            "0",
            "0",
            "0",
            "body",
            mosfet_type,
            MosfetLevel1Params {
                saturation_current,
                ..MosfetLevel1Params::default()
            },
        )));
        dc_op(&circuit)
            .unwrap()
            .branch_current("Vbias")
            .unwrap()
            .abs()
    };

    for (mosfet_type, bias_voltage) in [(MosfetType::Nmos, -0.3), (MosfetType::Pmos, 0.3)] {
        let unloaded = bias_current(mosfet_type, bias_voltage, 1.0e-30);
        let loaded = bias_current(mosfet_type, bias_voltage, 1.0e-12);
        assert!(loaded > unloaded);
    }
}

#[test]
fn dc_mosfet_bulk_junction_current_density_scales_complete_diffusion_areas() {
    let bias_current = |saturation_current: f64,
                        saturation_current_density: f64,
                        drain_area: f64,
                        source_area: f64| {
        let mut circuit = Circuit::new();
        circuit.add(Element::VoltageSource(VoltageSource::new(
            "Vbias", "body", "0", -0.3,
        )));
        circuit.add(Element::Mosfet(Mosfet::with_model(
            "M1",
            "0",
            "0",
            "0",
            "body",
            MosfetType::Nmos,
            MosfetLevel1Params {
                saturation_current,
                saturation_current_density,
                drain_area,
                source_area,
                ..MosfetLevel1Params::default()
            },
        )));
        dc_op(&circuit)
            .unwrap()
            .branch_current("Vbias")
            .unwrap()
            .abs()
    };

    let density_scaled = bias_current(1.0e-30, 1.0, 2.0e-12, 3.0e-12);
    let equivalent_scalar = bias_current(2.5e-12, 0.0, 2.0e-12, 3.0e-12);
    assert!((density_scaled / equivalent_scalar - 1.0).abs() < 1.0e-9);

    let incomplete_areas = bias_current(4.0e-13, 1.0, 0.0, 3.0e-12);
    let scalar_fallback = bias_current(4.0e-13, 0.0, 0.0, 3.0e-12);
    assert!((incomplete_areas / scalar_fallback - 1.0).abs() < 1.0e-9);
}

#[test]
fn dc_mosfet_source_resistance_raises_intrinsic_source_voltage() {
    let mut circuit = Circuit::new();
    circuit.add(Element::VoltageSource(VoltageSource::new(
        "Vdrain", "drain", "0", 5.0,
    )));
    circuit.add(Element::VoltageSource(VoltageSource::new(
        "Vgate", "gate", "0", 3.0,
    )));
    circuit.add(Element::Mosfet(Mosfet::with_model(
        "M1",
        "drain",
        "gate",
        "0",
        "0",
        MosfetType::Nmos,
        MosfetLevel1Params {
            source_resistance: 1_000.0,
            ..MosfetLevel1Params::default()
        },
    )));

    let result = dc_op(&circuit).unwrap();
    assert!(result.node_voltages["__spice_M1_source"] > 0.0);
}

#[test]
fn dc_mosfet_sheet_resistance_biases_both_intrinsic_terminals() {
    let mut circuit = Circuit::new();
    circuit.add(Element::VoltageSource(VoltageSource::new(
        "Vdrain", "drain", "0", 5.0,
    )));
    circuit.add(Element::VoltageSource(VoltageSource::new(
        "Vgate", "gate", "0", 3.0,
    )));
    circuit.add(Element::Mosfet(Mosfet::with_model(
        "M1",
        "drain",
        "gate",
        "0",
        "0",
        MosfetType::Nmos,
        MosfetLevel1Params {
            sheet_resistance: 1_000.0,
            drain_squares: 2.0,
            source_squares: 3.0,
            ..MosfetLevel1Params::default()
        },
    )));

    let result = dc_op(&circuit).unwrap();
    assert!(result.node_voltages["__spice_M1_drain"] < 5.0);
    assert!(result.node_voltages["__spice_M1_source"] > 0.0);
}

#[test]
fn dc_jfet_source_resistance_raises_intrinsic_source_voltage() {
    let mut circuit = Circuit::new();
    circuit.add(Element::VoltageSource(VoltageSource::new(
        "Vdrain", "drain", "0", 5.0,
    )));
    circuit.add(Element::VoltageSource(VoltageSource::new(
        "Vgate", "gate", "0", 3.0,
    )));
    let mut jfet = Jfet::new("J1", "drain", "gate", "0");
    jfet.beta = 1.0e-3;
    jfet.source_resistance = 1_000.0;
    circuit.add(Element::Jfet(jfet));

    let result = dc_op(&circuit).unwrap();
    assert!(result.node_voltages["__spice_J1_source"] > 0.0);
}

#[test]
fn dc_jfet_doping_tail_parameter_shapes_linear_and_saturation_current() {
    let drain_current = |drain_voltage: f64, doping_tail_parameter: f64| {
        let mut circuit = Circuit::new();
        circuit.add(Element::VoltageSource(VoltageSource::new(
            "Vdrain",
            "drain",
            "0",
            drain_voltage,
        )));
        circuit.add(Element::VoltageSource(VoltageSource::new(
            "Vgate", "gate", "0", 0.0,
        )));
        let mut jfet = Jfet::new("J1", "drain", "gate", "0");
        jfet.beta = 1.0e-3;
        jfet.threshold_voltage = -2.0;
        jfet.junction_potential = 1.0;
        jfet.doping_tail_parameter = doping_tail_parameter;
        circuit.add(Element::Jfet(jfet));
        dc_op(&circuit)
            .unwrap()
            .branch_current("Vdrain")
            .unwrap()
            .abs()
    };

    assert!(drain_current(1.0, 1.1) > drain_current(1.0, 1.0));
    assert!(drain_current(3.0, 1.1) > drain_current(3.0, 1.0));
}

#[test]
fn jfet_gate_saturation_current_loads_a_forward_biased_gate() {
    let gate_voltage = |polarity: JfetPolarity, bias_voltage: f64, gate_saturation_current: f64| {
        let mut circuit = Circuit::new();
        circuit.add(Element::VoltageSource(VoltageSource::new(
            "Vbias",
            "bias",
            "0",
            bias_voltage,
        )));
        circuit.add(Element::Resistor(Resistor::new(
            "Rgate", "bias", "gate", 1.0e6,
        )));
        let mut jfet = Jfet::with_model(
            "J1",
            "0",
            "gate",
            "0",
            polarity,
            1.0e-4,
            if polarity == JfetPolarity::Njf {
                -2.0
            } else {
                2.0
            },
            0.0,
        );
        jfet.gate_saturation_current = gate_saturation_current;
        circuit.add(Element::Jfet(jfet));
        dc_op(&circuit).unwrap().node_voltages["gate"]
    };

    assert!(
        gate_voltage(JfetPolarity::Njf, 0.3, 1.0e-9)
            < gate_voltage(JfetPolarity::Njf, 0.3, 1.0e-14)
    );
    assert!(
        gate_voltage(JfetPolarity::Pjf, -0.3, 1.0e-9)
            > gate_voltage(JfetPolarity::Pjf, -0.3, 1.0e-14)
    );
}

#[test]
fn subcircuit_expansion_preserves_mos_geometry() {
    let mosfet = Mosfet::with_model(
        "Mcell",
        "d",
        "g",
        "s",
        "b",
        MosfetType::Nmos,
        MosfetLevel1Params {
            l: 1.0e-6,
            lateral_diffusion_length: 0.1e-6,
            drain_resistance: 125.0,
            source_resistance: 75.0,
            sheet_resistance: 50.0,
            drain_squares: 2.0,
            source_squares: 3.0,
            drain_area: 4.0e-12,
            source_area: 5.0e-12,
            source_perimeter: 6.0e-6,
            sidewall_junction_grading_coefficient: 0.25,
            bottom_junction_capacitance: 2.0e-3,
            oxide_thickness: 25.0e-9,
            ..MosfetLevel1Params::default()
        },
    );
    let mut circuit = Circuit::new();
    circuit
        .define_subcircuit(SubcircuitDefinition::new(
            "mos-cell",
            vec![
                "d".to_string(),
                "g".to_string(),
                "s".to_string(),
                "b".to_string(),
            ],
            vec![SubcircuitElement::from(Element::Mosfet(mosfet))],
        ))
        .unwrap();
    circuit
        .instantiate(XInstance::new(
            "X1",
            vec![
                "d1".to_string(),
                "g1".to_string(),
                "0".to_string(),
                "0".to_string(),
            ],
            "mos-cell",
        ))
        .unwrap();

    let expanded = circuit
        .elements()
        .iter()
        .find_map(|element| match element {
            Element::Mosfet(mosfet) => Some(mosfet),
            _ => None,
        })
        .unwrap();
    assert_close(expanded.params.l, 1.0e-6);
    assert_close(expanded.params.lateral_diffusion_length, 0.1e-6);
    assert_close(expanded.params.drain_resistance, 125.0);
    assert_close(expanded.params.source_resistance, 75.0);
    assert_close(expanded.params.sheet_resistance, 50.0);
    assert_close(expanded.params.drain_squares, 2.0);
    assert_close(expanded.params.source_squares, 3.0);
    assert_close(expanded.params.drain_area, 4.0e-12);
    assert_close(expanded.params.source_area, 5.0e-12);
    assert_close(expanded.params.source_perimeter, 6.0e-6);
    assert_close(expanded.params.sidewall_junction_grading_coefficient, 0.25);
    assert_close(expanded.params.bottom_junction_capacitance, 2.0e-3);
    assert_close(expanded.params.oxide_thickness, 25.0e-9);
}

#[test]
fn subcircuit_expansion_preserves_complete_bjt_model() {
    let mut bjt = Bjt::with_model_temperature_depletion_early_rolloff_junction_leakage_and_reverse_beta_parameters(
        "Qcell",
        "c",
        "b",
        "e",
        BjtPolarity::Npn,
        2.0e-14,
        125.0,
        0.026,
        1.0e-12,
        2.0e-12,
        3.0e-9,
        4.0e-9,
        2.4,
        1.05,
        80.0,
        1.2,
        1.3,
        0.8,
        0.4,
        0.7,
        0.45,
        0.4,
        120.0,
        2.0e-3,
        3.0e-13,
        1.7,
        4.0e-13,
        1.8,
        1.5,
        0.25,
    );
    bjt.reverse_beta_rolloff_current = 3.0e-3;
    bjt.nominal_temperature_kelvin = Some(323.15);
    bjt.flicker_noise_coefficient = 1.0e-12;
    bjt.flicker_noise_exponent = 1.3;
    bjt.forward_excess_phase_degrees = 30.0;
    bjt.forward_transit_time_bias_coefficient = 2.0;
    bjt.forward_transit_time_current = 4.0e-3;
    bjt.forward_transit_time_voltage = 0.6;
    bjt.emitter_resistance = 12.0;
    bjt.collector_resistance = 13.0;
    bjt.base_resistance = 14.0;
    bjt.minimum_base_resistance = Some(2.0);
    bjt.base_resistance_half_current = 5.0e-6;
    bjt.base_collector_capacitance_fraction = 0.4;
    let mut circuit = Circuit::new();
    circuit
        .define_subcircuit(SubcircuitDefinition::new(
            "bjt-cell",
            vec!["c".to_string(), "b".to_string(), "e".to_string()],
            vec![SubcircuitElement::from(Element::Bjt(bjt))],
        ))
        .unwrap();
    circuit
        .instantiate(XInstance::new(
            "X1",
            vec!["c1".to_string(), "b1".to_string(), "0".to_string()],
            "bjt-cell",
        ))
        .unwrap();

    let expanded = circuit
        .elements()
        .iter()
        .find_map(|element| match element {
            Element::Bjt(bjt) => Some(bjt),
            _ => None,
        })
        .unwrap();
    assert_close(expanded.saturation_current_temperature_exponent, 2.4);
    assert_close(expanded.energy_gap_electron_volts, 1.05);
    assert_close(expanded.forward_early_voltage, 80.0);
    assert_close(expanded.reverse_early_voltage, 120.0);
    assert_close(expanded.forward_emission_coefficient, 1.2);
    assert_close(expanded.reverse_emission_coefficient, 1.3);
    assert_close(expanded.base_emitter_junction_potential, 0.8);
    assert_close(expanded.base_emitter_grading_coefficient, 0.4);
    assert_close(expanded.base_collector_junction_potential, 0.7);
    assert_close(expanded.base_collector_grading_coefficient, 0.45);
    assert_close(expanded.forward_bias_depletion_coefficient, 0.4);
    assert_close(expanded.forward_beta_rolloff_current, 2.0e-3);
    assert_close(expanded.base_emitter_leakage_saturation_current, 3.0e-13);
    assert_close(expanded.base_emitter_leakage_emission_coefficient, 1.7);
    assert_close(expanded.base_collector_leakage_saturation_current, 4.0e-13);
    assert_close(expanded.base_collector_leakage_emission_coefficient, 1.8);
    assert_close(expanded.forward_beta_temperature_exponent, 1.5);
    assert_close(expanded.reverse_beta, 0.25);
    assert_close(expanded.reverse_beta_rolloff_current, 3.0e-3);
    assert_close(expanded.nominal_temperature_kelvin.unwrap(), 323.15);
    assert_close(expanded.flicker_noise_coefficient, 1.0e-12);
    assert_close(expanded.flicker_noise_exponent, 1.3);
    assert_close(expanded.forward_excess_phase_degrees, 30.0);
    assert_close(expanded.forward_transit_time_bias_coefficient, 2.0);
    assert_close(expanded.forward_transit_time_current, 4.0e-3);
    assert_close(expanded.forward_transit_time_voltage, 0.6);
    assert_close(expanded.emitter_resistance, 12.0);
    assert_close(expanded.collector_resistance, 13.0);
    assert_close(expanded.base_resistance, 14.0);
    assert_close(expanded.minimum_base_resistance.unwrap(), 2.0);
    assert_close(expanded.base_resistance_half_current, 5.0e-6);
    assert_close(expanded.base_collector_capacitance_fraction, 0.4);
}

#[test]
fn dc_current_source_into_resistor_uses_positive_to_negative_orientation() {
    let mut circuit = Circuit::new();
    circuit.add(Element::CurrentSource(CurrentSource::new(
        "I1", "0", "n1", 1.0e-3,
    )));
    circuit.add(Element::Resistor(Resistor::new("R1", "n1", "0", 1_000.0)));

    let result = dc_op(&circuit).unwrap();

    assert_close(result.voltage("n1").unwrap(), 1.0);
}

#[test]
fn dc_behavioral_current_source_tracks_node_voltage() {
    let mut circuit = Circuit::new();
    circuit.add(Element::VoltageSource(VoltageSource::new(
        "Vin", "in", "0", 2.0,
    )));
    circuit.add(Element::BSource(BSource::current(
        "B1",
        "0",
        "out",
        "0.002 * V(in)",
    )));
    circuit.add(Element::Resistor(Resistor::new(
        "Rload", "out", "0", 1_000.0,
    )));

    let result = dc_op(&circuit).unwrap();

    assert!(result.converged);
    assert_close(result.voltage("out").unwrap(), 4.0);
}

#[test]
fn dc_behavioral_voltage_source_tracks_differential_voltage() {
    let mut circuit = Circuit::new();
    circuit.add(Element::VoltageSource(VoltageSource::new(
        "Vin", "in", "0", 3.0,
    )));
    circuit.add(Element::BSource(BSource::voltage(
        "B1",
        "out",
        "0",
        "2.0 * V(in, 0) + 1.0",
    )));
    circuit.add(Element::Resistor(Resistor::new(
        "Rload", "out", "0", 1_000.0,
    )));

    let result = dc_op(&circuit).unwrap();

    assert!(result.converged);
    assert_close(result.voltage("out").unwrap(), 7.0);
    assert_close(result.branch_current("B1").unwrap(), -7.0e-3);
}

#[test]
fn dc_vccs_injects_current_from_control_voltage() {
    let mut circuit = Circuit::new();
    circuit.add(Element::VoltageSource(VoltageSource::new(
        "Vctrl", "ctrl", "0", 2.0,
    )));
    circuit.add(Element::Vccs(Vccs::new(
        "G1", "0", "out", "ctrl", "0", 1.0e-3,
    )));
    circuit.add(Element::Resistor(Resistor::new(
        "Rload", "out", "0", 1_000.0,
    )));

    let result = dc_op(&circuit).unwrap();

    assert_close(result.voltage("out").unwrap(), 2.0);
}

#[test]
fn dc_vcvs_sets_output_from_control_voltage() {
    let mut circuit = Circuit::new();
    circuit.add(Element::VoltageSource(VoltageSource::new(
        "Vctrl", "ctrl", "0", 1.5,
    )));
    circuit.add(Element::Vcvs(Vcvs::new("E1", "out", "0", "ctrl", "0", 2.0)));
    circuit.add(Element::Resistor(Resistor::new(
        "Rload", "out", "0", 1_000.0,
    )));

    let result = dc_op(&circuit).unwrap();

    assert_close(result.voltage("out").unwrap(), 3.0);
    assert_close(result.branch_current("E1").unwrap(), -3.0e-3);
}

#[test]
fn dc_vcvs_respects_differential_control_polarity() {
    let mut circuit = Circuit::new();
    circuit.add(Element::VoltageSource(VoltageSource::new(
        "Vp", "p", "0", 4.0,
    )));
    circuit.add(Element::VoltageSource(VoltageSource::new(
        "Vn", "n", "0", 1.0,
    )));
    circuit.add(Element::Vcvs(Vcvs::new("E1", "out", "0", "p", "n", 0.5)));
    circuit.add(Element::Resistor(Resistor::new(
        "Rload", "out", "0", 1_000.0,
    )));

    let result = dc_op(&circuit).unwrap();

    assert_close(result.voltage("out").unwrap(), 1.5);
}

#[test]
fn dc_cccs_injects_current_from_voltage_source_branch_current() {
    let mut circuit = Circuit::new();
    circuit.add(Element::VoltageSource(VoltageSource::new(
        "Vin", "in", "0", 1.0,
    )));
    circuit.add(Element::Resistor(Resistor::new(
        "Rsense", "in", "sense", 1_000.0,
    )));
    circuit.add(Element::VoltageSource(VoltageSource::new(
        "Vsense", "sense", "0", 0.0,
    )));
    circuit.add(Element::Cccs(Cccs::new("F1", "0", "out", "Vsense", 2.0)));
    circuit.add(Element::Resistor(Resistor::new(
        "Rload", "out", "0", 1_000.0,
    )));

    let result = dc_op(&circuit).unwrap();

    assert_close(result.branch_current("Vsense").unwrap(), 1.0e-3);
    assert_close(result.voltage("out").unwrap(), 2.0);
}

#[test]
fn dc_ccvs_sets_voltage_from_voltage_source_branch_current() {
    let mut circuit = Circuit::new();
    circuit.add(Element::VoltageSource(VoltageSource::new(
        "Vin", "in", "0", 1.0,
    )));
    circuit.add(Element::Resistor(Resistor::new(
        "Rsense", "in", "sense", 1_000.0,
    )));
    circuit.add(Element::VoltageSource(VoltageSource::new(
        "Vsense", "sense", "0", 0.0,
    )));
    circuit.add(Element::Ccvs(Ccvs::new(
        "H1", "out", "0", "Vsense", 2_000.0,
    )));
    circuit.add(Element::Resistor(Resistor::new(
        "Rload", "out", "0", 1_000.0,
    )));

    let result = dc_op(&circuit).unwrap();

    assert_close(result.branch_current("Vsense").unwrap(), 1.0e-3);
    assert_close(result.voltage("out").unwrap(), 2.0);
    assert_close(result.branch_current("H1").unwrap(), -2.0e-3);
}

#[test]
fn dc_diode_solves_forward_biased_operating_point() {
    let mut circuit = Circuit::new();
    circuit.add(Element::VoltageSource(VoltageSource::new(
        "Vin", "in", "0", 0.7,
    )));
    circuit.add(Element::Diode(Diode::with_model(
        "D1", "in", "out", 1.0e-12, 0.025,
    )));
    circuit.add(Element::Resistor(Resistor::new(
        "Rload", "out", "0", 1_000.0,
    )));

    let result = dc_op(&circuit).unwrap();
    let out = result.voltage("out").unwrap();
    assert!(out > 0.1, "expected forward-biased output, got {out}");
    assert!(out < 0.7, "expected diode drop below source, got {out}");
}

#[test]
fn dc_diode_emission_coefficient_reduces_fixed_bias_current() {
    let mut base = Circuit::new();
    base.add(Element::VoltageSource(VoltageSource::new(
        "V1", "a", "0", 0.7,
    )));
    base.add(Element::Diode(Diode::with_model(
        "D1", "a", "0", 1.0e-15, 0.02585,
    )));

    let mut high_n = Circuit::new();
    high_n.add(Element::VoltageSource(VoltageSource::new(
        "V1", "a", "0", 0.7,
    )));
    high_n.add(Element::Diode(Diode::with_model_and_emission_coefficient(
        "D1", "a", "0", 1.0e-15, 0.02585, 2.0,
    )));

    let base_result = dc_op(&base).unwrap();
    let high_n_result = dc_op(&high_n).unwrap();
    assert!(
        high_n_result.branch_current("V1").unwrap().abs()
            < base_result.branch_current("V1").unwrap().abs() * 1.0e-3
    );
}

#[test]
fn dc_diode_series_resistance_limits_fixed_bias_current() {
    let mut ideal = Circuit::new();
    ideal.add(Element::VoltageSource(VoltageSource::new(
        "V1", "a", "0", 0.7,
    )));
    ideal.add(Element::Diode(Diode::new("D1", "a", "0")));

    let mut limited = Circuit::new();
    limited.add(Element::VoltageSource(VoltageSource::new(
        "V1", "a", "0", 0.7,
    )));
    let mut diode = Diode::new("D1", "a", "0");
    diode.series_resistance = 100.0;
    limited.add(Element::Diode(diode));

    let ideal_current = dc_op(&ideal).unwrap().branch_current("V1").unwrap().abs();
    let limited_current = dc_op(&limited).unwrap().branch_current("V1").unwrap().abs();
    assert!(limited_current < ideal_current);
    assert!(limited_current <= 0.7 / 100.0);
}

#[test]
fn dc_diode_breakdown_voltage_increases_reverse_bias_current() {
    let mut leakage = Circuit::new();
    leakage.add(Element::VoltageSource(VoltageSource::new(
        "V1", "0", "a", 5.0,
    )));
    leakage.add(Element::Diode(Diode::with_model(
        "D1", "a", "0", 1.0e-15, 0.02585,
    )));

    let mut breakdown = Circuit::new();
    breakdown.add(Element::VoltageSource(VoltageSource::new(
        "V1", "0", "a", 5.0,
    )));
    breakdown.add(Element::Diode(Diode::with_model_and_breakdown(
        "D1",
        "a",
        "0",
        1.0e-15,
        0.02585,
        1.0,
        Some(5.0),
        1.0e-6,
        0.0,
        0.0,
    )));

    let leakage_result = dc_op(&leakage).unwrap();
    let breakdown_result = dc_op(&breakdown).unwrap();
    assert!(
        breakdown_result.branch_current("V1").unwrap().abs()
            > leakage_result.branch_current("V1").unwrap().abs() * 1.0e6
    );
    assert!((breakdown_result.branch_current("V1").unwrap().abs() - 1.0e-6).abs() < 1.0e-9);
}

#[test]
fn dc_diode_temperature_scaling_reduces_fixed_current_forward_drop() {
    let mut nominal = Circuit::new();
    nominal.add(Element::VoltageSource(VoltageSource::new(
        "V1", "vcc", "0", 5.0,
    )));
    nominal.add(Element::Resistor(Resistor::new(
        "Rbias", "vcc", "a", 4_300.0,
    )));
    nominal.add(Element::Diode(Diode::with_model(
        "D1", "a", "0", 1.0e-15, 0.02585,
    )));

    let cold = circuit_at_temperature(&nominal, 275.0, 300.15, 1.11).unwrap();
    let hot = circuit_at_temperature(&nominal, 350.0, 300.15, 1.11).unwrap();

    let nominal_result = dc_op(&nominal).unwrap();
    let cold_result = dc_op(&cold).unwrap();
    let hot_result = dc_op(&hot).unwrap();

    assert!(cold_result.voltage("a").unwrap() > nominal_result.voltage("a").unwrap());
    assert!(hot_result.voltage("a").unwrap() < nominal_result.voltage("a").unwrap());
}

#[test]
fn diode_temperature_scaling_uses_model_saturation_current_exponent() {
    let default_xti = Diode::with_model_and_temperature_exponent(
        "D1", "a", "0", 1.0e-15, 0.02585, 1.0, None, 1.0e-3, 0.0, 0.0, 1.0, 0.5, 0.5, 3.0,
    );
    let flat_xti = Diode::with_model_and_temperature_exponent(
        "D1", "a", "0", 1.0e-15, 0.02585, 1.0, None, 1.0e-3, 0.0, 0.0, 1.0, 0.5, 0.5, 0.0,
    );

    let temperature_kelvin = 350.0;
    let nominal_temperature_kelvin = 300.15;
    let default_hot = diode_at_temperature(
        &default_xti,
        temperature_kelvin,
        nominal_temperature_kelvin,
        1.11,
    )
    .unwrap();
    let flat_hot = diode_at_temperature(
        &flat_xti,
        temperature_kelvin,
        nominal_temperature_kelvin,
        1.11,
    )
    .unwrap();

    assert_close(
        default_hot.saturation_current / flat_hot.saturation_current,
        (temperature_kelvin / nominal_temperature_kelvin).powi(3),
    );
}

#[test]
fn circuit_temperature_scaling_uses_model_energy_gap() {
    let mut silicon = Circuit::new();
    silicon.add(Element::Diode(
        Diode::with_model_and_temperature_parameters(
            "D1", "a", "0", 1.0e-15, 0.02585, 1.0, None, 1.0e-3, 0.0, 0.0, 1.0, 0.5, 0.5, 3.0, 1.11,
        ),
    ));
    let mut lower_gap = Circuit::new();
    lower_gap.add(Element::Diode(
        Diode::with_model_and_temperature_parameters(
            "D1", "a", "0", 1.0e-15, 0.02585, 1.0, None, 1.0e-3, 0.0, 0.0, 1.0, 0.5, 0.5, 3.0, 0.8,
        ),
    ));

    let silicon_hot = circuit_at_temperature(&silicon, 350.0, 300.15, 1.11).unwrap();
    let lower_gap_hot = circuit_at_temperature(&lower_gap, 350.0, 300.15, 1.11).unwrap();
    let saturation_current = |circuit: &Circuit| match &circuit.elements()[0] {
        Element::Diode(diode) => diode.saturation_current,
        _ => unreachable!(),
    };

    assert!(saturation_current(&silicon_hot) > saturation_current(&lower_gap_hot));
}

#[test]
fn dc_temperature_sweep_text_output_table_is_stable() {
    let mut circuit = Circuit::new();
    circuit.add(Element::VoltageSource(VoltageSource::new(
        "V1", "vcc", "0", 5.0,
    )));
    circuit.add(Element::Resistor(Resistor::new(
        "Rbias", "vcc", "a", 4_300.0,
    )));
    circuit.add(Element::Diode(Diode::with_model(
        "D1", "a", "0", 1.0e-15, 0.02585,
    )));

    let result = dc_temperature_sweep(
        &circuit,
        &[275.0, 300.15, 350.0],
        300.15,
        1.11,
        DcOpOptions::default(),
    )
    .unwrap();

    let _: TemperatureDcResult = result.clone();
    assert!(
        result.points[0].result.voltage("a").unwrap()
            > result.points[1].result.voltage("a").unwrap()
    );
    assert!(
        result.points[2].result.voltage("a").unwrap()
            < result.points[1].result.voltage("a").unwrap()
    );
    assert_eq!(
        format_temperature_dc_table(&result, &["V(a)", "I(V1)"]).unwrap(),
        "Index\tTemperatureKelvin\tV(a)\tI(V1)\n0\t2.750000e+02\t4.560039e+00\t-1.023164e-04\n1\t3.001500e+02\t3.613836e+00\t-3.223638e-04\n2\t3.500000e+02\t6.351989e-01\t-1.015070e-03\n"
    );
}

#[test]
fn corner_temperature_dc_text_output_table_is_stable() {
    let mut circuit = Circuit::new();
    circuit.add(Element::VoltageSource(VoltageSource::new(
        "V1", "vcc", "0", 5.0,
    )));
    circuit.add(Element::Resistor(Resistor::new(
        "Rbias", "vcc", "a", 4_300.0,
    )));
    circuit.add(Element::Diode(Diode::with_model(
        "D1", "a", "0", 1.0e-15, 0.02585,
    )));

    let result = dc_temperature_sweep_corners(
        &circuit,
        &[275.0, 350.0],
        300.15,
        1.11,
        DcOpOptions::default(),
        &[
            CornerSpec::new("nominal", vec![]),
            CornerSpec::new(
                "rbias-high",
                vec![CornerOverride::new("Rbias", "resistance", 8_600.0)],
            ),
        ],
    )
    .unwrap();

    let _: CornerTemperatureDcResult = result.clone();
    assert_eq!(result.points[0].corner_name, "nominal");
    assert_eq!(result.points[1].corner_name, "rbias-high");
    assert!(
        result.points[0].points[0].result.voltage("a").unwrap()
            > result.points[0].points[1].result.voltage("a").unwrap()
    );
    assert_eq!(
        format_corner_temperature_dc_table(&result, &["V(a)", "I(V1)"]).unwrap(),
        "Corner\tIndex\tTemperatureKelvin\tV(a)\tI(V1)\nnominal\t0\t2.750000e+02\t4.560039e+00\t-1.023164e-04\nnominal\t1\t3.500000e+02\t6.351989e-01\t-1.015070e-03\nrbias-high\t0\t2.750000e+02\t4.218594e+00\t-9.086118e-05\nrbias-high\t1\t3.500000e+02\t6.144482e-01\t-5.099479e-04\n"
    );
}

#[test]
fn dc_bjt_temperature_scaling_reduces_emitter_follower_forward_drop() {
    let mut nominal = Circuit::new();
    nominal.add(Element::VoltageSource(VoltageSource::new(
        "Vcc", "vcc", "0", 5.0,
    )));
    nominal.add(Element::VoltageSource(VoltageSource::new(
        "Vbase", "base", "0", 0.72,
    )));
    nominal.add(Element::Bjt(Bjt::with_model(
        "Q1",
        "vcc",
        "base",
        "out",
        BjtPolarity::Npn,
        1.0e-14,
        120.0,
        0.02585,
        0.0,
        0.0,
        0.0,
        0.0,
    )));
    nominal.add(Element::Resistor(Resistor::new(
        "Rload", "out", "0", 1_000.0,
    )));

    let cold = circuit_at_temperature(&nominal, 275.0, 300.15, 1.11).unwrap();
    let hot = circuit_at_temperature(&nominal, 350.0, 300.15, 1.11).unwrap();

    let nominal_result = dc_op(&nominal).unwrap();
    let cold_result = dc_op(&cold).unwrap();
    let hot_result = dc_op(&hot).unwrap();

    assert!(cold_result.voltage("out").unwrap() < nominal_result.voltage("out").unwrap());
    assert!(hot_result.voltage("out").unwrap() > nominal_result.voltage("out").unwrap());
}

#[test]
fn bjt_temperature_scaling_uses_model_temperature_exponent() {
    let low_exponent = Bjt::with_model_and_temperature_parameters(
        "Qlow",
        "c",
        "b",
        "e",
        BjtPolarity::Npn,
        1.0e-14,
        100.0,
        0.02585,
        0.0,
        0.0,
        0.0,
        0.0,
        0.0,
        1.11,
        0.0,
        1.0,
        1.0,
    );
    let high_exponent = Bjt::with_model_and_temperature_parameters(
        "Qhigh",
        "c",
        "b",
        "e",
        BjtPolarity::Npn,
        1.0e-14,
        100.0,
        0.02585,
        0.0,
        0.0,
        0.0,
        0.0,
        4.0,
        1.11,
        0.0,
        1.0,
        1.0,
    );
    let low = bjt_at_temperature(&low_exponent, 350.0, 300.15, 1.11).unwrap();
    let high = bjt_at_temperature(&high_exponent, 350.0, 300.15, 1.11).unwrap();
    assert!(high.saturation_current > low.saturation_current);
}

#[test]
fn bjt_temperature_scaling_uses_beta_temperature_exponent() {
    let mut transistor = Bjt::new("Q1", "c", "b", "e");
    transistor.reverse_beta = 2.0;
    transistor.forward_beta_temperature_exponent = 2.0;
    let hot = bjt_at_temperature(&transistor, 350.0, 300.15, 1.11).unwrap();
    assert!(hot.forward_beta > transistor.forward_beta);
    assert!(hot.reverse_beta > transistor.reverse_beta);
}

#[test]
fn jfet_temperature_scaling_uses_vtotc_betatce_and_model_nominal_temperature() {
    let mut transistor = Jfet::new("J1", "d", "g", "s");
    transistor.threshold_voltage = -2.0;
    transistor.threshold_voltage_temperature_coefficient = 0.01;
    transistor.alternative_threshold_voltage_temperature_coefficient = Some(-0.0025);
    transistor.nominal_temperature_kelvin = Some(310.0);
    transistor.mobility_temperature_exponent = -5.0;
    transistor.mobility_temperature_coefficient = Some(1.0);

    let at_model_nominal = jfet_at_temperature(&transistor, 310.0, 300.15).unwrap();
    let hot = jfet_at_temperature(&transistor, 320.0, 300.15).unwrap();
    let cold = jfet_at_temperature(&transistor, 300.0, 300.15).unwrap();
    let invariant = jfet_at_temperature(&Jfet::new("Jflat", "d", "g", "s"), 350.0, 300.15).unwrap();
    let mut bex_transistor = Jfet::new("Jbex", "d", "g", "s");
    bex_transistor.nominal_temperature_kelvin = Some(310.0);
    bex_transistor.mobility_temperature_exponent = 1.0;
    let bex_fallback = jfet_at_temperature(&bex_transistor, 320.0, 300.15).unwrap();

    assert_close(at_model_nominal.threshold_voltage, -2.0);
    assert_close(at_model_nominal.beta, transistor.beta);
    assert_close(hot.threshold_voltage, -2.025);
    assert_close(hot.beta, transistor.beta * 1.01_f64.powf(10.0));
    assert!(hot.gate_saturation_current > at_model_nominal.gate_saturation_current);
    assert_close(cold.threshold_voltage, -1.975);
    assert_close(cold.beta, transistor.beta * 1.01_f64.powf(-10.0));
    assert!(cold.gate_saturation_current < at_model_nominal.gate_saturation_current);
    let mut lower_gap = transistor.clone();
    lower_gap.bandgap_voltage = 1.0;
    let lower_gap_hot = jfet_at_temperature(&lower_gap, 320.0, 300.15).unwrap();
    assert!(lower_gap_hot.gate_saturation_current < hot.gate_saturation_current);
    assert_close(invariant.threshold_voltage, -2.0);
    assert_close(invariant.beta, 1.0e-4);
    assert_close(bex_fallback.beta, bex_transistor.beta * 320.0 / 310.0);
    let mut tcv_transistor = Jfet::new("Jtcv", "d", "g", "s");
    tcv_transistor.threshold_voltage = -2.0;
    tcv_transistor.threshold_voltage_temperature_coefficient = 0.01;
    tcv_transistor.nominal_temperature_kelvin = Some(310.0);
    let tcv_fallback = jfet_at_temperature(&tcv_transistor, 320.0, 300.15).unwrap();
    assert_close(tcv_fallback.threshold_voltage, -2.1);
}

#[test]
fn dc_rejects_invalid_jfet_temperature_parameters() {
    let mut transistor = Jfet::new("Jbad", "d", "g", "0");
    transistor.doping_tail_parameter = f64::NAN;
    let mut circuit = Circuit::new();
    circuit.add(Element::Jfet(transistor));
    let error = dc_op(&circuit).unwrap_err();
    assert!(error
        .to_string()
        .contains("doping-tail parameter must be finite"));

    let mut transistor = Jfet::new("Jbad", "d", "g", "0");
    transistor.gate_saturation_current_temperature_exponent = f64::NAN;
    let mut circuit = Circuit::new();
    circuit.add(Element::Jfet(transistor));
    let error = dc_op(&circuit).unwrap_err();
    assert!(error
        .to_string()
        .contains("gate saturation-current temperature exponent must be finite"));

    let mut transistor = Jfet::new("Jbad", "d", "g", "0");
    transistor.bandgap_voltage = 0.0;
    let mut circuit = Circuit::new();
    circuit.add(Element::Jfet(transistor));
    let error = dc_op(&circuit).unwrap_err();
    assert!(error
        .to_string()
        .contains("bandgap voltage must be finite and positive"));

    let mut transistor = Jfet::new("Jbad", "d", "g", "0");
    transistor.threshold_voltage_temperature_coefficient = f64::NAN;
    let mut circuit = Circuit::new();
    circuit.add(Element::Jfet(transistor));
    let error = dc_op(&circuit).unwrap_err();
    assert!(error
        .to_string()
        .contains("threshold-voltage temperature coefficient must be finite"));

    let mut transistor = Jfet::new("Jbad", "d", "g", "0");
    transistor.alternative_threshold_voltage_temperature_coefficient = Some(f64::NAN);
    let mut circuit = Circuit::new();
    circuit.add(Element::Jfet(transistor));
    let error = dc_op(&circuit).unwrap_err();
    assert!(error
        .to_string()
        .contains("alternative threshold-voltage temperature coefficient must be finite"));

    let mut transistor = Jfet::new("Jbad", "d", "g", "0");
    transistor.nominal_temperature_kelvin = Some(0.0);
    let mut circuit = Circuit::new();
    circuit.add(Element::Jfet(transistor));
    let error = dc_op(&circuit).unwrap_err();
    assert!(error
        .to_string()
        .contains("nominal temperature must be finite and positive"));

    let mut transistor = Jfet::new("Jbad", "d", "g", "0");
    transistor.mobility_temperature_exponent = f64::NAN;
    let mut circuit = Circuit::new();
    circuit.add(Element::Jfet(transistor));
    let error = dc_op(&circuit).unwrap_err();
    assert!(error
        .to_string()
        .contains("mobility temperature exponent must be finite"));

    let mut transistor = Jfet::new("Jbad", "d", "g", "0");
    transistor.mobility_temperature_coefficient = Some(f64::NAN);
    let mut circuit = Circuit::new();
    circuit.add(Element::Jfet(transistor));
    let error = dc_op(&circuit).unwrap_err();
    assert!(error
        .to_string()
        .contains("mobility temperature coefficient must be finite"));
}

#[test]
fn bjt_temperature_scaling_uses_model_nominal_temperature() {
    let mut transistor = Bjt::new("Q1", "c", "b", "e");
    transistor.nominal_temperature_kelvin = Some(325.0);
    let at_model_nominal = bjt_at_temperature(&transistor, 325.0, 300.15, 1.11).unwrap();
    assert_close(
        at_model_nominal.saturation_current,
        transistor.saturation_current,
    );
    assert_close(at_model_nominal.thermal_voltage, transistor.thermal_voltage);
}

#[test]
fn dc_rejects_invalid_bjt_nominal_temperature() {
    let mut transistor = Bjt::new("Qbad", "c", "b", "0");
    transistor.nominal_temperature_kelvin = Some(0.0);
    let mut circuit = Circuit::new();
    circuit.add(Element::Bjt(transistor));
    let error = dc_op(&circuit).unwrap_err();
    assert!(error
        .to_string()
        .contains("nominal temperature must be finite and positive"));
}

#[test]
fn dc_rejects_invalid_diode_flicker_noise_exponent() {
    let mut diode = Diode::new("Dbad", "a", "0");
    diode.flicker_noise_exponent = -1.0;
    let mut circuit = Circuit::new();
    circuit.add(Element::Diode(diode));
    let error = dc_op(&circuit).unwrap_err();
    assert!(error
        .to_string()
        .contains("flicker-noise exponent must be finite and non-negative"));
}

#[test]
fn dc_rejects_invalid_bjt_flicker_noise_coefficient() {
    let mut transistor = Bjt::new("Qbad", "c", "b", "0");
    transistor.flicker_noise_coefficient = -1.0;
    let mut circuit = Circuit::new();
    circuit.add(Element::Bjt(transistor));
    let error = dc_op(&circuit).unwrap_err();
    assert!(error
        .to_string()
        .contains("flicker noise coefficient must be finite and non-negative"));
}

#[test]
fn dc_rejects_invalid_bjt_flicker_noise_exponent() {
    let mut transistor = Bjt::new("Qbad", "c", "b", "0");
    transistor.flicker_noise_exponent = -1.0;
    let mut circuit = Circuit::new();
    circuit.add(Element::Bjt(transistor));
    let error = dc_op(&circuit).unwrap_err();
    assert!(error
        .to_string()
        .contains("flicker noise exponent must be finite and non-negative"));
}

#[test]
fn dc_rejects_invalid_bjt_forward_excess_phase() {
    let mut transistor = Bjt::new("Qbad", "c", "b", "0");
    transistor.forward_excess_phase_degrees = -1.0;
    let mut circuit = Circuit::new();
    circuit.add(Element::Bjt(transistor));
    let error = dc_op(&circuit).unwrap_err();
    assert!(error
        .to_string()
        .contains("forward excess phase must be finite and non-negative"));
}

#[test]
fn dc_rejects_invalid_bjt_forward_transit_time_bias_coefficient() {
    let mut transistor = Bjt::new("Qbad", "c", "b", "0");
    transistor.forward_transit_time_bias_coefficient = -1.0;
    let mut circuit = Circuit::new();
    circuit.add(Element::Bjt(transistor));
    let error = dc_op(&circuit).unwrap_err();
    assert!(error
        .to_string()
        .contains("forward transit-time bias coefficient must be finite and non-negative"));
}

#[test]
fn dc_rejects_invalid_bjt_forward_transit_time_current() {
    let mut transistor = Bjt::new("Qbad", "c", "b", "0");
    transistor.forward_transit_time_current = -1.0;
    let mut circuit = Circuit::new();
    circuit.add(Element::Bjt(transistor));
    let error = dc_op(&circuit).unwrap_err();
    assert!(error
        .to_string()
        .contains("forward transit-time current must be finite and non-negative"));
}

#[test]
fn dc_rejects_invalid_bjt_forward_transit_time_voltage() {
    let mut transistor = Bjt::new("Qbad", "c", "b", "0");
    transistor.forward_transit_time_voltage = -1.0;
    let mut circuit = Circuit::new();
    circuit.add(Element::Bjt(transistor));
    let error = dc_op(&circuit).unwrap_err();
    assert!(error
        .to_string()
        .contains("forward transit-time voltage must be finite and non-negative"));
}

#[test]
fn dc_rejects_invalid_bjt_emitter_resistance() {
    let mut transistor = Bjt::new("Qbad", "c", "b", "0");
    transistor.emitter_resistance = -1.0;
    let mut circuit = Circuit::new();
    circuit.add(Element::Bjt(transistor));
    let error = dc_op(&circuit).unwrap_err();
    assert!(error
        .to_string()
        .contains("emitter resistance must be finite and non-negative"));
}

#[test]
fn dc_rejects_invalid_bjt_collector_resistance() {
    let mut transistor = Bjt::new("Qbad", "c", "b", "0");
    transistor.collector_resistance = -1.0;
    let mut circuit = Circuit::new();
    circuit.add(Element::Bjt(transistor));
    let error = dc_op(&circuit).unwrap_err();
    assert!(error
        .to_string()
        .contains("collector resistance must be finite and non-negative"));
}

#[test]
fn dc_rejects_invalid_bjt_base_resistance() {
    let mut transistor = Bjt::new("Qbad", "c", "b", "0");
    transistor.base_resistance = -1.0;
    let mut circuit = Circuit::new();
    circuit.add(Element::Bjt(transistor));
    let error = dc_op(&circuit).unwrap_err();
    assert!(error
        .to_string()
        .contains("base resistance must be finite and non-negative"));
}

#[test]
fn dc_rejects_invalid_bjt_base_collector_capacitance_fraction() {
    let mut transistor = Bjt::new("Qbad", "c", "b", "0");
    transistor.base_collector_capacitance_fraction = 1.1;
    let mut circuit = Circuit::new();
    circuit.add(Element::Bjt(transistor));
    let error = dc_op(&circuit).unwrap_err();
    assert!(error
        .to_string()
        .contains("base-collector capacitance fraction must be between zero and one"));
}

#[test]
fn dc_rejects_invalid_bjt_beta_temperature_exponent() {
    let mut transistor = Bjt::new("Qbad", "c", "b", "0");
    transistor.forward_beta_temperature_exponent = f64::NAN;
    let mut circuit = Circuit::new();
    circuit.add(Element::Bjt(transistor));
    let error = dc_op(&circuit).unwrap_err();
    assert!(error
        .to_string()
        .contains("beta temperature exponent must be finite"));
}

#[test]
fn bjt_temperature_scales_base_emitter_leakage_saturation_current() {
    let mut transistor = Bjt::new("Q1", "c", "b", "e");
    transistor.base_emitter_leakage_saturation_current = 2.0e-13;
    let hot = bjt_at_temperature(&transistor, 350.0, 300.15, 1.11).unwrap();
    assert!(
        hot.base_emitter_leakage_saturation_current
            > transistor.base_emitter_leakage_saturation_current
    );
}

#[test]
fn bjt_temperature_scales_base_collector_leakage_saturation_current() {
    let mut transistor = Bjt::new("Q1", "c", "b", "e");
    transistor.base_collector_leakage_saturation_current = 2.0e-13;
    let hot = bjt_at_temperature(&transistor, 350.0, 300.15, 1.11).unwrap();
    assert!(
        hot.base_collector_leakage_saturation_current
            > transistor.base_collector_leakage_saturation_current
    );
}

#[test]
fn bjt_temperature_scaling_uses_model_energy_gap() {
    let mut silicon = Circuit::new();
    silicon.add(Element::Bjt(Bjt::with_model_and_temperature_parameters(
        "Qsilicon",
        "c",
        "b",
        "e",
        BjtPolarity::Npn,
        1.0e-14,
        100.0,
        0.02585,
        0.0,
        0.0,
        0.0,
        0.0,
        3.0,
        1.11,
        0.0,
        1.0,
        1.0,
    )));
    let mut lower_gap = Circuit::new();
    lower_gap.add(Element::Bjt(Bjt::with_model_and_temperature_parameters(
        "Qlower",
        "c",
        "b",
        "e",
        BjtPolarity::Npn,
        1.0e-14,
        100.0,
        0.02585,
        0.0,
        0.0,
        0.0,
        0.0,
        3.0,
        0.8,
        0.0,
        1.0,
        1.0,
    )));
    let silicon_hot = circuit_at_temperature(&silicon, 350.0, 300.15, 1.11).unwrap();
    let lower_gap_hot = circuit_at_temperature(&lower_gap, 350.0, 300.15, 1.11).unwrap();
    let saturation_current = |circuit: &Circuit| match &circuit.elements()[0] {
        Element::Bjt(bjt) => bjt.saturation_current,
        _ => unreachable!(),
    };
    assert!(saturation_current(&silicon_hot) > saturation_current(&lower_gap_hot));
}

#[test]
fn dc_rejects_invalid_bjt_energy_gap() {
    let mut circuit = Circuit::new();
    circuit.add(Element::Bjt(Bjt::with_model_and_temperature_parameters(
        "Qbad",
        "c",
        "b",
        "0",
        BjtPolarity::Npn,
        1.0e-14,
        100.0,
        0.02585,
        0.0,
        0.0,
        0.0,
        0.0,
        3.0,
        0.0,
        0.0,
        1.0,
        1.0,
    )));
    let error = dc_op(&circuit).unwrap_err();
    assert!(error
        .to_string()
        .contains("energy gap must be finite and positive"));
}

#[test]
fn bjt_forward_early_voltage_modulates_collector_current() {
    let collector_voltage = |forward_early_voltage: f64| {
        let mut circuit = Circuit::new();
        circuit.add(Element::VoltageSource(VoltageSource::new(
            "Vcc", "vcc", "0", 5.0,
        )));
        circuit.add(Element::VoltageSource(VoltageSource::new(
            "Vbase", "base", "0", 0.65,
        )));
        circuit.add(Element::Resistor(Resistor::new(
            "Rload", "vcc", "out", 1_000.0,
        )));
        circuit.add(Element::Bjt(Bjt::with_model_and_temperature_parameters(
            "Q1",
            "out",
            "base",
            "0",
            BjtPolarity::Npn,
            1.0e-14,
            100.0,
            0.02585,
            0.0,
            0.0,
            0.0,
            0.0,
            3.0,
            1.11,
            forward_early_voltage,
            1.0,
            1.0,
        )));
        dc_op(&circuit).unwrap().voltage("out").unwrap()
    };

    assert!(collector_voltage(20.0) < collector_voltage(0.0));
}

#[test]
fn dc_rejects_invalid_bjt_forward_early_voltage() {
    let mut circuit = Circuit::new();
    circuit.add(Element::Bjt(Bjt::with_model_and_temperature_parameters(
        "Qbad",
        "c",
        "b",
        "0",
        BjtPolarity::Npn,
        1.0e-14,
        100.0,
        0.02585,
        0.0,
        0.0,
        0.0,
        0.0,
        3.0,
        1.11,
        -1.0,
        1.0,
        1.0,
    )));
    let error = dc_op(&circuit).unwrap_err();
    assert!(error
        .to_string()
        .contains("forward Early voltage must be finite and non-negative"));
}

#[test]
fn bjt_reverse_early_voltage_modulates_collector_current() {
    let collector_voltage = |reverse_early_voltage: f64| {
        let mut circuit = Circuit::new();
        circuit.add(Element::VoltageSource(VoltageSource::new(
            "Vcc", "vcc", "0", 5.0,
        )));
        circuit.add(Element::VoltageSource(VoltageSource::new(
            "Vbase", "base", "0", 0.65,
        )));
        circuit.add(Element::Resistor(Resistor::new(
            "Rload", "vcc", "out", 1_000.0,
        )));
        let mut transistor = Bjt::new("Q1", "out", "base", "0");
        transistor.reverse_early_voltage = reverse_early_voltage;
        circuit.add(Element::Bjt(transistor));
        dc_op(&circuit).unwrap().voltage("out").unwrap()
    };

    assert!(collector_voltage(20.0) > collector_voltage(0.0));
}

#[test]
fn bjt_forward_beta_rolloff_reduces_high_current_transport() {
    let collector_voltage = |rolloff_current: f64| {
        let mut circuit = Circuit::new();
        circuit.add(Element::VoltageSource(VoltageSource::new(
            "Vcc", "vcc", "0", 5.0,
        )));
        circuit.add(Element::VoltageSource(VoltageSource::new(
            "Vbase", "base", "0", 0.65,
        )));
        circuit.add(Element::Resistor(Resistor::new(
            "Rload", "vcc", "out", 1_000.0,
        )));
        let mut transistor = Bjt::new("Q1", "out", "base", "0");
        transistor.forward_beta_rolloff_current = rolloff_current;
        circuit.add(Element::Bjt(transistor));
        dc_op(&circuit).unwrap().voltage("out").unwrap()
    };

    assert!(collector_voltage(1.0e-4) > collector_voltage(0.0));
}

#[test]
fn bjt_reverse_beta_controls_base_collector_junction_current() {
    let base_current = |reverse_beta: f64| {
        let mut circuit = Circuit::new();
        circuit.add(Element::VoltageSource(VoltageSource::new(
            "Vbase", "base", "0", 0.65,
        )));
        circuit.add(Element::VoltageSource(VoltageSource::new(
            "Vemitter", "emitter", "0", 0.65,
        )));
        let mut transistor = Bjt::new("Q1", "0", "base", "emitter");
        transistor.reverse_beta = reverse_beta;
        circuit.add(Element::Bjt(transistor));
        dc_op(&circuit)
            .unwrap()
            .branch_current("Vbase")
            .unwrap()
            .abs()
    };

    assert!(base_current(0.5) > base_current(5.0));
}

#[test]
fn dc_rejects_invalid_bjt_reverse_beta() {
    let mut circuit = Circuit::new();
    let mut transistor = Bjt::new("Qbad", "c", "b", "0");
    transistor.reverse_beta = 0.0;
    circuit.add(Element::Bjt(transistor));
    let error = dc_op(&circuit).unwrap_err();
    assert!(error.to_string().contains("reverse beta must be positive"));
}

#[test]
fn bjt_reverse_beta_rolloff_increases_high_current_base_current() {
    let base_current = |rolloff_current: f64| {
        let mut circuit = Circuit::new();
        circuit.add(Element::VoltageSource(VoltageSource::new(
            "Vbase", "base", "0", 0.65,
        )));
        circuit.add(Element::VoltageSource(VoltageSource::new(
            "Vemitter", "emitter", "0", 0.65,
        )));
        let mut transistor = Bjt::new("Q1", "0", "base", "emitter");
        transistor.reverse_beta = 1.0;
        transistor.reverse_beta_rolloff_current = rolloff_current;
        circuit.add(Element::Bjt(transistor));
        dc_op(&circuit)
            .unwrap()
            .branch_current("Vbase")
            .unwrap()
            .abs()
    };

    assert!(base_current(1.0e-4) > base_current(0.0));
}

#[test]
fn dc_rejects_invalid_bjt_reverse_beta_rolloff_current() {
    let mut circuit = Circuit::new();
    let mut transistor = Bjt::new("Qbad", "c", "b", "0");
    transistor.reverse_beta_rolloff_current = -1.0;
    circuit.add(Element::Bjt(transistor));
    let error = dc_op(&circuit).unwrap_err();
    assert!(error
        .to_string()
        .contains("reverse beta roll-off current must be finite and non-negative"));
}

#[test]
fn dc_rejects_invalid_bjt_forward_beta_rolloff_current() {
    let mut circuit = Circuit::new();
    let mut transistor = Bjt::new("Qbad", "c", "b", "0");
    transistor.forward_beta_rolloff_current = -1.0;
    circuit.add(Element::Bjt(transistor));
    let error = dc_op(&circuit).unwrap_err();
    assert!(error
        .to_string()
        .contains("forward beta roll-off current must be finite and non-negative"));
}

#[test]
fn bjt_base_emitter_leakage_increases_base_current() {
    let base_current = |leakage_current: f64| {
        let mut circuit = Circuit::new();
        circuit.add(Element::VoltageSource(VoltageSource::new(
            "Vbase", "base", "0", 0.65,
        )));
        let mut transistor = Bjt::new("Q1", "0", "base", "0");
        transistor.base_emitter_leakage_saturation_current = leakage_current;
        transistor.base_emitter_leakage_emission_coefficient = 1.5;
        circuit.add(Element::Bjt(transistor));
        dc_op(&circuit)
            .unwrap()
            .branch_current("Vbase")
            .unwrap()
            .abs()
    };

    assert!(base_current(1.0e-10) > base_current(0.0));
}

#[test]
fn dc_rejects_invalid_bjt_base_emitter_leakage_parameters() {
    let mut circuit = Circuit::new();
    let mut transistor = Bjt::new("Qbad", "c", "b", "0");
    transistor.base_emitter_leakage_saturation_current = -1.0;
    circuit.add(Element::Bjt(transistor));
    assert!(dc_op(&circuit)
        .unwrap_err()
        .to_string()
        .contains("base-emitter leakage saturation current must be finite and non-negative"));

    let mut circuit = Circuit::new();
    let mut transistor = Bjt::new("Qbad", "c", "b", "0");
    transistor.base_emitter_leakage_emission_coefficient = 0.0;
    circuit.add(Element::Bjt(transistor));
    assert!(dc_op(&circuit)
        .unwrap_err()
        .to_string()
        .contains("base-emitter leakage emission coefficient must be finite and positive"));
}

#[test]
fn bjt_base_collector_leakage_increases_base_current() {
    let base_current = |leakage_current: f64| {
        let mut circuit = Circuit::new();
        circuit.add(Element::VoltageSource(VoltageSource::new(
            "Vbase", "base", "0", 0.65,
        )));
        let mut transistor = Bjt::new("Q1", "0", "base", "base");
        transistor.base_collector_leakage_saturation_current = leakage_current;
        transistor.base_collector_leakage_emission_coefficient = 1.5;
        circuit.add(Element::Bjt(transistor));
        dc_op(&circuit)
            .unwrap()
            .branch_current("Vbase")
            .unwrap()
            .abs()
    };

    assert!(base_current(1.0e-10) > base_current(0.0));
}

#[test]
fn dc_rejects_invalid_bjt_base_collector_leakage_parameters() {
    let mut circuit = Circuit::new();
    let mut transistor = Bjt::new("Qbad", "c", "b", "0");
    transistor.base_collector_leakage_saturation_current = -1.0;
    circuit.add(Element::Bjt(transistor));
    assert!(dc_op(&circuit)
        .unwrap_err()
        .to_string()
        .contains("base-collector leakage saturation current must be finite and non-negative"));

    let mut circuit = Circuit::new();
    let mut transistor = Bjt::new("Qbad", "c", "b", "0");
    transistor.base_collector_leakage_emission_coefficient = 0.0;
    circuit.add(Element::Bjt(transistor));
    assert!(dc_op(&circuit)
        .unwrap_err()
        .to_string()
        .contains("base-collector leakage emission coefficient must be finite and positive"));
}

#[test]
fn dc_rejects_invalid_bjt_reverse_early_voltage() {
    let mut circuit = Circuit::new();
    let mut transistor = Bjt::new("Qbad", "c", "b", "0");
    transistor.reverse_early_voltage = -1.0;
    circuit.add(Element::Bjt(transistor));
    let error = dc_op(&circuit).unwrap_err();
    assert!(error
        .to_string()
        .contains("reverse Early voltage must be finite and non-negative"));
}

#[test]
fn bjt_forward_emission_coefficient_reduces_collector_current() {
    let collector_voltage = |forward_emission_coefficient: f64| {
        let mut circuit = Circuit::new();
        circuit.add(Element::VoltageSource(VoltageSource::new(
            "Vcc", "vcc", "0", 5.0,
        )));
        circuit.add(Element::VoltageSource(VoltageSource::new(
            "Vbase", "base", "0", 0.65,
        )));
        circuit.add(Element::Resistor(Resistor::new(
            "Rload", "vcc", "out", 1_000.0,
        )));
        circuit.add(Element::Bjt(Bjt::with_model_and_temperature_parameters(
            "Q1",
            "out",
            "base",
            "0",
            BjtPolarity::Npn,
            1.0e-14,
            100.0,
            0.02585,
            0.0,
            0.0,
            0.0,
            0.0,
            3.0,
            1.11,
            0.0,
            forward_emission_coefficient,
            1.0,
        )));
        dc_op(&circuit).unwrap().voltage("out").unwrap()
    };

    assert!(collector_voltage(2.0) > collector_voltage(1.0));
}

#[test]
fn dc_rejects_invalid_bjt_forward_emission_coefficient() {
    let mut circuit = Circuit::new();
    circuit.add(Element::Bjt(Bjt::with_model_and_temperature_parameters(
        "Qbad",
        "c",
        "b",
        "0",
        BjtPolarity::Npn,
        1.0e-14,
        100.0,
        0.02585,
        0.0,
        0.0,
        0.0,
        0.0,
        3.0,
        1.11,
        0.0,
        0.0,
        1.0,
    )));
    let error = dc_op(&circuit).unwrap_err();
    assert!(error
        .to_string()
        .contains("forward emission coefficient must be finite and positive"));
}

#[test]
fn dc_rejects_invalid_bjt_reverse_emission_coefficient() {
    let mut circuit = Circuit::new();
    circuit.add(Element::Bjt(Bjt::with_model_and_temperature_parameters(
        "Qbad",
        "c",
        "b",
        "0",
        BjtPolarity::Npn,
        1.0e-14,
        100.0,
        0.02585,
        0.0,
        0.0,
        0.0,
        0.0,
        3.0,
        1.11,
        0.0,
        1.0,
        0.0,
    )));
    let error = dc_op(&circuit).unwrap_err();
    assert!(error
        .to_string()
        .contains("reverse emission coefficient must be finite and positive"));
}

#[test]
fn dc_rejects_invalid_bjt_base_emitter_depletion_parameters() {
    for (junction_potential, grading_coefficient, message) in [
        (
            0.0,
            0.33,
            "base-emitter junction potential must be finite and positive",
        ),
        (
            0.75,
            1.0,
            "base-emitter grading coefficient must be finite and in [0, 1)",
        ),
    ] {
        let mut circuit = Circuit::new();
        circuit.add(Element::Bjt(
            Bjt::with_model_temperature_and_depletion_parameters(
                "Qbad",
                "c",
                "b",
                "0",
                BjtPolarity::Npn,
                1.0e-14,
                100.0,
                0.02585,
                0.0,
                0.0,
                0.0,
                0.0,
                3.0,
                1.11,
                0.0,
                1.0,
                1.0,
                junction_potential,
                grading_coefficient,
                0.75,
                0.33,
                0.5,
            ),
        ));
        assert!(dc_op(&circuit).unwrap_err().to_string().contains(message));
    }
}

#[test]
fn dc_rejects_invalid_bjt_base_collector_depletion_parameters() {
    for (junction_potential, grading_coefficient, message) in [
        (
            0.0,
            0.33,
            "base-collector junction potential must be finite and positive",
        ),
        (
            0.75,
            1.0,
            "base-collector grading coefficient must be finite and in [0, 1)",
        ),
    ] {
        let mut circuit = Circuit::new();
        circuit.add(Element::Bjt(
            Bjt::with_model_temperature_and_depletion_parameters(
                "Qbad",
                "c",
                "b",
                "0",
                BjtPolarity::Npn,
                1.0e-14,
                100.0,
                0.02585,
                0.0,
                0.0,
                0.0,
                0.0,
                3.0,
                1.11,
                0.0,
                1.0,
                1.0,
                0.75,
                0.33,
                junction_potential,
                grading_coefficient,
                0.5,
            ),
        ));
        assert!(dc_op(&circuit).unwrap_err().to_string().contains(message));
    }
}

#[test]
fn dc_rejects_invalid_bjt_forward_bias_depletion_coefficient() {
    for coefficient in [-0.1, 1.0, f64::NAN] {
        let mut bjt = Bjt::new("Qbad", "c", "b", "0");
        bjt.forward_bias_depletion_coefficient = coefficient;
        let mut circuit = Circuit::new();
        circuit.add(Element::Bjt(bjt));
        assert!(dc_op(&circuit)
            .unwrap_err()
            .to_string()
            .contains("forward-bias depletion coefficient must be finite and in [0, 1)"));
    }
}

#[test]
fn dc_mosfet_temperature_scaling_changes_common_source_bias() {
    let mut nominal = Circuit::new();
    nominal.add(Element::VoltageSource(VoltageSource::new(
        "Vdd", "vdd", "0", 1.8,
    )));
    nominal.add(Element::VoltageSource(VoltageSource::new(
        "Vgate", "gate", "0", 1.1,
    )));
    nominal.add(Element::Resistor(Resistor::new(
        "Rload", "vdd", "out", 1_000.0,
    )));
    nominal.add(Element::Mosfet(Mosfet::with_model(
        "M1",
        "out",
        "gate",
        "0",
        "0",
        MosfetType::Nmos,
        MosfetLevel1Params {
            vt0: 0.65,
            kp: 200.0e-6,
            w: 2.0e-6,
            l: 180.0e-9,
            lambda: 0.02,
            ..MosfetLevel1Params::default()
        },
    )));

    let cold = circuit_at_temperature(&nominal, 275.0, 300.15, 1.11).unwrap();
    let hot = circuit_at_temperature(&nominal, 350.0, 300.15, 1.11).unwrap();

    let nominal_result = dc_op(&nominal).unwrap();
    let cold_result = dc_op(&cold).unwrap();
    let hot_result = dc_op(&hot).unwrap();

    assert!(cold_result.voltage("out").unwrap() < nominal_result.voltage("out").unwrap());
    assert!(hot_result.voltage("out").unwrap() > nominal_result.voltage("out").unwrap());
}

#[test]
fn mosfet_temperature_scaling_keeps_surface_mobility_and_kp_aligned() {
    let nominal = Mosfet::with_model(
        "M1",
        "d",
        "g",
        "s",
        "b",
        MosfetType::Nmos,
        MosfetLevel1Params {
            kp: 200.0e-6,
            surface_mobility: 500.0,
            ..MosfetLevel1Params::default()
        },
    );
    let hot = mosfet_at_temperature(&nominal, 600.3, 300.15, 1.11).unwrap();
    let scale = 2.0_f64.powf(-1.5);

    assert_close(hot.params.kp, nominal.params.kp * scale);
    assert_close(
        hot.params.surface_mobility,
        nominal.params.surface_mobility * scale,
    );
    assert_close(
        hot.params.kp / hot.params.surface_mobility,
        nominal.params.kp / nominal.params.surface_mobility,
    );
}

#[test]
fn mosfet_temperature_scaling_adjusts_phi_and_threshold_by_polarity() {
    let nmos = Mosfet::with_model(
        "Mn",
        "d",
        "g",
        "s",
        "b",
        MosfetType::Nmos,
        MosfetLevel1Params::default(),
    );
    let pmos = Mosfet::with_model(
        "Mp",
        "d",
        "g",
        "s",
        "b",
        MosfetType::Pmos,
        MosfetLevel1Params {
            vt0: -0.42,
            ..MosfetLevel1Params::default()
        },
    );

    let hot_nmos = mosfet_at_temperature(&nmos, 350.0, 300.15, 1.11).unwrap();
    let hot_pmos = mosfet_at_temperature(&pmos, 350.0, 300.15, 1.11).unwrap();

    assert_close(hot_nmos.params.phi, 0.766_340_574_915_624_6);
    assert_close(hot_pmos.params.phi, hot_nmos.params.phi);
    assert_close(hot_nmos.params.vt0, 0.379_106_188_982_781_14);
    assert_close(hot_pmos.params.vt0, -0.365_036_965_282_786_06);
}

#[test]
fn mosfet_temperature_scaling_adjusts_bulk_junction_potential() {
    let nominal = Mosfet::with_model(
        "M1",
        "d",
        "g",
        "s",
        "b",
        MosfetType::Nmos,
        MosfetLevel1Params::default(),
    );

    let cold = mosfet_at_temperature(&nominal, 275.0, 300.15, 1.11).unwrap();
    let hot = mosfet_at_temperature(&nominal, 350.0, 300.15, 1.11).unwrap();

    assert_close(cold.params.bulk_junction_potential, 0.839_148_690_629_946_5);
    assert_close(hot.params.bulk_junction_potential, 0.719_697_229_921_455);
}

#[test]
fn mosfet_temperature_scaling_adjusts_zero_bias_junction_capacitances() {
    let nominal = Mosfet::with_model(
        "M1",
        "d",
        "g",
        "s",
        "b",
        MosfetType::Nmos,
        MosfetLevel1Params {
            bottom_junction_capacitance: 2.0e-12,
            source_bulk_capacitance: 3.0e-12,
            drain_bulk_capacitance: 4.0e-12,
            sidewall_junction_capacitance: 5.0e-12,
            ..MosfetLevel1Params::default()
        },
    );

    let cold = mosfet_at_temperature(&nominal, 275.0, 300.15, 1.11).unwrap();
    let hot = mosfet_at_temperature(&nominal, 350.0, 300.15, 1.11).unwrap();
    let cold_bottom_scale = 0.970_502_066_286_684_6;
    let cold_sidewall_scale = 0.980_531_363_923_873_3;
    let hot_bottom_scale = 1.060_159_235_535_130_4;
    let hot_sidewall_scale = 1.039_705_095_096_974_2;

    assert_close(
        cold.params.bottom_junction_capacitance,
        2.0e-12 * cold_bottom_scale,
    );
    assert_close(
        cold.params.source_bulk_capacitance,
        3.0e-12 * cold_bottom_scale,
    );
    assert_close(
        cold.params.drain_bulk_capacitance,
        4.0e-12 * cold_bottom_scale,
    );
    assert_close(
        cold.params.sidewall_junction_capacitance,
        5.0e-12 * cold_sidewall_scale,
    );
    assert_close(
        hot.params.bottom_junction_capacitance,
        2.0e-12 * hot_bottom_scale,
    );
    assert_close(
        hot.params.source_bulk_capacitance,
        3.0e-12 * hot_bottom_scale,
    );
    assert_close(
        hot.params.drain_bulk_capacitance,
        4.0e-12 * hot_bottom_scale,
    );
    assert_close(
        hot.params.sidewall_junction_capacitance,
        5.0e-12 * hot_sidewall_scale,
    );
}

#[test]
fn mosfet_temperature_scaling_adjusts_bulk_junction_saturation_currents() {
    let nominal = Mosfet::with_model(
        "M1",
        "d",
        "g",
        "s",
        "b",
        MosfetType::Nmos,
        MosfetLevel1Params {
            saturation_current: 2.0e-15,
            saturation_current_density: 3.0e-12,
            ..MosfetLevel1Params::default()
        },
    );
    let hot = mosfet_at_temperature(&nominal, 350.0, 300.15, 1.11).unwrap();
    let ratio: f64 = 350.0 / 300.15;
    let exponent: f64 = 1.11 * 1.602_176_634e-19 / 1.380_649e-23 * (1.0 / 300.15 - 1.0 / 350.0);
    let scale = ratio.powi(3) * exponent.exp();

    assert_close(hot.params.saturation_current, 2.0e-15 * scale);
    assert_close(hot.params.saturation_current_density, 3.0e-12 * scale);
    assert_close(
        hot.params.saturation_current_density / hot.params.saturation_current,
        1_500.0,
    );

    let mut circuit = Circuit::new();
    circuit.add(Element::Mosfet(nominal));
    let silicon = circuit_at_temperature(&circuit, 350.0, 300.15, 1.11).unwrap();
    let lower_gap = circuit_at_temperature(&circuit, 350.0, 300.15, 0.8).unwrap();
    let junction_current = |adjusted: &Circuit| {
        adjusted
            .elements()
            .iter()
            .find_map(|element| match element {
                Element::Mosfet(mosfet) => Some(mosfet.params.saturation_current),
                _ => None,
            })
            .unwrap()
    };
    assert!(junction_current(&silicon) > junction_current(&lower_gap));
}

#[test]
fn mosfet_temperature_scaling_prefers_model_nominal_temperature() {
    let nominal = Mosfet::with_model(
        "M1",
        "d",
        "g",
        "s",
        "b",
        MosfetType::Nmos,
        MosfetLevel1Params {
            kp: 200.0e-6,
            surface_mobility: 500.0,
            t_nom: 325.0,
            ..MosfetLevel1Params::default()
        },
    );
    let expected_scale = (350.0_f64 / 325.0).powf(-1.5);
    let hot = mosfet_at_temperature(&nominal, 350.0, 300.15, 1.11).unwrap();

    assert_close(hot.params.kp, nominal.params.kp * expected_scale);
    assert_close(
        hot.params.surface_mobility,
        nominal.params.surface_mobility * expected_scale,
    );

    let mut circuit = Circuit::new();
    circuit.add(Element::Mosfet(nominal));
    let adjusted = circuit_at_temperature(&circuit, 350.0, 300.15, 1.11).unwrap();
    let circuit_kp = adjusted
        .elements()
        .iter()
        .find_map(|element| match element {
            Element::Mosfet(mosfet) => Some(mosfet.params.kp),
            _ => None,
        })
        .unwrap();
    assert_close(circuit_kp, 200.0e-6 * expected_scale);

    let fallback = Mosfet::with_model(
        "M2",
        "d",
        "g",
        "s",
        "b",
        MosfetType::Nmos,
        MosfetLevel1Params {
            kp: 200.0e-6,
            ..MosfetLevel1Params::default()
        },
    );
    let fallback_hot = mosfet_at_temperature(&fallback, 350.0, 325.0, 1.11).unwrap();
    assert_close(fallback_hot.params.kp, 200.0e-6 * expected_scale);
}

#[test]
fn dc_bjt_solves_npn_emitter_follower_operating_point() {
    let mut circuit = Circuit::new();
    circuit.add(Element::VoltageSource(VoltageSource::new(
        "Vcc", "vcc", "0", 5.0,
    )));
    circuit.add(Element::VoltageSource(VoltageSource::new(
        "Vbase", "base", "0", 0.7,
    )));
    circuit.add(Element::Bjt(Bjt::with_model(
        "Q1",
        "vcc",
        "base",
        "out",
        BjtPolarity::Npn,
        1.0e-13,
        120.0,
        0.026,
        0.0,
        0.0,
        0.0,
        0.0,
    )));
    circuit.add(Element::Resistor(Resistor::new(
        "Rload", "out", "0", 1_000.0,
    )));

    let result = dc_op(&circuit).unwrap();
    let out = result.voltage("out").unwrap();
    assert!(
        out > 0.0,
        "expected emitter current through load, got {out}"
    );
    assert!(out < 0.7, "expected emitter below base bias, got {out}");
}

#[test]
fn dc_bjt_emitter_resistance_reduces_fixed_base_collector_current() {
    fn collector_voltage(emitter_resistance: f64) -> f64 {
        let mut transistor = Bjt::with_model(
            "Q1",
            "collector",
            "base",
            "0",
            BjtPolarity::Npn,
            1.0e-14,
            100.0,
            0.02585,
            0.0,
            0.0,
            0.0,
            0.0,
        );
        transistor.emitter_resistance = emitter_resistance;
        let mut circuit = Circuit::new();
        circuit.add(Element::VoltageSource(VoltageSource::new(
            "Vcc", "vcc", "0", 5.0,
        )));
        circuit.add(Element::VoltageSource(VoltageSource::new(
            "Vbase", "base", "0", 0.7,
        )));
        circuit.add(Element::Resistor(Resistor::new(
            "Rc",
            "vcc",
            "collector",
            1_000.0,
        )));
        circuit.add(Element::Bjt(transistor));
        dc_op(&circuit).unwrap().voltage("collector").unwrap()
    }

    assert!(collector_voltage(100.0) > collector_voltage(0.0) + 0.5);
}

#[test]
fn dc_bjt_collector_resistance_drops_intrinsic_collector_voltage() {
    let mut transistor = Bjt::new("Q1", "collector", "base", "0");
    transistor.collector_resistance = 100.0;
    let mut circuit = Circuit::new();
    circuit.add(Element::VoltageSource(VoltageSource::new(
        "Vcollector",
        "collector",
        "0",
        5.0,
    )));
    circuit.add(Element::VoltageSource(VoltageSource::new(
        "Vbase", "base", "0", 0.65,
    )));
    circuit.add(Element::Bjt(transistor));

    let intrinsic = dc_op(&circuit)
        .unwrap()
        .voltage("__spice_Q1_collector")
        .unwrap();
    assert!(intrinsic < 5.0);
    assert!(intrinsic > 0.0);
}

#[test]
fn dc_bjt_base_resistance_drops_intrinsic_base_voltage() {
    let mut transistor = Bjt::new("Q1", "collector", "base", "0");
    transistor.base_resistance = 1_000.0;
    let mut circuit = Circuit::new();
    circuit.add(Element::VoltageSource(VoltageSource::new(
        "Vcollector",
        "collector",
        "0",
        5.0,
    )));
    circuit.add(Element::VoltageSource(VoltageSource::new(
        "Vbase", "base", "0", 0.65,
    )));
    circuit.add(Element::Bjt(transistor));

    let intrinsic = dc_op(&circuit).unwrap().voltage("__spice_Q1_base").unwrap();
    assert!(intrinsic < 0.65);
    assert!(intrinsic > 0.0);
}

#[test]
fn dc_bjt_minimum_base_resistance_reduces_high_current_base_drop() {
    let intrinsic_base = |minimum_base_resistance: Option<f64>, half_current: f64| {
        let mut transistor = Bjt::new("Q1", "collector", "base", "0");
        transistor.base_resistance = 1_000.0;
        transistor.minimum_base_resistance = minimum_base_resistance;
        transistor.base_resistance_half_current = half_current;
        let mut circuit = Circuit::new();
        circuit.add(Element::VoltageSource(VoltageSource::new(
            "Vcollector",
            "collector",
            "0",
            5.0,
        )));
        circuit.add(Element::VoltageSource(VoltageSource::new(
            "Vbase", "base", "0", 0.65,
        )));
        circuit.add(Element::Bjt(transistor));
        dc_op(&circuit).unwrap().voltage("__spice_Q1_base").unwrap()
    };

    let fixed = intrinsic_base(None, 0.0);
    let bias_dependent = intrinsic_base(Some(10.0), 1.0e-6);
    assert!(bias_dependent > fixed);
    assert!(bias_dependent < 0.65);
}

#[test]
fn dc_bjt_solves_pnp_pullup_operating_point() {
    let mut circuit = Circuit::new();
    circuit.add(Element::VoltageSource(VoltageSource::new(
        "Vcc", "vcc", "0", 5.0,
    )));
    circuit.add(Element::VoltageSource(VoltageSource::new(
        "Vbase", "base", "0", 4.3,
    )));
    circuit.add(Element::Bjt(Bjt::with_model(
        "Q1",
        "out",
        "base",
        "vcc",
        BjtPolarity::Pnp,
        1.0e-13,
        100.0,
        0.026,
        0.0,
        0.0,
        0.0,
        0.0,
    )));
    circuit.add(Element::Resistor(Resistor::new(
        "Rload", "out", "0", 1_000.0,
    )));

    let result = dc_op(&circuit).unwrap();
    let out = result.voltage("out").unwrap();
    assert!(out > 0.0, "expected pullup current through load, got {out}");
}

#[test]
fn dc_mosfet_solves_nmos_source_follower_operating_point() {
    let mut circuit = Circuit::new();
    circuit.add(Element::VoltageSource(VoltageSource::new(
        "Vdd", "vdd", "0", 5.0,
    )));
    circuit.add(Element::VoltageSource(VoltageSource::new(
        "Vgate", "gate", "0", 2.5,
    )));
    circuit.add(Element::Mosfet(Mosfet::with_model(
        "M1",
        "vdd",
        "gate",
        "out",
        "0",
        MosfetType::Nmos,
        MosfetLevel1Params {
            kp: 250.0e-6,
            w: 4.0e-6,
            l: 200.0e-9,
            ..MosfetLevel1Params::default()
        },
    )));
    circuit.add(Element::Resistor(Resistor::new(
        "Rload", "out", "0", 1_000.0,
    )));

    let result = dc_op(&circuit).unwrap();
    let out = result.voltage("out").unwrap();
    assert!(out > 0.0, "expected source follower output, got {out}");
    assert!(out < 2.5, "expected source below gate bias, got {out}");
    assert!(result.converged);
    assert!(result.iterations > 0);
}

#[test]
fn dc_mosfet_lateral_diffusion_uses_effective_channel_length() {
    let drain_current = |lateral_diffusion_length| {
        let mut circuit = Circuit::new();
        circuit.add(Element::VoltageSource(VoltageSource::new(
            "Vdrain", "drain", "0", 1.8,
        )));
        circuit.add(Element::VoltageSource(VoltageSource::new(
            "Vgate", "gate", "0", 1.8,
        )));
        circuit.add(Element::Mosfet(Mosfet::with_model(
            "M1",
            "drain",
            "gate",
            "0",
            "0",
            MosfetType::Nmos,
            MosfetLevel1Params {
                l: 1.0e-6,
                lateral_diffusion_length,
                ..MosfetLevel1Params::default()
            },
        )));
        dc_op(&circuit)
            .unwrap()
            .branch_current("Vdrain")
            .unwrap()
            .abs()
    };

    assert_close(drain_current(0.1e-6) / drain_current(0.0), 1.25);
}

#[test]
fn dc_jfet_solves_n_channel_source_resistor_bias() {
    let mut circuit = Circuit::new();
    circuit.add(Element::VoltageSource(VoltageSource::new(
        "Vdd", "vdd", "0", 10.0,
    )));
    circuit.add(Element::VoltageSource(VoltageSource::new(
        "Vg", "gate", "0", 0.0,
    )));
    circuit.add(Element::Resistor(Resistor::new(
        "Rd", "vdd", "drain", 2_000.0,
    )));
    circuit.add(Element::Resistor(Resistor::new(
        "Rs", "source", "0", 1_000.0,
    )));
    circuit.add(Element::Jfet(Jfet::with_model(
        "J1",
        "drain",
        "gate",
        "source",
        JfetPolarity::Njf,
        1.0e-3,
        -2.0,
        0.0,
    )));

    let result = dc_op(&circuit).unwrap();

    assert!(result.converged);
    assert!((result.voltage("source").unwrap() - 1.0).abs() < 0.05);
    assert!((result.voltage("drain").unwrap() - 8.0).abs() < 0.1);
}

#[test]
fn dc_op_reports_unconverged_nonlinear_result_when_aids_are_disabled() {
    let mut circuit = Circuit::new();
    circuit.add(Element::VoltageSource(VoltageSource::new(
        "Vdd", "vdd", "0", 5.0,
    )));
    circuit.add(Element::VoltageSource(VoltageSource::new(
        "Vgate", "gate", "0", 2.5,
    )));
    circuit.add(Element::Mosfet(Mosfet::with_model(
        "M1",
        "vdd",
        "gate",
        "out",
        "0",
        MosfetType::Nmos,
        MosfetLevel1Params {
            kp: 250.0e-6,
            w: 4.0e-6,
            l: 200.0e-9,
            ..MosfetLevel1Params::default()
        },
    )));
    circuit.add(Element::Resistor(Resistor::new(
        "Rload", "out", "0", 1_000.0,
    )));

    let result = dc_op_with_options(
        &circuit,
        DcOpOptions {
            max_iterations: 1,
            convergence_aids: false,
            ..DcOpOptions::default()
        },
    )
    .unwrap();

    assert!(!result.converged);
    assert_eq!(result.convergence_aid, DcConvergenceAid::None);
    assert_eq!(result.iterations, 1);
}

#[test]
fn dc_op_newton_step_limit_reports_damped_nonlinear_step() {
    let mut circuit = Circuit::new();
    circuit.add(Element::VoltageSource(VoltageSource::new(
        "Vs", "in", "0", 10.0,
    )));
    circuit.add(Element::Diode(Diode::with_model(
        "D1", "in", "out", 1.0e-15, 0.02585,
    )));
    circuit.add(Element::Resistor(Resistor::new("Rload", "out", "0", 100.0)));

    let result = dc_op_with_options(
        &circuit,
        DcOpOptions {
            max_iterations: 1,
            convergence_aids: false,
            newton_step_limit: Some(0.25),
            ..DcOpOptions::default()
        },
    )
    .unwrap();

    assert!(!result.converged);
    assert_eq!(result.convergence_aid, DcConvergenceAid::None);
    assert_eq!(result.diagnostics.newton_step_limit, Some(0.25));
    assert_eq!(result.diagnostics.limited_newton_steps, 1);
    assert!(result.diagnostics.minimum_damping_factor > 0.0);
    assert!(result.diagnostics.minimum_damping_factor < 1.0);
    assert_close(result.diagnostics.max_delta, 0.25);
    let max_voltage = result
        .node_voltages
        .values()
        .map(|value| value.abs())
        .fold(0.0, f64::max);
    assert!(max_voltage <= 0.25 + 1.0e-12);
}

#[test]
fn dc_op_pseudo_transient_recovers_after_earlier_aids_fail() {
    let mut circuit = Circuit::new();
    circuit.add(Element::VoltageSource(VoltageSource::new(
        "Vs", "in", "0", 10.0,
    )));
    circuit.add(Element::Diode(Diode::with_model(
        "D1", "in", "out", 1.0e-15, 0.02585,
    )));
    circuit.add(Element::Resistor(Resistor::new("Rload", "out", "0", 100.0)));

    let result = dc_op_with_options(
        &circuit,
        DcOpOptions {
            max_iterations: 1,
            pseudo_transient_max_iterations: 500,
            pseudo_transient_steps: 40,
            ..DcOpOptions::default()
        },
    )
    .unwrap();

    assert!(result.converged);
    assert_eq!(result.convergence_aid, DcConvergenceAid::PseudoTransient);
    assert!(result.voltage("out").unwrap() > 0.0);
    assert!(result.voltage("out").unwrap() < 10.0);
}

#[test]
fn dc_mosfet_rejects_invalid_level1_params() {
    let mut circuit = Circuit::new();
    circuit.add(Element::VoltageSource(VoltageSource::new(
        "Vdd", "vdd", "0", 5.0,
    )));
    circuit.add(Element::Mosfet(Mosfet::with_model(
        "Mbad",
        "vdd",
        "gate",
        "0",
        "0",
        MosfetType::Nmos,
        MosfetLevel1Params {
            kp: 0.0,
            ..MosfetLevel1Params::default()
        },
    )));

    let err = dc_op(&circuit).unwrap_err();
    assert_eq!(
        err,
        SpiceError::InvalidElement {
            name: "Mbad".to_string(),
            reason: "MOSFET KP must be positive".to_string(),
        }
    );
}

#[test]
fn dc_mosfet_rejects_invalid_forward_bias_depletion_coefficient() {
    for coefficient in [f64::NAN, -0.1, 1.0] {
        let mut circuit = Circuit::new();
        circuit.add(Element::Mosfet(Mosfet::with_model(
            "Mbad",
            "drain",
            "gate",
            "0",
            "0",
            MosfetType::Nmos,
            MosfetLevel1Params {
                forward_bias_depletion_coefficient: coefficient,
                ..MosfetLevel1Params::default()
            },
        )));

        let err = dc_op(&circuit).unwrap_err();
        assert_eq!(
            err,
            SpiceError::InvalidElement {
                name: "Mbad".to_string(),
                reason: if coefficient.is_finite() {
                    "MOSFET FC must be in [0, 1)".to_string()
                } else {
                    "MOSFET FC must be finite".to_string()
                },
            }
        );
    }
}

#[test]
fn dc_mosfet_rejects_invalid_saturation_current_density() {
    for saturation_current_density in [f64::NAN, -1.0] {
        let mut circuit = Circuit::new();
        circuit.add(Element::Mosfet(Mosfet::with_model(
            "Mbad",
            "drain",
            "gate",
            "0",
            "0",
            MosfetType::Nmos,
            MosfetLevel1Params {
                saturation_current_density,
                ..MosfetLevel1Params::default()
            },
        )));

        let err = dc_op(&circuit).unwrap_err();
        assert_eq!(
            err,
            SpiceError::InvalidElement {
                name: "Mbad".to_string(),
                reason: if saturation_current_density.is_finite() {
                    "MOSFET JS must be non-negative".to_string()
                } else {
                    "MOSFET JS must be finite".to_string()
                },
            }
        );
    }
}

#[test]
fn dc_mosfet_rejects_invalid_lateral_diffusion_length() {
    for lateral_diffusion_length in [f64::NAN, -0.1e-6, 0.5e-6] {
        let mut circuit = Circuit::new();
        circuit.add(Element::Mosfet(Mosfet::with_model(
            "Mbad",
            "drain",
            "gate",
            "0",
            "0",
            MosfetType::Nmos,
            MosfetLevel1Params {
                l: 1.0e-6,
                lateral_diffusion_length,
                ..MosfetLevel1Params::default()
            },
        )));

        let err = dc_op(&circuit).unwrap_err();
        assert_eq!(
            err,
            SpiceError::InvalidElement {
                name: "Mbad".to_string(),
                reason: if lateral_diffusion_length.is_finite() {
                    "MOSFET LD must be non-negative with L - 2*LD > 0".to_string()
                } else {
                    "MOSFET LD must be finite".to_string()
                },
            }
        );
    }
}

#[test]
fn dc_mosfet_rejects_invalid_oxide_thickness() {
    for oxide_thickness in [f64::NAN, 0.0, -1.0e-9] {
        let mut circuit = Circuit::new();
        circuit.add(Element::Mosfet(Mosfet::with_model(
            "Mbad",
            "drain",
            "gate",
            "0",
            "0",
            MosfetType::Nmos,
            MosfetLevel1Params {
                oxide_thickness,
                ..MosfetLevel1Params::default()
            },
        )));

        let err = dc_op(&circuit).unwrap_err();
        assert_eq!(
            err,
            SpiceError::InvalidElement {
                name: "Mbad".to_string(),
                reason: if oxide_thickness.is_finite() {
                    "MOSFET TOX must be positive".to_string()
                } else {
                    "MOSFET TOX must be finite".to_string()
                },
            }
        );
    }
}

#[test]
fn dc_mosfet_rejects_invalid_surface_mobility() {
    for surface_mobility in [f64::NAN, -1.0] {
        let mut circuit = Circuit::new();
        circuit.add(Element::Mosfet(Mosfet::with_model(
            "Mbad",
            "drain",
            "gate",
            "0",
            "0",
            MosfetType::Nmos,
            MosfetLevel1Params {
                surface_mobility,
                ..MosfetLevel1Params::default()
            },
        )));

        let err = dc_op(&circuit).unwrap_err();
        assert_eq!(
            err,
            SpiceError::InvalidElement {
                name: "Mbad".to_string(),
                reason: if surface_mobility.is_finite() {
                    "MOSFET U0 must be non-negative".to_string()
                } else {
                    "MOSFET U0 must be finite".to_string()
                },
            }
        );
    }
}

#[test]
fn dc_mosfet_rejects_invalid_drain_resistance() {
    for drain_resistance in [f64::NAN, -1.0] {
        let mut circuit = Circuit::new();
        circuit.add(Element::Mosfet(Mosfet::with_model(
            "Mbad",
            "drain",
            "gate",
            "0",
            "0",
            MosfetType::Nmos,
            MosfetLevel1Params {
                drain_resistance,
                ..MosfetLevel1Params::default()
            },
        )));

        let err = dc_op(&circuit).unwrap_err();
        assert_eq!(
            err,
            SpiceError::InvalidElement {
                name: "Mbad".to_string(),
                reason: if drain_resistance.is_finite() {
                    "MOSFET RD must be non-negative".to_string()
                } else {
                    "MOSFET RD must be finite".to_string()
                },
            }
        );
    }
}

#[test]
fn dc_mosfet_rejects_invalid_source_resistance() {
    for source_resistance in [f64::NAN, -1.0] {
        let mut circuit = Circuit::new();
        circuit.add(Element::Mosfet(Mosfet::with_model(
            "Mbad",
            "drain",
            "gate",
            "0",
            "0",
            MosfetType::Nmos,
            MosfetLevel1Params {
                source_resistance,
                ..MosfetLevel1Params::default()
            },
        )));

        let err = dc_op(&circuit).unwrap_err();
        assert_eq!(
            err,
            SpiceError::InvalidElement {
                name: "Mbad".to_string(),
                reason: if source_resistance.is_finite() {
                    "MOSFET RS must be non-negative".to_string()
                } else {
                    "MOSFET RS must be finite".to_string()
                },
            }
        );
    }
}

#[test]
fn dc_mosfet_rejects_invalid_sheet_resistance() {
    for sheet_resistance in [f64::NAN, -1.0] {
        let mut circuit = Circuit::new();
        circuit.add(Element::Mosfet(Mosfet::with_model(
            "Mbad",
            "drain",
            "gate",
            "0",
            "0",
            MosfetType::Nmos,
            MosfetLevel1Params {
                sheet_resistance,
                ..MosfetLevel1Params::default()
            },
        )));

        let err = dc_op(&circuit).unwrap_err();
        assert_eq!(
            err,
            SpiceError::InvalidElement {
                name: "Mbad".to_string(),
                reason: if sheet_resistance.is_finite() {
                    "MOSFET RSH must be non-negative".to_string()
                } else {
                    "MOSFET RSH must be finite".to_string()
                },
            }
        );
    }
}

#[test]
fn dc_mosfet_rejects_invalid_drain_squares() {
    for drain_squares in [f64::NAN, -1.0] {
        let mut circuit = Circuit::new();
        circuit.add(Element::Mosfet(Mosfet::with_model(
            "Mbad",
            "drain",
            "gate",
            "0",
            "0",
            MosfetType::Nmos,
            MosfetLevel1Params {
                drain_squares,
                ..MosfetLevel1Params::default()
            },
        )));

        let err = dc_op(&circuit).unwrap_err();
        assert_eq!(
            err,
            SpiceError::InvalidElement {
                name: "Mbad".to_string(),
                reason: if drain_squares.is_finite() {
                    "MOSFET NRD must be non-negative".to_string()
                } else {
                    "MOSFET NRD must be finite".to_string()
                },
            }
        );
    }
}

#[test]
fn dc_mosfet_rejects_invalid_source_squares() {
    for source_squares in [f64::NAN, -1.0] {
        let mut circuit = Circuit::new();
        circuit.add(Element::Mosfet(Mosfet::with_model(
            "Mbad",
            "drain",
            "gate",
            "0",
            "0",
            MosfetType::Nmos,
            MosfetLevel1Params {
                source_squares,
                ..MosfetLevel1Params::default()
            },
        )));

        let err = dc_op(&circuit).unwrap_err();
        assert_eq!(
            err,
            SpiceError::InvalidElement {
                name: "Mbad".to_string(),
                reason: if source_squares.is_finite() {
                    "MOSFET NRS must be non-negative".to_string()
                } else {
                    "MOSFET NRS must be finite".to_string()
                },
            }
        );
    }
}

#[test]
fn dc_mosfet_rejects_invalid_drain_area() {
    for drain_area in [f64::NAN, -1.0] {
        let mut circuit = Circuit::new();
        circuit.add(Element::Mosfet(Mosfet::with_model(
            "Mbad",
            "drain",
            "gate",
            "0",
            "0",
            MosfetType::Nmos,
            MosfetLevel1Params {
                drain_area,
                ..MosfetLevel1Params::default()
            },
        )));

        let err = dc_op(&circuit).unwrap_err();
        assert_eq!(
            err,
            SpiceError::InvalidElement {
                name: "Mbad".to_string(),
                reason: if drain_area.is_finite() {
                    "MOSFET AD must be non-negative".to_string()
                } else {
                    "MOSFET AD must be finite".to_string()
                },
            }
        );
    }
}

#[test]
fn dc_mosfet_rejects_invalid_source_area() {
    for source_area in [f64::NAN, -1.0] {
        let mut circuit = Circuit::new();
        circuit.add(Element::Mosfet(Mosfet::with_model(
            "Mbad",
            "drain",
            "gate",
            "0",
            "0",
            MosfetType::Nmos,
            MosfetLevel1Params {
                source_area,
                ..MosfetLevel1Params::default()
            },
        )));

        let err = dc_op(&circuit).unwrap_err();
        assert_eq!(
            err,
            SpiceError::InvalidElement {
                name: "Mbad".to_string(),
                reason: if source_area.is_finite() {
                    "MOSFET AS must be non-negative".to_string()
                } else {
                    "MOSFET AS must be finite".to_string()
                },
            }
        );
    }
}

#[test]
fn dc_mosfet_rejects_invalid_source_perimeter() {
    for source_perimeter in [f64::NAN, -1.0] {
        let mut circuit = Circuit::new();
        circuit.add(Element::Mosfet(Mosfet::with_model(
            "Mbad",
            "drain",
            "gate",
            "0",
            "0",
            MosfetType::Nmos,
            MosfetLevel1Params {
                source_perimeter,
                ..MosfetLevel1Params::default()
            },
        )));

        let err = dc_op(&circuit).unwrap_err();
        assert_eq!(
            err,
            SpiceError::InvalidElement {
                name: "Mbad".to_string(),
                reason: if source_perimeter.is_finite() {
                    "MOSFET PS must be non-negative".to_string()
                } else {
                    "MOSFET PS must be finite".to_string()
                },
            }
        );
    }
}

#[test]
fn dc_mosfet_rejects_invalid_bottom_junction_capacitance() {
    for bottom_junction_capacitance in [f64::NAN, -1.0] {
        let mut circuit = Circuit::new();
        circuit.add(Element::Mosfet(Mosfet::with_model(
            "Mbad",
            "drain",
            "gate",
            "0",
            "0",
            MosfetType::Nmos,
            MosfetLevel1Params {
                bottom_junction_capacitance,
                ..MosfetLevel1Params::default()
            },
        )));

        let err = dc_op(&circuit).unwrap_err();
        assert_eq!(
            err,
            SpiceError::InvalidElement {
                name: "Mbad".to_string(),
                reason: if bottom_junction_capacitance.is_finite() {
                    "MOSFET CJ must be non-negative".to_string()
                } else {
                    "MOSFET CJ must be finite".to_string()
                },
            }
        );
    }
}

#[test]
fn dc_mosfet_rejects_invalid_sidewall_grading_coefficient() {
    for sidewall_junction_grading_coefficient in [f64::NAN, -1.0] {
        let mut circuit = Circuit::new();
        circuit.add(Element::Mosfet(Mosfet::with_model(
            "Mbad",
            "drain",
            "gate",
            "0",
            "0",
            MosfetType::Nmos,
            MosfetLevel1Params {
                sidewall_junction_grading_coefficient,
                ..MosfetLevel1Params::default()
            },
        )));

        let err = dc_op(&circuit).unwrap_err();
        assert_eq!(
            err,
            SpiceError::InvalidElement {
                name: "Mbad".to_string(),
                reason: if sidewall_junction_grading_coefficient.is_finite() {
                    "MOSFET MJSW must be non-negative".to_string()
                } else {
                    "MOSFET MJSW must be finite".to_string()
                },
            }
        );
    }
}

#[test]
fn dc_voltage_source_reports_branch_current() {
    let mut circuit = Circuit::new();
    circuit.add(Element::VoltageSource(VoltageSource::new(
        "V1", "n1", "0", 10.0,
    )));
    circuit.add(Element::Resistor(Resistor::new("R1", "n1", "0", 1_000.0)));

    let result = dc_op(&circuit).unwrap();

    assert_close(result.branch_current("V1").unwrap(), -10.0e-3);
    assert_close(result.branch_current("I(V1)").unwrap(), -10.0e-3);
}

#[test]
fn dc_ground_aliases_are_zero_volt_reference() {
    let mut circuit = Circuit::new();
    circuit.add(Element::VoltageSource(VoltageSource::new(
        "V1", "n1", "gnd", 3.3,
    )));
    circuit.add(Element::Resistor(Resistor::new("R1", "n1", "GND", 330.0)));

    let result = dc_op(&circuit).unwrap();

    assert_close(result.voltage("n1").unwrap(), 3.3);
    assert_close(result.voltage("gnd").unwrap(), 0.0);
    assert_close(result.voltage("GND").unwrap(), 0.0);
}

#[test]
fn dc_inductor_behaves_as_ideal_short() {
    let mut circuit = Circuit::new();
    circuit.add(Element::VoltageSource(VoltageSource::new(
        "V1", "vin", "0", 1.0,
    )));
    circuit.add(Element::Resistor(Resistor::new(
        "R1", "vin", "out", 1_000.0,
    )));
    circuit.add(Element::Inductor(Inductor::new("L1", "out", "0", 1.0)));

    let result = dc_op(&circuit).unwrap();

    assert_close(result.voltage("out").unwrap(), 0.0);
    assert_close(result.branch_current("L1").unwrap(), 1.0e-3);
}

#[test]
fn dc_sweep_varies_voltage_source_and_collects_operating_points() {
    let mut circuit = Circuit::new();
    circuit.add(Element::VoltageSource(VoltageSource::new(
        "V1", "vin", "0", 0.0,
    )));
    circuit.add(Element::Resistor(Resistor::new(
        "R1", "vin", "mid", 1_000.0,
    )));
    circuit.add(Element::Resistor(Resistor::new("R2", "mid", "0", 1_000.0)));

    let points = dc_sweep(&circuit, "V1", 0.0, 2.0, 1.0).unwrap();

    assert_eq!(points.len(), 3);
    assert_close(points[0].value, 0.0);
    assert_close(points[0].result.voltage("mid").unwrap(), 0.0);
    assert_close(points[1].value, 1.0);
    assert_close(points[1].result.voltage("mid").unwrap(), 0.5);
    assert_close(points[2].value, 2.0);
    assert_close(points[2].result.voltage("mid").unwrap(), 1.0);
}

#[test]
fn dc_sweep_supports_current_sources() {
    let mut circuit = Circuit::new();
    circuit.add(Element::CurrentSource(CurrentSource::new(
        "I1", "0", "n1", 0.0,
    )));
    circuit.add(Element::Resistor(Resistor::new("R1", "n1", "0", 1_000.0)));

    let points = dc_sweep(&circuit, "I1", 0.0, 2.0e-3, 1.0e-3).unwrap();

    assert_eq!(points.len(), 3);
    assert_close(points[0].result.voltage("n1").unwrap(), 0.0);
    assert_close(points[1].result.voltage("n1").unwrap(), 1.0);
    assert_close(points[2].result.voltage("n1").unwrap(), 2.0);
}

#[test]
fn dc_sweep_text_output_table_is_stable() {
    let mut circuit = Circuit::new();
    circuit.add(Element::VoltageSource(VoltageSource::new(
        "V1", "vin", "0", 0.0,
    )));
    circuit.add(Element::Resistor(Resistor::new(
        "R1", "vin", "mid", 1_000.0,
    )));
    circuit.add(Element::Resistor(Resistor::new("R2", "mid", "0", 1_000.0)));

    let points = dc_sweep(&circuit, "V1", 0.0, 2.0, 1.0).unwrap();

    assert_eq!(
        format_dc_sweep_table("V1", &points, &["V(mid)", "I(V1)"]).unwrap(),
        "Index\tSource\tValue\tV(mid)\tI(V1)\n0\tV1\t0.000000e+00\t0.000000e+00\t0.000000e+00\n1\tV1\t1.000000e+00\t5.000000e-01\t-5.000000e-04\n2\tV1\t2.000000e+00\t1.000000e+00\t-1.000000e-03\n"
    );
}

#[test]
fn dc_sweep_measurements_execute_probe_and_parsed_cards() {
    let mut circuit = Circuit::new();
    circuit.add(Element::VoltageSource(VoltageSource::new(
        "V1", "vin", "0", 0.0,
    )));
    circuit.add(Element::Resistor(Resistor::new(
        "R1", "vin", "mid", 1_000.0,
    )));
    circuit.add(Element::Resistor(Resistor::new("R2", "mid", "0", 1_000.0)));

    let points = dc_sweep(&circuit, "V1", 0.0, 2.0, 1.0).unwrap();
    let peak =
        measure_dc_sweep_probe(&points, "mid_peak", "V(mid)", "max", Some(1.0), Some(2.0)).unwrap();
    let average = measure_dc_sweep_probe(&points, "mid_avg", "V(mid)", "avg", None, None).unwrap();

    assert_close(peak.value, 1.0);
    assert_eq!(peak.analysis, "dc");
    assert_close(average.value, 0.5);
    assert_eq!(
        format_measurement_table(&[peak, average]),
        "Name\tAnalysis\tProbe\tMode\tFrom\tTo\tValue\nmid_peak\tdc\tV(mid)\tmax\t1.000000e+00\t2.000000e+00\t1.000000e+00\nmid_avg\tdc\tV(mid)\tavg\t\t\t5.000000e-01\n"
    );

    let measurements = measure_dc_sweep_deck(
        &points,
        "
.measure dc mid_swing PP V(mid) FROM=0 TO=2
.meas dc mid_final FINAL V(mid)
.end
",
    )
    .unwrap();

    assert_eq!(
        format_measurement_table(&measurements),
        "Name\tAnalysis\tProbe\tMode\tFrom\tTo\tValue\nmid_swing\tdc\tV(mid)\tpp\t0.000000e+00\t2.000000e+00\t1.000000e+00\nmid_final\tdc\tV(mid)\tlast\t\t\t1.000000e+00\n"
    );
}

#[test]
fn dc_sweep_rejects_step_that_does_not_reach_stop() {
    let circuit = Circuit::new();

    assert!(matches!(
        dc_sweep(&circuit, "V1", 0.0, 1.0, -0.1),
        Err(SpiceError::InvalidElement { name, .. }) if name == "V1"
    ));
}

#[test]
fn dc_sweep_rejects_missing_source() {
    let circuit = Circuit::new();

    assert!(matches!(
        dc_sweep(&circuit, "Vmissing", 0.0, 1.0, 1.0),
        Err(SpiceError::InvalidElement { name, .. }) if name == "Vmissing"
    ));
}

#[test]
fn dc_op_rejects_invalid_options() {
    let circuit = Circuit::new();

    assert!(matches!(
        dc_op_with_options(
            &circuit,
            DcOpOptions {
                max_iterations: 0,
                ..DcOpOptions::default()
            },
        ),
        Err(SpiceError::InvalidElement { name, reason })
            if name == "dc_op" && reason == "max_iterations must be positive"
    ));
    assert!(matches!(
        dc_op_with_options(
            &circuit,
            DcOpOptions {
                tolerance: 0.0,
                ..DcOpOptions::default()
            },
        ),
        Err(SpiceError::InvalidElement { name, reason })
            if name == "dc_op" && reason == "tolerance must be finite and positive"
    ));
    assert!(matches!(
        dc_op_with_options(
            &circuit,
            DcOpOptions {
                newton_step_limit: Some(0.0),
                ..DcOpOptions::default()
            },
        ),
        Err(SpiceError::InvalidElement { name, reason })
            if name == "dc_op" && reason == "newton_step_limit must be finite and positive"
    ));
}

#[test]
fn dc_singular_floating_resistor_returns_error() {
    let mut circuit = Circuit::new();
    circuit.add(Element::Resistor(Resistor::new("R1", "a", "b", 1_000.0)));

    assert_eq!(dc_op(&circuit), Err(SpiceError::SingularMatrix));
}

#[test]
fn dc_rejects_non_positive_resistance() {
    let mut circuit = Circuit::new();
    circuit.add(Element::Resistor(Resistor::new("Rbad", "n1", "0", 0.0)));

    assert!(matches!(
        dc_op(&circuit),
        Err(SpiceError::InvalidElement { name, .. }) if name == "Rbad"
    ));
}

#[test]
fn dc_rejects_duplicate_voltage_source_names() {
    let mut circuit = Circuit::new();
    circuit.add(Element::VoltageSource(VoltageSource::new(
        "V1", "n1", "0", 1.0,
    )));
    circuit.add(Element::VoltageSource(VoltageSource::new(
        "V1", "n2", "0", 2.0,
    )));

    assert!(matches!(
        dc_op(&circuit),
        Err(SpiceError::InvalidElement { name, .. }) if name == "V1"
    ));
}

#[test]
fn dc_rejects_non_finite_vccs_transconductance() {
    let mut circuit = Circuit::new();
    circuit.add(Element::VoltageSource(VoltageSource::new(
        "Vctrl", "ctrl", "0", 1.0,
    )));
    circuit.add(Element::Vccs(Vccs::new(
        "Gbad",
        "0",
        "out",
        "ctrl",
        "0",
        f64::NAN,
    )));
    circuit.add(Element::Resistor(Resistor::new(
        "Rload", "out", "0", 1_000.0,
    )));

    assert!(matches!(
        dc_op(&circuit),
        Err(SpiceError::InvalidElement { name, .. }) if name == "Gbad"
    ));
}

#[test]
fn dc_rejects_non_finite_vcvs_gain() {
    let mut circuit = Circuit::new();
    circuit.add(Element::VoltageSource(VoltageSource::new(
        "Vctrl", "ctrl", "0", 1.0,
    )));
    circuit.add(Element::Vcvs(Vcvs::new(
        "Ebad",
        "out",
        "0",
        "ctrl",
        "0",
        f64::NAN,
    )));
    circuit.add(Element::Resistor(Resistor::new(
        "Rload", "out", "0", 1_000.0,
    )));

    assert!(matches!(
        dc_op(&circuit),
        Err(SpiceError::InvalidElement { name, .. }) if name == "Ebad"
    ));
}

#[test]
fn dc_rejects_non_finite_cccs_gain() {
    let mut circuit = Circuit::new();
    circuit.add(Element::VoltageSource(VoltageSource::new(
        "Vsense", "sense", "0", 0.0,
    )));
    circuit.add(Element::Cccs(Cccs::new(
        "Fbad",
        "0",
        "out",
        "Vsense",
        f64::NAN,
    )));
    circuit.add(Element::Resistor(Resistor::new(
        "Rload", "out", "0", 1_000.0,
    )));

    assert!(matches!(
        dc_op(&circuit),
        Err(SpiceError::InvalidElement { name, .. }) if name == "Fbad"
    ));
}

#[test]
fn dc_rejects_non_finite_ccvs_transresistance() {
    let mut circuit = Circuit::new();
    circuit.add(Element::VoltageSource(VoltageSource::new(
        "Vsense", "sense", "0", 0.0,
    )));
    circuit.add(Element::Ccvs(Ccvs::new(
        "Hbad",
        "out",
        "0",
        "Vsense",
        f64::NAN,
    )));
    circuit.add(Element::Resistor(Resistor::new(
        "Rload", "out", "0", 1_000.0,
    )));

    assert!(matches!(
        dc_op(&circuit),
        Err(SpiceError::InvalidElement { name, .. }) if name == "Hbad"
    ));
}

#[test]
fn dc_rejects_missing_cccs_control_source() {
    let mut circuit = Circuit::new();
    circuit.add(Element::Cccs(Cccs::new(
        "Fbad", "0", "out", "Vmissing", 2.0,
    )));
    circuit.add(Element::Resistor(Resistor::new(
        "Rload", "out", "0", 1_000.0,
    )));

    assert!(matches!(
        dc_op(&circuit),
        Err(SpiceError::InvalidElement { name, .. }) if name == "Fbad"
    ));
}

#[test]
fn dc_rejects_missing_ccvs_control_source() {
    let mut circuit = Circuit::new();
    circuit.add(Element::Ccvs(Ccvs::new(
        "Hbad", "out", "0", "Vmissing", 2_000.0,
    )));
    circuit.add(Element::Resistor(Resistor::new(
        "Rload", "out", "0", 1_000.0,
    )));

    assert!(matches!(
        dc_op(&circuit),
        Err(SpiceError::InvalidElement { name, .. }) if name == "Hbad"
    ));
}

#[test]
fn dc_op_uses_static_source_value_when_waveform_is_present() {
    let mut circuit = Circuit::new();
    circuit.add(Element::VoltageSource(VoltageSource::with_waveform(
        "V1",
        "n1",
        "0",
        3.0,
        Waveform::Sin(SinWaveform::new(0.0, 10.0, 1_000.0)),
    )));
    circuit.add(Element::Resistor(Resistor::new("R1", "n1", "0", 1_000.0)));

    let result = dc_op(&circuit).unwrap();

    assert_close(result.voltage("n1").unwrap(), 3.0);
}

#[test]
fn dc_corners_runs_named_parameter_overrides() {
    let mut circuit = Circuit::new();
    circuit.add(Element::VoltageSource(VoltageSource::new(
        "Vin", "in", "0", 10.0,
    )));
    circuit.add(Element::Resistor(Resistor::new(
        "Rtop", "in", "out", 1_000.0,
    )));
    circuit.add(Element::Resistor(Resistor::new(
        "Rbot", "out", "0", 1_000.0,
    )));

    let result = dc_corners(
        &circuit,
        &[
            CornerSpec::new("nominal", Vec::new()),
            CornerSpec::new(
                "rbot-fast",
                vec![CornerOverride::new("Rbot", "resistance", 500.0)],
            ),
            CornerSpec::new(
                "vin-high",
                vec![CornerOverride::new("Vin", "voltage", 12.0)],
            ),
            CornerSpec::new(
                "vin-inverted",
                vec![CornerOverride::new("Vin", "voltage", -10.0)],
            ),
        ],
        DcOpOptions::default(),
    )
    .unwrap();

    assert_eq!(result.points[0].corner_name, "nominal");
    assert_eq!(result.points[1].corner_name, "rbot-fast");
    assert_eq!(result.points[2].corner_name, "vin-high");
    assert_eq!(result.points[3].corner_name, "vin-inverted");
    assert_close(result.points[0].result.voltage("out").unwrap(), 5.0);
    assert_close(result.points[1].result.voltage("out").unwrap(), 10.0 / 3.0);
    assert_close(result.points[2].result.voltage("out").unwrap(), 6.0);
    assert_close(result.points[3].result.voltage("out").unwrap(), -5.0);
}

#[test]
fn dc_corners_parallel_matches_ordered_sequential_results() {
    let mut circuit = Circuit::new();
    circuit.add(Element::VoltageSource(VoltageSource::new(
        "Vin", "in", "0", 10.0,
    )));
    circuit.add(Element::Resistor(Resistor::new(
        "Rtop", "in", "out", 1_000.0,
    )));
    circuit.add(Element::Resistor(Resistor::new(
        "Rbot", "out", "0", 1_000.0,
    )));
    let corners = [
        CornerSpec::new("nominal", Vec::new()),
        CornerSpec::new(
            "rbot-fast",
            vec![CornerOverride::new("Rbot", "resistance", 500.0)],
        ),
        CornerSpec::new(
            "vin-high",
            vec![CornerOverride::new("Vin", "voltage", 12.0)],
        ),
        CornerSpec::new(
            "vin-inverted",
            vec![CornerOverride::new("Vin", "voltage", -10.0)],
        ),
    ];

    let sequential = dc_corners(&circuit, &corners, DcOpOptions::default()).unwrap();
    let parallel = dc_corners_parallel(&circuit, &corners, DcOpOptions::default()).unwrap();

    assert_eq!(parallel.points.len(), sequential.points.len());
    for (parallel_point, sequential_point) in parallel.points.iter().zip(sequential.points.iter()) {
        assert_eq!(parallel_point.corner_name, sequential_point.corner_name);
        assert_close(
            parallel_point.result.voltage("out").unwrap(),
            sequential_point.result.voltage("out").unwrap(),
        );
        assert_close(
            parallel_point.result.branch_current("Vin").unwrap(),
            sequential_point.result.branch_current("Vin").unwrap(),
        );
    }
    assert_eq!(
        format_corner_dc_table(&parallel, &["V(out)", "I(Vin)"]).unwrap(),
        "Corner\tIndex\tV(out)\tI(Vin)\nnominal\t0\t5.000000e+00\t-5.000000e-03\nrbot-fast\t0\t3.333333e+00\t-6.666667e-03\nvin-high\t0\t6.000000e+00\t-6.000000e-03\nvin-inverted\t0\t-5.000000e+00\t5.000000e-03\n"
    );
}

#[test]
fn dc_corners_parallel_reports_corner_override_errors() {
    let mut circuit = Circuit::new();
    circuit.add(Element::VoltageSource(VoltageSource::new(
        "Vin", "in", "0", 10.0,
    )));
    circuit.add(Element::Resistor(Resistor::new(
        "Rtop", "in", "out", 1_000.0,
    )));
    circuit.add(Element::Resistor(Resistor::new(
        "Rbot", "out", "0", 1_000.0,
    )));
    let corners = [
        CornerSpec::new("nominal", Vec::new()),
        CornerSpec::new(
            "missing",
            vec![CornerOverride::new("Rmissing", "resistance", 500.0)],
        ),
    ];

    assert!(matches!(
        dc_corners_parallel(&circuit, &corners, DcOpOptions::default()),
        Err(SpiceError::InvalidElement { name, .. }) if name == "dc_corners"
    ));
}

#[test]
fn corner_dc_text_output_table_is_stable() {
    let mut circuit = Circuit::new();
    circuit.add(Element::VoltageSource(VoltageSource::new(
        "Vin", "in", "0", 10.0,
    )));
    circuit.add(Element::Resistor(Resistor::new(
        "Rtop", "in", "out", 1_000.0,
    )));
    circuit.add(Element::Resistor(Resistor::new(
        "Rbot", "out", "0", 1_000.0,
    )));

    let result = dc_corners(
        &circuit,
        &[
            CornerSpec::new("nominal", Vec::new()),
            CornerSpec::new(
                "rbot-fast",
                vec![CornerOverride::new("Rbot", "resistance", 500.0)],
            ),
            CornerSpec::new(
                "vin-high",
                vec![CornerOverride::new("Vin", "voltage", 12.0)],
            ),
        ],
        DcOpOptions::default(),
    )
    .unwrap();

    assert_eq!(
        format_corner_dc_table(&result, &["V(out)", "I(Vin)"]).unwrap(),
        "Corner\tIndex\tV(out)\tI(Vin)\nnominal\t0\t5.000000e+00\t-5.000000e-03\nrbot-fast\t0\t3.333333e+00\t-6.666667e-03\nvin-high\t0\t6.000000e+00\t-6.000000e-03\n"
    );
}

#[test]
fn dc_sweep_corners_runs_source_sweeps_per_corner() {
    let mut circuit = Circuit::new();
    circuit.add(Element::VoltageSource(VoltageSource::new(
        "Vin", "in", "0", 0.0,
    )));
    circuit.add(Element::Resistor(Resistor::new(
        "Rtop", "in", "out", 1_000.0,
    )));
    circuit.add(Element::Resistor(Resistor::new(
        "Rbot", "out", "0", 1_000.0,
    )));

    let result = dc_sweep_corners(
        &circuit,
        "Vin",
        0.0,
        10.0,
        5.0,
        &[
            CornerSpec::new("nominal", Vec::new()),
            CornerSpec::new(
                "rbot-fast",
                vec![CornerOverride::new("Rbot", "resistance", 500.0)],
            ),
        ],
    )
    .unwrap();

    assert_eq!(result.source_name, "Vin");
    assert_eq!(result.points[0].corner_name, "nominal");
    assert_eq!(result.points[1].corner_name, "rbot-fast");
    assert_eq!(result.points[0].points.len(), 3);
    assert_close(result.points[0].points[0].value, 0.0);
    assert_close(result.points[0].points[1].value, 5.0);
    assert_close(result.points[0].points[2].value, 10.0);
    assert_close(
        result.points[0].points[0].result.voltage("out").unwrap(),
        0.0,
    );
    assert_close(
        result.points[0].points[1].result.voltage("out").unwrap(),
        2.5,
    );
    assert_close(
        result.points[0].points[2].result.voltage("out").unwrap(),
        5.0,
    );
    assert_close(
        result.points[1].points[1].result.voltage("out").unwrap(),
        5.0 / 3.0,
    );
    assert_close(
        result.points[1].points[2].result.voltage("out").unwrap(),
        10.0 / 3.0,
    );
}

#[test]
fn dc_sweep_corners_parallel_matches_ordered_sequential_results() {
    let mut circuit = Circuit::new();
    circuit.add(Element::VoltageSource(VoltageSource::new(
        "Vin", "in", "0", 0.0,
    )));
    circuit.add(Element::Resistor(Resistor::new(
        "Rtop", "in", "out", 1_000.0,
    )));
    circuit.add(Element::Resistor(Resistor::new(
        "Rbot", "out", "0", 1_000.0,
    )));
    let corners = [
        CornerSpec::new("nominal", Vec::new()),
        CornerSpec::new(
            "rbot-fast",
            vec![CornerOverride::new("Rbot", "resistance", 500.0)],
        ),
        CornerSpec::new(
            "rtop-slow",
            vec![CornerOverride::new("Rtop", "resistance", 2_000.0)],
        ),
    ];

    let sequential = dc_sweep_corners(&circuit, "Vin", 0.0, 10.0, 5.0, &corners).unwrap();
    let parallel = dc_sweep_corners_parallel(&circuit, "Vin", 0.0, 10.0, 5.0, &corners).unwrap();

    assert_eq!(parallel.source_name, sequential.source_name);
    assert_eq!(parallel.points.len(), sequential.points.len());
    for (parallel_corner, sequential_corner) in parallel.points.iter().zip(sequential.points.iter())
    {
        assert_eq!(parallel_corner.corner_name, sequential_corner.corner_name);
        assert_eq!(parallel_corner.points.len(), sequential_corner.points.len());
        for (parallel_point, sequential_point) in parallel_corner
            .points
            .iter()
            .zip(sequential_corner.points.iter())
        {
            assert_close(parallel_point.value, sequential_point.value);
            assert_close(
                parallel_point.result.voltage("out").unwrap(),
                sequential_point.result.voltage("out").unwrap(),
            );
            assert_close(
                parallel_point.result.branch_current("Vin").unwrap(),
                sequential_point.result.branch_current("Vin").unwrap(),
            );
        }
    }
    assert_eq!(
        format_corner_dc_sweep_table(&parallel, &["V(out)", "I(Vin)"]).unwrap(),
        "Corner\tIndex\tSource\tValue\tV(out)\tI(Vin)\nnominal\t0\tVin\t0.000000e+00\t0.000000e+00\t0.000000e+00\nnominal\t1\tVin\t5.000000e+00\t2.500000e+00\t-2.500000e-03\nnominal\t2\tVin\t1.000000e+01\t5.000000e+00\t-5.000000e-03\nrbot-fast\t0\tVin\t0.000000e+00\t0.000000e+00\t0.000000e+00\nrbot-fast\t1\tVin\t5.000000e+00\t1.666667e+00\t-3.333333e-03\nrbot-fast\t2\tVin\t1.000000e+01\t3.333333e+00\t-6.666667e-03\nrtop-slow\t0\tVin\t0.000000e+00\t0.000000e+00\t0.000000e+00\nrtop-slow\t1\tVin\t5.000000e+00\t1.666667e+00\t-1.666667e-03\nrtop-slow\t2\tVin\t1.000000e+01\t3.333333e+00\t-3.333333e-03\n"
    );
}

#[test]
fn dc_sweep_corners_parallel_rejects_invalid_sweep_before_workers() {
    let circuit = Circuit::new();
    let corners = [CornerSpec::new("nominal", Vec::new())];

    assert!(matches!(
        dc_sweep_corners_parallel(&circuit, "Vin", 0.0, 10.0, 0.0, &corners),
        Err(SpiceError::InvalidElement { name, reason })
            if name == "Vin" && reason.contains("non-zero step")
    ));
}

#[test]
fn corner_dc_sweep_text_output_table_is_stable() {
    let mut circuit = Circuit::new();
    circuit.add(Element::VoltageSource(VoltageSource::new(
        "Vin", "in", "0", 0.0,
    )));
    circuit.add(Element::Resistor(Resistor::new(
        "Rtop", "in", "out", 1_000.0,
    )));
    circuit.add(Element::Resistor(Resistor::new(
        "Rbot", "out", "0", 1_000.0,
    )));

    let result = dc_sweep_corners(
        &circuit,
        "Vin",
        0.0,
        10.0,
        5.0,
        &[
            CornerSpec::new("nominal", Vec::new()),
            CornerSpec::new(
                "rbot-fast",
                vec![CornerOverride::new("Rbot", "resistance", 500.0)],
            ),
        ],
    )
    .unwrap();

    assert_eq!(
        format_corner_dc_sweep_table(&result, &["V(out)", "I(Vin)"]).unwrap(),
        "Corner\tIndex\tSource\tValue\tV(out)\tI(Vin)\nnominal\t0\tVin\t0.000000e+00\t0.000000e+00\t0.000000e+00\nnominal\t1\tVin\t5.000000e+00\t2.500000e+00\t-2.500000e-03\nnominal\t2\tVin\t1.000000e+01\t5.000000e+00\t-5.000000e-03\nrbot-fast\t0\tVin\t0.000000e+00\t0.000000e+00\t0.000000e+00\nrbot-fast\t1\tVin\t5.000000e+00\t1.666667e+00\t-3.333333e-03\nrbot-fast\t2\tVin\t1.000000e+01\t3.333333e+00\t-6.666667e-03\n"
    );
}
