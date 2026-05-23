use spice_engine::{
    dc_corners, dc_op, dc_op_with_options, dc_sweep, dc_sweep_corners, BSource, Bjt, BjtPolarity,
    Cccs, Ccvs, Circuit, CornerOverride, CornerSpec, CurrentSource, DcConvergenceAid, DcOpOptions,
    Diode, Element, Inductor, Jfet, JfetPolarity, Mosfet, MosfetLevel1Params, MosfetType, Resistor,
    SinWaveform, SpiceError, SubcircuitDefinition, SubcircuitElement, Vccs, Vcvs, VoltageSource,
    Waveform, XInstance,
};

fn assert_close(actual: f64, expected: f64) {
    assert!(
        (actual - expected).abs() < 1.0e-9,
        "expected {expected}, got {actual}"
    );
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
