use spice_engine::{
    transient, Capacitor, Circuit, CurrentSource, Element, ExpWaveform, Inductor, PulseWaveform,
    PwlWaveform, Resistor, SinWaveform, SpiceError, VoltageSource, Waveform,
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

#[test]
fn pwl_waveform_interpolates_and_clamps() {
    let waveform = PwlWaveform::new(vec![(0.0, 0.0), (0.5, 1.0), (1.0, -1.0)]);

    assert_close(waveform.value_at(-1.0), 0.0);
    assert_close(waveform.value_at(0.25), 0.5);
    assert_close(waveform.value_at(0.75), 0.0);
    assert_close(waveform.value_at(2.0), -1.0);
}

#[test]
fn transient_voltage_source_uses_pwl_waveform() {
    let mut circuit = Circuit::new();
    circuit.add(Element::VoltageSource(VoltageSource::with_waveform(
        "Vin",
        "in",
        "0",
        0.0,
        Waveform::Pwl(PwlWaveform::new(vec![(0.0, 0.0), (0.5, 1.0), (1.0, 1.0)])),
    )));
    circuit.add(Element::Resistor(Resistor::new(
        "Rload", "in", "0", 1_000.0,
    )));

    let points = transient(&circuit, 0.25, 1.0).unwrap();

    assert_eq!(points.len(), 4);
    assert_close(points[0].voltage("in").unwrap(), 0.5);
    assert_close(points[1].voltage("in").unwrap(), 1.0);
    assert_close(points[2].voltage("in").unwrap(), 1.0);
    assert_close(points[3].voltage("in").unwrap(), 1.0);
}

#[test]
fn sin_waveform_respects_delay_and_damping() {
    let waveform = SinWaveform::with_delay_damping(1.0, 2.0, 1.0, 0.5, 1.0);

    assert_close(waveform.value_at(0.25), 1.0);
    assert!((waveform.value_at(0.75) - (1.0 + 2.0 * (-0.25_f64).exp())).abs() < 1.0e-12);
}

#[test]
fn transient_voltage_source_uses_sin_waveform() {
    let mut circuit = Circuit::new();
    circuit.add(Element::VoltageSource(VoltageSource::with_waveform(
        "Vin",
        "in",
        "0",
        0.0,
        Waveform::Sin(SinWaveform::new(0.0, 2.0, 1.0)),
    )));
    circuit.add(Element::Resistor(Resistor::new(
        "Rload", "in", "0", 1_000.0,
    )));

    let points = transient(&circuit, 0.25, 0.5).unwrap();

    assert_close(points[0].voltage("in").unwrap(), 2.0);
    assert_close(points[1].voltage("in").unwrap(), 0.0);
}

#[test]
fn pulse_waveform_repeats_with_edges() {
    let waveform = PulseWaveform::new(0.0, 5.0, 0.0, 0.2, 0.2, 0.4, 1.0);

    assert_close(waveform.value_at(0.1), 2.5);
    assert_close(waveform.value_at(0.3), 5.0);
    assert_close(waveform.value_at(0.7), 2.5);
    assert_close(waveform.value_at(1.3), 5.0);
}

#[test]
fn transient_current_source_uses_pulse_waveform() {
    let mut circuit = Circuit::new();
    circuit.add(Element::CurrentSource(CurrentSource::with_waveform(
        "Iin",
        "0",
        "out",
        0.0,
        Waveform::Pulse(PulseWaveform::new(0.0, 0.01, 0.0, 0.0, 0.0, 0.5, 1.0)),
    )));
    circuit.add(Element::Resistor(Resistor::new("Rload", "out", "0", 100.0)));

    let points = transient(&circuit, 0.25, 0.75).unwrap();

    assert_close(points[0].voltage("out").unwrap(), 1.0);
    assert_close(points[1].voltage("out").unwrap(), 0.0);
    assert_close(points[2].voltage("out").unwrap(), 0.0);
}

#[test]
fn exp_waveform_rises_and_falls() {
    let waveform = ExpWaveform::new(0.0, 2.0, 0.0, 0.5, 1.0, 0.5);

    let rising = waveform.value_at(0.5);
    let falling = waveform.value_at(2.0);

    assert!(rising > 0.0 && rising < 2.0);
    assert!(falling < rising);
}

#[test]
fn transient_voltage_source_uses_exp_waveform() {
    let mut circuit = Circuit::new();
    circuit.add(Element::VoltageSource(VoltageSource::with_waveform(
        "Vin",
        "in",
        "0",
        0.0,
        Waveform::Exp(ExpWaveform::new(0.0, 1.0, 0.0, 0.5, 10.0, 1.0)),
    )));
    circuit.add(Element::Resistor(Resistor::new(
        "Rload", "in", "0", 1_000.0,
    )));

    let points = transient(&circuit, 0.5, 1.0).unwrap();

    assert_close(points[0].voltage("in").unwrap(), 1.0 - (-1.0_f64).exp());
    assert_close(points[1].voltage("in").unwrap(), 1.0 - (-2.0_f64).exp());
}

#[test]
fn transient_rejects_invalid_pwl_waveform() {
    let mut circuit = Circuit::new();
    circuit.add(Element::VoltageSource(VoltageSource::with_waveform(
        "Vin",
        "in",
        "0",
        0.0,
        Waveform::Pwl(PwlWaveform::new(vec![(0.0, 0.0), (0.0, 1.0)])),
    )));
    circuit.add(Element::Resistor(Resistor::new(
        "Rload", "in", "0", 1_000.0,
    )));

    assert!(matches!(
        transient(&circuit, 0.1, 0.1),
        Err(SpiceError::InvalidElement { name, .. }) if name == "Vin"
    ));
}
