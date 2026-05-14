use spice_engine::{
    tf, Capacitor, Cccs, Circuit, CurrentSource, Element, Inductor, Resistor, SpiceError, TfResult,
    Vccs, Vcvs, VoltageSource,
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
