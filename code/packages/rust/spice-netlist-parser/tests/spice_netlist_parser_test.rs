use spice_engine::{
    ac_sweep, dc_op, mc_dc, noise_ac, sens_dc, tf, BjtPolarity, Element, McDistribution, McOptions,
    MosfetType,
};
use spice_netlist_parser::{
    parse_netlist, parse_value, AcAnalysis, Analysis, DcAnalysis, McAnalysis, NetlistParseError,
    NoiseAnalysis, OpAnalysis, SensAnalysis, TfAnalysis, TranAnalysis,
};

fn assert_close(actual: f64, expected: f64) {
    assert!(
        (actual - expected).abs() < 1.0e-9,
        "expected {expected}, got {actual}"
    );
}

#[test]
fn parses_linear_operating_point_netlist_into_circuit() {
    let parsed = parse_netlist(
        r#"
* resistor divider
V1 vin 0 DC 10
R1 vin mid 1k
R2 mid 0 1k
.op
.end
"#,
    )
    .unwrap();

    assert_eq!(parsed.title.as_deref(), Some("resistor divider"));
    assert!(matches!(
        parsed.circuit.elements(),
        [
            Element::VoltageSource(_),
            Element::Resistor(_),
            Element::Resistor(_)
        ]
    ));
    assert_eq!(parsed.op_cards(), vec![&OpAnalysis]);

    let result = dc_op(&parsed.circuit).unwrap();
    assert_close(result.voltage("mid").unwrap(), 5.0);
}

#[test]
fn parses_reactive_elements_vccs_source_waveforms_and_analysis_cards() {
    let parsed = parse_netlist(
        r#"
Vstep in 0 PULSE(0 1 0 1n 1n 10n 20n)
I1 out 0 1m
Rload in out 2.2k
Cload out 0 10p IC=2.5
L1 out 0 1u IC=3m
G1 out 0 in 0 2m
.tran 1n 20n
.dc Vstep 0 1 0.5
.ac dec 10 1k 1meg
"#,
    )
    .unwrap();

    assert!(matches!(
        parsed.circuit.elements(),
        [
            Element::VoltageSource(_),
            Element::CurrentSource(_),
            Element::Resistor(_),
            Element::Capacitor(_),
            Element::Inductor(_),
            Element::Vccs(_)
        ]
    ));
    let Element::VoltageSource(voltage) = &parsed.circuit.elements()[0] else {
        panic!("expected voltage source");
    };
    assert!(voltage.waveform.is_some());
    let Element::Capacitor(capacitor) = &parsed.circuit.elements()[3] else {
        panic!("expected capacitor");
    };
    assert_close(capacitor.initial_voltage, 2.5);
    let Element::Inductor(inductor) = &parsed.circuit.elements()[4] else {
        panic!("expected inductor");
    };
    assert_close(inductor.initial_current, 3.0e-3);

    assert_eq!(
        parsed.analyses,
        vec![
            Analysis::Tran(TranAnalysis {
                time_step: 1.0e-9,
                stop_time: 20.0e-9,
            }),
            Analysis::Dc(DcAnalysis {
                source_name: "Vstep".to_string(),
                start: 0.0,
                stop: 1.0,
                step: 0.5,
            }),
            Analysis::Ac(AcAnalysis {
                mode: "dec".to_string(),
                points: 10,
                start_hz: 1.0e3,
                stop_hz: 1.0e6,
            }),
        ]
    );
}

#[test]
fn rejects_unsupported_capacitor_element_params() {
    let error = parse_netlist("C1 in 0 1u FOO=1").unwrap_err();

    assert!(error
        .to_string()
        .contains("unsupported capacitor parameter"));
}

#[test]
fn rejects_unsupported_inductor_element_params() {
    let error = parse_netlist("L1 in 0 1u FOO=1").unwrap_err();

    assert!(error.to_string().contains("unsupported inductor parameter"));
}

#[test]
fn parses_tf_transfer_function_analysis_cards() {
    let parsed = parse_netlist(
        r#"
Vin in 0 DC 1
R1 in out 1k
R2 out 0 1k
.tf V(out) Vin
"#,
    )
    .unwrap();

    assert_eq!(
        parsed.analyses,
        vec![Analysis::Tf(TfAnalysis {
            output_node: "out".to_string(),
            input_source: "Vin".to_string(),
        })]
    );
    assert_eq!(
        parsed.tf_cards(),
        vec![&TfAnalysis {
            output_node: "out".to_string(),
            input_source: "Vin".to_string(),
        }]
    );
    let card = parsed.tf_cards()[0];
    let result = tf(&parsed.circuit, &card.output_node, &card.input_source).unwrap();
    assert_close(result.transfer_ratio, 0.5);
}

#[test]
fn rejects_tf_cards_without_voltage_output_probe() {
    let error = parse_netlist(
        r#"
Vin in 0 DC 1
R1 in out 1k
.tf out Vin
"#,
    )
    .unwrap_err();

    assert!(error
        .to_string()
        .contains(".tf output must be a voltage probe"));
}

#[test]
fn parses_sens_dc_sensitivity_analysis_cards() {
    let parsed = parse_netlist(
        r#"
Vin in 0 DC 1
Rtop in out 1k
Rbot out 0 1k
.sens V(out)
"#,
    )
    .unwrap();

    assert_eq!(
        parsed.analyses,
        vec![Analysis::Sens(SensAnalysis {
            output_node: "out".to_string(),
        })]
    );
    assert_eq!(
        parsed.sens_cards(),
        vec![&SensAnalysis {
            output_node: "out".to_string(),
        }]
    );
    let card = parsed.sens_cards()[0];
    let result = sens_dc(&parsed.circuit, &card.output_node).unwrap();
    assert_close(result.nominal_voltage, 0.5);
    assert!(result.entry("Vin", "voltage").is_some());
}

#[test]
fn rejects_sens_cards_without_voltage_output_probe() {
    let error = parse_netlist(
        r#"
Vin in 0 DC 1
R1 in out 1k
.sens out
"#,
    )
    .unwrap_err();

    assert!(error
        .to_string()
        .contains(".sens output must be a voltage probe"));
}

#[test]
fn parses_mc_monte_carlo_dc_analysis_cards() {
    let parsed = parse_netlist(
        r#"
Vin in 0 DC 1
Rtop in out 1k
Rbot out 0 1k
.mc V(out) 6 0 uniform 7
"#,
    )
    .unwrap();

    assert_eq!(
        parsed.analyses,
        vec![Analysis::Mc(McAnalysis {
            output_node: "out".to_string(),
            n_trials: 6,
            tolerance: 0.0,
            distribution: "uniform".to_string(),
            seed: Some(7),
        })]
    );
    assert_eq!(
        parsed.mc_cards(),
        vec![match &parsed.analyses[0] {
            Analysis::Mc(card) => card,
            _ => panic!("expected mc card"),
        }]
    );
    let card = parsed.mc_cards()[0];
    let distribution = match card.distribution.as_str() {
        "gaussian" => McDistribution::Gaussian,
        "uniform" => McDistribution::Uniform,
        other => panic!("unexpected distribution {other}"),
    };
    let result = mc_dc(
        &parsed.circuit,
        &card.output_node,
        card.n_trials,
        McOptions {
            tolerance: card.tolerance,
            distribution,
            seed: card.seed,
        },
    )
    .unwrap();
    assert_eq!(result.n_trials, 6);
    assert_close(result.mean, 0.5);
    assert_close(result.std_dev, 0.0);
}

#[test]
fn rejects_mc_cards_without_voltage_output_probe() {
    let error = parse_netlist(
        r#"
Vin in 0 DC 1
R1 in out 1k
.mc out 10
"#,
    )
    .unwrap_err();

    assert!(error
        .to_string()
        .contains(".mc output must be a voltage probe"));
}

#[test]
fn parses_noise_ac_analysis_cards() {
    let parsed = parse_netlist(
        r#"
Vin in 0 DC 1
Rtop in out 1k
Rbot out 0 1k
.noise V(out) Vin 1k temp=300
"#,
    )
    .unwrap();

    assert_eq!(
        parsed.analyses,
        vec![Analysis::Noise(NoiseAnalysis {
            output_node: "out".to_string(),
            input_source: "Vin".to_string(),
            frequencies_hz: vec![1000.0],
            temperature: 300.0,
        })]
    );
    assert_eq!(
        parsed.noise_cards(),
        vec![match &parsed.analyses[0] {
            Analysis::Noise(card) => card,
            _ => panic!("expected noise card"),
        }]
    );
    let card = parsed.noise_cards()[0];
    let result = noise_ac(
        &parsed.circuit,
        &card.output_node,
        &card.input_source,
        &card.frequencies_hz,
        card.temperature,
    )
    .unwrap();
    assert_eq!(result.output_node, "out");
    assert_eq!(result.input_source, "Vin");
    assert_eq!(result.points.len(), 1);
    assert!(result.points[0].output_psd > 0.0);
}

#[test]
fn rejects_noise_cards_without_voltage_output_probe() {
    let error = parse_netlist(
        r#"
Vin in 0 DC 1
R1 in out 1k
.noise out Vin 1k
"#,
    )
    .unwrap_err();

    assert!(error
        .to_string()
        .contains(".noise output must be a voltage probe"));
}

#[test]
fn parses_source_ac_magnitude_phase_separate_from_dc_bias() {
    let parsed = parse_netlist(
        r#"
Vbias in 0 DC 2.5 AC 1 90
Iprobe out 0 AC 2m
R1 in out 1k
R2 out 0 1k
.ac lin 1 1k 1k
"#,
    )
    .unwrap();

    let Element::VoltageSource(voltage) = &parsed.circuit.elements()[0] else {
        panic!("expected voltage source");
    };
    assert_close(voltage.voltage, 2.5);
    let voltage_ac = voltage.ac.unwrap();
    assert_close(voltage_ac.magnitude, 1.0);
    assert_close(voltage_ac.phase_degrees, 90.0);

    let Element::CurrentSource(current) = &parsed.circuit.elements()[1] else {
        panic!("expected current source");
    };
    assert_close(current.current, 0.0);
    let current_ac = current.ac.unwrap();
    assert_close(current_ac.magnitude, 2.0e-3);
    assert_close(current_ac.phase_degrees, 0.0);

    let points = ac_sweep(&parsed.circuit, 1_000.0, 1_000.0, 1).unwrap();
    assert_eq!(points.len(), 1);
    assert!(points[0].voltage("out").unwrap().imag > 0.0);
}

#[test]
fn parses_vcvs_into_operating_point_circuit() {
    let parsed = parse_netlist(
        r#"
Vctrl in 0 DC 1.5
Eamp out 0 in 0 4
Rload out 0 1k
.op
"#,
    )
    .unwrap();

    let Element::Vcvs(source) = &parsed.circuit.elements()[1] else {
        panic!("expected VCVS");
    };
    assert_eq!(source.control_positive, "in");
    assert_close(source.gain, 4.0);

    let result = dc_op(&parsed.circuit).unwrap();
    assert_close(result.voltage("out").unwrap(), 6.0);
}

#[test]
fn parses_cccs_into_operating_point_circuit() {
    let parsed = parse_netlist(
        r#"
Vin in 0 DC 1
Rin in sense 1k
Vsense sense 0 DC 0
Fcopy out 0 Vsense 2
Rload out 0 500
.op
"#,
    )
    .unwrap();

    let Element::Cccs(source) = &parsed.circuit.elements()[3] else {
        panic!("expected CCCS");
    };
    assert_eq!(source.control_source, "Vsense");
    assert_close(source.gain, 2.0);

    let result = dc_op(&parsed.circuit).unwrap();
    assert_close(result.voltage("out").unwrap(), -1.0);
}

#[test]
fn parses_ccvs_into_operating_point_circuit() {
    let parsed = parse_netlist(
        r#"
Vin in 0 DC 1
Rin in sense 1k
Vsense sense 0 DC 0
Hamp out 0 Vsense 1k
Rload out 0 500
.op
"#,
    )
    .unwrap();

    let Element::Ccvs(source) = &parsed.circuit.elements()[3] else {
        panic!("expected CCVS");
    };
    assert_eq!(source.control_source, "Vsense");
    assert_close(source.transresistance_ohms, 1000.0);

    let result = dc_op(&parsed.circuit).unwrap();
    assert_close(result.voltage("out").unwrap(), 1.0);
}

#[test]
fn parses_diode_models_into_operating_point_circuits() {
    let parsed = parse_netlist(
        r#"
.model fast D(IS=1e-12 VT=25m)
V1 in 0 DC 0.7
D1 in out fast
Rload out 0 1k
.op
"#,
    )
    .unwrap();

    let model = parsed.models.get("fast").unwrap();
    assert_eq!(model.name, "fast");
    assert_eq!(model.kind, "D");
    assert_close(*model.params.get("IS").unwrap(), 1.0e-12);
    assert_close(*model.params.get("VT").unwrap(), 25.0e-3);

    let Element::Diode(diode) = &parsed.circuit.elements()[1] else {
        panic!("expected diode");
    };
    assert_eq!(diode.name, "D1");
    assert_eq!(diode.anode, "in");
    assert_eq!(diode.cathode, "out");
    assert_close(diode.saturation_current, 1.0e-12);
    assert_close(diode.thermal_voltage, 25.0e-3);

    let result = dc_op(&parsed.circuit).unwrap();
    let out = result.voltage("out").unwrap();
    assert!(out > 0.1, "expected forward-biased output, got {out}");
    assert!(out < 0.7, "expected diode drop below source, got {out}");
}

#[test]
fn parses_bjt_models_into_operating_point_circuits() {
    let parsed = parse_netlist(
        r#"
.model fast NPN(IS=1e-13 BF=120 VT=26m)
Vcc vcc 0 DC 5
Vbase base 0 DC 0.7
Q1 vcc base out fast
Rload out 0 1k
.op
"#,
    )
    .unwrap();

    let model = parsed.models.get("fast").unwrap();
    assert_eq!(model.name, "fast");
    assert_eq!(model.kind, "NPN");
    assert_close(*model.params.get("IS").unwrap(), 1.0e-13);
    assert_close(*model.params.get("BF").unwrap(), 120.0);
    assert_close(*model.params.get("VT").unwrap(), 26.0e-3);

    let Element::Bjt(bjt) = &parsed.circuit.elements()[2] else {
        panic!("expected BJT");
    };
    assert_eq!(bjt.name, "Q1");
    assert_eq!(bjt.collector, "vcc");
    assert_eq!(bjt.base, "base");
    assert_eq!(bjt.emitter, "out");
    assert_eq!(bjt.polarity, BjtPolarity::Npn);
    assert_close(bjt.saturation_current, 1.0e-13);
    assert_close(bjt.forward_beta, 120.0);
    assert_close(bjt.thermal_voltage, 26.0e-3);

    let result = dc_op(&parsed.circuit).unwrap();
    let out = result.voltage("out").unwrap();
    assert!(out > 0.0, "expected emitter follower output, got {out}");
    assert!(out < 0.7, "expected output below base bias, got {out}");
}

#[test]
fn parses_pnp_bjt_model_aliases() {
    let parsed = parse_netlist(
        r#"
.model pullup PNP(IS=2e-14 BETA_F=80 VT=27m)
Q1 vcc base out pullup
"#,
    )
    .unwrap();

    let Element::Bjt(bjt) = &parsed.circuit.elements()[0] else {
        panic!("expected BJT");
    };
    assert_eq!(bjt.polarity, BjtPolarity::Pnp);
    assert_close(bjt.saturation_current, 2.0e-14);
    assert_close(bjt.forward_beta, 80.0);
    assert_close(bjt.thermal_voltage, 27.0e-3);
}

#[test]
fn parses_mosfet_models_into_operating_point_circuits() {
    let parsed = parse_netlist(
        r#"
.model nch NMOS(VTO=0.45 KP=250u LAMBDA=0.02 GAMMA=0.3 PHI=0.8 W=2u L=180n NSUB=1.5 TNOM=300)
Vdd vdd 0 DC 5
Vgate gate 0 DC 2.5
M1 vdd gate out 0 nch W=4u L=200n
Rload out 0 1k
.op
"#,
    )
    .unwrap();

    let model = parsed.models.get("nch").unwrap();
    assert_eq!(model.name, "nch");
    assert_eq!(model.kind, "NMOS");
    assert_close(*model.params.get("VTO").unwrap(), 0.45);
    assert_close(*model.params.get("KP").unwrap(), 250.0e-6);

    let Element::Mosfet(mosfet) = &parsed.circuit.elements()[2] else {
        panic!("expected MOSFET");
    };
    assert_eq!(mosfet.name, "M1");
    assert_eq!(mosfet.drain, "vdd");
    assert_eq!(mosfet.gate, "gate");
    assert_eq!(mosfet.source, "out");
    assert_eq!(mosfet.body, "0");
    assert_eq!(mosfet.mosfet_type, MosfetType::Nmos);
    assert_close(mosfet.params.vt0, 0.45);
    assert_close(mosfet.params.kp, 250.0e-6);
    assert_close(mosfet.params.lambda, 0.02);
    assert_close(mosfet.params.gamma, 0.3);
    assert_close(mosfet.params.phi, 0.8);
    assert_close(mosfet.params.w, 4.0e-6);
    assert_close(mosfet.params.l, 200.0e-9);
    assert_close(mosfet.params.n_sub, 1.5);
    assert_close(mosfet.params.t_nom, 300.0);

    let result = dc_op(&parsed.circuit).unwrap();
    let out = result.voltage("out").unwrap();
    assert!(out > 0.0, "expected source follower output, got {out}");
    assert!(out < 2.5, "expected source below gate bias, got {out}");
}

#[test]
fn parses_pmos_mosfet_model_cards() {
    let parsed = parse_netlist(
        r#"
.model pch PMOS(VT0=-0.5 KP=90u W=3u L=180n)
Mpull out gate vdd vdd pch
"#,
    )
    .unwrap();

    let Element::Mosfet(mosfet) = &parsed.circuit.elements()[0] else {
        panic!("expected MOSFET");
    };
    assert_eq!(mosfet.mosfet_type, MosfetType::Pmos);
    assert_close(mosfet.params.vt0, -0.5);
    assert_close(mosfet.params.kp, 90.0e-6);
    assert_close(mosfet.params.w, 3.0e-6);
}

#[test]
fn parses_pwl_and_sin_source_waveforms() {
    let parsed = parse_netlist(
        r#"
V1 in 0 PWL(0 0, 1n 1.8, 2n 0)
I1 in 0 SIN(0 2m 1k 10u 5)
"#,
    )
    .unwrap();

    let Element::VoltageSource(voltage) = &parsed.circuit.elements()[0] else {
        panic!("expected voltage source");
    };
    let Element::CurrentSource(current) = &parsed.circuit.elements()[1] else {
        panic!("expected current source");
    };
    assert_close(voltage.waveform.as_ref().unwrap().value_at(0.5e-9), 0.9);
    assert_close(current.waveform.as_ref().unwrap().value_at(1.0e-6), 0.0);
}

#[test]
fn expands_subcircuit_instances_into_engine_elements() {
    let parsed = parse_netlist(
        r#"
.subckt divider top mid bot
Rtop top mid 1k
Rbot mid bot 1k
.ends divider
V1 vin 0 DC 10
Xdiv vin mid 0 divider
.op
"#,
    )
    .unwrap();

    let elements = parsed.circuit.elements();
    let names = elements
        .iter()
        .map(|element| match element {
            Element::VoltageSource(source) => source.name.as_str(),
            Element::Resistor(resistor) => resistor.name.as_str(),
            _ => panic!("unexpected element"),
        })
        .collect::<Vec<_>>();
    assert_eq!(names, vec!["V1", "Xdiv.Rtop", "Xdiv.Rbot"]);

    let Element::Resistor(resistor) = &elements[1] else {
        panic!("expected resistor");
    };
    assert_eq!(resistor.n1, "vin");
    assert_eq!(resistor.n2, "mid");

    let result = dc_op(&parsed.circuit).unwrap();
    assert_close(result.voltage("mid").unwrap(), 5.0);
}

#[test]
fn expands_subcircuit_vcvs_nodes_into_engine_elements() {
    let parsed = parse_netlist(
        r#"
.subckt gain inp outp
Ebuf outp 0 inp 0 2
.ends gain
V1 in 0 DC 1.25
Xgain in out gain
Rload out 0 1k
.op
"#,
    )
    .unwrap();

    let elements = parsed.circuit.elements();
    let names = elements
        .iter()
        .map(|element| match element {
            Element::VoltageSource(source) => source.name.as_str(),
            Element::Vcvs(source) => source.name.as_str(),
            Element::Resistor(resistor) => resistor.name.as_str(),
            _ => panic!("unexpected element"),
        })
        .collect::<Vec<_>>();
    assert_eq!(names, vec!["V1", "Xgain.Ebuf", "Rload"]);

    let Element::Vcvs(source) = &elements[1] else {
        panic!("expected VCVS");
    };
    assert_eq!(source.positive, "out");
    assert_eq!(source.control_positive, "in");

    let result = dc_op(&parsed.circuit).unwrap();
    assert_close(result.voltage("out").unwrap(), 2.5);
}

#[test]
fn expands_subcircuit_cccs_control_sources_into_engine_elements() {
    let parsed = parse_netlist(
        r#"
.subckt mirror inp outp
Rin inp sense 1k
Vsense sense 0 DC 0
Fcopy outp 0 Vsense 2
.ends mirror
Vin in 0 DC 1
Xmirror in out mirror
Rload out 0 500
.op
"#,
    )
    .unwrap();

    let elements = parsed.circuit.elements();
    let names = elements
        .iter()
        .map(|element| match element {
            Element::VoltageSource(source) => source.name.as_str(),
            Element::Resistor(resistor) => resistor.name.as_str(),
            Element::Cccs(source) => source.name.as_str(),
            _ => panic!("unexpected element"),
        })
        .collect::<Vec<_>>();
    assert_eq!(
        names,
        vec![
            "Vin",
            "Xmirror.Rin",
            "Xmirror.Vsense",
            "Xmirror.Fcopy",
            "Rload"
        ]
    );

    let Element::Cccs(source) = &elements[3] else {
        panic!("expected CCCS");
    };
    assert_eq!(source.positive, "out");
    assert_eq!(source.control_source, "Xmirror.Vsense");

    let result = dc_op(&parsed.circuit).unwrap();
    assert_close(result.voltage("out").unwrap(), -1.0);
}

#[test]
fn expands_subcircuit_ccvs_control_sources_into_engine_elements() {
    let parsed = parse_netlist(
        r#"
.subckt transimpedance inp outp
Rin inp sense 1k
Vsense sense 0 DC 0
Hamp outp 0 Vsense 1k
.ends transimpedance
Vin in 0 DC 1
Xamp in out transimpedance
Rload out 0 500
.op
"#,
    )
    .unwrap();

    let elements = parsed.circuit.elements();
    let names = elements
        .iter()
        .map(|element| match element {
            Element::VoltageSource(source) => source.name.as_str(),
            Element::Resistor(resistor) => resistor.name.as_str(),
            Element::Ccvs(source) => source.name.as_str(),
            _ => panic!("unexpected element"),
        })
        .collect::<Vec<_>>();
    assert_eq!(
        names,
        vec!["Vin", "Xamp.Rin", "Xamp.Vsense", "Xamp.Hamp", "Rload"]
    );

    let Element::Ccvs(source) = &elements[3] else {
        panic!("expected CCVS");
    };
    assert_eq!(source.positive, "out");
    assert_eq!(source.control_source, "Xamp.Vsense");
    assert_close(source.transresistance_ohms, 1000.0);

    let result = dc_op(&parsed.circuit).unwrap();
    assert_close(result.voltage("out").unwrap(), 1.0);
}

#[test]
fn expands_subcircuit_diode_nodes_into_engine_elements() {
    let parsed = parse_netlist(
        r#"
.model clamp D(IS=1e-12 VT=25m)
.subckt limiter inp outp
Dlim inp outp clamp
.ends limiter
Xlim in out limiter
"#,
    )
    .unwrap();

    let Element::Diode(diode) = &parsed.circuit.elements()[0] else {
        panic!("expected diode");
    };
    assert_eq!(diode.name, "Xlim.Dlim");
    assert_eq!(diode.anode, "in");
    assert_eq!(diode.cathode, "out");
}

#[test]
fn expands_subcircuit_bjt_nodes_into_engine_elements() {
    let parsed = parse_netlist(
        r#"
.model fast NPN(IS=1e-13 BF=120)
.subckt follower c b e
Qdrive c b e fast
.ends follower
Xbuf vcc in out follower
"#,
    )
    .unwrap();

    let Element::Bjt(bjt) = &parsed.circuit.elements()[0] else {
        panic!("expected BJT");
    };
    assert_eq!(bjt.name, "Xbuf.Qdrive");
    assert_eq!(bjt.collector, "vcc");
    assert_eq!(bjt.base, "in");
    assert_eq!(bjt.emitter, "out");
}

#[test]
fn expands_subcircuit_mosfet_nodes_into_engine_elements() {
    let parsed = parse_netlist(
        r#"
.model nch NMOS(KP=220u)
.subckt source_follower d g s b
Mdrive d g s b nch W=2u
.ends source_follower
Xbuf vdd in out 0 source_follower
"#,
    )
    .unwrap();

    let Element::Mosfet(mosfet) = &parsed.circuit.elements()[0] else {
        panic!("expected MOSFET");
    };
    assert_eq!(mosfet.name, "Xbuf.Mdrive");
    assert_eq!(mosfet.drain, "vdd");
    assert_eq!(mosfet.gate, "in");
    assert_eq!(mosfet.source, "out");
    assert_eq!(mosfet.body, "0");
    assert_close(mosfet.params.w, 2.0e-6);
}

#[test]
fn scopes_subcircuit_internal_nodes_by_instance() {
    let parsed = parse_netlist(
        r#"
.subckt load in out
R1 in inner 1k
C1 inner out 1u
.ends load
Xleft a b load
Xright c d load
"#,
    )
    .unwrap();

    let Element::Resistor(left_resistor) = &parsed.circuit.elements()[0] else {
        panic!("expected resistor");
    };
    let Element::Resistor(right_resistor) = &parsed.circuit.elements()[2] else {
        panic!("expected resistor");
    };
    assert_eq!(left_resistor.n2, "Xleft.inner");
    assert_eq!(right_resistor.n2, "Xright.inner");
}

#[test]
fn parses_engineering_suffixes() {
    assert_eq!(parse_value("1k").unwrap(), 1.0e3);
    assert_eq!(parse_value("2.2meg").unwrap(), 2.2e6);
    assert_eq!(parse_value("3u").unwrap(), 3.0e-6);
    assert_eq!(parse_value("4n").unwrap(), 4.0e-9);
}

#[test]
fn rejects_unsupported_elements_with_line_numbers() {
    let err = parse_netlist("\nZ1 c b e model\n").unwrap_err();

    assert!(err.to_string().contains("line 2: unsupported element"));
    assert_error_type(err);
}

#[test]
fn rejects_unknown_diode_models() {
    let err = parse_netlist("D1 a 0 missing\n").unwrap_err();

    assert!(err
        .to_string()
        .contains("line 1: unknown model \"missing\" for diode \"D1\""));
}

#[test]
fn rejects_non_diode_models_for_diode_elements() {
    let err = parse_netlist(".model amp NPN(IS=1e-12)\nD1 a 0 amp\n").unwrap_err();

    assert!(err
        .to_string()
        .contains("line 2: model \"amp\" has kind \"NPN\", expected \"D\""));
}

#[test]
fn rejects_unknown_bjt_models() {
    let err = parse_netlist("Q1 c b e missing\n").unwrap_err();

    assert!(err
        .to_string()
        .contains("line 1: unknown model \"missing\" for BJT \"Q1\""));
}

#[test]
fn rejects_non_bjt_models_for_bjt_elements() {
    let err = parse_netlist(".model clamp D(IS=1e-12)\nQ1 c b e clamp\n").unwrap_err();

    assert!(err
        .to_string()
        .contains("line 2: model \"clamp\" has kind \"D\", expected \"NPN\" or \"PNP\""));
}

#[test]
fn rejects_unknown_mosfet_models() {
    let err = parse_netlist("M1 d g s b missing\n").unwrap_err();

    assert!(err
        .to_string()
        .contains("line 1: unknown model \"missing\" for MOSFET \"M1\""));
}

#[test]
fn rejects_non_mosfet_models_for_mosfet_elements() {
    let err = parse_netlist(".model clamp D(IS=1e-12)\nM1 d g s b clamp\n").unwrap_err();

    assert!(err
        .to_string()
        .contains("line 2: model \"clamp\" has kind \"D\", expected \"NMOS\" or \"PMOS\""));
}

#[test]
fn rejects_invalid_mosfet_instance_parameters() {
    let err = parse_netlist(".model nch NMOS(KP=220u)\nM1 d g s b nch W\n").unwrap_err();

    assert!(err
        .to_string()
        .contains("line 2: invalid MOSFET parameter syntax \"W\""));
}

#[test]
fn rejects_unbalanced_waveform_parentheses() {
    let err = parse_netlist("V1 in 0 PULSE(0 1\n").unwrap_err();

    assert!(err.to_string().contains("unclosed parenthesis"));
}

#[test]
fn rejects_unknown_subcircuit_instances() {
    let err = parse_netlist("X1 a b missing\n").unwrap_err();

    assert!(err
        .to_string()
        .contains("line 1: unknown subcircuit \"missing\""));
}

fn assert_error_type(_: NetlistParseError) {}
