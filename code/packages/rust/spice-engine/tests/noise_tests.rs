use spice_engine::{
    device_model_noise_audit_fixtures, format_corner_noise_table, format_noise_table, noise_ac,
    noise_ac_corners, noise_ac_corners_parallel, noise_ac_default, Bjt, Capacitor, Circuit,
    CornerOverride, CornerSpec, CurrentSource, Diode, Element, Jfet, Mosfet, MosfetLevel1Params,
    MosfetType, NoiseType, Resistor, SpiceError, VoltageSource,
};

#[test]
fn diode_series_resistance_adds_thermal_noise() {
    let mut circuit = Circuit::new();
    circuit.add(Element::VoltageSource(VoltageSource::new(
        "Vbias", "bias", "0", 1.0,
    )));
    circuit.add(Element::Resistor(Resistor::new(
        "Rbias", "bias", "out", 1_000.0,
    )));
    let mut diode = Diode::new("D1", "out", "0");
    diode.series_resistance = 100.0;
    circuit.add(Element::Diode(diode));

    let result = noise_ac(&circuit, "out", "Vbias", &[1_000.0], 300.0).unwrap();
    let series_resistance = result.points[0]
        .entries
        .iter()
        .find(|entry| entry.element_name == "D1:RS")
        .unwrap();

    assert_eq!(series_resistance.noise_type, NoiseType::Thermal);
    assert!(series_resistance.source_psd > 0.0);
}

#[test]
fn bjt_forward_beta_rolloff_reduces_shot_noise() {
    let source_psd = |rolloff_current: f64| {
        let mut circuit = Circuit::new();
        circuit.add(Element::VoltageSource(VoltageSource::new(
            "Vcc", "vcc", "0", 5.0,
        )));
        circuit.add(Element::VoltageSource(VoltageSource::new(
            "Vbase", "base", "0", 0.65,
        )));
        circuit.add(Element::Resistor(Resistor::new(
            "Rload", "vcc", "out", 1_000.0,
        )));
        let mut transistor = Bjt::new("Q1", "out", "base", "0");
        transistor.forward_beta_rolloff_current = rolloff_current;
        circuit.add(Element::Bjt(transistor));
        noise_ac(&circuit, "out", "Vbase", &[1_000.0], 300.0)
            .unwrap()
            .points[0]
            .entries
            .iter()
            .find(|entry| entry.element_name == "Q1")
            .unwrap()
            .source_psd
    };

    assert!(source_psd(1.0e-4) < source_psd(0.0));
}

#[test]
fn bjt_base_emitter_leakage_increases_shot_noise() {
    let source_psd = |leakage_current: f64| {
        let mut circuit = Circuit::new();
        circuit.add(Element::VoltageSource(VoltageSource::new(
            "Vbase", "base", "0", 0.65,
        )));
        circuit.add(Element::Resistor(Resistor::new(
            "Rload", "out", "0", 1_000.0,
        )));
        let mut transistor = Bjt::new("Q1", "out", "base", "0");
        transistor.base_emitter_leakage_saturation_current = leakage_current;
        transistor.base_emitter_leakage_emission_coefficient = 1.5;
        circuit.add(Element::Bjt(transistor));
        noise_ac(&circuit, "out", "Vbase", &[1_000.0], 300.0)
            .unwrap()
            .points[0]
            .entries
            .iter()
            .find(|entry| entry.element_name == "Q1")
            .unwrap()
            .source_psd
    };

    assert!(source_psd(1.0e-10) > source_psd(0.0));
}

#[test]
fn bjt_base_collector_leakage_increases_shot_noise() {
    let source_psd = |leakage_current: f64| {
        let mut circuit = Circuit::new();
        circuit.add(Element::VoltageSource(VoltageSource::new(
            "Vbase", "base", "0", 0.65,
        )));
        circuit.add(Element::Resistor(Resistor::new(
            "Rload", "out", "0", 1_000.0,
        )));
        let mut transistor = Bjt::new("Q1", "out", "base", "base");
        transistor.base_collector_leakage_saturation_current = leakage_current;
        transistor.base_collector_leakage_emission_coefficient = 1.5;
        circuit.add(Element::Bjt(transistor));
        noise_ac(&circuit, "out", "Vbase", &[1_000.0], 300.0)
            .unwrap()
            .points[0]
            .entries
            .iter()
            .find(|entry| entry.element_name == "Q1")
            .unwrap()
            .source_psd
    };

    assert!(source_psd(1.0e-10) > source_psd(0.0));
}

#[test]
fn bjt_emitter_resistance_adds_thermal_noise() {
    let mut circuit = Circuit::new();
    circuit.add(Element::VoltageSource(VoltageSource::new(
        "Vbase", "base", "0", 0.65,
    )));
    circuit.add(Element::Resistor(Resistor::new(
        "Rload", "out", "0", 1_000.0,
    )));
    let mut transistor = Bjt::new("Q1", "out", "base", "0");
    transistor.emitter_resistance = 100.0;
    circuit.add(Element::Bjt(transistor));

    let result = noise_ac(&circuit, "out", "Vbase", &[1_000.0], 300.0).unwrap();
    let emitter_resistance = result.points[0]
        .entries
        .iter()
        .find(|entry| entry.element_name == "Q1:RE")
        .unwrap();

    assert_eq!(emitter_resistance.noise_type, NoiseType::Thermal);
    assert!(emitter_resistance.source_psd > 0.0);
}

#[test]
fn bjt_collector_resistance_adds_thermal_noise() {
    let mut circuit = Circuit::new();
    circuit.add(Element::VoltageSource(VoltageSource::new(
        "Vbase", "base", "0", 0.65,
    )));
    circuit.add(Element::Resistor(Resistor::new(
        "Rload", "out", "0", 1_000.0,
    )));
    let mut transistor = Bjt::new("Q1", "out", "base", "0");
    transistor.collector_resistance = 100.0;
    circuit.add(Element::Bjt(transistor));

    let result = noise_ac(&circuit, "out", "Vbase", &[1_000.0], 300.0).unwrap();
    let collector_resistance = result.points[0]
        .entries
        .iter()
        .find(|entry| entry.element_name == "Q1:RC")
        .unwrap();

    assert_eq!(collector_resistance.noise_type, NoiseType::Thermal);
    assert!(collector_resistance.source_psd > 0.0);
}

#[test]
fn bjt_base_resistance_adds_thermal_noise() {
    let mut circuit = Circuit::new();
    circuit.add(Element::VoltageSource(VoltageSource::new(
        "Vbase", "base", "0", 0.65,
    )));
    circuit.add(Element::Resistor(Resistor::new(
        "Rload", "out", "0", 1_000.0,
    )));
    let mut transistor = Bjt::new("Q1", "out", "base", "0");
    transistor.base_resistance = 100.0;
    circuit.add(Element::Bjt(transistor));

    let result = noise_ac(&circuit, "out", "Vbase", &[1_000.0], 300.0).unwrap();
    let base_resistance = result.points[0]
        .entries
        .iter()
        .find(|entry| entry.element_name == "Q1:RB")
        .unwrap();

    assert_eq!(base_resistance.noise_type, NoiseType::Thermal);
    assert!(base_resistance.source_psd > 0.0);
}

#[test]
fn bjt_minimum_base_resistance_increases_high_current_thermal_noise() {
    let source_psd = |minimum_base_resistance, base_resistance_half_current| {
        let mut circuit = Circuit::new();
        circuit.add(Element::VoltageSource(VoltageSource::new(
            "Vbase", "base", "0", 0.65,
        )));
        circuit.add(Element::Resistor(Resistor::new(
            "Rload", "out", "0", 1_000.0,
        )));
        let mut transistor = Bjt::new("Q1", "out", "base", "0");
        transistor.base_resistance = 100.0;
        transistor.minimum_base_resistance = minimum_base_resistance;
        transistor.base_resistance_half_current = base_resistance_half_current;
        circuit.add(Element::Bjt(transistor));

        let result = noise_ac(&circuit, "out", "Vbase", &[1_000.0], 300.0).unwrap();
        result.points[0]
            .entries
            .iter()
            .find(|entry| entry.element_name == "Q1:RB")
            .unwrap()
            .source_psd
    };

    let fixed = source_psd(None, 0.0);
    let bias_dependent = source_psd(Some(10.0), 1.0e-9);

    assert!(bias_dependent > fixed);
}

#[test]
fn bjt_flicker_noise_uses_kf_with_inverse_frequency_scaling() {
    let mut circuit = Circuit::new();
    circuit.add(Element::VoltageSource(VoltageSource::new(
        "Vcc", "vcc", "0", 5.0,
    )));
    circuit.add(Element::VoltageSource(VoltageSource::new(
        "Vbase", "base", "0", 0.7,
    )));
    circuit.add(Element::Resistor(Resistor::new(
        "Rload", "vcc", "out", 1_000.0,
    )));
    let mut transistor = Bjt::new("Q1", "out", "base", "0");
    transistor.flicker_noise_coefficient = 1.0e-12;
    circuit.add(Element::Bjt(transistor));

    let result = noise_ac(&circuit, "out", "Vbase", &[10.0, 1_000.0], 300.0).unwrap();
    let flicker_psd = |point_index: usize| {
        result.points[point_index]
            .entries
            .iter()
            .find(|entry| entry.element_name == "Q1" && entry.noise_type == NoiseType::Flicker)
            .unwrap()
            .source_psd
    };

    assert!(flicker_psd(0) > 0.0);
    assert_close(flicker_psd(0) / flicker_psd(1), 100.0, 1.0e-10);
}

#[test]
fn diode_flicker_noise_uses_kf_with_inverse_frequency_scaling() {
    let mut circuit = Circuit::new();
    circuit.add(Element::VoltageSource(VoltageSource::new(
        "Vbias", "bias", "0", 1.0,
    )));
    circuit.add(Element::Resistor(Resistor::new(
        "Rbias", "bias", "out", 1_000.0,
    )));
    let mut diode = Diode::new("D1", "out", "0");
    diode.flicker_noise_coefficient = 1.0e-12;
    circuit.add(Element::Diode(diode));

    let result = noise_ac(&circuit, "out", "Vbias", &[10.0, 1_000.0], 300.0).unwrap();
    let flicker_psd = |point_index: usize| {
        result.points[point_index]
            .entries
            .iter()
            .find(|entry| entry.element_name == "D1" && entry.noise_type == NoiseType::Flicker)
            .unwrap()
            .source_psd
    };

    assert!(flicker_psd(0) > 0.0);
    assert_close(flicker_psd(0) / flicker_psd(1), 100.0, 1.0e-10);
}

#[test]
fn jfet_flicker_noise_uses_kf_with_inverse_frequency_scaling() {
    let mut circuit = Circuit::new();
    circuit.add(Element::VoltageSource(VoltageSource::new(
        "Vdd", "vdd", "0", 5.0,
    )));
    circuit.add(Element::VoltageSource(VoltageSource::new(
        "Vgate", "gate", "0", 0.0,
    )));
    circuit.add(Element::Resistor(Resistor::new(
        "Rload", "vdd", "out", 1_000.0,
    )));
    let mut jfet = Jfet::with_model(
        "J1",
        "out",
        "gate",
        "0",
        spice_engine::JfetPolarity::Njf,
        1.0e-3,
        -2.0,
        0.0,
    );
    jfet.flicker_noise_coefficient = 1.0e-12;
    circuit.add(Element::Jfet(jfet));

    let result = noise_ac(&circuit, "out", "Vgate", &[10.0, 1_000.0], 300.0).unwrap();
    let flicker_psd = |point_index: usize| {
        result.points[point_index]
            .entries
            .iter()
            .find(|entry| entry.element_name == "J1" && entry.noise_type == NoiseType::Flicker)
            .unwrap()
            .source_psd
    };

    assert!(flicker_psd(0) > 0.0);
    assert_close(flicker_psd(0) / flicker_psd(1), 100.0, 1.0e-10);
}

#[test]
fn jfet_nlev_and_gdsnoi_select_and_scale_channel_noise() {
    let source_psd = |noise_equation_level: f64, channel_noise_coefficient: f64| {
        let mut circuit = Circuit::new();
        circuit.add(Element::VoltageSource(VoltageSource::new(
            "Vdrain", "out", "0", 1.0,
        )));
        circuit.add(Element::VoltageSource(VoltageSource::new(
            "Vgate", "gate", "0", 0.0,
        )));
        let mut jfet = Jfet::new("J1", "out", "gate", "0");
        jfet.beta = 1.0e-3;
        jfet.threshold_voltage = -2.0;
        jfet.noise_equation_level = noise_equation_level;
        jfet.channel_noise_coefficient = channel_noise_coefficient;
        circuit.add(Element::Jfet(jfet));
        noise_ac(&circuit, "out", "Vgate", &[1_000.0], 300.0)
            .unwrap()
            .points[0]
            .entries
            .iter()
            .find(|entry| entry.element_name == "J1" && entry.noise_type == NoiseType::Thermal)
            .unwrap()
            .source_psd
    };

    let expected_conductance = (2.0 / 3.0) * 1.0e-3 * 2.0 * 1.75 / 1.5;
    let expected_psd = 4.0 * BOLTZMANN * 300.0 * expected_conductance;
    assert_close(source_psd(3.0, 1.0), expected_psd, expected_psd * 1.0e-10);
    let legacy_psd = source_psd(1.0, 1.0);
    assert_close(source_psd(2.0, 4.0), legacy_psd, legacy_psd * 1.0e-10);
    let scaled_psd = 2.0 * source_psd(3.0, 1.0);
    assert_close(source_psd(3.0, 2.0), scaled_psd, scaled_psd * 1.0e-10);
}

#[test]
fn jfet_rejects_invalid_channel_noise_parameters() {
    for (noise_equation_level, channel_noise_coefficient, expected_reason) in [
        (
            2.5,
            1.0,
            "noise equation level must be a finite integer greater than or equal to 1",
        ),
        (
            1.0,
            -1.0,
            "channel noise coefficient must be finite and non-negative",
        ),
    ] {
        let mut circuit = Circuit::new();
        circuit.add(Element::VoltageSource(VoltageSource::new(
            "Vgate", "gate", "0", 0.0,
        )));
        let mut jfet = Jfet::new("J1", "out", "gate", "0");
        jfet.noise_equation_level = noise_equation_level;
        jfet.channel_noise_coefficient = channel_noise_coefficient;
        circuit.add(Element::Jfet(jfet));

        let error = noise_ac(&circuit, "out", "Vgate", &[1_000.0], 300.0).unwrap_err();
        assert!(matches!(
            error,
            SpiceError::InvalidElement { reason, .. } if reason == expected_reason
        ));
    }
}

#[test]
fn jfet_rejects_invalid_flicker_noise_coefficient() {
    let mut circuit = Circuit::new();
    circuit.add(Element::VoltageSource(VoltageSource::new(
        "Vgate", "gate", "0", 0.0,
    )));
    let mut jfet = Jfet::new("J1", "out", "gate", "0");
    jfet.flicker_noise_coefficient = -1.0;
    circuit.add(Element::Jfet(jfet));

    let error = noise_ac(&circuit, "out", "Vgate", &[1_000.0], 300.0).unwrap_err();
    assert!(matches!(
        error,
        SpiceError::InvalidElement { reason, .. }
            if reason == "flicker-noise coefficient must be finite and non-negative"
    ));
}

#[test]
fn jfet_rejects_invalid_flicker_noise_exponent() {
    let mut circuit = Circuit::new();
    circuit.add(Element::VoltageSource(VoltageSource::new(
        "Vgate", "gate", "0", 0.0,
    )));
    let mut jfet = Jfet::new("J1", "out", "gate", "0");
    jfet.flicker_noise_exponent = -1.0;
    circuit.add(Element::Jfet(jfet));

    let error = noise_ac(&circuit, "out", "Vgate", &[1_000.0], 300.0).unwrap_err();
    assert!(matches!(
        error,
        SpiceError::InvalidElement { reason, .. }
            if reason == "flicker-noise exponent must be finite and non-negative"
    ));
}

#[test]
fn jfet_rejects_invalid_junction_potential() {
    let mut circuit = Circuit::new();
    circuit.add(Element::VoltageSource(VoltageSource::new(
        "Vgate", "gate", "0", 0.0,
    )));
    let mut jfet = Jfet::new("J1", "out", "gate", "0");
    jfet.junction_potential = 0.0;
    circuit.add(Element::Jfet(jfet));

    let error = noise_ac(&circuit, "out", "Vgate", &[1_000.0], 300.0).unwrap_err();
    assert!(matches!(
        error,
        SpiceError::InvalidElement { reason, .. }
            if reason == "junction potential must be finite and positive"
    ));
}

#[test]
fn jfet_rejects_invalid_forward_bias_depletion_coefficient() {
    let mut circuit = Circuit::new();
    circuit.add(Element::VoltageSource(VoltageSource::new(
        "Vgate", "gate", "0", 0.0,
    )));
    let mut jfet = Jfet::new("J1", "out", "gate", "0");
    jfet.forward_bias_depletion_coefficient = 1.0;
    circuit.add(Element::Jfet(jfet));

    let error = noise_ac(&circuit, "out", "Vgate", &[1_000.0], 300.0).unwrap_err();
    assert!(matches!(
        error,
        SpiceError::InvalidElement { reason, .. }
            if reason == "forward-bias depletion coefficient must be finite and in [0, 1)"
    ));
}

#[test]
fn jfet_rejects_invalid_gate_saturation_current() {
    let mut circuit = Circuit::new();
    circuit.add(Element::VoltageSource(VoltageSource::new(
        "Vgate", "gate", "0", 0.0,
    )));
    let mut jfet = Jfet::new("J1", "out", "gate", "0");
    jfet.gate_saturation_current = -1.0;
    circuit.add(Element::Jfet(jfet));

    let error = noise_ac(&circuit, "out", "Vgate", &[1_000.0], 300.0).unwrap_err();
    assert!(matches!(
        error,
        SpiceError::InvalidElement { reason, .. }
            if reason == "gate saturation current must be finite and non-negative"
    ));
}

#[test]
fn jfet_rejects_invalid_drain_resistance() {
    let mut circuit = Circuit::new();
    circuit.add(Element::VoltageSource(VoltageSource::new(
        "Vgate", "gate", "0", 0.0,
    )));
    let mut jfet = Jfet::new("J1", "out", "gate", "0");
    jfet.drain_resistance = -1.0;
    circuit.add(Element::Jfet(jfet));

    let error = noise_ac(&circuit, "out", "Vgate", &[1_000.0], 300.0).unwrap_err();
    assert!(matches!(
        error,
        SpiceError::InvalidElement { reason, .. }
            if reason == "drain resistance must be finite and non-negative"
    ));
}

#[test]
fn jfet_rejects_invalid_source_resistance() {
    let mut circuit = Circuit::new();
    circuit.add(Element::VoltageSource(VoltageSource::new(
        "Vgate", "gate", "0", 0.0,
    )));
    let mut jfet = Jfet::new("J1", "out", "gate", "0");
    jfet.source_resistance = -1.0;
    circuit.add(Element::Jfet(jfet));

    let error = noise_ac(&circuit, "out", "Vgate", &[1_000.0], 300.0).unwrap_err();
    assert!(matches!(
        error,
        SpiceError::InvalidElement { reason, .. }
            if reason == "source resistance must be finite and non-negative"
    ));
}

#[test]
fn jfet_drain_resistance_emits_thermal_noise() {
    let mut circuit = Circuit::new();
    circuit.add(Element::VoltageSource(VoltageSource::new(
        "Vdd", "vdd", "0", 5.0,
    )));
    circuit.add(Element::VoltageSource(VoltageSource::new(
        "Vgate", "gate", "0", 0.0,
    )));
    circuit.add(Element::Resistor(Resistor::new(
        "Rload", "vdd", "out", 1_000.0,
    )));
    let mut jfet = Jfet::new("J1", "out", "gate", "0");
    jfet.drain_resistance = 250.0;
    circuit.add(Element::Jfet(jfet));

    let result = noise_ac(&circuit, "out", "Vgate", &[1_000.0], 300.0).unwrap();
    let entry = result.points[0]
        .entries
        .iter()
        .find(|entry| entry.element_name == "J1:RD" && entry.noise_type == NoiseType::Thermal)
        .unwrap();
    assert_close(
        entry.source_psd,
        4.0 * 1.380_649e-23 * 300.0 / 250.0,
        1.0e-30,
    );
}

#[test]
fn mosfet_drain_resistance_emits_thermal_noise() {
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
            drain_resistance: 250.0,
            ..MosfetLevel1Params::default()
        },
    )));

    let result = noise_ac(&circuit, "out", "Vgate", &[1_000.0], 300.0).unwrap();
    let entry = result.points[0]
        .entries
        .iter()
        .find(|entry| entry.element_name == "M1:RD" && entry.noise_type == NoiseType::Thermal)
        .unwrap();
    assert_close(
        entry.source_psd,
        4.0 * 1.380_649e-23 * 300.0 / 250.0,
        1.0e-30,
    );
}

#[test]
fn mosfet_source_resistance_emits_thermal_noise() {
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
            source_resistance: 250.0,
            ..MosfetLevel1Params::default()
        },
    )));

    let result = noise_ac(&circuit, "out", "Vgate", &[1_000.0], 300.0).unwrap();
    let entry = result.points[0]
        .entries
        .iter()
        .find(|entry| entry.element_name == "M1:RS" && entry.noise_type == NoiseType::Thermal)
        .unwrap();
    assert_close(
        entry.source_psd,
        4.0 * 1.380_649e-23 * 300.0 / 250.0,
        1.0e-30,
    );
}

#[test]
fn jfet_source_resistance_emits_thermal_noise() {
    let mut circuit = Circuit::new();
    circuit.add(Element::VoltageSource(VoltageSource::new(
        "Vdd", "vdd", "0", 5.0,
    )));
    circuit.add(Element::VoltageSource(VoltageSource::new(
        "Vgate", "gate", "0", 0.0,
    )));
    circuit.add(Element::Resistor(Resistor::new(
        "Rload", "vdd", "out", 1_000.0,
    )));
    let mut jfet = Jfet::new("J1", "out", "gate", "0");
    jfet.source_resistance = 250.0;
    circuit.add(Element::Jfet(jfet));

    let result = noise_ac(&circuit, "out", "Vgate", &[1_000.0], 300.0).unwrap();
    let entry = result.points[0]
        .entries
        .iter()
        .find(|entry| entry.element_name == "J1:RS" && entry.noise_type == NoiseType::Thermal)
        .unwrap();
    assert_close(
        entry.source_psd,
        4.0 * 1.380_649e-23 * 300.0 / 250.0,
        1.0e-30,
    );
}

#[test]
fn jfet_gate_junctions_emit_distinct_shot_noise_sources() {
    let mut circuit = Circuit::new();
    circuit.add(Element::VoltageSource(VoltageSource::new(
        "Vdd", "out", "0", 1.0,
    )));
    circuit.add(Element::VoltageSource(VoltageSource::new(
        "Vgate", "gate", "0", 0.3,
    )));
    let mut jfet = Jfet::new("J1", "out", "gate", "0");
    jfet.gate_saturation_current = 1.0e-12;
    circuit.add(Element::Jfet(jfet));

    let result = noise_ac(&circuit, "gate", "Vgate", &[1_000.0], 300.0).unwrap();
    let entries = &result.points[0].entries;
    for name in ["J1:IGS", "J1:IGD"] {
        let entry = entries
            .iter()
            .find(|entry| entry.element_name == name && entry.noise_type == NoiseType::Shot)
            .unwrap();
        assert!(entry.source_psd > 0.0);
    }
}

#[test]
fn jfet_flicker_noise_uses_af_current_exponent() {
    let source_psd = |exponent: f64| {
        let mut circuit = Circuit::new();
        circuit.add(Element::VoltageSource(VoltageSource::new(
            "Vdd", "vdd", "0", 5.0,
        )));
        circuit.add(Element::VoltageSource(VoltageSource::new(
            "Vgate", "gate", "0", 0.0,
        )));
        circuit.add(Element::Resistor(Resistor::new(
            "Rload", "vdd", "out", 1_000.0,
        )));
        let mut jfet = Jfet::with_model(
            "J1",
            "out",
            "gate",
            "0",
            spice_engine::JfetPolarity::Njf,
            1.0e-3,
            -2.0,
            0.0,
        );
        jfet.flicker_noise_coefficient = 1.0e-12;
        jfet.flicker_noise_exponent = exponent;
        circuit.add(Element::Jfet(jfet));
        noise_ac(&circuit, "out", "Vgate", &[1_000.0], 300.0)
            .unwrap()
            .points[0]
            .entries
            .iter()
            .find(|entry| entry.element_name == "J1" && entry.noise_type == NoiseType::Flicker)
            .unwrap()
            .source_psd
    };

    assert!(source_psd(2.0) < source_psd(1.0));
}

#[test]
fn diode_flicker_noise_uses_af_current_exponent() {
    let source_psd = |exponent: f64| {
        let mut circuit = Circuit::new();
        circuit.add(Element::VoltageSource(VoltageSource::new(
            "Vbias", "bias", "0", 1.0,
        )));
        circuit.add(Element::Resistor(Resistor::new(
            "Rbias", "bias", "out", 1_000.0,
        )));
        let mut diode = Diode::new("D1", "out", "0");
        diode.flicker_noise_coefficient = 1.0e-12;
        diode.flicker_noise_exponent = exponent;
        circuit.add(Element::Diode(diode));
        noise_ac(&circuit, "out", "Vbias", &[1_000.0], 300.0)
            .unwrap()
            .points[0]
            .entries
            .iter()
            .find(|entry| entry.element_name == "D1" && entry.noise_type == NoiseType::Flicker)
            .unwrap()
            .source_psd
    };

    assert!(source_psd(2.0) < source_psd(1.0));
}

#[test]
fn bjt_flicker_noise_uses_af_base_current_exponent() {
    let source_psd = |exponent: f64| {
        let mut circuit = Circuit::new();
        circuit.add(Element::VoltageSource(VoltageSource::new(
            "Vcc", "vcc", "0", 5.0,
        )));
        circuit.add(Element::VoltageSource(VoltageSource::new(
            "Vbase", "base", "0", 0.7,
        )));
        circuit.add(Element::Resistor(Resistor::new(
            "Rload", "vcc", "out", 1_000.0,
        )));
        let mut transistor = Bjt::new("Q1", "out", "base", "0");
        transistor.flicker_noise_coefficient = 1.0e-12;
        transistor.flicker_noise_exponent = exponent;
        circuit.add(Element::Bjt(transistor));
        noise_ac(&circuit, "out", "Vbase", &[1_000.0], 300.0)
            .unwrap()
            .points[0]
            .entries
            .iter()
            .find(|entry| entry.element_name == "Q1" && entry.noise_type == NoiseType::Flicker)
            .unwrap()
            .source_psd
    };

    assert!(source_psd(2.0) < source_psd(1.0));
}

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
fn corner_noise_text_output_table_is_stable() {
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

    assert_eq!(
        format_corner_noise_table(&result),
        "Corner\tIndex\tFrequency\tOutputNode\tInputSource\tOutputPSD\tInputReferredPSD\tElement\tType\tSourcePSD\tContributionPSD\n\
nominal\t0\t1.000000e+03\tout\tIin\t1.656779e-17\t1.656779e-23\tRload\tthermal\t1.656779e-23\t1.656779e-17\n\
rload-high\t0\t1.000000e+03\tout\tIin\t3.313558e-17\t8.283894e-24\tRload\tthermal\t8.283894e-24\t3.313558e-17\n"
    );
}

#[test]
fn noise_ac_corners_parallel_matches_ordered_sequential_results() {
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
    let corners = [
        CornerSpec::new("nominal", Vec::new()),
        CornerSpec::new(
            "rload-high",
            vec![CornerOverride::new("Rload", "resistance", 4_000.0)],
        ),
        CornerSpec::new(
            "rsource-fast",
            vec![CornerOverride::new("Rsource", "resistance", 500.0)],
        ),
    ];

    let frequencies_hz = [10.0, 1_000.0];
    let sequential =
        noise_ac_corners(&circuit, "out", "Vin", &frequencies_hz, 300.0, &corners).unwrap();
    let parallel =
        noise_ac_corners_parallel(&circuit, "out", "Vin", &frequencies_hz, 300.0, &corners)
            .unwrap();

    assert_eq!(parallel.output_node, sequential.output_node);
    assert_eq!(parallel.input_source, sequential.input_source);
    assert_eq!(parallel.points.len(), sequential.points.len());
    for (parallel_corner, sequential_corner) in parallel.points.iter().zip(sequential.points.iter())
    {
        assert_eq!(parallel_corner.corner_name, sequential_corner.corner_name);
        assert_eq!(
            parallel_corner.result.temperature_kelvin,
            sequential_corner.result.temperature_kelvin
        );
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
            assert_close(
                parallel_point.frequency_hz,
                sequential_point.frequency_hz,
                1.0e-12,
            );
            assert_close(
                parallel_point.output_psd,
                sequential_point.output_psd,
                1.0e-30,
            );
            assert_close(
                parallel_point.input_referred_psd,
                sequential_point.input_referred_psd,
                1.0e-30,
            );
            assert_eq!(parallel_point.entries.len(), sequential_point.entries.len());
            for (parallel_entry, sequential_entry) in parallel_point
                .entries
                .iter()
                .zip(sequential_point.entries.iter())
            {
                assert_eq!(parallel_entry.element_name, sequential_entry.element_name);
                assert_eq!(parallel_entry.noise_type, sequential_entry.noise_type);
                assert_close(
                    parallel_entry.source_psd,
                    sequential_entry.source_psd,
                    1.0e-32,
                );
                assert_close(
                    parallel_entry.output_psd,
                    sequential_entry.output_psd,
                    1.0e-30,
                );
            }
        }
    }
    assert_eq!(
        format_corner_noise_table(&parallel),
        "Corner\tIndex\tFrequency\tOutputNode\tInputSource\tOutputPSD\tInputReferredPSD\tElement\tType\tSourcePSD\tContributionPSD\n\
nominal\t0\t1.000000e+01\tout\tVin\t1.104519e-17\t2.485168e-17\tRsource\tthermal\t1.656779e-23\t7.363461e-18\n\
nominal\t0\t1.000000e+01\tout\tVin\t1.104519e-17\t2.485168e-17\tRload\tthermal\t8.283894e-24\t3.681731e-18\n\
nominal\t1\t1.000000e+03\tout\tVin\t1.104519e-17\t2.485168e-17\tRsource\tthermal\t1.656779e-23\t7.363461e-18\n\
nominal\t1\t1.000000e+03\tout\tVin\t1.104519e-17\t2.485168e-17\tRload\tthermal\t8.283894e-24\t3.681731e-18\n\
rload-high\t0\t1.000000e+01\tout\tVin\t1.325423e-17\t2.070973e-17\tRsource\tthermal\t1.656779e-23\t1.060338e-17\n\
rload-high\t0\t1.000000e+01\tout\tVin\t1.325423e-17\t2.070973e-17\tRload\tthermal\t4.141947e-24\t2.650846e-18\n\
rload-high\t1\t1.000000e+03\tout\tVin\t1.325423e-17\t2.070973e-17\tRsource\tthermal\t1.656779e-23\t1.060338e-17\n\
rload-high\t1\t1.000000e+03\tout\tVin\t1.325423e-17\t2.070973e-17\tRload\tthermal\t4.141947e-24\t2.650846e-18\n\
rsource-fast\t0\t1.000000e+01\tout\tVin\t6.627115e-18\t1.035487e-17\tRsource\tthermal\t3.313558e-23\t5.301692e-18\n\
rsource-fast\t0\t1.000000e+01\tout\tVin\t6.627115e-18\t1.035487e-17\tRload\tthermal\t8.283894e-24\t1.325423e-18\n\
rsource-fast\t1\t1.000000e+03\tout\tVin\t6.627115e-18\t1.035487e-17\tRsource\tthermal\t3.313558e-23\t5.301692e-18\n\
rsource-fast\t1\t1.000000e+03\tout\tVin\t6.627115e-18\t1.035487e-17\tRload\tthermal\t8.283894e-24\t1.325423e-18\n"
    );
}

#[test]
fn noise_ac_corners_parallel_reports_corner_override_errors() {
    let mut circuit = Circuit::new();
    circuit.add(Element::CurrentSource(CurrentSource::new(
        "Iin", "0", "out", 0.0,
    )));
    circuit.add(Element::Resistor(Resistor::new(
        "Rload", "out", "0", 1_000.0,
    )));
    let corners = [CornerSpec::new(
        "missing",
        vec![CornerOverride::new("Rmissing", "resistance", 500.0)],
    )];

    assert!(matches!(
        noise_ac_corners_parallel(&circuit, "out", "Iin", &[1_000.0], 300.0, &corners),
        Err(SpiceError::InvalidElement { name, reason })
            if name == "dc_corners" && reason.contains("Rmissing")
    ));
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
fn noise_ac_adds_inverse_frequency_mosfet_flicker_noise() {
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
            flicker_noise_coefficient: 2.0e-18,
            flicker_noise_exponent: 2.0,
            ..MosfetLevel1Params::default()
        },
    )));

    let result = noise_ac(&circuit, "out", "Vgate", &[100.0, 1_000.0], 300.0).unwrap();
    let flicker_psds = result
        .points
        .iter()
        .map(|point| {
            point
                .entries
                .iter()
                .find(|entry| entry.element_name == "M1" && entry.noise_type == NoiseType::Flicker)
                .expect("missing MOSFET flicker-noise entry")
                .source_psd
        })
        .collect::<Vec<_>>();

    assert!(flicker_psds[0] > 0.0);
    assert_close(flicker_psds[0], 10.0 * flicker_psds[1], 1.0e-30);
    assert!(result.points[0]
        .entries
        .iter()
        .any(|entry| { entry.element_name == "M1" && entry.noise_type == NoiseType::Thermal }));
}

#[test]
fn noise_ac_rejects_invalid_mosfet_flicker_noise_coefficient() {
    let mut circuit = Circuit::new();
    circuit.add(Element::VoltageSource(VoltageSource::new(
        "Vgate", "gate", "0", 3.0,
    )));
    circuit.add(Element::Mosfet(Mosfet::with_model(
        "M1",
        "0",
        "gate",
        "0",
        "0",
        MosfetType::Nmos,
        MosfetLevel1Params {
            flicker_noise_coefficient: -1.0,
            ..MosfetLevel1Params::default()
        },
    )));

    let error = noise_ac(&circuit, "0", "Vgate", &[1_000.0], 300.0).unwrap_err();
    assert!(error.to_string().contains("MOSFET KF must be non-negative"));
}

#[test]
fn noise_ac_rejects_invalid_mosfet_flicker_noise_exponent() {
    let mut circuit = Circuit::new();
    circuit.add(Element::VoltageSource(VoltageSource::new(
        "Vgate", "gate", "0", 3.0,
    )));
    circuit.add(Element::Mosfet(Mosfet::with_model(
        "M1",
        "0",
        "gate",
        "0",
        "0",
        MosfetType::Nmos,
        MosfetLevel1Params {
            flicker_noise_exponent: -1.0,
            ..MosfetLevel1Params::default()
        },
    )));

    let error = noise_ac(&circuit, "0", "Vgate", &[1_000.0], 300.0).unwrap_err();
    assert!(error.to_string().contains("MOSFET AF must be non-negative"));
}

#[test]
fn device_model_noise_audit_fixtures_run_reference_noise_points() {
    let fixtures = device_model_noise_audit_fixtures().unwrap();
    assert_eq!(
        fixtures
            .iter()
            .map(|fixture| fixture.name.as_str())
            .collect::<Vec<_>>(),
        vec![
            "diode-shot-noise",
            "bjt-shot-noise",
            "jfet-channel-noise",
            "mos-level1-channel-noise"
        ]
    );

    for fixture in &fixtures {
        let result = noise_ac(
            &fixture.circuit,
            &fixture.output_node,
            &fixture.input_source,
            &[fixture.frequency_hz],
            300.0,
        )
        .unwrap();
        let entry = result.points[0]
            .entries
            .iter()
            .find(|entry| entry.element_name == fixture.expected_noise_element)
            .expect("expected model noise entry");
        assert_eq!(entry.noise_type, fixture.expected_noise_type);
        assert!(
            fixture.expected_source_psd_min <= entry.source_psd
                && entry.source_psd <= fixture.expected_source_psd_max,
            "{} expected {} <= source {} <= {}",
            fixture.name,
            fixture.expected_source_psd_min,
            entry.source_psd,
            fixture.expected_source_psd_max
        );
        assert!(
            fixture.expected_output_psd_min <= entry.output_psd
                && entry.output_psd <= fixture.expected_output_psd_max,
            "{} expected {} <= output {} <= {}",
            fixture.name,
            fixture.expected_output_psd_min,
            entry.output_psd,
            fixture.expected_output_psd_max
        );
        assert!(fixture.deck_lines[0].starts_with("* device-model noise fixture:"));
        assert!(fixture
            .deck_lines
            .iter()
            .any(|line| line.starts_with(".model ")));
        assert!(fixture
            .deck_lines
            .iter()
            .any(|line| line.starts_with(".noise ")));
        assert!(!fixture.noise_behavior.is_empty());
    }
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
