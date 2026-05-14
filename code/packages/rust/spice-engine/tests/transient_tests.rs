use spice_engine::{
    transient, Capacitor, Circuit, Element, Inductor, Resistor, SpiceError, VoltageSource,
};

fn assert_close(actual: f64, expected: f64) {
    assert!(
        (actual - expected).abs() < 1.0e-9,
        "expected {expected}, got {actual}"
    );
}

#[test]
fn transient_rc_step_uses_backward_euler_capacitor_companion() {
    let mut circuit = Circuit::new();
    circuit.add(Element::VoltageSource(VoltageSource::new(
        "V1", "vin", "0", 1.0,
    )));
    circuit.add(Element::Resistor(Resistor::new(
        "R1", "vin", "out", 1_000.0,
    )));
    circuit.add(Element::Capacitor(Capacitor::new("C1", "out", "0", 1.0e-6)));

    let points = transient(&circuit, 1.0e-3, 3.0e-3).unwrap();

    assert_eq!(points.len(), 3);
    assert_close(points[0].time, 1.0e-3);
    assert_close(points[0].voltage("out").unwrap(), 0.5);
    assert_close(points[1].voltage("out").unwrap(), 0.75);
    assert_close(points[2].voltage("out").unwrap(), 0.875);
}

#[test]
fn transient_respects_capacitor_initial_voltage() {
    let mut circuit = Circuit::new();
    circuit.add(Element::Resistor(Resistor::new("R1", "out", "0", 1_000.0)));
    circuit.add(Element::Capacitor(Capacitor::with_initial_voltage(
        "C1", "out", "0", 1.0e-6, 1.0,
    )));

    let points = transient(&circuit, 1.0e-3, 2.0e-3).unwrap();

    assert_close(points[0].voltage("out").unwrap(), 0.5);
    assert_close(points[1].voltage("out").unwrap(), 0.25);
}

#[test]
fn transient_rl_step_uses_backward_euler_inductor_companion() {
    let mut circuit = Circuit::new();
    circuit.add(Element::VoltageSource(VoltageSource::new(
        "V1", "vin", "0", 1.0,
    )));
    circuit.add(Element::Resistor(Resistor::new(
        "R1", "vin", "out", 1_000.0,
    )));
    circuit.add(Element::Inductor(Inductor::new("L1", "out", "0", 1.0)));

    let points = transient(&circuit, 1.0e-3, 3.0e-3).unwrap();

    assert_eq!(points.len(), 3);
    assert_close(points[0].voltage("out").unwrap(), 0.5);
    assert_close(points[0].branch_current("L1").unwrap(), 0.5e-3);
    assert_close(points[1].voltage("out").unwrap(), 0.25);
    assert_close(points[1].branch_current("L1").unwrap(), 0.75e-3);
    assert_close(points[2].voltage("out").unwrap(), 0.125);
    assert_close(points[2].branch_current("L1").unwrap(), 0.875e-3);
}

#[test]
fn transient_respects_inductor_initial_current() {
    let mut circuit = Circuit::new();
    circuit.add(Element::Resistor(Resistor::new("R1", "out", "0", 1_000.0)));
    circuit.add(Element::Inductor(Inductor::with_initial_current(
        "L1", "out", "0", 1.0, 1.0e-3,
    )));

    let points = transient(&circuit, 1.0e-3, 2.0e-3).unwrap();

    assert_close(points[0].voltage("out").unwrap(), -0.5);
    assert_close(points[0].branch_current("L1").unwrap(), 0.5e-3);
    assert_close(points[1].voltage("out").unwrap(), -0.25);
    assert_close(points[1].branch_current("L1").unwrap(), 0.25e-3);
}

#[test]
fn transient_rejects_non_positive_capacitance() {
    let mut circuit = Circuit::new();
    circuit.add(Element::Capacitor(Capacitor::new("Cbad", "out", "0", 0.0)));

    assert!(matches!(
        transient(&circuit, 1.0e-3, 1.0e-3),
        Err(SpiceError::InvalidElement { name, .. }) if name == "Cbad"
    ));
}

#[test]
fn transient_rejects_non_positive_inductance() {
    let mut circuit = Circuit::new();
    circuit.add(Element::Inductor(Inductor::new("Lbad", "out", "0", 0.0)));

    assert!(matches!(
        transient(&circuit, 1.0e-3, 1.0e-3),
        Err(SpiceError::InvalidElement { name, .. }) if name == "Lbad"
    ));
}

#[test]
fn transient_rejects_non_positive_time_step() {
    let circuit = Circuit::new();

    assert!(matches!(
        transient(&circuit, 0.0, 1.0e-3),
        Err(SpiceError::InvalidElement { name, .. }) if name == "transient"
    ));
}
