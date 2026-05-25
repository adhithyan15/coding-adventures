use spice_engine::{
    dc_op, distortion_from_fourier, distortion_from_transient, estimate_period, format_dc_table,
    format_transient_table, fourier, pole_zero_rc_highpass, pole_zero_rc_lowpass,
    pole_zero_rlc_bandpass, pole_zero_rlc_highpass, pole_zero_rlc_lowpass,
    pss_newton_candidate_with_tolerance, pss_newton_iteration_with_tolerance,
    pss_newton_solve_with_tolerance, pss_newton_update, pss_newton_update_with_tolerance,
    pss_residual, pss_residual_jacobian_with_tolerance, pss_residual_with_tolerance,
    pss_with_tolerance, transient, transient_adaptive, transient_with_method,
    AdaptiveTransientOptions, AdaptiveTransientResult, Capacitor, Cccs, Ccvs, Circuit,
    CurrentSource, DistortionHarmonic, DistortionPoint, DistortionResult, Element, ExpWaveform,
    Inductor, MutualInductor, PoleZeroEntry, PoleZeroEntryKind, PoleZeroResult,
    PssNewtonCandidateResult, PssNewtonIterationResult, PssNewtonSolveResult,
    PssNewtonUpdateResult, PssResidualJacobianResult, PssResidualResult, PssResult, PulseWaveform,
    PwlWaveform, Resistor, SinWaveform, SpiceError, TransientMethod, TransientPoint,
    TransmissionLine, VoltageSource, Waveform,
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
    assert_close(result.residual_tolerance, 1.0e-6);
    assert!(result.within_tolerance);
    assert_close(*result.node_residuals.get("in").unwrap(), 0.0);
    assert_close(*result.branch_residuals.get("I(V1)").unwrap(), 0.0);
    assert_eq!(result.residual_vector[0].kind, "node");
    assert_eq!(result.residual_vector[0].name, "in");
    assert_eq!(result.residual_vector[1].kind, "branch_current");
    assert_eq!(result.residual_vector[1].name, "I(V1)");
    assert_close(result.residual_vector[0].value, 0.0);
    assert_close(result.residual_vector[1].value, 0.0);
    assert_close(result.max_abs_branch_residual, 0.0);
    assert_close(result.max_abs_residual, 0.0);
    let expected_l2_norm = result
        .residual_vector
        .iter()
        .map(|entry| entry.value * entry.value)
        .sum::<f64>()
        .sqrt();
    assert_close(result.residual_l2_norm, expected_l2_norm);
    assert_close(
        result.residual_rms_norm,
        expected_l2_norm / (result.residual_vector.len() as f64).sqrt(),
    );
}

#[test]
fn pss_residual_jacobian_reports_reactive_initial_state_columns() {
    let mut circuit = Circuit::new();
    circuit.add(Element::VoltageSource(VoltageSource::with_waveform(
        "V1",
        "in",
        "0",
        0.0,
        Waveform::Sin(SinWaveform::new(0.0, 1.0, 1_000.0)),
    )));
    circuit.add(Element::Resistor(Resistor::new("R1", "in", "out", 1_000.0)));
    circuit.add(Element::Capacitor(Capacitor::with_initial_voltage(
        "C1", "out", "0", 1.0e-6, 0.1,
    )));

    let result = pss_residual_jacobian_with_tolerance(&circuit, 32, 1.0e-6, 1.0e-5)
        .unwrap()
        .unwrap();

    let _: PssResidualJacobianResult = result.clone();
    assert_close(result.perturbation, 1.0e-5);
    assert_eq!(result.state_vector.len(), 1);
    assert_eq!(result.state_vector[0].kind, "capacitor_voltage");
    assert_eq!(result.state_vector[0].name, "C1");
    assert_close(result.state_vector[0].value, 0.1);
    assert_eq!(result.columns[0].state, result.state_vector[0]);
    assert_eq!(result.jacobian.len(), result.residual.residual_vector.len());
    assert!(result.jacobian.iter().all(|row| row.len() == 1));
    let out_derivative = result.columns[0]
        .residual_derivatives
        .iter()
        .find(|entry| entry.name == "out")
        .unwrap()
        .value;
    let out_row = result
        .residual
        .residual_vector
        .iter()
        .position(|entry| entry.name == "out")
        .unwrap();
    assert_close(result.jacobian[out_row][0], out_derivative);
    assert!(out_derivative.abs() > 0.1);
    assert!(result.jacobian.iter().all(|row| row[0].is_finite()));
}

#[test]
fn pss_newton_update_reports_reactive_state_corrections() {
    let mut circuit = Circuit::new();
    circuit.add(Element::VoltageSource(VoltageSource::with_waveform(
        "V1",
        "in",
        "0",
        0.0,
        Waveform::Sin(SinWaveform::new(0.0, 1.0, 1_000.0)),
    )));
    circuit.add(Element::Resistor(Resistor::new("R1", "in", "out", 1_000.0)));
    circuit.add(Element::Capacitor(Capacitor::with_initial_voltage(
        "C1", "out", "0", 1.0e-6, 0.1,
    )));

    let result = pss_newton_update_with_tolerance(&circuit, 32, 1.0e-6, 1.0e-5)
        .unwrap()
        .unwrap();

    let _: PssNewtonUpdateResult = result.clone();
    assert_eq!(result.jacobian.state_vector[0].name, "C1");
    assert_eq!(result.state_updates[0].kind, "capacitor_voltage");
    assert_eq!(result.state_updates[0].name, "C1");
    assert_close(
        result.next_state_vector[0].value,
        result.jacobian.state_vector[0].value + result.state_updates[0].value,
    );
    assert_close(result.update_l2_norm, result.state_updates[0].value.abs());
    assert!(result.state_updates[0].value.is_finite());
}

#[test]
fn pss_newton_candidate_applies_reactive_state_update() {
    let mut circuit = Circuit::new();
    circuit.add(Element::VoltageSource(VoltageSource::with_waveform(
        "V1",
        "in",
        "0",
        0.0,
        Waveform::Sin(SinWaveform::new(0.0, 1.0, 1_000.0)),
    )));
    circuit.add(Element::Resistor(Resistor::new("R1", "in", "out", 1_000.0)));
    circuit.add(Element::Capacitor(Capacitor::with_initial_voltage(
        "C1", "out", "0", 1.0e-6, 0.1,
    )));

    let result = pss_newton_candidate_with_tolerance(&circuit, 32, 1.0e-6, 1.0e-5)
        .unwrap()
        .unwrap();

    let _: PssNewtonCandidateResult = result.clone();
    assert_eq!(result.update.next_state_vector[0].name, "C1");
    assert_eq!(
        result.candidate_state_vector,
        result.update.next_state_vector
    );
    let candidate_cap = result
        .candidate_circuit
        .elements()
        .iter()
        .find_map(|element| match element {
            Element::Capacitor(capacitor) if capacitor.name == "C1" => Some(capacitor),
            _ => None,
        })
        .unwrap();
    let original_cap = circuit
        .elements()
        .iter()
        .find_map(|element| match element {
            Element::Capacitor(capacitor) if capacitor.name == "C1" => Some(capacitor),
            _ => None,
        })
        .unwrap();
    assert_close(original_cap.initial_voltage, 0.1);
    assert_close(
        candidate_cap.initial_voltage,
        result.update.next_state_vector[0].value,
    );
    assert_close(
        result.candidate_residual.period_seconds,
        result.update.jacobian.residual.period_seconds,
    );
    assert!(result.candidate_residual.residual_l2_norm.is_finite());
}

#[test]
fn pss_newton_iteration_accepts_improving_candidate() {
    let mut circuit = Circuit::new();
    circuit.add(Element::VoltageSource(VoltageSource::with_waveform(
        "V1",
        "in",
        "0",
        0.0,
        Waveform::Sin(SinWaveform::new(0.0, 1.0, 1_000.0)),
    )));
    circuit.add(Element::Resistor(Resistor::new("R1", "in", "out", 1_000.0)));
    circuit.add(Element::Capacitor(Capacitor::with_initial_voltage(
        "C1", "out", "0", 1.0e-6, 0.1,
    )));

    let result = pss_newton_iteration_with_tolerance(&circuit, 32, 1.0e-6, 1.0e-5)
        .unwrap()
        .unwrap();

    let _: PssNewtonIterationResult = result.clone();
    let base_residual = &result.candidate.update.jacobian.residual;
    let candidate_residual = &result.candidate.candidate_residual;
    assert!(result.accepted);
    assert_eq!(result.next_circuit, result.candidate.candidate_circuit);
    assert_eq!(
        result.next_state_vector,
        result.candidate.candidate_state_vector
    );
    assert_eq!(result.next_residual, *candidate_residual);
    assert_eq!(result.converged, candidate_residual.within_tolerance);
    assert!(candidate_residual.residual_l2_norm < base_residual.residual_l2_norm);
    assert_close(
        result.residual_l2_reduction,
        base_residual.residual_l2_norm - candidate_residual.residual_l2_norm,
    );
    assert_close(
        result.residual_l2_ratio,
        candidate_residual.residual_l2_norm / base_residual.residual_l2_norm,
    );
}

#[test]
fn pss_newton_solve_runs_accepted_iterations_to_convergence() {
    let mut circuit = Circuit::new();
    circuit.add(Element::VoltageSource(VoltageSource::with_waveform(
        "V1",
        "in",
        "0",
        0.0,
        Waveform::Sin(SinWaveform::new(0.0, 1.0, 1_000.0)),
    )));
    circuit.add(Element::Resistor(Resistor::new("R1", "in", "out", 1_000.0)));
    circuit.add(Element::Capacitor(Capacitor::with_initial_voltage(
        "C1", "out", "0", 1.0e-6, 0.1,
    )));

    let result = pss_newton_solve_with_tolerance(&circuit, 32, 1.0e-3, 1.0e-5, 4)
        .unwrap()
        .unwrap();

    let _: PssNewtonSolveResult = result.clone();
    assert_eq!(result.iteration_count, result.iterations.len());
    assert!((1..=4).contains(&result.iteration_count));
    assert!(result.iterations.iter().all(|iteration| iteration.accepted));
    assert!(result.converged);
    assert!(result.final_residual.within_tolerance);
    assert!(
        result.final_residual.residual_l2_norm
            < result.iterations[0]
                .candidate
                .update
                .jacobian
                .residual
                .residual_l2_norm
    );
    let last_iteration = result.iterations.last().unwrap();
    assert_eq!(result.final_circuit, last_iteration.next_circuit);
    assert_eq!(result.final_state_vector, last_iteration.next_state_vector);
}

#[test]
fn pss_returns_solved_steady_state_period() {
    let mut circuit = Circuit::new();
    circuit.add(Element::VoltageSource(VoltageSource::with_waveform(
        "V1",
        "in",
        "0",
        0.0,
        Waveform::Sin(SinWaveform::new(0.0, 1.0, 1_000.0)),
    )));
    circuit.add(Element::Resistor(Resistor::new("R1", "in", "out", 1_000.0)));
    circuit.add(Element::Capacitor(Capacitor::with_initial_voltage(
        "C1", "out", "0", 1.0e-6, 0.1,
    )));

    let result = pss_with_tolerance(&circuit, 32, 1.0e-3, 1.0e-5, 4)
        .unwrap()
        .unwrap();

    let _: PssResult = result.clone();
    assert!(result.converged);
    assert!(result.solve.converged);
    assert_eq!(
        result.period_seconds,
        result.solve.final_residual.period_seconds
    );
    assert_eq!(
        result.time_step_seconds,
        result.solve.final_residual.time_step_seconds
    );
    assert!(!result.steady_state.is_empty());
    assert_close(
        result.steady_state.last().unwrap().time,
        result.period_seconds,
    );
    let residual = pss_residual_with_tolerance(&result.solve.final_circuit, 32, 1.0e-3)
        .unwrap()
        .unwrap();
    assert_close(
        residual.residual_l2_norm,
        result.solve.final_residual.residual_l2_norm,
    );
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
fn pss_residual_rejects_negative_residual_tolerance() {
    let mut circuit = Circuit::new();
    circuit.add(Element::VoltageSource(VoltageSource::with_waveform(
        "V1",
        "in",
        "0",
        0.0,
        Waveform::Sin(SinWaveform::new(0.0, 1.0, 1_000.0)),
    )));

    assert!(matches!(
        pss_residual_with_tolerance(&circuit, 32, -1.0),
        Err(SpiceError::InvalidElement { .. })
    ));
}

#[test]
fn pss_residual_jacobian_rejects_non_positive_perturbation() {
    let mut circuit = Circuit::new();
    circuit.add(Element::VoltageSource(VoltageSource::with_waveform(
        "V1",
        "in",
        "0",
        0.0,
        Waveform::Sin(SinWaveform::new(0.0, 1.0, 1_000.0)),
    )));

    assert!(matches!(
        pss_residual_jacobian_with_tolerance(&circuit, 32, 1.0e-6, 0.0),
        Err(SpiceError::InvalidElement { .. })
    ));
}

#[test]
fn pss_newton_update_without_reactive_state_returns_empty_update() {
    let mut circuit = Circuit::new();
    circuit.add(Element::VoltageSource(VoltageSource::with_waveform(
        "V1",
        "in",
        "0",
        0.0,
        Waveform::Sin(SinWaveform::new(0.0, 1.0, 1_000.0)),
    )));
    circuit.add(Element::Resistor(Resistor::new("R1", "in", "0", 1_000.0)));

    let result = pss_newton_update(&circuit, 32).unwrap().unwrap();

    assert!(result.state_updates.is_empty());
    assert!(result.next_state_vector.is_empty());
    assert_close(result.update_l2_norm, 0.0);
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
fn transient_gear2_rc_charging_bootstraps_then_uses_bdf2() {
    let mut circuit = Circuit::new();
    circuit.add(Element::VoltageSource(VoltageSource::new(
        "V1", "vin", "0", 1.0,
    )));
    circuit.add(Element::Resistor(Resistor::new(
        "R1", "vin", "out", 1_000.0,
    )));
    circuit.add(Element::Capacitor(Capacitor::new("C1", "out", "0", 1.0e-6)));

    let points = transient_with_method(&circuit, 1.0e-3, 3.0e-3, TransientMethod::Gear2).unwrap();

    assert_eq!(points.len(), 3);
    assert_close(points[0].voltage("out").unwrap(), 0.5);
    assert_close(points[1].voltage("out").unwrap(), 0.8);
    assert_close(points[2].voltage("out").unwrap(), 0.94);
}

#[test]
fn transient_trap_rc_charging_uses_trapezoidal_companion() {
    let mut circuit = Circuit::new();
    circuit.add(Element::VoltageSource(VoltageSource::new(
        "V1", "vin", "0", 1.0,
    )));
    circuit.add(Element::Resistor(Resistor::new(
        "R1", "vin", "out", 1_000.0,
    )));
    circuit.add(Element::Capacitor(Capacitor::new("C1", "out", "0", 1.0e-6)));

    let points = transient_with_method(&circuit, 1.0e-3, 3.0e-3, TransientMethod::Trap).unwrap();

    assert_eq!(points.len(), 3);
    assert_close(points[0].voltage("out").unwrap(), 1.0 / 3.0);
    assert_close(points[1].voltage("out").unwrap(), 7.0 / 9.0);
    assert_close(points[2].voltage("out").unwrap(), 25.0 / 27.0);
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
fn transient_gear2_rl_current_buildup_bootstraps_then_uses_bdf2() {
    let mut circuit = Circuit::new();
    circuit.add(Element::VoltageSource(VoltageSource::new(
        "V1", "vin", "0", 1.0,
    )));
    circuit.add(Element::Resistor(Resistor::new(
        "R1", "vin", "out", 1_000.0,
    )));
    circuit.add(Element::Inductor(Inductor::new("L1", "out", "0", 1.0)));

    let points = transient_with_method(&circuit, 1.0e-3, 3.0e-3, TransientMethod::Gear2).unwrap();

    assert_eq!(points.len(), 3);
    assert_close(points[0].branch_current("L1").unwrap(), 0.5e-3);
    assert_close(points[1].branch_current("L1").unwrap(), 0.8e-3);
    assert_close(points[2].branch_current("L1").unwrap(), 0.94e-3);
}

#[test]
fn transient_trap_rl_current_buildup_uses_trapezoidal_companion() {
    let mut circuit = Circuit::new();
    circuit.add(Element::VoltageSource(VoltageSource::new(
        "V1", "vin", "0", 1.0,
    )));
    circuit.add(Element::Resistor(Resistor::new(
        "R1", "vin", "out", 1_000.0,
    )));
    circuit.add(Element::Inductor(Inductor::new("L1", "out", "0", 1.0)));

    let points = transient_with_method(&circuit, 1.0e-3, 3.0e-3, TransientMethod::Trap).unwrap();

    assert_eq!(points.len(), 3);
    assert_close(points[0].branch_current("L1").unwrap(), 1.0e-3 / 3.0);
    assert_close(points[1].branch_current("L1").unwrap(), 7.0e-3 / 9.0);
    assert_close(points[2].branch_current("L1").unwrap(), 25.0e-3 / 27.0);
}

#[test]
fn transient_gear2_damps_coarse_lc_oscillator_more_than_trap() {
    let mut circuit = Circuit::new();
    circuit.add(Element::Capacitor(Capacitor::with_initial_voltage(
        "C1", "tank", "0", 1.0, 1.0,
    )));
    circuit.add(Element::Inductor(Inductor::new("L1", "tank", "0", 1.0)));

    let trap_points = transient_with_method(&circuit, 1.0, 10.0, TransientMethod::Trap).unwrap();
    let gear_points = transient_with_method(&circuit, 1.0, 10.0, TransientMethod::Gear2).unwrap();
    let trap_tail = trap_points
        .iter()
        .rev()
        .take(4)
        .map(|point| point.voltage("tank").unwrap().abs())
        .fold(0.0_f64, f64::max);
    let gear_tail = gear_points
        .iter()
        .rev()
        .take(4)
        .map(|point| point.voltage("tank").unwrap().abs())
        .fold(0.0_f64, f64::max);

    assert!(gear_tail < trap_tail * 0.75);
}

#[test]
fn adaptive_transient_matches_fixed_trap_when_bounds_pin_step() {
    let mut circuit = Circuit::new();
    circuit.add(Element::VoltageSource(VoltageSource::new(
        "V1", "vin", "0", 1.0,
    )));
    circuit.add(Element::Resistor(Resistor::new(
        "R1", "vin", "out", 1_000.0,
    )));
    circuit.add(Element::Capacitor(Capacitor::new("C1", "out", "0", 1.0e-6)));

    let fixed = transient_with_method(&circuit, 1.0e-3, 3.0e-3, TransientMethod::Trap).unwrap();
    let adaptive = transient_adaptive(
        &circuit,
        1.0e-3,
        3.0e-3,
        AdaptiveTransientOptions {
            method: TransientMethod::Trap,
            tolerance: 1.0,
            min_step: Some(1.0e-3),
            max_step: Some(1.0e-3),
        },
    )
    .unwrap();

    let _: AdaptiveTransientResult = adaptive.clone();
    assert!(adaptive.converged);
    assert_eq!(adaptive.steps_rejected, 0);
    assert_eq!(adaptive.points.len(), fixed.len());
    assert_close(adaptive.points[0].time, fixed[0].time);
    assert_close(
        adaptive.points.last().unwrap().voltage("out").unwrap(),
        fixed.last().unwrap().voltage("out").unwrap(),
    );
}

#[test]
fn adaptive_transient_uses_variable_steps_with_gear2_after_bootstrap() {
    let mut circuit = Circuit::new();
    circuit.add(Element::VoltageSource(VoltageSource::new(
        "V1", "vin", "0", 1.0,
    )));
    circuit.add(Element::Resistor(Resistor::new(
        "R1", "vin", "out", 1_000.0,
    )));
    circuit.add(Element::Capacitor(Capacitor::new("C1", "out", "0", 1.0e-6)));

    let fixed = transient_with_method(&circuit, 1.0e-4, 1.0e-3, TransientMethod::Gear2).unwrap();
    let adaptive = transient_adaptive(
        &circuit,
        1.0e-4,
        1.0e-3,
        AdaptiveTransientOptions {
            method: TransientMethod::Gear2,
            tolerance: 1.0,
            min_step: None,
            max_step: Some(5.0e-4),
        },
    )
    .unwrap();

    assert_eq!(adaptive.method, TransientMethod::Gear2);
    assert!(adaptive.converged);
    assert_eq!(adaptive.steps_rejected, 0);
    assert!(adaptive.points.len() < fixed.len());
    assert_close(adaptive.points.last().unwrap().time, 1.0e-3);
    assert!(adaptive.points.last().unwrap().voltage("out").unwrap() > 0.0);
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
fn transient_mutual_inductor_couples_secondary_voltage() {
    let mut circuit = Circuit::new();
    circuit.add(Element::CurrentSource(CurrentSource::new(
        "Istep", "0", "pri", 1.0,
    )));
    circuit.add(Element::Inductor(Inductor::new("Lpri", "pri", "0", 1.0)));
    circuit.add(Element::Inductor(Inductor::new("Lsec", "sec", "0", 1.0)));
    circuit.add(Element::MutualInductor(MutualInductor::new(
        "K1", "Lpri", "Lsec", 0.5,
    )));
    circuit.add(Element::Resistor(Resistor::new("Rload", "sec", "0", 10.0)));

    let points = transient(&circuit, 0.1, 0.1).unwrap();

    assert_close(points[0].voltage("pri").unwrap(), 8.75);
    assert_close(points[0].voltage("sec").unwrap(), 2.5);
    assert_close(points[0].branch_current("Lsec").unwrap(), -0.25);
}

#[test]
fn transient_transmission_line_delays_matched_step() {
    let delay = 1.0e-9;
    let mut circuit = Circuit::new();
    circuit.add(Element::VoltageSource(VoltageSource::new(
        "VIN", "in", "0", 1.0,
    )));
    circuit.add(Element::TransmissionLine(TransmissionLine::new(
        "T1", "in", "0", "out", "0", 50.0, delay,
    )));
    circuit.add(Element::Resistor(Resistor::new("RL", "out", "0", 50.0)));

    let points = transient(&circuit, delay / 2.0, 2.0 * delay).unwrap();

    assert_close(points[0].voltage("out").unwrap_or(0.0), 0.0);
    assert_close(points[1].voltage("out").unwrap(), 1.0);
    assert_close(points[1].branch_current("T1:2").unwrap(), -0.02);
}

#[test]
fn transient_transmission_line_rejects_invalid_parameters() {
    let mut circuit = Circuit::new();
    circuit.add(Element::VoltageSource(VoltageSource::new(
        "VIN", "in", "0", 1.0,
    )));
    circuit.add(Element::TransmissionLine(TransmissionLine::new(
        "Tbad", "in", "0", "out", "0", 50.0, 0.0,
    )));
    circuit.add(Element::Resistor(Resistor::new("RL", "out", "0", 50.0)));

    assert!(matches!(
        transient(&circuit, 1.0e-9, 1.0e-9),
        Err(SpiceError::InvalidElement { name, .. }) if name == "Tbad"
    ));
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
fn fourier_extracts_transient_sinusoid_components() {
    let freq = 1_000.0;
    let amp = 2.0;
    let offset = 0.25;
    let period = 1.0 / freq;
    let mut circuit = Circuit::new();
    circuit.add(Element::VoltageSource(VoltageSource::with_waveform(
        "Vin",
        "in",
        "0",
        0.0,
        Waveform::Sin(SinWaveform::new(offset, amp, freq)),
    )));

    let points = transient(&circuit, period / 64.0, 2.0 * period).unwrap();
    let analysis = fourier(&points, freq, &["V(in)"], 5).unwrap();
    let probe = &analysis.probes[0];
    let fundamental = &probe.harmonics[0];

    assert!((analysis.start_time - period).abs() < 1.0e-12);
    assert!((probe.dc - offset).abs() < 2.0e-3);
    assert_close(fundamental.frequency_hz, freq);
    assert!((fundamental.magnitude - amp).abs() < 2.0e-3);
    assert!((fundamental.sine - amp).abs() < 2.0e-3);
    assert!(fundamental.cosine.abs() < 2.0e-3);
    assert!(probe.total_harmonic_distortion < 2.0e-3);
}

#[test]
fn pole_zero_result_shape_supports_simple_rc_pole_fixture() {
    let resistance = 1_000.0;
    let capacitance = 1.0e-6;
    let pole_rad_per_second = -1.0 / (resistance * capacitance);
    let result = PoleZeroResult {
        input_source: "Vin".to_string(),
        output_node: "out".to_string(),
        entries: vec![PoleZeroEntry {
            kind: PoleZeroEntryKind::Pole,
            real: pole_rad_per_second,
            imaginary: 0.0,
            frequency_hz: pole_rad_per_second.abs() / (2.0 * std::f64::consts::PI),
            damping: 1.0,
        }],
    };

    assert_eq!(result.entries[0].kind, PoleZeroEntryKind::Pole);
    assert_close(
        result.entries[0].frequency_hz,
        1.0 / (2.0 * std::f64::consts::PI * resistance * capacitance),
    );
}

#[test]
fn pole_zero_rc_lowpass_returns_simple_rc_pole() {
    let mut circuit = Circuit::new();
    circuit.add(Element::VoltageSource(VoltageSource::new(
        "Vin", "in", "0", 1.0,
    )));
    circuit.add(Element::Resistor(Resistor::new("R1", "in", "out", 1_000.0)));
    circuit.add(Element::Capacitor(Capacitor::new("C1", "out", "0", 1.0e-6)));

    let result = pole_zero_rc_lowpass(&circuit, "Vin", "out").unwrap();

    assert_eq!(
        result,
        PoleZeroResult {
            input_source: "Vin".to_string(),
            output_node: "out".to_string(),
            entries: vec![PoleZeroEntry {
                kind: PoleZeroEntryKind::Pole,
                real: -1.0e3,
                imaginary: 0.0,
                frequency_hz: 1.0e3 / (2.0 * std::f64::consts::PI),
                damping: 1.0,
            }],
        }
    );
}

#[test]
fn pole_zero_rc_highpass_returns_origin_zero_and_simple_rc_pole() {
    let mut circuit = Circuit::new();
    circuit.add(Element::VoltageSource(VoltageSource::new(
        "Vin", "in", "0", 1.0,
    )));
    circuit.add(Element::Capacitor(Capacitor::new(
        "C1", "in", "out", 1.0e-6,
    )));
    circuit.add(Element::Resistor(Resistor::new("R1", "out", "0", 1_000.0)));

    let result = pole_zero_rc_highpass(&circuit, "Vin", "out").unwrap();

    assert_eq!(
        result,
        PoleZeroResult {
            input_source: "Vin".to_string(),
            output_node: "out".to_string(),
            entries: vec![
                PoleZeroEntry {
                    kind: PoleZeroEntryKind::Zero,
                    real: 0.0,
                    imaginary: 0.0,
                    frequency_hz: 0.0,
                    damping: 1.0,
                },
                PoleZeroEntry {
                    kind: PoleZeroEntryKind::Pole,
                    real: -1.0e3,
                    imaginary: 0.0,
                    frequency_hz: 1.0e3 / (2.0 * std::f64::consts::PI),
                    damping: 1.0,
                },
            ],
        }
    );
}

#[test]
fn pole_zero_rlc_lowpass_returns_complex_conjugate_poles() {
    let mut circuit = Circuit::new();
    circuit.add(Element::VoltageSource(VoltageSource::new(
        "Vin", "in", "0", 1.0,
    )));
    circuit.add(Element::Resistor(Resistor::new("R1", "in", "mid", 10.0)));
    circuit.add(Element::Inductor(Inductor::new("L1", "mid", "out", 1.0e-3)));
    circuit.add(Element::Capacitor(Capacitor::new("C1", "out", "0", 1.0e-6)));

    let result = pole_zero_rlc_lowpass(&circuit, "Vin", "out").unwrap();

    let alpha = 10.0 / (2.0 * 1.0e-3);
    let omega0 = 1.0 / f64::sqrt(1.0e-3 * 1.0e-6);
    let imaginary = f64::sqrt(omega0 * omega0 - alpha * alpha);
    assert_eq!(
        result,
        PoleZeroResult {
            input_source: "Vin".to_string(),
            output_node: "out".to_string(),
            entries: vec![
                PoleZeroEntry {
                    kind: PoleZeroEntryKind::Pole,
                    real: -alpha,
                    imaginary,
                    frequency_hz: omega0 / (2.0 * std::f64::consts::PI),
                    damping: alpha / omega0,
                },
                PoleZeroEntry {
                    kind: PoleZeroEntryKind::Pole,
                    real: -alpha,
                    imaginary: -imaginary,
                    frequency_hz: omega0 / (2.0 * std::f64::consts::PI),
                    damping: alpha / omega0,
                },
            ],
        }
    );
}

#[test]
fn pole_zero_rlc_highpass_returns_origin_zeros_and_complex_conjugate_poles() {
    let mut circuit = Circuit::new();
    circuit.add(Element::VoltageSource(VoltageSource::new(
        "Vin", "in", "0", 1.0,
    )));
    circuit.add(Element::Resistor(Resistor::new("R1", "in", "mid", 10.0)));
    circuit.add(Element::Capacitor(Capacitor::new(
        "C1", "mid", "out", 1.0e-6,
    )));
    circuit.add(Element::Inductor(Inductor::new("L1", "out", "0", 1.0e-3)));

    let result = pole_zero_rlc_highpass(&circuit, "Vin", "out").unwrap();

    let alpha = 10.0 / (2.0 * 1.0e-3);
    let omega0 = 1.0 / f64::sqrt(1.0e-3 * 1.0e-6);
    let imaginary = f64::sqrt(omega0 * omega0 - alpha * alpha);
    assert_eq!(
        result,
        PoleZeroResult {
            input_source: "Vin".to_string(),
            output_node: "out".to_string(),
            entries: vec![
                PoleZeroEntry {
                    kind: PoleZeroEntryKind::Zero,
                    real: 0.0,
                    imaginary: 0.0,
                    frequency_hz: 0.0,
                    damping: 1.0,
                },
                PoleZeroEntry {
                    kind: PoleZeroEntryKind::Zero,
                    real: 0.0,
                    imaginary: 0.0,
                    frequency_hz: 0.0,
                    damping: 1.0,
                },
                PoleZeroEntry {
                    kind: PoleZeroEntryKind::Pole,
                    real: -alpha,
                    imaginary,
                    frequency_hz: omega0 / (2.0 * std::f64::consts::PI),
                    damping: alpha / omega0,
                },
                PoleZeroEntry {
                    kind: PoleZeroEntryKind::Pole,
                    real: -alpha,
                    imaginary: -imaginary,
                    frequency_hz: omega0 / (2.0 * std::f64::consts::PI),
                    damping: alpha / omega0,
                },
            ],
        }
    );
}

#[test]
fn pole_zero_rlc_bandpass_returns_origin_zero_and_complex_conjugate_poles() {
    let mut circuit = Circuit::new();
    circuit.add(Element::VoltageSource(VoltageSource::new(
        "Vin", "in", "0", 1.0,
    )));
    circuit.add(Element::Inductor(Inductor::new("L1", "in", "mid", 1.0e-3)));
    circuit.add(Element::Capacitor(Capacitor::new(
        "C1", "mid", "out", 1.0e-6,
    )));
    circuit.add(Element::Resistor(Resistor::new("R1", "out", "0", 10.0)));

    let result = pole_zero_rlc_bandpass(&circuit, "Vin", "out").unwrap();

    let alpha = 10.0 / (2.0 * 1.0e-3);
    let omega0 = 1.0 / f64::sqrt(1.0e-3 * 1.0e-6);
    let imaginary = f64::sqrt(omega0 * omega0 - alpha * alpha);
    assert_eq!(
        result,
        PoleZeroResult {
            input_source: "Vin".to_string(),
            output_node: "out".to_string(),
            entries: vec![
                PoleZeroEntry {
                    kind: PoleZeroEntryKind::Zero,
                    real: 0.0,
                    imaginary: 0.0,
                    frequency_hz: 0.0,
                    damping: 1.0,
                },
                PoleZeroEntry {
                    kind: PoleZeroEntryKind::Pole,
                    real: -alpha,
                    imaginary,
                    frequency_hz: omega0 / (2.0 * std::f64::consts::PI),
                    damping: alpha / omega0,
                },
                PoleZeroEntry {
                    kind: PoleZeroEntryKind::Pole,
                    real: -alpha,
                    imaginary: -imaginary,
                    frequency_hz: omega0 / (2.0 * std::f64::consts::PI),
                    damping: alpha / omega0,
                },
            ],
        }
    );
}

#[test]
fn distortion_result_shape_supports_nonlinear_device_smoke_fixture() {
    let result = DistortionResult {
        input_source: "Vin".to_string(),
        output_probe: "V(out)".to_string(),
        points: vec![DistortionPoint {
            frequency_hz: 1.0e3,
            fundamental_magnitude: 1.0,
            harmonics: vec![DistortionHarmonic {
                harmonic: 2,
                frequency_hz: 2.0e3,
                magnitude: 0.025,
                phase_degrees: -12.0,
            }],
            total_harmonic_distortion: 0.025,
        }],
    };

    assert_eq!(result.points[0].harmonics[0].harmonic, 2);
    assert_close(result.points[0].total_harmonic_distortion, 0.025);
}

#[test]
fn distortion_from_fourier_projects_probe_harmonics() {
    let fourier_result = spice_engine::FourierResult {
        fundamental_frequency_hz: 1.0e3,
        start_time: 0.0,
        end_time: 1.0e-3,
        probes: vec![spice_engine::FourierProbeResult {
            probe: "V(out)".to_string(),
            dc: 0.0,
            harmonics: vec![
                spice_engine::FourierHarmonic {
                    harmonic: 1,
                    frequency_hz: 1.0e3,
                    cosine: 0.0,
                    sine: 1.0,
                    magnitude: 1.0,
                    phase_degrees: 0.0,
                },
                spice_engine::FourierHarmonic {
                    harmonic: 2,
                    frequency_hz: 2.0e3,
                    cosine: 0.0,
                    sine: 0.025,
                    magnitude: 0.025,
                    phase_degrees: -12.0,
                },
            ],
            total_harmonic_distortion: 0.025,
        }],
    };

    let result = distortion_from_fourier(&fourier_result, "Vin", "V(out)").unwrap();

    assert_eq!(
        result,
        DistortionResult {
            input_source: "Vin".to_string(),
            output_probe: "V(out)".to_string(),
            points: vec![DistortionPoint {
                frequency_hz: 1.0e3,
                fundamental_magnitude: 1.0,
                harmonics: vec![DistortionHarmonic {
                    harmonic: 2,
                    frequency_hz: 2.0e3,
                    magnitude: 0.025,
                    phase_degrees: -12.0,
                }],
                total_harmonic_distortion: 0.025,
            }],
        }
    );
}

#[test]
fn distortion_from_transient_extracts_harmonic_content() {
    let freq = 1.0e3;
    let period = 1.0 / freq;
    let points = (0..=128)
        .map(|index| {
            let time = index as f64 * period / 64.0;
            let value = (2.0 * std::f64::consts::PI * freq * time).sin()
                + 0.1 * (4.0 * std::f64::consts::PI * freq * time).sin();
            TransientPoint {
                time,
                node_voltages: std::collections::BTreeMap::from([("out".to_string(), value)]),
                branch_currents: std::collections::BTreeMap::new(),
            }
        })
        .collect::<Vec<_>>();

    let result = distortion_from_transient(&points, freq, "Vin", "V(out)", 3).unwrap();

    assert_eq!(result.input_source, "Vin");
    assert_eq!(result.output_probe, "V(out)");
    let point = &result.points[0];
    assert_close(point.frequency_hz, freq);
    assert!((point.fundamental_magnitude - 1.0).abs() < 2.0e-3);
    assert_eq!(point.harmonics[0].harmonic, 2);
    assert!((point.harmonics[0].magnitude - 0.1).abs() < 2.0e-3);
    assert!((point.total_harmonic_distortion - 0.1).abs() < 2.0e-3);
}

#[test]
fn text_output_tables_are_stable_for_dc_and_transient_results() {
    let mut circuit = Circuit::new();
    circuit.add(Element::VoltageSource(VoltageSource::new(
        "V1", "vin", "0", 10.0,
    )));
    circuit.add(Element::Resistor(Resistor::new(
        "R1", "vin", "mid", 1_000.0,
    )));
    circuit.add(Element::Resistor(Resistor::new("R2", "mid", "0", 1_000.0)));
    let dc_result = dc_op(&circuit).unwrap();

    assert_eq!(
        format_dc_table(&dc_result, &[]).unwrap(),
        "Index\tV(mid)\tV(vin)\tI(V1)\n0\t5.000000e+00\t1.000000e+01\t-5.000000e-03\n"
    );
    assert_eq!(
        format_dc_table(&dc_result, &["V(vin, mid)", "I(V1)"]).unwrap(),
        "Index\tV(vin, mid)\tI(V1)\n0\t5.000000e+00\t-5.000000e-03\n"
    );

    let points = transient(&circuit, 1.0e-3, 2.0e-3).unwrap();
    assert_eq!(
        format_transient_table(&points, &["V(vin)", "V(mid)", "I(V1)"]).unwrap(),
        "Index\tTime\tV(vin)\tV(mid)\tI(V1)\n0\t1.000000e-03\t1.000000e+01\t5.000000e+00\t-5.000000e-03\n1\t2.000000e-03\t1.000000e+01\t5.000000e+00\t-5.000000e-03\n"
    );
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
