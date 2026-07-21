use std::collections::BTreeMap;

use spice_engine::{
    ac_sweep, ac_sweep_corners, ac_sweep_corners_parallel, device_model_capacitance_audit_fixtures,
    format_ac_table, format_corner_ac_table, format_corner_s_parameter_table,
    format_measurement_table, format_s_parameter_table, measure_ac_sweep_deck,
    measure_ac_sweep_probe, s_parameters, s_parameters_corners, s_parameters_corners_parallel,
    AcPoint, Bjt, BjtPolarity, Capacitor, Cccs, Ccvs, Circuit, Complex, CornerOverride, CornerSpec,
    CurrentSource, Diode, Element, Inductor, Jfet, JfetPolarity, Mosfet, MosfetLevel1Params,
    MosfetType, MutualInductor, Resistor, SpiceError, TransmissionLine, Vcvs, VoltageSource,
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
fn ac_large_resistor_ladder_uses_sparse_complex_solver_path() {
    let mut circuit = Circuit::new();
    circuit.add(Element::VoltageSource(VoltageSource::with_ac(
        "V1", "n0", "0", 0.0, 1.0, 0.0,
    )));
    for index in 0..34 {
        circuit.add(Element::Resistor(Resistor::new(
            format!("R{index}"),
            format!("n{index}"),
            format!("n{}", index + 1),
            1_000.0,
        )));
    }
    circuit.add(Element::Resistor(Resistor::new("R34", "n34", "0", 1_000.0)));

    let points = ac_sweep(&circuit, 1_000.0, 1_000.0, 1).unwrap();

    assert_eq!(points.len(), 1);
    let output = points[0].voltage("n34").unwrap();
    assert_close(output.real, 1.0 / 35.0);
    assert_close(output.imag, 0.0);
}

#[test]
fn ac_text_output_table_is_stable() {
    let resistance = 1_000.0;
    let capacitance = 1.0e-6;
    let corner = 1.0 / (2.0 * std::f64::consts::PI * resistance * capacitance);

    let mut circuit = Circuit::new();
    circuit.add(Element::VoltageSource(VoltageSource::with_ac(
        "V1", "in", "0", 0.0, 1.0, 0.0,
    )));
    circuit.add(Element::Resistor(Resistor::new(
        "R1", "in", "out", resistance,
    )));
    circuit.add(Element::Capacitor(Capacitor::new(
        "C1",
        "out",
        "0",
        capacitance,
    )));

    let points = ac_sweep(&circuit, corner, corner, 10).unwrap();

    assert_eq!(
        format_ac_table(&points, &["V(out)", "I(V1)"]).unwrap(),
        "Index\tFrequency\tProbe\tReal\tImaginary\tMagnitude\tPhase\n0\t1.591549e+02\tV(out)\t5.000000e-01\t-5.000000e-01\t7.071068e-01\t-4.500000e+01\n0\t1.591549e+02\tI(V1)\t-5.000000e-04\t-5.000000e-04\t7.071068e-04\t-1.350000e+02\n"
    );
}

#[test]
fn ac_sweep_measurements_execute_probe_and_parsed_cards() {
    let points = vec![
        AcPoint {
            frequency_hz: 10.0,
            node_voltages: BTreeMap::from([("out".to_string(), Complex::new(1.0, 0.0))]),
            branch_currents: BTreeMap::new(),
        },
        AcPoint {
            frequency_hz: 100.0,
            node_voltages: BTreeMap::from([("out".to_string(), Complex::new(0.0, 2.0))]),
            branch_currents: BTreeMap::new(),
        },
        AcPoint {
            frequency_hz: 1_000.0,
            node_voltages: BTreeMap::from([("out".to_string(), Complex::new(0.0, 0.5))]),
            branch_currents: BTreeMap::new(),
        },
    ];

    let peak = measure_ac_sweep_probe(
        &points,
        "out_peak",
        "V(out)",
        "max",
        Some(10.0),
        Some(100.0),
    )
    .unwrap();
    let average = measure_ac_sweep_probe(&points, "out_avg", "V(out)", "avg", None, None).unwrap();

    assert_close(peak.value, 2.0);
    assert_eq!(peak.analysis, "ac");
    assert_close(average.value, 1.1666666666666667);
    assert_eq!(
        format_measurement_table(&[peak, average]),
        "Name\tAnalysis\tProbe\tMode\tFrom\tTo\tValue\nout_peak\tac\tV(out)\tmax\t1.000000e+01\t1.000000e+02\t2.000000e+00\nout_avg\tac\tV(out)\tavg\t\t\t1.166667e+00\n"
    );

    let measurements = measure_ac_sweep_deck(
        &points,
        "
.measure ac out_swing PP V(out) FROM=10 TO=1000
.meas ac out_final FINAL V(out)
.end
",
    )
    .unwrap();

    assert_eq!(
        format_measurement_table(&measurements),
        "Name\tAnalysis\tProbe\tMode\tFrom\tTo\tValue\nout_swing\tac\tV(out)\tpp\t1.000000e+01\t1.000000e+03\t1.500000e+00\nout_final\tac\tV(out)\tlast\t\t\t5.000000e-01\n"
    );
}

#[test]
fn device_model_capacitance_audit_fixtures_run_reference_ac_points() {
    let fixtures = device_model_capacitance_audit_fixtures().unwrap();
    assert_eq!(
        fixtures
            .iter()
            .map(|fixture| fixture.name.as_str())
            .collect::<Vec<_>>(),
        vec![
            "diode-capacitance-ac",
            "bjt-capacitance-ac",
            "jfet-capacitance-ac",
            "mos-level1-capacitance-ac"
        ]
    );

    for fixture in &fixtures {
        let points = ac_sweep(
            &fixture.circuit,
            fixture.frequency_hz,
            fixture.frequency_hz,
            1,
        )
        .unwrap();
        let value = points[0]
            .voltage(&fixture.probe_node)
            .expect("fixture probe node should be present")
            .abs();
        assert!(
            value >= fixture.expected_magnitude_min && value <= fixture.expected_magnitude_max,
            "{} expected {} <= {} <= {}",
            fixture.name,
            fixture.expected_magnitude_min,
            value,
            fixture.expected_magnitude_max
        );
        assert!(fixture.deck_lines[0].starts_with("* device-model capacitance fixture:"));
        assert!(fixture
            .deck_lines
            .iter()
            .any(|line| line.starts_with(".model ")));
        assert!(fixture
            .deck_lines
            .iter()
            .any(|line| line.starts_with(".ac ")));
        assert!(!fixture.capacitance_behavior.is_empty());
    }

    let jfet_fixture = fixtures
        .iter()
        .find(|fixture| fixture.kind.as_str() == "NJF")
        .expect("JFET capacitance fixture should be present");
    assert!(jfet_fixture.capacitance_behavior.contains("CGS/CGD"));
}

#[test]
fn ac_corner_text_output_table_is_stable() {
    let resistance = 1_000.0;
    let capacitance = 1.0e-6;
    let corner = 1.0 / (2.0 * std::f64::consts::PI * resistance * capacitance);

    let mut circuit = Circuit::new();
    circuit.add(Element::VoltageSource(VoltageSource::with_ac(
        "V1", "in", "0", 0.0, 1.0, 0.0,
    )));
    circuit.add(Element::Resistor(Resistor::new(
        "R1", "in", "out", resistance,
    )));
    circuit.add(Element::Capacitor(Capacitor::new(
        "C1",
        "out",
        "0",
        capacitance,
    )));

    let result = ac_sweep_corners(
        &circuit,
        corner,
        corner,
        10,
        &[
            CornerSpec::new("nominal", Vec::new()),
            CornerSpec::new(
                "r-fast",
                vec![CornerOverride::new("R1", "resistance", 500.0)],
            ),
        ],
    )
    .unwrap();

    assert_eq!(
        format_corner_ac_table(&result, &["V(out)", "I(V1)"]).unwrap(),
        "Corner\tIndex\tFrequency\tProbe\tReal\tImaginary\tMagnitude\tPhase\nnominal\t0\t1.591549e+02\tV(out)\t5.000000e-01\t-5.000000e-01\t7.071068e-01\t-4.500000e+01\nnominal\t0\t1.591549e+02\tI(V1)\t-5.000000e-04\t-5.000000e-04\t7.071068e-04\t-1.350000e+02\nr-fast\t0\t1.591549e+02\tV(out)\t8.000000e-01\t-4.000000e-01\t8.944272e-01\t-2.656505e+01\nr-fast\t0\t1.591549e+02\tI(V1)\t-4.000000e-04\t-8.000000e-04\t8.944272e-04\t-1.165651e+02\n"
    );
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
fn ac_diode_junction_capacitance_shunts_high_frequency() {
    let mut circuit = Circuit::new();
    circuit.add(Element::VoltageSource(VoltageSource::with_ac(
        "Vac", "in", "0", 0.0, 1.0, 0.0,
    )));
    circuit.add(Element::Resistor(Resistor::new(
        "R1", "in", "node", 1_000.0,
    )));
    circuit.add(Element::Diode(Diode::with_model_and_breakdown(
        "D1", "0", "node", 1.0e-15, 0.02585, 1.0, None, 1.0e-3, 1.0e-6, 0.0,
    )));

    let points = ac_sweep(&circuit, 10.0, 100_000.0, 2).unwrap();
    let low = points[0].voltage("node").unwrap().abs();
    let high = points[points.len() - 1].voltage("node").unwrap().abs();

    assert!(low > 0.9, "expected low-frequency pass, got {low}");
    assert!(
        high < low / 100.0,
        "expected high-frequency shunt, got low={low} high={high}"
    );
}

#[test]
fn ac_diode_depletion_capacitance_falls_with_reverse_bias() {
    fn high_frequency_voltage(dc_bias: f64) -> f64 {
        let mut circuit = Circuit::new();
        circuit.add(Element::VoltageSource(VoltageSource::with_ac(
            "Vac", "in", "0", dc_bias, 1.0, 0.0,
        )));
        circuit.add(Element::Resistor(Resistor::new(
            "R1", "in", "node", 1_000.0,
        )));
        circuit.add(Element::Diode(Diode::with_model_and_depletion(
            "D1", "0", "node", 1.0e-15, 0.02585, 1.0, None, 1.0e-3, 1.0e-6, 0.0, 1.0, 0.5,
        )));
        ac_sweep(&circuit, 100_000.0, 100_000.0, 1).unwrap()[0]
            .voltage("node")
            .unwrap()
            .abs()
    }

    let zero_bias = high_frequency_voltage(0.0);
    let reverse_biased = high_frequency_voltage(4.0);

    assert!(
        reverse_biased > zero_bias * 1.8,
        "expected reverse bias to reduce depletion capacitance, got zero={zero_bias} reverse={reverse_biased}"
    );
}

#[test]
fn ac_diode_forward_depletion_coefficient_shapes_capacitance() {
    fn forward_biased_voltage(coefficient: f64) -> f64 {
        let mut circuit = Circuit::new();
        circuit.add(Element::VoltageSource(VoltageSource::with_ac(
            "Vac", "in", "0", 0.75, 1.0, 0.0,
        )));
        circuit.add(Element::Resistor(Resistor::new(
            "R1", "in", "node", 1_000.0,
        )));
        circuit.add(Element::Diode(Diode::with_model_and_forward_depletion(
            "D1",
            "node",
            "0",
            1.0e-30,
            0.02585,
            1.0,
            None,
            1.0e-3,
            1.0e-6,
            0.0,
            1.0,
            0.5,
            coefficient,
        )));
        ac_sweep(&circuit, 1_000.0, 1_000.0, 1).unwrap()[0]
            .voltage("node")
            .unwrap()
            .abs()
    }

    let early_transition = forward_biased_voltage(0.2);
    let late_transition = forward_biased_voltage(0.8);

    assert!(
        late_transition < early_transition * 0.85,
        "expected FC to shape forward depletion capacitance, got early={early_transition} late={late_transition}"
    );
}

#[test]
fn ac_diode_transit_time_shunts_forward_bias_at_high_frequency() {
    fn high_frequency_anode(transit_time: f64) -> f64 {
        let mut circuit = Circuit::new();
        circuit.add(Element::VoltageSource(VoltageSource::with_ac(
            "Vac", "in", "0", 1.0, 1.0, 0.0,
        )));
        circuit.add(Element::Resistor(Resistor::new("R1", "in", "anode", 1.0e6)));
        circuit.add(Element::Diode(Diode::with_model_and_breakdown(
            "D1",
            "anode",
            "0",
            1.0e-15,
            0.02585,
            1.0,
            None,
            1.0e-3,
            0.0,
            transit_time,
        )));
        ac_sweep(&circuit, 100_000_000.0, 100_000_000.0, 1).unwrap()[0]
            .voltage("anode")
            .unwrap()
            .abs()
    }

    let without_transit = high_frequency_anode(0.0);
    let with_transit = high_frequency_anode(1.0e-6);

    assert!(
        without_transit > 0.01,
        "expected measurable high-frequency voltage without transit time, got {without_transit}"
    );
    assert!(
        with_transit < without_transit / 100.0,
        "expected transit-time high-frequency shunt, got no_tt={without_transit} tt={with_transit}"
    );
}

#[test]
fn ac_sweep_corners_runs_frequency_sweeps_per_corner() {
    let resistance = 1_000.0;
    let capacitance = 1.0e-6;
    let corner = 1.0 / (2.0 * std::f64::consts::PI * resistance * capacitance);

    let mut circuit = Circuit::new();
    circuit.add(Element::VoltageSource(VoltageSource::new(
        "Vin", "in", "0", 1.0,
    )));
    circuit.add(Element::Resistor(Resistor::new(
        "R1", "in", "out", resistance,
    )));
    circuit.add(Element::Capacitor(Capacitor::new(
        "C1",
        "out",
        "0",
        capacitance,
    )));

    let result = ac_sweep_corners(
        &circuit,
        corner,
        corner,
        10,
        &[
            CornerSpec::new("nominal", Vec::new()),
            CornerSpec::new(
                "r-fast",
                vec![CornerOverride::new("R1", "resistance", 500.0)],
            ),
        ],
    )
    .unwrap();

    assert_eq!(result.points[0].corner_name, "nominal");
    assert_eq!(result.points[1].corner_name, "r-fast");
    assert_eq!(result.points[0].points.len(), 1);
    assert_close(result.points[0].points[0].frequency_hz, corner);
    assert_close(
        result.points[0].points[0].voltage("out").unwrap().abs(),
        1.0 / 2.0_f64.sqrt(),
    );
    assert_close(
        result.points[1].points[0].voltage("out").unwrap().abs(),
        1.0 / 1.25_f64.sqrt(),
    );
}

#[test]
fn ac_sweep_corners_parallel_matches_ordered_sequential_results() {
    let resistance = 1_000.0;
    let capacitance = 1.0e-6;
    let corner = 1.0 / (2.0 * std::f64::consts::PI * resistance * capacitance);

    let mut circuit = Circuit::new();
    circuit.add(Element::VoltageSource(VoltageSource::with_ac(
        "V1", "in", "0", 0.0, 1.0, 0.0,
    )));
    circuit.add(Element::Resistor(Resistor::new(
        "R1", "in", "out", resistance,
    )));
    circuit.add(Element::Capacitor(Capacitor::new(
        "C1",
        "out",
        "0",
        capacitance,
    )));
    let corners = [
        CornerSpec::new("nominal", Vec::new()),
        CornerSpec::new(
            "r-fast",
            vec![CornerOverride::new("R1", "resistance", 500.0)],
        ),
        CornerSpec::new(
            "c-large",
            vec![CornerOverride::new("C1", "capacitance", 2.0e-6)],
        ),
    ];

    let sequential = ac_sweep_corners(&circuit, corner, corner, 10, &corners).unwrap();
    let parallel = ac_sweep_corners_parallel(&circuit, corner, corner, 10, &corners).unwrap();

    assert_eq!(parallel.points.len(), sequential.points.len());
    for (parallel_corner, sequential_corner) in parallel.points.iter().zip(sequential.points.iter())
    {
        assert_eq!(parallel_corner.corner_name, sequential_corner.corner_name);
        assert_eq!(parallel_corner.points.len(), sequential_corner.points.len());
        for (parallel_point, sequential_point) in parallel_corner
            .points
            .iter()
            .zip(sequential_corner.points.iter())
        {
            assert_close(parallel_point.frequency_hz, sequential_point.frequency_hz);
            let parallel_vout = parallel_point.voltage("out").unwrap();
            let sequential_vout = sequential_point.voltage("out").unwrap();
            assert_close(parallel_vout.real, sequential_vout.real);
            assert_close(parallel_vout.imag, sequential_vout.imag);
            let parallel_i_v1 = parallel_point.branch_current("V1").unwrap();
            let sequential_i_v1 = sequential_point.branch_current("V1").unwrap();
            assert_close(parallel_i_v1.real, sequential_i_v1.real);
            assert_close(parallel_i_v1.imag, sequential_i_v1.imag);
        }
    }
    assert_eq!(
        format_corner_ac_table(&parallel, &["V(out)", "I(V1)"]).unwrap(),
        "Corner\tIndex\tFrequency\tProbe\tReal\tImaginary\tMagnitude\tPhase\nnominal\t0\t1.591549e+02\tV(out)\t5.000000e-01\t-5.000000e-01\t7.071068e-01\t-4.500000e+01\nnominal\t0\t1.591549e+02\tI(V1)\t-5.000000e-04\t-5.000000e-04\t7.071068e-04\t-1.350000e+02\nr-fast\t0\t1.591549e+02\tV(out)\t8.000000e-01\t-4.000000e-01\t8.944272e-01\t-2.656505e+01\nr-fast\t0\t1.591549e+02\tI(V1)\t-4.000000e-04\t-8.000000e-04\t8.944272e-04\t-1.165651e+02\nc-large\t0\t1.591549e+02\tV(out)\t2.000000e-01\t-4.000000e-01\t4.472136e-01\t-6.343495e+01\nc-large\t0\t1.591549e+02\tI(V1)\t-8.000000e-04\t-4.000000e-04\t8.944272e-04\t-1.534349e+02\n"
    );
}

#[test]
fn ac_sweep_corners_parallel_rejects_invalid_sweep_before_workers() {
    let circuit = Circuit::new();
    let corners = [CornerSpec::new("nominal", Vec::new())];

    assert!(matches!(
        ac_sweep_corners_parallel(&circuit, 0.0, 1_000.0, 10, &corners),
        Err(SpiceError::InvalidElement { name, reason })
            if name == "ac_sweep" && reason.contains("positive")
    ));
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
fn ac_mutual_inductor_transformer_ratio() {
    let primary_l: f64 = 1.0e-3;
    let secondary_l: f64 = 4.0e-3;
    let coupling: f64 = 0.9;
    let load: f64 = 1_000.0;
    let frequency: f64 = 1_000.0;
    let mutual_l = coupling * (primary_l * secondary_l).sqrt();
    let denom_real: f64 = 1.0;
    let denom_imag = 2.0 * std::f64::consts::PI * frequency * secondary_l / load;
    let numerator_imag = 2.0 * std::f64::consts::PI * frequency * mutual_l;
    let scale = denom_real.powi(2) + denom_imag.powi(2);
    let expected_real = numerator_imag * denom_imag / scale;
    let expected_imag = numerator_imag * denom_real / scale;

    let mut circuit = Circuit::new();
    circuit.add(Element::CurrentSource(CurrentSource::with_ac(
        "Iin", "0", "pri", 0.0, 1.0, 0.0,
    )));
    circuit.add(Element::Inductor(Inductor::new(
        "Lpri", "pri", "0", primary_l,
    )));
    circuit.add(Element::Inductor(Inductor::new(
        "Lsec",
        "sec",
        "0",
        secondary_l,
    )));
    circuit.add(Element::MutualInductor(MutualInductor::new(
        "K1", "Lpri", "Lsec", coupling,
    )));
    circuit.add(Element::Resistor(Resistor::new("Rload", "sec", "0", load)));

    let points = ac_sweep(&circuit, frequency, frequency, 10).unwrap();

    let secondary = points[0].voltage("sec").unwrap();
    assert_close(secondary.real, expected_real);
    assert_close(secondary.imag, expected_imag);
}

#[test]
fn ac_mutual_inductor_rejects_missing_reference() {
    let mut circuit = Circuit::new();
    circuit.add(Element::VoltageSource(VoltageSource::new(
        "Vin", "pri", "0", 1.0,
    )));
    circuit.add(Element::Inductor(Inductor::new("Lpri", "pri", "0", 1.0e-3)));
    circuit.add(Element::MutualInductor(MutualInductor::new(
        "Kbad", "Lpri", "Lmissing", 0.9,
    )));

    assert!(matches!(
        ac_sweep(&circuit, 1_000.0, 1_000.0, 10),
        Err(SpiceError::InvalidElement { name, .. }) if name == "Kbad"
    ));
}

#[test]
fn ac_transmission_line_matched_load_phase_delay() {
    let frequency: f64 = 1_000_000.0;
    let delay = 1.0 / (4.0 * frequency);
    let mut circuit = Circuit::new();
    circuit.add(Element::VoltageSource(VoltageSource::with_ac(
        "Vin", "src", "0", 0.0, 1.0, 0.0,
    )));
    circuit.add(Element::Resistor(Resistor::new("Rsrc", "src", "in", 50.0)));
    circuit.add(Element::TransmissionLine(TransmissionLine::new(
        "T1", "in", "0", "out", "0", 50.0, delay,
    )));
    circuit.add(Element::Resistor(Resistor::new("Rload", "out", "0", 50.0)));

    let points = ac_sweep(&circuit, frequency, frequency, 10).unwrap();

    let out = points[0].voltage("out").unwrap();
    assert_close(out.real, 0.0);
    assert_close(out.imag, -0.5);
}

#[test]
fn ac_transmission_line_rejects_invalid_parameters() {
    let mut circuit = Circuit::new();
    circuit.add(Element::VoltageSource(VoltageSource::with_ac(
        "Vin", "src", "0", 0.0, 1.0, 0.0,
    )));
    circuit.add(Element::TransmissionLine(TransmissionLine::new(
        "Tbad", "src", "0", "out", "0", 0.0, 1.0e-9,
    )));
    circuit.add(Element::Resistor(Resistor::new("Rload", "out", "0", 50.0)));

    assert!(matches!(
        ac_sweep(&circuit, 1_000.0, 1_000.0, 10),
        Err(SpiceError::InvalidElement { name, .. }) if name == "Tbad"
    ));
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
fn ac_bjt_applies_zero_bias_common_emitter_gain() {
    let mut circuit = Circuit::new();
    circuit.add(Element::VoltageSource(VoltageSource::new(
        "Vin", "base", "0", 1.0,
    )));
    circuit.add(Element::Bjt(Bjt::with_model(
        "Q1",
        "out",
        "base",
        "0",
        BjtPolarity::Npn,
        25.85e-6,
        100.0,
        0.02585,
        0.0,
        0.0,
        0.0,
        0.0,
    )));
    circuit.add(Element::Resistor(Resistor::new(
        "Rload", "out", "0", 1_000.0,
    )));

    let points = ac_sweep(&circuit, 1_000.0, 1_000.0, 10).unwrap();

    assert_eq!(points.len(), 1);
    let out = points[0].voltage("out").unwrap();
    assert_close(out.real, -1.0);
    assert_close(out.imag, 0.0);
}

#[test]
fn ac_bjt_uses_explicit_ac_source_and_dc_bias_operating_point() {
    let mut circuit = Circuit::new();
    let thermal_voltage = 0.02585;
    circuit.add(Element::VoltageSource(VoltageSource::with_ac(
        "Vin",
        "base",
        "0",
        thermal_voltage * 2.0_f64.ln(),
        1.0,
        0.0,
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

    let points = ac_sweep(&circuit, 1_000.0, 1_000.0, 10).unwrap();

    assert_eq!(points.len(), 1);
    let out = points[0].voltage("out").unwrap();
    assert_close(out.real, -2.0);
    assert_close(out.imag, 0.0);
}

#[test]
fn ac_bjt_uses_base_emitter_capacitance() {
    fn base_amplitude(base_emitter_capacitance: f64) -> f64 {
        let mut circuit = Circuit::new();
        circuit.add(Element::VoltageSource(VoltageSource::with_ac(
            "Vac", "in", "0", 0.0, 1.0, 0.0,
        )));
        circuit.add(Element::Resistor(Resistor::new(
            "Rin", "in", "base", 1_000.0,
        )));
        circuit.add(Element::Resistor(Resistor::new("Rc", "col", "0", 1_000.0)));
        circuit.add(Element::Bjt(Bjt::with_model(
            "Q1",
            "col",
            "base",
            "0",
            BjtPolarity::Npn,
            1.0e-14,
            100.0,
            0.02585,
            base_emitter_capacitance,
            0.0,
            0.0,
            0.0,
        )));

        ac_sweep(&circuit, 100_000.0, 100_000.0, 1).unwrap()[0]
            .voltage("base")
            .unwrap()
            .abs()
    }

    let without_capacitance = base_amplitude(0.0);
    let with_capacitance = base_amplitude(1.0e-6);

    assert!(without_capacitance > 0.9);
    assert!(with_capacitance < without_capacitance / 100.0);
}

#[test]
fn ac_bjt_uses_forward_transit_time_as_diffusion_capacitance() {
    fn base_amplitude(forward_transit_time: f64) -> f64 {
        let mut circuit = Circuit::new();
        circuit.add(Element::VoltageSource(VoltageSource::with_ac(
            "Vac", "in", "0", 0.0, 1.0, 0.0,
        )));
        circuit.add(Element::Resistor(Resistor::new(
            "Rin", "in", "base", 1_000.0,
        )));
        circuit.add(Element::Resistor(Resistor::new("Rc", "col", "0", 1_000.0)));
        circuit.add(Element::Bjt(Bjt::with_model(
            "Q1",
            "col",
            "base",
            "0",
            BjtPolarity::Npn,
            25.85e-6,
            100.0,
            0.02585,
            0.0,
            0.0,
            forward_transit_time,
            0.0,
        )));

        ac_sweep(&circuit, 100_000.0, 100_000.0, 1).unwrap()[0]
            .voltage("base")
            .unwrap()
            .abs()
    }

    let without_transit_time = base_amplitude(0.0);
    let with_transit_time = base_amplitude(1.0e-3);

    assert!(without_transit_time > 0.9);
    assert!(with_transit_time < without_transit_time / 100.0);
}

#[test]
fn ac_bjt_uses_reverse_transit_time_as_base_collector_diffusion_capacitance() {
    fn base_amplitude(reverse_transit_time: f64) -> f64 {
        let mut circuit = Circuit::new();
        circuit.add(Element::VoltageSource(VoltageSource::with_ac(
            "Vac", "in", "0", 0.0, 1.0, 0.0,
        )));
        circuit.add(Element::Resistor(Resistor::new(
            "Rin", "in", "base", 1_000.0,
        )));
        circuit.add(Element::Resistor(Resistor::new("Rc", "col", "0", 1.0)));
        circuit.add(Element::Bjt(Bjt::with_model(
            "Q1",
            "col",
            "base",
            "0",
            BjtPolarity::Npn,
            25.85e-6,
            100.0,
            0.02585,
            0.0,
            0.0,
            0.0,
            reverse_transit_time,
        )));

        ac_sweep(&circuit, 100_000.0, 100_000.0, 1).unwrap()[0]
            .voltage("base")
            .unwrap()
            .abs()
    }

    let without_transit_time = base_amplitude(0.0);
    let with_transit_time = base_amplitude(1.0e-2);

    assert!(without_transit_time > 0.9);
    assert!(with_transit_time < without_transit_time / 100.0);
}

#[test]
fn ac_bjt_reverse_emission_coefficient_reduces_base_collector_diffusion_capacitance() {
    fn base_amplitude(reverse_emission_coefficient: f64) -> f64 {
        let mut circuit = Circuit::new();
        circuit.add(Element::VoltageSource(VoltageSource::with_ac(
            "Vac", "in", "0", 0.0, 1.0, 0.0,
        )));
        circuit.add(Element::Resistor(Resistor::new(
            "Rin", "in", "base", 1_000.0,
        )));
        circuit.add(Element::Resistor(Resistor::new("Rc", "col", "0", 1.0)));
        circuit.add(Element::Bjt(Bjt::with_model_and_temperature_parameters(
            "Q1",
            "col",
            "base",
            "0",
            BjtPolarity::Npn,
            25.85e-6,
            100.0,
            0.02585,
            0.0,
            0.0,
            0.0,
            1.0e-2,
            3.0,
            1.11,
            0.0,
            1.0,
            reverse_emission_coefficient,
        )));

        ac_sweep(&circuit, 100_000.0, 100_000.0, 1).unwrap()[0]
            .voltage("base")
            .unwrap()
            .abs()
    }

    assert!(base_amplitude(2.0) > base_amplitude(1.0));
}

#[test]
fn ac_bjt_base_emitter_depletion_capacitance_falls_with_reverse_bias() {
    fn base_amplitude(grading_coefficient: f64) -> f64 {
        let mut circuit = Circuit::new();
        circuit.add(Element::VoltageSource(VoltageSource::with_ac(
            "Vac", "in", "0", -1.0, 1.0, 0.0,
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
                1.0e-6,
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
            ),
        ));

        ac_sweep(&circuit, 1_000.0, 1_000.0, 1).unwrap()[0]
            .voltage("base")
            .unwrap()
            .abs()
    }

    assert!(base_amplitude(0.5) > base_amplitude(0.0));
}

#[test]
fn ac_bjt_base_collector_depletion_capacitance_falls_with_reverse_bias() {
    fn collector_amplitude(grading_coefficient: f64) -> f64 {
        let mut circuit = Circuit::new();
        circuit.add(Element::VoltageSource(VoltageSource::with_ac(
            "Vac", "in", "0", 1.0, 1.0, 0.0,
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
                1.0e-6,
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
            ),
        ));

        ac_sweep(&circuit, 1_000.0, 1_000.0, 1).unwrap()[0]
            .voltage("collector")
            .unwrap()
            .abs()
    }

    assert!(collector_amplitude(0.5) > collector_amplitude(0.0));
}

#[test]
fn ac_mosfet_common_source_suppresses_dc_supplies_without_ac_spec() {
    let mut circuit = Circuit::new();
    circuit.add(Element::VoltageSource(VoltageSource::new(
        "Vdd", "vdd", "0", 5.0,
    )));
    circuit.add(Element::VoltageSource(VoltageSource::with_ac(
        "Vin", "gate", "0", 1.5, 1.0, 0.0,
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

    let points = ac_sweep(&circuit, 1_000.0, 1_000.0, 10).unwrap();

    assert_eq!(points.len(), 1);
    let out = points[0].voltage("out").unwrap();
    assert_close(out.real, -1.0);
    assert_close(out.imag, 0.0);
    assert_close(points[0].voltage("vdd").unwrap().abs(), 0.0);
}

#[test]
fn ac_mosfet_overlap_capacitance_shunts_high_frequency_gate_drive() {
    fn gate_amplitude(cgso: f64) -> f64 {
        let mut circuit = Circuit::new();
        circuit.add(Element::VoltageSource(VoltageSource::with_ac(
            "Vac", "in", "0", 0.0, 1.0, 0.0,
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
                gate_source_overlap_capacitance: cgso,
                ..MosfetLevel1Params::default()
            },
        )));

        ac_sweep(&circuit, 100_000.0, 100_000.0, 1).unwrap()[0]
            .voltage("gate")
            .unwrap()
            .abs()
    }

    let without_capacitance = gate_amplitude(0.0);
    let with_capacitance = gate_amplitude(1.0e-6);

    assert!(without_capacitance > 0.9);
    assert!(with_capacitance < without_capacitance / 100.0);
}

#[test]
fn ac_jfet_common_source_uses_dc_bias_for_small_signal_gain() {
    let mut circuit = Circuit::new();
    circuit.add(Element::VoltageSource(VoltageSource::new(
        "Vdd", "vdd", "0", 10.0,
    )));
    circuit.add(Element::VoltageSource(VoltageSource::with_ac(
        "Vin", "gate", "0", 0.0, 1.0, 0.0,
    )));
    circuit.add(Element::Resistor(Resistor::new(
        "Rd", "vdd", "drain", 1_000.0,
    )));
    circuit.add(Element::Jfet(Jfet::with_model(
        "J1",
        "drain",
        "gate",
        "0",
        JfetPolarity::Njf,
        1.0e-3,
        -2.0,
        0.0,
    )));

    let points = ac_sweep(&circuit, 1_000.0, 1_000.0, 10).unwrap();

    assert_eq!(points.len(), 1);
    let out = points[0].voltage("drain").unwrap();
    assert_close(out.real, -4.0);
    assert_close(out.imag, 0.0);
    assert_close(points[0].voltage("vdd").unwrap().abs(), 0.0);
}

#[test]
fn ac_jfet_gate_source_capacitance_shunts_high_frequency_gate_drive() {
    fn gate_amplitude(gate_source_capacitance: f64) -> f64 {
        let mut circuit = Circuit::new();
        circuit.add(Element::VoltageSource(VoltageSource::with_ac(
            "Vac", "in", "0", 0.0, 1.0, 0.0,
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
        ac_sweep(&circuit, 100_000.0, 100_000.0, 1).unwrap()[0]
            .voltage("gate")
            .unwrap()
            .abs()
    }

    let without_capacitance = gate_amplitude(0.0);
    let with_capacitance = gate_amplitude(1.0e-6);

    assert!(without_capacitance > 0.9);
    assert!(with_capacitance < without_capacitance / 100.0);
}

#[test]
fn ac_current_source_supports_explicit_magnitude_and_phase() {
    let mut circuit = Circuit::new();
    circuit.add(Element::CurrentSource(CurrentSource::with_ac(
        "I1", "0", "n1", 0.0, 1.0e-3, 90.0,
    )));
    circuit.add(Element::Resistor(Resistor::new("R1", "n1", "0", 1_000.0)));

    let points = ac_sweep(&circuit, 1_000.0, 1_000.0, 10).unwrap();

    assert_eq!(points.len(), 1);
    let n1 = points[0].voltage("n1").unwrap();
    assert!(
        n1.real.abs() < 1.0e-9,
        "expected real near zero, got {n1:?}"
    );
    assert_close(n1.imag, 1.0);
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

#[test]
fn s_parameters_series_resistor_two_port() {
    let mut circuit = Circuit::new();
    circuit.add(Element::VoltageSource(VoltageSource::new(
        "P1", "p1", "0", 0.0,
    )));
    circuit.add(Element::VoltageSource(VoltageSource::new(
        "P2", "p2", "0", 0.0,
    )));
    circuit.add(Element::Resistor(Resistor::new(
        "Rseries", "p1", "p2", 50.0,
    )));

    let result = s_parameters(&circuit, "P1", "P2", &[1.0e6], 50.0).unwrap();
    let point = result.points[0];

    assert_close(point.s11.real, 1.0 / 3.0);
    assert_close(point.s22.real, 1.0 / 3.0);
    assert_close(point.s21.real, 2.0 / 3.0);
    assert_close(point.s12.real, 2.0 / 3.0);
    assert_close(point.s11.imag, 0.0);
    assert_close(point.s21.imag, 0.0);
}

#[test]
fn s_parameter_text_output_table_is_stable() {
    let mut circuit = Circuit::new();
    circuit.add(Element::VoltageSource(VoltageSource::new(
        "P1", "p1", "0", 0.0,
    )));
    circuit.add(Element::VoltageSource(VoltageSource::new(
        "P2", "p2", "0", 0.0,
    )));
    circuit.add(Element::Resistor(Resistor::new(
        "Rseries", "p1", "p2", 50.0,
    )));

    let result = s_parameters(&circuit, "P1", "P2", &[1.0e6], 50.0).unwrap();

    assert_eq!(
        format_s_parameter_table(&result),
        "Index\tFrequency\tPort1\tPort2\tParameter\tReal\tImaginary\tMagnitude\tPhase\n0\t1.000000e+06\tP1\tP2\tS11\t3.333333e-01\t0.000000e+00\t3.333333e-01\t0.000000e+00\n0\t1.000000e+06\tP1\tP2\tS21\t6.666667e-01\t0.000000e+00\t6.666667e-01\t0.000000e+00\n0\t1.000000e+06\tP1\tP2\tS12\t6.666667e-01\t0.000000e+00\t6.666667e-01\t0.000000e+00\n0\t1.000000e+06\tP1\tP2\tS22\t3.333333e-01\t0.000000e+00\t3.333333e-01\t0.000000e+00\n"
    );
}

#[test]
fn s_parameters_corners_runs_two_port_extraction_per_corner() {
    let mut circuit = Circuit::new();
    circuit.add(Element::VoltageSource(VoltageSource::new(
        "P1", "p1", "0", 0.0,
    )));
    circuit.add(Element::VoltageSource(VoltageSource::new(
        "P2", "p2", "0", 0.0,
    )));
    circuit.add(Element::Resistor(Resistor::new(
        "Rseries", "p1", "p2", 50.0,
    )));

    let result = s_parameters_corners(
        &circuit,
        "P1",
        "P2",
        &[1.0e6],
        50.0,
        &[
            CornerSpec::new("nominal", Vec::new()),
            CornerSpec::new(
                "series-high",
                vec![CornerOverride::new("Rseries", "resistance", 100.0)],
            ),
        ],
    )
    .unwrap();

    assert_eq!(result.port1_source, "P1");
    assert_eq!(result.port2_source, "P2");
    assert_close(result.reference_impedance_ohms, 50.0);
    assert_eq!(result.points[0].corner_name, "nominal");
    assert_eq!(result.points[1].corner_name, "series-high");
    assert_close(result.points[0].result.points[0].s21.real, 2.0 / 3.0);
    assert_close(result.points[1].result.points[0].s21.real, 0.5);
    assert_close(result.points[0].result.points[0].s11.real, 1.0 / 3.0);
    assert_close(result.points[1].result.points[0].s11.real, 0.5);
}

#[test]
fn corner_s_parameter_text_output_table_is_stable() {
    let mut circuit = Circuit::new();
    circuit.add(Element::VoltageSource(VoltageSource::new(
        "P1", "p1", "0", 0.0,
    )));
    circuit.add(Element::VoltageSource(VoltageSource::new(
        "P2", "p2", "0", 0.0,
    )));
    circuit.add(Element::Resistor(Resistor::new(
        "Rseries", "p1", "p2", 50.0,
    )));

    let result = s_parameters_corners(
        &circuit,
        "P1",
        "P2",
        &[1.0e6],
        50.0,
        &[
            CornerSpec::new("nominal", Vec::new()),
            CornerSpec::new(
                "series-high",
                vec![CornerOverride::new("Rseries", "resistance", 100.0)],
            ),
        ],
    )
    .unwrap();

    assert_eq!(
        format_corner_s_parameter_table(&result),
        "Corner\tIndex\tFrequency\tPort1\tPort2\tParameter\tReal\tImaginary\tMagnitude\tPhase\nnominal\t0\t1.000000e+06\tP1\tP2\tS11\t3.333333e-01\t0.000000e+00\t3.333333e-01\t0.000000e+00\nnominal\t0\t1.000000e+06\tP1\tP2\tS21\t6.666667e-01\t0.000000e+00\t6.666667e-01\t0.000000e+00\nnominal\t0\t1.000000e+06\tP1\tP2\tS12\t6.666667e-01\t0.000000e+00\t6.666667e-01\t0.000000e+00\nnominal\t0\t1.000000e+06\tP1\tP2\tS22\t3.333333e-01\t0.000000e+00\t3.333333e-01\t0.000000e+00\nseries-high\t0\t1.000000e+06\tP1\tP2\tS11\t5.000000e-01\t0.000000e+00\t5.000000e-01\t0.000000e+00\nseries-high\t0\t1.000000e+06\tP1\tP2\tS21\t5.000000e-01\t0.000000e+00\t5.000000e-01\t0.000000e+00\nseries-high\t0\t1.000000e+06\tP1\tP2\tS12\t5.000000e-01\t0.000000e+00\t5.000000e-01\t0.000000e+00\nseries-high\t0\t1.000000e+06\tP1\tP2\tS22\t5.000000e-01\t0.000000e+00\t5.000000e-01\t0.000000e+00\n"
    );
}

#[test]
fn s_parameters_corners_parallel_matches_ordered_sequential_results() {
    let mut circuit = Circuit::new();
    circuit.add(Element::VoltageSource(VoltageSource::new(
        "P1", "p1", "0", 0.0,
    )));
    circuit.add(Element::VoltageSource(VoltageSource::new(
        "P2", "p2", "0", 0.0,
    )));
    circuit.add(Element::Resistor(Resistor::new(
        "Rseries", "p1", "p2", 50.0,
    )));
    let corners = [
        CornerSpec::new("nominal", Vec::new()),
        CornerSpec::new(
            "series-high",
            vec![CornerOverride::new("Rseries", "resistance", 100.0)],
        ),
        CornerSpec::new(
            "series-low",
            vec![CornerOverride::new("Rseries", "resistance", 25.0)],
        ),
    ];
    let frequencies_hz = [1.0e6, 2.0e6];

    let sequential =
        s_parameters_corners(&circuit, "P1", "P2", &frequencies_hz, 50.0, &corners).unwrap();
    let parallel =
        s_parameters_corners_parallel(&circuit, "P1", "P2", &frequencies_hz, 50.0, &corners)
            .unwrap();

    assert_eq!(parallel.port1_source, sequential.port1_source);
    assert_eq!(parallel.port2_source, sequential.port2_source);
    assert_close(
        parallel.reference_impedance_ohms,
        sequential.reference_impedance_ohms,
    );
    assert_eq!(parallel.points.len(), sequential.points.len());
    for (parallel_corner, sequential_corner) in parallel.points.iter().zip(sequential.points.iter())
    {
        assert_eq!(parallel_corner.corner_name, sequential_corner.corner_name);
        assert_eq!(
            parallel_corner.result.points.len(),
            sequential_corner.result.points.len()
        );
        for (parallel_point, sequential_point) in parallel_corner
            .result
            .points
            .iter()
            .zip(sequential_corner.result.points.iter())
        {
            assert_close(parallel_point.frequency_hz, sequential_point.frequency_hz);
            for (parallel_value, sequential_value) in [
                (parallel_point.s11, sequential_point.s11),
                (parallel_point.s21, sequential_point.s21),
                (parallel_point.s12, sequential_point.s12),
                (parallel_point.s22, sequential_point.s22),
            ] {
                assert_close(parallel_value.real, sequential_value.real);
                assert_close(parallel_value.imag, sequential_value.imag);
            }
        }
    }
    assert_eq!(
        format_corner_s_parameter_table(&parallel),
        "Corner\tIndex\tFrequency\tPort1\tPort2\tParameter\tReal\tImaginary\tMagnitude\tPhase\nnominal\t0\t1.000000e+06\tP1\tP2\tS11\t3.333333e-01\t0.000000e+00\t3.333333e-01\t0.000000e+00\nnominal\t0\t1.000000e+06\tP1\tP2\tS21\t6.666667e-01\t0.000000e+00\t6.666667e-01\t0.000000e+00\nnominal\t0\t1.000000e+06\tP1\tP2\tS12\t6.666667e-01\t0.000000e+00\t6.666667e-01\t0.000000e+00\nnominal\t0\t1.000000e+06\tP1\tP2\tS22\t3.333333e-01\t0.000000e+00\t3.333333e-01\t0.000000e+00\nnominal\t1\t2.000000e+06\tP1\tP2\tS11\t3.333333e-01\t0.000000e+00\t3.333333e-01\t0.000000e+00\nnominal\t1\t2.000000e+06\tP1\tP2\tS21\t6.666667e-01\t0.000000e+00\t6.666667e-01\t0.000000e+00\nnominal\t1\t2.000000e+06\tP1\tP2\tS12\t6.666667e-01\t0.000000e+00\t6.666667e-01\t0.000000e+00\nnominal\t1\t2.000000e+06\tP1\tP2\tS22\t3.333333e-01\t0.000000e+00\t3.333333e-01\t0.000000e+00\nseries-high\t0\t1.000000e+06\tP1\tP2\tS11\t5.000000e-01\t0.000000e+00\t5.000000e-01\t0.000000e+00\nseries-high\t0\t1.000000e+06\tP1\tP2\tS21\t5.000000e-01\t0.000000e+00\t5.000000e-01\t0.000000e+00\nseries-high\t0\t1.000000e+06\tP1\tP2\tS12\t5.000000e-01\t0.000000e+00\t5.000000e-01\t0.000000e+00\nseries-high\t0\t1.000000e+06\tP1\tP2\tS22\t5.000000e-01\t0.000000e+00\t5.000000e-01\t0.000000e+00\nseries-high\t1\t2.000000e+06\tP1\tP2\tS11\t5.000000e-01\t0.000000e+00\t5.000000e-01\t0.000000e+00\nseries-high\t1\t2.000000e+06\tP1\tP2\tS21\t5.000000e-01\t0.000000e+00\t5.000000e-01\t0.000000e+00\nseries-high\t1\t2.000000e+06\tP1\tP2\tS12\t5.000000e-01\t0.000000e+00\t5.000000e-01\t0.000000e+00\nseries-high\t1\t2.000000e+06\tP1\tP2\tS22\t5.000000e-01\t0.000000e+00\t5.000000e-01\t0.000000e+00\nseries-low\t0\t1.000000e+06\tP1\tP2\tS11\t2.000000e-01\t0.000000e+00\t2.000000e-01\t0.000000e+00\nseries-low\t0\t1.000000e+06\tP1\tP2\tS21\t8.000000e-01\t0.000000e+00\t8.000000e-01\t0.000000e+00\nseries-low\t0\t1.000000e+06\tP1\tP2\tS12\t8.000000e-01\t0.000000e+00\t8.000000e-01\t0.000000e+00\nseries-low\t0\t1.000000e+06\tP1\tP2\tS22\t2.000000e-01\t0.000000e+00\t2.000000e-01\t0.000000e+00\nseries-low\t1\t2.000000e+06\tP1\tP2\tS11\t2.000000e-01\t0.000000e+00\t2.000000e-01\t0.000000e+00\nseries-low\t1\t2.000000e+06\tP1\tP2\tS21\t8.000000e-01\t0.000000e+00\t8.000000e-01\t0.000000e+00\nseries-low\t1\t2.000000e+06\tP1\tP2\tS12\t8.000000e-01\t0.000000e+00\t8.000000e-01\t0.000000e+00\nseries-low\t1\t2.000000e+06\tP1\tP2\tS22\t2.000000e-01\t0.000000e+00\t2.000000e-01\t0.000000e+00\n"
    );
}

#[test]
fn s_parameters_corners_parallel_reports_corner_override_errors() {
    let mut circuit = Circuit::new();
    circuit.add(Element::VoltageSource(VoltageSource::new(
        "P1", "p1", "0", 0.0,
    )));
    circuit.add(Element::VoltageSource(VoltageSource::new(
        "P2", "p2", "0", 0.0,
    )));
    circuit.add(Element::Resistor(Resistor::new(
        "Rseries", "p1", "p2", 50.0,
    )));
    let corners = [CornerSpec::new(
        "missing",
        vec![CornerOverride::new("Rmissing", "resistance", 100.0)],
    )];

    assert!(matches!(
        s_parameters_corners_parallel(&circuit, "P1", "P2", &[1.0e6], 50.0, &corners),
        Err(SpiceError::InvalidElement { name, reason })
            if name == "dc_corners" && reason.contains("Rmissing")
    ));
}
