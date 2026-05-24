use spice_engine::{
    ac_sweep, ac_sweep_corners, s_parameters, Bjt, BjtPolarity, Capacitor, Cccs, Ccvs, Circuit,
    CornerOverride, CornerSpec, CurrentSource, Diode, Element, Inductor, Jfet, JfetPolarity,
    Mosfet, MosfetLevel1Params, MosfetType, MutualInductor, Resistor, SpiceError, TransmissionLine,
    Vcvs, VoltageSource,
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
