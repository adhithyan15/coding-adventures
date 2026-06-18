use std::collections::BTreeMap;

use spice_engine::{
    dc_op, digital_event_streams_to_voltage_sources, distortion_from_fourier,
    distortion_from_transient, distortion_from_transient_corners, estimate_period,
    format_adaptive_digital_event_stream_table, format_adaptive_transient_table,
    format_corner_adaptive_digital_event_stream_table, format_corner_adaptive_transient_table,
    format_corner_digital_event_stream_table, format_corner_distortion_table,
    format_corner_fourier_table, format_corner_pole_zero_table, format_corner_pss_table,
    format_corner_transient_table, format_dc_table, format_deck_transient_table,
    format_digital_bridge_schedule_table, format_digital_event_stream_table,
    format_digital_event_table, format_distortion_table, format_fourier_table,
    format_measurement_table, format_pole_zero_table, format_pss_table, format_transient_table,
    fourier, fourier_corners, fourier_transient_deck, measure_transient_deck,
    measure_transient_delay_between_probes, measure_transient_find_at_probe,
    measure_transient_probe, measure_transient_when_probe, measure_transient_when_probe_counted,
    pole_zero_rc_highpass, pole_zero_rc_lowpass, pole_zero_rlc_bandpass, pole_zero_rlc_highpass,
    pole_zero_rlc_lowpass, pole_zero_rlc_notch, pss_corners_with_tolerance,
    pss_newton_candidate_with_tolerance, pss_newton_iteration_with_tolerance,
    pss_newton_solve_with_tolerance, pss_newton_update, pss_newton_update_with_tolerance,
    pss_residual, pss_residual_jacobian_with_tolerance, pss_residual_with_tolerance,
    pss_with_tolerance, run_deck_analysis, sample_transient_probe_as_digital_events,
    sample_transient_probes_as_digital_event_streams, transient, transient_adaptive,
    transient_adaptive_corners, transient_adaptive_with_digital_event_streams,
    transient_adaptive_with_digital_event_streams_corners, transient_corners,
    transient_with_digital_event_streams, transient_with_digital_event_streams_corners,
    transient_with_method, AdaptiveTransientOptions, AdaptiveTransientResult, Capacitor, Cccs,
    Ccvs, Circuit, CornerDistortionPoint, CornerDistortionResult, CornerOverride, CornerSpec,
    CurrentSource, DeckAnalysisExecutionResult, DigitalBridgeSchedule, DigitalEvent,
    DigitalEventStream, DigitalLogicLevels, DigitalState, DigitalThresholds, DistortionHarmonic,
    DistortionPoint, DistortionResult, Element, ExpWaveform, FourierHarmonic, FourierProbeResult,
    FourierResult, Inductor, Jfet, JfetPolarity, MutualInductor, PoleZeroEntry, PoleZeroEntryKind,
    PoleZeroResult, PoleZeroTopology, PssNewtonCandidateResult, PssNewtonIterationResult,
    PssNewtonSolveResult, PssNewtonUpdateResult, PssResidualJacobianResult, PssResidualResult,
    PssResult, PulseWaveform, PwlWaveform, Resistor, SinWaveform, SpiceError, TransientMethod,
    TransientPoint, TransmissionLine, VoltageSource, Waveform,
};

fn assert_close(actual: f64, expected: f64) {
    assert!(
        (actual - expected).abs() < 1.0e-9,
        "expected {expected}, got {actual}"
    );
}

#[test]
fn transient_jfet_source_follower_charges_output_capacitor() {
    let mut circuit = Circuit::new();
    circuit.add(Element::VoltageSource(VoltageSource::new(
        "Vdd", "vdd", "0", 10.0,
    )));
    circuit.add(Element::VoltageSource(VoltageSource::with_waveform(
        "Vg",
        "gate",
        "0",
        0.0,
        Waveform::Pwl(PwlWaveform::new(vec![
            (0.0, 0.0),
            (1.0e-6, 1.0),
            (2.0e-6, 1.0),
        ])),
    )));
    circuit.add(Element::Jfet(Jfet::with_model(
        "J1",
        "vdd",
        "gate",
        "out",
        JfetPolarity::Njf,
        1.0e-3,
        -2.0,
        0.0,
    )));
    circuit.add(Element::Resistor(Resistor::new("Rs", "out", "0", 1_000.0)));
    circuit.add(Element::Capacitor(Capacitor::new(
        "Cout", "out", "0", 1.0e-9,
    )));

    let points = transient(&circuit, 1.0e-7, 2.0e-6).unwrap();

    let initial_out = points[0].voltage("out").unwrap();
    let final_out = points.last().unwrap().voltage("out").unwrap();
    assert!(
        final_out > initial_out + 1.0,
        "expected JFET output to charge from {initial_out}, got {final_out}"
    );
    assert!(
        final_out > 1.5,
        "expected JFET output to charge, got {final_out}"
    );
    assert!(
        final_out < 2.0,
        "expected source below gate plus threshold, got {final_out}"
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
fn pss_corners_runs_analysis_per_corner() {
    let mut circuit = Circuit::new();
    circuit.add(Element::VoltageSource(VoltageSource::with_waveform(
        "V1",
        "in",
        "0",
        0.0,
        Waveform::Sin(SinWaveform::new(0.0, 1.0, 1_000.0)),
    )));
    circuit.add(Element::Resistor(Resistor::new("R1", "in", "0", 1_000.0)));

    let result = pss_corners_with_tolerance(
        &circuit,
        4,
        1.0e-9,
        1.0e-5,
        2,
        &[
            CornerSpec::new("nominal", Vec::new()),
            CornerSpec::new(
                "rload-high",
                vec![CornerOverride::new("R1", "resistance", 2_000.0)],
            ),
        ],
    )
    .unwrap()
    .unwrap();

    assert_eq!(result.points.len(), 2);
    assert_eq!(result.points[0].corner_name, "nominal");
    assert_eq!(result.points[1].corner_name, "rload-high");
    assert!(result.points.iter().all(|point| point.result.converged));
    assert_close(result.points[0].result.period_seconds, 1.0e-3);
    assert_close(result.points[1].result.time_step_seconds, 2.5e-4);
    assert_close(
        result.points[0].result.steady_state[0]
            .branch_current("V1")
            .unwrap(),
        -1.0e-3,
    );
    assert_close(
        result.points[1].result.steady_state[0]
            .branch_current("V1")
            .unwrap(),
        -5.0e-4,
    );
}

#[test]
fn pss_text_output_table_is_stable() {
    let mut circuit = Circuit::new();
    circuit.add(Element::VoltageSource(VoltageSource::with_waveform(
        "V1",
        "in",
        "0",
        0.0,
        Waveform::Sin(SinWaveform::new(0.0, 1.0, 1_000.0)),
    )));
    circuit.add(Element::Resistor(Resistor::new("R1", "in", "0", 1_000.0)));

    let result = pss_with_tolerance(&circuit, 4, 1.0e-9, 1.0e-5, 2)
        .unwrap()
        .unwrap();

    assert_eq!(
        format_pss_table(&result, &["V(in)", "I(V1)"]).unwrap(),
        "Index\tPeriod\tTimeStep\tConverged\tIterations\tResidualL2\tTime\tV(in)\tI(V1)\n\
0\t1.000000e-03\t2.500000e-04\ttrue\t1\t2.449295e-16\t2.500000e-04\t1.000000e+00\t-1.000000e-03\n\
1\t1.000000e-03\t2.500000e-04\ttrue\t1\t2.449295e-16\t5.000000e-04\t1.224647e-16\t-1.224647e-19\n\
2\t1.000000e-03\t2.500000e-04\ttrue\t1\t2.449295e-16\t7.500000e-04\t-1.000000e+00\t1.000000e-03\n\
3\t1.000000e-03\t2.500000e-04\ttrue\t1\t2.449295e-16\t1.000000e-03\t-2.449294e-16\t2.449294e-19\n"
    );
}

#[test]
fn corner_pss_text_output_table_is_stable() {
    let mut circuit = Circuit::new();
    circuit.add(Element::VoltageSource(VoltageSource::with_waveform(
        "V1",
        "in",
        "0",
        0.0,
        Waveform::Sin(SinWaveform::new(0.0, 1.0, 1_000.0)),
    )));
    circuit.add(Element::Resistor(Resistor::new("R1", "in", "0", 1_000.0)));

    let result = pss_corners_with_tolerance(
        &circuit,
        4,
        1.0e-9,
        1.0e-5,
        2,
        &[
            CornerSpec::new("nominal", Vec::new()),
            CornerSpec::new(
                "rload-high",
                vec![CornerOverride::new("R1", "resistance", 2_000.0)],
            ),
        ],
    )
    .unwrap()
    .unwrap();

    assert_eq!(
        format_corner_pss_table(&result, &["V(in)", "I(V1)"]).unwrap(),
        "Corner\tIndex\tPeriod\tTimeStep\tConverged\tIterations\tResidualL2\tTime\tV(in)\tI(V1)\n\
nominal\t0\t1.000000e-03\t2.500000e-04\ttrue\t1\t2.449295e-16\t2.500000e-04\t1.000000e+00\t-1.000000e-03\n\
nominal\t1\t1.000000e-03\t2.500000e-04\ttrue\t1\t2.449295e-16\t5.000000e-04\t1.224647e-16\t-1.224647e-19\n\
nominal\t2\t1.000000e-03\t2.500000e-04\ttrue\t1\t2.449295e-16\t7.500000e-04\t-1.000000e+00\t1.000000e-03\n\
nominal\t3\t1.000000e-03\t2.500000e-04\ttrue\t1\t2.449295e-16\t1.000000e-03\t-2.449294e-16\t2.449294e-19\n\
rload-high\t0\t1.000000e-03\t2.500000e-04\ttrue\t1\t2.449294e-16\t2.500000e-04\t1.000000e+00\t-5.000000e-04\n\
rload-high\t1\t1.000000e-03\t2.500000e-04\ttrue\t1\t2.449294e-16\t5.000000e-04\t1.224647e-16\t-6.123234e-20\n\
rload-high\t2\t1.000000e-03\t2.500000e-04\ttrue\t1\t2.449294e-16\t7.500000e-04\t-1.000000e+00\t5.000000e-04\n\
rload-high\t3\t1.000000e-03\t2.500000e-04\ttrue\t1\t2.449294e-16\t1.000000e-03\t-2.449294e-16\t1.224647e-19\n"
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
    assert_eq!(
        format_adaptive_transient_table(&adaptive, &["V(vin)", "V(out)", "I(V1)"]).unwrap(),
        "Method\tStepsRejected\tConverged\tIndex\tTime\tV(vin)\tV(out)\tI(V1)\ntrap\t0\ttrue\t0\t1.000000e-03\t1.000000e+00\t3.333333e-01\t-6.666667e-04\ntrap\t0\ttrue\t1\t2.000000e-03\t1.000000e+00\t7.777778e-01\t-2.222222e-04\ntrap\t0\ttrue\t2\t3.000000e-03\t1.000000e+00\t9.259259e-01\t-7.407407e-05\n"
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
fn fourier_transient_deck_routes_parsed_four_cards() {
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
    let analyses = fourier_transient_deck(
        &points,
        "
.tran 15.625u 2m
.four 1k V(in) HARMONICS=5 FROM=1m
.end
",
    )
    .unwrap();

    assert_eq!(analyses.len(), 1);
    let analysis = &analyses[0];
    let probe = &analysis.probes[0];
    let fundamental = &probe.harmonics[0];

    assert_eq!(probe.probe, "V(in)");
    assert_eq!(probe.harmonics.len(), 5);
    assert!((analysis.start_time - period).abs() < 1.0e-12);
    assert!((probe.dc - offset).abs() < 2.0e-3);
    assert_close(fundamental.frequency_hz, freq);
    assert!((fundamental.magnitude - amp).abs() < 2.0e-3);
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
fn pole_zero_corners_runs_selected_topology_per_corner() {
    let mut circuit = Circuit::new();
    circuit.add(Element::VoltageSource(VoltageSource::new(
        "Vin", "in", "0", 1.0,
    )));
    circuit.add(Element::Resistor(Resistor::new("R1", "in", "out", 1_000.0)));
    circuit.add(Element::Capacitor(Capacitor::new("C1", "out", "0", 1.0e-6)));

    let result = spice_engine::pole_zero_corners(
        &circuit,
        "Vin",
        "out",
        PoleZeroTopology::RcLowpass,
        &[
            CornerSpec::new("nominal", Vec::new()),
            CornerSpec::new(
                "cap-high",
                vec![CornerOverride::new("C1", "capacitance", 2.0e-6)],
            ),
        ],
    )
    .unwrap();

    assert_eq!(result.input_source, "Vin");
    assert_eq!(result.output_node, "out");
    assert_eq!(result.topology, PoleZeroTopology::RcLowpass);
    assert_eq!(result.points.len(), 2);
    assert_eq!(result.points[0].corner_name, "nominal");
    assert_eq!(result.points[1].corner_name, "cap-high");
    assert_close(result.points[0].result.entries[0].real, -1.0e3);
    assert_close(result.points[1].result.entries[0].real, -5.0e2);
}

#[test]
fn corner_pole_zero_text_output_table_is_stable() {
    let mut circuit = Circuit::new();
    circuit.add(Element::VoltageSource(VoltageSource::new(
        "Vin", "in", "0", 1.0,
    )));
    circuit.add(Element::Resistor(Resistor::new("R1", "in", "out", 1_000.0)));
    circuit.add(Element::Capacitor(Capacitor::new("C1", "out", "0", 1.0e-6)));

    let result = spice_engine::pole_zero_corners(
        &circuit,
        "Vin",
        "out",
        PoleZeroTopology::RcLowpass,
        &[
            CornerSpec::new("nominal", Vec::new()),
            CornerSpec::new(
                "cap-high",
                vec![CornerOverride::new("C1", "capacitance", 2.0e-6)],
            ),
        ],
    )
    .unwrap();

    assert_eq!(
        format_corner_pole_zero_table(&result),
        "Corner\tIndex\tKind\tReal\tImaginary\tFrequency\tDamping\n\
nominal\t0\tpole\t-1.000000e+03\t0.000000e+00\t1.591549e+02\t1.000000e+00\n\
cap-high\t0\tpole\t-5.000000e+02\t0.000000e+00\t7.957747e+01\t1.000000e+00\n"
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
fn pole_zero_rlc_notch_returns_imaginary_axis_zeros_and_complex_conjugate_poles() {
    let mut circuit = Circuit::new();
    circuit.add(Element::VoltageSource(VoltageSource::new(
        "Vin", "in", "0", 1.0,
    )));
    circuit.add(Element::Resistor(Resistor::new("R1", "in", "out", 10.0)));
    circuit.add(Element::Inductor(Inductor::new("L1", "out", "mid", 1.0e-3)));
    circuit.add(Element::Capacitor(Capacitor::new("C1", "mid", "0", 1.0e-6)));

    let result = pole_zero_rlc_notch(&circuit, "Vin", "out").unwrap();

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
                    imaginary: omega0,
                    frequency_hz: omega0 / (2.0 * std::f64::consts::PI),
                    damping: 0.0,
                },
                PoleZeroEntry {
                    kind: PoleZeroEntryKind::Zero,
                    real: 0.0,
                    imaginary: -omega0,
                    frequency_hz: omega0 / (2.0 * std::f64::consts::PI),
                    damping: 0.0,
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
fn distortion_from_transient_corners_projects_each_corner() {
    let freq = 1.0e3;
    let period = 1.0 / freq;
    let mut circuit = Circuit::new();
    circuit.add(Element::VoltageSource(VoltageSource::with_waveform(
        "Vin",
        "in",
        "0",
        0.0,
        Waveform::Sin(SinWaveform::new(0.0, 1.0, freq)),
    )));
    circuit.add(Element::Resistor(Resistor::new(
        "Rtop", "in", "out", 1_000.0,
    )));
    circuit.add(Element::Resistor(Resistor::new(
        "Rbot", "out", "0", 1_000.0,
    )));

    let result = distortion_from_transient_corners(
        &circuit,
        period / 64.0,
        2.0 * period,
        freq,
        "Vin",
        "V(out)",
        3,
        &[
            CornerSpec::new("nominal", Vec::new()),
            CornerSpec::new(
                "rbot-high",
                vec![CornerOverride::new("Rbot", "resistance", 3_000.0)],
            ),
        ],
    )
    .unwrap();

    assert_eq!(result.input_source, "Vin");
    assert_eq!(result.output_probe, "V(out)");
    assert_eq!(result.points.len(), 2);
    assert_eq!(result.points[0].corner_name, "nominal");
    assert_eq!(result.points[1].corner_name, "rbot-high");
    assert!((result.points[0].result.points[0].fundamental_magnitude - 0.5).abs() < 2.0e-3);
    assert!((result.points[1].result.points[0].fundamental_magnitude - 0.75).abs() < 2.0e-3);
    assert!(result.points[0].result.points[0].total_harmonic_distortion < 2.0e-3);
    assert!(result.points[1].result.points[0].total_harmonic_distortion < 2.0e-3);
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
fn transient_probe_measurements_are_stable() {
    let points = vec![
        TransientPoint {
            time: 0.0,
            node_voltages: BTreeMap::from([("in".to_string(), 0.0), ("out".to_string(), 0.0)]),
            branch_currents: BTreeMap::new(),
        },
        TransientPoint {
            time: 1.0e-3,
            node_voltages: BTreeMap::from([("in".to_string(), 1.0), ("out".to_string(), 1.25)]),
            branch_currents: BTreeMap::new(),
        },
        TransientPoint {
            time: 2.0e-3,
            node_voltages: BTreeMap::from([("in".to_string(), 1.0), ("out".to_string(), -0.25)]),
            branch_currents: BTreeMap::new(),
        },
        TransientPoint {
            time: 3.0e-3,
            node_voltages: BTreeMap::from([("in".to_string(), 1.0), ("out".to_string(), 0.75)]),
            branch_currents: BTreeMap::new(),
        },
    ];

    let peak_to_peak = measure_transient_probe(
        &points,
        "swing",
        "V(out)",
        "peak-to-peak",
        Some(1.0e-3),
        Some(3.0e-3),
    )
    .unwrap();
    let final_value =
        measure_transient_probe(&points, "settled", "V(out)", "final", None, None).unwrap();
    let midpoint = measure_transient_find_at_probe(&points, "midpoint", "V(out)", 1.5e-3).unwrap();
    let crossing = measure_transient_when_probe(
        &points,
        "crossing",
        "V(out)",
        0.5,
        Some(1.0e-3),
        Some(3.0e-3),
    )
    .unwrap();
    let second_crossing = measure_transient_when_probe_counted(
        &points,
        "second_crossing",
        "V(out)",
        0.5,
        "cross",
        2,
        Some(1.0e-3),
        Some(3.0e-3),
    )
    .unwrap();
    let propagation_delay = measure_transient_delay_between_probes(
        &points,
        "prop_delay",
        "V(in)",
        0.5,
        "rise",
        1,
        "V(out)",
        0.5,
        "fall",
        1,
        Some(0.0),
        Some(3.0e-3),
    )
    .unwrap();

    assert_close(peak_to_peak.value, 1.5);
    assert_eq!(peak_to_peak.mode, "pp");
    assert_close(final_value.value, 0.75);
    assert_eq!(final_value.mode, "last");
    assert_close(midpoint.value, 0.5);
    assert_eq!(midpoint.mode, "find");
    assert_close(crossing.value, 1.5e-3);
    assert_eq!(crossing.mode, "when");
    assert_close(second_crossing.value, 2.75e-3);
    assert_eq!(second_crossing.mode, "when");
    assert_close(propagation_delay.value, 1.0e-3);
    assert_eq!(propagation_delay.probe, "V(in)->V(out)");
    assert_eq!(propagation_delay.mode, "delay");
    assert_eq!(
        format_measurement_table(&[
            peak_to_peak,
            final_value,
            midpoint,
            crossing,
            second_crossing,
            propagation_delay
        ]),
        "Name\tAnalysis\tProbe\tMode\tFrom\tTo\tValue\nswing\ttran\tV(out)\tpp\t1.000000e-03\t3.000000e-03\t1.500000e+00\nsettled\ttran\tV(out)\tlast\t\t\t7.500000e-01\nmidpoint\ttran\tV(out)\tfind\t1.500000e-03\t1.500000e-03\t5.000000e-01\ncrossing\ttran\tV(out)\twhen\t1.000000e-03\t3.000000e-03\t1.500000e-03\nsecond_crossing\ttran\tV(out)\twhen\t1.000000e-03\t3.000000e-03\t2.750000e-03\nprop_delay\ttran\tV(in)->V(out)\tdelay\t0.000000e+00\t3.000000e-03\t1.000000e-03\n"
    );
}

#[test]
fn transient_deck_measurements_execute_parsed_cards() {
    let points = vec![
        TransientPoint {
            time: 0.0,
            node_voltages: BTreeMap::from([("in".to_string(), 0.0), ("out".to_string(), 0.0)]),
            branch_currents: BTreeMap::new(),
        },
        TransientPoint {
            time: 1.0e-3,
            node_voltages: BTreeMap::from([("in".to_string(), 1.0), ("out".to_string(), 1.25)]),
            branch_currents: BTreeMap::new(),
        },
        TransientPoint {
            time: 2.0e-3,
            node_voltages: BTreeMap::from([("in".to_string(), 1.0), ("out".to_string(), -0.25)]),
            branch_currents: BTreeMap::new(),
        },
        TransientPoint {
            time: 3.0e-3,
            node_voltages: BTreeMap::from([("in".to_string(), 1.0), ("out".to_string(), 0.75)]),
            branch_currents: BTreeMap::new(),
        },
    ];

    let measurements = measure_transient_deck(
        &points,
        "
V1 in 0 DC 1
.measure tran swing PP V(out) FROM=1m TO=3m
.measure tran midpoint FIND V(out) AT=1.5m
.measure tran crossing WHEN V(out)=0.5 FROM=1m TO=3m
.measure tran second_cross WHEN V(out)=0.5 FROM=1m TO=3m CROSS=2
.measure tran falling WHEN V(out)=0.5 FROM=1m TO=3m FALL=1
.measure tran rising WHEN V(out)=0.5 FROM=1m TO=3m RISE=1
.measure tran prop_delay TRIG V(in) VAL=0.5 RISE=1 TARG V(out) VAL=0.5 FALL=1 FROM=0 TO=3m
.meas tran settled LAST V(out)
.end
",
    )
    .unwrap();

    assert_eq!(
        format_measurement_table(&measurements),
        "Name\tAnalysis\tProbe\tMode\tFrom\tTo\tValue\nswing\ttran\tV(out)\tpp\t1.000000e-03\t3.000000e-03\t1.500000e+00\nmidpoint\ttran\tV(out)\tfind\t1.500000e-03\t1.500000e-03\t5.000000e-01\ncrossing\ttran\tV(out)\twhen\t1.000000e-03\t3.000000e-03\t1.500000e-03\nsecond_cross\ttran\tV(out)\twhen\t1.000000e-03\t3.000000e-03\t2.750000e-03\nfalling\ttran\tV(out)\twhen\t1.000000e-03\t3.000000e-03\t1.500000e-03\nrising\ttran\tV(out)\twhen\t1.000000e-03\t3.000000e-03\t2.750000e-03\nprop_delay\ttran\tV(in)->V(out)\tdelay\t0.000000e+00\t3.000000e-03\t1.000000e-03\nsettled\ttran\tV(out)\tlast\t\t\t7.500000e-01\n"
    );
}

#[test]
fn transient_deck_output_cards_select_table_probes() {
    let points = vec![
        TransientPoint {
            time: 0.0,
            node_voltages: BTreeMap::from([
                ("out".to_string(), 0.0),
                ("clk".to_string(), 0.0),
                ("ignored".to_string(), 1.0),
            ]),
            branch_currents: BTreeMap::from([("I(V1)".to_string(), -1.0e-3)]),
        },
        TransientPoint {
            time: 1.0e-3,
            node_voltages: BTreeMap::from([
                ("out".to_string(), 1.0),
                ("clk".to_string(), 5.0),
                ("ignored".to_string(), 2.0),
            ]),
            branch_currents: BTreeMap::from([("I(V1)".to_string(), -2.0e-3)]),
        },
    ];

    let table = format_deck_transient_table(
        &points,
        "
.save V(out) I(V1)
.probe tran V(clk) V(out)
.print tran V(ignored)
.plot tran I(V1)
.probe ac V(ignored)
.end
",
    )
    .unwrap();

    assert_eq!(
        table,
        "Index\tTime\tV(out)\tI(V1)\tV(clk)\tV(ignored)\n0\t0.000000e+00\t0.000000e+00\t-1.000000e-03\t0.000000e+00\t1.000000e+00\n1\t1.000000e-03\t1.000000e+00\t-2.000000e-03\t5.000000e+00\t2.000000e+00\n"
    );
}

#[test]
fn run_deck_analysis_routes_selected_plan_and_output_table() {
    let mut circuit = Circuit::new();
    circuit.add(Element::VoltageSource(VoltageSource::new(
        "V1", "vin", "0", 1.0,
    )));
    circuit.add(Element::Resistor(Resistor::new(
        "R1", "vin", "mid", 1_000.0,
    )));
    circuit.add(Element::Resistor(Resistor::new("R2", "mid", "0", 1_000.0)));
    let netlist = "
.save V(mid)
.probe dc I(V1)
.op
.dc V1 0 1 1
.ac dec 1 1k 1k
.tran 1m 1m
.measure dc mid_avg avg V(mid)
.measure ac mid_peak max V(mid)
.measure tran mid_final final V(mid)
.end
";

    let op_execution = run_deck_analysis(&circuit, netlist, Some("op")).unwrap();
    assert_eq!(op_execution.plan.analysis, "op");
    assert_eq!(op_execution.output_probes, vec!["V(mid)".to_string()]);
    assert!(op_execution.measurements.is_empty());
    assert_eq!(
        op_execution.measurement_table,
        "Name\tAnalysis\tProbe\tMode\tFrom\tTo\tValue\n"
    );
    assert_eq!(op_execution.table, "Index\tV(mid)\n0\t5.000000e-01\n");
    assert_eq!(op_execution.run_artifacts[0].result_rows, 1);
    assert_eq!(
        op_execution.run_artifacts[0].output_probes,
        vec!["V(mid)".to_string()]
    );
    assert!(op_execution.run_artifacts[0].measurement_names.is_empty());
    assert_eq!(
        op_execution.run_artifact_table,
        format!(
            "Analysis\tDirective\tLine\tResultRows\tOutputProbes\tOutputProbeList\tMeasurements\tMeasurementList\tFourier\nop\t.op\t{}\t1\t1\tV(mid)\t0\t\t0\n",
            op_execution.plan.line_number
        )
    );

    let dc_execution = run_deck_analysis(&circuit, netlist, Some("dc")).unwrap();
    assert_eq!(dc_execution.plan.source_name.as_deref(), Some("V1"));
    assert_eq!(
        dc_execution.output_probes,
        vec!["V(mid)".to_string(), "I(V1)".to_string()]
    );
    assert_eq!(dc_execution.measurements[0].name, "mid_avg");
    assert_eq!(
        dc_execution.measurement_table,
        "Name\tAnalysis\tProbe\tMode\tFrom\tTo\tValue\nmid_avg\tdc\tV(mid)\tavg\t\t\t2.500000e-01\n"
    );
    match dc_execution.result {
        DeckAnalysisExecutionResult::DcSweep(points) => assert_eq!(points.len(), 2),
        other => panic!("expected DC sweep result, got {other:?}"),
    }
    assert_eq!(
        dc_execution.table,
        "Index\tSource\tValue\tV(mid)\tI(V1)\n0\tV1\t0.000000e+00\t0.000000e+00\t0.000000e+00\n1\tV1\t1.000000e+00\t5.000000e-01\t-5.000000e-04\n"
    );
    assert_eq!(dc_execution.run_artifacts[0].analysis, "dc");
    assert_eq!(
        dc_execution.run_artifacts[0].output_probes,
        vec!["V(mid)".to_string(), "I(V1)".to_string()]
    );
    assert_eq!(
        dc_execution.run_artifacts[0].measurement_names,
        vec!["mid_avg".to_string()]
    );
    assert_eq!(
        dc_execution.run_artifact_table,
        format!(
            "Analysis\tDirective\tLine\tResultRows\tOutputProbes\tOutputProbeList\tMeasurements\tMeasurementList\tFourier\ndc\t.dc\t{}\t2\t2\tV(mid);I(V1)\t1\tmid_avg\t0\n",
            dc_execution.plan.line_number
        )
    );

    let ac_execution = run_deck_analysis(&circuit, netlist, Some("ac")).unwrap();
    assert_eq!(ac_execution.output_probes, vec!["V(mid)".to_string()]);
    assert_eq!(ac_execution.measurements[0].name, "mid_peak");
    assert_eq!(
        ac_execution.measurement_table,
        "Name\tAnalysis\tProbe\tMode\tFrom\tTo\tValue\nmid_peak\tac\tV(mid)\tmax\t\t\t5.000000e-01\n"
    );
    match ac_execution.result {
        DeckAnalysisExecutionResult::Ac(points) => assert_eq!(points.len(), 1),
        other => panic!("expected AC result, got {other:?}"),
    }
    assert_eq!(
        ac_execution.table,
        "Index\tFrequency\tProbe\tReal\tImaginary\tMagnitude\tPhase\n0\t1.000000e+03\tV(mid)\t5.000000e-01\t0.000000e+00\t5.000000e-01\t0.000000e+00\n"
    );
    assert_eq!(
        ac_execution.run_artifacts[0].output_probes,
        vec!["V(mid)".to_string()]
    );
    assert_eq!(
        ac_execution.run_artifacts[0].measurement_names,
        vec!["mid_peak".to_string()]
    );
    assert_eq!(
        ac_execution.run_artifact_table,
        format!(
            "Analysis\tDirective\tLine\tResultRows\tOutputProbes\tOutputProbeList\tMeasurements\tMeasurementList\tFourier\nac\t.ac\t{}\t1\t1\tV(mid)\t1\tmid_peak\t0\n",
            ac_execution.plan.line_number
        )
    );

    let tran_execution = run_deck_analysis(&circuit, netlist, Some("tran")).unwrap();
    assert_eq!(tran_execution.output_probes, vec!["V(mid)".to_string()]);
    assert_eq!(tran_execution.measurements[0].name, "mid_final");
    assert_eq!(
        tran_execution.measurement_table,
        "Name\tAnalysis\tProbe\tMode\tFrom\tTo\tValue\nmid_final\ttran\tV(mid)\tlast\t\t\t5.000000e-01\n"
    );
    match tran_execution.result {
        DeckAnalysisExecutionResult::Tran(points) => assert_eq!(points.len(), 1),
        other => panic!("expected transient result, got {other:?}"),
    }
    assert_eq!(
        tran_execution.table,
        "Index\tTime\tV(mid)\n0\t1.000000e-03\t5.000000e-01\n"
    );
    assert_eq!(
        tran_execution.run_artifacts[0].output_probes,
        vec!["V(mid)".to_string()]
    );
    assert_eq!(
        tran_execution.run_artifacts[0].measurement_names,
        vec!["mid_final".to_string()]
    );
    assert_eq!(
        tran_execution.run_artifact_table,
        format!(
            "Analysis\tDirective\tLine\tResultRows\tOutputProbes\tOutputProbeList\tMeasurements\tMeasurementList\tFourier\ntran\t.tran\t{}\t1\t1\tV(mid)\t1\tmid_final\t0\n",
            tran_execution.plan.line_number
        )
    );

    let tran_window_execution = run_deck_analysis(
        &circuit,
        ".save V(mid)\n.tran 2m 6m 2m 1m uic\n.end\n",
        None,
    )
    .unwrap();
    assert!((tran_window_execution.plan.start_time.unwrap() - 2.0e-3).abs() < 1.0e-12);
    assert!((tran_window_execution.plan.max_step.unwrap() - 1.0e-3).abs() < 1.0e-12);
    assert!(tran_window_execution.plan.use_initial_conditions);
    assert_eq!(
        tran_window_execution.output_probes,
        vec!["V(mid)".to_string()]
    );
    match &tran_window_execution.result {
        DeckAnalysisExecutionResult::Tran(points) => {
            let expected_times = [2.0e-3, 4.0e-3, 6.0e-3];
            assert_eq!(points.len(), expected_times.len());
            for (point, expected_time) in points.iter().zip(expected_times) {
                assert!((point.time - expected_time).abs() < 1.0e-12);
            }
        }
        other => panic!("expected transient result, got {other:?}"),
    }
    assert_eq!(
        tran_window_execution.table,
        "Index\tTime\tV(mid)\n0\t2.000000e-03\t5.000000e-01\n1\t4.000000e-03\t5.000000e-01\n2\t6.000000e-03\t5.000000e-01\n"
    );

    let error = run_deck_analysis(&circuit, netlist, None).unwrap_err();
    assert!(error.to_string().contains("multiple analysis cards"));

    let lin_execution =
        run_deck_analysis(&circuit, ".save V(mid)\n.ac lin 3 1 3\n.end\n", None).unwrap();
    assert_eq!(lin_execution.output_probes, vec!["V(mid)".to_string()]);
    match &lin_execution.result {
        DeckAnalysisExecutionResult::Ac(points) => assert_eq!(
            points
                .iter()
                .map(|point| point.frequency_hz)
                .collect::<Vec<_>>(),
            vec![1.0, 2.0, 3.0]
        ),
        other => panic!("expected AC result, got {other:?}"),
    }
    assert_eq!(
        lin_execution.table,
        "Index\tFrequency\tProbe\tReal\tImaginary\tMagnitude\tPhase\n0\t1.000000e+00\tV(mid)\t5.000000e-01\t0.000000e+00\t5.000000e-01\t0.000000e+00\n1\t2.000000e+00\tV(mid)\t5.000000e-01\t0.000000e+00\t5.000000e-01\t0.000000e+00\n2\t3.000000e+00\tV(mid)\t5.000000e-01\t0.000000e+00\t5.000000e-01\t0.000000e+00\n"
    );

    let oct_execution =
        run_deck_analysis(&circuit, ".save V(mid)\n.ac oct 1 1 4\n.end\n", None).unwrap();
    assert_eq!(oct_execution.output_probes, vec!["V(mid)".to_string()]);
    match &oct_execution.result {
        DeckAnalysisExecutionResult::Ac(points) => assert_eq!(
            points
                .iter()
                .map(|point| point.frequency_hz)
                .collect::<Vec<_>>(),
            vec![1.0, 2.0, 4.0]
        ),
        other => panic!("expected AC result, got {other:?}"),
    }
    assert_eq!(
        oct_execution.table,
        "Index\tFrequency\tProbe\tReal\tImaginary\tMagnitude\tPhase\n0\t1.000000e+00\tV(mid)\t5.000000e-01\t0.000000e+00\t5.000000e-01\t0.000000e+00\n1\t2.000000e+00\tV(mid)\t5.000000e-01\t0.000000e+00\t5.000000e-01\t0.000000e+00\n2\t4.000000e+00\tV(mid)\t5.000000e-01\t0.000000e+00\t5.000000e-01\t0.000000e+00\n"
    );
}

#[test]
fn run_deck_analysis_exposes_selected_fourier_artifacts() {
    let mut circuit = Circuit::new();
    circuit.add(Element::VoltageSource(VoltageSource::new(
        "V1", "vin", "0", 1.0,
    )));
    circuit.add(Element::Resistor(Resistor::new(
        "R1", "vin", "mid", 1_000.0,
    )));
    circuit.add(Element::Resistor(Resistor::new("R2", "mid", "0", 1_000.0)));
    let netlist = "
.save V(mid)
.op
.tran 0.5m 1m
.four 2k V(mid) harmonics=1
.end
";

    let op_execution = run_deck_analysis(&circuit, netlist, Some("op")).unwrap();
    assert!(op_execution.fourier.is_empty());
    assert_eq!(op_execution.fourier_table, "");

    let tran_execution = run_deck_analysis(&circuit, netlist, Some("tran")).unwrap();
    assert_eq!(tran_execution.fourier.len(), 1);
    let result = &tran_execution.fourier[0];
    assert!((result.fundamental_frequency_hz - 2_000.0).abs() < 1.0e-12);
    assert_eq!(result.probes[0].probe, "V(mid)");
    assert_eq!(result.probes[0].harmonics.len(), 1);
    assert_eq!(tran_execution.fourier_table, format_fourier_table(result));
    assert_eq!(tran_execution.run_artifacts[0].fourier_count, 1);
    assert_eq!(
        tran_execution.run_artifacts[0].output_probes,
        vec!["V(mid)".to_string()]
    );
    assert!(tran_execution.run_artifacts[0].measurement_names.is_empty());
    assert_eq!(
        tran_execution.run_artifact_table,
        format!(
            "Analysis\tDirective\tLine\tResultRows\tOutputProbes\tOutputProbeList\tMeasurements\tMeasurementList\tFourier\ntran\t.tran\t{}\t2\t1\tV(mid)\t0\t\t1\n",
            tran_execution.plan.line_number
        )
    );
}

#[test]
fn corner_transient_text_output_table_is_stable() {
    let mut circuit = Circuit::new();
    circuit.add(Element::VoltageSource(VoltageSource::new(
        "V1", "vin", "0", 10.0,
    )));
    circuit.add(Element::Resistor(Resistor::new(
        "R1", "vin", "mid", 1_000.0,
    )));
    circuit.add(Element::Resistor(Resistor::new("R2", "mid", "0", 1_000.0)));

    let result = transient_corners(
        &circuit,
        1.0e-3,
        2.0e-3,
        &[
            CornerSpec::new("nominal", vec![]),
            CornerSpec::new(
                "r2-high",
                vec![CornerOverride::new("R2", "resistance", 2_000.0)],
            ),
        ],
    )
    .unwrap();

    assert_eq!(
        format_corner_transient_table(&result, &["V(vin)", "V(mid)", "I(V1)"]).unwrap(),
        "Corner\tIndex\tTime\tV(vin)\tV(mid)\tI(V1)\nnominal\t0\t1.000000e-03\t1.000000e+01\t5.000000e+00\t-5.000000e-03\nnominal\t1\t2.000000e-03\t1.000000e+01\t5.000000e+00\t-5.000000e-03\nr2-high\t0\t1.000000e-03\t1.000000e+01\t6.666667e+00\t-3.333333e-03\nr2-high\t1\t2.000000e-03\t1.000000e+01\t6.666667e+00\t-3.333333e-03\n"
    );
}

#[test]
fn corner_adaptive_transient_text_output_table_is_stable() {
    let mut circuit = Circuit::new();
    circuit.add(Element::VoltageSource(VoltageSource::new(
        "V1", "vin", "0", 1.0,
    )));
    circuit.add(Element::Resistor(Resistor::new(
        "R1", "vin", "out", 1_000.0,
    )));
    circuit.add(Element::Capacitor(Capacitor::new("C1", "out", "0", 1.0e-6)));

    let result = transient_adaptive_corners(
        &circuit,
        1.0e-3,
        2.0e-3,
        AdaptiveTransientOptions {
            method: TransientMethod::Trap,
            tolerance: 1.0,
            min_step: Some(1.0e-3),
            max_step: Some(1.0e-3),
        },
        &[
            CornerSpec::new("nominal", vec![]),
            CornerSpec::new(
                "r1-high",
                vec![CornerOverride::new("R1", "resistance", 2_000.0)],
            ),
        ],
    )
    .unwrap();

    assert_eq!(
        format_corner_adaptive_transient_table(&result, &["V(vin)", "V(out)", "I(V1)"]).unwrap(),
        "Corner\tMethod\tStepsRejected\tConverged\tIndex\tTime\tV(vin)\tV(out)\tI(V1)\nnominal\ttrap\t0\ttrue\t0\t1.000000e-03\t1.000000e+00\t3.333333e-01\t-6.666667e-04\nnominal\ttrap\t0\ttrue\t1\t2.000000e-03\t1.000000e+00\t7.777778e-01\t-2.222222e-04\nr1-high\ttrap\t0\ttrue\t0\t1.000000e-03\t1.000000e+00\t2.000000e-01\t-4.000000e-04\nr1-high\ttrap\t0\ttrue\t1\t2.000000e-03\t1.000000e+00\t5.200000e-01\t-2.400000e-04\n"
    );
}

#[test]
fn pole_zero_text_output_table_is_stable() {
    let result = PoleZeroResult {
        input_source: "Vin".to_string(),
        output_node: "out".to_string(),
        entries: vec![
            PoleZeroEntry {
                kind: PoleZeroEntryKind::Zero,
                real: 0.0,
                imaginary: 1.0e3,
                frequency_hz: 1.0e3 / (2.0 * std::f64::consts::PI),
                damping: 0.0,
            },
            PoleZeroEntry {
                kind: PoleZeroEntryKind::Pole,
                real: -5.0,
                imaginary: -999.987499921874,
                frequency_hz: 1.0e3 / (2.0 * std::f64::consts::PI),
                damping: 5.0e-3,
            },
        ],
    };

    assert_eq!(
        format_pole_zero_table(&result),
        "Index\tKind\tReal\tImaginary\tFrequency\tDamping\n0\tzero\t0.000000e+00\t1.000000e+03\t1.591549e+02\t0.000000e+00\n1\tpole\t-5.000000e+00\t-9.999875e+02\t1.591549e+02\t5.000000e-03\n"
    );
}

#[test]
fn distortion_text_output_table_is_stable() {
    let result = DistortionResult {
        input_source: "Vin".to_string(),
        output_probe: "V(out)".to_string(),
        points: vec![DistortionPoint {
            frequency_hz: 1000.0,
            fundamental_magnitude: 1.0,
            harmonics: vec![
                DistortionHarmonic {
                    harmonic: 1,
                    frequency_hz: 1000.0,
                    magnitude: 1.0,
                    phase_degrees: 0.0,
                },
                DistortionHarmonic {
                    harmonic: 2,
                    frequency_hz: 2000.0,
                    magnitude: 0.025,
                    phase_degrees: -1.5707963267948966,
                },
            ],
            total_harmonic_distortion: 0.025,
        }],
    };

    assert_eq!(
        format_distortion_table(&result),
        "Frequency\tInput\tOutput\tHarmonic\tMagnitude\tPhase\tTHD\n1.000000e+03\tVin\tV(out)\t1\t1.000000e+00\t0.000000e+00\t2.500000e-02\n1.000000e+03\tVin\tV(out)\t2\t2.500000e-02\t-1.570796e+00\t2.500000e-02\n"
    );
}

#[test]
fn corner_distortion_text_output_table_is_stable() {
    let result = CornerDistortionResult {
        input_source: "Vin".to_string(),
        output_probe: "V(out)".to_string(),
        points: vec![
            CornerDistortionPoint {
                corner_name: "nominal".to_string(),
                result: DistortionResult {
                    input_source: "Vin".to_string(),
                    output_probe: "V(out)".to_string(),
                    points: vec![DistortionPoint {
                        frequency_hz: 1000.0,
                        fundamental_magnitude: 1.0,
                        harmonics: vec![
                            DistortionHarmonic {
                                harmonic: 1,
                                frequency_hz: 1000.0,
                                magnitude: 1.0,
                                phase_degrees: 0.0,
                            },
                            DistortionHarmonic {
                                harmonic: 2,
                                frequency_hz: 2000.0,
                                magnitude: 0.025,
                                phase_degrees: -1.5707963267948966,
                            },
                        ],
                        total_harmonic_distortion: 0.025,
                    }],
                },
            },
            CornerDistortionPoint {
                corner_name: "slow".to_string(),
                result: DistortionResult {
                    input_source: "Vin".to_string(),
                    output_probe: "V(out)".to_string(),
                    points: vec![DistortionPoint {
                        frequency_hz: 1000.0,
                        fundamental_magnitude: 0.8,
                        harmonics: vec![DistortionHarmonic {
                            harmonic: 2,
                            frequency_hz: 2000.0,
                            magnitude: 0.04,
                            phase_degrees: 12.5,
                        }],
                        total_harmonic_distortion: 0.05,
                    }],
                },
            },
        ],
    };

    assert_eq!(
        format_corner_distortion_table(&result),
        "Corner\tFrequency\tInput\tOutput\tHarmonic\tMagnitude\tPhase\tTHD\nnominal\t1.000000e+03\tVin\tV(out)\t1\t1.000000e+00\t0.000000e+00\t2.500000e-02\nnominal\t1.000000e+03\tVin\tV(out)\t2\t2.500000e-02\t-1.570796e+00\t2.500000e-02\nslow\t1.000000e+03\tVin\tV(out)\t2\t4.000000e-02\t1.250000e+01\t5.000000e-02\n"
    );
}

#[test]
fn fourier_text_output_table_is_stable() {
    let result = FourierResult {
        fundamental_frequency_hz: 1000.0,
        start_time: 0.0,
        end_time: 0.001,
        probes: vec![FourierProbeResult {
            probe: "V(out)".to_string(),
            dc: 0.1,
            harmonics: vec![
                FourierHarmonic {
                    harmonic: 1,
                    frequency_hz: 1000.0,
                    cosine: 1.0,
                    sine: 0.0,
                    magnitude: 1.0,
                    phase_degrees: 0.0,
                },
                FourierHarmonic {
                    harmonic: 2,
                    frequency_hz: 2000.0,
                    cosine: 0.0,
                    sine: -0.025,
                    magnitude: 0.025,
                    phase_degrees: -90.0,
                },
            ],
            total_harmonic_distortion: 0.025,
        }],
    };

    assert_eq!(
        format_fourier_table(&result),
        "Probe\tHarmonic\tFrequency\tCosine\tSine\tMagnitude\tPhase\tDC\tTHD\nV(out)\t1\t1.000000e+03\t1.000000e+00\t0.000000e+00\t1.000000e+00\t0.000000e+00\t1.000000e-01\t2.500000e-02\nV(out)\t2\t2.000000e+03\t0.000000e+00\t-2.500000e-02\t2.500000e-02\t-9.000000e+01\t1.000000e-01\t2.500000e-02\n"
    );
}

#[test]
fn corner_fourier_text_output_table_is_stable() {
    let mut circuit = Circuit::new();
    circuit.add(Element::VoltageSource(VoltageSource::with_waveform(
        "Vin",
        "in",
        "0",
        0.0,
        Waveform::Sin(SinWaveform::new(0.0, 1.0, 1_000.0)),
    )));
    circuit.add(Element::Resistor(Resistor::new("R1", "in", "out", 1_000.0)));
    circuit.add(Element::Resistor(Resistor::new("R2", "out", "0", 1_000.0)));

    let result = fourier_corners(
        &circuit,
        2.5e-4,
        2.0e-3,
        1_000.0,
        &["V(out)"],
        2,
        &[
            CornerSpec::new("nominal", vec![]),
            CornerSpec::new(
                "r2-high",
                vec![CornerOverride::new("R2", "resistance", 2_000.0)],
            ),
        ],
    )
    .unwrap();

    assert_eq!(result.points[0].corner_name, "nominal");
    assert_eq!(result.points[1].corner_name, "r2-high");
    assert_close(
        result.points[0].result.probes[0].harmonics[0].magnitude,
        0.5,
    );
    assert_close(
        result.points[1].result.probes[0].harmonics[0].magnitude,
        2.0 / 3.0,
    );

    assert_eq!(
        format_corner_fourier_table(&result),
        "Corner\tProbe\tHarmonic\tFrequency\tCosine\tSine\tMagnitude\tPhase\tDC\tTHD\nnominal\tV(out)\t1\t1.000000e+03\t6.018531e-33\t5.000000e-01\t5.000000e-01\t6.896729e-31\t0.000000e+00\t1.224647e-16\nnominal\tV(out)\t2\t2.000000e+03\t0.000000e+00\t-6.123234e-17\t6.123234e-17\t1.800000e+02\t0.000000e+00\t1.224647e-16\nr2-high\tV(out)\t1\t1.000000e+03\t7.523164e-33\t6.666667e-01\t6.666667e-01\t6.465683e-31\t1.355253e-17\t1.290373e-16\nr2-high\tV(out)\t2\t2.000000e+03\t2.710505e-17\t-8.164312e-17\t8.602490e-17\t1.616341e+02\t1.355253e-17\t1.290373e-16\n"
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

#[test]
fn digital_events_build_finite_edge_pwl_voltage_source() {
    let events = [
        DigitalEvent::new(0.0, DigitalState::Low),
        DigitalEvent::new(0.5e-9, DigitalState::High),
        DigitalEvent::new(1.25e-9, DigitalState::Low),
    ];
    let levels = DigitalLogicLevels::cmos_1v8(0.25e-9);

    let source =
        spice_engine::digital_events_to_voltage_source("Vdin", "din", "0", &events, levels)
            .unwrap();
    let waveform = source.waveform.as_ref().unwrap();

    assert_close(source.voltage, 0.0);
    assert_close(waveform.value_at(0.25e-9), 0.0);
    assert_close(waveform.value_at(0.625e-9), 0.9);
    assert_close(waveform.value_at(0.75e-9), 1.8);
    assert_close(waveform.value_at(1.5e-9), 0.0);

    let mut circuit = Circuit::new();
    circuit.add(Element::VoltageSource(source));
    circuit.add(Element::Resistor(Resistor::new(
        "Rload", "din", "0", 1_000.0,
    )));

    let points = transient(&circuit, 0.25e-9, 1.5e-9).unwrap();

    assert_close(points[0].voltage("din").unwrap(), 0.0);
    assert_close(points[2].voltage("din").unwrap(), 1.8);
    assert_close(points.last().unwrap().voltage("din").unwrap(), 0.0);
}

#[test]
fn named_digital_event_streams_build_pwl_voltage_sources() {
    let streams = [
        DigitalEventStream::new(
            "din",
            vec![
                DigitalEvent::new(0.0, DigitalState::Low),
                DigitalEvent::new(0.5e-9, DigitalState::High),
                DigitalEvent::new(1.25e-9, DigitalState::Low),
            ],
        ),
        DigitalEventStream::new(
            "enable",
            vec![
                DigitalEvent::new(0.0, DigitalState::High),
                DigitalEvent::new(1.0e-9, DigitalState::Low),
            ],
        ),
    ];
    let sources = digital_event_streams_to_voltage_sources(
        &streams,
        "0",
        DigitalLogicLevels::cmos_1v8(0.25e-9),
    )
    .unwrap();

    assert_eq!(sources.len(), 2);
    assert_eq!(sources[0].name, "Vdin");
    assert_eq!(sources[0].positive, "din");
    assert_eq!(sources[1].name, "Venable");
    assert_eq!(sources[1].positive, "enable");

    let mut circuit = Circuit::new();
    for source in sources {
        circuit.add(Element::VoltageSource(source));
    }
    circuit.add(Element::Resistor(Resistor::new(
        "Rdin", "din", "0", 1_000.0,
    )));
    circuit.add(Element::Resistor(Resistor::new(
        "Renable", "enable", "0", 1_000.0,
    )));

    let points = transient(&circuit, 0.25e-9, 1.5e-9).unwrap();

    assert_close(points[0].voltage("din").unwrap(), 0.0);
    assert_close(points[2].voltage("din").unwrap(), 1.8);
    assert_close(points.last().unwrap().voltage("din").unwrap(), 0.0);
    assert_close(points[0].voltage("enable").unwrap(), 1.8);
    assert_close(points.last().unwrap().voltage("enable").unwrap(), 0.0);
}

#[test]
fn digital_bridge_schedule_collects_unique_event_and_transition_breakpoints() {
    let streams = [
        DigitalEventStream::new(
            "clk",
            vec![
                DigitalEvent::new(0.0, DigitalState::Low),
                DigitalEvent::new(0.5e-9, DigitalState::High),
                DigitalEvent::new(1.25e-9, DigitalState::Low),
            ],
        ),
        DigitalEventStream::new(
            "enable",
            vec![
                DigitalEvent::new(0.25e-9, DigitalState::Low),
                DigitalEvent::new(0.75e-9, DigitalState::High),
            ],
        ),
    ];

    let schedule = spice_engine::digital_event_streams_to_bridge_schedule(
        &streams,
        DigitalLogicLevels::cmos_1v8(0.25e-9),
    )
    .unwrap();

    assert_close(schedule.stop_time, 1.5e-9);
    assert_eq!(schedule.breakpoints.len(), 7);
    assert_close(schedule.breakpoints[0], 0.0);
    assert_close(schedule.breakpoints[1], 0.25e-9);
    assert_close(schedule.breakpoints[2], 0.5e-9);
    assert_close(schedule.breakpoints[3], 0.75e-9);
    assert_close(schedule.breakpoints[4], 1.0e-9);
    assert_close(schedule.breakpoints[5], 1.25e-9);
    assert_close(schedule.breakpoints[6], 1.5e-9);
    assert_eq!(
        format_digital_bridge_schedule_table(&schedule).unwrap(),
        "Index\tTime\tStopTime\n0\t0.000000e+00\t1.500000e-09\n1\t2.500000e-10\t1.500000e-09\n2\t5.000000e-10\t1.500000e-09\n3\t7.500000e-10\t1.500000e-09\n4\t1.000000e-09\t1.500000e-09\n5\t1.250000e-09\t1.500000e-09\n6\t1.500000e-09\t1.500000e-09\n"
    );
}

#[test]
fn digital_bridge_schedule_rejects_overlapping_transitions() {
    let streams = [DigitalEventStream::new(
        "din",
        vec![
            DigitalEvent::new(0.0, DigitalState::Low),
            DigitalEvent::new(0.5e-9, DigitalState::High),
            DigitalEvent::new(0.6e-9, DigitalState::Low),
        ],
    )];

    assert!(matches!(
        spice_engine::digital_event_streams_to_bridge_schedule(
            &streams,
            DigitalLogicLevels::cmos_1v8(0.25e-9),
        ),
        Err(SpiceError::InvalidElement { name, .. }) if name == "digital_events"
    ));
}

#[test]
fn digital_bridge_schedule_table_rejects_unsorted_breakpoints() {
    let schedule = DigitalBridgeSchedule {
        stop_time: 1.0e-9,
        breakpoints: vec![0.5e-9, 0.25e-9],
    };

    assert!(matches!(
        format_digital_bridge_schedule_table(&schedule),
        Err(SpiceError::InvalidElement { name, .. }) if name == "digital_bridge_schedule"
    ));
}

#[test]
fn transient_bridge_runs_digital_input_and_samples_output_stream() {
    let input_streams = [DigitalEventStream::new(
        "din",
        vec![
            DigitalEvent::new(0.0, DigitalState::Low),
            DigitalEvent::new(0.5e-9, DigitalState::High),
            DigitalEvent::new(1.25e-9, DigitalState::Low),
        ],
    )];
    let mut circuit = Circuit::new();
    circuit.add(Element::Resistor(Resistor::new(
        "Rout", "din", "out", 1_000.0,
    )));
    circuit.add(Element::Capacitor(Capacitor::new(
        "Cout", "out", "0", 0.1e-12,
    )));
    circuit.add(Element::Resistor(Resistor::new(
        "Rload", "out", "0", 10_000.0,
    )));

    let result = transient_with_digital_event_streams(
        &circuit,
        &input_streams,
        "0",
        DigitalLogicLevels::cmos_1v8(0.25e-9),
        0.25e-9,
        1.5e-9,
        &[("dout", "V(out)")],
        DigitalThresholds::cmos_1v8(),
    )
    .unwrap();

    assert_eq!(result.output_streams.len(), 1);
    assert_eq!(result.output_streams[0].signal_name, "dout");
    assert_eq!(
        format_digital_event_stream_table(&result.output_streams).unwrap(),
        "Signal\tIndex\tTime\tState\ndout\t0\t2.500000e-10\tlow\ndout\t1\t7.500000e-10\thigh\ndout\t2\t1.500000e-09\tlow\n"
    );
    assert!(result
        .points
        .iter()
        .any(|point| point.voltage("out").unwrap() > 1.2));
}

#[test]
fn digital_transient_bridge_runs_across_named_corners_and_formats_stream_table() {
    let input_streams = [DigitalEventStream::new(
        "din",
        vec![
            DigitalEvent::new(0.0, DigitalState::Low),
            DigitalEvent::new(0.5e-9, DigitalState::High),
            DigitalEvent::new(1.25e-9, DigitalState::Low),
        ],
    )];
    let mut circuit = Circuit::new();
    circuit.add(Element::Resistor(Resistor::new(
        "Rout", "din", "out", 1_000.0,
    )));
    circuit.add(Element::Capacitor(Capacitor::new(
        "Cout", "out", "0", 0.1e-12,
    )));
    circuit.add(Element::Resistor(Resistor::new(
        "Rload", "out", "0", 10_000.0,
    )));
    let corners = [
        CornerSpec::new("nominal", vec![]),
        CornerSpec::new(
            "cout-large",
            vec![CornerOverride::new("Cout", "capacitance", 10.0e-12)],
        ),
    ];

    let result = transient_with_digital_event_streams_corners(
        &circuit,
        &input_streams,
        "0",
        DigitalLogicLevels::cmos_1v8(0.25e-9),
        0.25e-9,
        1.5e-9,
        &[("dout", "V(out)")],
        DigitalThresholds::cmos_1v8(),
        &corners,
    )
    .unwrap();

    assert_eq!(result.points.len(), 2);
    assert_eq!(result.points[0].corner_name, "nominal");
    assert_eq!(result.points[1].corner_name, "cout-large");
    assert!(result.points[0]
        .result
        .points
        .iter()
        .any(|point| point.voltage("out").unwrap() > 1.2));
    assert!(result.points[1]
        .result
        .points
        .iter()
        .all(|point| point.voltage("out").unwrap() < 1.2));
    assert_eq!(
        format_corner_digital_event_stream_table(&result).unwrap(),
        "Corner\tSignal\tIndex\tTime\tState\nnominal\tdout\t0\t2.500000e-10\tlow\nnominal\tdout\t1\t7.500000e-10\thigh\nnominal\tdout\t2\t1.500000e-09\tlow\ncout-large\tdout\t0\t2.500000e-10\tlow\n"
    );
}

#[test]
fn adaptive_digital_transient_bridge_samples_output_stream_and_formats_metadata() {
    let input_streams = [DigitalEventStream::new(
        "din",
        vec![
            DigitalEvent::new(0.0, DigitalState::Low),
            DigitalEvent::new(0.5e-9, DigitalState::High),
            DigitalEvent::new(1.25e-9, DigitalState::Low),
        ],
    )];
    let mut circuit = Circuit::new();
    circuit.add(Element::Resistor(Resistor::new(
        "Rout", "din", "out", 1_000.0,
    )));
    circuit.add(Element::Capacitor(Capacitor::new(
        "Cout", "out", "0", 0.1e-12,
    )));
    circuit.add(Element::Resistor(Resistor::new(
        "Rload", "out", "0", 10_000.0,
    )));

    let result = transient_adaptive_with_digital_event_streams(
        &circuit,
        &input_streams,
        "0",
        DigitalLogicLevels::cmos_1v8(0.25e-9),
        0.25e-9,
        1.5e-9,
        AdaptiveTransientOptions {
            method: TransientMethod::Euler,
            tolerance: 1.0e-3,
            min_step: Some(0.25e-9),
            max_step: Some(0.25e-9),
        },
        &[("dout", "V(out)")],
        DigitalThresholds::cmos_1v8(),
    )
    .unwrap();

    assert_eq!(result.result.method, TransientMethod::Euler);
    assert!(result.result.converged);
    assert_eq!(result.result.steps_rejected, 0);
    assert_eq!(result.output_streams.len(), 1);
    assert_eq!(result.output_streams[0].signal_name, "dout");
    assert_eq!(
        format_adaptive_digital_event_stream_table(&result).unwrap(),
        "Method\tStepsRejected\tConverged\tSignal\tIndex\tTime\tState\neuler\t0\ttrue\tdout\t0\t2.500000e-10\tlow\neuler\t0\ttrue\tdout\t1\t7.500000e-10\thigh\neuler\t0\ttrue\tdout\t2\t1.500000e-09\tlow\n"
    );
    assert!(result
        .result
        .points
        .iter()
        .any(|point| point.voltage("out").unwrap() > 1.2));
}

#[test]
fn adaptive_digital_transient_bridge_runs_named_corners_and_formats_stream_table() {
    let input_streams = [DigitalEventStream::new(
        "din",
        vec![
            DigitalEvent::new(0.0, DigitalState::Low),
            DigitalEvent::new(0.5e-9, DigitalState::High),
            DigitalEvent::new(1.25e-9, DigitalState::Low),
        ],
    )];
    let mut circuit = Circuit::new();
    circuit.add(Element::Resistor(Resistor::new(
        "Rout", "din", "out", 1_000.0,
    )));
    circuit.add(Element::Capacitor(Capacitor::new(
        "Cout", "out", "0", 0.1e-12,
    )));
    circuit.add(Element::Resistor(Resistor::new(
        "Rload", "out", "0", 10_000.0,
    )));
    let corners = [
        CornerSpec::new("nominal", vec![]),
        CornerSpec::new(
            "cout-large",
            vec![CornerOverride::new("Cout", "capacitance", 10.0e-12)],
        ),
    ];

    let result = transient_adaptive_with_digital_event_streams_corners(
        &circuit,
        &input_streams,
        "0",
        DigitalLogicLevels::cmos_1v8(0.25e-9),
        0.25e-9,
        1.5e-9,
        AdaptiveTransientOptions {
            method: TransientMethod::Euler,
            tolerance: 1.0e-3,
            min_step: Some(0.25e-9),
            max_step: Some(0.25e-9),
        },
        &[("dout", "V(out)")],
        DigitalThresholds::cmos_1v8(),
        &corners,
    )
    .unwrap();

    assert_eq!(result.points.len(), 2);
    assert_eq!(result.points[0].corner_name, "nominal");
    assert_eq!(result.points[1].corner_name, "cout-large");
    assert!(result.points[0].result.result.converged);
    assert!(result.points[1].result.result.converged);
    assert!(result.points[0]
        .result
        .result
        .points
        .iter()
        .any(|point| point.voltage("out").unwrap() > 1.2));
    assert!(result.points[1]
        .result
        .result
        .points
        .iter()
        .all(|point| point.voltage("out").unwrap() < 1.2));
    assert_eq!(
        format_corner_adaptive_digital_event_stream_table(&result).unwrap(),
        "Corner\tMethod\tStepsRejected\tConverged\tSignal\tIndex\tTime\tState\nnominal\teuler\t0\ttrue\tdout\t0\t2.500000e-10\tlow\nnominal\teuler\t0\ttrue\tdout\t1\t7.500000e-10\thigh\nnominal\teuler\t0\ttrue\tdout\t2\t1.500000e-09\tlow\ncout-large\teuler\t0\ttrue\tdout\t0\t2.500000e-10\tlow\n"
    );
}

#[test]
fn transient_probe_samples_back_to_digital_events() {
    let events = [
        DigitalEvent::new(0.0, DigitalState::Low),
        DigitalEvent::new(0.5e-9, DigitalState::High),
        DigitalEvent::new(1.25e-9, DigitalState::Low),
    ];
    let source = spice_engine::digital_events_to_voltage_source(
        "Vdin",
        "din",
        "0",
        &events,
        DigitalLogicLevels::cmos_1v8(0.25e-9),
    )
    .unwrap();
    let mut circuit = Circuit::new();
    circuit.add(Element::VoltageSource(source));
    circuit.add(Element::Resistor(Resistor::new(
        "Rload", "din", "0", 1_000.0,
    )));

    let points = transient(&circuit, 0.25e-9, 1.5e-9).unwrap();
    let sampled =
        sample_transient_probe_as_digital_events(&points, "V(din)", DigitalThresholds::cmos_1v8())
            .unwrap();

    assert_eq!(sampled.len(), 3);
    assert_eq!(sampled[0].state, DigitalState::Low);
    assert_close(sampled[0].time_seconds, 0.25e-9);
    assert_eq!(sampled[1].state, DigitalState::High);
    assert_close(sampled[1].time_seconds, 0.75e-9);
    assert_eq!(sampled[2].state, DigitalState::Low);
    assert_close(sampled[2].time_seconds, 1.5e-9);
}

#[test]
fn digital_event_text_output_table_is_stable() {
    let events = [
        DigitalEvent::new(0.25e-9, DigitalState::Low),
        DigitalEvent::new(0.75e-9, DigitalState::High),
        DigitalEvent::new(1.5e-9, DigitalState::Low),
    ];

    assert_eq!(
        format_digital_event_table(&events).unwrap(),
        "Index\tTime\tState\n0\t2.500000e-10\tlow\n1\t7.500000e-10\thigh\n2\t1.500000e-09\tlow\n"
    );
}

#[test]
fn sampled_digital_event_text_output_table_is_stable() {
    let events = [
        DigitalEvent::new(0.0, DigitalState::Low),
        DigitalEvent::new(0.5e-9, DigitalState::High),
        DigitalEvent::new(1.25e-9, DigitalState::Low),
    ];
    let source = spice_engine::digital_events_to_voltage_source(
        "Vdin",
        "din",
        "0",
        &events,
        DigitalLogicLevels::cmos_1v8(0.25e-9),
    )
    .unwrap();
    let mut circuit = Circuit::new();
    circuit.add(Element::VoltageSource(source));
    circuit.add(Element::Resistor(Resistor::new(
        "Rload", "din", "0", 1_000.0,
    )));

    let points = transient(&circuit, 0.25e-9, 1.5e-9).unwrap();
    let sampled =
        sample_transient_probe_as_digital_events(&points, "V(din)", DigitalThresholds::cmos_1v8())
            .unwrap();

    assert_eq!(
        format_digital_event_table(&sampled).unwrap(),
        "Index\tTime\tState\n0\t2.500000e-10\tlow\n1\t7.500000e-10\thigh\n2\t1.500000e-09\tlow\n"
    );
}

#[test]
fn named_digital_event_stream_text_output_table_is_stable() {
    let streams = [
        DigitalEventStream::new(
            "clk",
            vec![
                DigitalEvent::new(0.0, DigitalState::Low),
                DigitalEvent::new(0.5e-9, DigitalState::High),
                DigitalEvent::new(1.0e-9, DigitalState::Low),
            ],
        ),
        DigitalEventStream::new(
            "enable",
            vec![
                DigitalEvent::new(0.25e-9, DigitalState::Low),
                DigitalEvent::new(0.75e-9, DigitalState::High),
            ],
        ),
    ];

    assert_eq!(
        format_digital_event_stream_table(&streams).unwrap(),
        "Signal\tIndex\tTime\tState\nclk\t0\t0.000000e+00\tlow\nclk\t1\t5.000000e-10\thigh\nclk\t2\t1.000000e-09\tlow\nenable\t0\t2.500000e-10\tlow\nenable\t1\t7.500000e-10\thigh\n"
    );
}

#[test]
fn digital_event_stream_vcd_output_is_stable() {
    let streams = [
        DigitalEventStream::new(
            "clk",
            vec![
                DigitalEvent::new(0.0, DigitalState::Low),
                DigitalEvent::new(0.5e-9, DigitalState::High),
                DigitalEvent::new(1.0e-9, DigitalState::Low),
            ],
        ),
        DigitalEventStream::new(
            "enable",
            vec![
                DigitalEvent::new(0.25e-9, DigitalState::Low),
                DigitalEvent::new(0.75e-9, DigitalState::High),
            ],
        ),
    ];

    assert_eq!(
        spice_engine::format_digital_event_stream_vcd(&streams).unwrap(),
        "$version coding-adventures spice-engine mixed-signal bridge $end\n$timescale 1ps $end\n$scope module spice_bridge $end\n$var wire 1 s0 clk $end\n$var wire 1 s1 enable $end\n$upscope $end\n$enddefinitions $end\n$dumpvars\n0s0\n0s1\n$end\n#0\n0s0\n#250\n0s1\n#500\n1s0\n#750\n1s1\n#1000\n0s0\n"
    );
}

#[test]
fn sampled_named_digital_event_stream_text_output_table_is_stable() {
    let events = [
        DigitalEvent::new(0.0, DigitalState::Low),
        DigitalEvent::new(0.5e-9, DigitalState::High),
        DigitalEvent::new(1.25e-9, DigitalState::Low),
    ];
    let source = spice_engine::digital_events_to_voltage_source(
        "Vdin",
        "din",
        "0",
        &events,
        DigitalLogicLevels::cmos_1v8(0.25e-9),
    )
    .unwrap();
    let mut circuit = Circuit::new();
    circuit.add(Element::VoltageSource(source));
    circuit.add(Element::Resistor(Resistor::new(
        "Rload", "din", "0", 1_000.0,
    )));

    let points = transient(&circuit, 0.25e-9, 1.5e-9).unwrap();
    let sampled =
        sample_transient_probe_as_digital_events(&points, "V(din)", DigitalThresholds::cmos_1v8())
            .unwrap();
    let streams = [DigitalEventStream::new("din", sampled)];

    assert_eq!(
        format_digital_event_stream_table(&streams).unwrap(),
        "Signal\tIndex\tTime\tState\ndin\t0\t2.500000e-10\tlow\ndin\t1\t7.500000e-10\thigh\ndin\t2\t1.500000e-09\tlow\n"
    );
}

#[test]
fn multiple_transient_probes_sample_to_named_digital_event_streams() {
    let din_events = [
        DigitalEvent::new(0.0, DigitalState::Low),
        DigitalEvent::new(0.5e-9, DigitalState::High),
        DigitalEvent::new(1.25e-9, DigitalState::Low),
    ];
    let enable_events = [
        DigitalEvent::new(0.0, DigitalState::High),
        DigitalEvent::new(1.0e-9, DigitalState::Low),
    ];
    let levels = DigitalLogicLevels::cmos_1v8(0.25e-9);
    let mut circuit = Circuit::new();
    circuit.add(Element::VoltageSource(
        spice_engine::digital_events_to_voltage_source("Vdin", "din", "0", &din_events, levels)
            .unwrap(),
    ));
    circuit.add(Element::VoltageSource(
        spice_engine::digital_events_to_voltage_source(
            "Venable",
            "enable",
            "0",
            &enable_events,
            levels,
        )
        .unwrap(),
    ));
    circuit.add(Element::Resistor(Resistor::new(
        "Rdin", "din", "0", 1_000.0,
    )));
    circuit.add(Element::Resistor(Resistor::new(
        "Renable", "enable", "0", 1_000.0,
    )));

    let points = transient(&circuit, 0.25e-9, 1.5e-9).unwrap();
    let streams = sample_transient_probes_as_digital_event_streams(
        &points,
        &[("din", "V(din)"), ("enable", "V(enable)")],
        DigitalThresholds::cmos_1v8(),
    )
    .unwrap();

    assert_eq!(
        format_digital_event_stream_table(&streams).unwrap(),
        "Signal\tIndex\tTime\tState\ndin\t0\t2.500000e-10\tlow\ndin\t1\t7.500000e-10\thigh\ndin\t2\t1.500000e-09\tlow\nenable\t0\t2.500000e-10\thigh\nenable\t1\t1.250000e-09\tlow\n"
    );
}
