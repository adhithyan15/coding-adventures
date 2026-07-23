// The literal -1.5707963267948966 is hand-written test data (a specific phase in
// degrees), not an approximation to FRAC_PI_2 that clippy should replace.
#![allow(clippy::approx_constant)]

use std::collections::BTreeMap;

use spice_engine::{
    dc_op, deck_output_plan_artifact_records, deck_table_records,
    device_model_charge_audit_fixtures, digital_event_streams_to_voltage_sources,
    distortion_from_fourier, distortion_from_transient, distortion_from_transient_corners,
    estimate_period, format_adaptive_digital_event_stream_table, format_adaptive_transient_table,
    format_corner_adaptive_digital_event_stream_table, format_corner_adaptive_transient_table,
    format_corner_digital_event_stream_table, format_corner_distortion_table,
    format_corner_fourier_table, format_corner_pole_zero_table, format_corner_pss_table,
    format_corner_transient_table, format_dc_table, format_deck_control_policy_artifact_csv,
    format_deck_control_policy_artifact_json, format_deck_control_policy_artifact_table,
    format_deck_control_policy_summary_artifact_csv,
    format_deck_control_policy_summary_artifact_json,
    format_deck_control_policy_summary_artifact_table, format_deck_noise_table,
    format_deck_output_plan_artifact_csv, format_deck_output_plan_artifact_json,
    format_deck_output_plan_artifact_table, format_deck_rawfile_artifact_csv,
    format_deck_rawfile_artifact_json, format_deck_rawfile_artifact_table,
    format_deck_run_artifact_csv, format_deck_run_artifact_json, format_deck_run_artifact_table,
    format_deck_table_csv, format_deck_table_json, format_deck_transient_table,
    format_deck_wrdata_artifact_csv, format_deck_wrdata_artifact_json,
    format_deck_wrdata_artifact_table, format_deck_wrdata_ascii,
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
    pss_with_tolerance, run_deck, run_deck_analysis, sample_transient_probe_as_digital_events,
    sample_transient_probes_as_digital_event_streams, transient, transient_adaptive,
    transient_adaptive_corners, transient_adaptive_with_digital_event_streams,
    transient_adaptive_with_digital_event_streams_corners, transient_corners,
    transient_with_digital_event_streams, transient_with_digital_event_streams_corners,
    transient_with_method, AdaptiveTransientOptions, AdaptiveTransientResult, Bjt, BjtPolarity,
    Capacitor, Cccs, Ccvs, Circuit, CornerDistortionPoint, CornerDistortionResult, CornerOverride,
    CornerSpec, CurrentSource, DeckAnalysisExecution, DeckAnalysisExecutionResult,
    DigitalBridgeSchedule, DigitalEvent, DigitalEventStream, DigitalLogicLevels, DigitalState,
    DigitalThresholds, Diode, DistortionHarmonic, DistortionPoint, DistortionResult, Element,
    ExpWaveform, FourierHarmonic, FourierProbeResult, FourierResult, Inductor, Jfet, JfetPolarity,
    Mosfet, MosfetLevel1Params, MosfetType, MutualInductor, PoleZeroEntry, PoleZeroEntryKind,
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

fn assert_run_artifact_table_matches(
    execution: &DeckAnalysisExecution,
) -> BTreeMap<String, String> {
    assert_eq!(
        execution.run_artifact_table,
        format_deck_run_artifact_table(&execution.run_artifacts)
    );
    let records = deck_table_records(&execution.run_artifact_table);
    assert_eq!(records.len(), 1);
    records[0].clone()
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
fn device_model_charge_audit_fixtures_run_reference_transients() {
    let fixtures = device_model_charge_audit_fixtures().unwrap();
    assert_eq!(
        fixtures
            .iter()
            .map(|fixture| fixture.name.as_str())
            .collect::<Vec<_>>(),
        vec![
            "diode-storage-charge",
            "bjt-storage-charge",
            "jfet-storage-charge",
            "mos-level1-storage-charge"
        ]
    );

    for fixture in &fixtures {
        let points = transient(&fixture.circuit, fixture.time_step_s, fixture.stop_time_s).unwrap();
        assert!(!points.is_empty());
        let initial = points[0]
            .voltage(&fixture.probe_node)
            .expect("fixture initial probe node should be present");
        let final_value = points
            .last()
            .and_then(|point| point.voltage(&fixture.probe_node))
            .expect("fixture final probe node should be present");
        assert!(
            fixture.expected_initial_min <= initial && initial <= fixture.expected_initial_max,
            "{} expected {} <= initial {} <= {}",
            fixture.name,
            fixture.expected_initial_min,
            initial,
            fixture.expected_initial_max
        );
        assert!(
            fixture.expected_final_min <= final_value && final_value <= fixture.expected_final_max,
            "{} expected {} <= final {} <= {}",
            fixture.name,
            fixture.expected_final_min,
            final_value,
            fixture.expected_final_max
        );
        assert!(fixture.storage_capacitance_f > 0.0);
        assert!(fixture.deck_lines[0].starts_with("* device-model charge fixture:"));
        assert!(fixture
            .deck_lines
            .iter()
            .any(|line| line.starts_with(".model ")));
        assert!(fixture
            .deck_lines
            .iter()
            .any(|line| line.starts_with(".tran ")));
        assert!(!fixture.charge_behavior.is_empty());
    }

    let jfet_fixture = fixtures
        .iter()
        .find(|fixture| fixture.kind.as_str() == "NJF")
        .expect("expected JFET charge fixture");
    assert!(jfet_fixture.charge_behavior.contains("CGS/CGD"));
    let mos_fixture = fixtures
        .iter()
        .find(|fixture| fixture.kind.as_str() == "NMOS")
        .expect("expected MOS charge fixture");
    assert!(mos_fixture.charge_behavior.contains("CGSO/CGDO/CGBO"));
    assert!(mos_fixture.charge_behavior.contains("CBS/CBD"));
}

#[test]
fn transient_diode_junction_capacitance_slows_current_step() {
    fn run(junction_capacitance: f64) -> Vec<TransientPoint> {
        let mut circuit = Circuit::new();
        circuit.add(Element::CurrentSource(CurrentSource::with_waveform(
            "Istep",
            "0",
            "out",
            0.0,
            Waveform::Pwl(PwlWaveform::new(vec![
                (0.0, 0.0),
                (1.0e-9, 1.0e-6),
                (5.0e-9, 1.0e-6),
            ])),
        )));
        circuit.add(Element::Resistor(Resistor::new(
            "Rshunt", "out", "0", 1.0e12,
        )));
        circuit.add(Element::Diode(Diode::with_model_and_breakdown(
            "D1",
            "out",
            "0",
            1.0e-15,
            0.02585,
            1.0,
            None,
            1.0e-3,
            junction_capacitance,
            0.0,
        )));
        transient(&circuit, 1.0e-9, 5.0e-9).unwrap()
    }

    let uncharged = run(0.0);
    let charged = run(1.0e-12);
    let uncharged_first = uncharged[0].voltage("out").unwrap();
    let charged_first = charged[0].voltage("out").unwrap();

    assert!(uncharged_first > 0.5);
    assert!(charged_first < 0.01);
    assert!(charged_first < uncharged_first);
}

#[test]
fn transient_jfet_gate_source_capacitance_slows_gate_step() {
    fn run(gate_source_capacitance: f64) -> Vec<TransientPoint> {
        let mut circuit = Circuit::new();
        circuit.add(Element::VoltageSource(VoltageSource::with_waveform(
            "Vstep",
            "in",
            "0",
            0.0,
            Waveform::Pwl(PwlWaveform::new(vec![
                (0.0, 0.0),
                (1.0e-9, 1.0),
                (5.0e-9, 1.0),
            ])),
        )));
        circuit.add(Element::Resistor(Resistor::new(
            "Rin", "in", "gate", 1_000.0,
        )));
        circuit.add(Element::Resistor(Resistor::new(
            "Rdrain", "drain", "0", 1_000.0,
        )));
        circuit.add(Element::Jfet(Jfet::with_model_and_capacitance(
            "J1",
            "drain",
            "gate",
            "0",
            JfetPolarity::Njf,
            1.0e-12,
            -2.0,
            0.0,
            gate_source_capacitance,
            0.0,
        )));
        transient_with_method(&circuit, 1.0e-9, 5.0e-9, TransientMethod::Euler).unwrap()
    }

    let uncharged = run(0.0);
    let charged = run(1.0e-9);
    let uncharged_first = uncharged[0].voltage("gate").unwrap();
    let charged_first = charged[0].voltage("gate").unwrap();

    assert!(uncharged_first > 0.5);
    assert!(charged_first < 0.01);
    assert!(charged_first < uncharged_first);
}

#[test]
fn transient_mosfet_overlap_capacitance_slows_gate_step() {
    fn run(gate_source_overlap_capacitance: f64) -> Vec<TransientPoint> {
        let mut circuit = Circuit::new();
        circuit.add(Element::VoltageSource(VoltageSource::with_waveform(
            "Vstep",
            "in",
            "0",
            0.0,
            Waveform::Pwl(PwlWaveform::new(vec![
                (0.0, 0.0),
                (1.0e-9, 1.0),
                (5.0e-9, 1.0),
            ])),
        )));
        circuit.add(Element::Resistor(Resistor::new(
            "Rin", "in", "gate", 1_000.0,
        )));
        circuit.add(Element::Resistor(Resistor::new(
            "Rdrain", "drain", "0", 1_000.0,
        )));
        circuit.add(Element::Mosfet(Mosfet::with_model(
            "M1",
            "drain",
            "gate",
            "0",
            "0",
            MosfetType::Nmos,
            MosfetLevel1Params {
                kp: 1.0e-12,
                w: 1.0,
                l: 1.0,
                gate_source_overlap_capacitance,
                ..MosfetLevel1Params::default()
            },
        )));
        transient_with_method(&circuit, 1.0e-9, 5.0e-9, TransientMethod::Euler).unwrap()
    }

    let uncharged = run(0.0);
    let charged = run(1.0e-9);
    let uncharged_first = uncharged[0].voltage("gate").unwrap();
    let charged_first = charged[0].voltage("gate").unwrap();

    assert!(uncharged_first > 0.5);
    assert!(charged_first < 0.01);
    assert!(charged_first < uncharged_first);
}

#[test]
fn transient_mosfet_bulk_junction_capacitance_slows_drain_step() {
    fn run(drain_bulk_capacitance: f64) -> Vec<TransientPoint> {
        let mut circuit = Circuit::new();
        circuit.add(Element::VoltageSource(VoltageSource::with_waveform(
            "Vstep",
            "in",
            "0",
            0.0,
            Waveform::Pwl(PwlWaveform::new(vec![
                (0.0, 0.0),
                (1.0e-9, 1.0),
                (5.0e-9, 1.0),
            ])),
        )));
        circuit.add(Element::Resistor(Resistor::new(
            "Rin", "in", "drain", 1_000.0,
        )));
        circuit.add(Element::Mosfet(Mosfet::with_model(
            "M1",
            "drain",
            "0",
            "0",
            "0",
            MosfetType::Nmos,
            MosfetLevel1Params {
                kp: 1.0e-12,
                w: 1.0,
                l: 1.0,
                drain_bulk_capacitance,
                ..MosfetLevel1Params::default()
            },
        )));
        transient_with_method(&circuit, 1.0e-9, 5.0e-9, TransientMethod::Euler).unwrap()
    }

    let uncharged = run(0.0);
    let charged = run(1.0e-9);
    let uncharged_first = uncharged[0].voltage("drain").unwrap();
    let charged_first = charged[0].voltage("drain").unwrap();

    assert!(uncharged_first > 0.5);
    assert!(charged_first < 0.01);
    assert!(charged_first < uncharged_first);
}

#[test]
fn transient_mosfet_bulk_junction_depletion_shaping_reduces_reverse_bias_capacitance() {
    fn run(grading_coefficient: f64) -> Vec<TransientPoint> {
        let mut circuit = Circuit::new();
        circuit.add(Element::VoltageSource(VoltageSource::with_waveform(
            "Vstep",
            "in",
            "0",
            1.0,
            Waveform::Pwl(PwlWaveform::new(vec![
                (0.0, 1.0),
                (1.0e-9, 2.0),
                (5.0e-9, 2.0),
            ])),
        )));
        circuit.add(Element::Resistor(Resistor::new(
            "Rin", "in", "drain", 1_000.0,
        )));
        circuit.add(Element::Mosfet(Mosfet::with_model(
            "M1",
            "drain",
            "0",
            "0",
            "0",
            MosfetType::Nmos,
            MosfetLevel1Params {
                kp: 1.0e-12,
                w: 1.0,
                l: 1.0,
                drain_bulk_capacitance: 1.0e-12,
                bulk_junction_potential: 1.0,
                bulk_junction_grading_coefficient: grading_coefficient,
                ..MosfetLevel1Params::default()
            },
        )));
        transient_with_method(&circuit, 1.0e-9, 5.0e-9, TransientMethod::Euler).unwrap()
    }

    let fixed = run(0.0);
    let shaped = run(0.5);
    let fixed_first = fixed[0].voltage("drain").unwrap();
    let shaped_first = shaped[0].voltage("drain").unwrap();

    assert!(
        (fixed_first - 1.25).abs() < 0.08,
        "expected fixed first step near 1.25 V, got fixed={fixed_first}, shaped={shaped_first}"
    );
    assert!(shaped_first > fixed_first + 0.04);
    assert!(shaped_first < 1.4);
}

#[test]
fn transient_diode_transit_time_holds_forward_charge_on_turnoff() {
    fn run(transit_time: f64) -> Vec<TransientPoint> {
        let mut circuit = Circuit::new();
        circuit.add(Element::CurrentSource(CurrentSource::with_waveform(
            "Istep",
            "0",
            "out",
            0.0,
            Waveform::Pwl(PwlWaveform::new(vec![
                (0.0, 1.0e-3),
                (1.0e-9, 0.0),
                (5.0e-9, 0.0),
            ])),
        )));
        circuit.add(Element::Resistor(Resistor::new(
            "Rshunt", "out", "0", 1.0e12,
        )));
        circuit.add(Element::Diode(Diode::with_model_and_breakdown(
            "D1",
            "out",
            "0",
            1.0e-15,
            0.02585,
            1.0,
            None,
            1.0e-3,
            0.0,
            transit_time,
        )));
        transient(&circuit, 1.0e-9, 5.0e-9).unwrap()
    }

    let no_storage = run(0.0);
    let stored = run(1.0e-9);
    let no_storage_first = no_storage[0].voltage("out").unwrap();
    let stored_first = stored[0].voltage("out").unwrap();
    let stored_last = stored
        .last()
        .and_then(|point| point.voltage("out"))
        .unwrap();

    assert_close(no_storage_first, 0.0);
    assert!(stored_first > 0.6);
    assert!(stored_last < stored_first);
}

#[test]
fn transient_bjt_base_emitter_capacitance_slows_base_current_step() {
    fn run(base_emitter_capacitance: f64) -> Vec<TransientPoint> {
        let mut circuit = Circuit::new();
        circuit.add(Element::VoltageSource(VoltageSource::new(
            "Vcc",
            "collector",
            "0",
            5.0,
        )));
        circuit.add(Element::CurrentSource(CurrentSource::with_waveform(
            "Istep",
            "0",
            "base",
            0.0,
            Waveform::Pwl(PwlWaveform::new(vec![
                (0.0, 0.0),
                (1.0e-9, 1.0e-6),
                (5.0e-9, 1.0e-6),
            ])),
        )));
        circuit.add(Element::Resistor(Resistor::new(
            "Rshunt", "base", "0", 1.0e12,
        )));
        circuit.add(Element::Bjt(Bjt::with_model(
            "Q1",
            "collector",
            "base",
            "0",
            BjtPolarity::Npn,
            1.0e-15,
            100.0,
            0.02585,
            base_emitter_capacitance,
            0.0,
            0.0,
            0.0,
        )));
        transient(&circuit, 1.0e-9, 5.0e-9).unwrap()
    }

    let uncharged = run(0.0);
    let charged = run(1.0e-12);
    let uncharged_first = uncharged[0].voltage("base").unwrap();
    let charged_first = charged[0].voltage("base").unwrap();

    assert!(uncharged_first > 0.5);
    assert!(charged_first < 0.01);
    assert!(charged_first < uncharged_first);
}

#[test]
fn transient_bjt_base_emitter_depletion_capacitance_falls_with_reverse_bias() {
    fn stepped_base_voltage(grading_coefficient: f64) -> f64 {
        let mut circuit = Circuit::new();
        circuit.add(Element::VoltageSource(VoltageSource::with_waveform(
            "Vdrive",
            "in",
            "0",
            -1.0,
            Waveform::Pwl(PwlWaveform::new(vec![
                (0.0, -1.0),
                (1.0e-9, -1.0),
                (2.0e-9, 0.0),
                (5.0e-9, 0.0),
            ])),
        )));
        circuit.add(Element::Resistor(Resistor::new(
            "Rin", "in", "base", 1_000.0,
        )));
        circuit.add(Element::Bjt(
            Bjt::with_model_temperature_and_depletion_parameters(
                "Q1",
                "0",
                "base",
                "0",
                BjtPolarity::Npn,
                1.0e-14,
                100.0,
                0.02585,
                1.0e-12,
                0.0,
                0.0,
                0.0,
                3.0,
                1.11,
                0.0,
                1.0,
                1.0,
                0.75,
                grading_coefficient,
                0.75,
                0.33,
                0.5,
            ),
        ));
        transient(&circuit, 1.0e-9, 5.0e-9).unwrap()[1]
            .voltage("base")
            .unwrap()
    }

    assert!(stepped_base_voltage(0.5) > stepped_base_voltage(0.0));
}

#[test]
fn transient_bjt_base_collector_depletion_capacitance_falls_with_reverse_bias() {
    fn stepped_collector_voltage(grading_coefficient: f64) -> f64 {
        let mut circuit = Circuit::new();
        circuit.add(Element::VoltageSource(VoltageSource::with_waveform(
            "Vdrive",
            "in",
            "0",
            1.0,
            Waveform::Pwl(PwlWaveform::new(vec![
                (0.0, 1.0),
                (1.0e-9, 1.0),
                (2.0e-9, 0.0),
                (5.0e-9, 0.0),
            ])),
        )));
        circuit.add(Element::Resistor(Resistor::new(
            "Rin",
            "in",
            "collector",
            1_000.0,
        )));
        circuit.add(Element::Bjt(
            Bjt::with_model_temperature_and_depletion_parameters(
                "Q1",
                "collector",
                "0",
                "0",
                BjtPolarity::Npn,
                1.0e-14,
                100.0,
                0.02585,
                0.0,
                1.0e-12,
                0.0,
                0.0,
                3.0,
                1.11,
                0.0,
                1.0,
                1.0,
                0.75,
                0.33,
                0.75,
                grading_coefficient,
                0.5,
            ),
        ));
        transient(&circuit, 1.0e-9, 5.0e-9).unwrap()[1]
            .voltage("collector")
            .unwrap()
    }

    assert!(stepped_collector_voltage(0.5) < stepped_collector_voltage(0.0));
}

#[test]
fn transient_bjt_forward_bias_depletion_coefficient_shapes_both_junctions() {
    fn held_voltage(coefficient: f64, base_emitter: bool) -> f64 {
        let mut circuit = Circuit::new();
        circuit.add(Element::VoltageSource(VoltageSource::with_waveform(
            "Vdrive",
            "in",
            "0",
            0.6,
            Waveform::Pwl(PwlWaveform::new(vec![
                (0.0, 0.6),
                (1.0e-9, 0.6),
                (2.0e-9, 0.0),
                (5.0e-9, 0.0),
            ])),
        )));
        circuit.add(Element::Resistor(Resistor::new(
            "Rin", "in", "base", 1_000.0,
        )));
        circuit.add(Element::Bjt(
            Bjt::with_model_temperature_and_depletion_parameters(
                "Q1",
                "0",
                "base",
                "0",
                BjtPolarity::Npn,
                1.0e-30,
                100.0,
                0.02585,
                if base_emitter { 1.0e-12 } else { 0.0 },
                if base_emitter { 0.0 } else { 1.0e-12 },
                0.0,
                0.0,
                3.0,
                1.11,
                0.0,
                1.0,
                1.0,
                0.75,
                0.33,
                0.75,
                0.33,
                coefficient,
            ),
        ));
        transient(&circuit, 1.0e-9, 5.0e-9).unwrap()[1]
            .voltage("base")
            .unwrap()
    }

    for base_emitter in [true, false] {
        assert!(held_voltage(0.8, base_emitter) > held_voltage(0.2, base_emitter));
    }
}

#[test]
fn transient_bjt_forward_transit_time_holds_base_charge_on_turnoff() {
    fn run(
        forward_transit_time: f64,
        forward_transit_time_bias_coefficient: f64,
        forward_transit_time_current: f64,
        forward_transit_time_voltage: f64,
        collector_voltage: f64,
    ) -> Vec<TransientPoint> {
        let mut circuit = Circuit::new();
        circuit.add(Element::VoltageSource(VoltageSource::new(
            "Vcc",
            "collector",
            "0",
            collector_voltage,
        )));
        circuit.add(Element::CurrentSource(CurrentSource::with_waveform(
            "Istep",
            "0",
            "base",
            0.0,
            Waveform::Pwl(PwlWaveform::new(vec![
                (0.0, 1.0e-3),
                (1.0e-9, 0.0),
                (5.0e-9, 0.0),
            ])),
        )));
        circuit.add(Element::Resistor(Resistor::new(
            "Rshunt", "base", "0", 1.0e12,
        )));
        let mut transistor = Bjt::with_model(
            "Q1",
            "collector",
            "base",
            "0",
            BjtPolarity::Npn,
            1.0e-15,
            100.0,
            0.02585,
            0.0,
            0.0,
            forward_transit_time,
            0.0,
        );
        transistor.forward_transit_time_bias_coefficient = forward_transit_time_bias_coefficient;
        transistor.forward_transit_time_current = forward_transit_time_current;
        transistor.forward_transit_time_voltage = forward_transit_time_voltage;
        circuit.add(Element::Bjt(transistor));
        transient(&circuit, 1.0e-9, 5.0e-9).unwrap()
    }

    let no_storage = run(0.0, 0.0, 0.0, 0.0, 5.0);
    let stored = run(1.0e-9, 0.0, 0.0, 0.0, 5.0);
    let bias_scaled = run(1.0e-9, 9.0, 0.0, 0.0, 5.0);
    let current_limited = run(1.0e-9, 9.0, 1.0, 0.0, 5.0);
    let voltage_limited = run(1.0e-9, 9.0, 0.0, 0.5, 10.0);
    let no_storage_first = no_storage[0].voltage("base").unwrap();
    let stored_first = stored[0].voltage("base").unwrap();

    assert_close(no_storage_first, 0.0);
    assert!(stored_first > 0.6);
    assert!(
        stored
            .last()
            .and_then(|point| point.voltage("base"))
            .unwrap()
            < stored_first
    );
    let bias_scaled_last = bias_scaled
        .last()
        .and_then(|point| point.voltage("base"))
        .unwrap();
    let stored_last = stored
        .last()
        .and_then(|point| point.voltage("base"))
        .unwrap();
    assert!((bias_scaled_last - stored_last).abs() > 1.0e-12);
    let current_limited_last = current_limited
        .last()
        .and_then(|point| point.voltage("base"))
        .unwrap();
    assert!((current_limited_last - stored_last).abs() < (bias_scaled_last - stored_last).abs());
    let voltage_limited_last = voltage_limited
        .last()
        .and_then(|point| point.voltage("base"))
        .unwrap();
    assert!((voltage_limited_last - stored_last).abs() < (bias_scaled_last - stored_last).abs());
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
fn deck_wrdata_ascii_selects_marker_probe_columns() {
    let table =
        "Index\tV(in)\tI(V1)\n0\t1.000000e+00\t-1.000000e-03\n1\t2.000000e+00\t-2.000000e-03\n";
    assert_eq!(
        format_deck_wrdata_ascii(
            table,
            &["I(V1)".to_string()],
            &[
                "set wr_vecnames".to_string(),
                "set wr_singlescale".to_string()
            ],
        ),
        "# SPICE deck wrdata artifact\n\
Probes: I(V1)\n\
Options: set wr_vecnames;set wr_singlescale\n\
VectorNames: Index;I(V1)\n\
Scale: Index\n\
Index\tI(V1)\n\
0\t-1.000000e-03\n\
1\t-2.000000e-03\n"
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
.print dc V(mid)
.plot ac V(mid)
.op
.dc V1 0 1 1
.ac dec 1 1k 1k
.tran 1m 1m
.tf V(mid) V1
.sens V(mid)
.noise V(mid) V1 lin 1 1k 1k
.measure dc mid_avg avg V(mid)
.measure ac mid_peak max V(mid)
.measure tran mid_final final V(mid)
.end
";
    let save_line = netlist
        .lines()
        .position(|line| line.trim_start().starts_with(".save"))
        .unwrap()
        + 1;
    let probe_dc_line = netlist
        .lines()
        .position(|line| line.trim_start().starts_with(".probe dc"))
        .unwrap()
        + 1;
    let print_dc_line = netlist
        .lines()
        .position(|line| line.trim_start().starts_with(".print dc"))
        .unwrap()
        + 1;
    let plot_ac_line = netlist
        .lines()
        .position(|line| line.trim_start().starts_with(".plot ac"))
        .unwrap()
        + 1;

    let op_execution = run_deck_analysis(&circuit, netlist, Some("op")).unwrap();
    assert_eq!(op_execution.plan.analysis, "op");
    assert_eq!(op_execution.output_probes, vec!["V(mid)".to_string()]);
    assert!(op_execution.measurements.is_empty());
    assert_eq!(
        op_execution.measurement_table,
        "Name\tAnalysis\tProbe\tMode\tFrom\tTo\tValue\n"
    );
    assert_eq!(op_execution.table, "Index\tV(mid)\n0\t5.000000e-01\n");
    assert_eq!(op_execution.output_directives, vec![".save".to_string()]);
    assert_eq!(op_execution.analysis_directives, vec![".op".to_string()]);
    assert_eq!(op_execution.table_count, 3);
    assert_eq!(
        op_execution.tables,
        vec![
            "result".to_string(),
            "output-plan".to_string(),
            "run-artifact".to_string()
        ]
    );
    assert_eq!(
        format_deck_table_csv(&op_execution.table),
        "Index,V(mid)\n0,5.000000e-01\n"
    );
    assert_eq!(
        format_deck_table_json(&op_execution.table),
        "[{\"Index\":\"0\",\"V(mid)\":\"5.000000e-01\"}]\n"
    );
    let op_records = deck_table_records(&op_execution.table);
    assert_eq!(op_records.len(), 1);
    assert_eq!(op_records[0].get("Index").map(String::as_str), Some("0"));
    assert_eq!(
        op_records[0].get("V(mid)").map(String::as_str),
        Some("5.000000e-01")
    );
    assert_eq!(
        op_execution
            .table_artifacts
            .iter()
            .map(|artifact| artifact.name.as_str())
            .collect::<Vec<_>>(),
        vec!["result", "output-plan", "run-artifact"]
    );
    assert_eq!(op_execution.table_artifacts[0].table, op_execution.table);
    assert_eq!(
        op_execution.table_artifacts[0].csv,
        format_deck_table_csv(&op_execution.table)
    );
    assert_eq!(
        op_execution.table_artifacts[0].json,
        format_deck_table_json(&op_execution.table)
    );
    assert_eq!(&op_execution.table_artifacts[0].records, &op_records);
    assert_eq!(op_execution.output_plan_artifact_count, 1);
    let output_plan_artifact = &op_execution.output_plan_artifacts[0];
    assert_eq!(output_plan_artifact.analysis, "op");
    assert_eq!(output_plan_artifact.directive, ".op");
    assert_eq!(
        output_plan_artifact.line_number,
        op_execution.plan.line_number
    );
    assert_eq!(output_plan_artifact.source_name, None);
    assert_eq!(output_plan_artifact.output_node, None);
    assert_eq!(output_plan_artifact.sweep_kind, None);
    assert_eq!(output_plan_artifact.start_value, None);
    assert_eq!(output_plan_artifact.stop_value, None);
    assert_eq!(output_plan_artifact.step_value, None);
    assert_eq!(output_plan_artifact.point_count, None);
    assert_eq!(output_plan_artifact.start_frequency_hz, None);
    assert_eq!(output_plan_artifact.stop_frequency_hz, None);
    assert_eq!(output_plan_artifact.step_time, None);
    assert_eq!(output_plan_artifact.stop_time, None);
    assert_eq!(output_plan_artifact.start_time, None);
    assert_eq!(output_plan_artifact.max_step, None);
    assert_eq!(output_plan_artifact.use_initial_conditions, None);
    assert_eq!(output_plan_artifact.result_row_count, 1);
    assert_eq!(
        output_plan_artifact.result_columns,
        vec!["Index".to_string(), "V(mid)".to_string()]
    );
    assert_eq!(
        output_plan_artifact.output_probes,
        vec!["V(mid)".to_string()]
    );
    assert_eq!(output_plan_artifact.output_probe_line_count, 1);
    assert_eq!(output_plan_artifact.output_probe_lines, vec![save_line]);
    assert_eq!(
        output_plan_artifact.output_directives,
        vec![".save".to_string()]
    );
    assert_eq!(output_plan_artifact.output_directive_kind_count, 1);
    assert_eq!(
        output_plan_artifact.output_directive_kinds,
        vec!["save".to_string()]
    );
    assert_eq!(output_plan_artifact.output_directive_analysis_kind_count, 1);
    assert_eq!(
        output_plan_artifact.output_directive_analysis_kinds,
        vec!["global".to_string()]
    );
    assert_eq!(output_plan_artifact.output_directive_line_count, 1);
    assert_eq!(output_plan_artifact.output_directive_lines, vec![save_line]);
    assert_eq!(
        output_plan_artifact.tables,
        vec![
            "result".to_string(),
            "output-plan".to_string(),
            "run-artifact".to_string()
        ]
    );
    let expected_output_plan_columns = [
        "Analysis",
        "Directive",
        "Line",
        "SourceName",
        "OutputNode",
        "SweepKind",
        "StartValue",
        "StopValue",
        "StepValue",
        "PointCount",
        "StartFrequencyHz",
        "StopFrequencyHz",
        "StepTime",
        "StopTime",
        "StartTime",
        "MaxStep",
        "UseInitialConditions",
        "ResultRows",
        "ResultColumns",
        "ResultColumnList",
        "OutputProbes",
        "OutputProbeList",
        "OutputProbeLines",
        "OutputProbeLineList",
        "OutputDirectives",
        "OutputDirectiveList",
        "OutputDirectiveKinds",
        "OutputDirectiveKindList",
        "OutputDirectiveAnalysisKinds",
        "OutputDirectiveAnalysisKindList",
        "OutputDirectiveLines",
        "OutputDirectiveLineList",
        "Tables",
        "TableList",
    ];
    let expected_output_plan_row = vec![
        "op".to_string(),
        ".op".to_string(),
        op_execution.plan.line_number.to_string(),
        String::new(),
        String::new(),
        String::new(),
        String::new(),
        String::new(),
        String::new(),
        String::new(),
        String::new(),
        String::new(),
        String::new(),
        String::new(),
        String::new(),
        String::new(),
        String::new(),
        "1".to_string(),
        "2".to_string(),
        "Index;V(mid)".to_string(),
        "1".to_string(),
        "V(mid)".to_string(),
        "1".to_string(),
        save_line.to_string(),
        "1".to_string(),
        ".save".to_string(),
        "1".to_string(),
        "save".to_string(),
        "1".to_string(),
        "global".to_string(),
        "1".to_string(),
        save_line.to_string(),
        "3".to_string(),
        "result;output-plan;run-artifact".to_string(),
    ];
    assert_eq!(
        op_execution.output_plan_artifact_table,
        format!(
            "{}\n{}\n",
            expected_output_plan_columns.join("\t"),
            expected_output_plan_row.join("\t")
        )
    );
    assert_eq!(
        op_execution.output_plan_artifact_table,
        format_deck_output_plan_artifact_table(&op_execution.output_plan_artifacts)
    );
    assert_eq!(
        op_execution.output_plan_artifact_csv,
        format!(
            "{}\n{}\n",
            expected_output_plan_columns.join(","),
            expected_output_plan_row.join(",")
        )
    );
    assert_eq!(
        op_execution.output_plan_artifact_csv,
        format_deck_output_plan_artifact_csv(&op_execution.output_plan_artifacts)
    );
    assert_eq!(
        op_execution.output_plan_artifact_json,
        format_deck_output_plan_artifact_json(&op_execution.output_plan_artifacts)
    );
    assert_eq!(
        op_execution.output_plan_artifact_records,
        deck_output_plan_artifact_records(&op_execution.output_plan_artifacts)
    );
    let op_line = op_execution.plan.line_number.to_string();
    assert_eq!(
        op_execution.output_plan_artifact_records[0]
            .get("Line")
            .map(String::as_str),
        Some(op_line.as_str())
    );
    assert_eq!(
        op_execution.output_plan_artifact_records[0]
            .get("OutputNode")
            .map(String::as_str),
        Some("")
    );
    assert_eq!(
        op_execution.output_plan_artifact_records[0]
            .get("SweepKind")
            .map(String::as_str),
        Some("")
    );
    assert_eq!(
        op_execution.output_plan_artifact_records[0]
            .get("StepTime")
            .map(String::as_str),
        Some("")
    );
    assert_eq!(
        op_execution.output_plan_artifact_records[0]
            .get("UseInitialConditions")
            .map(String::as_str),
        Some("")
    );
    assert_eq!(
        op_execution.output_plan_artifact_records[0]
            .get("SourceName")
            .map(String::as_str),
        Some("")
    );
    assert_eq!(
        op_execution.output_plan_artifact_records[0]
            .get("ResultRows")
            .map(String::as_str),
        Some("1")
    );
    assert_eq!(
        op_execution.output_plan_artifact_records[0]
            .get("ResultColumnList")
            .map(String::as_str),
        Some("Index;V(mid)")
    );
    assert_eq!(
        op_execution.output_plan_artifact_records[0]
            .get("OutputDirectiveAnalysisKindList")
            .map(String::as_str),
        Some("global")
    );
    let save_line_list = save_line.to_string();
    assert_eq!(
        op_execution.output_plan_artifact_records[0]
            .get("OutputProbeLineList")
            .map(String::as_str),
        Some(save_line_list.as_str())
    );
    assert_eq!(
        op_execution.output_plan_artifact_records[0]
            .get("OutputDirectiveLineList")
            .map(String::as_str),
        Some(save_line_list.as_str())
    );
    assert_eq!(
        op_execution.output_plan_artifact_records[0]
            .get("OutputDirectiveList")
            .map(String::as_str),
        Some(".save")
    );
    assert_eq!(
        op_execution.output_plan_artifact_records[0]
            .get("OutputDirectiveKindList")
            .map(String::as_str),
        Some("save")
    );
    assert_eq!(
        op_execution.output_plan_artifact_records[0]
            .get("TableList")
            .map(String::as_str),
        Some("result;output-plan;run-artifact")
    );
    assert_eq!(op_execution.run_artifacts[0].result_rows, 1);
    assert_eq!(op_execution.run_artifacts[0].result_column_count, 2);
    assert_eq!(
        op_execution.run_artifacts[0].result_columns,
        vec!["Index".to_string(), "V(mid)".to_string()]
    );
    assert_eq!(op_execution.run_artifacts[0].table_count, 3);
    assert_eq!(
        op_execution.run_artifacts[0].tables,
        vec![
            "result".to_string(),
            "output-plan".to_string(),
            "run-artifact".to_string()
        ]
    );
    assert_eq!(op_execution.run_artifacts[0].source_name, None);
    assert_eq!(op_execution.run_artifacts[0].output_node, None);
    assert_eq!(op_execution.run_artifacts[0].sweep_kind, None);
    assert_eq!(op_execution.run_artifacts[0].point_count, None);
    assert_eq!(op_execution.run_artifacts[0].step_time, None);
    assert_eq!(op_execution.run_artifacts[0].use_initial_conditions, None);
    assert_eq!(
        op_execution.run_artifacts[0].output_probes,
        vec!["V(mid)".to_string()]
    );
    assert_eq!(
        op_execution.run_artifacts[0].output_directives,
        vec![".save".to_string()]
    );
    assert_eq!(op_execution.run_artifacts[0].analysis_directive_count, 1);
    assert_eq!(
        op_execution.run_artifacts[0].analysis_directives,
        vec![".op".to_string()]
    );
    assert!(op_execution.run_artifacts[0].measurement_names.is_empty());
    assert!(op_execution.run_artifacts[0].fourier_probes.is_empty());
    assert_eq!(op_execution.run_artifacts[0].control_line_count, 0);
    assert!(op_execution.run_artifacts[0].control_lines.is_empty());
    assert_eq!(op_execution.diagnostic_count, 0);
    assert!(op_execution.diagnostic_codes.is_empty());
    assert_eq!(op_execution.run_artifacts[0].diagnostic_count, 0);
    assert!(op_execution.run_artifacts[0].diagnostic_codes.is_empty());
    let op_run_artifact_record = assert_run_artifact_table_matches(&op_execution);
    assert_eq!(
        op_run_artifact_record.get("Analysis").map(String::as_str),
        Some("op")
    );
    assert_eq!(
        op_run_artifact_record
            .get("DeckAnalysisKinds")
            .map(String::as_str),
        Some("7")
    );
    assert_eq!(
        op_run_artifact_record
            .get("DeckAnalysisKindList")
            .map(String::as_str),
        Some("op;dc;ac;tran;tf;sens;noise")
    );
    assert_eq!(
        op_run_artifact_record
            .get("DeckAnalysisDirectives")
            .map(String::as_str),
        Some("7")
    );
    assert_eq!(
        format_deck_table_csv(&op_execution.run_artifact_table),
        format_deck_run_artifact_csv(&op_execution.run_artifacts)
    );
    assert_eq!(
        format_deck_table_json(&op_execution.run_artifact_table),
        format_deck_run_artifact_json(&op_execution.run_artifacts)
    );
    let artifact_records = vec![op_run_artifact_record];
    assert_eq!(op_execution.table_artifacts[1].name.as_str(), "output-plan");
    assert_eq!(
        op_execution.table_artifacts[1].table,
        op_execution.output_plan_artifact_table
    );
    assert_eq!(
        op_execution.table_artifacts[1].csv,
        op_execution.output_plan_artifact_csv
    );
    assert_eq!(
        op_execution.table_artifacts[1].json,
        op_execution.output_plan_artifact_json
    );
    assert_eq!(
        &op_execution.table_artifacts[1].records,
        &op_execution.output_plan_artifact_records
    );
    assert_eq!(
        op_execution.table_artifacts[2].name.as_str(),
        "run-artifact"
    );
    assert_eq!(
        op_execution.table_artifacts[2].table,
        op_execution.run_artifact_table
    );
    assert_eq!(&op_execution.table_artifacts[2].records, &artifact_records);
    assert_eq!(artifact_records.len(), 1);
    assert_eq!(
        artifact_records[0].get("Analysis").map(String::as_str),
        Some("op")
    );
    assert_eq!(
        artifact_records[0]
            .get("ResultColumnList")
            .map(String::as_str),
        Some("Index;V(mid)")
    );
    assert_eq!(
        artifact_records[0].get("TableList").map(String::as_str),
        Some("result;output-plan;run-artifact")
    );
    assert_eq!(
        artifact_records[0]
            .get("OutputProbeList")
            .map(String::as_str),
        Some("V(mid)")
    );
    assert_eq!(
        artifact_records[0]
            .get("AnalysisDirectiveList")
            .map(String::as_str),
        Some(".op")
    );
    assert_eq!(
        format_deck_run_artifact_csv(&op_execution.run_artifacts),
        format_deck_table_csv(&op_execution.run_artifact_table)
    );
    assert_eq!(
        format_deck_table_csv("Name\tValue\nprobe\tSPICE,\"QUOTED\"\n"),
        "Name,Value\nprobe,\"SPICE,\"\"QUOTED\"\"\"\n"
    );
    assert_eq!(
        format_deck_table_json("Name\tValue\nprobe\tSPICE,\"QUOTED\"\n"),
        "[{\"Name\":\"probe\",\"Value\":\"SPICE,\\\"QUOTED\\\"\"}]\n"
    );
    let quoted_records = deck_table_records("Name\tValue\nprobe\tSPICE,\"QUOTED\"\n");
    assert_eq!(quoted_records.len(), 1);
    assert_eq!(
        quoted_records[0].get("Value").map(String::as_str),
        Some("SPICE,\"QUOTED\"")
    );
    let artifact_json = format_deck_run_artifact_json(&op_execution.run_artifacts);
    assert!(artifact_json.starts_with(&format!(
        "[{{\"Analysis\":\"op\",\"Directive\":\".op\",\"AnalysisDirectives\":\"1\",\"AnalysisDirectiveList\":\".op\",\"Line\":\"{}\",\"SourceName\":\"\",\"OutputNode\":\"\"",
        op_execution.plan.line_number
    )));
    assert!(artifact_json.contains("\"ResultColumnList\":\"Index;V(mid)\""));
    assert!(artifact_json.contains("\"TableList\":\"result;output-plan;run-artifact\""));
    assert!(artifact_json.contains("\"OutputProbeList\":\"V(mid)\""));
    assert!(artifact_json.contains("\"DiagnosticCodeList\":\"\""));
    assert!(artifact_json.contains("\"DeckAnalysisKinds\":\"7\""));
    assert!(artifact_json.contains("\"DeckAnalysisKindList\":\"op;dc;ac;tran;tf;sens;noise\""));
    let mut diagnostic_artifact = op_execution.run_artifacts[0].clone();
    diagnostic_artifact.diagnostic_codes = vec![
        "SPICE_DECK_ANALYSIS_TOKEN".to_string(),
        "SPICE_DECK_ANALYSIS_RANGE".to_string(),
    ];
    diagnostic_artifact.diagnostic_count = diagnostic_artifact.diagnostic_codes.len();
    let diagnostic_table = format_deck_run_artifact_table(&[diagnostic_artifact]);
    let diagnostic_record = deck_table_records(&diagnostic_table);
    assert_eq!(diagnostic_record.len(), 1);
    assert_eq!(
        diagnostic_record[0].get("Diagnostics").map(String::as_str),
        Some("2")
    );
    assert_eq!(
        diagnostic_record[0]
            .get("DiagnosticCodeList")
            .map(String::as_str),
        Some("SPICE_DECK_ANALYSIS_TOKEN;SPICE_DECK_ANALYSIS_RANGE")
    );
    let mut quoted_diagnostic_artifact = op_execution.run_artifacts[0].clone();
    quoted_diagnostic_artifact.diagnostic_codes = vec![
        "SPICE_DECK_ANALYSIS_TOKEN".to_string(),
        "SPICE,\"QUOTED\"".to_string(),
    ];
    quoted_diagnostic_artifact.diagnostic_count = quoted_diagnostic_artifact.diagnostic_codes.len();
    let quoted_csv = format_deck_run_artifact_csv(&[quoted_diagnostic_artifact.clone()]);
    assert!(quoted_csv.contains("\"SPICE_DECK_ANALYSIS_TOKEN;SPICE,\"\"QUOTED\"\"\""));
    assert!(quoted_csv
        .ends_with(",7,op;dc;ac;tran;tf;sens;noise,7,.op;.dc;.ac;.tran;.tf;.sens;.noise\n"));
    let quoted_json = format_deck_run_artifact_json(&[quoted_diagnostic_artifact]);
    assert!(quoted_json
        .contains("\"DiagnosticCodeList\":\"SPICE_DECK_ANALYSIS_TOKEN;SPICE,\\\"QUOTED\\\"\""));
    assert!(quoted_json.contains("\"DeckAnalysisKindList\":\"op;dc;ac;tran;tf;sens;noise\""));

    let dc_execution = run_deck_analysis(&circuit, netlist, Some("dc")).unwrap();
    assert_eq!(dc_execution.plan.source_name.as_deref(), Some("V1"));
    assert_eq!(
        dc_execution.output_probes,
        vec!["V(mid)".to_string(), "I(V1)".to_string()]
    );
    assert_eq!(
        dc_execution.output_directives,
        vec![
            ".save".to_string(),
            ".probe".to_string(),
            ".print".to_string()
        ]
    );
    assert_eq!(
        dc_execution.output_plan_artifacts[0].line_number,
        dc_execution.plan.line_number
    );
    assert_eq!(
        dc_execution.output_plan_artifacts[0].source_name.as_deref(),
        Some("V1")
    );
    assert_eq!(dc_execution.output_plan_artifacts[0].output_node, None);
    assert_eq!(dc_execution.output_plan_artifacts[0].sweep_kind, None);
    assert_eq!(dc_execution.output_plan_artifacts[0].start_value, Some(0.0));
    assert_eq!(dc_execution.output_plan_artifacts[0].stop_value, Some(1.0));
    assert_eq!(dc_execution.output_plan_artifacts[0].step_value, Some(1.0));
    assert_eq!(dc_execution.output_plan_artifacts[0].point_count, None);
    assert_eq!(
        dc_execution.output_plan_artifacts[0].start_frequency_hz,
        None
    );
    assert_eq!(
        dc_execution.output_plan_artifacts[0].stop_frequency_hz,
        None
    );
    assert_eq!(dc_execution.output_plan_artifacts[0].step_time, None);
    assert_eq!(
        dc_execution.output_plan_artifacts[0].use_initial_conditions,
        None
    );
    let dc_line = dc_execution.plan.line_number.to_string();
    assert_eq!(
        dc_execution.output_plan_artifact_records[0]
            .get("Line")
            .map(String::as_str),
        Some(dc_line.as_str())
    );
    assert_eq!(
        dc_execution.output_plan_artifact_records[0]
            .get("SourceName")
            .map(String::as_str),
        Some("V1")
    );
    assert_eq!(
        dc_execution.output_plan_artifact_records[0]
            .get("OutputNode")
            .map(String::as_str),
        Some("")
    );
    assert_eq!(
        dc_execution.output_plan_artifact_records[0]
            .get("StartValue")
            .map(String::as_str),
        Some("0.000000e+00")
    );
    assert_eq!(
        dc_execution.output_plan_artifact_records[0]
            .get("StopValue")
            .map(String::as_str),
        Some("1.000000e+00")
    );
    assert_eq!(
        dc_execution.output_plan_artifact_records[0]
            .get("StepValue")
            .map(String::as_str),
        Some("1.000000e+00")
    );
    assert_eq!(
        dc_execution.output_plan_artifact_records[0]
            .get("PointCount")
            .map(String::as_str),
        Some("")
    );
    assert_eq!(
        dc_execution.output_plan_artifacts[0].output_directive_kinds,
        vec!["save".to_string(), "probe".to_string(), "print".to_string()]
    );
    assert_eq!(
        dc_execution.output_plan_artifact_records[0]
            .get("OutputDirectiveKindList")
            .map(String::as_str),
        Some("save;probe;print")
    );
    assert_eq!(
        dc_execution.output_plan_artifacts[0].output_directive_analysis_kinds,
        vec!["global".to_string(), "dc".to_string()]
    );
    let dc_output_directive_lines = vec![save_line, probe_dc_line, print_dc_line];
    let dc_output_probe_lines = vec![save_line, probe_dc_line];
    assert_eq!(
        dc_execution.output_plan_artifacts[0].output_probe_lines,
        dc_output_probe_lines
    );
    assert_eq!(
        dc_execution.output_plan_artifacts[0].output_directive_lines,
        dc_output_directive_lines
    );
    let dc_output_probe_line_list = dc_output_probe_lines
        .iter()
        .map(usize::to_string)
        .collect::<Vec<_>>()
        .join(";");
    let dc_output_directive_line_list = dc_output_directive_lines
        .iter()
        .map(usize::to_string)
        .collect::<Vec<_>>()
        .join(";");
    assert_eq!(
        dc_execution.output_plan_artifact_records[0]
            .get("OutputDirectiveAnalysisKindList")
            .map(String::as_str),
        Some("global;dc")
    );
    assert_eq!(
        dc_execution.output_plan_artifact_records[0]
            .get("OutputProbeLineList")
            .map(String::as_str),
        Some(dc_output_probe_line_list.as_str())
    );
    assert_eq!(
        dc_execution.output_plan_artifact_records[0]
            .get("OutputDirectiveLineList")
            .map(String::as_str),
        Some(dc_output_directive_line_list.as_str())
    );
    assert_eq!(dc_execution.analysis_directives, vec![".dc".to_string()]);
    assert_eq!(dc_execution.table_count, 4);
    assert_eq!(
        dc_execution.tables,
        vec![
            "result".to_string(),
            "measurement".to_string(),
            "output-plan".to_string(),
            "run-artifact".to_string()
        ]
    );
    assert_eq!(dc_execution.measurements[0].name, "mid_avg");
    assert_eq!(
        dc_execution.measurement_table,
        "Name\tAnalysis\tProbe\tMode\tFrom\tTo\tValue\nmid_avg\tdc\tV(mid)\tavg\t\t\t2.500000e-01\n"
    );
    assert_eq!(
        dc_execution
            .table_artifacts
            .iter()
            .map(|artifact| artifact.name.as_str())
            .collect::<Vec<_>>(),
        vec!["result", "measurement", "output-plan", "run-artifact"]
    );
    assert_eq!(
        dc_execution.table_artifacts[1].table,
        dc_execution.measurement_table
    );
    assert_eq!(
        dc_execution.table_artifacts[1].csv,
        format_deck_table_csv(&dc_execution.measurement_table)
    );
    assert_eq!(
        dc_execution.table_artifacts[1].json,
        format_deck_table_json(&dc_execution.measurement_table)
    );
    assert_eq!(
        dc_execution.table_artifacts[1].records,
        deck_table_records(&dc_execution.measurement_table)
    );
    match &dc_execution.result {
        DeckAnalysisExecutionResult::DcSweep(points) => assert_eq!(points.len(), 2),
        other => panic!("expected DC sweep result, got {other:?}"),
    }
    assert_eq!(
        dc_execution.table,
        "Index\tSource\tValue\tV(mid)\tI(V1)\n0\tV1\t0.000000e+00\t0.000000e+00\t0.000000e+00\n1\tV1\t1.000000e+00\t5.000000e-01\t-5.000000e-04\n"
    );
    assert_eq!(dc_execution.run_artifacts[0].analysis, "dc");
    assert_eq!(
        dc_execution.run_artifacts[0].source_name.as_deref(),
        Some("V1")
    );
    assert_eq!(dc_execution.run_artifacts[0].output_node, None);
    assert_eq!(dc_execution.run_artifacts[0].start_value, Some(0.0));
    assert_eq!(dc_execution.run_artifacts[0].stop_value, Some(1.0));
    assert_eq!(dc_execution.run_artifacts[0].step_value, Some(1.0));
    assert_eq!(dc_execution.run_artifacts[0].result_column_count, 5);
    assert_eq!(
        dc_execution.run_artifacts[0].result_columns,
        vec![
            "Index".to_string(),
            "Source".to_string(),
            "Value".to_string(),
            "V(mid)".to_string(),
            "I(V1)".to_string()
        ]
    );
    assert_eq!(dc_execution.run_artifacts[0].table_count, 4);
    assert_eq!(
        dc_execution.run_artifacts[0].tables,
        vec![
            "result".to_string(),
            "measurement".to_string(),
            "output-plan".to_string(),
            "run-artifact".to_string()
        ]
    );
    assert_eq!(dc_execution.run_artifacts[0].step_time, None);
    assert_eq!(dc_execution.run_artifacts[0].use_initial_conditions, None);
    assert_eq!(
        dc_execution.run_artifacts[0].output_probes,
        vec!["V(mid)".to_string(), "I(V1)".to_string()]
    );
    assert_eq!(
        dc_execution.run_artifacts[0].output_directives,
        vec![
            ".save".to_string(),
            ".probe".to_string(),
            ".print".to_string()
        ]
    );
    assert_eq!(
        dc_execution.run_artifacts[0].analysis_directives,
        vec![".dc".to_string()]
    );
    assert_eq!(
        dc_execution.run_artifacts[0].measurement_names,
        vec!["mid_avg".to_string()]
    );
    assert!(dc_execution.run_artifacts[0].fourier_probes.is_empty());
    let dc_run_artifact_record = assert_run_artifact_table_matches(&dc_execution);
    assert_eq!(
        dc_run_artifact_record.get("Analysis").map(String::as_str),
        Some("dc")
    );
    assert_eq!(
        dc_run_artifact_record
            .get("DeckAnalysisKinds")
            .map(String::as_str),
        Some("7")
    );
    assert_eq!(
        dc_run_artifact_record
            .get("DeckAnalysisKindList")
            .map(String::as_str),
        Some("op;dc;ac;tran;tf;sens;noise")
    );

    let ac_execution = run_deck_analysis(&circuit, netlist, Some("ac")).unwrap();
    assert_eq!(ac_execution.output_probes, vec!["V(mid)".to_string()]);
    assert_eq!(
        ac_execution.output_directives,
        vec![".save".to_string(), ".plot".to_string()]
    );
    assert_eq!(ac_execution.output_plan_artifacts[0].output_node, None);
    assert_eq!(
        ac_execution.output_plan_artifacts[0].sweep_kind.as_deref(),
        Some("dec")
    );
    assert_eq!(ac_execution.output_plan_artifacts[0].point_count, Some(1));
    assert_eq!(
        ac_execution.output_plan_artifacts[0].start_frequency_hz,
        Some(1.0e3)
    );
    assert_eq!(
        ac_execution.output_plan_artifacts[0].stop_frequency_hz,
        Some(1.0e3)
    );
    assert_eq!(ac_execution.output_plan_artifacts[0].start_value, None);
    assert_eq!(ac_execution.output_plan_artifacts[0].step_time, None);
    assert_eq!(
        ac_execution.output_plan_artifacts[0].use_initial_conditions,
        None
    );
    assert_eq!(
        ac_execution.output_plan_artifact_records[0]
            .get("SweepKind")
            .map(String::as_str),
        Some("dec")
    );
    assert_eq!(
        ac_execution.output_plan_artifact_records[0]
            .get("PointCount")
            .map(String::as_str),
        Some("1")
    );
    assert_eq!(
        ac_execution.output_plan_artifact_records[0]
            .get("StartFrequencyHz")
            .map(String::as_str),
        Some("1.000000e+03")
    );
    assert_eq!(
        ac_execution.output_plan_artifact_records[0]
            .get("StopFrequencyHz")
            .map(String::as_str),
        Some("1.000000e+03")
    );
    assert_eq!(
        ac_execution.output_plan_artifacts[0].output_directive_kinds,
        vec!["save".to_string(), "plot".to_string()]
    );
    assert_eq!(
        ac_execution.output_plan_artifact_records[0]
            .get("OutputDirectiveKindList")
            .map(String::as_str),
        Some("save;plot")
    );
    assert_eq!(
        ac_execution.output_plan_artifacts[0].output_directive_analysis_kinds,
        vec!["global".to_string(), "ac".to_string()]
    );
    let ac_output_directive_lines = vec![save_line, plot_ac_line];
    assert_eq!(
        ac_execution.output_plan_artifacts[0].output_probe_lines,
        vec![save_line]
    );
    assert_eq!(
        ac_execution.output_plan_artifacts[0].output_directive_lines,
        ac_output_directive_lines
    );
    let ac_output_directive_line_list = ac_output_directive_lines
        .iter()
        .map(usize::to_string)
        .collect::<Vec<_>>()
        .join(";");
    assert_eq!(
        ac_execution.output_plan_artifact_records[0]
            .get("OutputDirectiveAnalysisKindList")
            .map(String::as_str),
        Some("global;ac")
    );
    assert_eq!(
        ac_execution.output_plan_artifact_records[0]
            .get("OutputDirectiveLineList")
            .map(String::as_str),
        Some(ac_output_directive_line_list.as_str())
    );
    assert_eq!(ac_execution.analysis_directives, vec![".ac".to_string()]);
    assert_eq!(ac_execution.table_count, 4);
    assert_eq!(
        ac_execution.tables,
        vec![
            "result".to_string(),
            "measurement".to_string(),
            "output-plan".to_string(),
            "run-artifact".to_string()
        ]
    );
    assert_eq!(ac_execution.measurements[0].name, "mid_peak");
    assert_eq!(
        ac_execution.measurement_table,
        "Name\tAnalysis\tProbe\tMode\tFrom\tTo\tValue\nmid_peak\tac\tV(mid)\tmax\t\t\t5.000000e-01\n"
    );
    match &ac_execution.result {
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
    assert_eq!(ac_execution.run_artifacts[0].source_name, None);
    assert_eq!(ac_execution.run_artifacts[0].output_node, None);
    assert_eq!(
        ac_execution.run_artifacts[0].sweep_kind.as_deref(),
        Some("dec")
    );
    assert_eq!(ac_execution.run_artifacts[0].point_count, Some(1));
    assert_eq!(
        ac_execution.run_artifacts[0].start_frequency_hz,
        Some(1.0e3)
    );
    assert_eq!(ac_execution.run_artifacts[0].stop_frequency_hz, Some(1.0e3));
    assert_eq!(ac_execution.run_artifacts[0].result_column_count, 7);
    assert_eq!(
        ac_execution.run_artifacts[0].result_columns,
        vec![
            "Index".to_string(),
            "Frequency".to_string(),
            "Probe".to_string(),
            "Real".to_string(),
            "Imaginary".to_string(),
            "Magnitude".to_string(),
            "Phase".to_string()
        ]
    );
    assert_eq!(ac_execution.run_artifacts[0].table_count, 4);
    assert_eq!(
        ac_execution.run_artifacts[0].tables,
        vec![
            "result".to_string(),
            "measurement".to_string(),
            "output-plan".to_string(),
            "run-artifact".to_string()
        ]
    );
    assert_eq!(ac_execution.run_artifacts[0].step_time, None);
    assert_eq!(ac_execution.run_artifacts[0].use_initial_conditions, None);
    assert_eq!(
        ac_execution.run_artifacts[0].output_directives,
        vec![".save".to_string(), ".plot".to_string()]
    );
    assert_eq!(
        ac_execution.run_artifacts[0].measurement_names,
        vec!["mid_peak".to_string()]
    );
    assert!(ac_execution.run_artifacts[0].fourier_probes.is_empty());
    let ac_run_artifact_record = assert_run_artifact_table_matches(&ac_execution);
    assert_eq!(
        ac_run_artifact_record.get("Analysis").map(String::as_str),
        Some("ac")
    );
    assert_eq!(
        ac_run_artifact_record
            .get("DeckAnalysisKinds")
            .map(String::as_str),
        Some("7")
    );
    assert_eq!(
        ac_run_artifact_record
            .get("DeckAnalysisKindList")
            .map(String::as_str),
        Some("op;dc;ac;tran;tf;sens;noise")
    );

    let tran_execution = run_deck_analysis(&circuit, netlist, Some("tran")).unwrap();
    assert_eq!(tran_execution.output_probes, vec!["V(mid)".to_string()]);
    assert_eq!(tran_execution.output_directives, vec![".save".to_string()]);
    assert_eq!(
        tran_execution.output_plan_artifacts[0].step_time,
        Some(1.0e-3)
    );
    assert_eq!(
        tran_execution.output_plan_artifacts[0].stop_time,
        Some(1.0e-3)
    );
    assert_eq!(tran_execution.output_plan_artifacts[0].start_time, None);
    assert_eq!(tran_execution.output_plan_artifacts[0].max_step, None);
    assert_eq!(
        tran_execution.output_plan_artifacts[0].use_initial_conditions,
        Some(false)
    );
    assert_eq!(
        tran_execution.output_plan_artifact_records[0]
            .get("StepTime")
            .map(String::as_str),
        Some("1.000000e-03")
    );
    assert_eq!(
        tran_execution.output_plan_artifact_records[0]
            .get("StopTime")
            .map(String::as_str),
        Some("1.000000e-03")
    );
    assert_eq!(
        tran_execution.output_plan_artifact_records[0]
            .get("UseInitialConditions")
            .map(String::as_str),
        Some("false")
    );
    assert_eq!(
        tran_execution.output_plan_artifacts[0].output_directive_lines,
        vec![save_line]
    );
    assert_eq!(
        tran_execution.output_plan_artifacts[0].output_probe_lines,
        vec![save_line]
    );
    assert_eq!(
        tran_execution.analysis_directives,
        vec![".tran".to_string()]
    );
    assert_eq!(tran_execution.table_count, 4);
    assert_eq!(
        tran_execution.tables,
        vec![
            "result".to_string(),
            "measurement".to_string(),
            "output-plan".to_string(),
            "run-artifact".to_string()
        ]
    );
    assert_eq!(tran_execution.measurements[0].name, "mid_final");
    assert_eq!(
        tran_execution.measurement_table,
        "Name\tAnalysis\tProbe\tMode\tFrom\tTo\tValue\nmid_final\ttran\tV(mid)\tlast\t\t\t5.000000e-01\n"
    );
    match &tran_execution.result {
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
    assert_eq!(tran_execution.run_artifacts[0].source_name, None);
    assert_eq!(tran_execution.run_artifacts[0].output_node, None);
    assert_eq!(tran_execution.run_artifacts[0].step_time, Some(1.0e-3));
    assert_eq!(tran_execution.run_artifacts[0].stop_time, Some(1.0e-3));
    assert_eq!(tran_execution.run_artifacts[0].result_column_count, 3);
    assert_eq!(
        tran_execution.run_artifacts[0].result_columns,
        vec![
            "Index".to_string(),
            "Time".to_string(),
            "V(mid)".to_string()
        ]
    );
    assert_eq!(tran_execution.run_artifacts[0].table_count, 4);
    assert_eq!(
        tran_execution.run_artifacts[0].tables,
        vec![
            "result".to_string(),
            "measurement".to_string(),
            "output-plan".to_string(),
            "run-artifact".to_string()
        ]
    );
    assert_eq!(tran_execution.run_artifacts[0].start_time, None);
    assert_eq!(tran_execution.run_artifacts[0].max_step, None);
    assert_eq!(
        tran_execution.run_artifacts[0].use_initial_conditions,
        Some(false)
    );
    assert_eq!(
        tran_execution.run_artifacts[0].output_directives,
        vec![".save".to_string()]
    );
    assert_eq!(
        tran_execution.run_artifacts[0].measurement_names,
        vec!["mid_final".to_string()]
    );
    assert!(tran_execution.run_artifacts[0].fourier_probes.is_empty());
    assert_eq!(tran_execution.run_artifacts[0].diagnostic_count, 0);
    assert!(tran_execution.run_artifacts[0].diagnostic_codes.is_empty());
    let tran_run_artifact_record = assert_run_artifact_table_matches(&tran_execution);
    assert_eq!(
        tran_run_artifact_record.get("Analysis").map(String::as_str),
        Some("tran")
    );
    assert_eq!(
        tran_run_artifact_record
            .get("DeckAnalysisKinds")
            .map(String::as_str),
        Some("7")
    );
    assert_eq!(
        tran_run_artifact_record
            .get("DeckAnalysisKindList")
            .map(String::as_str),
        Some("op;dc;ac;tran;tf;sens;noise")
    );

    let tf_execution = run_deck_analysis(&circuit, netlist, Some("tf")).unwrap();
    assert_eq!(tf_execution.plan.output_node.as_deref(), Some("mid"));
    assert_eq!(tf_execution.plan.source_name.as_deref(), Some("V1"));
    match &tf_execution.result {
        DeckAnalysisExecutionResult::Tf(result) => {
            assert_close(result.transfer_ratio, 0.5);
            assert_close(result.input_impedance_ohms, 2_000.0);
            assert_close(result.output_impedance_ohms, 500.0);
        }
        other => panic!("expected TF result, got {other:?}"),
    }
    assert_eq!(tf_execution.output_probes, vec!["V(mid)".to_string()]);
    assert!(tf_execution.output_directives.is_empty());
    assert_eq!(
        tf_execution.output_plan_artifacts[0].output_node.as_deref(),
        Some("mid")
    );
    assert_eq!(
        tf_execution.output_plan_artifact_records[0]
            .get("OutputNode")
            .map(String::as_str),
        Some("mid")
    );
    assert_eq!(tf_execution.analysis_directives, vec![".tf".to_string()]);
    assert_eq!(tf_execution.table_count, 3);
    assert_eq!(
        tf_execution.tables,
        vec![
            "result".to_string(),
            "output-plan".to_string(),
            "run-artifact".to_string()
        ]
    );
    assert!(tf_execution.measurements.is_empty());
    assert_eq!(
        tf_execution.measurement_table,
        "Name\tAnalysis\tProbe\tMode\tFrom\tTo\tValue\n"
    );
    assert_eq!(
        tf_execution.table,
        "TransferRatio\tInputImpedance\tOutputImpedance\n5.000000e-01\t2.000000e+03\t5.000000e+02\n"
    );
    assert_eq!(tf_execution.run_artifacts[0].analysis, "tf");
    assert_eq!(
        tf_execution.run_artifacts[0].source_name.as_deref(),
        Some("V1")
    );
    assert_eq!(
        tf_execution.run_artifacts[0].output_node.as_deref(),
        Some("mid")
    );
    assert_eq!(tf_execution.run_artifacts[0].result_rows, 1);
    assert_eq!(tf_execution.run_artifacts[0].result_column_count, 3);
    assert_eq!(
        tf_execution.run_artifacts[0].result_columns,
        vec![
            "TransferRatio".to_string(),
            "InputImpedance".to_string(),
            "OutputImpedance".to_string()
        ]
    );
    assert_eq!(tf_execution.run_artifacts[0].table_count, 3);
    assert_eq!(
        tf_execution.run_artifacts[0].tables,
        vec![
            "result".to_string(),
            "output-plan".to_string(),
            "run-artifact".to_string()
        ]
    );
    assert_eq!(tf_execution.run_artifacts[0].step_time, None);
    assert_eq!(tf_execution.run_artifacts[0].use_initial_conditions, None);
    assert_eq!(
        tf_execution.run_artifacts[0].output_probes,
        vec!["V(mid)".to_string()]
    );
    assert!(tf_execution.run_artifacts[0].output_directives.is_empty());
    assert!(tf_execution.run_artifacts[0].measurement_names.is_empty());
    assert!(tf_execution.run_artifacts[0].fourier_probes.is_empty());
    let tf_run_artifact_record = assert_run_artifact_table_matches(&tf_execution);
    assert_eq!(
        tf_run_artifact_record.get("Analysis").map(String::as_str),
        Some("tf")
    );
    assert_eq!(
        tf_run_artifact_record
            .get("DeckAnalysisKinds")
            .map(String::as_str),
        Some("7")
    );
    assert_eq!(
        tf_run_artifact_record
            .get("DeckAnalysisKindList")
            .map(String::as_str),
        Some("op;dc;ac;tran;tf;sens;noise")
    );

    let sens_execution = run_deck_analysis(&circuit, netlist, Some("sens")).unwrap();
    assert_eq!(sens_execution.plan.output_node.as_deref(), Some("mid"));
    assert_eq!(sens_execution.plan.source_name, None);
    match &sens_execution.result {
        DeckAnalysisExecutionResult::Sens(result) => {
            assert_eq!(result.output_node, "mid");
            assert_eq!(result.entries.len(), 3);
        }
        other => panic!("expected sensitivity result, got {other:?}"),
    }
    assert_eq!(sens_execution.output_probes, vec!["V(mid)".to_string()]);
    assert_eq!(
        sens_execution.output_plan_artifacts[0]
            .output_node
            .as_deref(),
        Some("mid")
    );
    assert_eq!(
        sens_execution.output_plan_artifact_records[0]
            .get("OutputNode")
            .map(String::as_str),
        Some("mid")
    );
    assert_eq!(
        sens_execution.analysis_directives,
        vec![".sens".to_string()]
    );
    assert_eq!(sens_execution.table_count, 3);
    assert_eq!(
        sens_execution.tables,
        vec![
            "result".to_string(),
            "output-plan".to_string(),
            "run-artifact".to_string()
        ]
    );
    assert!(sens_execution.measurements.is_empty());
    assert_eq!(
        sens_execution.measurement_table,
        "Name\tAnalysis\tProbe\tMode\tFrom\tTo\tValue\n"
    );
    assert!(sens_execution.table.starts_with(
        "OutputNode\tNominalVoltage\tElement\tParameter\tNominalValue\tSensitivity\tRelativeSensitivity\n"
    ));
    assert_eq!(sens_execution.run_artifacts[0].analysis, "sens");
    assert_eq!(sens_execution.run_artifacts[0].source_name, None);
    assert_eq!(
        sens_execution.run_artifacts[0].output_node.as_deref(),
        Some("mid")
    );
    assert_eq!(sens_execution.run_artifacts[0].result_rows, 1);
    assert_eq!(sens_execution.run_artifacts[0].result_column_count, 7);
    assert_eq!(
        sens_execution.run_artifacts[0].result_columns,
        vec![
            "OutputNode".to_string(),
            "NominalVoltage".to_string(),
            "Element".to_string(),
            "Parameter".to_string(),
            "NominalValue".to_string(),
            "Sensitivity".to_string(),
            "RelativeSensitivity".to_string()
        ]
    );
    assert_eq!(sens_execution.run_artifacts[0].table_count, 3);
    assert_eq!(
        sens_execution.run_artifacts[0].tables,
        vec![
            "result".to_string(),
            "output-plan".to_string(),
            "run-artifact".to_string()
        ]
    );
    assert_eq!(sens_execution.run_artifacts[0].step_time, None);
    assert_eq!(sens_execution.run_artifacts[0].use_initial_conditions, None);
    assert_eq!(
        sens_execution.run_artifacts[0].output_probes,
        vec!["V(mid)".to_string()]
    );
    assert!(sens_execution.run_artifacts[0].output_directives.is_empty());
    assert!(sens_execution.run_artifacts[0].measurement_names.is_empty());
    assert!(sens_execution.run_artifacts[0].fourier_probes.is_empty());
    let sens_run_artifact_record = assert_run_artifact_table_matches(&sens_execution);
    assert_eq!(
        sens_run_artifact_record.get("Analysis").map(String::as_str),
        Some("sens")
    );
    assert_eq!(
        sens_run_artifact_record
            .get("DeckAnalysisKinds")
            .map(String::as_str),
        Some("7")
    );
    assert_eq!(
        sens_run_artifact_record
            .get("DeckAnalysisKindList")
            .map(String::as_str),
        Some("op;dc;ac;tran;tf;sens;noise")
    );

    let noise_execution = run_deck_analysis(&circuit, netlist, Some("noise")).unwrap();
    assert_eq!(noise_execution.plan.output_node.as_deref(), Some("mid"));
    assert_eq!(noise_execution.plan.source_name.as_deref(), Some("V1"));
    assert_eq!(noise_execution.plan.sweep_kind.as_deref(), Some("lin"));
    assert_eq!(noise_execution.plan.point_count, Some(1));
    assert!((noise_execution.plan.start_frequency_hz.unwrap() - 1.0e3).abs() < 1.0e-9);
    assert!((noise_execution.plan.stop_frequency_hz.unwrap() - 1.0e3).abs() < 1.0e-9);
    match &noise_execution.result {
        DeckAnalysisExecutionResult::Noise(result) => {
            assert_eq!(result.output_node, "mid");
            assert_eq!(result.input_source, "V1");
            assert_eq!(result.points.len(), 1);
            assert_eq!(noise_execution.table, format_deck_noise_table(result));
        }
        other => panic!("expected noise result, got {other:?}"),
    }
    assert_eq!(noise_execution.output_probes, vec!["V(mid)".to_string()]);
    assert_eq!(
        noise_execution.output_plan_artifacts[0]
            .source_name
            .as_deref(),
        Some("V1")
    );
    assert_eq!(
        noise_execution.output_plan_artifacts[0]
            .output_node
            .as_deref(),
        Some("mid")
    );
    assert_eq!(
        noise_execution.output_plan_artifacts[0]
            .sweep_kind
            .as_deref(),
        Some("lin")
    );
    assert_eq!(
        noise_execution.output_plan_artifacts[0].point_count,
        Some(1)
    );
    assert_eq!(
        noise_execution.output_plan_artifacts[0].start_frequency_hz,
        Some(1.0e3)
    );
    assert_eq!(
        noise_execution.output_plan_artifacts[0].stop_frequency_hz,
        Some(1.0e3)
    );
    assert_eq!(
        noise_execution.output_plan_artifact_records[0]
            .get("OutputNode")
            .map(String::as_str),
        Some("mid")
    );
    assert_eq!(
        noise_execution.output_plan_artifact_records[0]
            .get("SweepKind")
            .map(String::as_str),
        Some("lin")
    );
    assert_eq!(
        noise_execution.output_plan_artifact_records[0]
            .get("PointCount")
            .map(String::as_str),
        Some("1")
    );
    assert_eq!(
        noise_execution.output_plan_artifact_records[0]
            .get("StartFrequencyHz")
            .map(String::as_str),
        Some("1.000000e+03")
    );
    assert_eq!(
        noise_execution.output_plan_artifact_records[0]
            .get("StopFrequencyHz")
            .map(String::as_str),
        Some("1.000000e+03")
    );
    assert_eq!(
        noise_execution.analysis_directives,
        vec![".noise".to_string()]
    );
    assert_eq!(noise_execution.table_count, 3);
    assert_eq!(
        noise_execution.tables,
        vec![
            "result".to_string(),
            "output-plan".to_string(),
            "run-artifact".to_string()
        ]
    );
    assert!(noise_execution.measurements.is_empty());
    assert_eq!(
        noise_execution.measurement_table,
        "Name\tAnalysis\tProbe\tMode\tFrom\tTo\tValue\n"
    );
    assert!(noise_execution.table.starts_with(
        "Index\tFrequency\tOutputNode\tInputSource\tOutputPSD\tInputReferredPSD\tElement\tType\tSourcePSD\tContributionPSD\n"
    ));
    assert_eq!(noise_execution.run_artifacts[0].analysis, "noise");
    assert_eq!(
        noise_execution.run_artifacts[0].source_name.as_deref(),
        Some("V1")
    );
    assert_eq!(
        noise_execution.run_artifacts[0].output_node.as_deref(),
        Some("mid")
    );
    assert_eq!(
        noise_execution.run_artifacts[0].sweep_kind.as_deref(),
        Some("lin")
    );
    assert_eq!(noise_execution.run_artifacts[0].point_count, Some(1));
    assert_eq!(
        noise_execution.run_artifacts[0].start_frequency_hz,
        Some(1.0e3)
    );
    assert_eq!(
        noise_execution.run_artifacts[0].stop_frequency_hz,
        Some(1.0e3)
    );
    assert_eq!(noise_execution.run_artifacts[0].result_rows, 1);
    assert_eq!(noise_execution.run_artifacts[0].result_column_count, 10);
    assert_eq!(
        noise_execution.run_artifacts[0].result_columns,
        vec![
            "Index".to_string(),
            "Frequency".to_string(),
            "OutputNode".to_string(),
            "InputSource".to_string(),
            "OutputPSD".to_string(),
            "InputReferredPSD".to_string(),
            "Element".to_string(),
            "Type".to_string(),
            "SourcePSD".to_string(),
            "ContributionPSD".to_string()
        ]
    );
    assert_eq!(noise_execution.run_artifacts[0].table_count, 3);
    assert_eq!(
        noise_execution.run_artifacts[0].tables,
        vec![
            "result".to_string(),
            "output-plan".to_string(),
            "run-artifact".to_string()
        ]
    );
    assert_eq!(noise_execution.run_artifacts[0].step_time, None);
    assert_eq!(
        noise_execution.run_artifacts[0].use_initial_conditions,
        None
    );
    assert_eq!(
        noise_execution.run_artifacts[0].output_probes,
        vec!["V(mid)".to_string()]
    );
    assert!(noise_execution.run_artifacts[0]
        .output_directives
        .is_empty());
    assert!(noise_execution.run_artifacts[0]
        .measurement_names
        .is_empty());
    assert!(noise_execution.run_artifacts[0].fourier_probes.is_empty());
    let noise_run_artifact_record = assert_run_artifact_table_matches(&noise_execution);
    assert_eq!(
        noise_run_artifact_record
            .get("Analysis")
            .map(String::as_str),
        Some("noise")
    );
    assert_eq!(
        noise_run_artifact_record
            .get("DeckAnalysisKinds")
            .map(String::as_str),
        Some("7")
    );
    assert_eq!(
        noise_run_artifact_record
            .get("DeckAnalysisKindList")
            .map(String::as_str),
        Some("op;dc;ac;tran;tf;sens;noise")
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
        tran_window_execution.run_artifacts[0].step_time,
        Some(2.0e-3)
    );
    assert_eq!(
        tran_window_execution.run_artifacts[0].stop_time,
        Some(6.0e-3)
    );
    assert_eq!(
        tran_window_execution.run_artifacts[0].start_time,
        Some(2.0e-3)
    );
    assert_eq!(
        tran_window_execution.run_artifacts[0].max_step,
        Some(1.0e-3)
    );
    assert_eq!(
        tran_window_execution.run_artifacts[0].use_initial_conditions,
        Some(true)
    );
    assert_eq!(
        tran_window_execution.run_artifacts[0].result_column_count,
        3
    );
    assert_eq!(
        tran_window_execution.run_artifacts[0].result_columns,
        vec![
            "Index".to_string(),
            "Time".to_string(),
            "V(mid)".to_string()
        ]
    );
    assert_eq!(tran_window_execution.run_artifacts[0].table_count, 3);
    assert_eq!(
        tran_window_execution.run_artifacts[0].tables,
        vec![
            "result".to_string(),
            "output-plan".to_string(),
            "run-artifact".to_string()
        ]
    );
    assert_eq!(tran_window_execution.table_count, 3);
    assert_eq!(
        tran_window_execution.tables,
        vec![
            "result".to_string(),
            "output-plan".to_string(),
            "run-artifact".to_string()
        ]
    );
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
    let tran_window_run_artifact_record = assert_run_artifact_table_matches(&tran_window_execution);
    assert_eq!(
        tran_window_run_artifact_record
            .get("Analysis")
            .map(String::as_str),
        Some("tran")
    );
    assert_eq!(
        tran_window_run_artifact_record
            .get("DeckAnalysisKinds")
            .map(String::as_str),
        Some("1")
    );
    assert_eq!(
        tran_window_run_artifact_record
            .get("DeckAnalysisKindList")
            .map(String::as_str),
        Some("tran")
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
fn run_deck_executes_all_analysis_cards_in_source_order() {
    let mut circuit = Circuit::new();
    circuit.add(Element::VoltageSource(VoltageSource::new(
        "V1", "in", "0", 1.0,
    )));
    circuit.add(Element::Resistor(Resistor::new("R1", "in", "0", 1_000.0)));
    let netlist = ".save V(in)\n.op\n.dc V1 0 1 1\n.op\n.end\n";

    let error = run_deck_analysis(&circuit, netlist, None).unwrap_err();
    assert!(error.to_string().contains("multiple analysis cards"));

    let execution = run_deck(&circuit, netlist).unwrap();

    assert_eq!(execution.execution_count, 3);
    assert_eq!(execution.analysis_order, vec!["op", "dc", "op"]);
    assert_eq!(execution.analysis_directives, vec![".op", ".dc", ".op"]);
    assert_eq!(
        execution
            .executions
            .iter()
            .map(|item| item.plan.analysis.as_str())
            .collect::<Vec<_>>(),
        vec!["op", "dc", "op"]
    );
    assert_eq!(execution.run_artifact_count, 3);
    assert_eq!(
        execution
            .run_artifacts
            .iter()
            .map(|artifact| artifact.analysis.as_str())
            .collect::<Vec<_>>(),
        vec!["op", "dc", "op"]
    );
    assert_eq!(
        execution.run_artifact_records,
        deck_table_records(&execution.run_artifact_table)
    );
    assert_eq!(
        execution.run_artifact_records[1]
            .get("Analysis")
            .map(String::as_str),
        Some("dc")
    );
    assert_eq!(
        execution.run_artifact_records[1]
            .get("DeckAnalysisKinds")
            .map(String::as_str),
        Some("2")
    );
    assert_eq!(
        execution.run_artifact_records[1]
            .get("DeckAnalysisKindList")
            .map(String::as_str),
        Some("op;dc")
    );
    assert_eq!(
        execution.run_artifact_records[1]
            .get("DeckAnalysisDirectives")
            .map(String::as_str),
        Some("3")
    );
    assert_eq!(
        execution.run_artifact_records[1]
            .get("DeckAnalysisDirectiveList")
            .map(String::as_str),
        Some(".op;.dc;.op")
    );
}

#[test]
fn run_deck_analysis_surfaces_control_diagnostics_in_artifacts() {
    let mut circuit = Circuit::new();
    circuit.add(Element::VoltageSource(VoltageSource::new(
        "V1", "in", "0", 1.0,
    )));
    circuit.add(Element::Resistor(Resistor::new("R1", "in", "0", 1_000.0)));
    let netlist = "
.save V(in)
.control
save V(in)
probe V(in)
set filetype=ascii
set wr_vecnames
set wr_singlescale
set appendwrite
.set WR_VECNAMES
write out.raw V(in) V(missing)
wrdata out.dat V(in) V(missing)
source other.cir
cd /tmp
if v(in) > 0
let gain = 2
.endc
.op
.end
";

    let execution = run_deck_analysis(&circuit, netlist, Some("op")).unwrap();
    let expected_codes = vec![
        "SPICE_DECK_CONTROL_SCRIPT_COMMAND".to_string(),
        "SPICE_DECK_CONTROL_WORKDIR_COMMAND".to_string(),
        "SPICE_DECK_CONTROL_FLOW_COMMAND".to_string(),
        "SPICE_DECK_CONTROL_VARIABLE_COMMAND".to_string(),
    ];
    let code_list = expected_codes.join(";");
    let expected_control_lines = vec![".save V(in)".to_string(), ".probe V(in)".to_string()];
    let control_line_list = expected_control_lines.join(";");
    let expected_write_markers = vec![
        "write out.raw V(in) V(missing)".to_string(),
        "wrdata out.dat V(in) V(missing)".to_string(),
    ];
    let write_marker_list = expected_write_markers.join(";");
    let expected_rawfile_options = vec![
        "set filetype=ascii".to_string(),
        "set wr_vecnames".to_string(),
        "set wr_singlescale".to_string(),
        "set appendwrite".to_string(),
        "set wr_vecnames".to_string(),
    ];
    let rawfile_option_list = expected_rawfile_options.join(";");
    let rawfile_option_count = expected_rawfile_options.len().to_string();
    let expected_policy_lines = vec![13, 14, 15, 16];
    let expected_policy_categories = vec![
        "script".to_string(),
        "workdir".to_string(),
        "control-flow".to_string(),
        "variable".to_string(),
    ];
    let policy_category_list = expected_policy_categories.join(";");
    let expected_policy_commands = vec![
        "source other.cir".to_string(),
        "cd /tmp".to_string(),
        "if v(in) > 0".to_string(),
        "let gain = 2".to_string(),
    ];
    let expected_table_names = vec![
        "result".to_string(),
        "control-policy".to_string(),
        "control-policy-summary".to_string(),
        "output-plan".to_string(),
        "run-artifact".to_string(),
    ];
    let table_list = expected_table_names.join(";");

    assert_eq!(execution.control_line_count, expected_control_lines.len());
    assert_eq!(execution.control_lines, expected_control_lines);
    assert_eq!(execution.write_marker_count, expected_write_markers.len());
    assert_eq!(execution.write_markers, expected_write_markers);
    assert_eq!(
        execution.rawfile_option_count,
        expected_rawfile_options.len()
    );
    assert_eq!(execution.rawfile_options, expected_rawfile_options);
    assert_eq!(execution.rawfile_artifact_count, 1);
    assert_eq!(execution.rawfile_artifacts[0].target, "out.raw");
    assert_eq!(
        execution.rawfile_artifacts[0].marker,
        "write out.raw V(in) V(missing)"
    );
    assert_eq!(execution.rawfile_artifacts[0].probe_count, 2);
    assert_eq!(
        execution.rawfile_artifacts[0].probes,
        vec!["V(in)", "V(missing)"]
    );
    assert_eq!(execution.rawfile_artifacts[0].matched_probe_count, 1);
    assert_eq!(execution.rawfile_artifacts[0].matched_probes, vec!["V(in)"]);
    assert_eq!(execution.rawfile_artifacts[0].unmatched_probe_count, 1);
    assert_eq!(
        execution.rawfile_artifacts[0].unmatched_probes,
        vec!["V(missing)"]
    );
    assert_eq!(
        execution.rawfile_artifacts[0].option_count,
        expected_rawfile_options.len()
    );
    assert_eq!(
        execution.rawfile_artifacts[0].options,
        expected_rawfile_options
    );
    assert!(execution.rawfile_artifacts[0]
        .rawfile
        .contains("Title: SPICE deck op result\n"));
    assert!(execution.rawfile_artifacts[0]
        .rawfile
        .contains("No. Variables: 2\n"));
    assert!(execution.rawfile_artifacts[0]
        .rawfile
        .contains(&format!("Options: {rawfile_option_list}\n")));
    assert!(execution.rawfile_artifacts[0]
        .rawfile
        .contains("0\t0\t1.000000e+00\n"));
    assert_eq!(
        execution.rawfile_artifact_records[0]
            .get("Target")
            .map(String::as_str),
        Some("out.raw")
    );
    assert_eq!(
        execution.rawfile_artifact_records[0]
            .get("Marker")
            .map(String::as_str),
        Some("write out.raw V(in) V(missing)")
    );
    assert_eq!(
        execution.rawfile_artifact_records[0]
            .get("Probes")
            .map(String::as_str),
        Some("2")
    );
    assert_eq!(
        execution.rawfile_artifact_records[0]
            .get("ProbeList")
            .map(String::as_str),
        Some("V(in);V(missing)")
    );
    assert_eq!(
        execution.rawfile_artifact_records[0]
            .get("MatchedProbes")
            .map(String::as_str),
        Some("1")
    );
    assert_eq!(
        execution.rawfile_artifact_records[0]
            .get("MatchedProbeList")
            .map(String::as_str),
        Some("V(in)")
    );
    assert_eq!(
        execution.rawfile_artifact_records[0]
            .get("UnmatchedProbes")
            .map(String::as_str),
        Some("1")
    );
    assert_eq!(
        execution.rawfile_artifact_records[0]
            .get("UnmatchedProbeList")
            .map(String::as_str),
        Some("V(missing)")
    );
    assert_eq!(
        execution.rawfile_artifact_records[0]
            .get("RawfileOptionList")
            .map(String::as_str),
        Some(rawfile_option_list.as_str())
    );
    let rawfile_bytes = execution.rawfile_artifacts[0].rawfile.len().to_string();
    assert_eq!(
        execution.rawfile_artifact_records[0]
            .get("Bytes")
            .map(String::as_str),
        Some(rawfile_bytes.as_str())
    );
    assert_eq!(
        execution.rawfile_artifact_table,
        format_deck_rawfile_artifact_table(&execution.rawfile_artifacts)
    );
    assert_eq!(
        execution.rawfile_artifact_csv,
        format_deck_rawfile_artifact_csv(&execution.rawfile_artifacts)
    );
    assert_eq!(
        execution.rawfile_artifact_json,
        format_deck_rawfile_artifact_json(&execution.rawfile_artifacts)
    );
    assert_eq!(execution.wrdata_artifact_count, 1);
    assert_eq!(execution.wrdata_artifacts[0].target, "out.dat");
    assert_eq!(
        execution.wrdata_artifacts[0].marker,
        "wrdata out.dat V(in) V(missing)"
    );
    assert_eq!(execution.wrdata_artifacts[0].probe_count, 2);
    assert_eq!(
        execution.wrdata_artifacts[0].probes,
        vec!["V(in)", "V(missing)"]
    );
    assert_eq!(execution.wrdata_artifacts[0].matched_probe_count, 1);
    assert_eq!(execution.wrdata_artifacts[0].matched_probes, vec!["V(in)"]);
    assert_eq!(execution.wrdata_artifacts[0].unmatched_probe_count, 1);
    assert_eq!(
        execution.wrdata_artifacts[0].unmatched_probes,
        vec!["V(missing)"]
    );
    assert_eq!(
        execution.wrdata_artifacts[0].option_count,
        expected_rawfile_options.len()
    );
    assert_eq!(
        execution.wrdata_artifacts[0].options,
        expected_rawfile_options
    );
    assert!(execution.wrdata_artifacts[0]
        .datafile
        .contains("# SPICE deck wrdata artifact\n"));
    assert!(execution.wrdata_artifacts[0]
        .datafile
        .contains("Probes: V(in);V(missing)\n"));
    assert!(execution.wrdata_artifacts[0]
        .datafile
        .contains(&format!("Options: {rawfile_option_list}\n")));
    assert!(execution.wrdata_artifacts[0]
        .datafile
        .contains("VectorNames: Index;V(in)\n"));
    assert!(execution.wrdata_artifacts[0]
        .datafile
        .contains("Scale: Index\n"));
    assert!(execution.wrdata_artifacts[0]
        .datafile
        .contains("Index\tV(in)\n"));
    assert!(execution.wrdata_artifacts[0]
        .datafile
        .contains("0\t1.000000e+00\n"));
    assert_eq!(
        execution.wrdata_artifact_records[0]
            .get("Target")
            .map(String::as_str),
        Some("out.dat")
    );
    assert_eq!(
        execution.wrdata_artifact_records[0]
            .get("Marker")
            .map(String::as_str),
        Some("wrdata out.dat V(in) V(missing)")
    );
    assert_eq!(
        execution.wrdata_artifact_records[0]
            .get("ProbeList")
            .map(String::as_str),
        Some("V(in);V(missing)")
    );
    assert_eq!(
        execution.wrdata_artifact_records[0]
            .get("MatchedProbes")
            .map(String::as_str),
        Some("1")
    );
    assert_eq!(
        execution.wrdata_artifact_records[0]
            .get("MatchedProbeList")
            .map(String::as_str),
        Some("V(in)")
    );
    assert_eq!(
        execution.wrdata_artifact_records[0]
            .get("UnmatchedProbes")
            .map(String::as_str),
        Some("1")
    );
    assert_eq!(
        execution.wrdata_artifact_records[0]
            .get("UnmatchedProbeList")
            .map(String::as_str),
        Some("V(missing)")
    );
    assert_eq!(
        execution.wrdata_artifact_records[0]
            .get("Options")
            .map(String::as_str),
        Some(rawfile_option_count.as_str())
    );
    assert_eq!(
        execution.wrdata_artifact_records[0]
            .get("RawfileOptionList")
            .map(String::as_str),
        Some(rawfile_option_list.as_str())
    );
    let wrdata_bytes = execution.wrdata_artifacts[0].datafile.len().to_string();
    assert_eq!(
        execution.wrdata_artifact_records[0]
            .get("Bytes")
            .map(String::as_str),
        Some(wrdata_bytes.as_str())
    );
    assert_eq!(
        execution.wrdata_artifact_table,
        format_deck_wrdata_artifact_table(&execution.wrdata_artifacts)
    );
    assert_eq!(
        execution.wrdata_artifact_csv,
        format_deck_wrdata_artifact_csv(&execution.wrdata_artifacts)
    );
    assert_eq!(
        execution.wrdata_artifact_json,
        format_deck_wrdata_artifact_json(&execution.wrdata_artifacts)
    );
    assert_eq!(
        execution.control_policy_artifact_count,
        expected_codes.len()
    );
    assert_eq!(
        execution
            .control_policy_artifacts
            .iter()
            .map(|artifact| artifact.line_number)
            .collect::<Vec<_>>(),
        expected_policy_lines
    );
    assert_eq!(
        execution
            .control_policy_artifacts
            .iter()
            .map(|artifact| artifact.category.clone())
            .collect::<Vec<_>>(),
        expected_policy_categories
    );
    assert_eq!(
        execution
            .control_policy_artifacts
            .iter()
            .map(|artifact| artifact.command.clone())
            .collect::<Vec<_>>(),
        expected_policy_commands
    );
    assert_eq!(
        execution
            .control_policy_artifacts
            .iter()
            .map(|artifact| artifact.code.clone())
            .collect::<Vec<_>>(),
        expected_codes
    );
    assert_eq!(
        execution
            .control_policy_artifacts
            .iter()
            .map(|artifact| artifact.severity.clone())
            .collect::<Vec<_>>(),
        vec!["error".to_string(); expected_codes.len()]
    );
    assert!(execution.control_policy_artifacts[0]
        .message
        .contains("external script and shell commands are disabled"));
    assert_eq!(
        execution.control_policy_artifact_records[0]
            .get("Line")
            .map(String::as_str),
        Some("13")
    );
    assert_eq!(
        execution.control_policy_artifact_records[0]
            .get("Category")
            .map(String::as_str),
        Some("script")
    );
    assert_eq!(
        execution.control_policy_artifact_records[0]
            .get("Command")
            .map(String::as_str),
        Some("source other.cir")
    );
    assert_eq!(
        execution.control_policy_artifact_records[0]
            .get("Code")
            .map(String::as_str),
        Some("SPICE_DECK_CONTROL_SCRIPT_COMMAND")
    );
    assert_eq!(
        execution.control_policy_artifact_records[0]
            .get("Severity")
            .map(String::as_str),
        Some("error")
    );
    assert_eq!(
        execution.control_policy_artifact_table,
        format_deck_control_policy_artifact_table(&execution.control_policy_artifacts)
    );
    assert_eq!(
        execution.control_policy_artifact_csv,
        format_deck_control_policy_artifact_csv(&execution.control_policy_artifacts)
    );
    assert_eq!(
        execution.control_policy_artifact_json,
        format_deck_control_policy_artifact_json(&execution.control_policy_artifacts)
    );
    assert!(execution
        .control_policy_artifact_json
        .contains("\"Command\":\"let gain = 2\""));
    assert_eq!(
        execution.control_policy_summary_artifact_count,
        expected_policy_categories.len()
    );
    assert_eq!(
        execution
            .control_policy_summary_artifacts
            .iter()
            .map(|artifact| artifact.category.clone())
            .collect::<Vec<_>>(),
        expected_policy_categories
    );
    assert_eq!(
        execution
            .control_policy_summary_artifacts
            .iter()
            .map(|artifact| artifact.artifact_count)
            .collect::<Vec<_>>(),
        vec![1, 1, 1, 1]
    );
    assert_eq!(
        execution
            .control_policy_summary_artifacts
            .iter()
            .map(|artifact| artifact.line_numbers.clone())
            .collect::<Vec<_>>(),
        expected_policy_lines
            .iter()
            .map(|line_number| vec![*line_number])
            .collect::<Vec<_>>()
    );
    assert_eq!(
        execution
            .control_policy_summary_artifacts
            .iter()
            .map(|artifact| artifact.commands.clone())
            .collect::<Vec<_>>(),
        expected_policy_commands
            .iter()
            .map(|command| vec![command.clone()])
            .collect::<Vec<_>>()
    );
    assert_eq!(
        execution
            .control_policy_summary_artifacts
            .iter()
            .map(|artifact| artifact.codes.clone())
            .collect::<Vec<_>>(),
        expected_codes
            .iter()
            .map(|code| vec![code.clone()])
            .collect::<Vec<_>>()
    );
    assert_eq!(
        execution.control_policy_summary_artifact_records[0]
            .get("Category")
            .map(String::as_str),
        Some("script")
    );
    assert_eq!(
        execution.control_policy_summary_artifact_records[0]
            .get("Artifacts")
            .map(String::as_str),
        Some("1")
    );
    assert_eq!(
        execution.control_policy_summary_artifact_records[0]
            .get("LineList")
            .map(String::as_str),
        Some("13")
    );
    assert_eq!(
        execution.control_policy_summary_artifact_records[0]
            .get("CommandList")
            .map(String::as_str),
        Some("source other.cir")
    );
    assert_eq!(
        execution.control_policy_summary_artifact_records[0]
            .get("CodeList")
            .map(String::as_str),
        Some("SPICE_DECK_CONTROL_SCRIPT_COMMAND")
    );
    assert_eq!(
        execution.control_policy_summary_artifact_records[0]
            .get("SeverityList")
            .map(String::as_str),
        Some("error")
    );
    assert_eq!(
        execution.control_policy_summary_artifact_table,
        format_deck_control_policy_summary_artifact_table(
            &execution.control_policy_summary_artifacts
        )
    );
    assert_eq!(
        execution.control_policy_summary_artifact_csv,
        format_deck_control_policy_summary_artifact_csv(
            &execution.control_policy_summary_artifacts
        )
    );
    assert_eq!(
        execution.control_policy_summary_artifact_json,
        format_deck_control_policy_summary_artifact_json(
            &execution.control_policy_summary_artifacts
        )
    );
    assert!(execution
        .control_policy_summary_artifact_json
        .contains("\"CommandList\":\"let gain = 2\""));
    assert_eq!(execution.diagnostic_count, expected_codes.len());
    assert_eq!(execution.diagnostic_codes, expected_codes);
    assert_eq!(execution.table_count, expected_table_names.len());
    assert_eq!(execution.tables, expected_table_names);
    assert_eq!(
        execution
            .table_artifacts
            .iter()
            .map(|artifact| artifact.name.clone())
            .collect::<Vec<_>>(),
        expected_table_names
    );
    let policy_table_artifact = &execution.table_artifacts[execution.table_artifacts.len() - 4];
    assert_eq!(policy_table_artifact.name, "control-policy");
    assert_eq!(
        policy_table_artifact.table,
        execution.control_policy_artifact_table
    );
    assert_eq!(
        policy_table_artifact.csv,
        execution.control_policy_artifact_csv
    );
    assert_eq!(
        policy_table_artifact.json,
        format_deck_table_json(&execution.control_policy_artifact_table)
    );
    assert_eq!(
        policy_table_artifact.records,
        execution.control_policy_artifact_records
    );
    let summary_table_artifact = &execution.table_artifacts[execution.table_artifacts.len() - 3];
    assert_eq!(summary_table_artifact.name, "control-policy-summary");
    assert_eq!(
        summary_table_artifact.table,
        execution.control_policy_summary_artifact_table
    );
    assert_eq!(
        summary_table_artifact.csv,
        execution.control_policy_summary_artifact_csv
    );
    assert_eq!(
        summary_table_artifact.json,
        format_deck_table_json(&execution.control_policy_summary_artifact_table)
    );
    assert_eq!(
        summary_table_artifact.records,
        execution.control_policy_summary_artifact_records
    );
    let output_plan_table_artifact =
        &execution.table_artifacts[execution.table_artifacts.len() - 2];
    assert_eq!(output_plan_table_artifact.name, "output-plan");
    assert_eq!(
        output_plan_table_artifact.table,
        execution.output_plan_artifact_table
    );
    assert_eq!(
        output_plan_table_artifact.csv,
        execution.output_plan_artifact_csv
    );
    assert_eq!(
        output_plan_table_artifact.json,
        execution.output_plan_artifact_json
    );
    assert_eq!(
        output_plan_table_artifact.records,
        execution.output_plan_artifact_records
    );
    assert_eq!(
        execution.run_artifacts[0].control_line_count,
        expected_control_lines.len()
    );
    assert_eq!(
        execution.run_artifacts[0].control_lines,
        expected_control_lines
    );
    assert_eq!(
        execution.run_artifacts[0].write_marker_count,
        expected_write_markers.len()
    );
    assert_eq!(
        execution.run_artifacts[0].write_markers,
        expected_write_markers
    );
    assert_eq!(
        execution.run_artifacts[0].rawfile_option_count,
        expected_rawfile_options.len()
    );
    assert_eq!(
        execution.run_artifacts[0].rawfile_options,
        expected_rawfile_options
    );
    assert_eq!(
        execution.run_artifacts[0].control_policy_artifact_count,
        expected_codes.len()
    );
    assert_eq!(
        execution.run_artifacts[0].control_policy_categories,
        expected_policy_categories
    );
    assert_eq!(
        execution.run_artifacts[0].control_policy_codes,
        expected_codes
    );
    assert_eq!(
        execution.run_artifacts[0].control_policy_severities,
        vec!["error".to_string()]
    );
    assert_eq!(
        execution.run_artifacts[0].table_count,
        expected_table_names.len()
    );
    assert_eq!(execution.run_artifacts[0].tables, expected_table_names);
    assert_eq!(
        execution.run_artifacts[0].diagnostic_count,
        expected_codes.len()
    );
    assert_eq!(execution.run_artifacts[0].diagnostic_codes, expected_codes);
    let records = deck_table_records(&execution.run_artifact_table);
    assert_eq!(records[0].get("Tables").map(String::as_str), Some("5"));
    assert_eq!(
        records[0].get("TableList").map(String::as_str),
        Some(table_list.as_str())
    );
    assert_eq!(
        records[0].get("ControlLines").map(String::as_str),
        Some("2")
    );
    assert_eq!(
        records[0].get("ControlLineList").map(String::as_str),
        Some(control_line_list.as_str())
    );
    assert_eq!(
        records[0].get("WriteMarkers").map(String::as_str),
        Some("2")
    );
    assert_eq!(
        records[0].get("WriteMarkerList").map(String::as_str),
        Some(write_marker_list.as_str())
    );
    assert_eq!(
        records[0].get("RawfileOptions").map(String::as_str),
        Some("5")
    );
    assert_eq!(
        records[0].get("RawfileOptionList").map(String::as_str),
        Some(rawfile_option_list.as_str())
    );
    assert_eq!(
        records[0].get("ControlPolicyArtifacts").map(String::as_str),
        Some("4")
    );
    assert_eq!(
        records[0]
            .get("ControlPolicyCategoryList")
            .map(String::as_str),
        Some(policy_category_list.as_str())
    );
    assert_eq!(
        records[0].get("ControlPolicyCodeList").map(String::as_str),
        Some(code_list.as_str())
    );
    assert_eq!(
        records[0]
            .get("ControlPolicySeverityList")
            .map(String::as_str),
        Some("error")
    );
    assert_eq!(records[0].get("Diagnostics").map(String::as_str), Some("4"));
    assert_eq!(
        records[0].get("DiagnosticCodeList").map(String::as_str),
        Some(code_list.as_str())
    );
    let run_artifact = execution.table_artifacts.last().unwrap();
    assert_eq!(run_artifact.name, "run-artifact");
    assert_eq!(
        run_artifact.records[0]
            .get("ControlLineList")
            .map(String::as_str),
        Some(control_line_list.as_str())
    );
    assert_eq!(
        run_artifact.records[0]
            .get("WriteMarkerList")
            .map(String::as_str),
        Some(write_marker_list.as_str())
    );
    assert_eq!(
        run_artifact.records[0]
            .get("RawfileOptionList")
            .map(String::as_str),
        Some(rawfile_option_list.as_str())
    );
    assert_eq!(
        run_artifact.records[0]
            .get("ControlPolicyCategoryList")
            .map(String::as_str),
        Some(policy_category_list.as_str())
    );
    assert_eq!(
        run_artifact.records[0]
            .get("ControlPolicyCodeList")
            .map(String::as_str),
        Some(code_list.as_str())
    );
    assert_eq!(
        run_artifact.records[0]
            .get("ControlPolicySeverityList")
            .map(String::as_str),
        Some("error")
    );
    assert_eq!(
        run_artifact.records[0]
            .get("DiagnosticCodeList")
            .map(String::as_str),
        Some(code_list.as_str())
    );
    assert_eq!(
        run_artifact.csv,
        format_deck_run_artifact_csv(&execution.run_artifacts)
    );
    assert_eq!(
        run_artifact.json,
        format_deck_run_artifact_json(&execution.run_artifacts)
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
    assert_eq!(op_execution.table_count, 3);
    assert_eq!(
        op_execution.tables,
        vec![
            "result".to_string(),
            "output-plan".to_string(),
            "run-artifact".to_string()
        ]
    );

    let tran_execution = run_deck_analysis(&circuit, netlist, Some("tran")).unwrap();
    assert_eq!(tran_execution.fourier.len(), 1);
    assert_eq!(tran_execution.table_count, 4);
    assert_eq!(
        tran_execution.tables,
        vec![
            "result".to_string(),
            "fourier".to_string(),
            "output-plan".to_string(),
            "run-artifact".to_string()
        ]
    );
    assert_eq!(
        tran_execution
            .table_artifacts
            .iter()
            .map(|artifact| artifact.name.as_str())
            .collect::<Vec<_>>(),
        vec!["result", "fourier", "output-plan", "run-artifact"]
    );
    let result = &tran_execution.fourier[0];
    assert!((result.fundamental_frequency_hz - 2_000.0).abs() < 1.0e-12);
    assert_eq!(result.probes[0].probe, "V(mid)");
    assert_eq!(result.probes[0].harmonics.len(), 1);
    assert_eq!(tran_execution.fourier_table, format_fourier_table(result));
    assert_eq!(
        tran_execution.table_artifacts[1].table,
        tran_execution.fourier_table
    );
    assert_eq!(
        tran_execution.table_artifacts[1].csv,
        format_deck_table_csv(&tran_execution.fourier_table)
    );
    assert_eq!(
        tran_execution.table_artifacts[1].json,
        format_deck_table_json(&tran_execution.fourier_table)
    );
    assert_eq!(
        tran_execution.table_artifacts[1].records,
        deck_table_records(&tran_execution.fourier_table)
    );
    assert_eq!(tran_execution.run_artifacts[0].fourier_count, 1);
    assert_eq!(tran_execution.run_artifacts[0].source_name, None);
    assert_eq!(tran_execution.run_artifacts[0].output_node, None);
    assert_eq!(tran_execution.run_artifacts[0].step_time, Some(5.0e-4));
    assert_eq!(tran_execution.run_artifacts[0].stop_time, Some(1.0e-3));
    assert_eq!(tran_execution.run_artifacts[0].result_column_count, 3);
    assert_eq!(
        tran_execution.run_artifacts[0].result_columns,
        vec![
            "Index".to_string(),
            "Time".to_string(),
            "V(mid)".to_string()
        ]
    );
    assert_eq!(tran_execution.run_artifacts[0].table_count, 4);
    assert_eq!(
        tran_execution.run_artifacts[0].tables,
        vec![
            "result".to_string(),
            "fourier".to_string(),
            "output-plan".to_string(),
            "run-artifact".to_string()
        ]
    );
    assert_eq!(tran_execution.run_artifacts[0].start_time, None);
    assert_eq!(tran_execution.run_artifacts[0].max_step, None);
    assert_eq!(
        tran_execution.run_artifacts[0].use_initial_conditions,
        Some(false)
    );
    assert_eq!(
        tran_execution.run_artifacts[0].output_probes,
        vec!["V(mid)".to_string()]
    );
    assert_eq!(
        tran_execution.run_artifacts[0].output_directives,
        vec![".save".to_string()]
    );
    assert!(tran_execution.run_artifacts[0].measurement_names.is_empty());
    assert_eq!(
        tran_execution.run_artifacts[0].fourier_probes,
        vec!["V(mid)".to_string()]
    );
    let tran_run_artifact_record = assert_run_artifact_table_matches(&tran_execution);
    assert_eq!(
        tran_run_artifact_record.get("Analysis").map(String::as_str),
        Some("tran")
    );
    assert_eq!(
        tran_run_artifact_record.get("Fourier").map(String::as_str),
        Some("1")
    );
    assert_eq!(
        tran_run_artifact_record
            .get("FourierList")
            .map(String::as_str),
        Some("V(mid)")
    );
    assert_eq!(
        tran_run_artifact_record
            .get("DeckAnalysisKindList")
            .map(String::as_str),
        Some("op;tran")
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
