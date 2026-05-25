use spice_engine::{
    format_noise_table, noise_ac, noise_ac_corners, noise_ac_default, Capacitor, Circuit,
    CornerOverride, CornerSpec, CurrentSource, Element, Mosfet, MosfetLevel1Params, MosfetType,
    NoiseType, Resistor, SpiceError, VoltageSource,
};

const BOLTZMANN: f64 = 1.380_649e-23;
const MOSFET_CHANNEL_NOISE_GAMMA: f64 = 2.0 / 3.0;

fn assert_close(actual: f64, expected: f64, tolerance: f64) {
    assert!(
        (actual - expected).abs() <= tolerance,
        "expected {expected}, got {actual}"
    );
}

#[test]
fn noise_ac_single_grounded_resistor_johnson_noise() {
    let mut circuit = Circuit::new();
    circuit.add(Element::CurrentSource(CurrentSource::new(
        "Iin", "0", "out", 0.0,
    )));
    circuit.add(Element::Resistor(Resistor::new(
        "Rload", "out", "0", 1_000.0,
    )));

    let result = noise_ac(&circuit, "out", "Iin", &[1_000.0], 300.0).unwrap();
    let source_psd = 4.0 * BOLTZMANN * 300.0 / 1_000.0;
    let output_psd = source_psd * 1_000.0_f64.powi(2);

    assert_eq!(result.output_node, "out");
    assert_eq!(result.input_source, "Iin");
    assert_eq!(result.temperature_kelvin, 300.0);
    assert_eq!(result.points.len(), 1);
    assert_eq!(result.points[0].frequency_hz, 1_000.0);
    assert_eq!(result.points[0].entries.len(), 1);
    assert_eq!(result.points[0].entries[0].element_name, "Rload");
    assert_eq!(result.points[0].entries[0].noise_type, NoiseType::Thermal);
    assert_close(result.points[0].entries[0].source_psd, source_psd, 1.0e-32);
    assert_close(result.points[0].output_psd, output_psd, 1.0e-27);
    assert_close(result.points[0].input_referred_psd, source_psd, 1.0e-32);
}

#[test]
fn noise_ac_sorts_resistor_contributions_by_output_noise() {
    let mut circuit = Circuit::new();
    circuit.add(Element::VoltageSource(VoltageSource::new(
        "Vin", "in", "0", 1.0,
    )));
    circuit.add(Element::Resistor(Resistor::new(
        "Rsource", "in", "out", 1_000.0,
    )));
    circuit.add(Element::Resistor(Resistor::new(
        "Rload", "out", "0", 1_000.0,
    )));

    let point = &noise_ac(&circuit, "out", "Vin", &[1_000.0], 300.0)
        .unwrap()
        .points[0];

    let names: Vec<&str> = point
        .entries
        .iter()
        .map(|entry| entry.element_name.as_str())
        .collect();
    assert_eq!(names, vec!["Rload", "Rsource"]);
    assert_close(
        point.entries[0].output_psd,
        point.entries[1].output_psd,
        1.0e-30,
    );
    assert!(point.output_psd > 0.0);
    assert_close(
        point.output_psd,
        point.entries[0].output_psd + point.entries[1].output_psd,
        1.0e-30,
    );
    assert!(point.input_referred_psd > point.output_psd);
}

#[test]
fn noise_text_output_table_is_stable() {
    let mut circuit = Circuit::new();
    circuit.add(Element::VoltageSource(VoltageSource::new(
        "Vin", "in", "0", 1.0,
    )));
    circuit.add(Element::Resistor(Resistor::new(
        "Rsource", "in", "out", 1_000.0,
    )));
    circuit.add(Element::Resistor(Resistor::new(
        "Rload", "out", "0", 2_000.0,
    )));

    let result = noise_ac(&circuit, "out", "Vin", &[10.0, 1_000.0], 300.0).unwrap();

    assert_eq!(
        format_noise_table(&result),
        "Index\tFrequency\tOutputNode\tInputSource\tOutputPSD\tInputReferredPSD\tElement\tType\tSourcePSD\tContributionPSD\n\
0\t1.000000e+01\tout\tVin\t1.104519e-17\t2.485168e-17\tRsource\tthermal\t1.656779e-23\t7.363461e-18\n\
0\t1.000000e+01\tout\tVin\t1.104519e-17\t2.485168e-17\tRload\tthermal\t8.283894e-24\t3.681731e-18\n\
1\t1.000000e+03\tout\tVin\t1.104519e-17\t2.485168e-17\tRsource\tthermal\t1.656779e-23\t7.363461e-18\n\
1\t1.000000e+03\tout\tVin\t1.104519e-17\t2.485168e-17\tRload\tthermal\t8.283894e-24\t3.681731e-18\n"
    );
}

#[test]
fn noise_ac_corners_runs_analysis_per_corner() {
    let mut circuit = Circuit::new();
    circuit.add(Element::CurrentSource(CurrentSource::new(
        "Iin", "0", "out", 0.0,
    )));
    circuit.add(Element::Resistor(Resistor::new(
        "Rload", "out", "0", 1_000.0,
    )));

    let result = noise_ac_corners(
        &circuit,
        "out",
        "Iin",
        &[1_000.0],
        300.0,
        &[
            CornerSpec::new("nominal", Vec::new()),
            CornerSpec::new(
                "rload-high",
                vec![CornerOverride::new("Rload", "resistance", 2_000.0)],
            ),
        ],
    )
    .unwrap();

    assert_eq!(result.output_node, "out");
    assert_eq!(result.input_source, "Iin");
    assert_eq!(result.points.len(), 2);
    assert_eq!(result.points[0].corner_name, "nominal");
    assert_eq!(result.points[1].corner_name, "rload-high");
    assert_eq!(result.points[0].result.temperature_kelvin, 300.0);
    assert_eq!(result.points[1].result.points[0].frequency_hz, 1_000.0);
    assert_eq!(
        result.points[0].result.points[0].entries[0].element_name,
        "Rload"
    );
    assert_close(
        result.points[1].result.points[0].output_psd,
        result.points[0].result.points[0].output_psd * 2.0,
        1.0e-27,
    );
    assert_close(
        result.points[1].result.points[0].input_referred_psd,
        result.points[0].result.points[0].input_referred_psd / 2.0,
        1.0e-32,
    );
}

#[test]
fn noise_ac_rc_low_pass_rolls_off_with_frequency() {
    let mut circuit = Circuit::new();
    circuit.add(Element::VoltageSource(VoltageSource::new(
        "Vin", "in", "0", 1.0,
    )));
    circuit.add(Element::Resistor(Resistor::new("R1", "in", "out", 1_000.0)));
    circuit.add(Element::Capacitor(Capacitor::new("C1", "out", "0", 1.0e-6)));

    let corner = 1.0 / (2.0 * std::f64::consts::PI * 1_000.0 * 1.0e-6);
    let result = noise_ac(&circuit, "out", "Vin", &[1.0, corner, 1.0e6], 300.0).unwrap();

    let low = result.points[0].output_psd;
    let at_corner = result.points[1].output_psd;
    let high = result.points[2].output_psd;

    assert!(low > at_corner, "low={low}, corner={at_corner}");
    assert!(at_corner > high, "corner={at_corner}, high={high}");
    assert!((at_corner - low / 2.0).abs() < low * 0.05);
    assert!(high < low * 1.0e-4, "high={high}, low={low}");
}

#[test]
fn noise_ac_includes_mosfet_channel_thermal_noise() {
    let mut circuit = Circuit::new();
    circuit.add(Element::VoltageSource(VoltageSource::new(
        "Vdd", "vdd", "0", 5.0,
    )));
    circuit.add(Element::VoltageSource(VoltageSource::new(
        "Vgate", "gate", "0", 3.0,
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
            vt0: 1.0,
            kp: 1.0e-3,
            lambda: 0.0,
            gamma: 0.0,
            w: 1.0,
            l: 1.0,
            ..MosfetLevel1Params::default()
        },
    )));

    let point = &noise_ac(&circuit, "out", "Vgate", &[1_000.0], 300.0)
        .unwrap()
        .points[0];
    let entry = point
        .entries
        .iter()
        .find(|entry| entry.element_name == "M1")
        .expect("missing MOSFET channel noise entry");
    let gm = 1.0e-3 * (3.0 - 1.0);
    let expected_source_psd = 4.0 * BOLTZMANN * 300.0 * MOSFET_CHANNEL_NOISE_GAMMA * gm;

    assert_eq!(entry.noise_type, NoiseType::Thermal);
    assert_close(entry.source_psd, expected_source_psd, 1.0e-32);
    assert_close(
        entry.output_psd,
        expected_source_psd * 1_000.0_f64.powi(2),
        1.0e-27,
    );
}

#[test]
fn noise_ac_default_uses_log_frequency_grid() {
    let mut circuit = Circuit::new();
    circuit.add(Element::VoltageSource(VoltageSource::new(
        "Vin", "in", "0", 1.0,
    )));
    circuit.add(Element::Resistor(Resistor::new(
        "Rload", "in", "0", 1_000.0,
    )));

    let result = noise_ac_default(&circuit, "in", "Vin").unwrap();

    assert_eq!(result.points.len(), 50);
    assert_close(result.points[0].frequency_hz, 1.0, 1.0e-12);
    assert_close(result.points[49].frequency_hz, 1.0e6, 1.0e-6);
}

#[test]
fn noise_ac_ground_output_has_zero_output_with_source_psds() {
    let mut circuit = Circuit::new();
    circuit.add(Element::VoltageSource(VoltageSource::new(
        "Vin", "in", "0", 1.0,
    )));
    circuit.add(Element::Resistor(Resistor::new(
        "Rload", "in", "0", 1_000.0,
    )));

    let point = &noise_ac(&circuit, "0", "Vin", &[1_000.0], 300.0)
        .unwrap()
        .points[0];

    assert_eq!(point.output_psd, 0.0);
    assert_eq!(point.input_referred_psd, 0.0);
    assert_eq!(point.entries.len(), 1);
    assert!(point.entries[0].source_psd > 0.0);
    assert_eq!(point.entries[0].output_psd, 0.0);
}

#[test]
fn noise_ac_rejects_invalid_inputs() {
    let mut circuit = Circuit::new();
    circuit.add(Element::VoltageSource(VoltageSource::new(
        "Vin", "in", "0", 1.0,
    )));
    circuit.add(Element::Resistor(Resistor::new(
        "Rload", "in", "0", 1_000.0,
    )));

    assert!(matches!(
        noise_ac(&circuit, "missing", "Vin", &[1.0], 300.0),
        Err(SpiceError::InvalidElement { name, .. }) if name == "missing"
    ));
    assert!(matches!(
        noise_ac(&circuit, "in", "Rload", &[1.0], 300.0),
        Err(SpiceError::InvalidElement { name, .. }) if name == "Rload"
    ));
    assert!(matches!(
        noise_ac(&circuit, "in", "Vin", &[0.0], 300.0),
        Err(SpiceError::InvalidElement { name, .. }) if name == "noise_ac"
    ));
    assert!(matches!(
        noise_ac(&circuit, "in", "Vin", &[1.0], 0.0),
        Err(SpiceError::InvalidElement { name, .. }) if name == "noise_ac"
    ));
}
