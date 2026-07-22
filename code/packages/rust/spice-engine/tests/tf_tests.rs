use spice_engine::{
    format_corner_tf_table, format_tf_table, tf, tf_corners, tf_corners_parallel, Bjt, BjtPolarity,
    Capacitor, Cccs, Ccvs, Circuit, CornerOverride, CornerSpec, CurrentSource, Element, Inductor,
    Mosfet, MosfetLevel1Params, MosfetType, Resistor, SpiceError, TfResult, Vccs, Vcvs,
    VoltageSource,
};

fn assert_close(actual: f64, expected: f64) {
    assert!(
        (actual - expected).abs() < 1.0e-9,
        "expected {expected}, got {actual}"
    );
}

#[test]
fn tf_result_exposes_gain_alias() {
    let result = TfResult {
        transfer_ratio: 0.5,
        input_impedance_ohms: 2_000.0,
        output_impedance_ohms: 500.0,
    };

    assert_close(result.gain(), 0.5);
}

#[test]
fn tf_voltage_divider_reports_gain_and_impedances() {
    let mut circuit = Circuit::new();
    circuit.add(Element::VoltageSource(VoltageSource::new(
        "Vin", "vin", "0", 10.0,
    )));
    circuit.add(Element::Resistor(Resistor::new(
        "R1", "vin", "mid", 1_000.0,
    )));
    circuit.add(Element::Resistor(Resistor::new("R2", "mid", "0", 1_000.0)));

    let result = tf(&circuit, "mid", "Vin").unwrap();

    assert_close(result.transfer_ratio, 0.5);
    assert_close(result.input_impedance_ohms, 2_000.0);
    assert_close(result.output_impedance_ohms, 500.0);
}

#[test]
fn tf_text_output_table_is_stable() {
    let result = TfResult {
        transfer_ratio: 0.5,
        input_impedance_ohms: 2_000.0,
        output_impedance_ohms: 500.0,
    };

    assert_eq!(
        format_tf_table(&result),
        "TransferRatio\tInputImpedance\tOutputImpedance\n5.000000e-01\t2.000000e+03\t5.000000e+02\n"
    );
}

#[test]
fn tf_unequal_divider_matches_thevenin_values() {
    let mut circuit = Circuit::new();
    circuit.add(Element::VoltageSource(VoltageSource::new(
        "Vin", "in", "0", 5.0,
    )));
    circuit.add(Element::Resistor(Resistor::new(
        "Rtop", "in", "out", 1_000.0,
    )));
    circuit.add(Element::Resistor(Resistor::new(
        "Rbot", "out", "0", 3_000.0,
    )));

    let result = tf(&circuit, "out", "Vin").unwrap();

    assert_close(result.gain(), 0.75);
    assert_close(result.input_impedance_ohms, 4_000.0);
    assert_close(result.output_impedance_ohms, 750.0);
}

#[test]
fn tf_corners_runs_transfer_function_per_corner() {
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

    let result = tf_corners(
        &circuit,
        "out",
        "Vin",
        &[
            CornerSpec::new("nominal", vec![]),
            CornerSpec::new(
                "rbot-fast",
                vec![CornerOverride::new("Rbot", "resistance", 500.0)],
            ),
            CornerSpec::new(
                "rbot-slow",
                vec![CornerOverride::new("Rbot", "resistance", 2_000.0)],
            ),
        ],
    )
    .unwrap();

    assert_eq!(result.input_source, "Vin");
    assert_eq!(result.output_node, "out");
    assert_eq!(result.points[0].corner_name, "nominal");
    assert_eq!(result.points[1].corner_name, "rbot-fast");
    assert_eq!(result.points[2].corner_name, "rbot-slow");
    assert_close(result.points[0].result.gain(), 0.5);
    assert_close(result.points[1].result.gain(), 1.0 / 3.0);
    assert_close(result.points[2].result.gain(), 2.0 / 3.0);
    assert_close(result.points[0].result.input_impedance_ohms, 2_000.0);
    assert_close(result.points[1].result.input_impedance_ohms, 1_500.0);
    assert_close(result.points[2].result.input_impedance_ohms, 3_000.0);
}

#[test]
fn corner_tf_text_output_table_is_stable() {
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

    let result = tf_corners(
        &circuit,
        "out",
        "Vin",
        &[
            CornerSpec::new("nominal", vec![]),
            CornerSpec::new(
                "rbot-fast",
                vec![CornerOverride::new("Rbot", "resistance", 500.0)],
            ),
            CornerSpec::new(
                "rbot-slow",
                vec![CornerOverride::new("Rbot", "resistance", 2_000.0)],
            ),
        ],
    )
    .unwrap();

    assert_eq!(
        format_corner_tf_table(&result),
        "Corner\tTransferRatio\tInputImpedance\tOutputImpedance\nnominal\t5.000000e-01\t2.000000e+03\t5.000000e+02\nrbot-fast\t3.333333e-01\t1.500000e+03\t3.333333e+02\nrbot-slow\t6.666667e-01\t3.000000e+03\t6.666667e+02\n"
    );
}

#[test]
fn tf_corners_parallel_matches_ordered_sequential_results() {
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
        CornerSpec::new("nominal", vec![]),
        CornerSpec::new(
            "rbot-fast",
            vec![CornerOverride::new("Rbot", "resistance", 500.0)],
        ),
        CornerSpec::new(
            "rtop-slow",
            vec![CornerOverride::new("Rtop", "resistance", 2_000.0)],
        ),
    ];

    let sequential = tf_corners(&circuit, "out", "Vin", &corners).unwrap();
    let parallel = tf_corners_parallel(&circuit, "out", "Vin", &corners).unwrap();

    assert_eq!(parallel.input_source, sequential.input_source);
    assert_eq!(parallel.output_node, sequential.output_node);
    assert_eq!(parallel.points.len(), sequential.points.len());
    for (parallel_point, sequential_point) in parallel.points.iter().zip(sequential.points.iter()) {
        assert_eq!(parallel_point.corner_name, sequential_point.corner_name);
        assert_close(
            parallel_point.result.transfer_ratio,
            sequential_point.result.transfer_ratio,
        );
        assert_close(
            parallel_point.result.input_impedance_ohms,
            sequential_point.result.input_impedance_ohms,
        );
        assert_close(
            parallel_point.result.output_impedance_ohms,
            sequential_point.result.output_impedance_ohms,
        );
    }
    assert_eq!(
        format_corner_tf_table(&parallel),
        "Corner\tTransferRatio\tInputImpedance\tOutputImpedance\nnominal\t5.000000e-01\t2.000000e+03\t5.000000e+02\nrbot-fast\t3.333333e-01\t1.500000e+03\t3.333333e+02\nrtop-slow\t3.333333e-01\t3.000000e+03\t6.666667e+02\n"
    );
}

#[test]
fn tf_corners_parallel_reports_corner_override_errors() {
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
    let corners = [CornerSpec::new(
        "bad",
        vec![CornerOverride::new("Rmissing", "resistance", 500.0)],
    )];

    assert!(matches!(
        tf_corners_parallel(&circuit, "out", "Vin", &corners),
        Err(SpiceError::InvalidElement { name, reason })
            if name == "dc_corners" && reason.contains("Rmissing")
    ));
}

#[test]
fn tf_current_source_input_reports_transimpedance() {
    let mut circuit = Circuit::new();
    circuit.add(Element::CurrentSource(CurrentSource::new(
        "Iin", "0", "out", 1.0e-3,
    )));
    circuit.add(Element::Resistor(Resistor::new(
        "Rload", "out", "0", 2_000.0,
    )));

    let result = tf(&circuit, "out", "Iin").unwrap();

    assert_close(result.transfer_ratio, 2_000.0);
    assert_close(result.input_impedance_ohms, 2_000.0);
    assert_close(result.output_impedance_ohms, 2_000.0);
}

#[test]
fn tf_vccs_stage_reports_transconductance_gain() {
    let mut circuit = Circuit::new();
    circuit.add(Element::VoltageSource(VoltageSource::new(
        "Vin", "in", "0", 1.0,
    )));
    circuit.add(Element::Vccs(Vccs::new(
        "Gm", "0", "out", "in", "0", 2.0e-3,
    )));
    circuit.add(Element::Resistor(Resistor::new(
        "Rload", "out", "0", 1_000.0,
    )));

    let result = tf(&circuit, "out", "Vin").unwrap();

    assert_close(result.gain(), 2.0);
    assert_close(result.output_impedance_ohms, 1_000.0);
    assert!(result.input_impedance_ohms.is_infinite());
}

#[test]
fn tf_vcvs_stage_reports_voltage_gain() {
    let mut circuit = Circuit::new();
    circuit.add(Element::VoltageSource(VoltageSource::new(
        "Vin", "in", "0", 1.0,
    )));
    circuit.add(Element::Vcvs(Vcvs::new("E1", "out", "0", "in", "0", 4.0)));
    circuit.add(Element::Resistor(Resistor::new(
        "Rload", "out", "0", 1_000.0,
    )));

    let result = tf(&circuit, "out", "Vin").unwrap();

    assert_close(result.gain(), 4.0);
    assert_close(result.output_impedance_ohms, 0.0);
    assert!(result.input_impedance_ohms.is_infinite());
}

#[test]
fn tf_bjt_common_emitter_reports_small_signal_gain() {
    let mut circuit = Circuit::new();
    let thermal_voltage = 0.02585;
    circuit.add(Element::VoltageSource(VoltageSource::new(
        "Vin",
        "base",
        "0",
        thermal_voltage * 2.0_f64.ln(),
    )));
    circuit.add(Element::Bjt(Bjt::with_model(
        "Q1",
        "out",
        "base",
        "0",
        BjtPolarity::Npn,
        25.85e-6,
        100.0,
        thermal_voltage,
        0.0,
        0.0,
        0.0,
        0.0,
    )));
    circuit.add(Element::Resistor(Resistor::new(
        "Rload", "out", "0", 1_000.0,
    )));

    let result = tf(&circuit, "out", "Vin").unwrap();

    assert_close(result.gain(), -2.0);
    assert_close(result.input_impedance_ohms, 50_000.0);
    assert_close(result.output_impedance_ohms, 1_000.0);
}

#[test]
fn tf_bjt_forward_early_voltage_reduces_output_impedance() {
    let output_impedance = |forward_early_voltage: f64| {
        let mut circuit = Circuit::new();
        let thermal_voltage = 0.02585;
        circuit.add(Element::VoltageSource(VoltageSource::new(
            "Vcc", "vcc", "0", 5.0,
        )));
        circuit.add(Element::VoltageSource(VoltageSource::new(
            "Vin",
            "base",
            "0",
            thermal_voltage * 2.0_f64.ln(),
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
            25.85e-6,
            100.0,
            thermal_voltage,
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
        tf(&circuit, "out", "Vin").unwrap().output_impedance_ohms
    };

    assert!(output_impedance(10.0) < output_impedance(0.0));
}

#[test]
fn tf_bjt_reverse_early_voltage_reduces_gain() {
    let gain = |reverse_early_voltage: f64| {
        let mut circuit = Circuit::new();
        circuit.add(Element::VoltageSource(VoltageSource::new(
            "Vin", "base", "0", 0.65,
        )));
        circuit.add(Element::Resistor(Resistor::new(
            "Rload", "out", "0", 1_000.0,
        )));
        let mut transistor = Bjt::new("Q1", "out", "base", "0");
        transistor.reverse_early_voltage = reverse_early_voltage;
        circuit.add(Element::Bjt(transistor));
        tf(&circuit, "out", "Vin").unwrap().gain().abs()
    };

    assert!(gain(1.0) < gain(0.0));
}

#[test]
fn tf_bjt_forward_beta_rolloff_reduces_gain() {
    let gain = |rolloff_current: f64| {
        let mut circuit = Circuit::new();
        circuit.add(Element::VoltageSource(VoltageSource::new(
            "Vin", "base", "0", 0.65,
        )));
        circuit.add(Element::Resistor(Resistor::new(
            "Rload", "out", "0", 1_000.0,
        )));
        let mut transistor = Bjt::new("Q1", "out", "base", "0");
        transistor.forward_beta_rolloff_current = rolloff_current;
        circuit.add(Element::Bjt(transistor));
        tf(&circuit, "out", "Vin").unwrap().gain().abs()
    };

    assert!(gain(1.0e-4) < gain(0.0));
}

#[test]
fn tf_bjt_forward_emission_coefficient_reduces_gain_and_raises_input_impedance() {
    let transfer = |forward_emission_coefficient: f64| {
        let mut circuit = Circuit::new();
        let thermal_voltage = 0.02585;
        circuit.add(Element::VoltageSource(VoltageSource::new(
            "Vin", "base", "0", 0.65,
        )));
        circuit.add(Element::Resistor(Resistor::new(
            "Rload", "out", "0", 1_000.0,
        )));
        circuit.add(Element::Bjt(Bjt::with_model_and_temperature_parameters(
            "Q1",
            "out",
            "base",
            "0",
            BjtPolarity::Npn,
            1.0e-14,
            100.0,
            thermal_voltage,
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
        tf(&circuit, "out", "Vin").unwrap()
    };

    let ideal = transfer(1.0);
    let shaped = transfer(2.0);
    assert!(shaped.gain().abs() < ideal.gain().abs());
    assert!(shaped.input_impedance_ohms > ideal.input_impedance_ohms);
}

#[test]
fn tf_mosfet_common_source_uses_gate_bias_for_small_signal_gain() {
    let mut circuit = Circuit::new();
    circuit.add(Element::VoltageSource(VoltageSource::new(
        "Vdd", "vdd", "0", 5.0,
    )));
    circuit.add(Element::VoltageSource(VoltageSource::new(
        "Vin", "gate", "0", 1.5,
    )));
    circuit.add(Element::Resistor(Resistor::new(
        "Rload", "vdd", "out", 1_000.0,
    )));
    circuit.add(Element::Mosfet(Mosfet::with_model(
        "M1",
        "out",
        "gate",
        "0",
        "0",
        MosfetType::Nmos,
        MosfetLevel1Params {
            vt0: 0.5,
            kp: 1.0e-3,
            lambda: 0.0,
            gamma: 0.0,
            phi: 0.7,
            w: 1.0,
            l: 1.0,
            saturation_current: 1.0e-15,
            n_sub: 1.0,
            t_nom: 300.15,
            ..MosfetLevel1Params::default()
        },
    )));

    let result = tf(&circuit, "out", "Vin").unwrap();

    assert_close(result.gain(), -1.0);
    assert!(result.input_impedance_ohms.is_infinite());
    assert_close(result.output_impedance_ohms, 1_000.0);
}

#[test]
fn tf_cccs_stage_reports_current_gain() {
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

    let result = tf(&circuit, "out", "Vin").unwrap();

    assert_close(result.gain(), 2.0);
    assert_close(result.output_impedance_ohms, 1_000.0);
}

#[test]
fn tf_ccvs_stage_reports_transresistance_gain() {
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

    let result = tf(&circuit, "out", "Vin").unwrap();

    assert_close(result.gain(), 2.0);
    assert_close(result.output_impedance_ohms, 0.0);
}

#[test]
fn tf_capacitor_is_open_and_inductor_is_short_at_dc_small_signal() {
    let mut circuit = Circuit::new();
    circuit.add(Element::VoltageSource(VoltageSource::new(
        "Vin", "in", "0", 1.0,
    )));
    circuit.add(Element::Capacitor(Capacitor::new(
        "Cblock", "in", "blocked", 1.0e-6,
    )));
    circuit.add(Element::Resistor(Resistor::new(
        "Rblocked", "blocked", "0", 1_000.0,
    )));
    circuit.add(Element::Inductor(Inductor::new(
        "Lshort", "in", "out", 1.0e-3,
    )));
    circuit.add(Element::Resistor(Resistor::new(
        "Rload", "out", "0", 1_000.0,
    )));

    let result = tf(&circuit, "out", "Vin").unwrap();

    assert!((result.gain() - 1.0).abs() < 1.0e-9);
    assert!(result.output_impedance_ohms < 1.0e-6);
}

#[test]
fn tf_rejects_missing_output_node() {
    let circuit = Circuit::new();

    assert!(matches!(
        tf(&circuit, "missing", "Vin"),
        Err(SpiceError::InvalidElement { name, .. }) if name == "missing"
    ));
}

#[test]
fn tf_rejects_non_source_input_element() {
    let mut circuit = Circuit::new();
    circuit.add(Element::Resistor(Resistor::new("Rin", "in", "0", 1_000.0)));

    assert!(matches!(
        tf(&circuit, "in", "Rin"),
        Err(SpiceError::InvalidElement { name, .. }) if name == "Rin"
    ));
}

#[test]
fn tf_rejects_vccs_as_input_source() {
    let mut circuit = Circuit::new();
    circuit.add(Element::Vccs(Vccs::new(
        "Gm", "0", "out", "in", "0", 1.0e-3,
    )));

    assert!(matches!(
        tf(&circuit, "out", "Gm"),
        Err(SpiceError::InvalidElement { name, .. }) if name == "Gm"
    ));
}

#[test]
fn tf_rejects_cccs_as_input_source() {
    let mut circuit = Circuit::new();
    circuit.add(Element::VoltageSource(VoltageSource::new(
        "Vsense", "sense", "0", 0.0,
    )));
    circuit.add(Element::Cccs(Cccs::new("F1", "0", "out", "Vsense", 1.0)));
    circuit.add(Element::Resistor(Resistor::new(
        "Rload", "out", "0", 1_000.0,
    )));

    assert!(matches!(
        tf(&circuit, "out", "F1"),
        Err(SpiceError::InvalidElement { name, .. }) if name == "F1"
    ));
}

#[test]
fn tf_rejects_ccvs_as_input_source() {
    let mut circuit = Circuit::new();
    circuit.add(Element::VoltageSource(VoltageSource::new(
        "Vsense", "sense", "0", 0.0,
    )));
    circuit.add(Element::Ccvs(Ccvs::new(
        "H1", "out", "0", "Vsense", 1_000.0,
    )));
    circuit.add(Element::Resistor(Resistor::new(
        "Rload", "out", "0", 1_000.0,
    )));

    assert!(matches!(
        tf(&circuit, "out", "H1"),
        Err(SpiceError::InvalidElement { name, .. }) if name == "H1"
    ));
}
