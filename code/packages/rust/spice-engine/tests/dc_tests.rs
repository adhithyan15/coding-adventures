use spice_engine::{
    analyze_custom_model_source, bjt_from_model_card, circuit_at_temperature, dc_corners,
    dc_corners_parallel, dc_initial_vector_from_conditions, dc_op, dc_op_with_initial_conditions,
    dc_op_with_options, dc_sweep, dc_sweep_corners, dc_sweep_corners_parallel,
    dc_temperature_sweep, dc_temperature_sweep_corners, device_model_audit_fixtures,
    diode_from_model_card, format_corner_dc_sweep_table, format_corner_dc_table,
    format_corner_temperature_dc_table, format_dc_sweep_table, format_measurement_table,
    format_temperature_dc_table, jfet_from_model_card, measure_dc_sweep_deck,
    measure_dc_sweep_probe, mosfet_from_model_card, normalize_model_card,
    normalize_model_card_type, resolve_deck_initial_conditions, BSource, Bjt, BjtPolarity, Cccs,
    Ccvs, Circuit, CornerOverride, CornerSpec, CornerTemperatureDcResult, CurrentSource,
    CustomModel, DcConvergenceAid, DcOpOptions, Diode, Element, Inductor, Jfet, JfetPolarity,
    ModelCardKind, Mosfet, MosfetLevel1Params, MosfetType, Resistor, SinWaveform, SpiceError,
    SubcircuitDefinition, SubcircuitElement, TemperatureDcResult, Vccs, Vcvs, VoltageSource,
    Waveform, XInstance,
};

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
fn model_card_aliases_build_device_instances() {
    let diode_card = normalize_model_card(
        "Dfast",
        "diode",
        &[
            ("JS", 2.0e-14),
            ("CJ", 1.5e-12),
            ("TT", 4.0e-9),
            ("RS", 10.0),
        ],
    )
    .unwrap();
    let diode_model = diode_from_model_card("D1", "a", "k", &diode_card).unwrap();
    assert_close(*diode_card.parameters.get("IS").unwrap(), 2.0e-14);
    assert_close(*diode_card.parameters.get("CJO").unwrap(), 1.5e-12);
    assert_eq!(diode_card.unsupported_parameters, vec!["RS".to_string()]);
    assert_close(diode_model.saturation_current, 2.0e-14);
    assert_close(diode_model.junction_capacitance, 1.5e-12);
    assert_close(diode_model.transit_time, 4.0e-9);

    let bjt_card =
        normalize_model_card("Qsmall", "npn", &[("BETA", 125.0), ("CBE", 2.0e-12)]).unwrap();
    let bjt_model = bjt_from_model_card("Q1", "c", "b", "e", &bjt_card).unwrap();
    assert_close(*bjt_card.parameters.get("BF").unwrap(), 125.0);
    assert_close(*bjt_card.parameters.get("CJE").unwrap(), 2.0e-12);
    assert_eq!(bjt_model.polarity, BjtPolarity::Npn);
    assert_close(bjt_model.forward_beta, 125.0);
    assert_close(bjt_model.base_emitter_capacitance, 2.0e-12);

    let jfet_card = normalize_model_card(
        "Jn",
        "njfet",
        &[("BET", 9.0e-4), ("VT0", -1.8), ("LAM", 0.02)],
    )
    .unwrap();
    let jfet_model = jfet_from_model_card("J1", "d", "g", "s", &jfet_card).unwrap();
    assert_close(*jfet_card.parameters.get("BETA").unwrap(), 9.0e-4);
    assert_close(*jfet_card.parameters.get("VTO").unwrap(), -1.8);
    assert_close(*jfet_card.parameters.get("LAMBDA").unwrap(), 0.02);
    assert_eq!(jfet_model.polarity, JfetPolarity::Njf);
    assert_close(jfet_model.beta, 9.0e-4);
    assert_close(jfet_model.threshold_voltage, -1.8);
    assert_close(jfet_model.channel_length_modulation, 0.02);

    let mos_card = normalize_model_card(
        "Mn",
        "nmos",
        &[
            ("LEVEL", 1.0),
            ("VTO", 0.55),
            ("LAM", 0.04),
            ("NSUB", 1.6),
            ("CJD", 3.0e-13),
        ],
    )
    .unwrap();
    let mos_model = mosfet_from_model_card("M1", "d", "g", "s", "b", &mos_card).unwrap();
    assert_close(*mos_card.parameters.get("VT0").unwrap(), 0.55);
    assert_close(*mos_card.parameters.get("LAMBDA").unwrap(), 0.04);
    assert_close(*mos_card.parameters.get("N_SUB").unwrap(), 1.6);
    assert_close(*mos_card.parameters.get("CBD").unwrap(), 3.0e-13);
    assert_eq!(mos_model.mosfet_type, MosfetType::Nmos);
    assert_close(mos_model.params.vt0, 0.55);
    assert_close(mos_model.params.lambda, 0.04);
    assert_close(mos_model.params.n_sub, 1.6);
    assert_close(mos_model.params.drain_bulk_capacitance, 3.0e-13);
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

    assert!(cold_result.voltage("out").unwrap() > nominal_result.voltage("out").unwrap());
    assert!(hot_result.voltage("out").unwrap() < nominal_result.voltage("out").unwrap());
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
