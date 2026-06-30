use spice_engine::{
    ac_sweep, dc_op, dc_op_with_options, mc_dc, noise_ac, sens_dc, tf, transient_adaptive,
    BjtPolarity, Element, JfetPolarity, McDistribution, McOptions, MosfetType, TransientMethod,
};
use spice_netlist_parser::{
    berkeley_app_package_manifest, berkeley_app_package_manifest_json, build_analysis_plan,
    parse_berkeley_app_deck, parse_berkeley_syntax, parse_netlist, parse_value, run_netlist,
    AcAnalysis, Analysis, AnalysisKind, AnalysisResult, BerkeleyAppEditorActionKind,
    BerkeleyAppHostPanelKind, BerkeleyAppPersistedEditorState, BerkeleyCardKind,
    BerkeleyDiagnosticSeverity, BerkeleyGrammarMetadata, DcAnalysis, DistortionAnalysis,
    FourAnalysis, McAnalysis, MeasureAnalysis, MeasureOperation, NetlistParseError, NoiseAnalysis,
    OpAnalysis, OptionValue, OutputProbe, PlotAnalysis, PoleZeroAnalysis, PoleZeroKind,
    PrintAnalysis, ProbeAnalysis, SaveAnalysis, SelectedOutputValue, SensAnalysis, TempAnalysis,
    TfAnalysis, TranAnalysis, BERKELEY_APP_BOOTSTRAP_SCHEMA_VERSION,
    BERKELEY_APP_HOST_SURFACE_WIRE_SCHEMA_VERSION, BERKELEY_APP_PACKAGE_MANIFEST_SCHEMA_VERSION,
    BERKELEY_APP_PACKAGE_NAME, BERKELEY_APP_SOURCE_FINGERPRINT_ALGORITHM,
    BERKELEY_SPICE_GRAMMAR_NAME, BERKELEY_SPICE_GRAMMAR_VERSION,
};

fn assert_close(actual: f64, expected: f64) {
    assert!(
        (actual - expected).abs() < 1.0e-9,
        "expected {expected}, got {actual}"
    );
}

fn selected_real(value: &SelectedOutputValue) -> f64 {
    match value {
        SelectedOutputValue::Real(value) => *value,
        SelectedOutputValue::Complex(_) => panic!("expected real selected output value"),
    }
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
fn builds_and_runs_core_analysis_plan() {
    let deck = r#"
V1 in 0 DC 1 AC 1
R1 in out 1k
R2 out 0 1k
C1 out 0 1u IC=0
.options method=trap
.op
.dc V1 0 1 0.5
.ac dec 1 1k 1k
.tran 1m 1m
.end
"#;
    let parsed = parse_netlist(deck).unwrap();

    let plan = parsed.analysis_plan();
    assert_eq!(plan, build_analysis_plan(&parsed));
    assert_eq!(
        plan.iter()
            .map(|step| (step.index, step.kind))
            .collect::<Vec<_>>(),
        vec![
            (1, AnalysisKind::Op),
            (2, AnalysisKind::Dc),
            (3, AnalysisKind::Ac),
            (4, AnalysisKind::Tran),
        ]
    );

    let results = parsed.run_analysis_plan().unwrap();
    assert_eq!(
        results.iter().map(|result| result.kind).collect::<Vec<_>>(),
        vec![
            AnalysisKind::Op,
            AnalysisKind::Dc,
            AnalysisKind::Ac,
            AnalysisKind::Tran
        ]
    );
    let AnalysisResult::Op(op) = &results[0].result else {
        panic!("expected .op result");
    };
    assert_close(op.voltage("out").unwrap(), 0.5);
    let AnalysisResult::Dc(dc_points) = &results[1].result else {
        panic!("expected .dc result");
    };
    assert_eq!(dc_points.len(), 3);
    assert_close(
        dc_points.last().unwrap().result.voltage("out").unwrap(),
        0.5,
    );
    let AnalysisResult::Ac(ac_points) = &results[2].result else {
        panic!("expected .ac result");
    };
    assert_eq!(ac_points.len(), 1);
    assert!(ac_points[0].voltage("out").unwrap().abs() > 0.0);
    let AnalysisResult::Tran(transient_points) = &results[3].result else {
        panic!("expected .tran result");
    };
    assert_eq!(transient_points.len(), 1);
    assert!(transient_points[0].voltage("out").unwrap() > 0.0);

    assert_eq!(run_netlist(deck).unwrap().len(), 4);
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
                method: None,
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
fn parses_mutual_inductor_cards() {
    let parsed = parse_netlist(
        r#"
Lpri p 0 10m
Lsec s 0 40m
Kcouple Lpri Lsec 0.75
"#,
    )
    .unwrap();

    let Element::MutualInductor(mutual) = &parsed.circuit.elements()[2] else {
        panic!("expected mutual inductor");
    };
    assert_eq!(mutual.name, "Kcouple");
    assert_eq!(mutual.primary, "Lpri");
    assert_eq!(mutual.secondary, "Lsec");
    assert_close(mutual.coupling, 0.75);
}

#[test]
fn rejects_mutual_inductor_missing_referenced_inductor() {
    let error = parse_netlist(
        r#"
Lpri p 0 10m
Kbad Lpri Lmissing 0.75
"#,
    )
    .unwrap_err();

    assert!(error.to_string().contains("referenced inductor"));
}

#[test]
fn rejects_mutual_inductor_non_finite_coupling() {
    let error = parse_netlist(
        r#"
Lpri p 0 10m
Lsec s 0 40m
Kbad Lpri Lsec 1e999
"#,
    )
    .unwrap_err();

    assert!(error.to_string().contains("coupling must be finite"));
}

#[test]
fn parses_transmission_line_card() {
    let parsed = parse_netlist(
        r#"
Tdelay in 0 out 0 Z0=50 TD=1n
"#,
    )
    .unwrap();

    let Element::TransmissionLine(line) = &parsed.circuit.elements()[0] else {
        panic!("expected transmission line");
    };
    assert_eq!(line.name, "Tdelay");
    assert_eq!(line.n1, "in");
    assert_eq!(line.n2, "0");
    assert_eq!(line.n3, "out");
    assert_eq!(line.n4, "0");
    assert_close(line.characteristic_impedance_ohms, 50.0);
    assert_close(line.delay_seconds, 1.0e-9);
}

#[test]
fn rejects_transmission_line_positional_form() {
    let error = parse_netlist("Tdelay in 0 out 0 50 1n").unwrap_err();

    assert!(error
        .to_string()
        .contains("invalid transmission line parameter syntax"));
}

#[test]
fn rejects_transmission_line_missing_parameter() {
    let error = parse_netlist("Tdelay in 0 out 0 Z0=50").unwrap_err();

    assert!(error.to_string().contains("requires TD"));
}

#[test]
fn rejects_transmission_line_non_positive_parameters() {
    let impedance_error = parse_netlist("Tdelay in 0 out 0 Z0=0 TD=1n").unwrap_err();
    assert!(impedance_error
        .to_string()
        .contains("characteristic impedance must be positive"));

    let delay_error = parse_netlist("Tdelay in 0 out 0 Z0=50 TD=0").unwrap_err();
    assert!(delay_error.to_string().contains("delay must be positive"));
}

#[test]
fn parses_options_analysis_cards() {
    let parsed = parse_netlist(
        r#"
.options reltol=1m abstol=1n gmin=1p method=trap noopiter
"#,
    )
    .unwrap();

    let cards = parsed.options_cards();
    let [card] = cards.as_slice() else {
        panic!("expected one options card");
    };
    assert_option_number(card.values.get("reltol"), 1.0e-3);
    assert_option_number(card.values.get("abstol"), 1.0e-9);
    assert_option_number(card.values.get("gmin"), 1.0e-12);
    assert_eq!(
        card.values.get("method"),
        Some(&OptionValue::Text("trap".to_string()))
    );
    assert_eq!(card.values.get("noopiter"), Some(&OptionValue::Flag(true)));
    assert!(matches!(parsed.analyses.as_slice(), [Analysis::Options(_)]));
}

#[test]
fn options_cards_build_engine_call_options() {
    let parsed = parse_netlist(
        r#"
V1 vin 0 DC 10
R1 vin mid 1k
R2 mid 0 1k
.options reltol=1u itl1=7 gmin=1p method=gear2 trtol=2m minstep=1n maxstep=5n
.op
.tran 1n 2n
"#,
    )
    .unwrap();
    let tran = parsed.tran_cards()[0];

    let dc_options = parsed.dc_op_options().unwrap();
    assert_eq!(dc_options.max_iterations, 7);
    assert_close(dc_options.tolerance, 1.0e-6);
    assert_close(dc_options.pseudo_transient_conductance, 1.0e-12);
    let result = dc_op_with_options(&parsed.circuit, dc_options).unwrap();
    assert_close(result.voltage("mid").unwrap(), 5.0);

    let transient_options = parsed.adaptive_transient_options(Some(tran)).unwrap();
    assert_eq!(transient_options.method, TransientMethod::Gear2);
    assert_close(transient_options.tolerance, 2.0e-3);
    assert_close(transient_options.min_step.unwrap(), 1.0e-9);
    assert_close(transient_options.max_step.unwrap(), 5.0e-9);
    let transient = transient_adaptive(
        &parsed.circuit,
        tran.time_step,
        tran.stop_time,
        transient_options,
    )
    .unwrap();
    assert!(transient.converged);
    assert_eq!(transient.method, TransientMethod::Gear2);
}

#[test]
fn parses_temp_analysis_cards() {
    let parsed = parse_netlist(".temp 27 75 -40").unwrap();

    assert_eq!(
        parsed.analyses,
        vec![Analysis::Temp(TempAnalysis {
            temperatures_celsius: vec![27.0, 75.0, -40.0],
        })]
    );
    let cards = parsed.temp_cards();
    let [card] = cards.as_slice() else {
        panic!("expected one temp card");
    };
    assert_eq!(card.temperatures_celsius, vec![27.0, 75.0, -40.0]);
    assert_close(
        parsed.operating_temperature_kelvin(0, 300.0).unwrap(),
        300.15,
    );
    assert_close(
        parsed.operating_temperature_kelvin(1, 300.0).unwrap(),
        348.15,
    );
}

#[test]
fn defaults_operating_temperature_without_temp_cards() {
    let parsed = parse_netlist("R1 in out 1k").unwrap();

    assert_close(
        parsed.operating_temperature_kelvin(0, 301.0).unwrap(),
        301.0,
    );
    let error = parse_netlist(".temp 27")
        .unwrap()
        .operating_temperature_kelvin(3, 300.0)
        .unwrap_err();
    assert!(error
        .to_string()
        .contains("temperature index 3 exceeds .temp entries"));
}

#[test]
fn rejects_temp_cards_without_temperatures() {
    let error = parse_netlist(".temp").unwrap_err();

    assert!(error
        .to_string()
        .contains(".temp expects at least 2 fields"));
}

#[test]
fn parses_print_and_plot_output_cards() {
    let parsed = parse_netlist(
        r#"
.print TRAN V(out) I(Vin)
.plot ac V(in) V(out)
"#,
    )
    .unwrap();

    assert_eq!(
        parsed.analyses,
        vec![
            Analysis::Print(PrintAnalysis {
                analysis: "tran".to_string(),
                probes: vec![
                    OutputProbe::Voltage {
                        node: "out".to_string()
                    },
                    OutputProbe::Current {
                        source_name: "Vin".to_string()
                    },
                ],
            }),
            Analysis::Plot(PlotAnalysis {
                analysis: "ac".to_string(),
                probes: vec![
                    OutputProbe::Voltage {
                        node: "in".to_string()
                    },
                    OutputProbe::Voltage {
                        node: "out".to_string()
                    },
                ],
            }),
        ]
    );
    assert!(matches!(parsed.print_cards().as_slice(), [_]));
    assert!(matches!(parsed.plot_cards().as_slice(), [_]));
}

#[test]
fn parses_save_probe_and_measure_cards() {
    let parsed = parse_netlist(
        r#"
.save V(out) I(Vin)
.probe tran V(out)
.measure tran peak MAX V(out) FROM=0 TO=1m
"#,
    )
    .unwrap();

    assert_eq!(
        parsed.analyses,
        vec![
            Analysis::Save(SaveAnalysis {
                probes: vec![
                    OutputProbe::Voltage {
                        node: "out".to_string()
                    },
                    OutputProbe::Current {
                        source_name: "Vin".to_string()
                    },
                ],
            }),
            Analysis::Probe(ProbeAnalysis {
                analysis: Some("tran".to_string()),
                probes: vec![OutputProbe::Voltage {
                    node: "out".to_string()
                }],
            }),
            Analysis::Measure(MeasureAnalysis {
                analysis: "tran".to_string(),
                name: "peak".to_string(),
                operation: MeasureOperation::Max,
                probe: OutputProbe::Voltage {
                    node: "out".to_string()
                },
                at: None,
                start: Some(0.0),
                stop: Some(1.0e-3),
            }),
        ]
    );
    assert!(matches!(parsed.save_cards().as_slice(), [_]));
    assert!(matches!(parsed.probe_cards().as_slice(), [_]));
    assert!(matches!(parsed.measure_cards().as_slice(), [_]));
}

#[test]
fn rejects_output_cards_with_missing_or_unknown_probes() {
    let missing_error = parse_netlist(".print tran").unwrap_err();
    assert!(missing_error
        .to_string()
        .contains(".print expects at least 3 fields"));

    let probe_error = parse_netlist(".plot tran P(out)").unwrap_err();
    assert!(probe_error
        .to_string()
        .contains(".plot probe must be V(node) or I(source)"));

    let save_error = parse_netlist(".save P(out)").unwrap_err();
    assert!(save_error
        .to_string()
        .contains(".save probe must be V(node) or I(source)"));

    let probe_error = parse_netlist(".probe tran").unwrap_err();
    assert!(probe_error
        .to_string()
        .contains(".probe probe must be V(node) or I(source)"));

    let measure_at_error = parse_netlist(".measure tran final FIND V(out)").unwrap_err();
    assert!(measure_at_error
        .to_string()
        .contains(".measure FIND requires AT=<value>"));

    let measure_operation_error =
        parse_netlist(".measure tran peak PEAK V(out) AT=1m").unwrap_err();
    assert!(measure_operation_error
        .to_string()
        .contains(".measure operation must be FIND"));
}

#[test]
fn selects_outputs_and_evaluates_measure_results_from_analysis_plan() {
    let deck = r#"
V1 in 0 DC 1 AC 1
R1 in out 1k
R2 out 0 1k
C1 out 0 1u IC=0
.save V(out)
.print dc V(in)
.probe tran I(V1)
.measure dc half FIND V(out) AT=1
.measure tran final FIND V(out) AT=1m
.measure tran average AVG V(out)
.op
.dc V1 0 1 0.5
.ac dec 1 1k 1k
.tran 1m 1m
.end
"#;
    let parsed = parse_netlist(deck).unwrap();
    let results = parsed.run_analysis_plan().unwrap();

    let outputs = parsed.select_outputs(&results).unwrap();
    assert_eq!(
        outputs.iter().map(|output| output.kind).collect::<Vec<_>>(),
        vec![
            AnalysisKind::Op,
            AnalysisKind::Dc,
            AnalysisKind::Ac,
            AnalysisKind::Tran,
        ]
    );
    assert_close(
        selected_real(outputs[0].rows[0].values.get("V(out)").unwrap()),
        0.5,
    );
    assert_close(
        selected_real(outputs[1].rows.last().unwrap().values.get("V(in)").unwrap()),
        1.0,
    );
    assert!(matches!(
        outputs[2].rows[0].values.get("V(out)").unwrap(),
        SelectedOutputValue::Complex(_)
    ));
    assert!(outputs[3].rows.last().unwrap().values.contains_key("I(V1)"));

    let measures = parsed.measure_results(&results).unwrap();
    assert_eq!(
        measures
            .iter()
            .map(|measure| measure.name.as_str())
            .collect::<Vec<_>>(),
        vec!["half", "final", "average"]
    );
    assert_close(measures[0].value, 0.5);
    let final_voltage = selected_real(
        outputs[3]
            .rows
            .last()
            .unwrap()
            .values
            .get("V(out)")
            .unwrap(),
    );
    assert_close(measures[1].value, final_voltage);
    assert!(measures[2].value > 0.0);
    assert!(measures[2].value <= final_voltage);
}

#[test]
fn parses_four_analysis_cards() {
    let parsed = parse_netlist(".four 1k V(out) I(Vin)").unwrap();

    assert_eq!(
        parsed.analyses,
        vec![Analysis::Four(FourAnalysis {
            frequency_hz: 1.0e3,
            probes: vec![
                OutputProbe::Voltage {
                    node: "out".to_string()
                },
                OutputProbe::Current {
                    source_name: "Vin".to_string()
                },
            ],
        })]
    );
    assert!(matches!(parsed.four_cards().as_slice(), [_]));
}

#[test]
fn rejects_four_cards_with_missing_or_unknown_probes() {
    let missing_error = parse_netlist(".four 1k").unwrap_err();
    assert!(missing_error
        .to_string()
        .contains(".four expects at least 3 fields"));

    let probe_error = parse_netlist(".four 1k P(out)").unwrap_err();
    assert!(probe_error
        .to_string()
        .contains(".four probe must be V(node) or I(source)"));
}

#[test]
fn parses_distortion_and_pole_zero_analysis_cards() {
    let parsed = parse_netlist(
        r#"
.disto dec 5 1k 1meg V(out) I(Vin)
.pz V(out) Vin pole
"#,
    )
    .unwrap();

    assert_eq!(
        parsed.analyses,
        vec![
            Analysis::Distortion(DistortionAnalysis {
                mode: "dec".to_string(),
                points: 5,
                start_hz: 1.0e3,
                stop_hz: 1.0e6,
                probes: vec![
                    OutputProbe::Voltage {
                        node: "out".to_string()
                    },
                    OutputProbe::Current {
                        source_name: "Vin".to_string()
                    },
                ],
            }),
            Analysis::PoleZero(PoleZeroAnalysis {
                output_node: "out".to_string(),
                input_source: "Vin".to_string(),
                kind: PoleZeroKind::Pole,
            }),
        ]
    );
    assert!(matches!(parsed.distortion_cards().as_slice(), [_]));
    assert!(matches!(parsed.pole_zero_cards().as_slice(), [_]));
}

#[test]
fn rejects_distortion_and_pole_zero_cards_with_invalid_shapes() {
    let missing_error = parse_netlist(".disto dec 5 1k 1meg").unwrap_err();
    assert!(missing_error
        .to_string()
        .contains(".disto expects at least 6 fields"));

    let probe_error = parse_netlist(".disto dec 5 1k 1meg P(out)").unwrap_err();
    assert!(probe_error
        .to_string()
        .contains(".disto probe must be V(node) or I(source)"));

    let output_error = parse_netlist(".pz out Vin").unwrap_err();
    assert!(output_error
        .to_string()
        .contains(".pz output must be a voltage probe"));

    let kind_error = parse_netlist(".pz V(out) Vin residue").unwrap_err();
    assert!(kind_error.to_string().contains(".pz kind must be"));
}

#[test]
fn parses_transient_methods_from_tran_cards() {
    let parsed = parse_netlist(".tran 1n 20n method=gear2").unwrap();

    assert_eq!(
        parsed.tran_cards(),
        vec![&TranAnalysis {
            time_step: 1.0e-9,
            stop_time: 20.0e-9,
            method: Some(TransientMethod::Gear2),
        }]
    );
    assert_eq!(
        parsed
            .transient_method(parsed.tran_cards().first().copied())
            .unwrap(),
        Some(TransientMethod::Gear2)
    );
}

#[test]
fn transient_method_falls_back_to_options_and_tran_takes_precedence() {
    let parsed = parse_netlist(
        r#"
.options method=trap
.tran 1n 20n method=euler
"#,
    )
    .unwrap();

    assert_eq!(
        parsed.options_cards()[0].values.get("method"),
        Some(&OptionValue::Text("trap".to_string()))
    );
    assert_eq!(
        parsed.transient_method(None).unwrap(),
        Some(TransientMethod::Trap)
    );
    assert_eq!(
        parsed
            .transient_method(parsed.tran_cards().first().copied())
            .unwrap(),
        Some(TransientMethod::Euler)
    );
}

#[test]
fn rejects_unsupported_transient_methods() {
    let tran_error = parse_netlist(".tran 1n 20n method=bogus").unwrap_err();
    assert!(tran_error
        .to_string()
        .contains("must be euler, trap, or gear2"));

    let option_error = parse_netlist(".options method=bogus").unwrap_err();
    assert!(option_error
        .to_string()
        .contains("must be euler, trap, or gear2"));
}

fn assert_option_number(actual: Option<&OptionValue>, expected: f64) {
    let Some(OptionValue::Number(actual)) = actual else {
        panic!("expected numeric option, got {actual:?}");
    };
    assert_close(*actual, expected);
}

#[test]
fn rejects_options_cards_with_empty_values() {
    let error = parse_netlist(".options gmin=").unwrap_err();

    assert!(error
        .to_string()
        .contains(".options \"gmin\" requires a value"));
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
.temp 75
Vin in 0 DC 1
Rtop in out 1k
Rbot out 0 1k
.noise V(out) Vin 1k temp=300
"#,
    )
    .unwrap();

    assert_eq!(
        parsed.analyses,
        vec![
            Analysis::Temp(TempAnalysis {
                temperatures_celsius: vec![75.0],
            }),
            Analysis::Noise(NoiseAnalysis {
                output_node: "out".to_string(),
                input_source: "Vin".to_string(),
                frequencies_hz: vec![1000.0],
                temperature: 300.0,
                temperature_is_explicit: true,
            })
        ]
    );
    assert_eq!(
        parsed.noise_cards(),
        vec![match &parsed.analyses[1] {
            Analysis::Noise(card) => card,
            _ => panic!("expected noise card"),
        }]
    );
    let card = parsed.noise_cards()[0];
    assert_close(
        parsed
            .noise_temperature_kelvin(Some(card), 0, 300.0)
            .unwrap(),
        300.0,
    );
    let result = noise_ac(
        &parsed.circuit,
        &card.output_node,
        &card.input_source,
        &card.frequencies_hz,
        parsed
            .noise_temperature_kelvin(Some(card), 0, 300.0)
            .unwrap(),
    )
    .unwrap();
    assert_eq!(result.output_node, "out");
    assert_eq!(result.input_source, "Vin");
    assert_eq!(result.points.len(), 1);
    assert!(result.points[0].output_psd > 0.0);
}

#[test]
fn noise_analysis_uses_temp_card_when_noise_temp_is_omitted() {
    let parsed = parse_netlist(
        r#"
.temp 50
Vin in 0 DC 1
Rtop in out 1k
Rbot out 0 1k
.noise V(out) Vin 1k
"#,
    )
    .unwrap();
    let card = parsed.noise_cards()[0];

    assert!(!card.temperature_is_explicit);
    assert_close(
        parsed
            .noise_temperature_kelvin(Some(card), 0, 300.0)
            .unwrap(),
        323.15,
    );
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
.model fast D(IS=1e-12 VT=25m N=2 BV=5 IBV=1u CJO=2p TT=4n)
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
    assert_close(*model.params.get("N").unwrap(), 2.0);
    assert_close(*model.params.get("BV").unwrap(), 5.0);
    assert_close(*model.params.get("IBV").unwrap(), 1.0e-6);
    assert_close(*model.params.get("CJO").unwrap(), 2.0e-12);
    assert_close(*model.params.get("TT").unwrap(), 4.0e-9);

    let Element::Diode(diode) = &parsed.circuit.elements()[1] else {
        panic!("expected diode");
    };
    assert_eq!(diode.name, "D1");
    assert_eq!(diode.anode, "in");
    assert_eq!(diode.cathode, "out");
    assert_close(diode.saturation_current, 1.0e-12);
    assert_close(diode.thermal_voltage, 25.0e-3);
    assert_close(diode.emission_coefficient, 2.0);
    assert_close(diode.breakdown_voltage.unwrap(), 5.0);
    assert_close(diode.breakdown_current, 1.0e-6);
    assert_close(diode.junction_capacitance, 2.0e-12);
    assert_close(diode.transit_time, 4.0e-9);

    let result = dc_op(&parsed.circuit).unwrap();
    let out = result.voltage("out").unwrap();
    assert!(out > 0.0, "expected forward-biased output, got {out}");
    assert!(out < 0.7, "expected diode drop below source, got {out}");
}

#[test]
fn parses_bjt_models_into_operating_point_circuits() {
    let parsed = parse_netlist(
        r#"
.model fast NPN(IS=1e-13 BF=120 VT=26m CJE=2p CJC=3p TF=4n TR=5n)
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
    assert_close(*model.params.get("CJE").unwrap(), 2.0e-12);
    assert_close(*model.params.get("CJC").unwrap(), 3.0e-12);
    assert_close(*model.params.get("TF").unwrap(), 4.0e-9);
    assert_close(*model.params.get("TR").unwrap(), 5.0e-9);

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
    assert_close(bjt.base_emitter_capacitance, 2.0e-12);
    assert_close(bjt.base_collector_capacitance, 3.0e-12);
    assert_close(bjt.forward_transit_time, 4.0e-9);
    assert_close(bjt.reverse_transit_time, 5.0e-9);

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
fn parses_jfet_model_cards() {
    let parsed = parse_netlist(
        r#"
.model nch NJF(BETA=2m VTO=-3 LAMBDA=0.02)
J1 drain gate source nch
"#,
    )
    .unwrap();

    let model = parsed.models.get("nch").unwrap();
    assert_eq!(model.name, "nch");
    assert_eq!(model.kind, "NJF");
    assert_close(*model.params.get("BETA").unwrap(), 2.0e-3);
    assert_close(*model.params.get("VTO").unwrap(), -3.0);
    assert_close(*model.params.get("LAMBDA").unwrap(), 0.02);

    let Element::Jfet(jfet) = &parsed.circuit.elements()[0] else {
        panic!("expected JFET");
    };
    assert_eq!(jfet.name, "J1");
    assert_eq!(jfet.drain, "drain");
    assert_eq!(jfet.gate, "gate");
    assert_eq!(jfet.source, "source");
    assert_eq!(jfet.polarity, JfetPolarity::Njf);
    assert_close(jfet.beta, 2.0e-3);
    assert_close(jfet.threshold_voltage, -3.0);
    assert_close(jfet.channel_length_modulation, 0.02);
}

#[test]
fn parses_pjf_model_beta_aliases() {
    let parsed = parse_netlist(
        r#"
.model pch PJF(B=750u)
Jpull drain gate source pch
"#,
    )
    .unwrap();

    let Element::Jfet(jfet) = &parsed.circuit.elements()[0] else {
        panic!("expected JFET");
    };
    assert_eq!(jfet.polarity, JfetPolarity::Pjf);
    assert_close(jfet.beta, 750.0e-6);
    assert_close(jfet.threshold_voltage, 2.0);
}

#[test]
fn parses_mosfet_models_into_operating_point_circuits() {
    let parsed = parse_netlist(
        r#"
.model nch NMOS(VTO=0.45 KP=250u LAMBDA=0.02 GAMMA=0.3 PHI=0.8 W=2u L=180n NSUB=1.5 TNOM=300 CGSO=3p CGDO=4p CGBO=5p CBS=6p CBD=7p)
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
    assert_close(*model.params.get("CGSO").unwrap(), 3.0e-12);

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
    assert_close(mosfet.params.gate_source_overlap_capacitance, 3.0e-12);
    assert_close(mosfet.params.gate_drain_overlap_capacitance, 4.0e-12);
    assert_close(mosfet.params.gate_bulk_overlap_capacitance, 5.0e-12);
    assert_close(mosfet.params.source_bulk_capacitance, 6.0e-12);
    assert_close(mosfet.params.drain_bulk_capacitance, 7.0e-12);

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
.model clamp D(IS=1e-12 VT=25m N=2 BV=5 IBV=1u CJ0=3p TT=5n)
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
    assert_close(diode.emission_coefficient, 2.0);
    assert_close(diode.breakdown_voltage.unwrap(), 5.0);
    assert_close(diode.breakdown_current, 1.0e-6);
    assert_close(diode.junction_capacitance, 3.0e-12);
    assert_close(diode.transit_time, 5.0e-9);
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
fn expands_subcircuit_jfet_nodes_into_engine_elements() {
    let parsed = parse_netlist(
        r#"
.model nch NJF(BETA=1m)
.subckt source_follower d g s
Jdrive d g inner nch
Rtail inner s 100
.ends source_follower
Xbuf vdd in out source_follower
"#,
    )
    .unwrap();

    let Element::Jfet(jfet) = &parsed.circuit.elements()[0] else {
        panic!("expected JFET");
    };
    assert_eq!(jfet.name, "Xbuf.Jdrive");
    assert_eq!(jfet.drain, "vdd");
    assert_eq!(jfet.gate, "in");
    assert_eq!(jfet.source, "Xbuf.inner");
    assert_close(jfet.beta, 1.0e-3);
}

#[test]
fn expands_subcircuit_mutual_inductor_refs_into_engine_elements() {
    let parsed = parse_netlist(
        r#"
.subckt transformer p1 p2 s1 s2
Lpri p1 p2 10m
Lsec s1 s2 40m
Kcore Lpri Lsec 0.9
.ends transformer
Xtx in 0 out 0 transformer
"#,
    )
    .unwrap();

    let Element::MutualInductor(mutual) = &parsed.circuit.elements()[2] else {
        panic!("expected mutual inductor");
    };
    assert_eq!(mutual.name, "Xtx.Kcore");
    assert_eq!(mutual.primary, "Xtx.Lpri");
    assert_eq!(mutual.secondary, "Xtx.Lsec");
    assert_close(mutual.coupling, 0.9);
}

#[test]
fn expands_subcircuit_transmission_line_nodes_into_engine_elements() {
    let parsed = parse_netlist(
        r#"
.subckt delay in out
T1 in 0 out 0 Z0=75 TD=2n
.ends delay
Xdelay a b delay
"#,
    )
    .unwrap();

    let Element::TransmissionLine(line) = &parsed.circuit.elements()[0] else {
        panic!("expected transmission line");
    };
    assert_eq!(line.name, "Xdelay.T1");
    assert_eq!(line.n1, "a");
    assert_eq!(line.n2, "0");
    assert_eq!(line.n3, "b");
    assert_eq!(line.n4, "0");
    assert_close(line.characteristic_impedance_ohms, 75.0);
    assert_close(line.delay_seconds, 2.0e-9);
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

#[test]
fn berkeley_syntax_facade_preserves_logical_cards_tokens_and_spans() {
    let syntax = parse_berkeley_syntax(
        r#"
* RC low pass
V1 in 0 DC 1
R1 in out 1k ; inline comment
+ TC=1m
.op
.tran 1n 2n
.end
"#,
    );

    assert!(!syntax.has_errors(), "{:?}", syntax.diagnostics);
    assert_eq!(
        syntax.grammar,
        BerkeleyGrammarMetadata {
            name: BERKELEY_SPICE_GRAMMAR_NAME,
            version: BERKELEY_SPICE_GRAMMAR_VERSION,
            token_grammar: syntax.grammar.token_grammar,
            parser_grammar: syntax.grammar.parser_grammar,
        }
    );
    assert_eq!(syntax.title.as_deref(), Some("RC low pass"));
    assert_eq!(syntax.cards.len(), 5);

    let resistor = &syntax.cards[1];
    assert_eq!(resistor.kind, BerkeleyCardKind::Element);
    assert_eq!(resistor.head, "R1");
    assert_eq!(resistor.text, "R1 in out 1k TC=1m");
    assert_eq!(resistor.physical_lines, vec![4, 5]);
    assert_eq!(resistor.span.start_line, 4);
    assert_eq!(resistor.span.end_line, 5);
    assert_eq!(
        resistor
            .tokens
            .iter()
            .map(|token| (token.kind.as_str(), token.text.as_str()))
            .collect::<Vec<_>>(),
        vec![
            ("ATOM", "R1"),
            ("ATOM", "in"),
            ("ATOM", "out"),
            ("NUMBER", "1k"),
            ("ATOM", "TC"),
            ("EQUALS", "="),
            ("NUMBER", "1m"),
        ]
    );

    let inventory = syntax.analysis_inventory();
    assert_eq!(
        inventory
            .iter()
            .map(|entry| (entry.index, entry.analysis.as_str()))
            .collect::<Vec<_>>(),
        vec![(2, "op"), (3, "tran")]
    );
}

#[test]
fn berkeley_syntax_facade_reports_stable_diagnostics() {
    let syntax = parse_berkeley_syntax(
        r#"
+ orphan
V1 in 0 PULSE(0 1
.measure tran bad PARAM="unterminated
"#,
    );

    assert_eq!(
        syntax
            .diagnostics
            .iter()
            .map(|diagnostic| (diagnostic.code.as_str(), diagnostic.severity))
            .collect::<Vec<_>>(),
        vec![
            (
                "SPICE_SYNTAX_CONTINUATION_WITHOUT_CARD",
                BerkeleyDiagnosticSeverity::Error
            ),
            (
                "SPICE_SYNTAX_UNCLOSED_PAREN",
                BerkeleyDiagnosticSeverity::Error
            ),
            (
                "SPICE_SYNTAX_UNCLOSED_QUOTE",
                BerkeleyDiagnosticSeverity::Error
            ),
        ]
    );
}

#[test]
fn berkeley_app_facade_exposes_inventory_and_runs_source_order() {
    let app = parse_berkeley_app_deck(
        r#"
* divider
V1 in 0 DC 1
R1 in out 1k
R2 out 0 1k
.op
.dc V1 0 1 1
.end
"#,
    );

    assert!(!app.has_errors(), "{:?}", app.diagnostics);
    assert_eq!(
        app.analysis_inventory()
            .iter()
            .map(|entry| (entry.index, entry.directive.as_str()))
            .collect::<Vec<_>>(),
        vec![(3, ".op"), (4, ".dc")]
    );

    let results = app.run_source_order().unwrap();
    assert_eq!(
        results.iter().map(|result| result.kind).collect::<Vec<_>>(),
        vec![AnalysisKind::Op, AnalysisKind::Dc]
    );

    let selected = app.run_selected_analysis(4).unwrap().unwrap();
    assert_eq!(selected.kind, AnalysisKind::Dc);
}

#[test]
fn berkeley_app_facade_exports_card_indexed_result_artifacts() {
    let app = parse_berkeley_app_deck(
        r#"
* transient UI
V1 in 0 PULSE(0 1 0 1n 1n 1n 4n)
R1 in out
+ 1k
C1 out 0 1p
.tran 1n 3n
.print tran V(out)
.end
"#,
    );

    assert!(!app.has_errors(), "{:?}", app.diagnostics);
    assert!(app.canonical_source.contains("R1 in out 1k\n"));

    let execution = app.run_artifacts().unwrap();
    assert_eq!(execution.analyses.len(), 1);
    assert_eq!(execution.run_artifacts.len(), 1);
    assert!(execution.run_artifact_table.contains("Analysis\tDirective"));

    let tran = &execution.analyses[0];
    assert_eq!(tran.syntax_card_index, Some(3));
    assert_eq!(tran.directive, ".tran");
    assert_eq!(tran.analysis, "tran");
    assert_eq!(tran.span.unwrap().start_line, 7);
    assert!(tran.table_row_count >= 1);
    assert!(tran.table_columns.iter().any(|column| column == "Time"));
    assert!(tran.table_columns.iter().any(|column| column == "V(out)"));
    assert_eq!(tran.waveform_series_count, 1);
    assert_eq!(tran.waveform_series[0].name, "V(out)");
    assert_eq!(tran.waveform_series[0].x_column, "Time");
    assert_eq!(tran.waveform_series[0].y_column, "V(out)");
    assert_eq!(tran.waveform_series[0].point_count, tran.table_row_count);
    assert_eq!(tran.waveform_series[0].points[0].row_index, 0);
    assert!(tran
        .table_artifacts
        .iter()
        .any(|artifact| artifact.name == "result"));
    assert!(tran
        .table_artifacts
        .iter()
        .any(|artifact| artifact.name == "output-plan"));
    assert_eq!(tran.output_plan_artifacts[0].output_probes, vec!["V(out)"]);
    assert!(tran.run_artifacts[0]
        .tables
        .iter()
        .any(|table| table == "result"));

    let selected = app.run_selected_artifact(3).unwrap().unwrap();
    assert_eq!(selected.analysis, "tran");
    let selected_waveforms = app.run_selected_waveform_series(3).unwrap().unwrap();
    assert_eq!(selected_waveforms[0].name, "V(out)");
    assert!(app.run_selected_artifact(4).unwrap().is_none());
}

#[test]
fn berkeley_app_facade_exports_probe_grouped_waveform_series() {
    let app = parse_berkeley_app_deck(
        r#"
* ac UI
V1 in 0 DC 0 AC 1
R1 in out 1k
C1 out 0 1u
.ac dec 1 1k 1k
.print ac V(out)
.end
"#,
    );

    assert!(!app.has_errors(), "{:?}", app.diagnostics);
    let execution = app.run_artifacts().unwrap();
    let ac = &execution.analyses[0];

    assert_eq!(ac.syntax_card_index, Some(3));
    assert_eq!(ac.analysis, "ac");
    assert!(ac
        .waveform_series
        .iter()
        .any(|series| series.name == "V(out):Magnitude"
            && series.x_column == "Frequency"
            && series.group_column.as_deref() == Some("Probe")
            && series.group_value.as_deref() == Some("V(out)")
            && series.point_count == 1));
    assert!(execution
        .waveform_series
        .iter()
        .any(|series| series.name == "V(out):Phase"));
}

#[test]
fn berkeley_app_facade_exports_selected_session_state() {
    let app = parse_berkeley_app_deck(
        r#"
* transient session
V1 in 0 PULSE(0 1 0 1n 1n 1n 4n)
R1 in out 1k
C1 out 0 1p
.tran 1n 3n
.print tran V(out)
.end
"#,
    );

    let state = app.run_session_state(Some(3)).unwrap();

    assert!(state.parsed);
    assert!(state.execution_available);
    assert!(!state.has_errors, "{:?}", state.diagnostics);
    assert_eq!(state.title.as_deref(), Some("transient session"));
    assert_eq!(state.card_count, 6);
    assert_eq!(state.analysis_count, 1);
    assert_eq!(state.selected_syntax_card_index, Some(3));
    assert_eq!(state.source_fingerprint.len(), 16);
    assert!(state
        .source_fingerprint
        .chars()
        .all(|ch| ch.is_ascii_hexdigit()));
    assert_eq!(state.selected_waveform_series_count, Some(1));
    assert_eq!(state.selected_output_probes, vec!["V(out)"]);
    assert!(state
        .selected_table_columns
        .iter()
        .any(|column| column == "Time"));
    assert!(state
        .selected_table_columns
        .iter()
        .any(|column| column == "V(out)"));

    let selected = state.selected_analysis.unwrap();
    assert!(selected.selected);
    assert!(selected.runnable);
    assert!(selected.artifact_supported);
    assert!(selected.execution_available);
    assert_eq!(selected.directive, ".tran");
    assert_eq!(selected.analysis, "tran");
    assert_eq!(selected.waveform_series_count, Some(1));
    assert_eq!(selected.table_row_count, Some(3));
    assert_eq!(selected.output_probes, vec!["V(out)"]);
}

#[test]
fn berkeley_app_facade_exports_editor_controls_after_execution() {
    let app = parse_berkeley_app_deck(
        r#"
* transient editor controls
V1 in 0 PULSE(0 1 0 1n 1n 1n 4n)
R1 in out 1k
C1 out 0 1p
.tran 1n 3n
.print tran V(out)
.end
"#,
    );

    let controls = app.run_editor_controls(Some(3)).unwrap();

    assert!(controls.parsed);
    assert!(controls.execution_available);
    assert_eq!(controls.control_count, 1);
    assert_eq!(controls.selected_syntax_card_index, Some(3));
    assert_eq!(controls.source_fingerprint.len(), 16);

    let selected = controls.selected_control.as_ref().unwrap();
    assert_eq!(selected.syntax_card_index, 3);
    assert_eq!(selected.directive, ".tran");
    assert!(selected.selected);
    assert!(selected.runnable);
    assert!(selected.artifact_supported);
    assert!(selected.execution_available);
    assert!(selected.table_available);
    assert!(selected.waveform_available);
    assert_eq!(selected.action_count, 4);

    let action = |kind| {
        selected
            .actions
            .iter()
            .find(|action| action.kind == kind)
            .unwrap()
    };
    assert_eq!(
        action(BerkeleyAppEditorActionKind::SelectAnalysis).label,
        "Select .tran"
    );
    assert!(action(BerkeleyAppEditorActionKind::RunAnalysis).enabled);
    assert!(action(BerkeleyAppEditorActionKind::InspectTable).enabled);
    assert!(action(BerkeleyAppEditorActionKind::InspectWaveform).enabled);
}

#[test]
fn berkeley_app_facade_editor_controls_explain_disabled_actions() {
    let app = parse_berkeley_app_deck(
        r#"
* transient editor controls
V1 in 0 PULSE(0 1 0 1n 1n 1n 4n)
R1 in out 1k
C1 out 0 1p
.tran 1n 3n
.print tran V(out)
.end
"#,
    );

    let controls = app.editor_controls(Some(3));
    let selected = controls.selected_control.as_ref().unwrap();
    let table = selected
        .actions
        .iter()
        .find(|action| action.kind == BerkeleyAppEditorActionKind::InspectTable)
        .unwrap();
    let waveform = selected
        .actions
        .iter()
        .find(|action| action.kind == BerkeleyAppEditorActionKind::InspectWaveform)
        .unwrap();
    assert!(!table.enabled);
    assert_eq!(
        table.disabled_reason.as_deref(),
        Some("run deck artifacts to populate analysis table")
    );
    assert!(!waveform.enabled);
    assert_eq!(
        waveform.disabled_reason.as_deref(),
        Some("run deck artifacts to populate waveform series")
    );

    let blocked = parse_berkeley_app_deck(
        r#"
V1 in 0 DC 1
R1 in out
.op
.end
"#,
    );
    let controls = blocked.editor_controls(Some(2));
    let selected = controls.selected_control.as_ref().unwrap();
    let run = selected
        .actions
        .iter()
        .find(|action| action.kind == BerkeleyAppEditorActionKind::RunAnalysis)
        .unwrap();
    assert!(!run.enabled);
    assert!(run
        .disabled_reason
        .as_deref()
        .unwrap()
        .contains("Berkeley SPICE app deck:"));
}

#[test]
fn berkeley_app_facade_exports_editor_command_plan_after_execution() {
    let app = parse_berkeley_app_deck(
        r#"
* transient editor commands
V1 in 0 PULSE(0 1 0 1n 1n 1n 4n)
R1 in out 1k
C1 out 0 1p
.tran 1n 3n
.print tran V(out)
.end
"#,
    );

    let plan = app.run_editor_command_plan(Some(3)).unwrap();

    assert!(plan.parsed);
    assert!(plan.execution_available);
    assert_eq!(plan.selected_syntax_card_index, Some(3));
    assert_eq!(plan.command_count, 4);
    assert_eq!(
        plan.commands
            .iter()
            .map(|command| command.id.as_str())
            .collect::<Vec<_>>(),
        vec![
            "analysis.3.select",
            "analysis.3.run",
            "analysis.3.inspect-table",
            "analysis.3.inspect-waveform",
        ]
    );

    let waveform = plan
        .commands
        .iter()
        .find(|command| command.id == "analysis.3.inspect-waveform")
        .unwrap();
    assert_eq!(waveform.kind, BerkeleyAppEditorActionKind::InspectWaveform);
    assert_eq!(waveform.syntax_card_index, 3);
    assert_eq!(waveform.directive, ".tran");
    assert_eq!(waveform.analysis, "tran");
    assert_eq!(waveform.target, "analysis-waveform");
    assert_eq!(waveform.label, "Inspect .tran waveform");
    assert!(waveform.enabled);
    assert!(waveform.selected);
    assert_eq!(waveform.disabled_reason, None);
}

#[test]
fn berkeley_app_facade_editor_command_plan_preserves_disabled_reasons() {
    let app = parse_berkeley_app_deck(
        r#"
* transient editor commands
V1 in 0 PULSE(0 1 0 1n 1n 1n 4n)
R1 in out 1k
C1 out 0 1p
.tran 1n 3n
.print tran V(out)
.end
"#,
    );

    let plan = app.editor_command_plan(Some(3));
    let table = plan
        .commands
        .iter()
        .find(|command| command.id == "analysis.3.inspect-table")
        .unwrap();
    assert!(!table.enabled);
    assert_eq!(table.target, "analysis-table");
    assert_eq!(
        table.disabled_reason.as_deref(),
        Some("run deck artifacts to populate analysis table")
    );

    let blocked = parse_berkeley_app_deck(
        r#"
V1 in 0 DC 1
R1 in out
.op
.end
"#,
    );
    let plan = blocked.editor_command_plan(Some(2));
    let run = plan
        .commands
        .iter()
        .find(|command| command.id == "analysis.2.run")
        .unwrap();
    assert!(!run.enabled);
    assert_eq!(run.target, "analysis-runner");
    assert!(run
        .disabled_reason
        .as_deref()
        .unwrap()
        .contains("Berkeley SPICE app deck:"));
}

#[test]
fn berkeley_app_facade_restores_persisted_editor_state_after_execution() {
    let app = parse_berkeley_app_deck(
        r#"
* transient editor state
V1 in 0 PULSE(0 1 0 1n 1n 1n 4n)
R1 in out 1k
C1 out 0 1p
.tran 1n 3n
.print tran V(out)
.end
"#,
    );
    let requested = BerkeleyAppPersistedEditorState {
        selected_syntax_card_index: Some(3),
        active_command_id: Some("analysis.3.inspect-waveform".to_string()),
    };

    let snapshot = app.run_editor_state_snapshot(requested.clone()).unwrap();

    assert!(snapshot.parsed);
    assert!(snapshot.execution_available);
    assert_eq!(snapshot.requested_state, requested);
    assert_eq!(
        snapshot.resolved_state,
        BerkeleyAppPersistedEditorState {
            selected_syntax_card_index: Some(3),
            active_command_id: Some("analysis.3.inspect-waveform".to_string()),
        }
    );
    assert!(!snapshot.selection_stale);
    assert!(!snapshot.command_stale);
    assert_eq!(snapshot.command_plan.command_count, 4);

    let selected = snapshot.selected_control.as_ref().unwrap();
    assert_eq!(selected.directive, ".tran");
    assert!(selected.waveform_available);

    let active = snapshot.active_command.as_ref().unwrap();
    assert_eq!(active.id, "analysis.3.inspect-waveform");
    assert_eq!(active.target, "analysis-waveform");
    assert!(active.enabled);
    assert!(active.selected);
}

#[test]
fn berkeley_app_facade_repairs_stale_persisted_editor_state() {
    let app = parse_berkeley_app_deck(
        r#"
* transient editor state
V1 in 0 PULSE(0 1 0 1n 1n 1n 4n)
R1 in out 1k
C1 out 0 1p
.tran 1n 3n
.print tran V(out)
.end
"#,
    );

    let snapshot = app.editor_state_snapshot(BerkeleyAppPersistedEditorState {
        selected_syntax_card_index: Some(99),
        active_command_id: Some("analysis.99.run".to_string()),
    });

    assert_eq!(
        snapshot.resolved_state,
        BerkeleyAppPersistedEditorState {
            selected_syntax_card_index: Some(3),
            active_command_id: Some("analysis.3.select".to_string()),
        }
    );
    assert!(snapshot.selection_stale);
    assert!(snapshot.command_stale);
    assert_eq!(
        snapshot
            .selected_control
            .as_ref()
            .unwrap()
            .syntax_card_index,
        3
    );

    let active = snapshot.active_command.as_ref().unwrap();
    assert_eq!(active.kind, BerkeleyAppEditorActionKind::SelectAnalysis);
    assert_eq!(active.target, "analysis-selection");
    assert!(active.enabled);
    assert!(active.selected);
}

#[test]
fn berkeley_app_facade_exports_host_surface_panels_after_execution() {
    let app = parse_berkeley_app_deck(
        r#"
* transient host surface
V1 in 0 PULSE(0 1 0 1n 1n 1n 4n)
R1 in out 1k
C1 out 0 1p
.tran 1n 3n
.print tran V(out)
.end
"#,
    );

    let surface = app
        .run_host_surface(BerkeleyAppPersistedEditorState {
            selected_syntax_card_index: Some(3),
            active_command_id: Some("analysis.3.inspect-waveform".to_string()),
        })
        .unwrap();

    assert!(surface.parsed);
    assert!(surface.execution_available);
    assert_eq!(surface.panel_count, 5);
    assert_eq!(
        surface.active_panel.as_ref().unwrap().kind,
        BerkeleyAppHostPanelKind::Waveform
    );
    assert_eq!(
        surface
            .panels
            .iter()
            .map(|panel| panel.id.as_str())
            .collect::<Vec<_>>(),
        vec!["source", "diagnostics", "analysis", "table", "waveform"]
    );

    let waveform = surface
        .panels
        .iter()
        .find(|panel| panel.kind == BerkeleyAppHostPanelKind::Waveform)
        .unwrap();
    assert_eq!(waveform.target, "analysis-waveform");
    assert!(waveform.enabled);
    assert!(waveform.active);
    assert_eq!(waveform.disabled_reason, None);

    let table = surface
        .panels
        .iter()
        .find(|panel| panel.kind == BerkeleyAppHostPanelKind::Table)
        .unwrap();
    assert!(table.enabled);
    assert!(!table.active);
}

#[test]
fn berkeley_app_facade_exports_host_surface_wire_json_after_execution() {
    let app = parse_berkeley_app_deck(
        r#"
* transient host surface
V1 in 0 PULSE(0 1 0 1n 1n 1n 4n)
R1 in out 1k
C1 out 0 1p
.tran 1n 3n
.print tran V(out)
.end
"#,
    );

    let requested_state = BerkeleyAppPersistedEditorState {
        selected_syntax_card_index: Some(3),
        active_command_id: Some("analysis.3.inspect-waveform".to_string()),
    };
    let wire = app.run_host_surface_wire(requested_state.clone()).unwrap();

    assert_eq!(wire.schema_version, 1);
    assert!(wire.parsed);
    assert!(wire.execution_available);
    assert_eq!(wire.requested_selected_syntax_card_index, Some(3));
    assert_eq!(
        wire.requested_active_command_id.as_deref(),
        Some("analysis.3.inspect-waveform")
    );
    assert_eq!(wire.resolved_selected_syntax_card_index, Some(3));
    assert_eq!(
        wire.resolved_active_command_id.as_deref(),
        Some("analysis.3.inspect-waveform")
    );
    assert!(!wire.selection_stale);
    assert!(!wire.command_stale);
    assert_eq!(wire.panel_count, 5);
    assert_eq!(wire.active_panel_id.as_deref(), Some("waveform"));
    assert_eq!(
        wire.panels
            .iter()
            .map(|panel| panel.kind.as_str())
            .collect::<Vec<_>>(),
        vec!["source", "diagnostics", "analysis", "table", "waveform"]
    );

    let payload: serde_json::Value =
        serde_json::from_str(&app.run_host_surface_wire_json(requested_state).unwrap()).unwrap();
    assert_eq!(payload["schemaVersion"], 1);
    assert_eq!(payload["activePanelId"], "waveform");
    assert_eq!(payload["panels"][4]["kind"], "waveform");
    assert_eq!(payload["panels"][4]["target"], "analysis-waveform");
    assert_eq!(payload["panels"][4]["enabled"], true);
    assert_eq!(payload["panels"][4]["active"], true);
    assert!(payload["diagnostics"].as_array().unwrap().is_empty());
}

#[test]
fn berkeley_app_facade_exports_package_manifest_json() {
    let manifest = berkeley_app_package_manifest();

    assert_eq!(
        manifest.schema_version,
        BERKELEY_APP_PACKAGE_MANIFEST_SCHEMA_VERSION
    );
    assert_eq!(manifest.package_name, BERKELEY_APP_PACKAGE_NAME);
    assert_eq!(manifest.grammar_name, BERKELEY_SPICE_GRAMMAR_NAME);
    assert_eq!(manifest.grammar_version, BERKELEY_SPICE_GRAMMAR_VERSION);
    assert_eq!(
        manifest.host_surface_wire_schema_version,
        BERKELEY_APP_HOST_SURFACE_WIRE_SCHEMA_VERSION
    );
    assert_eq!(
        manifest.source_fingerprint_algorithm,
        BERKELEY_APP_SOURCE_FINGERPRINT_ALGORITHM
    );
    assert_eq!(
        manifest.host_panel_kinds,
        vec!["source", "diagnostics", "analysis", "table", "waveform"]
    );
    assert_eq!(
        manifest.command_targets,
        vec![
            "analysis-selection",
            "analysis-runner",
            "analysis-table",
            "analysis-waveform"
        ]
    );
    assert_eq!(
        manifest.runnable_analysis_directives,
        vec![".op", ".dc", ".ac", ".tran"]
    );
    assert!(manifest
        .artifact_capabilities
        .iter()
        .any(|capability| capability == "host-surface-wire-json"));

    let payload: serde_json::Value = serde_json::from_str(&berkeley_app_package_manifest_json())
        .expect("manifest JSON should parse");
    assert_eq!(payload["schemaVersion"], 1);
    assert_eq!(payload["packageName"], "berkeley-spice-mosaic-app");
    assert_eq!(payload["grammarName"], BERKELEY_SPICE_GRAMMAR_NAME);
    assert_eq!(
        payload["hostSurfaceWireSchemaVersion"],
        BERKELEY_APP_HOST_SURFACE_WIRE_SCHEMA_VERSION
    );
    assert_eq!(payload["hostPanelKinds"][4], "waveform");
    assert_eq!(payload["editorActionKinds"][1], "run-analysis");
    assert_eq!(payload["commandTargets"][3], "analysis-waveform");
    assert_eq!(payload["artifactAnalysisDirectives"][6], ".noise");
    assert!(payload["artifactCapabilities"]
        .as_array()
        .unwrap()
        .iter()
        .any(|capability| capability == "source-fingerprint"));
    assert!(payload["artifactCapabilities"]
        .as_array()
        .unwrap()
        .iter()
        .any(|capability| capability == "app-bootstrap-json"));
}

#[test]
fn berkeley_app_facade_exports_bootstrap_json_after_execution() {
    let app = parse_berkeley_app_deck(
        r#"
* transient bootstrap
V1 in 0 PULSE(0 1 0 1n 1n 1n 4n)
R1 in out 1k
C1 out 0 1p
.tran 1n 3n
.print tran V(out)
.end
"#,
    );

    let requested_state = BerkeleyAppPersistedEditorState {
        selected_syntax_card_index: Some(3),
        active_command_id: Some("analysis.3.inspect-waveform".to_string()),
    };
    let snapshot = app
        .run_app_bootstrap_snapshot(requested_state.clone())
        .expect("bootstrap snapshot should execute");

    assert_eq!(
        snapshot.schema_version,
        BERKELEY_APP_BOOTSTRAP_SCHEMA_VERSION
    );
    assert_eq!(
        snapshot.package_manifest.package_name,
        BERKELEY_APP_PACKAGE_NAME
    );
    assert_eq!(
        snapshot.package_manifest.host_surface_wire_schema_version,
        BERKELEY_APP_HOST_SURFACE_WIRE_SCHEMA_VERSION
    );
    assert_eq!(
        snapshot.host_surface.active_panel_id.as_deref(),
        Some("waveform")
    );
    assert!(snapshot.host_surface.execution_available);
    assert_eq!(snapshot.host_surface.panel_count, 5);

    let payload: serde_json::Value =
        serde_json::from_str(&app.run_app_bootstrap_json(requested_state).unwrap())
            .expect("bootstrap JSON should parse");
    assert_eq!(payload["schemaVersion"], 1);
    assert_eq!(
        payload["packageManifest"]["packageName"],
        "berkeley-spice-mosaic-app"
    );
    assert_eq!(
        payload["packageManifest"]["hostSurfaceWireSchemaVersion"],
        BERKELEY_APP_HOST_SURFACE_WIRE_SCHEMA_VERSION
    );
    assert_eq!(payload["hostSurface"]["activePanelId"], "waveform");
    assert_eq!(
        payload["hostSurface"]["panels"][4]["target"],
        "analysis-waveform"
    );
    assert_eq!(payload["hostSurface"]["panels"][4]["enabled"], true);
    assert!(payload["packageManifest"]["artifactCapabilities"]
        .as_array()
        .unwrap()
        .iter()
        .any(|capability| capability == "app-bootstrap-json"));
}

#[test]
fn berkeley_app_facade_bootstrap_json_preserves_blocked_deck_diagnostics() {
    let app = parse_berkeley_app_deck(
        r#"
V1 in 0 DC 1
R1 in out
.op
.end
"#,
    );

    let requested_state = BerkeleyAppPersistedEditorState {
        selected_syntax_card_index: Some(2),
        active_command_id: Some("analysis.2.run".to_string()),
    };
    let snapshot = app.app_bootstrap_snapshot(requested_state.clone());

    assert_eq!(
        snapshot.schema_version,
        BERKELEY_APP_BOOTSTRAP_SCHEMA_VERSION
    );
    assert!(!snapshot.host_surface.parsed);
    assert_eq!(
        snapshot.host_surface.active_panel_id.as_deref(),
        Some("diagnostics")
    );
    assert_eq!(snapshot.host_surface.diagnostics[0].severity, "error");

    let payload: serde_json::Value = serde_json::from_str(&app.app_bootstrap_json(requested_state))
        .expect("blocked bootstrap JSON should parse");
    assert_eq!(payload["packageManifest"]["schemaVersion"], 1);
    assert_eq!(payload["hostSurface"]["activePanelId"], "diagnostics");
    assert_eq!(
        payload["hostSurface"]["diagnostics"][0]["severity"],
        "error"
    );
    assert_eq!(
        payload["hostSurface"]["diagnostics"][0]["span"]["startLine"],
        3
    );
}

#[test]
fn berkeley_app_facade_host_surface_routes_blocked_decks_to_diagnostics() {
    let app = parse_berkeley_app_deck(
        r#"
V1 in 0 DC 1
R1 in out
.op
.end
"#,
    );

    let surface = app.host_surface(BerkeleyAppPersistedEditorState {
        selected_syntax_card_index: Some(2),
        active_command_id: Some("analysis.2.run".to_string()),
    });

    assert!(!surface.parsed);
    assert!(!surface.execution_available);
    assert_eq!(
        surface.active_panel.as_ref().unwrap().kind,
        BerkeleyAppHostPanelKind::Diagnostics
    );

    let diagnostics = surface
        .panels
        .iter()
        .find(|panel| panel.kind == BerkeleyAppHostPanelKind::Diagnostics)
        .unwrap();
    assert!(diagnostics.enabled);
    assert!(diagnostics.active);
    assert_eq!(diagnostics.disabled_reason, None);

    let table = surface
        .panels
        .iter()
        .find(|panel| panel.kind == BerkeleyAppHostPanelKind::Table)
        .unwrap();
    assert!(!table.enabled);
    assert!(table
        .disabled_reason
        .as_deref()
        .unwrap()
        .contains("Berkeley SPICE app deck:"));
}

#[test]
fn berkeley_app_facade_host_surface_wire_json_preserves_blocked_diagnostics() {
    let app = parse_berkeley_app_deck(
        r#"
V1 in 0 DC 1
R1 in out
.op
.end
"#,
    );

    let requested_state = BerkeleyAppPersistedEditorState {
        selected_syntax_card_index: Some(2),
        active_command_id: Some("analysis.2.run".to_string()),
    };
    let wire = app.host_surface_wire(requested_state.clone());

    assert!(!wire.parsed);
    assert!(!wire.execution_available);
    assert_eq!(wire.active_panel_id.as_deref(), Some("diagnostics"));
    assert_eq!(
        wire.resolved_active_command_id.as_deref(),
        Some("analysis.2.run")
    );
    assert_eq!(wire.diagnostics[0].severity, "error");
    assert_eq!(wire.diagnostics[0].span.as_ref().unwrap().start_line, 3);
    assert!(wire
        .blocking_message
        .as_deref()
        .unwrap()
        .contains("Berkeley SPICE app deck:"));

    let payload: serde_json::Value =
        serde_json::from_str(&app.host_surface_wire_json(requested_state)).unwrap();
    assert_eq!(payload["activePanelId"], "diagnostics");
    assert_eq!(payload["diagnostics"][0]["severity"], "error");
    assert_eq!(payload["diagnostics"][0]["span"]["startLine"], 3);
    assert!(payload["panels"][3]["disabledReason"]
        .as_str()
        .unwrap()
        .contains("Berkeley SPICE app deck:"));
}

#[test]
fn berkeley_app_facade_session_state_reports_blocked_decks() {
    let app = parse_berkeley_app_deck(
        r#"
V1 in 0 DC 1
R1 in out
.op
.end
"#,
    );

    let state = app.run_session_state(Some(2)).unwrap();

    assert!(!state.parsed);
    assert!(!state.execution_available);
    assert!(state.has_errors);
    assert_eq!(state.analysis_count, 1);
    assert_eq!(state.selected_syntax_card_index, Some(2));
    assert!(state
        .blocking_message
        .as_deref()
        .unwrap()
        .contains("Berkeley SPICE app deck:"));
    assert_eq!(state.diagnostics[0].code, "SPICE_BERKELEY_LOWERING_ERROR");

    let selected = state.selected_analysis.unwrap();
    assert!(selected.selected);
    assert!(selected.runnable);
    assert!(selected.artifact_supported);
    assert!(!selected.execution_available);
    assert_eq!(selected.directive, ".op");
    assert_eq!(selected.table_row_count, None);
    assert_eq!(selected.waveform_series_count, None);
}

#[test]
fn berkeley_app_facade_reports_lowering_errors_without_running() {
    let app = parse_berkeley_app_deck(
        r#"
* invalid semantic deck
V1 in 0 DC 1
Rbad in
.op
"#,
    );

    assert!(app.has_errors());
    assert_eq!(app.parsed, None);
    assert!(app
        .diagnostics
        .iter()
        .any(|diagnostic| diagnostic.code == "SPICE_BERKELEY_LOWERING_ERROR"));
    let err = app.run_source_order().unwrap_err();
    assert!(err.to_string().contains("Berkeley SPICE app deck"));
}

#[test]
fn parse_netlist_lowers_berkeley_logical_card_continuations() {
    let parsed = parse_netlist(
        r#"
* continued divider
V1 in 0 DC 10
R1 in mid
+ 1k
R2 mid 0 1k
.op
.end
"#,
    )
    .unwrap();

    assert_eq!(parsed.title.as_deref(), Some("continued divider"));
    let result = dc_op(&parsed.circuit).unwrap();
    assert_close(result.voltage("mid").unwrap(), 5.0);
}

#[test]
fn parse_netlist_reports_berkeley_syntax_diagnostics_before_lowering() {
    let err = parse_netlist(
        r#"
+ orphan
V1 in 0 DC 1
"#,
    )
    .unwrap_err();

    assert!(err
        .to_string()
        .contains("SPICE_SYNTAX_CONTINUATION_WITHOUT_CARD"));
}

fn assert_error_type(_: NetlistParseError) {}
