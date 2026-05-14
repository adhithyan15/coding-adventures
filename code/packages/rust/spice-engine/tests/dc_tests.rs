use spice_engine::{dc_op, Circuit, CurrentSource, Element, Resistor, SpiceError, VoltageSource};

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
