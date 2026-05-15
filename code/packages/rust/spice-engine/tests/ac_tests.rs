use spice_engine::{
    ac_sweep, Capacitor, Cccs, Ccvs, Circuit, CurrentSource, Element, Inductor, Resistor,
    SpiceError, Vcvs, VoltageSource,
};

fn assert_close(actual: f64, expected: f64) {
    assert!(
        (actual - expected).abs() < 1.0e-9,
        "expected {expected}, got {actual}"
    );
}

#[test]
fn ac_resistive_divider_is_frequency_independent() {
    let mut circuit = Circuit::new();
    circuit.add(Element::VoltageSource(VoltageSource::new(
        "V1", "vin", "0", 1.0,
    )));
    circuit.add(Element::Resistor(Resistor::new(
        "R1", "vin", "mid", 1_000.0,
    )));
    circuit.add(Element::Resistor(Resistor::new("R2", "mid", "0", 1_000.0)));

    let points = ac_sweep(&circuit, 10.0, 1_000.0, 1).unwrap();

    assert_eq!(points.len(), 3);
    for point in points {
        let mid = point.voltage("mid").unwrap();
        assert_close(mid.real, 0.5);
        assert_close(mid.imag, 0.0);
        assert_close(mid.abs(), 0.5);
    }
}

#[test]
fn ac_rc_low_pass_has_minus_three_db_corner() {
    let resistance = 1_000.0;
    let capacitance = 1.0e-6;
    let corner = 1.0 / (2.0 * std::f64::consts::PI * resistance * capacitance);

    let mut circuit = Circuit::new();
    circuit.add(Element::VoltageSource(VoltageSource::new(
        "V1", "vin", "0", 1.0,
    )));
    circuit.add(Element::Resistor(Resistor::new(
        "R1", "vin", "out", resistance,
    )));
    circuit.add(Element::Capacitor(Capacitor::new(
        "C1",
        "out",
        "0",
        capacitance,
    )));

    let points = ac_sweep(&circuit, corner, corner, 10).unwrap();

    assert_eq!(points.len(), 1);
    let out = points[0].voltage("out").unwrap();
    assert_close(out.abs(), 1.0 / 2.0_f64.sqrt());
    assert_close(out.phase(), -std::f64::consts::FRAC_PI_4);
}

#[test]
fn ac_rl_high_pass_has_minus_three_db_corner() {
    let resistance = 1_000.0;
    let inductance = 1.0;
    let corner = resistance / (2.0 * std::f64::consts::PI * inductance);

    let mut circuit = Circuit::new();
    circuit.add(Element::VoltageSource(VoltageSource::new(
        "V1", "vin", "0", 1.0,
    )));
    circuit.add(Element::Resistor(Resistor::new(
        "R1", "vin", "out", resistance,
    )));
    circuit.add(Element::Inductor(Inductor::new(
        "L1", "out", "0", inductance,
    )));

    let points = ac_sweep(&circuit, corner, corner, 10).unwrap();

    assert_eq!(points.len(), 1);
    let out = points[0].voltage("out").unwrap();
    assert_close(out.abs(), 1.0 / 2.0_f64.sqrt());
    assert_close(out.phase(), std::f64::consts::FRAC_PI_4);
}

#[test]
fn ac_current_source_injects_real_phasor_current() {
    let mut circuit = Circuit::new();
    circuit.add(Element::CurrentSource(CurrentSource::new(
        "I1", "0", "n1", 1.0e-3,
    )));
    circuit.add(Element::Resistor(Resistor::new("R1", "n1", "0", 1_000.0)));

    let points = ac_sweep(&circuit, 1_000.0, 1_000.0, 10).unwrap();

    assert_eq!(points.len(), 1);
    let n1 = points[0].voltage("n1").unwrap();
    assert_close(n1.real, 1.0);
    assert_close(n1.imag, 0.0);
}

#[test]
fn ac_vcvs_applies_controlled_voltage_gain() {
    let mut circuit = Circuit::new();
    circuit.add(Element::VoltageSource(VoltageSource::new(
        "Vin", "in", "0", 1.0,
    )));
    circuit.add(Element::Vcvs(Vcvs::new("E1", "out", "0", "in", "0", 4.0)));
    circuit.add(Element::Resistor(Resistor::new(
        "Rload", "out", "0", 1_000.0,
    )));

    let points = ac_sweep(&circuit, 1_000.0, 1_000.0, 10).unwrap();

    assert_eq!(points.len(), 1);
    let out = points[0].voltage("out").unwrap();
    assert_close(out.real, 4.0);
    assert_close(out.imag, 0.0);
    assert_close(points[0].branch_current("E1").unwrap().real, -4.0e-3);
}

#[test]
fn ac_cccs_applies_current_gain_from_sensed_branch_current() {
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
    circuit.add(Element::Cccs(Cccs::new("F1", "0", "out", "Vsense", 3.0)));
    circuit.add(Element::Resistor(Resistor::new(
        "Rload", "out", "0", 1_000.0,
    )));

    let points = ac_sweep(&circuit, 1_000.0, 1_000.0, 10).unwrap();

    assert_eq!(points.len(), 1);
    let out = points[0].voltage("out").unwrap();
    assert_close(out.real, 3.0);
    assert_close(out.imag, 0.0);
}

#[test]
fn ac_ccvs_applies_transresistance_from_sensed_branch_current() {
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
        "H1", "out", "0", "Vsense", 3_000.0,
    )));
    circuit.add(Element::Resistor(Resistor::new(
        "Rload", "out", "0", 1_000.0,
    )));

    let points = ac_sweep(&circuit, 1_000.0, 1_000.0, 10).unwrap();

    assert_eq!(points.len(), 1);
    let out = points[0].voltage("out").unwrap();
    assert_close(out.real, 3.0);
    assert_close(out.imag, 0.0);
    assert_close(points[0].branch_current("H1").unwrap().real, -3.0e-3);
}

#[test]
fn ac_sweep_rejects_invalid_frequency_bounds() {
    let circuit = Circuit::new();

    assert!(matches!(
        ac_sweep(&circuit, 0.0, 1.0, 10),
        Err(SpiceError::InvalidElement { name, .. }) if name == "ac_sweep"
    ));
    assert!(matches!(
        ac_sweep(&circuit, 10.0, 1.0, 10),
        Err(SpiceError::InvalidElement { name, .. }) if name == "ac_sweep"
    ));
}
