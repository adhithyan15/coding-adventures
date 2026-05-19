use spice_engine::{
    estimate_period, pss_residual, transient, Capacitor, Cccs, Ccvs, Circuit, CurrentSource,
    Element, ExpWaveform, Inductor, PssResidualResult, PulseWaveform, PwlWaveform, Resistor,
    SinWaveform, SpiceError, VoltageSource, Waveform,
};

fn assert_close(actual: f64, expected: f64) {
    assert!(
        (actual - expected).abs() < 1.0e-9,
        "expected {expected}, got {actual}"
    );
}

#[test]
fn waveform_period_reports_periodic_source_forms() {
    assert_close(
        Waveform::Sin(SinWaveform::new(0.0, 1.0, 2.0))
            .period_seconds()
            .unwrap(),
        0.5,
    );
    assert!(
        Waveform::Sin(SinWaveform::with_delay_damping(0.0, 1.0, 2.0, 0.0, 1.0))
            .period_seconds()
            .is_none()
    );
    assert!(Waveform::Sin(SinWaveform::new(0.0, 1.0, 0.0))
        .period_seconds()
        .is_none());
    assert_close(
        Waveform::Pulse(PulseWaveform::new(0.0, 1.0, 0.0, 0.0, 0.0, 0.5, 2.5))
            .period_seconds()
            .unwrap(),
        2.5,
    );
    assert!(
        Waveform::Pwl(PwlWaveform::new(vec![(0.0, 0.0), (1.0, 1.0)]))
            .period_seconds()
            .is_none()
    );
    assert!(
        Waveform::Exp(ExpWaveform::new(0.0, 1.0, 0.0, 0.5, 1.0, 0.5))
            .period_seconds()
            .is_none()
    );
}

#[test]
fn estimate_period_finds_harmonic_periodic_source_period() {
    let mut circuit = Circuit::new();
    circuit.add(Element::VoltageSource(VoltageSource::with_waveform(
        "V1",
        "in",
        "0",
        0.0,
        Waveform::Sin(SinWaveform::new(0.0, 1.0, 1_000.0)),
    )));
    circuit.add(Element::CurrentSource(CurrentSource::with_waveform(
        "I1",
        "out",
        "0",
        0.0,
        Waveform::Pulse(PulseWaveform::new(
            0.0, 1.0e-3, 0.0, 0.0, 0.0, 0.25e-3, 0.5e-3,
        )),
    )));
    circuit.add(Element::Resistor(Resistor::new("R1", "in", "out", 1_000.0)));

    assert_close(estimate_period(&circuit).unwrap(), 1.0e-3);
}

#[test]
fn estimate_period_rejects_nonperiodic_or_incommensurate_sources() {
    let mut non_periodic = Circuit::new();
    non_periodic.add(Element::VoltageSource(VoltageSource::with_waveform(
        "V1",
        "in",
        "0",
        0.0,
        Waveform::Pwl(PwlWaveform::new(vec![(0.0, 0.0), (1.0e-3, 1.0)])),
    )));
    assert!(estimate_period(&non_periodic).is_none());

    let mut incommensurate = Circuit::new();
    incommensurate.add(Element::VoltageSource(VoltageSource::with_waveform(
        "V1",
        "in",
        "0",
        0.0,
        Waveform::Pulse(PulseWaveform::new(0.0, 1.0, 0.0, 0.0, 0.0, 0.25e-3, 1.0e-3)),
    )));
    incommensurate.add(Element::CurrentSource(CurrentSource::with_waveform(
        "I1",
        "out",
        "0",
        0.0,
        Waveform::Pulse(PulseWaveform::new(
            0.0, 1.0e-3, 0.0, 0.0, 0.0, 0.25e-3, 0.7e-3,
        )),
    )));
    assert!(estimate_period(&incommensurate).is_none());
}

#[test]
fn pss_residual_reports_one_period_node_closure() {
    let mut circuit = Circuit::new();
    circuit.add(Element::VoltageSource(VoltageSource::with_waveform(
        "V1",
        "in",
        "0",
        0.0,
        Waveform::Sin(SinWaveform::new(0.0, 1.0, 1_000.0)),
    )));
    circuit.add(Element::Resistor(Resistor::new("R1", "in", "0", 1_000.0)));

    let result = pss_residual(&circuit, 32).unwrap().unwrap();

    let _: PssResidualResult = result.clone();
    assert_close(result.period_seconds, 1.0e-3);
    assert_close(result.time_step_seconds, 1.0e-3 / 32.0);
    assert_close(*result.node_residuals.get("in").unwrap(), 0.0);
    assert_close(result.max_abs_residual, 0.0);
}

#[test]
fn pss_residual_requires_periodic_sources() {
    let mut circuit = Circuit::new();
    circuit.add(Element::VoltageSource(VoltageSource::with_waveform(
        "V1",
        "in",
        "0",
        0.0,
        Waveform::Pwl(PwlWaveform::new(vec![(0.0, 0.0), (1.0e-3, 1.0)])),
    )));

    assert!(pss_residual(&circuit, 32).unwrap().is_none());
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
fn transient_cccs_updates_from_sensed_branch_current() {
    let mut circuit = Circuit::new();
    circuit.add(Element::VoltageSource(VoltageSource::with_waveform(
        "Vin",
        "in",
        "0",
        0.0,
        Waveform::Pwl(PwlWaveform::new(vec![(0.0, 0.0), (0.25, 1.0), (0.5, 1.0)])),
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

    let points = transient(&circuit, 0.25, 0.5).unwrap();

    assert_close(points[0].branch_current("Vsense").unwrap(), 1.0e-3);
    assert_close(points[0].voltage("out").unwrap(), 2.0);
    assert_close(points[1].voltage("out").unwrap(), 2.0);
}

#[test]
fn transient_ccvs_updates_from_sensed_branch_current() {
    let mut circuit = Circuit::new();
    circuit.add(Element::VoltageSource(VoltageSource::with_waveform(
        "Vin",
        "in",
        "0",
        0.0,
        Waveform::Pwl(PwlWaveform::new(vec![(0.0, 0.0), (0.25, 1.0), (0.5, 1.0)])),
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

    let points = transient(&circuit, 0.25, 0.5).unwrap();

    assert_close(points[0].branch_current("Vsense").unwrap(), 1.0e-3);
    assert_close(points[0].voltage("out").unwrap(), 2.0);
    assert_close(points[1].voltage("out").unwrap(), 2.0);
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
