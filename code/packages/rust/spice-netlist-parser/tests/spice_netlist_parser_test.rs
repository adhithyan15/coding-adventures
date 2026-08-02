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
    BERKELEY_APP_HOST_SURFACE_WIRE_SCHEMA_VERSION, BERKELEY_APP_LAUNCH_PLAN_SCHEMA_VERSION,
    BERKELEY_APP_PACKAGE_MANIFEST_SCHEMA_VERSION, BERKELEY_APP_PACKAGE_NAME,
    BERKELEY_APP_READINESS_REPORT_SCHEMA_VERSION,
    BERKELEY_APP_SHELL_DASHBOARD_ACTION_DISPATCH_SCHEMA_VERSION,
    BERKELEY_APP_SHELL_DASHBOARD_BREADCRUMBS_SCHEMA_VERSION,
    BERKELEY_APP_SHELL_DASHBOARD_CARDS_SCHEMA_VERSION,
    BERKELEY_APP_SHELL_DASHBOARD_DISPATCH_EVENTS_SCHEMA_VERSION,
    BERKELEY_APP_SHELL_DASHBOARD_DISPATCH_QUEUE_DIGEST_SCHEMA_VERSION,
    BERKELEY_APP_SHELL_DASHBOARD_DISPATCH_QUEUE_LANES_SCHEMA_VERSION,
    BERKELEY_APP_SHELL_DASHBOARD_DISPATCH_QUEUE_LANE_TABS_SCHEMA_VERSION,
    BERKELEY_APP_SHELL_DASHBOARD_DISPATCH_QUEUE_LANE_TAB_PANELS_SCHEMA_VERSION,
    BERKELEY_APP_SHELL_DASHBOARD_DISPATCH_QUEUE_LANE_TAB_PANEL_CARDS_SCHEMA_VERSION,
    BERKELEY_APP_SHELL_DASHBOARD_DISPATCH_QUEUE_LANE_TAB_PANEL_CARD_ACTIONS_SCHEMA_VERSION,
    BERKELEY_APP_SHELL_DASHBOARD_DISPATCH_QUEUE_LANE_TAB_PANEL_CARD_ACTION_MENU_GROUPS_SCHEMA_VERSION,
    BERKELEY_APP_SHELL_DASHBOARD_DISPATCH_QUEUE_LANE_TAB_PANEL_CARD_ACTION_MENU_GROUP_SHORTCUTS_SCHEMA_VERSION,
    BERKELEY_APP_SHELL_DASHBOARD_DISPATCH_QUEUE_LANE_TAB_PANEL_CARD_ACTION_MENU_GROUP_SHORTCUT_BINDINGS_SCHEMA_VERSION,
    BERKELEY_APP_SHELL_DASHBOARD_DISPATCH_QUEUE_LANE_TAB_PANEL_CARD_ACTION_MENU_GROUP_SHORTCUT_COMMAND_PALETTE_SCHEMA_VERSION,
    BERKELEY_APP_SHELL_DASHBOARD_DISPATCH_QUEUE_LANE_TAB_PANEL_CARD_ACTION_MENU_GROUP_SHORTCUT_COMMAND_PALETTE_SEARCH_INDEX_SCHEMA_VERSION,
    BERKELEY_APP_SHELL_DASHBOARD_DISPATCH_QUEUE_LANE_TAB_PANEL_CARD_ACTION_MENU_GROUP_SHORTCUT_COMMAND_PALETTE_SEARCH_INVOCATION_RECEIPTS_SCHEMA_VERSION,
    BERKELEY_APP_SHELL_DASHBOARD_DISPATCH_QUEUE_LANE_TAB_PANEL_CARD_ACTION_MENU_GROUP_SHORTCUT_COMMAND_PALETTE_SEARCH_INVOCATION_RECEIPT_NOTIFICATION_SCHEMA_VERSION,
    BERKELEY_APP_SHELL_DASHBOARD_DISPATCH_QUEUE_LANE_TAB_PANEL_CARD_ACTION_MENU_GROUP_SHORTCUT_COMMAND_PALETTE_SEARCH_INVOCATION_RECEIPT_NOTIFICATION_STACK_SCHEMA_VERSION,
    BERKELEY_APP_SHELL_DASHBOARD_DISPATCH_QUEUE_LANE_TAB_PANEL_CARD_ACTION_MENU_GROUP_SHORTCUT_COMMAND_PALETTE_SEARCH_INVOCATION_RECEIPT_NOTIFICATION_STACK_SUMMARY_PRODUCT_HANDOFF_DELIVERY_PACKAGE_EMBED_LOADER_PLAN_SCHEMA_VERSION,
    BERKELEY_APP_SHELL_DASHBOARD_DISPATCH_QUEUE_LANE_TAB_PANEL_CARD_ACTION_MENU_GROUP_SHORTCUT_COMMAND_PALETTE_SEARCH_INVOCATION_RECEIPT_NOTIFICATION_STACK_SUMMARY_PRODUCT_HANDOFF_DELIVERY_PACKAGE_EMBED_MANIFEST_SCHEMA_VERSION,
    BERKELEY_APP_SHELL_DASHBOARD_DISPATCH_QUEUE_LANE_TAB_PANEL_CARD_ACTION_MENU_GROUP_SHORTCUT_COMMAND_PALETTE_SEARCH_INVOCATION_RECEIPT_NOTIFICATION_STACK_SUMMARY_PRODUCT_HANDOFF_DELIVERY_PACKAGE_EMBED_RUNTIME_ACTIVATION_PLAN_SCHEMA_VERSION,
    BERKELEY_APP_SHELL_DASHBOARD_DISPATCH_QUEUE_LANE_TAB_PANEL_CARD_ACTION_MENU_GROUP_SHORTCUT_COMMAND_PALETTE_SEARCH_INVOCATION_RECEIPT_NOTIFICATION_STACK_SUMMARY_PRODUCT_HANDOFF_DELIVERY_PACKAGE_EMBED_RUNTIME_ACTIVATION_RECEIPT_JOURNAL_SCHEMA_VERSION,
    BERKELEY_APP_SHELL_DASHBOARD_DISPATCH_QUEUE_LANE_TAB_PANEL_CARD_ACTION_MENU_GROUP_SHORTCUT_COMMAND_PALETTE_SEARCH_INVOCATION_RECEIPT_NOTIFICATION_STACK_SUMMARY_PRODUCT_HANDOFF_DELIVERY_PACKAGE_EMBED_RUNTIME_ACTIVATION_RECEIPT_JOURNAL_SUMMARY_HANDOFF_RECEIPT_ACKNOWLEDGEMENT_RECORD_RECEIPT_ACKNOWLEDGEMENT_RECORD_SCHEMA_VERSION,
    BERKELEY_APP_SHELL_DASHBOARD_DISPATCH_QUEUE_LANE_TAB_PANEL_CARD_ACTION_MENU_GROUP_SHORTCUT_COMMAND_PALETTE_SEARCH_INVOCATION_RECEIPT_NOTIFICATION_STACK_SUMMARY_PRODUCT_HANDOFF_DELIVERY_PACKAGE_EMBED_RUNTIME_ACTIVATION_RECEIPT_JOURNAL_SUMMARY_HANDOFF_RECEIPT_ACKNOWLEDGEMENT_RECORD_RECEIPT_ACKNOWLEDGEMENT_RECORD_SUMMARY_DIGEST_SCHEMA_VERSION,
    BERKELEY_APP_SHELL_DASHBOARD_DISPATCH_QUEUE_LANE_TAB_PANEL_CARD_ACTION_MENU_GROUP_SHORTCUT_COMMAND_PALETTE_SEARCH_INVOCATION_RECEIPT_NOTIFICATION_STACK_SUMMARY_PRODUCT_HANDOFF_DELIVERY_PACKAGE_EMBED_RUNTIME_ACTIVATION_RECEIPT_JOURNAL_SUMMARY_HANDOFF_RECEIPT_ACKNOWLEDGEMENT_RECORD_RECEIPT_ACKNOWLEDGEMENT_RECORD_SUMMARY_SCHEMA_VERSION,
    BERKELEY_APP_SHELL_DASHBOARD_DISPATCH_QUEUE_LANE_TAB_PANEL_CARD_ACTION_MENU_GROUP_SHORTCUT_COMMAND_PALETTE_SEARCH_INVOCATION_RECEIPT_NOTIFICATION_STACK_SUMMARY_PRODUCT_HANDOFF_DELIVERY_PACKAGE_EMBED_RUNTIME_ACTIVATION_RECEIPT_JOURNAL_SUMMARY_HANDOFF_RECEIPT_ACKNOWLEDGEMENT_RECORD_RECEIPT_ACKNOWLEDGEMENT_SCHEMA_VERSION,
    BERKELEY_APP_SHELL_DASHBOARD_DISPATCH_QUEUE_LANE_TAB_PANEL_CARD_ACTION_MENU_GROUP_SHORTCUT_COMMAND_PALETTE_SEARCH_INVOCATION_RECEIPT_NOTIFICATION_STACK_SUMMARY_PRODUCT_HANDOFF_DELIVERY_PACKAGE_EMBED_RUNTIME_ACTIVATION_RECEIPT_JOURNAL_SUMMARY_HANDOFF_RECEIPT_ACKNOWLEDGEMENT_RECORD_RECEIPT_SCHEMA_VERSION,
    BERKELEY_APP_SHELL_DASHBOARD_DISPATCH_QUEUE_LANE_TAB_PANEL_CARD_ACTION_MENU_GROUP_SHORTCUT_COMMAND_PALETTE_SEARCH_INVOCATION_RECEIPT_NOTIFICATION_STACK_SUMMARY_PRODUCT_HANDOFF_DELIVERY_PACKAGE_EMBED_RUNTIME_ACTIVATION_RECEIPT_JOURNAL_SUMMARY_HANDOFF_RECEIPT_ACKNOWLEDGEMENT_RECORD_SCHEMA_VERSION,
    BERKELEY_APP_SHELL_DASHBOARD_DISPATCH_QUEUE_LANE_TAB_PANEL_CARD_ACTION_MENU_GROUP_SHORTCUT_COMMAND_PALETTE_SEARCH_INVOCATION_RECEIPT_NOTIFICATION_STACK_SUMMARY_PRODUCT_HANDOFF_DELIVERY_PACKAGE_EMBED_RUNTIME_ACTIVATION_RECEIPT_JOURNAL_SUMMARY_HANDOFF_RECEIPT_ACKNOWLEDGEMENT_SCHEMA_VERSION,
    BERKELEY_APP_SHELL_DASHBOARD_DISPATCH_QUEUE_LANE_TAB_PANEL_CARD_ACTION_MENU_GROUP_SHORTCUT_COMMAND_PALETTE_SEARCH_INVOCATION_RECEIPT_NOTIFICATION_STACK_SUMMARY_PRODUCT_HANDOFF_DELIVERY_PACKAGE_EMBED_RUNTIME_ACTIVATION_RECEIPT_JOURNAL_SUMMARY_HANDOFF_RECEIPT_SCHEMA_VERSION,
    BERKELEY_APP_SHELL_DASHBOARD_DISPATCH_QUEUE_LANE_TAB_PANEL_CARD_ACTION_MENU_GROUP_SHORTCUT_COMMAND_PALETTE_SEARCH_INVOCATION_RECEIPT_NOTIFICATION_STACK_SUMMARY_PRODUCT_HANDOFF_DELIVERY_PACKAGE_EMBED_RUNTIME_ACTIVATION_RECEIPT_JOURNAL_SUMMARY_HANDOFF_SCHEMA_VERSION,
    BERKELEY_APP_SHELL_DASHBOARD_DISPATCH_QUEUE_LANE_TAB_PANEL_CARD_ACTION_MENU_GROUP_SHORTCUT_COMMAND_PALETTE_SEARCH_INVOCATION_RECEIPT_NOTIFICATION_STACK_SUMMARY_PRODUCT_HANDOFF_DELIVERY_PACKAGE_EMBED_RUNTIME_ACTIVATION_RECEIPT_JOURNAL_SUMMARY_SCHEMA_VERSION,
    BERKELEY_APP_SHELL_DASHBOARD_DISPATCH_QUEUE_LANE_TAB_PANEL_CARD_ACTION_MENU_GROUP_SHORTCUT_COMMAND_PALETTE_SEARCH_INVOCATION_RECEIPT_NOTIFICATION_STACK_SUMMARY_PRODUCT_HANDOFF_DELIVERY_PACKAGE_EMBED_RUNTIME_ACTIVATION_RECEIPT_SCHEMA_VERSION,
    BERKELEY_APP_SHELL_DASHBOARD_DISPATCH_QUEUE_LANE_TAB_PANEL_CARD_ACTION_MENU_GROUP_SHORTCUT_COMMAND_PALETTE_SEARCH_INVOCATION_RECEIPT_NOTIFICATION_STACK_SUMMARY_PRODUCT_HANDOFF_DELIVERY_PACKAGE_EMBED_RUNTIME_PLAN_SCHEMA_VERSION,
    BERKELEY_APP_SHELL_DASHBOARD_DISPATCH_QUEUE_LANE_TAB_PANEL_CARD_ACTION_MENU_GROUP_SHORTCUT_COMMAND_PALETTE_SEARCH_INVOCATION_RECEIPT_NOTIFICATION_STACK_SUMMARY_PRODUCT_HANDOFF_DELIVERY_PACKAGE_EMBED_RUNTIME_SESSION_PLAN_SCHEMA_VERSION,
    BERKELEY_APP_SHELL_DASHBOARD_DISPATCH_QUEUE_LANE_TAB_PANEL_CARD_ACTION_MENU_GROUP_SHORTCUT_COMMAND_PALETTE_SEARCH_INVOCATION_RECEIPT_NOTIFICATION_STACK_SUMMARY_PRODUCT_HANDOFF_DELIVERY_PACKAGE_SCHEMA_VERSION,
    BERKELEY_APP_SHELL_DASHBOARD_DISPATCH_QUEUE_LANE_TAB_PANEL_CARD_ACTION_MENU_GROUP_SHORTCUT_COMMAND_PALETTE_SEARCH_INVOCATION_RECEIPT_NOTIFICATION_STACK_SUMMARY_PRODUCT_HANDOFF_SCHEMA_VERSION,
    BERKELEY_APP_SHELL_DASHBOARD_DISPATCH_QUEUE_LANE_TAB_PANEL_CARD_ACTION_MENU_GROUP_SHORTCUT_COMMAND_PALETTE_SEARCH_INVOCATION_RECEIPT_NOTIFICATION_STACK_SUMMARY_SCHEMA_VERSION,
    BERKELEY_APP_SHELL_DASHBOARD_DISPATCH_QUEUE_LANE_TAB_PANEL_CARD_ACTION_MENU_GROUP_SHORTCUT_COMMAND_PALETTE_SEARCH_INVOCATION_RECEIPT_SUMMARY_SCHEMA_VERSION,
    BERKELEY_APP_SHELL_DASHBOARD_DISPATCH_QUEUE_LANE_TAB_PANEL_CARD_ACTION_MENU_GROUP_SHORTCUT_COMMAND_PALETTE_SEARCH_INVOCATION_SCHEMA_VERSION,
    BERKELEY_APP_SHELL_DASHBOARD_DISPATCH_QUEUE_LANE_TAB_PANEL_CARD_ACTION_MENU_GROUP_SHORTCUT_COMMAND_PALETTE_SEARCH_RESULTS_SCHEMA_VERSION,
    BERKELEY_APP_SHELL_DASHBOARD_DISPATCH_QUEUE_LANE_TAB_PANEL_CARD_ACTION_MENU_GROUP_SHORTCUT_COMMAND_PALETTE_SEARCH_SELECTION_SCHEMA_VERSION,
    BERKELEY_APP_SHELL_DASHBOARD_DISPATCH_QUEUE_LANE_TAB_PANEL_CARD_ACTION_MENU_GROUP_SHORTCUT_COMMAND_REGISTRY_SCHEMA_VERSION,
    BERKELEY_APP_SHELL_DASHBOARD_DISPATCH_QUEUE_LANE_TAB_PANEL_CARD_ACTION_MENU_SCHEMA_VERSION,
    BERKELEY_APP_SHELL_DASHBOARD_DISPATCH_QUEUE_SCHEMA_VERSION,
    BERKELEY_APP_SHELL_DASHBOARD_DISPATCH_QUEUE_SUMMARY_SCHEMA_VERSION,
    BERKELEY_APP_SHELL_DASHBOARD_LAYOUT_SCHEMA_VERSION,
    BERKELEY_APP_SHELL_DASHBOARD_NAVIGATION_SCHEMA_VERSION,
    BERKELEY_APP_SHELL_DASHBOARD_PACKAGE_SCHEMA_VERSION,
    BERKELEY_APP_SHELL_DASHBOARD_PANEL_CARDS_SCHEMA_VERSION,
    BERKELEY_APP_SHELL_DASHBOARD_PANEL_CARD_ACTIONS_SCHEMA_VERSION,
    BERKELEY_APP_SHELL_DASHBOARD_ROUTES_SCHEMA_VERSION,
    BERKELEY_APP_SHELL_DASHBOARD_TABS_SCHEMA_VERSION,
    BERKELEY_APP_SHELL_DASHBOARD_TAB_PANELS_SCHEMA_VERSION,
    BERKELEY_APP_SHELL_DASHBOARD_VIEW_SCHEMA_VERSION,
    BERKELEY_APP_SHELL_EVENT_DASHBOARD_SCHEMA_VERSION,
    BERKELEY_APP_SHELL_EVENT_DIGEST_SCHEMA_VERSION, BERKELEY_APP_SHELL_EVENT_LOG_SCHEMA_VERSION,
    BERKELEY_APP_SHELL_EVENT_SUMMARY_SCHEMA_VERSION, BERKELEY_APP_SHELL_HANDOFF_SCHEMA_VERSION,
    BERKELEY_APP_SHELL_STATUS_SCHEMA_VERSION, BERKELEY_APP_SHELL_TELEMETRY_SCHEMA_VERSION,
    BERKELEY_APP_SOURCE_FINGERPRINT_ALGORITHM, BERKELEY_APP_STARTUP_SUMMARY_SCHEMA_VERSION,
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
fn rejects_unsupported_mosfet_element_params() {
    let error = parse_netlist(".model nch NMOS\nM1 d g s b nch WIDTH=1u").unwrap_err();

    assert!(error.to_string().contains("unsupported MOSFET parameter"));
}

#[test]
fn rejects_invalid_mosfet_instance_widths() {
    for width in ["0", "-1u", "1e999"] {
        let error =
            parse_netlist(&format!(".model nch NMOS\nM1 d g s b nch W={width}")).unwrap_err();

        assert!(error
            .to_string()
            .contains("MOSFET W must be finite and positive"));
    }
}

#[test]
fn rejects_invalid_mosfet_instance_lengths() {
    for length in ["0", "-1u", "1e999"] {
        let error =
            parse_netlist(&format!(".model nch NMOS\nM1 d g s b nch L={length}")).unwrap_err();

        assert!(error
            .to_string()
            .contains("MOSFET L must be finite and positive"));
    }
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
.model nch NMOS(VTO=0.45 KP=250u UO=500 LAMBDA=0.02 GAMMA=0.3 PHI=0.8 W=2u L=180n LD=10n TOX=20n RD=10 RS=20 RSH=250 IS=4p JS=2m NSUB=1.5 TNOM=300 CGSO=3p CGDO=4p CGBO=5p CBS=6p CBD=7p CJ=2m CJSW=5u PB=0.9 MJ=0.45 MJSW=0.25 FC=0.4 KF=1p AF=1.2)
Vdd vdd 0 DC 5
Vgate gate 0 DC 2.5
M1 vdd gate out 0 nch W=4u L=200n NRD=2 NRS=3 AD=3n AS=4n PD=6u PS=7u
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
    assert_close(mosfet.params.surface_mobility, 500.0);
    assert_close(mosfet.params.lambda, 0.02);
    assert_close(mosfet.params.gamma, 0.3);
    assert_close(mosfet.params.phi, 0.8);
    assert_close(mosfet.params.w, 4.0e-6);
    assert_close(mosfet.params.l, 200.0e-9);
    assert_close(mosfet.params.lateral_diffusion_length, 10.0e-9);
    assert_close(mosfet.params.oxide_thickness, 20.0e-9);
    assert_close(mosfet.params.drain_resistance, 10.0);
    assert_close(mosfet.params.source_resistance, 20.0);
    assert_close(mosfet.params.sheet_resistance, 250.0);
    assert_close(mosfet.params.saturation_current, 4.0e-12);
    assert_close(mosfet.params.saturation_current_density, 2.0e-3);
    assert_close(mosfet.params.drain_squares, 2.0);
    assert_close(mosfet.params.source_squares, 3.0);
    assert_close(mosfet.params.drain_area, 3.0e-9);
    assert_close(mosfet.params.source_area, 4.0e-9);
    assert_close(mosfet.params.drain_perimeter, 6.0e-6);
    assert_close(mosfet.params.source_perimeter, 7.0e-6);
    assert_close(mosfet.params.bottom_junction_capacitance, 2.0e-3);
    assert_close(mosfet.params.sidewall_junction_capacitance, 5.0e-6);
    assert_close(mosfet.params.bulk_junction_potential, 0.9);
    assert_close(mosfet.params.bulk_junction_grading_coefficient, 0.45);
    assert_close(mosfet.params.sidewall_junction_grading_coefficient, 0.25);
    assert_close(mosfet.params.forward_bias_depletion_coefficient, 0.4);
    assert_close(mosfet.params.flicker_noise_coefficient, 1.0e-12);
    assert_close(mosfet.params.flicker_noise_exponent, 1.2);
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
fn derives_mosfet_transconductance_from_surface_mobility_and_oxide_thickness() {
    let parsed = parse_netlist(".model mobile NMOS(TOX=100n UO=500)\nM1 d g s b mobile\n").unwrap();

    let Element::Mosfet(mosfet) = &parsed.circuit.elements()[0] else {
        panic!("expected MOSFET");
    };
    assert_close(mosfet.params.surface_mobility, 500.0);
    assert_close(mosfet.params.kp, 500.0 * 1.0e-4 * 3.453_133e-11 / 100.0e-9);
}

#[test]
fn parses_pmos_mosfet_model_cards() {
    let parsed = parse_netlist(
        r#"
.model pch PMOS(VTH=-0.5 KP=90u LAM=0.03 W=3u L=180n CJS=2p CJD=3p)
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
    assert_close(mosfet.params.lambda, 0.03);
    assert_close(mosfet.params.w, 3.0e-6);
    assert_close(mosfet.params.source_bulk_capacitance, 2.0e-12);
    assert_close(mosfet.params.drain_bulk_capacitance, 3.0e-12);
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
    assert!(payload["artifactCapabilities"]
        .as_array()
        .unwrap()
        .iter()
        .any(|capability| capability == "app-startup-summary-json"));
    assert!(payload["artifactCapabilities"]
        .as_array()
        .unwrap()
        .iter()
        .any(|capability| capability == "app-launch-plan-json"));
    assert!(payload["artifactCapabilities"]
        .as_array()
        .unwrap()
        .iter()
        .any(|capability| capability == "app-readiness-report-json"));
    assert!(payload["artifactCapabilities"]
        .as_array()
        .unwrap()
        .iter()
        .any(|capability| capability == "app-shell-handoff-json"));
    assert!(payload["artifactCapabilities"]
        .as_array()
        .unwrap()
        .iter()
        .any(|capability| capability == "app-shell-status-json"));
    assert!(payload["artifactCapabilities"]
        .as_array()
        .unwrap()
        .iter()
        .any(|capability| capability == "app-shell-telemetry-json"));
    assert!(payload["artifactCapabilities"]
        .as_array()
        .unwrap()
        .iter()
        .any(|capability| capability == "app-shell-events-json"));
    assert!(payload["artifactCapabilities"]
        .as_array()
        .unwrap()
        .iter()
        .any(|capability| capability == "app-shell-event-summary-json"));
    assert!(payload["artifactCapabilities"]
        .as_array()
        .unwrap()
        .iter()
        .any(|capability| capability == "app-shell-dashboard-package-json"));
    assert!(payload["artifactCapabilities"]
        .as_array()
        .unwrap()
        .iter()
        .any(|capability| capability == "app-shell-dashboard-cards-json"));
    assert!(payload["artifactCapabilities"]
        .as_array()
        .unwrap()
        .iter()
        .any(|capability| capability == "app-shell-dashboard-routes-json"));
    assert!(payload["artifactCapabilities"]
        .as_array()
        .unwrap()
        .iter()
        .any(|capability| capability == "app-shell-dashboard-breadcrumbs-json"));
    assert!(payload["artifactCapabilities"]
        .as_array()
        .unwrap()
        .iter()
        .any(|capability| capability == "app-shell-dashboard-tabs-json"));
    assert!(payload["artifactCapabilities"]
        .as_array()
        .unwrap()
        .iter()
        .any(|capability| capability == "app-shell-dashboard-tab-panels-json"));
    assert!(payload["artifactCapabilities"]
        .as_array()
        .unwrap()
        .iter()
        .any(|capability| capability == "app-shell-dashboard-panel-cards-json"));
    assert!(payload["artifactCapabilities"]
        .as_array()
        .unwrap()
        .iter()
        .any(|capability| capability == "app-shell-dashboard-panel-card-actions-json"));
    assert!(payload["artifactCapabilities"]
        .as_array()
        .unwrap()
        .iter()
        .any(|capability| capability
            == "app-shell-dashboard-dispatch-queue-lane-tab-panel-card-action-menu-json"));
    assert!(payload["artifactCapabilities"]
        .as_array()
        .unwrap()
        .iter()
        .any(|capability| capability
            == "app-shell-dashboard-dispatch-queue-lane-tab-panel-card-action-menu-groups-json"));
    assert!(payload["artifactCapabilities"]
        .as_array()
        .unwrap()
        .iter()
        .any(|capability| capability
            == "app-shell-dashboard-dispatch-queue-lane-tab-panel-card-action-menu-group-shortcut-bindings-json"));
    assert!(payload["artifactCapabilities"]
        .as_array()
        .unwrap()
        .iter()
        .any(|capability| capability
            == "app-shell-dashboard-dispatch-queue-lane-tab-panel-card-action-menu-group-shortcut-command-registry-json"));
    assert!(payload["artifactCapabilities"]
        .as_array()
        .unwrap()
        .iter()
        .any(|capability| capability
            == "app-shell-dashboard-dispatch-queue-lane-tab-panel-card-action-menu-group-shortcut-command-palette-json"));
    assert!(payload["artifactCapabilities"]
        .as_array()
        .unwrap()
        .iter()
        .any(|capability| capability
            == "app-shell-dashboard-dispatch-queue-lane-tab-panel-card-action-menu-group-shortcut-command-palette-search-index-json"));
    assert!(payload["artifactCapabilities"]
        .as_array()
        .unwrap()
        .iter()
        .any(|capability| capability
            == "app-shell-dashboard-dispatch-queue-lane-tab-panel-card-action-menu-group-shortcut-command-palette-search-results-json"));
    assert!(payload["artifactCapabilities"]
        .as_array()
        .unwrap()
        .iter()
        .any(|capability| capability
            == "app-shell-dashboard-dispatch-queue-lane-tab-panel-card-action-menu-group-shortcut-command-palette-search-selection-json"));
    assert!(payload["artifactCapabilities"]
        .as_array()
        .unwrap()
        .iter()
        .any(|capability| capability
            == "app-shell-dashboard-dispatch-queue-lane-tab-panel-card-action-menu-group-shortcut-command-palette-search-invocation-json"));
    assert!(payload["artifactCapabilities"]
        .as_array()
        .unwrap()
        .iter()
        .any(|capability| capability
            == "app-shell-dashboard-dispatch-queue-lane-tab-panel-card-action-menu-group-shortcut-command-palette-search-invocation-receipts-json"));
    assert!(payload["artifactCapabilities"]
        .as_array()
        .unwrap()
        .iter()
        .any(|capability| capability
            == "app-shell-dashboard-dispatch-queue-lane-tab-panel-card-action-menu-group-shortcut-command-palette-search-invocation-receipt-summary-json"));
    assert!(payload["artifactCapabilities"]
        .as_array()
        .unwrap()
        .iter()
        .any(|capability| capability
            == "app-shell-dashboard-dispatch-queue-lane-tab-panel-card-action-menu-group-shortcut-command-palette-search-invocation-receipt-notification-json"));
    assert!(payload["artifactCapabilities"]
        .as_array()
        .unwrap()
        .iter()
        .any(|capability| capability
            == "app-shell-dashboard-dispatch-queue-lane-tab-panel-card-action-menu-group-shortcut-command-palette-search-invocation-receipt-notification-stack-json"));
    assert!(payload["artifactCapabilities"]
        .as_array()
        .unwrap()
        .iter()
        .any(|capability| capability
            == "app-shell-dashboard-dispatch-queue-lane-tab-panel-card-action-menu-group-shortcut-command-palette-search-invocation-receipt-notification-stack-summary-json"));
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

    let summary = app
        .run_app_startup_summary(BerkeleyAppPersistedEditorState {
            selected_syntax_card_index: Some(3),
            active_command_id: Some("analysis.3.inspect-waveform".to_string()),
        })
        .expect("startup summary should execute");
    assert_eq!(
        summary.schema_version,
        BERKELEY_APP_STARTUP_SUMMARY_SCHEMA_VERSION
    );
    assert_eq!(summary.package_name, BERKELEY_APP_PACKAGE_NAME);
    assert_eq!(
        summary.source_fingerprint,
        snapshot.host_surface.source_fingerprint
    );
    assert!(summary.ready);
    assert!(summary.parsed);
    assert!(summary.execution_available);
    assert_eq!(summary.active_panel_id.as_deref(), Some("waveform"));
    assert_eq!(summary.panel_count, 5);
    assert_eq!(summary.diagnostic_count, 0);
    assert!(!summary.selection_stale);
    assert!(!summary.command_stale);

    let summary_payload: serde_json::Value = serde_json::from_str(
        &app.run_app_startup_summary_json(BerkeleyAppPersistedEditorState {
            selected_syntax_card_index: Some(3),
            active_command_id: Some("analysis.3.inspect-waveform".to_string()),
        })
        .unwrap(),
    )
    .expect("startup summary JSON should parse");
    assert_eq!(summary_payload["schemaVersion"], 1);
    assert_eq!(summary_payload["packageName"], "berkeley-spice-mosaic-app");
    assert_eq!(summary_payload["ready"], true);
    assert_eq!(summary_payload["activePanelId"], "waveform");
    assert_eq!(
        summary_payload["resolvedActiveCommandId"],
        "analysis.3.inspect-waveform"
    );
    assert_eq!(summary_payload["diagnosticCount"], 0);

    let launch_plan = app
        .run_app_launch_plan(BerkeleyAppPersistedEditorState {
            selected_syntax_card_index: Some(3),
            active_command_id: Some("analysis.3.inspect-waveform".to_string()),
        })
        .expect("launch plan should execute");
    assert_eq!(
        launch_plan.schema_version,
        BERKELEY_APP_LAUNCH_PLAN_SCHEMA_VERSION
    );
    assert_eq!(launch_plan.package_name, BERKELEY_APP_PACKAGE_NAME);
    assert_eq!(launch_plan.startup_route, "ready");
    assert!(launch_plan.ready);
    assert_eq!(launch_plan.entry_panel_id.as_deref(), Some("waveform"));
    assert_eq!(launch_plan.entry_panel_kind.as_deref(), Some("waveform"));
    assert_eq!(
        launch_plan.entry_target.as_deref(),
        Some("analysis-waveform")
    );
    assert_eq!(
        launch_plan.resolved_active_command_id.as_deref(),
        Some("analysis.3.inspect-waveform")
    );
    assert_eq!(launch_plan.action_count, 5);
    let primary_action = launch_plan
        .actions
        .iter()
        .find(|action| action.primary)
        .expect("launch plan should expose one primary action");
    assert_eq!(primary_action.id, "launch.waveform");
    assert_eq!(primary_action.panel_id, "waveform");
    assert_eq!(primary_action.target, "analysis-waveform");
    assert!(primary_action.enabled);

    let launch_payload: serde_json::Value = serde_json::from_str(
        &app.run_app_launch_plan_json(BerkeleyAppPersistedEditorState {
            selected_syntax_card_index: Some(3),
            active_command_id: Some("analysis.3.inspect-waveform".to_string()),
        })
        .unwrap(),
    )
    .expect("launch plan JSON should parse");
    assert_eq!(launch_payload["schemaVersion"], 1);
    assert_eq!(launch_payload["startupRoute"], "ready");
    assert_eq!(launch_payload["entryPanelId"], "waveform");
    assert_eq!(launch_payload["entryPanelKind"], "waveform");
    assert_eq!(launch_payload["entryTarget"], "analysis-waveform");
    assert_eq!(launch_payload["actionCount"], 5);
    assert_eq!(launch_payload["actions"][4]["id"], "launch.waveform");
    assert_eq!(launch_payload["actions"][4]["primary"], true);
    assert_eq!(launch_payload["actions"][4]["enabled"], true);

    let readiness_report = app
        .run_app_readiness_report(BerkeleyAppPersistedEditorState {
            selected_syntax_card_index: Some(3),
            active_command_id: Some("analysis.3.inspect-waveform".to_string()),
        })
        .expect("readiness report should execute");
    assert_eq!(
        readiness_report.schema_version,
        BERKELEY_APP_READINESS_REPORT_SCHEMA_VERSION
    );
    assert_eq!(readiness_report.package_name, BERKELEY_APP_PACKAGE_NAME);
    assert_eq!(readiness_report.startup_route, "ready");
    assert!(readiness_report.ready);
    assert!(readiness_report.parsed);
    assert!(readiness_report.execution_available);
    assert_eq!(readiness_report.entry_panel_id.as_deref(), Some("waveform"));
    assert_eq!(
        readiness_report.primary_action_id.as_deref(),
        Some("launch.waveform")
    );
    assert!(readiness_report.primary_action_enabled);
    assert_eq!(readiness_report.panel_count, 5);
    assert_eq!(readiness_report.enabled_panel_count, 4);
    assert_eq!(readiness_report.disabled_panel_count, 1);
    assert_eq!(readiness_report.action_count, 5);
    assert_eq!(readiness_report.enabled_action_count, 4);
    assert_eq!(readiness_report.disabled_action_count, 1);
    assert_eq!(readiness_report.diagnostic_count, 0);
    assert_eq!(readiness_report.error_count, 0);
    assert_eq!(readiness_report.warning_count, 0);
    assert_eq!(readiness_report.note_count, 0);
    assert!(!readiness_report.repaired_state);

    let readiness_payload: serde_json::Value = serde_json::from_str(
        &app.run_app_readiness_report_json(BerkeleyAppPersistedEditorState {
            selected_syntax_card_index: Some(3),
            active_command_id: Some("analysis.3.inspect-waveform".to_string()),
        })
        .unwrap(),
    )
    .expect("readiness report JSON should parse");
    assert_eq!(readiness_payload["schemaVersion"], 1);
    assert_eq!(readiness_payload["startupRoute"], "ready");
    assert_eq!(readiness_payload["entryPanelId"], "waveform");
    assert_eq!(readiness_payload["primaryActionId"], "launch.waveform");
    assert_eq!(readiness_payload["enabledPanelCount"], 4);
    assert_eq!(readiness_payload["disabledPanelCount"], 1);
    assert_eq!(readiness_payload["enabledActionCount"], 4);
    assert_eq!(readiness_payload["errorCount"], 0);
    assert_eq!(readiness_payload["repairedState"], false);

    let handoff = app
        .run_app_shell_handoff(BerkeleyAppPersistedEditorState {
            selected_syntax_card_index: Some(3),
            active_command_id: Some("analysis.3.inspect-waveform".to_string()),
        })
        .expect("shell handoff should execute");
    assert_eq!(
        handoff.schema_version,
        BERKELEY_APP_SHELL_HANDOFF_SCHEMA_VERSION
    );
    assert_eq!(
        handoff.package_manifest.package_name,
        BERKELEY_APP_PACKAGE_NAME
    );
    assert!(handoff.startup_summary.ready);
    assert_eq!(
        handoff.launch_plan.entry_panel_id.as_deref(),
        Some("waveform")
    );
    assert_eq!(handoff.readiness_report.error_count, 0);

    let handoff_payload: serde_json::Value = serde_json::from_str(
        &app.run_app_shell_handoff_json(BerkeleyAppPersistedEditorState {
            selected_syntax_card_index: Some(3),
            active_command_id: Some("analysis.3.inspect-waveform".to_string()),
        })
        .unwrap(),
    )
    .expect("shell handoff JSON should parse");
    assert_eq!(handoff_payload["schemaVersion"], 1);
    assert_eq!(
        handoff_payload["packageManifest"]["packageName"],
        "berkeley-spice-mosaic-app"
    );
    assert_eq!(handoff_payload["startupSummary"]["ready"], true);
    assert_eq!(handoff_payload["launchPlan"]["entryPanelId"], "waveform");
    assert_eq!(handoff_payload["readinessReport"]["errorCount"], 0);

    let shell_status = app
        .run_app_shell_status(BerkeleyAppPersistedEditorState {
            selected_syntax_card_index: Some(3),
            active_command_id: Some("analysis.3.inspect-waveform".to_string()),
        })
        .expect("shell status should execute");
    assert_eq!(
        shell_status.schema_version,
        BERKELEY_APP_SHELL_STATUS_SCHEMA_VERSION
    );
    assert_eq!(shell_status.package_name, BERKELEY_APP_PACKAGE_NAME);
    assert!(shell_status.ready);
    assert_eq!(shell_status.startup_route, "ready");
    assert_eq!(shell_status.severity, "ready");
    assert_eq!(shell_status.message, "Ready to launch waveform panel");
    assert_eq!(shell_status.entry_panel_id.as_deref(), Some("waveform"));
    assert_eq!(
        shell_status.primary_action_id.as_deref(),
        Some("launch.waveform")
    );
    assert_eq!(shell_status.error_count, 0);

    let shell_status_payload: serde_json::Value = serde_json::from_str(
        &app.run_app_shell_status_json(BerkeleyAppPersistedEditorState {
            selected_syntax_card_index: Some(3),
            active_command_id: Some("analysis.3.inspect-waveform".to_string()),
        })
        .unwrap(),
    )
    .expect("shell status JSON should parse");
    assert_eq!(shell_status_payload["schemaVersion"], 1);
    assert_eq!(shell_status_payload["ready"], true);
    assert_eq!(shell_status_payload["severity"], "ready");
    assert_eq!(
        shell_status_payload["message"],
        "Ready to launch waveform panel"
    );
    assert_eq!(shell_status_payload["entryPanelId"], "waveform");
    assert_eq!(shell_status_payload["primaryActionId"], "launch.waveform");
    assert_eq!(shell_status_payload["errorCount"], 0);

    let shell_telemetry = app
        .run_app_shell_telemetry(BerkeleyAppPersistedEditorState {
            selected_syntax_card_index: Some(3),
            active_command_id: Some("analysis.3.inspect-waveform".to_string()),
        })
        .expect("shell telemetry should execute");
    assert_eq!(
        shell_telemetry.schema_version,
        BERKELEY_APP_SHELL_TELEMETRY_SCHEMA_VERSION
    );
    assert_eq!(shell_telemetry.package_name, BERKELEY_APP_PACKAGE_NAME);
    assert!(shell_telemetry.ready);
    assert_eq!(shell_telemetry.startup_route, "ready");
    assert_eq!(shell_telemetry.severity, "ready");
    assert_eq!(shell_telemetry.message, "Ready to launch waveform panel");
    assert_eq!(
        shell_telemetry.primary_action_id.as_deref(),
        Some("launch.waveform")
    );
    assert_eq!(shell_telemetry.panel_count, 5);
    assert_eq!(shell_telemetry.enabled_panel_count, 4);
    assert_eq!(shell_telemetry.disabled_panel_count, 1);
    assert_eq!(shell_telemetry.action_count, 5);
    assert_eq!(shell_telemetry.enabled_action_count, 4);
    assert_eq!(shell_telemetry.disabled_action_count, 1);
    assert_eq!(shell_telemetry.error_count, 0);
    assert!(!shell_telemetry.repaired_state);
    assert_eq!(
        shell_telemetry.artifact_capability_count,
        handoff.package_manifest.artifact_capabilities.len()
    );

    let shell_telemetry_payload: serde_json::Value = serde_json::from_str(
        &app.run_app_shell_telemetry_json(BerkeleyAppPersistedEditorState {
            selected_syntax_card_index: Some(3),
            active_command_id: Some("analysis.3.inspect-waveform".to_string()),
        })
        .unwrap(),
    )
    .expect("shell telemetry JSON should parse");
    assert_eq!(shell_telemetry_payload["schemaVersion"], 1);
    assert_eq!(shell_telemetry_payload["ready"], true);
    assert_eq!(shell_telemetry_payload["severity"], "ready");
    assert_eq!(shell_telemetry_payload["panelCount"], 5);
    assert_eq!(shell_telemetry_payload["enabledPanelCount"], 4);
    assert_eq!(shell_telemetry_payload["disabledActionCount"], 1);
    assert_eq!(shell_telemetry_payload["repairedState"], false);
    assert_eq!(
        shell_telemetry_payload["artifactCapabilityCount"].as_u64(),
        Some(handoff.package_manifest.artifact_capabilities.len() as u64)
    );

    let shell_events = app
        .run_app_shell_event_log(BerkeleyAppPersistedEditorState {
            selected_syntax_card_index: Some(3),
            active_command_id: Some("analysis.3.inspect-waveform".to_string()),
        })
        .expect("shell event log should execute");
    assert_eq!(
        shell_events.schema_version,
        BERKELEY_APP_SHELL_EVENT_LOG_SCHEMA_VERSION
    );
    assert_eq!(shell_events.package_name, BERKELEY_APP_PACKAGE_NAME);
    assert!(shell_events.ready);
    assert_eq!(shell_events.startup_route, "ready");
    assert_eq!(shell_events.event_count, 6);
    assert_eq!(shell_events.events[0].id, "shell.status");
    assert_eq!(shell_events.events[0].kind, "status");
    assert_eq!(shell_events.events[0].severity, "ready");
    assert_eq!(shell_events.events[0].panel_id.as_deref(), Some("waveform"));
    assert_eq!(
        shell_events.events[0].action_id.as_deref(),
        Some("launch.waveform")
    );
    assert_eq!(shell_events.events[1].id, "shell.route.ready");
    assert_eq!(shell_events.events[2].id, "shell.action.primary");
    assert_eq!(
        shell_events.events[2].action_id.as_deref(),
        Some("launch.waveform")
    );
    assert_eq!(shell_events.events[3].id, "shell.diagnostics");
    assert_eq!(shell_events.events[3].severity, "info");
    assert_eq!(shell_events.events[3].count, Some(0));
    assert_eq!(shell_events.events[4].id, "shell.state");
    assert_eq!(shell_events.events[4].severity, "info");
    assert_eq!(shell_events.events[4].count, Some(0));
    assert_eq!(shell_events.events[5].id, "shell.capabilities");
    assert_eq!(
        shell_events.events[5].count,
        Some(handoff.package_manifest.artifact_capabilities.len())
    );

    let shell_events_payload: serde_json::Value = serde_json::from_str(
        &app.run_app_shell_event_log_json(BerkeleyAppPersistedEditorState {
            selected_syntax_card_index: Some(3),
            active_command_id: Some("analysis.3.inspect-waveform".to_string()),
        })
        .unwrap(),
    )
    .expect("shell event log JSON should parse");
    assert_eq!(shell_events_payload["schemaVersion"], 1);
    assert_eq!(shell_events_payload["ready"], true);
    assert_eq!(shell_events_payload["startupRoute"], "ready");
    assert_eq!(shell_events_payload["eventCount"], 6);
    assert_eq!(shell_events_payload["events"][0]["id"], "shell.status");
    assert_eq!(shell_events_payload["events"][0]["severity"], "ready");
    assert_eq!(
        shell_events_payload["events"][2]["actionId"],
        "launch.waveform"
    );
    assert_eq!(shell_events_payload["events"][3]["count"], 0);
    assert_eq!(
        shell_events_payload["events"][5]["count"].as_u64(),
        Some(handoff.package_manifest.artifact_capabilities.len() as u64)
    );

    let shell_event_summary = app
        .run_app_shell_event_summary(BerkeleyAppPersistedEditorState {
            selected_syntax_card_index: Some(3),
            active_command_id: Some("analysis.3.inspect-waveform".to_string()),
        })
        .expect("shell event summary should execute");
    assert_eq!(
        shell_event_summary.schema_version,
        BERKELEY_APP_SHELL_EVENT_SUMMARY_SCHEMA_VERSION
    );
    assert_eq!(shell_event_summary.package_name, BERKELEY_APP_PACKAGE_NAME);
    assert!(shell_event_summary.ready);
    assert_eq!(shell_event_summary.startup_route, "ready");
    assert_eq!(shell_event_summary.severity, "ready");
    assert_eq!(
        shell_event_summary.status_event_id.as_deref(),
        Some("shell.status")
    );
    assert_eq!(
        shell_event_summary.primary_action_id.as_deref(),
        Some("launch.waveform")
    );
    assert_eq!(shell_event_summary.event_count, 6);
    assert_eq!(shell_event_summary.status_event_count, 1);
    assert_eq!(shell_event_summary.route_event_count, 1);
    assert_eq!(shell_event_summary.action_event_count, 1);
    assert_eq!(shell_event_summary.diagnostic_event_count, 1);
    assert_eq!(shell_event_summary.state_event_count, 1);
    assert_eq!(shell_event_summary.capability_event_count, 1);
    assert_eq!(shell_event_summary.ready_event_count, 3);
    assert_eq!(shell_event_summary.blocked_event_count, 0);
    assert_eq!(shell_event_summary.info_event_count, 3);
    assert_eq!(shell_event_summary.warning_event_count, 0);
    assert_eq!(shell_event_summary.error_event_count, 0);
    assert_eq!(
        shell_event_summary.counted_event_total,
        handoff.package_manifest.artifact_capabilities.len()
    );
    assert_eq!(shell_event_summary.diagnostic_count, 0);
    assert_eq!(shell_event_summary.repaired_state_count, 0);
    assert_eq!(
        shell_event_summary.artifact_capability_count,
        handoff.package_manifest.artifact_capabilities.len()
    );

    let shell_event_summary_payload: serde_json::Value = serde_json::from_str(
        &app.run_app_shell_event_summary_json(BerkeleyAppPersistedEditorState {
            selected_syntax_card_index: Some(3),
            active_command_id: Some("analysis.3.inspect-waveform".to_string()),
        })
        .unwrap(),
    )
    .expect("shell event summary JSON should parse");
    assert_eq!(shell_event_summary_payload["schemaVersion"], 1);
    assert_eq!(shell_event_summary_payload["ready"], true);
    assert_eq!(shell_event_summary_payload["severity"], "ready");
    assert_eq!(shell_event_summary_payload["eventCount"], 6);
    assert_eq!(shell_event_summary_payload["readyEventCount"], 3);
    assert_eq!(shell_event_summary_payload["infoEventCount"], 3);
    assert_eq!(shell_event_summary_payload["diagnosticCount"], 0);
    assert_eq!(
        shell_event_summary_payload["primaryActionId"],
        "launch.waveform"
    );
    assert_eq!(
        shell_event_summary_payload["artifactCapabilityCount"].as_u64(),
        Some(handoff.package_manifest.artifact_capabilities.len() as u64)
    );

    let shell_event_digest = app
        .run_app_shell_event_digest(BerkeleyAppPersistedEditorState {
            selected_syntax_card_index: Some(3),
            active_command_id: Some("analysis.3.inspect-waveform".to_string()),
        })
        .expect("shell event digest should execute");
    assert_eq!(
        shell_event_digest.schema_version,
        BERKELEY_APP_SHELL_EVENT_DIGEST_SCHEMA_VERSION
    );
    assert_eq!(shell_event_digest.package_name, BERKELEY_APP_PACKAGE_NAME);
    assert!(shell_event_digest.ready);
    assert_eq!(shell_event_digest.startup_route, "ready");
    assert_eq!(shell_event_digest.severity, "ready");
    assert_eq!(
        shell_event_digest.headline_event_id.as_deref(),
        Some("shell.status")
    );
    assert_eq!(
        shell_event_digest.headline_message,
        "Ready to launch waveform panel"
    );
    assert_eq!(
        shell_event_digest.primary_action_id.as_deref(),
        Some("launch.waveform")
    );
    assert_eq!(shell_event_digest.attention_event_count, 0);
    assert!(shell_event_digest.attention_event_ids.is_empty());
    assert_eq!(shell_event_digest.metric_event_count, 3);
    assert_eq!(
        shell_event_digest.metric_event_ids,
        vec![
            "shell.diagnostics".to_string(),
            "shell.state".to_string(),
            "shell.capabilities".to_string()
        ]
    );
    assert_eq!(shell_event_digest.event_count, 6);
    assert_eq!(
        shell_event_digest.artifact_capability_count,
        handoff.package_manifest.artifact_capabilities.len()
    );

    let shell_event_digest_payload: serde_json::Value = serde_json::from_str(
        &app.run_app_shell_event_digest_json(BerkeleyAppPersistedEditorState {
            selected_syntax_card_index: Some(3),
            active_command_id: Some("analysis.3.inspect-waveform".to_string()),
        })
        .unwrap(),
    )
    .expect("shell event digest JSON should parse");
    assert_eq!(shell_event_digest_payload["schemaVersion"], 1);
    assert_eq!(shell_event_digest_payload["ready"], true);
    assert_eq!(shell_event_digest_payload["severity"], "ready");
    assert_eq!(
        shell_event_digest_payload["headlineMessage"],
        "Ready to launch waveform panel"
    );
    assert_eq!(shell_event_digest_payload["attentionEventCount"], 0);
    assert_eq!(shell_event_digest_payload["metricEventCount"], 3);
    assert_eq!(
        shell_event_digest_payload["metricEventIds"][2],
        "shell.capabilities"
    );

    let shell_event_dashboard = app
        .run_app_shell_event_dashboard(BerkeleyAppPersistedEditorState {
            selected_syntax_card_index: Some(3),
            active_command_id: Some("analysis.3.inspect-waveform".to_string()),
        })
        .expect("shell event dashboard should execute");
    assert_eq!(
        shell_event_dashboard.schema_version,
        BERKELEY_APP_SHELL_EVENT_DASHBOARD_SCHEMA_VERSION
    );
    assert_eq!(
        shell_event_dashboard.package_name,
        BERKELEY_APP_PACKAGE_NAME
    );
    assert!(shell_event_dashboard.ready);
    assert_eq!(shell_event_dashboard.startup_route, "ready");
    assert_eq!(shell_event_dashboard.severity, "ready");
    assert_eq!(
        shell_event_dashboard.headline_event_id.as_deref(),
        Some("shell.status")
    );
    assert_eq!(
        shell_event_dashboard.headline_message,
        "Ready to launch waveform panel"
    );
    assert!(!shell_event_dashboard.attention_required);
    assert_eq!(shell_event_dashboard.section_count, 3);
    assert_eq!(shell_event_dashboard.sections[0].id, "status");
    assert_eq!(shell_event_dashboard.sections[0].severity, "ready");
    assert_eq!(
        shell_event_dashboard.sections[0].event_ids,
        vec!["shell.status".to_string()]
    );
    assert_eq!(shell_event_dashboard.sections[1].id, "attention");
    assert_eq!(shell_event_dashboard.sections[1].event_count, 0);
    assert_eq!(shell_event_dashboard.sections[1].severity, "ready");
    assert_eq!(shell_event_dashboard.sections[2].id, "metrics");
    assert_eq!(shell_event_dashboard.sections[2].event_count, 3);

    let shell_event_dashboard_payload: serde_json::Value = serde_json::from_str(
        &app.run_app_shell_event_dashboard_json(BerkeleyAppPersistedEditorState {
            selected_syntax_card_index: Some(3),
            active_command_id: Some("analysis.3.inspect-waveform".to_string()),
        })
        .unwrap(),
    )
    .expect("shell event dashboard JSON should parse");
    assert_eq!(shell_event_dashboard_payload["schemaVersion"], 1);
    assert_eq!(shell_event_dashboard_payload["ready"], true);
    assert_eq!(
        shell_event_dashboard_payload["headlineEventId"],
        "shell.status"
    );
    assert_eq!(shell_event_dashboard_payload["attentionRequired"], false);
    assert_eq!(shell_event_dashboard_payload["sectionCount"], 3);
    assert_eq!(
        shell_event_dashboard_payload["sections"][1]["id"],
        "attention"
    );
    assert_eq!(
        shell_event_dashboard_payload["sections"][2]["eventIds"][2],
        "shell.capabilities"
    );

    let shell_dashboard_package = app
        .run_app_shell_dashboard_package(BerkeleyAppPersistedEditorState {
            selected_syntax_card_index: Some(3),
            active_command_id: Some("analysis.3.inspect-waveform".to_string()),
        })
        .expect("shell dashboard package should execute");
    assert_eq!(
        shell_dashboard_package.schema_version,
        BERKELEY_APP_SHELL_DASHBOARD_PACKAGE_SCHEMA_VERSION
    );
    assert_eq!(
        shell_dashboard_package.package_name,
        BERKELEY_APP_PACKAGE_NAME
    );
    assert!(shell_dashboard_package.ready);
    assert_eq!(shell_dashboard_package.startup_route, "ready");
    assert_eq!(shell_dashboard_package.severity, "ready");
    assert!(!shell_dashboard_package.attention_required);
    assert_eq!(shell_dashboard_package.section_count, 3);
    assert_eq!(
        shell_dashboard_package.dashboard_capability_id,
        "app-shell-event-dashboard-json"
    );
    assert_eq!(
        shell_dashboard_package.package_capability_id,
        "app-shell-dashboard-package-json"
    );
    assert!(shell_dashboard_package
        .package_manifest
        .artifact_capabilities
        .iter()
        .any(|capability| capability == &shell_dashboard_package.package_capability_id));
    assert_eq!(
        shell_dashboard_package.artifact_capability_count,
        shell_dashboard_package
            .package_manifest
            .artifact_capabilities
            .len()
    );
    assert_eq!(
        shell_dashboard_package
            .event_dashboard
            .headline_event_id
            .as_deref(),
        Some("shell.status")
    );

    let shell_dashboard_package_payload: serde_json::Value = serde_json::from_str(
        &app.run_app_shell_dashboard_package_json(BerkeleyAppPersistedEditorState {
            selected_syntax_card_index: Some(3),
            active_command_id: Some("analysis.3.inspect-waveform".to_string()),
        })
        .unwrap(),
    )
    .expect("shell dashboard package JSON should parse");
    assert_eq!(shell_dashboard_package_payload["schemaVersion"], 1);
    assert_eq!(shell_dashboard_package_payload["ready"], true);
    assert_eq!(
        shell_dashboard_package_payload["dashboardCapabilityId"],
        "app-shell-event-dashboard-json"
    );
    assert_eq!(
        shell_dashboard_package_payload["packageManifest"]["packageName"],
        BERKELEY_APP_PACKAGE_NAME
    );
    assert_eq!(
        shell_dashboard_package_payload["eventDashboard"]["sections"][0]["id"],
        "status"
    );

    let shell_dashboard_cards = app
        .run_app_shell_dashboard_cards(BerkeleyAppPersistedEditorState {
            selected_syntax_card_index: Some(3),
            active_command_id: Some("analysis.3.inspect-waveform".to_string()),
        })
        .expect("shell dashboard cards should execute");
    assert_eq!(
        shell_dashboard_cards.schema_version,
        BERKELEY_APP_SHELL_DASHBOARD_CARDS_SCHEMA_VERSION
    );
    assert_eq!(
        shell_dashboard_cards.package_name,
        BERKELEY_APP_PACKAGE_NAME
    );
    assert!(shell_dashboard_cards.ready);
    assert_eq!(shell_dashboard_cards.startup_route, "ready");
    assert_eq!(shell_dashboard_cards.severity, "ready");
    assert!(!shell_dashboard_cards.attention_required);
    assert_eq!(shell_dashboard_cards.card_count, 3);
    assert_eq!(
        shell_dashboard_cards.primary_card_id.as_deref(),
        Some("dashboard.status")
    );
    assert_eq!(shell_dashboard_cards.cards[0].id, "dashboard.status");
    assert_eq!(shell_dashboard_cards.cards[0].section_id, "status");
    assert!(shell_dashboard_cards.cards[0].primary);
    assert!(!shell_dashboard_cards.cards[0].attention);
    assert_eq!(shell_dashboard_cards.cards[1].id, "dashboard.attention");
    assert_eq!(shell_dashboard_cards.cards[1].event_count, 0);
    assert!(!shell_dashboard_cards.cards[1].primary);
    assert!(!shell_dashboard_cards.cards[1].attention);
    assert_eq!(shell_dashboard_cards.cards[2].id, "dashboard.metrics");
    assert_eq!(shell_dashboard_cards.cards[2].event_count, 3);
    assert_eq!(
        shell_dashboard_cards.cards[2].event_ids[2],
        "shell.capabilities"
    );
    assert_eq!(
        shell_dashboard_cards.cards_capability_id,
        "app-shell-dashboard-cards-json"
    );
    assert_eq!(
        shell_dashboard_cards.artifact_capability_count,
        shell_dashboard_package.artifact_capability_count
    );

    let shell_dashboard_cards_payload: serde_json::Value = serde_json::from_str(
        &app.run_app_shell_dashboard_cards_json(BerkeleyAppPersistedEditorState {
            selected_syntax_card_index: Some(3),
            active_command_id: Some("analysis.3.inspect-waveform".to_string()),
        })
        .unwrap(),
    )
    .expect("shell dashboard cards JSON should parse");
    assert_eq!(shell_dashboard_cards_payload["schemaVersion"], 1);
    assert_eq!(shell_dashboard_cards_payload["ready"], true);
    assert_eq!(
        shell_dashboard_cards_payload["primaryCardId"],
        "dashboard.status"
    );
    assert_eq!(shell_dashboard_cards_payload["cardCount"], 3);
    assert_eq!(
        shell_dashboard_cards_payload["cards"][0]["sectionId"],
        "status"
    );
    assert_eq!(shell_dashboard_cards_payload["cards"][0]["primary"], true);
    assert_eq!(
        shell_dashboard_cards_payload["cardsCapabilityId"],
        "app-shell-dashboard-cards-json"
    );

    let shell_dashboard_view = app
        .run_app_shell_dashboard_view(BerkeleyAppPersistedEditorState {
            selected_syntax_card_index: Some(3),
            active_command_id: Some("analysis.3.inspect-waveform".to_string()),
        })
        .expect("shell dashboard view should execute");
    assert_eq!(
        shell_dashboard_view.schema_version,
        BERKELEY_APP_SHELL_DASHBOARD_VIEW_SCHEMA_VERSION
    );
    assert_eq!(shell_dashboard_view.ready, shell_dashboard_cards.ready);
    assert_eq!(
        shell_dashboard_view.primary_card_id.as_deref(),
        Some("dashboard.status")
    );
    assert_eq!(
        shell_dashboard_view.primary_card_title.as_deref(),
        Some("Startup status")
    );
    assert_eq!(shell_dashboard_view.card_count, 3);
    assert_eq!(shell_dashboard_view.visible_card_count, 2);
    assert_eq!(shell_dashboard_view.attention_card_count, 0);
    assert_eq!(shell_dashboard_view.metric_card_count, 1);
    assert_eq!(
        shell_dashboard_view.card_ids,
        vec![
            "dashboard.status".to_string(),
            "dashboard.attention".to_string(),
            "dashboard.metrics".to_string()
        ]
    );
    assert_eq!(
        shell_dashboard_view.visible_card_ids,
        vec![
            "dashboard.status".to_string(),
            "dashboard.metrics".to_string()
        ]
    );
    assert!(shell_dashboard_view.attention_card_ids.is_empty());
    assert_eq!(
        shell_dashboard_view.metric_card_ids,
        vec!["dashboard.metrics".to_string()]
    );
    assert_eq!(
        shell_dashboard_view.cards_capability_id,
        shell_dashboard_cards.cards_capability_id
    );
    assert_eq!(
        shell_dashboard_view.view_capability_id,
        "app-shell-dashboard-view-json"
    );
    assert_eq!(
        shell_dashboard_view.artifact_capability_count,
        shell_dashboard_cards.artifact_capability_count
    );

    let shell_dashboard_view_payload: serde_json::Value = serde_json::from_str(
        &app.run_app_shell_dashboard_view_json(BerkeleyAppPersistedEditorState {
            selected_syntax_card_index: Some(3),
            active_command_id: Some("analysis.3.inspect-waveform".to_string()),
        })
        .unwrap(),
    )
    .expect("shell dashboard view JSON should parse");
    assert_eq!(shell_dashboard_view_payload["schemaVersion"], 1);
    assert_eq!(
        shell_dashboard_view_payload["primaryCardTitle"],
        "Startup status"
    );
    assert_eq!(
        shell_dashboard_view_payload["visibleCardIds"][1],
        "dashboard.metrics"
    );
    assert_eq!(
        shell_dashboard_view_payload["viewCapabilityId"],
        "app-shell-dashboard-view-json"
    );

    let shell_dashboard_layout = app
        .run_app_shell_dashboard_layout(BerkeleyAppPersistedEditorState {
            selected_syntax_card_index: Some(3),
            active_command_id: Some("analysis.3.inspect-waveform".to_string()),
        })
        .expect("shell dashboard layout should execute");
    assert_eq!(
        shell_dashboard_layout.schema_version,
        BERKELEY_APP_SHELL_DASHBOARD_LAYOUT_SCHEMA_VERSION
    );
    assert!(shell_dashboard_layout.ready);
    assert_eq!(
        shell_dashboard_layout.primary_card_id.as_deref(),
        Some("dashboard.status")
    );
    assert_eq!(
        shell_dashboard_layout.primary_region_id.as_deref(),
        Some("dashboard.layout.status")
    );
    assert_eq!(shell_dashboard_layout.region_count, 3);
    assert_eq!(shell_dashboard_layout.visible_region_count, 2);
    assert_eq!(
        shell_dashboard_layout.visible_card_count,
        shell_dashboard_view.visible_card_count
    );
    assert_eq!(shell_dashboard_layout.regions[0].role, "status");
    assert!(shell_dashboard_layout.regions[0].primary);
    assert!(shell_dashboard_layout.regions[0].visible);
    assert_eq!(
        shell_dashboard_layout.regions[0].card_ids,
        vec!["dashboard.status".to_string()]
    );
    assert_eq!(shell_dashboard_layout.regions[1].role, "attention");
    assert!(!shell_dashboard_layout.regions[1].primary);
    assert!(!shell_dashboard_layout.regions[1].visible);
    assert_eq!(
        shell_dashboard_layout.regions[1].card_ids,
        vec!["dashboard.attention".to_string()]
    );
    assert_eq!(shell_dashboard_layout.regions[2].role, "metrics");
    assert!(shell_dashboard_layout.regions[2].visible);
    assert_eq!(
        shell_dashboard_layout.layout_capability_id,
        "app-shell-dashboard-layout-json"
    );
    assert_eq!(
        shell_dashboard_layout.view_capability_id,
        shell_dashboard_view.view_capability_id
    );

    let shell_dashboard_layout_payload: serde_json::Value = serde_json::from_str(
        &app.run_app_shell_dashboard_layout_json(BerkeleyAppPersistedEditorState {
            selected_syntax_card_index: Some(3),
            active_command_id: Some("analysis.3.inspect-waveform".to_string()),
        })
        .unwrap(),
    )
    .expect("shell dashboard layout JSON should parse");
    assert_eq!(
        shell_dashboard_layout_payload["primaryRegionId"],
        "dashboard.layout.status"
    );
    assert_eq!(shell_dashboard_layout_payload["visibleRegionCount"], 2);
    assert_eq!(
        shell_dashboard_layout_payload["regions"][1]["visible"],
        false
    );
    assert_eq!(
        shell_dashboard_layout_payload["regions"][2]["role"],
        "metrics"
    );
    assert_eq!(
        shell_dashboard_layout_payload["layoutCapabilityId"],
        "app-shell-dashboard-layout-json"
    );

    let shell_dashboard_navigation = app
        .run_app_shell_dashboard_navigation(BerkeleyAppPersistedEditorState {
            selected_syntax_card_index: Some(3),
            active_command_id: Some("analysis.3.inspect-waveform".to_string()),
        })
        .expect("shell dashboard navigation should execute");
    assert_eq!(
        shell_dashboard_navigation.schema_version,
        BERKELEY_APP_SHELL_DASHBOARD_NAVIGATION_SCHEMA_VERSION
    );
    assert!(shell_dashboard_navigation.ready);
    assert_eq!(
        shell_dashboard_navigation.active_item_id.as_deref(),
        Some("dashboard.nav.status")
    );
    assert_eq!(
        shell_dashboard_navigation.primary_region_id.as_deref(),
        shell_dashboard_layout.primary_region_id.as_deref()
    );
    assert_eq!(shell_dashboard_navigation.item_count, 3);
    assert_eq!(shell_dashboard_navigation.visible_item_count, 2);
    assert_eq!(shell_dashboard_navigation.enabled_item_count, 2);
    assert_eq!(
        shell_dashboard_navigation.items[0].id,
        "dashboard.nav.status"
    );
    assert_eq!(
        shell_dashboard_navigation.items[0].region_id,
        "dashboard.layout.status"
    );
    assert!(shell_dashboard_navigation.items[0].active);
    assert!(shell_dashboard_navigation.items[0].enabled);
    assert_eq!(shell_dashboard_navigation.items[1].role, "attention");
    assert!(!shell_dashboard_navigation.items[1].visible);
    assert!(!shell_dashboard_navigation.items[1].enabled);
    assert_eq!(shell_dashboard_navigation.items[2].label, "Metrics");
    assert_eq!(shell_dashboard_navigation.items[2].badge_count, 1);
    assert_eq!(
        shell_dashboard_navigation.navigation_capability_id,
        "app-shell-dashboard-navigation-json"
    );
    assert_eq!(
        shell_dashboard_navigation.layout_capability_id,
        shell_dashboard_layout.layout_capability_id
    );

    let shell_dashboard_navigation_payload: serde_json::Value = serde_json::from_str(
        &app.run_app_shell_dashboard_navigation_json(BerkeleyAppPersistedEditorState {
            selected_syntax_card_index: Some(3),
            active_command_id: Some("analysis.3.inspect-waveform".to_string()),
        })
        .unwrap(),
    )
    .expect("shell dashboard navigation JSON should parse");
    assert_eq!(
        shell_dashboard_navigation_payload["activeItemId"],
        "dashboard.nav.status"
    );
    assert_eq!(shell_dashboard_navigation_payload["visibleItemCount"], 2);
    assert_eq!(
        shell_dashboard_navigation_payload["items"][1]["enabled"],
        false
    );
    assert_eq!(
        shell_dashboard_navigation_payload["items"][2]["label"],
        "Metrics"
    );
    assert_eq!(
        shell_dashboard_navigation_payload["navigationCapabilityId"],
        "app-shell-dashboard-navigation-json"
    );

    let shell_dashboard_routes = app
        .run_app_shell_dashboard_routes(BerkeleyAppPersistedEditorState {
            selected_syntax_card_index: Some(3),
            active_command_id: Some("analysis.3.inspect-waveform".to_string()),
        })
        .expect("shell dashboard routes should execute");
    assert_eq!(
        shell_dashboard_routes.schema_version,
        BERKELEY_APP_SHELL_DASHBOARD_ROUTES_SCHEMA_VERSION
    );
    assert!(shell_dashboard_routes.ready);
    assert_eq!(
        shell_dashboard_routes.active_route_id.as_deref(),
        Some("dashboard.route.status")
    );
    assert_eq!(
        shell_dashboard_routes.active_route_path.as_deref(),
        Some("/dashboard/status")
    );
    assert_eq!(
        shell_dashboard_routes.default_route_id.as_deref(),
        Some("dashboard.route.status")
    );
    assert_eq!(shell_dashboard_routes.route_count, 3);
    assert_eq!(shell_dashboard_routes.visible_route_count, 2);
    assert_eq!(shell_dashboard_routes.enabled_route_count, 2);
    assert_eq!(
        shell_dashboard_routes.routes[0].item_id,
        "dashboard.nav.status"
    );
    assert_eq!(
        shell_dashboard_routes.routes[0].region_id,
        "dashboard.layout.status"
    );
    assert!(shell_dashboard_routes.routes[0].active);
    assert!(shell_dashboard_routes.routes[0].default_route);
    assert_eq!(shell_dashboard_routes.routes[1].role, "attention");
    assert!(!shell_dashboard_routes.routes[1].visible);
    assert!(!shell_dashboard_routes.routes[1].enabled);
    assert_eq!(shell_dashboard_routes.routes[2].path, "/dashboard/metrics");
    assert_eq!(
        shell_dashboard_routes.routes_capability_id,
        "app-shell-dashboard-routes-json"
    );
    assert_eq!(
        shell_dashboard_routes.navigation_capability_id,
        shell_dashboard_navigation.navigation_capability_id
    );

    let shell_dashboard_routes_payload: serde_json::Value = serde_json::from_str(
        &app.run_app_shell_dashboard_routes_json(BerkeleyAppPersistedEditorState {
            selected_syntax_card_index: Some(3),
            active_command_id: Some("analysis.3.inspect-waveform".to_string()),
        })
        .unwrap(),
    )
    .expect("shell dashboard routes JSON should parse");
    assert_eq!(
        shell_dashboard_routes_payload["activeRouteId"],
        "dashboard.route.status"
    );
    assert_eq!(
        shell_dashboard_routes_payload["defaultRoutePath"],
        "/dashboard/status"
    );
    assert_eq!(shell_dashboard_routes_payload["enabledRouteCount"], 2);
    assert_eq!(
        shell_dashboard_routes_payload["routes"][1]["enabled"],
        false
    );
    assert_eq!(
        shell_dashboard_routes_payload["routes"][2]["path"],
        "/dashboard/metrics"
    );
    assert_eq!(
        shell_dashboard_routes_payload["routesCapabilityId"],
        "app-shell-dashboard-routes-json"
    );

    let shell_dashboard_breadcrumbs = app
        .run_app_shell_dashboard_breadcrumbs(BerkeleyAppPersistedEditorState {
            selected_syntax_card_index: Some(3),
            active_command_id: Some("analysis.3.inspect-waveform".to_string()),
        })
        .expect("shell dashboard breadcrumbs should execute");
    assert_eq!(
        shell_dashboard_breadcrumbs.schema_version,
        BERKELEY_APP_SHELL_DASHBOARD_BREADCRUMBS_SCHEMA_VERSION
    );
    assert!(shell_dashboard_breadcrumbs.ready);
    assert_eq!(
        shell_dashboard_breadcrumbs.active_breadcrumb_id.as_deref(),
        Some("dashboard.breadcrumb.status")
    );
    assert_eq!(
        shell_dashboard_breadcrumbs
            .active_breadcrumb_path
            .as_deref(),
        Some("/dashboard/status")
    );
    assert_eq!(
        shell_dashboard_breadcrumbs.default_breadcrumb_id.as_deref(),
        Some("dashboard.breadcrumb.status")
    );
    assert_eq!(shell_dashboard_breadcrumbs.breadcrumb_count, 3);
    assert_eq!(shell_dashboard_breadcrumbs.visible_breadcrumb_count, 2);
    assert_eq!(shell_dashboard_breadcrumbs.enabled_breadcrumb_count, 2);
    assert_eq!(
        shell_dashboard_breadcrumbs.breadcrumbs[0].route_id,
        "dashboard.route.status"
    );
    assert_eq!(shell_dashboard_breadcrumbs.breadcrumbs[0].position, 1);
    assert!(shell_dashboard_breadcrumbs.breadcrumbs[0].active);
    assert!(shell_dashboard_breadcrumbs.breadcrumbs[0].default_route);
    assert_eq!(shell_dashboard_breadcrumbs.breadcrumbs[1].role, "attention");
    assert!(!shell_dashboard_breadcrumbs.breadcrumbs[1].visible);
    assert!(!shell_dashboard_breadcrumbs.breadcrumbs[1].enabled);
    assert_eq!(
        shell_dashboard_breadcrumbs.breadcrumbs[2].path,
        "/dashboard/metrics"
    );
    assert_eq!(
        shell_dashboard_breadcrumbs.breadcrumbs_capability_id,
        "app-shell-dashboard-breadcrumbs-json"
    );
    assert_eq!(
        shell_dashboard_breadcrumbs.routes_capability_id,
        shell_dashboard_routes.routes_capability_id
    );

    let shell_dashboard_breadcrumbs_payload: serde_json::Value = serde_json::from_str(
        &app.run_app_shell_dashboard_breadcrumbs_json(BerkeleyAppPersistedEditorState {
            selected_syntax_card_index: Some(3),
            active_command_id: Some("analysis.3.inspect-waveform".to_string()),
        })
        .unwrap(),
    )
    .expect("shell dashboard breadcrumbs JSON should parse");
    assert_eq!(
        shell_dashboard_breadcrumbs_payload["activeBreadcrumbId"],
        "dashboard.breadcrumb.status"
    );
    assert_eq!(
        shell_dashboard_breadcrumbs_payload["defaultBreadcrumbPath"],
        "/dashboard/status"
    );
    assert_eq!(
        shell_dashboard_breadcrumbs_payload["visibleBreadcrumbCount"],
        2
    );
    assert_eq!(
        shell_dashboard_breadcrumbs_payload["breadcrumbs"][1]["enabled"],
        false
    );
    assert_eq!(
        shell_dashboard_breadcrumbs_payload["breadcrumbs"][2]["routeId"],
        "dashboard.route.metrics"
    );
    assert_eq!(
        shell_dashboard_breadcrumbs_payload["breadcrumbsCapabilityId"],
        "app-shell-dashboard-breadcrumbs-json"
    );

    let shell_dashboard_tabs = app
        .run_app_shell_dashboard_tabs(BerkeleyAppPersistedEditorState {
            selected_syntax_card_index: Some(3),
            active_command_id: Some("analysis.3.inspect-waveform".to_string()),
        })
        .expect("shell dashboard tabs should execute");
    assert_eq!(
        shell_dashboard_tabs.schema_version,
        BERKELEY_APP_SHELL_DASHBOARD_TABS_SCHEMA_VERSION
    );
    assert!(shell_dashboard_tabs.ready);
    assert_eq!(
        shell_dashboard_tabs.selected_tab_id.as_deref(),
        Some("dashboard.tab.status")
    );
    assert_eq!(
        shell_dashboard_tabs.selected_tab_path.as_deref(),
        Some("/dashboard/status")
    );
    assert_eq!(
        shell_dashboard_tabs.default_tab_id.as_deref(),
        Some("dashboard.tab.status")
    );
    assert_eq!(shell_dashboard_tabs.tab_count, 3);
    assert_eq!(shell_dashboard_tabs.visible_tab_count, 2);
    assert_eq!(shell_dashboard_tabs.enabled_tab_count, 2);
    assert_eq!(
        shell_dashboard_tabs.tabs[0].breadcrumb_id,
        "dashboard.breadcrumb.status"
    );
    assert_eq!(
        shell_dashboard_tabs.tabs[0].route_id,
        "dashboard.route.status"
    );
    assert_eq!(shell_dashboard_tabs.tabs[0].position, 1);
    assert!(shell_dashboard_tabs.tabs[0].selected);
    assert!(shell_dashboard_tabs.tabs[0].default_tab);
    assert_eq!(shell_dashboard_tabs.tabs[1].role, "attention");
    assert!(!shell_dashboard_tabs.tabs[1].visible);
    assert!(!shell_dashboard_tabs.tabs[1].enabled);
    assert_eq!(shell_dashboard_tabs.tabs[2].path, "/dashboard/metrics");
    assert_eq!(
        shell_dashboard_tabs.tabs_capability_id,
        "app-shell-dashboard-tabs-json"
    );
    assert_eq!(
        shell_dashboard_tabs.breadcrumbs_capability_id,
        shell_dashboard_breadcrumbs.breadcrumbs_capability_id
    );

    let shell_dashboard_tabs_payload: serde_json::Value = serde_json::from_str(
        &app.run_app_shell_dashboard_tabs_json(BerkeleyAppPersistedEditorState {
            selected_syntax_card_index: Some(3),
            active_command_id: Some("analysis.3.inspect-waveform".to_string()),
        })
        .unwrap(),
    )
    .expect("shell dashboard tabs JSON should parse");
    assert_eq!(
        shell_dashboard_tabs_payload["selectedTabId"],
        "dashboard.tab.status"
    );
    assert_eq!(
        shell_dashboard_tabs_payload["defaultTabPath"],
        "/dashboard/status"
    );
    assert_eq!(shell_dashboard_tabs_payload["visibleTabCount"], 2);
    assert_eq!(shell_dashboard_tabs_payload["tabs"][1]["enabled"], false);
    assert_eq!(
        shell_dashboard_tabs_payload["tabs"][2]["breadcrumbId"],
        "dashboard.breadcrumb.metrics"
    );
    assert_eq!(
        shell_dashboard_tabs_payload["tabsCapabilityId"],
        "app-shell-dashboard-tabs-json"
    );

    let shell_dashboard_tab_panels = app
        .run_app_shell_dashboard_tab_panels(BerkeleyAppPersistedEditorState {
            selected_syntax_card_index: Some(3),
            active_command_id: Some("analysis.3.inspect-waveform".to_string()),
        })
        .expect("shell dashboard tab panels should execute");
    assert_eq!(
        shell_dashboard_tab_panels.schema_version,
        BERKELEY_APP_SHELL_DASHBOARD_TAB_PANELS_SCHEMA_VERSION
    );
    assert!(shell_dashboard_tab_panels.ready);
    assert_eq!(
        shell_dashboard_tab_panels.selected_panel_id.as_deref(),
        Some("dashboard.tab-panel.status")
    );
    assert_eq!(
        shell_dashboard_tab_panels.selected_panel_path.as_deref(),
        Some("/dashboard/status")
    );
    assert_eq!(
        shell_dashboard_tab_panels.default_panel_id.as_deref(),
        Some("dashboard.tab-panel.status")
    );
    assert_eq!(shell_dashboard_tab_panels.panel_count, 3);
    assert_eq!(shell_dashboard_tab_panels.visible_panel_count, 2);
    assert_eq!(shell_dashboard_tab_panels.enabled_panel_count, 2);
    assert_eq!(
        shell_dashboard_tab_panels.panels[0].tab_id,
        "dashboard.tab.status"
    );
    assert_eq!(
        shell_dashboard_tab_panels.panels[0].region_id,
        "dashboard.layout.status"
    );
    assert_eq!(shell_dashboard_tab_panels.panels[0].title, "Status");
    assert_eq!(shell_dashboard_tab_panels.panels[0].position, 1);
    assert!(shell_dashboard_tab_panels.panels[0].selected);
    assert!(shell_dashboard_tab_panels.panels[0].default_panel);
    assert_eq!(shell_dashboard_tab_panels.panels[1].role, "attention");
    assert!(!shell_dashboard_tab_panels.panels[1].visible);
    assert!(!shell_dashboard_tab_panels.panels[1].enabled);
    assert_eq!(
        shell_dashboard_tab_panels.panels[2].path,
        "/dashboard/metrics"
    );
    assert_eq!(
        shell_dashboard_tab_panels.tab_panels_capability_id,
        "app-shell-dashboard-tab-panels-json"
    );
    assert_eq!(
        shell_dashboard_tab_panels.tabs_capability_id,
        shell_dashboard_tabs.tabs_capability_id
    );

    let shell_dashboard_tab_panels_payload: serde_json::Value = serde_json::from_str(
        &app.run_app_shell_dashboard_tab_panels_json(BerkeleyAppPersistedEditorState {
            selected_syntax_card_index: Some(3),
            active_command_id: Some("analysis.3.inspect-waveform".to_string()),
        })
        .unwrap(),
    )
    .expect("shell dashboard tab panels JSON should parse");
    assert_eq!(
        shell_dashboard_tab_panels_payload["selectedPanelId"],
        "dashboard.tab-panel.status"
    );
    assert_eq!(
        shell_dashboard_tab_panels_payload["defaultPanelPath"],
        "/dashboard/status"
    );
    assert_eq!(shell_dashboard_tab_panels_payload["visiblePanelCount"], 2);
    assert_eq!(
        shell_dashboard_tab_panels_payload["panels"][1]["enabled"],
        false
    );
    assert_eq!(
        shell_dashboard_tab_panels_payload["panels"][2]["tabId"],
        "dashboard.tab.metrics"
    );
    assert_eq!(
        shell_dashboard_tab_panels_payload["tabPanelsCapabilityId"],
        "app-shell-dashboard-tab-panels-json"
    );

    let shell_dashboard_panel_cards = app
        .run_app_shell_dashboard_panel_cards(BerkeleyAppPersistedEditorState {
            selected_syntax_card_index: Some(3),
            active_command_id: Some("analysis.3.inspect-waveform".to_string()),
        })
        .expect("shell dashboard panel cards should execute");
    assert_eq!(
        shell_dashboard_panel_cards.schema_version,
        BERKELEY_APP_SHELL_DASHBOARD_PANEL_CARDS_SCHEMA_VERSION
    );
    assert!(shell_dashboard_panel_cards.ready);
    assert_eq!(
        shell_dashboard_panel_cards
            .selected_panel_card_id
            .as_deref(),
        Some("dashboard.panel-card.status")
    );
    assert_eq!(
        shell_dashboard_panel_cards.selected_card_id.as_deref(),
        Some("dashboard.status")
    );
    assert_eq!(
        shell_dashboard_panel_cards.default_panel_card_id.as_deref(),
        Some("dashboard.panel-card.status")
    );
    assert_eq!(
        shell_dashboard_panel_cards.default_card_id.as_deref(),
        Some("dashboard.status")
    );
    assert_eq!(shell_dashboard_panel_cards.panel_card_count, 3);
    assert_eq!(shell_dashboard_panel_cards.visible_panel_card_count, 2);
    assert_eq!(shell_dashboard_panel_cards.enabled_panel_card_count, 2);
    assert_eq!(
        shell_dashboard_panel_cards.panel_cards[0].panel_id,
        "dashboard.tab-panel.status"
    );
    assert_eq!(
        shell_dashboard_panel_cards.panel_cards[0].card_id,
        "dashboard.status"
    );
    assert_eq!(
        shell_dashboard_panel_cards.panel_cards[0].title,
        "Startup status"
    );
    assert!(shell_dashboard_panel_cards.panel_cards[0].selected);
    assert!(shell_dashboard_panel_cards.panel_cards[0].primary);
    assert_eq!(shell_dashboard_panel_cards.panel_cards[1].role, "attention");
    assert!(!shell_dashboard_panel_cards.panel_cards[1].visible);
    assert!(!shell_dashboard_panel_cards.panel_cards[1].enabled);
    assert_eq!(shell_dashboard_panel_cards.panel_cards[2].event_count, 3);
    assert_eq!(
        shell_dashboard_panel_cards.panel_cards[2].event_ids,
        vec![
            "shell.diagnostics".to_string(),
            "shell.state".to_string(),
            "shell.capabilities".to_string()
        ]
    );
    assert_eq!(
        shell_dashboard_panel_cards.panel_cards_capability_id,
        "app-shell-dashboard-panel-cards-json"
    );
    assert_eq!(
        shell_dashboard_panel_cards.tab_panels_capability_id,
        shell_dashboard_tab_panels.tab_panels_capability_id
    );

    let shell_dashboard_panel_cards_payload: serde_json::Value = serde_json::from_str(
        &app.run_app_shell_dashboard_panel_cards_json(BerkeleyAppPersistedEditorState {
            selected_syntax_card_index: Some(3),
            active_command_id: Some("analysis.3.inspect-waveform".to_string()),
        })
        .unwrap(),
    )
    .expect("shell dashboard panel cards JSON should parse");
    assert_eq!(
        shell_dashboard_panel_cards_payload["selectedPanelCardId"],
        "dashboard.panel-card.status"
    );
    assert_eq!(
        shell_dashboard_panel_cards_payload["defaultCardId"],
        "dashboard.status"
    );
    assert_eq!(
        shell_dashboard_panel_cards_payload["visiblePanelCardCount"],
        2
    );
    assert_eq!(
        shell_dashboard_panel_cards_payload["panelCards"][1]["visible"],
        false
    );
    assert_eq!(
        shell_dashboard_panel_cards_payload["panelCards"][2]["eventIds"][0],
        "shell.diagnostics"
    );
    assert_eq!(
        shell_dashboard_panel_cards_payload["panelCardsCapabilityId"],
        "app-shell-dashboard-panel-cards-json"
    );

    let shell_dashboard_panel_card_actions = app
        .run_app_shell_dashboard_panel_card_actions(BerkeleyAppPersistedEditorState {
            selected_syntax_card_index: Some(3),
            active_command_id: Some("analysis.3.inspect-waveform".to_string()),
        })
        .expect("shell dashboard panel-card actions should execute");
    assert_eq!(
        shell_dashboard_panel_card_actions.schema_version,
        BERKELEY_APP_SHELL_DASHBOARD_PANEL_CARD_ACTIONS_SCHEMA_VERSION
    );
    assert!(shell_dashboard_panel_card_actions.ready);
    assert_eq!(
        shell_dashboard_panel_card_actions
            .selected_panel_card_action_id
            .as_deref(),
        Some("dashboard.panel-card-action.status")
    );
    assert_eq!(
        shell_dashboard_panel_card_actions
            .selected_action_id
            .as_deref(),
        Some("launch.analysis")
    );
    assert_eq!(
        shell_dashboard_panel_card_actions
            .default_panel_card_action_id
            .as_deref(),
        Some("dashboard.panel-card-action.status")
    );
    assert_eq!(
        shell_dashboard_panel_card_actions
            .default_action_id
            .as_deref(),
        Some("launch.analysis")
    );
    assert_eq!(shell_dashboard_panel_card_actions.action_count, 5);
    assert_eq!(
        shell_dashboard_panel_card_actions.panel_card_action_count,
        3
    );
    assert_eq!(
        shell_dashboard_panel_card_actions.visible_panel_card_action_count,
        2
    );
    assert_eq!(
        shell_dashboard_panel_card_actions.enabled_panel_card_action_count,
        2
    );
    assert_eq!(
        shell_dashboard_panel_card_actions.panel_card_actions[0].panel_card_id,
        "dashboard.panel-card.status"
    );
    assert_eq!(
        shell_dashboard_panel_card_actions.panel_card_actions[0].action_id,
        "launch.analysis"
    );
    assert_eq!(
        shell_dashboard_panel_card_actions.panel_card_actions[0].target,
        "analysis-controls"
    );
    assert!(shell_dashboard_panel_card_actions.panel_card_actions[0].selected);
    assert!(shell_dashboard_panel_card_actions.panel_card_actions[0].card_primary);
    assert_eq!(
        shell_dashboard_panel_card_actions.panel_card_actions[1].action_id,
        "launch.diagnostics"
    );
    assert!(!shell_dashboard_panel_card_actions.panel_card_actions[1].enabled);
    assert_eq!(
        shell_dashboard_panel_card_actions.panel_card_actions[2].action_id,
        "launch.waveform"
    );
    assert_eq!(
        shell_dashboard_panel_card_actions.panel_card_actions[2].target,
        "analysis-waveform"
    );
    assert_eq!(
        shell_dashboard_panel_card_actions.panel_card_actions_capability_id,
        "app-shell-dashboard-panel-card-actions-json"
    );
    assert_eq!(
        shell_dashboard_panel_card_actions.panel_cards_capability_id,
        shell_dashboard_panel_cards.panel_cards_capability_id
    );

    let shell_dashboard_panel_card_actions_payload: serde_json::Value = serde_json::from_str(
        &app.run_app_shell_dashboard_panel_card_actions_json(BerkeleyAppPersistedEditorState {
            selected_syntax_card_index: Some(3),
            active_command_id: Some("analysis.3.inspect-waveform".to_string()),
        })
        .unwrap(),
    )
    .expect("shell dashboard panel-card actions JSON should parse");
    assert_eq!(
        shell_dashboard_panel_card_actions_payload["selectedPanelCardActionId"],
        "dashboard.panel-card-action.status"
    );
    assert_eq!(
        shell_dashboard_panel_card_actions_payload["selectedActionId"],
        "launch.analysis"
    );
    assert_eq!(
        shell_dashboard_panel_card_actions_payload["visiblePanelCardActionCount"],
        2
    );
    assert_eq!(
        shell_dashboard_panel_card_actions_payload["panelCardActions"][1]["enabled"],
        false
    );
    assert_eq!(
        shell_dashboard_panel_card_actions_payload["panelCardActions"][2]["actionId"],
        "launch.waveform"
    );
    assert_eq!(
        shell_dashboard_panel_card_actions_payload["panelCardActionsCapabilityId"],
        "app-shell-dashboard-panel-card-actions-json"
    );

    let shell_dashboard_action_dispatch = app
        .run_app_shell_dashboard_action_dispatch(BerkeleyAppPersistedEditorState {
            selected_syntax_card_index: Some(3),
            active_command_id: Some("analysis.3.inspect-waveform".to_string()),
        })
        .expect("shell dashboard action dispatch should execute");
    assert_eq!(
        shell_dashboard_action_dispatch.schema_version,
        BERKELEY_APP_SHELL_DASHBOARD_ACTION_DISPATCH_SCHEMA_VERSION
    );
    assert!(shell_dashboard_action_dispatch.ready);
    assert_eq!(
        shell_dashboard_action_dispatch
            .selected_action_dispatch_id
            .as_deref(),
        Some("dashboard.action-dispatch.status")
    );
    assert_eq!(
        shell_dashboard_action_dispatch
            .selected_action_id
            .as_deref(),
        Some("launch.analysis")
    );
    assert_eq!(
        shell_dashboard_action_dispatch
            .default_action_dispatch_id
            .as_deref(),
        Some("dashboard.action-dispatch.status")
    );
    assert_eq!(
        shell_dashboard_action_dispatch.default_action_id.as_deref(),
        Some("launch.analysis")
    );
    assert_eq!(shell_dashboard_action_dispatch.action_dispatch_count, 3);
    assert_eq!(
        shell_dashboard_action_dispatch.visible_action_dispatch_count,
        2
    );
    assert_eq!(
        shell_dashboard_action_dispatch.enabled_action_dispatch_count,
        2
    );
    assert_eq!(
        shell_dashboard_action_dispatch.action_dispatches[0].panel_card_action_id,
        "dashboard.panel-card-action.status"
    );
    assert_eq!(
        shell_dashboard_action_dispatch.action_dispatches[0].action_id,
        "launch.analysis"
    );
    assert!(shell_dashboard_action_dispatch.action_dispatches[0].dispatchable);
    assert_eq!(
        shell_dashboard_action_dispatch.action_dispatches[1].action_id,
        "launch.diagnostics"
    );
    assert!(!shell_dashboard_action_dispatch.action_dispatches[1].dispatchable);
    assert_eq!(
        shell_dashboard_action_dispatch.action_dispatches[2].target,
        "analysis-waveform"
    );
    assert_eq!(
        shell_dashboard_action_dispatch.action_dispatch_capability_id,
        "app-shell-dashboard-action-dispatch-json"
    );
    assert_eq!(
        shell_dashboard_action_dispatch.panel_card_actions_capability_id,
        shell_dashboard_panel_card_actions.panel_card_actions_capability_id
    );

    let shell_dashboard_action_dispatch_payload: serde_json::Value = serde_json::from_str(
        &app.run_app_shell_dashboard_action_dispatch_json(BerkeleyAppPersistedEditorState {
            selected_syntax_card_index: Some(3),
            active_command_id: Some("analysis.3.inspect-waveform".to_string()),
        })
        .unwrap(),
    )
    .expect("shell dashboard action dispatch JSON should parse");
    assert_eq!(
        shell_dashboard_action_dispatch_payload["selectedActionDispatchId"],
        "dashboard.action-dispatch.status"
    );
    assert_eq!(
        shell_dashboard_action_dispatch_payload["selectedActionId"],
        "launch.analysis"
    );
    assert_eq!(
        shell_dashboard_action_dispatch_payload["enabledActionDispatchCount"],
        2
    );
    assert_eq!(
        shell_dashboard_action_dispatch_payload["actionDispatches"][1]["dispatchable"],
        false
    );
    assert_eq!(
        shell_dashboard_action_dispatch_payload["actionDispatches"][2]["target"],
        "analysis-waveform"
    );
    assert_eq!(
        shell_dashboard_action_dispatch_payload["actionDispatchCapabilityId"],
        "app-shell-dashboard-action-dispatch-json"
    );

    let shell_dashboard_dispatch_events = app
        .run_app_shell_dashboard_dispatch_events(BerkeleyAppPersistedEditorState {
            selected_syntax_card_index: Some(3),
            active_command_id: Some("analysis.3.inspect-waveform".to_string()),
        })
        .expect("shell dashboard dispatch events should execute");
    assert_eq!(
        shell_dashboard_dispatch_events.schema_version,
        BERKELEY_APP_SHELL_DASHBOARD_DISPATCH_EVENTS_SCHEMA_VERSION
    );
    assert!(shell_dashboard_dispatch_events.ready);
    assert_eq!(
        shell_dashboard_dispatch_events
            .selected_dispatch_event_id
            .as_deref(),
        Some("dashboard.dispatch-event.status")
    );
    assert_eq!(
        shell_dashboard_dispatch_events
            .default_dispatch_event_id
            .as_deref(),
        Some("dashboard.dispatch-event.status")
    );
    assert_eq!(shell_dashboard_dispatch_events.dispatch_event_count, 3);
    assert_eq!(
        shell_dashboard_dispatch_events.dispatch_ready_event_count,
        2
    );
    assert_eq!(
        shell_dashboard_dispatch_events.dispatch_blocked_event_count,
        1
    );
    assert_eq!(
        shell_dashboard_dispatch_events.attention_dispatch_event_count,
        0
    );
    assert!(shell_dashboard_dispatch_events.selected_dispatchable);
    assert_eq!(
        shell_dashboard_dispatch_events.dispatch_events[0].action_dispatch_id,
        "dashboard.action-dispatch.status"
    );
    assert_eq!(
        shell_dashboard_dispatch_events.dispatch_events[0].kind,
        "dispatch-ready"
    );
    assert_eq!(
        shell_dashboard_dispatch_events.dispatch_events[1].kind,
        "dispatch-blocked"
    );
    assert_eq!(
        shell_dashboard_dispatch_events.dispatch_events[2].target,
        "analysis-waveform"
    );
    assert_eq!(
        shell_dashboard_dispatch_events.action_dispatch_capability_id,
        shell_dashboard_action_dispatch.action_dispatch_capability_id
    );
    assert_eq!(
        shell_dashboard_dispatch_events.dispatch_events_capability_id,
        "app-shell-dashboard-dispatch-events-json"
    );

    let shell_dashboard_dispatch_events_payload: serde_json::Value = serde_json::from_str(
        &app.run_app_shell_dashboard_dispatch_events_json(BerkeleyAppPersistedEditorState {
            selected_syntax_card_index: Some(3),
            active_command_id: Some("analysis.3.inspect-waveform".to_string()),
        })
        .unwrap(),
    )
    .expect("shell dashboard dispatch events JSON should parse");
    assert_eq!(
        shell_dashboard_dispatch_events_payload["selectedDispatchEventId"],
        "dashboard.dispatch-event.status"
    );
    assert_eq!(
        shell_dashboard_dispatch_events_payload["dispatchReadyEventCount"],
        2
    );
    assert_eq!(
        shell_dashboard_dispatch_events_payload["dispatchBlockedEventCount"],
        1
    );
    assert_eq!(
        shell_dashboard_dispatch_events_payload["dispatchEvents"][1]["kind"],
        "dispatch-blocked"
    );
    assert_eq!(
        shell_dashboard_dispatch_events_payload["dispatchEvents"][2]["actionId"],
        "launch.waveform"
    );
    assert_eq!(
        shell_dashboard_dispatch_events_payload["dispatchEventsCapabilityId"],
        "app-shell-dashboard-dispatch-events-json"
    );

    let shell_dashboard_dispatch_queue = app
        .run_app_shell_dashboard_dispatch_queue(BerkeleyAppPersistedEditorState {
            selected_syntax_card_index: Some(3),
            active_command_id: Some("analysis.3.inspect-waveform".to_string()),
        })
        .expect("shell dashboard dispatch queue should execute");
    assert_eq!(
        shell_dashboard_dispatch_queue.schema_version,
        BERKELEY_APP_SHELL_DASHBOARD_DISPATCH_QUEUE_SCHEMA_VERSION
    );
    assert!(shell_dashboard_dispatch_queue.ready);
    assert_eq!(
        shell_dashboard_dispatch_queue
            .selected_dispatch_queue_item_id
            .as_deref(),
        Some("dashboard.dispatch-queue.status")
    );
    assert_eq!(
        shell_dashboard_dispatch_queue
            .default_dispatch_queue_item_id
            .as_deref(),
        Some("dashboard.dispatch-queue.status")
    );
    assert_eq!(shell_dashboard_dispatch_queue.dispatch_queue_item_count, 3);
    assert_eq!(shell_dashboard_dispatch_queue.queued_dispatch_count, 2);
    assert_eq!(shell_dashboard_dispatch_queue.blocked_dispatch_count, 1);
    assert_eq!(
        shell_dashboard_dispatch_queue.attention_dispatch_queue_item_count,
        0
    );
    assert!(shell_dashboard_dispatch_queue.selected_queued);
    assert_eq!(
        shell_dashboard_dispatch_queue.dispatch_queue_items[0].dispatch_event_id,
        "dashboard.dispatch-event.status"
    );
    assert_eq!(
        shell_dashboard_dispatch_queue.dispatch_queue_items[0].queue_state,
        "queued"
    );
    assert_eq!(
        shell_dashboard_dispatch_queue.dispatch_queue_items[1].queue_state,
        "blocked"
    );
    assert_eq!(
        shell_dashboard_dispatch_queue.dispatch_queue_items[2].target,
        "analysis-waveform"
    );
    assert_eq!(
        shell_dashboard_dispatch_queue.dispatch_events_capability_id,
        shell_dashboard_dispatch_events.dispatch_events_capability_id
    );
    assert_eq!(
        shell_dashboard_dispatch_queue.dispatch_queue_capability_id,
        "app-shell-dashboard-dispatch-queue-json"
    );

    let shell_dashboard_dispatch_queue_payload: serde_json::Value = serde_json::from_str(
        &app.run_app_shell_dashboard_dispatch_queue_json(BerkeleyAppPersistedEditorState {
            selected_syntax_card_index: Some(3),
            active_command_id: Some("analysis.3.inspect-waveform".to_string()),
        })
        .unwrap(),
    )
    .expect("shell dashboard dispatch queue JSON should parse");
    assert_eq!(
        shell_dashboard_dispatch_queue_payload["selectedDispatchQueueItemId"],
        "dashboard.dispatch-queue.status"
    );
    assert_eq!(
        shell_dashboard_dispatch_queue_payload["queuedDispatchCount"],
        2
    );
    assert_eq!(
        shell_dashboard_dispatch_queue_payload["blockedDispatchCount"],
        1
    );
    assert_eq!(
        shell_dashboard_dispatch_queue_payload["dispatchQueueItems"][1]["queueState"],
        "blocked"
    );
    assert_eq!(
        shell_dashboard_dispatch_queue_payload["dispatchQueueItems"][2]["actionId"],
        "launch.waveform"
    );
    assert_eq!(
        shell_dashboard_dispatch_queue_payload["dispatchQueueCapabilityId"],
        "app-shell-dashboard-dispatch-queue-json"
    );

    let shell_dashboard_dispatch_queue_summary = app
        .run_app_shell_dashboard_dispatch_queue_summary(BerkeleyAppPersistedEditorState {
            selected_syntax_card_index: Some(3),
            active_command_id: Some("analysis.3.inspect-waveform".to_string()),
        })
        .expect("shell dashboard dispatch queue summary should execute");
    assert_eq!(
        shell_dashboard_dispatch_queue_summary.schema_version,
        BERKELEY_APP_SHELL_DASHBOARD_DISPATCH_QUEUE_SUMMARY_SCHEMA_VERSION
    );
    assert!(shell_dashboard_dispatch_queue_summary.ready);
    assert_eq!(
        shell_dashboard_dispatch_queue_summary
            .selected_dispatch_queue_item_id
            .as_deref(),
        Some("dashboard.dispatch-queue.status")
    );
    assert_eq!(
        shell_dashboard_dispatch_queue_summary
            .default_dispatch_queue_item_id
            .as_deref(),
        Some("dashboard.dispatch-queue.status")
    );
    assert_eq!(
        shell_dashboard_dispatch_queue_summary.dispatch_queue_item_count,
        3
    );
    assert_eq!(
        shell_dashboard_dispatch_queue_summary.queued_dispatch_count,
        2
    );
    assert_eq!(
        shell_dashboard_dispatch_queue_summary.blocked_dispatch_count,
        1
    );
    assert_eq!(
        shell_dashboard_dispatch_queue_summary.attention_dispatch_queue_item_count,
        0
    );
    assert!(shell_dashboard_dispatch_queue_summary.selected_queued);
    assert_eq!(
        shell_dashboard_dispatch_queue_summary
            .first_queued_dispatch_queue_item_id
            .as_deref(),
        Some("dashboard.dispatch-queue.status")
    );
    assert_eq!(
        shell_dashboard_dispatch_queue_summary
            .first_blocked_dispatch_queue_item_id
            .as_deref(),
        Some("dashboard.dispatch-queue.attention")
    );
    assert_eq!(
        shell_dashboard_dispatch_queue_summary
            .first_attention_dispatch_queue_item_id
            .as_deref(),
        None
    );
    assert_eq!(
        shell_dashboard_dispatch_queue_summary.queued_dispatch_queue_item_ids,
        vec![
            "dashboard.dispatch-queue.status".to_string(),
            "dashboard.dispatch-queue.metrics".to_string()
        ]
    );
    assert_eq!(
        shell_dashboard_dispatch_queue_summary.blocked_dispatch_queue_item_ids,
        vec!["dashboard.dispatch-queue.attention".to_string()]
    );
    assert_eq!(
        shell_dashboard_dispatch_queue_summary.dispatch_queue_capability_id,
        shell_dashboard_dispatch_queue.dispatch_queue_capability_id
    );
    assert_eq!(
        shell_dashboard_dispatch_queue_summary.dispatch_queue_summary_capability_id,
        "app-shell-dashboard-dispatch-queue-summary-json"
    );

    let shell_dashboard_dispatch_queue_summary_payload: serde_json::Value = serde_json::from_str(
        &app.run_app_shell_dashboard_dispatch_queue_summary_json(BerkeleyAppPersistedEditorState {
            selected_syntax_card_index: Some(3),
            active_command_id: Some("analysis.3.inspect-waveform".to_string()),
        })
        .unwrap(),
    )
    .expect("shell dashboard dispatch queue summary JSON should parse");
    assert_eq!(
        shell_dashboard_dispatch_queue_summary_payload["selectedDispatchQueueItemId"],
        "dashboard.dispatch-queue.status"
    );
    assert_eq!(
        shell_dashboard_dispatch_queue_summary_payload["queuedDispatchQueueItemIds"][1],
        "dashboard.dispatch-queue.metrics"
    );
    assert_eq!(
        shell_dashboard_dispatch_queue_summary_payload["blockedDispatchQueueItemIds"][0],
        "dashboard.dispatch-queue.attention"
    );
    assert!(
        shell_dashboard_dispatch_queue_summary_payload["firstAttentionDispatchQueueItemId"]
            .is_null()
    );
    assert_eq!(
        shell_dashboard_dispatch_queue_summary_payload["dispatchQueueSummaryCapabilityId"],
        "app-shell-dashboard-dispatch-queue-summary-json"
    );

    let shell_dashboard_dispatch_queue_digest = app
        .run_app_shell_dashboard_dispatch_queue_digest(BerkeleyAppPersistedEditorState {
            selected_syntax_card_index: Some(3),
            active_command_id: Some("analysis.3.inspect-waveform".to_string()),
        })
        .expect("shell dashboard dispatch queue digest should execute");
    assert_eq!(
        shell_dashboard_dispatch_queue_digest.schema_version,
        BERKELEY_APP_SHELL_DASHBOARD_DISPATCH_QUEUE_DIGEST_SCHEMA_VERSION
    );
    assert!(shell_dashboard_dispatch_queue_digest.ready);
    assert_eq!(
        shell_dashboard_dispatch_queue_digest
            .headline_dispatch_queue_item_id
            .as_deref(),
        Some("dashboard.dispatch-queue.status")
    );
    assert_eq!(
        shell_dashboard_dispatch_queue_digest
            .headline_action_dispatch_id
            .as_deref(),
        Some("dashboard.action-dispatch.status")
    );
    assert_eq!(
        shell_dashboard_dispatch_queue_digest
            .headline_action_id
            .as_deref(),
        Some("launch.analysis")
    );
    assert_eq!(
        shell_dashboard_dispatch_queue_digest
            .headline_queue_state
            .as_deref(),
        Some("queued")
    );
    assert_eq!(
        shell_dashboard_dispatch_queue_digest.headline_message,
        "Analysis queued for dispatch"
    );
    assert!(shell_dashboard_dispatch_queue_digest.headline_selected);
    assert!(shell_dashboard_dispatch_queue_digest.headline_default_dispatch);
    assert!(shell_dashboard_dispatch_queue_digest.headline_queued);
    assert!(!shell_dashboard_dispatch_queue_digest.headline_attention);
    assert_eq!(
        shell_dashboard_dispatch_queue_digest.blocked_dispatch_count,
        shell_dashboard_dispatch_queue_summary.blocked_dispatch_count
    );
    assert_eq!(
        shell_dashboard_dispatch_queue_digest
            .first_blocked_dispatch_queue_item_id
            .as_deref(),
        Some("dashboard.dispatch-queue.attention")
    );
    assert_eq!(
        shell_dashboard_dispatch_queue_digest.dispatch_queue_summary_capability_id,
        shell_dashboard_dispatch_queue_summary.dispatch_queue_summary_capability_id
    );
    assert_eq!(
        shell_dashboard_dispatch_queue_digest.dispatch_queue_digest_capability_id,
        "app-shell-dashboard-dispatch-queue-digest-json"
    );

    let shell_dashboard_dispatch_queue_digest_payload: serde_json::Value = serde_json::from_str(
        &app.run_app_shell_dashboard_dispatch_queue_digest_json(BerkeleyAppPersistedEditorState {
            selected_syntax_card_index: Some(3),
            active_command_id: Some("analysis.3.inspect-waveform".to_string()),
        })
        .unwrap(),
    )
    .expect("shell dashboard dispatch queue digest JSON should parse");
    assert_eq!(
        shell_dashboard_dispatch_queue_digest_payload["headlineDispatchQueueItemId"],
        "dashboard.dispatch-queue.status"
    );
    assert_eq!(
        shell_dashboard_dispatch_queue_digest_payload["headlineQueueState"],
        "queued"
    );
    assert_eq!(
        shell_dashboard_dispatch_queue_digest_payload["headlineMessage"],
        "Analysis queued for dispatch"
    );
    assert_eq!(
        shell_dashboard_dispatch_queue_digest_payload["headlineQueued"],
        true
    );
    assert_eq!(
        shell_dashboard_dispatch_queue_digest_payload["dispatchQueueDigestCapabilityId"],
        "app-shell-dashboard-dispatch-queue-digest-json"
    );

    let shell_dashboard_dispatch_queue_lanes = app
        .run_app_shell_dashboard_dispatch_queue_lanes(BerkeleyAppPersistedEditorState {
            selected_syntax_card_index: Some(3),
            active_command_id: Some("analysis.3.inspect-waveform".to_string()),
        })
        .expect("shell dashboard dispatch queue lanes should execute");
    assert_eq!(
        shell_dashboard_dispatch_queue_lanes.schema_version,
        BERKELEY_APP_SHELL_DASHBOARD_DISPATCH_QUEUE_LANES_SCHEMA_VERSION
    );
    assert!(shell_dashboard_dispatch_queue_lanes.ready);
    assert_eq!(
        shell_dashboard_dispatch_queue_lanes
            .headline_dispatch_queue_item_id
            .as_deref(),
        Some("dashboard.dispatch-queue.status")
    );
    assert_eq!(
        shell_dashboard_dispatch_queue_lanes
            .active_lane_id
            .as_deref(),
        Some("dashboard.dispatch-queue-lane.queued")
    );
    assert_eq!(
        shell_dashboard_dispatch_queue_lanes
            .attention_lane_id
            .as_deref(),
        None
    );
    assert_eq!(shell_dashboard_dispatch_queue_lanes.lane_count, 3);
    assert_eq!(
        shell_dashboard_dispatch_queue_lanes.lanes[0].queue_state,
        "queued"
    );
    assert_eq!(
        shell_dashboard_dispatch_queue_lanes.lanes[0].dispatch_queue_item_ids,
        vec![
            "dashboard.dispatch-queue.status".to_string(),
            "dashboard.dispatch-queue.metrics".to_string()
        ]
    );
    assert!(shell_dashboard_dispatch_queue_lanes.lanes[0].selected);
    assert!(shell_dashboard_dispatch_queue_lanes.lanes[0].default_dispatch);
    assert_eq!(
        shell_dashboard_dispatch_queue_lanes.lanes[1].queue_state,
        "blocked"
    );
    assert_eq!(
        shell_dashboard_dispatch_queue_lanes.lanes[1].dispatch_queue_item_ids,
        vec!["dashboard.dispatch-queue.attention".to_string()]
    );
    assert_eq!(
        shell_dashboard_dispatch_queue_lanes.lanes[2].queue_state,
        "attention"
    );
    assert_eq!(
        shell_dashboard_dispatch_queue_lanes.lanes[2].dispatch_queue_item_count,
        0
    );
    assert_eq!(
        shell_dashboard_dispatch_queue_lanes.dispatch_queue_lanes_capability_id,
        "app-shell-dashboard-dispatch-queue-lanes-json"
    );

    let shell_dashboard_dispatch_queue_lanes_payload: serde_json::Value = serde_json::from_str(
        &app.run_app_shell_dashboard_dispatch_queue_lanes_json(BerkeleyAppPersistedEditorState {
            selected_syntax_card_index: Some(3),
            active_command_id: Some("analysis.3.inspect-waveform".to_string()),
        })
        .unwrap(),
    )
    .expect("shell dashboard dispatch queue lanes JSON should parse");
    assert_eq!(
        shell_dashboard_dispatch_queue_lanes_payload["activeLaneId"],
        "dashboard.dispatch-queue-lane.queued"
    );
    assert!(shell_dashboard_dispatch_queue_lanes_payload["attentionLaneId"].is_null());
    assert_eq!(
        shell_dashboard_dispatch_queue_lanes_payload["lanes"][0]["dispatchQueueItemIds"][1],
        "dashboard.dispatch-queue.metrics"
    );
    assert_eq!(
        shell_dashboard_dispatch_queue_lanes_payload["lanes"][1]["dispatchQueueItemIds"][0],
        "dashboard.dispatch-queue.attention"
    );
    assert_eq!(
        shell_dashboard_dispatch_queue_lanes_payload["dispatchQueueLanesCapabilityId"],
        "app-shell-dashboard-dispatch-queue-lanes-json"
    );

    let shell_dashboard_dispatch_queue_lane_tabs = app
        .run_app_shell_dashboard_dispatch_queue_lane_tabs(BerkeleyAppPersistedEditorState {
            selected_syntax_card_index: Some(3),
            active_command_id: Some("analysis.3.inspect-waveform".to_string()),
        })
        .expect("shell dashboard dispatch queue lane tabs should execute");
    assert_eq!(
        shell_dashboard_dispatch_queue_lane_tabs.schema_version,
        BERKELEY_APP_SHELL_DASHBOARD_DISPATCH_QUEUE_LANE_TABS_SCHEMA_VERSION
    );
    assert!(shell_dashboard_dispatch_queue_lane_tabs.ready);
    assert_eq!(
        shell_dashboard_dispatch_queue_lane_tabs
            .active_lane_id
            .as_deref(),
        Some("dashboard.dispatch-queue-lane.queued")
    );
    assert_eq!(
        shell_dashboard_dispatch_queue_lane_tabs
            .active_tab_id
            .as_deref(),
        Some("dashboard.dispatch-queue-lane-tab.queued")
    );
    assert_eq!(
        shell_dashboard_dispatch_queue_lane_tabs
            .attention_tab_id
            .as_deref(),
        None
    );
    assert_eq!(shell_dashboard_dispatch_queue_lane_tabs.lane_count, 3);
    assert_eq!(shell_dashboard_dispatch_queue_lane_tabs.tab_count, 3);
    assert_eq!(
        shell_dashboard_dispatch_queue_lane_tabs.enabled_tab_count,
        2
    );
    assert_eq!(
        shell_dashboard_dispatch_queue_lane_tabs.disabled_tab_count,
        1
    );
    assert_eq!(
        shell_dashboard_dispatch_queue_lane_tabs.tabs[0].id,
        "dashboard.dispatch-queue-lane-tab.queued"
    );
    assert_eq!(
        shell_dashboard_dispatch_queue_lane_tabs.tabs[0].lane_id,
        "dashboard.dispatch-queue-lane.queued"
    );
    assert!(shell_dashboard_dispatch_queue_lane_tabs.tabs[0].active);
    assert!(shell_dashboard_dispatch_queue_lane_tabs.tabs[0].selected);
    assert!(shell_dashboard_dispatch_queue_lane_tabs.tabs[0].default_dispatch);
    assert!(!shell_dashboard_dispatch_queue_lane_tabs.tabs[2].attention);
    assert!(shell_dashboard_dispatch_queue_lane_tabs.tabs[2].disabled);
    assert_eq!(
        shell_dashboard_dispatch_queue_lane_tabs.dispatch_queue_lane_tabs_capability_id,
        "app-shell-dashboard-dispatch-queue-lane-tabs-json"
    );

    let shell_dashboard_dispatch_queue_lane_tabs_payload: serde_json::Value = serde_json::from_str(
        &app.run_app_shell_dashboard_dispatch_queue_lane_tabs_json(
            BerkeleyAppPersistedEditorState {
                selected_syntax_card_index: Some(3),
                active_command_id: Some("analysis.3.inspect-waveform".to_string()),
            },
        )
        .unwrap(),
    )
    .expect("shell dashboard dispatch queue lane tabs JSON should parse");
    assert_eq!(
        shell_dashboard_dispatch_queue_lane_tabs_payload["activeTabId"],
        "dashboard.dispatch-queue-lane-tab.queued"
    );
    assert_eq!(
        shell_dashboard_dispatch_queue_lane_tabs_payload["tabs"][0]["laneId"],
        "dashboard.dispatch-queue-lane.queued"
    );
    assert_eq!(
        shell_dashboard_dispatch_queue_lane_tabs_payload["tabs"][2]["disabled"],
        true
    );
    assert_eq!(
        shell_dashboard_dispatch_queue_lane_tabs_payload["dispatchQueueLaneTabsCapabilityId"],
        "app-shell-dashboard-dispatch-queue-lane-tabs-json"
    );

    let shell_dashboard_dispatch_queue_lane_tab_panels = app
        .run_app_shell_dashboard_dispatch_queue_lane_tab_panels(BerkeleyAppPersistedEditorState {
            selected_syntax_card_index: Some(3),
            active_command_id: Some("analysis.3.inspect-waveform".to_string()),
        })
        .expect("shell dashboard dispatch queue lane tab panels should execute");
    assert_eq!(
        shell_dashboard_dispatch_queue_lane_tab_panels.schema_version,
        BERKELEY_APP_SHELL_DASHBOARD_DISPATCH_QUEUE_LANE_TAB_PANELS_SCHEMA_VERSION
    );
    assert!(shell_dashboard_dispatch_queue_lane_tab_panels.ready);
    assert_eq!(
        shell_dashboard_dispatch_queue_lane_tab_panels
            .active_tab_id
            .as_deref(),
        Some("dashboard.dispatch-queue-lane-tab.queued")
    );
    assert_eq!(
        shell_dashboard_dispatch_queue_lane_tab_panels
            .active_panel_id
            .as_deref(),
        Some("dashboard.dispatch-queue-lane-tab-panel.queued")
    );
    assert_eq!(
        shell_dashboard_dispatch_queue_lane_tab_panels
            .attention_panel_id
            .as_deref(),
        None
    );
    assert_eq!(
        shell_dashboard_dispatch_queue_lane_tab_panels.panel_count,
        3
    );
    assert_eq!(
        shell_dashboard_dispatch_queue_lane_tab_panels.enabled_panel_count,
        2
    );
    assert_eq!(
        shell_dashboard_dispatch_queue_lane_tab_panels.disabled_panel_count,
        1
    );
    assert_eq!(
        shell_dashboard_dispatch_queue_lane_tab_panels.empty_panel_count,
        1
    );
    assert_eq!(
        shell_dashboard_dispatch_queue_lane_tab_panels.panels[0].id,
        "dashboard.dispatch-queue-lane-tab-panel.queued"
    );
    assert_eq!(
        shell_dashboard_dispatch_queue_lane_tab_panels.panels[0].tab_id,
        "dashboard.dispatch-queue-lane-tab.queued"
    );
    assert!(shell_dashboard_dispatch_queue_lane_tab_panels.panels[0].active);
    assert!(!shell_dashboard_dispatch_queue_lane_tab_panels.panels[0].empty);
    assert!(shell_dashboard_dispatch_queue_lane_tab_panels.panels[2].disabled);
    assert!(shell_dashboard_dispatch_queue_lane_tab_panels.panels[2].empty);
    assert_eq!(
        shell_dashboard_dispatch_queue_lane_tab_panels.panels[2]
            .empty_message
            .as_deref(),
        Some("No attention dispatches")
    );
    assert_eq!(
        shell_dashboard_dispatch_queue_lane_tab_panels.dispatch_queue_lane_tab_panels_capability_id,
        "app-shell-dashboard-dispatch-queue-lane-tab-panels-json"
    );

    let shell_dashboard_dispatch_queue_lane_tab_panels_payload: serde_json::Value =
        serde_json::from_str(
            &app.run_app_shell_dashboard_dispatch_queue_lane_tab_panels_json(
                BerkeleyAppPersistedEditorState {
                    selected_syntax_card_index: Some(3),
                    active_command_id: Some("analysis.3.inspect-waveform".to_string()),
                },
            )
            .unwrap(),
        )
        .expect("shell dashboard dispatch queue lane tab panels JSON should parse");
    assert_eq!(
        shell_dashboard_dispatch_queue_lane_tab_panels_payload["activePanelId"],
        "dashboard.dispatch-queue-lane-tab-panel.queued"
    );
    assert_eq!(
        shell_dashboard_dispatch_queue_lane_tab_panels_payload["panels"][0]["tabId"],
        "dashboard.dispatch-queue-lane-tab.queued"
    );
    assert_eq!(
        shell_dashboard_dispatch_queue_lane_tab_panels_payload["panels"][2]["emptyMessage"],
        "No attention dispatches"
    );
    assert_eq!(
        shell_dashboard_dispatch_queue_lane_tab_panels_payload
            ["dispatchQueueLaneTabPanelsCapabilityId"],
        "app-shell-dashboard-dispatch-queue-lane-tab-panels-json"
    );

    let shell_dashboard_dispatch_queue_lane_tab_panel_cards = app
        .run_app_shell_dashboard_dispatch_queue_lane_tab_panel_cards(
            BerkeleyAppPersistedEditorState {
                selected_syntax_card_index: Some(3),
                active_command_id: Some("analysis.3.inspect-waveform".to_string()),
            },
        )
        .expect("shell dashboard dispatch queue lane tab panel cards should execute");
    assert_eq!(
        shell_dashboard_dispatch_queue_lane_tab_panel_cards.schema_version,
        BERKELEY_APP_SHELL_DASHBOARD_DISPATCH_QUEUE_LANE_TAB_PANEL_CARDS_SCHEMA_VERSION
    );
    assert!(shell_dashboard_dispatch_queue_lane_tab_panel_cards.ready);
    assert_eq!(
        shell_dashboard_dispatch_queue_lane_tab_panel_cards
            .active_panel_card_id
            .as_deref(),
        Some("dashboard.dispatch-queue-lane-tab-panel-card.queued")
    );
    assert_eq!(
        shell_dashboard_dispatch_queue_lane_tab_panel_cards
            .attention_panel_card_id
            .as_deref(),
        None
    );
    assert_eq!(
        shell_dashboard_dispatch_queue_lane_tab_panel_cards.panel_card_count,
        3
    );
    assert_eq!(
        shell_dashboard_dispatch_queue_lane_tab_panel_cards.enabled_panel_card_count,
        2
    );
    assert_eq!(
        shell_dashboard_dispatch_queue_lane_tab_panel_cards.disabled_panel_card_count,
        1
    );
    assert_eq!(
        shell_dashboard_dispatch_queue_lane_tab_panel_cards.empty_panel_card_count,
        1
    );
    assert_eq!(
        shell_dashboard_dispatch_queue_lane_tab_panel_cards.panel_cards[0].id,
        "dashboard.dispatch-queue-lane-tab-panel-card.queued"
    );
    assert_eq!(
        shell_dashboard_dispatch_queue_lane_tab_panel_cards.panel_cards[0].panel_id,
        "dashboard.dispatch-queue-lane-tab-panel.queued"
    );
    assert_eq!(
        shell_dashboard_dispatch_queue_lane_tab_panel_cards.panel_cards[0].summary,
        "2 queued dispatches"
    );
    assert_eq!(
        shell_dashboard_dispatch_queue_lane_tab_panel_cards.panel_cards[0].badge_count,
        2
    );
    assert!(shell_dashboard_dispatch_queue_lane_tab_panel_cards.panel_cards[0].active);
    assert!(shell_dashboard_dispatch_queue_lane_tab_panel_cards.panel_cards[2].empty);
    assert_eq!(
        shell_dashboard_dispatch_queue_lane_tab_panel_cards.panel_cards[2].summary,
        "No attention dispatches"
    );
    assert_eq!(
        shell_dashboard_dispatch_queue_lane_tab_panel_cards
            .dispatch_queue_lane_tab_panel_cards_capability_id,
        "app-shell-dashboard-dispatch-queue-lane-tab-panel-cards-json"
    );

    let shell_dashboard_dispatch_queue_lane_tab_panel_cards_payload: serde_json::Value =
        serde_json::from_str(
            &app.run_app_shell_dashboard_dispatch_queue_lane_tab_panel_cards_json(
                BerkeleyAppPersistedEditorState {
                    selected_syntax_card_index: Some(3),
                    active_command_id: Some("analysis.3.inspect-waveform".to_string()),
                },
            )
            .unwrap(),
        )
        .expect("shell dashboard dispatch queue lane tab panel cards JSON should parse");
    assert_eq!(
        shell_dashboard_dispatch_queue_lane_tab_panel_cards_payload["activePanelCardId"],
        "dashboard.dispatch-queue-lane-tab-panel-card.queued"
    );
    assert_eq!(
        shell_dashboard_dispatch_queue_lane_tab_panel_cards_payload["panelCards"][0]["summary"],
        "2 queued dispatches"
    );
    assert_eq!(
        shell_dashboard_dispatch_queue_lane_tab_panel_cards_payload["panelCards"][2]
            ["emptyMessage"],
        "No attention dispatches"
    );
    assert_eq!(
        shell_dashboard_dispatch_queue_lane_tab_panel_cards_payload
            ["dispatchQueueLaneTabPanelCardsCapabilityId"],
        "app-shell-dashboard-dispatch-queue-lane-tab-panel-cards-json"
    );

    let shell_dashboard_dispatch_queue_lane_tab_panel_card_actions = app
        .run_app_shell_dashboard_dispatch_queue_lane_tab_panel_card_actions(
            BerkeleyAppPersistedEditorState {
                selected_syntax_card_index: Some(3),
                active_command_id: Some("analysis.3.inspect-waveform".to_string()),
            },
        )
        .expect("shell dashboard dispatch queue lane tab panel card actions should execute");
    assert_eq!(
        shell_dashboard_dispatch_queue_lane_tab_panel_card_actions.schema_version,
        BERKELEY_APP_SHELL_DASHBOARD_DISPATCH_QUEUE_LANE_TAB_PANEL_CARD_ACTIONS_SCHEMA_VERSION
    );
    assert!(shell_dashboard_dispatch_queue_lane_tab_panel_card_actions.ready);
    assert_eq!(
        shell_dashboard_dispatch_queue_lane_tab_panel_card_actions
            .active_panel_card_action_id
            .as_deref(),
        Some("dashboard.dispatch-queue-lane-tab-panel-card-action.queued")
    );
    assert_eq!(
        shell_dashboard_dispatch_queue_lane_tab_panel_card_actions
            .attention_panel_card_action_id
            .as_deref(),
        None
    );
    assert_eq!(
        shell_dashboard_dispatch_queue_lane_tab_panel_card_actions.panel_card_action_count,
        3
    );
    assert_eq!(
        shell_dashboard_dispatch_queue_lane_tab_panel_card_actions.enabled_panel_card_action_count,
        2
    );
    assert_eq!(
        shell_dashboard_dispatch_queue_lane_tab_panel_card_actions.disabled_panel_card_action_count,
        1
    );
    assert_eq!(
        shell_dashboard_dispatch_queue_lane_tab_panel_card_actions.empty_panel_card_action_count,
        1
    );
    assert_eq!(
        shell_dashboard_dispatch_queue_lane_tab_panel_card_actions.panel_card_actions[0].id,
        "dashboard.dispatch-queue-lane-tab-panel-card-action.queued"
    );
    assert_eq!(
        shell_dashboard_dispatch_queue_lane_tab_panel_card_actions.panel_card_actions[0]
            .panel_card_id,
        "dashboard.dispatch-queue-lane-tab-panel-card.queued"
    );
    assert_eq!(
        shell_dashboard_dispatch_queue_lane_tab_panel_card_actions.panel_card_actions[0].label,
        "Open queued dispatches"
    );
    assert_eq!(
        shell_dashboard_dispatch_queue_lane_tab_panel_card_actions.panel_card_actions[0].target,
        "dashboard.dispatch-queue-lane-tab-panel-card.queued"
    );
    assert!(
        shell_dashboard_dispatch_queue_lane_tab_panel_card_actions.panel_card_actions[0].enabled
    );
    assert!(
        shell_dashboard_dispatch_queue_lane_tab_panel_card_actions.panel_card_actions[0].primary
    );
    assert_eq!(
        shell_dashboard_dispatch_queue_lane_tab_panel_card_actions.panel_card_actions[2]
            .disabled_reason
            .as_deref(),
        Some("No attention dispatches")
    );
    assert_eq!(
        shell_dashboard_dispatch_queue_lane_tab_panel_card_actions
            .dispatch_queue_lane_tab_panel_card_actions_capability_id,
        "app-shell-dashboard-dispatch-queue-lane-tab-panel-card-actions-json"
    );

    let shell_dashboard_dispatch_queue_lane_tab_panel_card_actions_payload: serde_json::Value =
        serde_json::from_str(
            &app.run_app_shell_dashboard_dispatch_queue_lane_tab_panel_card_actions_json(
                BerkeleyAppPersistedEditorState {
                    selected_syntax_card_index: Some(3),
                    active_command_id: Some("analysis.3.inspect-waveform".to_string()),
                },
            )
            .unwrap(),
        )
        .expect("shell dashboard dispatch queue lane tab panel card actions JSON should parse");
    assert_eq!(
        shell_dashboard_dispatch_queue_lane_tab_panel_card_actions_payload
            ["activePanelCardActionId"],
        "dashboard.dispatch-queue-lane-tab-panel-card-action.queued"
    );
    assert_eq!(
        shell_dashboard_dispatch_queue_lane_tab_panel_card_actions_payload["panelCardActions"][0]
            ["label"],
        "Open queued dispatches"
    );
    assert_eq!(
        shell_dashboard_dispatch_queue_lane_tab_panel_card_actions_payload["panelCardActions"][2]
            ["disabledReason"],
        "No attention dispatches"
    );
    assert_eq!(
        shell_dashboard_dispatch_queue_lane_tab_panel_card_actions_payload
            ["dispatchQueueLaneTabPanelCardActionsCapabilityId"],
        "app-shell-dashboard-dispatch-queue-lane-tab-panel-card-actions-json"
    );

    let shell_dashboard_dispatch_queue_lane_tab_panel_card_action_menu = app
        .run_app_shell_dashboard_dispatch_queue_lane_tab_panel_card_action_menu(
            BerkeleyAppPersistedEditorState {
                selected_syntax_card_index: Some(3),
                active_command_id: Some("analysis.3.inspect-waveform".to_string()),
            },
        )
        .expect("shell dashboard dispatch queue lane tab panel card action menu should execute");
    assert_eq!(
        shell_dashboard_dispatch_queue_lane_tab_panel_card_action_menu.schema_version,
        BERKELEY_APP_SHELL_DASHBOARD_DISPATCH_QUEUE_LANE_TAB_PANEL_CARD_ACTION_MENU_SCHEMA_VERSION
    );
    assert!(shell_dashboard_dispatch_queue_lane_tab_panel_card_action_menu.ready);
    assert_eq!(
        shell_dashboard_dispatch_queue_lane_tab_panel_card_action_menu
            .active_menu_item_id
            .as_deref(),
        Some("dashboard.dispatch-queue-lane-tab-panel-card-action-menu-item.queued")
    );
    assert_eq!(
        shell_dashboard_dispatch_queue_lane_tab_panel_card_action_menu
            .attention_menu_item_id
            .as_deref(),
        None
    );
    assert_eq!(
        shell_dashboard_dispatch_queue_lane_tab_panel_card_action_menu
            .default_menu_item_id
            .as_deref(),
        Some("dashboard.dispatch-queue-lane-tab-panel-card-action-menu-item.queued")
    );
    assert_eq!(
        shell_dashboard_dispatch_queue_lane_tab_panel_card_action_menu.menu_item_count,
        3
    );
    assert_eq!(
        shell_dashboard_dispatch_queue_lane_tab_panel_card_action_menu.enabled_menu_item_count,
        2
    );
    assert_eq!(
        shell_dashboard_dispatch_queue_lane_tab_panel_card_action_menu.disabled_menu_item_count,
        1
    );
    assert_eq!(
        shell_dashboard_dispatch_queue_lane_tab_panel_card_action_menu.empty_menu_item_count,
        1
    );
    assert_eq!(
        shell_dashboard_dispatch_queue_lane_tab_panel_card_action_menu.menu_items[0].id,
        "dashboard.dispatch-queue-lane-tab-panel-card-action-menu-item.queued"
    );
    assert_eq!(
        shell_dashboard_dispatch_queue_lane_tab_panel_card_action_menu.menu_items[0].action_id,
        "dashboard.dispatch-queue-lane-tab-panel-card-action.queued"
    );
    assert_eq!(
        shell_dashboard_dispatch_queue_lane_tab_panel_card_action_menu.menu_items[0].position,
        0
    );
    assert_eq!(
        shell_dashboard_dispatch_queue_lane_tab_panel_card_action_menu.menu_items[0].label,
        "Open queued dispatches"
    );
    assert!(shell_dashboard_dispatch_queue_lane_tab_panel_card_action_menu.menu_items[0].primary);
    assert_eq!(
        shell_dashboard_dispatch_queue_lane_tab_panel_card_action_menu.menu_items[2]
            .disabled_reason
            .as_deref(),
        Some("No attention dispatches")
    );
    assert_eq!(
        shell_dashboard_dispatch_queue_lane_tab_panel_card_action_menu
            .dispatch_queue_lane_tab_panel_card_action_menu_capability_id,
        "app-shell-dashboard-dispatch-queue-lane-tab-panel-card-action-menu-json"
    );

    let shell_dashboard_dispatch_queue_lane_tab_panel_card_action_menu_payload: serde_json::Value =
        serde_json::from_str(
            &app.run_app_shell_dashboard_dispatch_queue_lane_tab_panel_card_action_menu_json(
                BerkeleyAppPersistedEditorState {
                    selected_syntax_card_index: Some(3),
                    active_command_id: Some("analysis.3.inspect-waveform".to_string()),
                },
            )
            .unwrap(),
        )
        .expect("shell dashboard dispatch queue lane tab panel card action menu JSON should parse");
    assert_eq!(
        shell_dashboard_dispatch_queue_lane_tab_panel_card_action_menu_payload["activeMenuItemId"],
        "dashboard.dispatch-queue-lane-tab-panel-card-action-menu-item.queued"
    );
    assert_eq!(
        shell_dashboard_dispatch_queue_lane_tab_panel_card_action_menu_payload["menuItems"][0]
            ["actionId"],
        "dashboard.dispatch-queue-lane-tab-panel-card-action.queued"
    );
    assert_eq!(
        shell_dashboard_dispatch_queue_lane_tab_panel_card_action_menu_payload["menuItems"][2]
            ["disabledReason"],
        "No attention dispatches"
    );
    assert_eq!(
        shell_dashboard_dispatch_queue_lane_tab_panel_card_action_menu_payload
            ["dispatchQueueLaneTabPanelCardActionMenuCapabilityId"],
        "app-shell-dashboard-dispatch-queue-lane-tab-panel-card-action-menu-json"
    );

    let shell_dashboard_dispatch_queue_lane_tab_panel_card_action_menu_groups = app
        .run_app_shell_dashboard_dispatch_queue_lane_tab_panel_card_action_menu_groups(
            BerkeleyAppPersistedEditorState {
                selected_syntax_card_index: Some(3),
                active_command_id: Some("analysis.3.inspect-waveform".to_string()),
            },
        )
        .expect(
            "shell dashboard dispatch queue lane tab panel card action menu groups should execute",
        );
    assert_eq!(
        shell_dashboard_dispatch_queue_lane_tab_panel_card_action_menu_groups.schema_version,
        BERKELEY_APP_SHELL_DASHBOARD_DISPATCH_QUEUE_LANE_TAB_PANEL_CARD_ACTION_MENU_GROUPS_SCHEMA_VERSION
    );
    assert!(shell_dashboard_dispatch_queue_lane_tab_panel_card_action_menu_groups.ready);
    assert_eq!(
        shell_dashboard_dispatch_queue_lane_tab_panel_card_action_menu_groups
            .active_menu_group_id
            .as_deref(),
        Some("dashboard.dispatch-queue-lane-tab-panel-card-action-menu-group.queued")
    );
    assert_eq!(
        shell_dashboard_dispatch_queue_lane_tab_panel_card_action_menu_groups
            .attention_menu_group_id
            .as_deref(),
        None
    );
    assert_eq!(
        shell_dashboard_dispatch_queue_lane_tab_panel_card_action_menu_groups
            .default_menu_group_id
            .as_deref(),
        Some("dashboard.dispatch-queue-lane-tab-panel-card-action-menu-group.queued")
    );
    assert_eq!(
        shell_dashboard_dispatch_queue_lane_tab_panel_card_action_menu_groups.menu_group_count,
        3
    );
    assert_eq!(
        shell_dashboard_dispatch_queue_lane_tab_panel_card_action_menu_groups
            .enabled_menu_group_count,
        2
    );
    assert_eq!(
        shell_dashboard_dispatch_queue_lane_tab_panel_card_action_menu_groups
            .disabled_menu_group_count,
        1
    );
    assert_eq!(
        shell_dashboard_dispatch_queue_lane_tab_panel_card_action_menu_groups
            .empty_menu_group_count,
        1
    );
    assert_eq!(
        shell_dashboard_dispatch_queue_lane_tab_panel_card_action_menu_groups.menu_groups[0].id,
        "dashboard.dispatch-queue-lane-tab-panel-card-action-menu-group.queued"
    );
    assert_eq!(
        shell_dashboard_dispatch_queue_lane_tab_panel_card_action_menu_groups.menu_groups[0].label,
        "Queued dispatches"
    );
    assert_eq!(
        shell_dashboard_dispatch_queue_lane_tab_panel_card_action_menu_groups.menu_groups[0]
            .menu_item_ids[0],
        "dashboard.dispatch-queue-lane-tab-panel-card-action-menu-item.queued"
    );
    assert_eq!(
        shell_dashboard_dispatch_queue_lane_tab_panel_card_action_menu_groups.menu_groups[0]
            .action_ids[0],
        "dashboard.dispatch-queue-lane-tab-panel-card-action.queued"
    );
    assert_eq!(
        shell_dashboard_dispatch_queue_lane_tab_panel_card_action_menu_groups.menu_groups[2]
            .disabled_reason
            .as_deref(),
        Some("No attention dispatches")
    );
    assert_eq!(
        shell_dashboard_dispatch_queue_lane_tab_panel_card_action_menu_groups
            .dispatch_queue_lane_tab_panel_card_action_menu_groups_capability_id,
        "app-shell-dashboard-dispatch-queue-lane-tab-panel-card-action-menu-groups-json"
    );

    let shell_dashboard_dispatch_queue_lane_tab_panel_card_action_menu_groups_payload:
        serde_json::Value = serde_json::from_str(
        &app.run_app_shell_dashboard_dispatch_queue_lane_tab_panel_card_action_menu_groups_json(
            BerkeleyAppPersistedEditorState {
                selected_syntax_card_index: Some(3),
                active_command_id: Some("analysis.3.inspect-waveform".to_string()),
            },
        )
        .unwrap(),
    )
    .expect(
        "shell dashboard dispatch queue lane tab panel card action menu groups JSON should parse",
    );
    assert_eq!(
        shell_dashboard_dispatch_queue_lane_tab_panel_card_action_menu_groups_payload
            ["activeMenuGroupId"],
        "dashboard.dispatch-queue-lane-tab-panel-card-action-menu-group.queued"
    );
    assert_eq!(
        shell_dashboard_dispatch_queue_lane_tab_panel_card_action_menu_groups_payload["menuGroups"]
            [0]["menuItemIds"][0],
        "dashboard.dispatch-queue-lane-tab-panel-card-action-menu-item.queued"
    );
    assert_eq!(
        shell_dashboard_dispatch_queue_lane_tab_panel_card_action_menu_groups_payload["menuGroups"]
            [2]["disabledReason"],
        "No attention dispatches"
    );
    assert_eq!(
        shell_dashboard_dispatch_queue_lane_tab_panel_card_action_menu_groups_payload
            ["dispatchQueueLaneTabPanelCardActionMenuGroupsCapabilityId"],
        "app-shell-dashboard-dispatch-queue-lane-tab-panel-card-action-menu-groups-json"
    );

    let shell_dashboard_dispatch_queue_lane_tab_panel_card_action_menu_group_shortcuts = app
        .run_app_shell_dashboard_dispatch_queue_lane_tab_panel_card_action_menu_group_shortcuts(
            BerkeleyAppPersistedEditorState {
                selected_syntax_card_index: Some(3),
                active_command_id: Some("analysis.3.inspect-waveform".to_string()),
            },
        )
        .expect(
            "shell dashboard dispatch queue lane tab panel card action menu group shortcuts should execute",
        );
    assert_eq!(
        shell_dashboard_dispatch_queue_lane_tab_panel_card_action_menu_group_shortcuts
            .schema_version,
        BERKELEY_APP_SHELL_DASHBOARD_DISPATCH_QUEUE_LANE_TAB_PANEL_CARD_ACTION_MENU_GROUP_SHORTCUTS_SCHEMA_VERSION
    );
    assert!(shell_dashboard_dispatch_queue_lane_tab_panel_card_action_menu_group_shortcuts.ready);
    assert_eq!(
        shell_dashboard_dispatch_queue_lane_tab_panel_card_action_menu_group_shortcuts
            .active_menu_group_shortcut_id
            .as_deref(),
        Some("dashboard.dispatch-queue-lane-tab-panel-card-action-menu-group-shortcut.queued")
    );
    assert_eq!(
        shell_dashboard_dispatch_queue_lane_tab_panel_card_action_menu_group_shortcuts
            .default_menu_group_shortcut_id
            .as_deref(),
        Some("dashboard.dispatch-queue-lane-tab-panel-card-action-menu-group-shortcut.queued")
    );
    assert_eq!(
        shell_dashboard_dispatch_queue_lane_tab_panel_card_action_menu_group_shortcuts
            .menu_group_shortcut_count,
        3
    );
    assert_eq!(
        shell_dashboard_dispatch_queue_lane_tab_panel_card_action_menu_group_shortcuts
            .enabled_menu_group_shortcut_count,
        2
    );
    assert_eq!(
        shell_dashboard_dispatch_queue_lane_tab_panel_card_action_menu_group_shortcuts
            .disabled_menu_group_shortcut_count,
        1
    );
    assert_eq!(
        shell_dashboard_dispatch_queue_lane_tab_panel_card_action_menu_group_shortcuts
            .empty_menu_group_shortcut_count,
        1
    );
    assert_eq!(
        shell_dashboard_dispatch_queue_lane_tab_panel_card_action_menu_group_shortcuts
            .menu_group_shortcuts[0]
            .accelerator,
        "mod+1"
    );
    assert_eq!(
        shell_dashboard_dispatch_queue_lane_tab_panel_card_action_menu_group_shortcuts
            .menu_group_shortcuts[0]
            .target,
        "dashboard.dispatch-queue-lane-tab-panel-card-action-menu-group.queued"
    );
    assert_eq!(
        shell_dashboard_dispatch_queue_lane_tab_panel_card_action_menu_group_shortcuts
            .menu_group_shortcuts[2]
            .disabled_reason
            .as_deref(),
        Some("No attention dispatches")
    );
    assert_eq!(
        shell_dashboard_dispatch_queue_lane_tab_panel_card_action_menu_group_shortcuts
            .dispatch_queue_lane_tab_panel_card_action_menu_group_shortcuts_capability_id,
        "app-shell-dashboard-dispatch-queue-lane-tab-panel-card-action-menu-group-shortcuts-json"
    );

    let shell_dashboard_dispatch_queue_lane_tab_panel_card_action_menu_group_shortcuts_payload:
        serde_json::Value = serde_json::from_str(
        &app.run_app_shell_dashboard_dispatch_queue_lane_tab_panel_card_action_menu_group_shortcuts_json(
            BerkeleyAppPersistedEditorState {
                selected_syntax_card_index: Some(3),
                active_command_id: Some("analysis.3.inspect-waveform".to_string()),
            },
        )
        .unwrap(),
    )
    .expect(
        "shell dashboard dispatch queue lane tab panel card action menu group shortcuts JSON should parse",
    );
    assert_eq!(
        shell_dashboard_dispatch_queue_lane_tab_panel_card_action_menu_group_shortcuts_payload
            ["activeMenuGroupShortcutId"],
        "dashboard.dispatch-queue-lane-tab-panel-card-action-menu-group-shortcut.queued"
    );
    assert_eq!(
        shell_dashboard_dispatch_queue_lane_tab_panel_card_action_menu_group_shortcuts_payload
            ["menuGroupShortcuts"][0]["accelerator"],
        "mod+1"
    );
    assert_eq!(
        shell_dashboard_dispatch_queue_lane_tab_panel_card_action_menu_group_shortcuts_payload
            ["menuGroupShortcuts"][2]["disabledReason"],
        "No attention dispatches"
    );
    assert_eq!(
        shell_dashboard_dispatch_queue_lane_tab_panel_card_action_menu_group_shortcuts_payload
            ["dispatchQueueLaneTabPanelCardActionMenuGroupShortcutsCapabilityId"],
        "app-shell-dashboard-dispatch-queue-lane-tab-panel-card-action-menu-group-shortcuts-json"
    );

    let shell_dashboard_dispatch_queue_lane_tab_panel_card_action_menu_group_shortcut_bindings =
        app.run_app_shell_dashboard_dispatch_queue_lane_tab_panel_card_action_menu_group_shortcut_bindings(
            BerkeleyAppPersistedEditorState {
                selected_syntax_card_index: Some(3),
                active_command_id: Some("analysis.3.inspect-waveform".to_string()),
            },
        )
        .expect(
            "shell dashboard dispatch queue lane tab panel card action menu group shortcut bindings should execute",
        );
    assert_eq!(
        shell_dashboard_dispatch_queue_lane_tab_panel_card_action_menu_group_shortcut_bindings
            .schema_version,
        BERKELEY_APP_SHELL_DASHBOARD_DISPATCH_QUEUE_LANE_TAB_PANEL_CARD_ACTION_MENU_GROUP_SHORTCUT_BINDINGS_SCHEMA_VERSION
    );
    assert!(
        shell_dashboard_dispatch_queue_lane_tab_panel_card_action_menu_group_shortcut_bindings
            .ready
    );
    assert_eq!(
        shell_dashboard_dispatch_queue_lane_tab_panel_card_action_menu_group_shortcut_bindings
            .active_menu_group_shortcut_binding_id
            .as_deref(),
        Some(
            "dashboard.dispatch-queue-lane-tab-panel-card-action-menu-group-shortcut-binding.queued"
        )
    );
    assert_eq!(
        shell_dashboard_dispatch_queue_lane_tab_panel_card_action_menu_group_shortcut_bindings
            .default_menu_group_shortcut_binding_id
            .as_deref(),
        Some(
            "dashboard.dispatch-queue-lane-tab-panel-card-action-menu-group-shortcut-binding.queued"
        )
    );
    assert_eq!(
        shell_dashboard_dispatch_queue_lane_tab_panel_card_action_menu_group_shortcut_bindings
            .menu_group_shortcut_binding_count,
        3
    );
    assert_eq!(
        shell_dashboard_dispatch_queue_lane_tab_panel_card_action_menu_group_shortcut_bindings
            .enabled_menu_group_shortcut_binding_count,
        2
    );
    assert_eq!(
        shell_dashboard_dispatch_queue_lane_tab_panel_card_action_menu_group_shortcut_bindings
            .disabled_menu_group_shortcut_binding_count,
        1
    );
    assert_eq!(
        shell_dashboard_dispatch_queue_lane_tab_panel_card_action_menu_group_shortcut_bindings
            .empty_menu_group_shortcut_binding_count,
        1
    );
    assert_eq!(
        shell_dashboard_dispatch_queue_lane_tab_panel_card_action_menu_group_shortcut_bindings
            .menu_group_shortcut_bindings[0]
            .accelerator,
        "mod+1"
    );
    assert_eq!(
        shell_dashboard_dispatch_queue_lane_tab_panel_card_action_menu_group_shortcut_bindings
            .menu_group_shortcut_bindings[0]
            .shortcut_id,
        "dashboard.dispatch-queue-lane-tab-panel-card-action-menu-group-shortcut.queued"
    );
    assert_eq!(
        shell_dashboard_dispatch_queue_lane_tab_panel_card_action_menu_group_shortcut_bindings
            .menu_group_shortcut_bindings[0]
            .command_id,
        "berkeley.app-shell.dashboard.dispatch-queue.menu-group-shortcut.queued"
    );
    assert_eq!(
        shell_dashboard_dispatch_queue_lane_tab_panel_card_action_menu_group_shortcut_bindings
            .menu_group_shortcut_bindings[0]
            .scope,
        "dashboard.dispatch-queue"
    );
    assert_eq!(
        shell_dashboard_dispatch_queue_lane_tab_panel_card_action_menu_group_shortcut_bindings
            .menu_group_shortcut_bindings[0]
            .target_kind,
        "menu-group"
    );
    assert_eq!(
        shell_dashboard_dispatch_queue_lane_tab_panel_card_action_menu_group_shortcut_bindings
            .menu_group_shortcut_bindings[2]
            .disabled_reason
            .as_deref(),
        Some("No attention dispatches")
    );
    assert_eq!(
        shell_dashboard_dispatch_queue_lane_tab_panel_card_action_menu_group_shortcut_bindings
            .dispatch_queue_lane_tab_panel_card_action_menu_group_shortcut_bindings_capability_id,
        "app-shell-dashboard-dispatch-queue-lane-tab-panel-card-action-menu-group-shortcut-bindings-json"
    );

    let shell_dashboard_dispatch_queue_lane_tab_panel_card_action_menu_group_shortcut_bindings_payload:
        serde_json::Value = serde_json::from_str(
        &app.run_app_shell_dashboard_dispatch_queue_lane_tab_panel_card_action_menu_group_shortcut_bindings_json(
            BerkeleyAppPersistedEditorState {
                selected_syntax_card_index: Some(3),
                active_command_id: Some("analysis.3.inspect-waveform".to_string()),
            },
        )
        .unwrap(),
    )
    .expect(
        "shell dashboard dispatch queue lane tab panel card action menu group shortcut bindings JSON should parse",
    );
    assert_eq!(
        shell_dashboard_dispatch_queue_lane_tab_panel_card_action_menu_group_shortcut_bindings_payload
            ["activeMenuGroupShortcutBindingId"],
        "dashboard.dispatch-queue-lane-tab-panel-card-action-menu-group-shortcut-binding.queued"
    );
    assert_eq!(
        shell_dashboard_dispatch_queue_lane_tab_panel_card_action_menu_group_shortcut_bindings_payload
            ["menuGroupShortcutBindings"][0]["commandId"],
        "berkeley.app-shell.dashboard.dispatch-queue.menu-group-shortcut.queued"
    );
    assert_eq!(
        shell_dashboard_dispatch_queue_lane_tab_panel_card_action_menu_group_shortcut_bindings_payload
            ["menuGroupShortcutBindings"][2]["disabledReason"],
        "No attention dispatches"
    );
    assert_eq!(
        shell_dashboard_dispatch_queue_lane_tab_panel_card_action_menu_group_shortcut_bindings_payload
            ["dispatchQueueLaneTabPanelCardActionMenuGroupShortcutBindingsCapabilityId"],
        "app-shell-dashboard-dispatch-queue-lane-tab-panel-card-action-menu-group-shortcut-bindings-json"
    );

    let shell_dashboard_dispatch_queue_lane_tab_panel_card_action_menu_group_shortcut_command_registry =
        app.run_app_shell_dashboard_dispatch_queue_lane_tab_panel_card_action_menu_group_shortcut_command_registry(
            BerkeleyAppPersistedEditorState {
                selected_syntax_card_index: Some(3),
                active_command_id: Some("analysis.3.inspect-waveform".to_string()),
            },
        )
        .expect(
            "shell dashboard dispatch queue lane tab panel card action menu group shortcut command registry should execute",
        );
    assert_eq!(
        shell_dashboard_dispatch_queue_lane_tab_panel_card_action_menu_group_shortcut_command_registry
            .schema_version,
        BERKELEY_APP_SHELL_DASHBOARD_DISPATCH_QUEUE_LANE_TAB_PANEL_CARD_ACTION_MENU_GROUP_SHORTCUT_COMMAND_REGISTRY_SCHEMA_VERSION
    );
    assert!(
        shell_dashboard_dispatch_queue_lane_tab_panel_card_action_menu_group_shortcut_command_registry
            .ready
    );
    assert_eq!(
        shell_dashboard_dispatch_queue_lane_tab_panel_card_action_menu_group_shortcut_command_registry
            .registry_id,
        "dashboard.dispatch-queue.shortcut-command-registry"
    );
    assert_eq!(
        shell_dashboard_dispatch_queue_lane_tab_panel_card_action_menu_group_shortcut_command_registry
            .active_menu_group_shortcut_command_id
            .as_deref(),
        Some("dashboard.dispatch-queue-lane-tab-panel-card-action-menu-group-shortcut-command.queued")
    );
    assert_eq!(
        shell_dashboard_dispatch_queue_lane_tab_panel_card_action_menu_group_shortcut_command_registry
            .default_menu_group_shortcut_command_id
            .as_deref(),
        Some("dashboard.dispatch-queue-lane-tab-panel-card-action-menu-group-shortcut-command.queued")
    );
    assert_eq!(
        shell_dashboard_dispatch_queue_lane_tab_panel_card_action_menu_group_shortcut_command_registry
            .menu_group_shortcut_command_count,
        3
    );
    assert_eq!(
        shell_dashboard_dispatch_queue_lane_tab_panel_card_action_menu_group_shortcut_command_registry
            .enabled_menu_group_shortcut_command_count,
        2
    );
    assert_eq!(
        shell_dashboard_dispatch_queue_lane_tab_panel_card_action_menu_group_shortcut_command_registry
            .visible_menu_group_shortcut_command_count,
        3
    );
    assert_eq!(
        shell_dashboard_dispatch_queue_lane_tab_panel_card_action_menu_group_shortcut_command_registry
            .menu_group_shortcut_commands[0]
            .binding_id,
        "dashboard.dispatch-queue-lane-tab-panel-card-action-menu-group-shortcut-binding.queued"
    );
    assert_eq!(
        shell_dashboard_dispatch_queue_lane_tab_panel_card_action_menu_group_shortcut_command_registry
            .menu_group_shortcut_commands[0]
            .command_id,
        "berkeley.app-shell.dashboard.dispatch-queue.menu-group-shortcut.queued"
    );
    assert_eq!(
        shell_dashboard_dispatch_queue_lane_tab_panel_card_action_menu_group_shortcut_command_registry
            .menu_group_shortcut_commands[0]
            .handler_id,
        "handler.berkeley.app-shell.dashboard.dispatch-queue.menu-group-shortcut.queued"
    );
    assert_eq!(
        shell_dashboard_dispatch_queue_lane_tab_panel_card_action_menu_group_shortcut_command_registry
            .menu_group_shortcut_commands[0]
            .invocation_kind,
        "shortcut-binding"
    );
    assert_eq!(
        shell_dashboard_dispatch_queue_lane_tab_panel_card_action_menu_group_shortcut_command_registry
            .menu_group_shortcut_commands[2]
            .disabled_reason
            .as_deref(),
        Some("No attention dispatches")
    );
    assert_eq!(
        shell_dashboard_dispatch_queue_lane_tab_panel_card_action_menu_group_shortcut_command_registry
            .dispatch_queue_lane_tab_panel_card_action_menu_group_shortcut_command_registry_capability_id,
        "app-shell-dashboard-dispatch-queue-lane-tab-panel-card-action-menu-group-shortcut-command-registry-json"
    );

    let shell_dashboard_dispatch_queue_lane_tab_panel_card_action_menu_group_shortcut_command_registry_payload:
        serde_json::Value = serde_json::from_str(
        &app.run_app_shell_dashboard_dispatch_queue_lane_tab_panel_card_action_menu_group_shortcut_command_registry_json(
            BerkeleyAppPersistedEditorState {
                selected_syntax_card_index: Some(3),
                active_command_id: Some("analysis.3.inspect-waveform".to_string()),
            },
        )
        .unwrap(),
    )
    .expect(
        "shell dashboard dispatch queue lane tab panel card action menu group shortcut command registry JSON should parse",
    );
    assert_eq!(
        shell_dashboard_dispatch_queue_lane_tab_panel_card_action_menu_group_shortcut_command_registry_payload
            ["activeMenuGroupShortcutCommandId"],
        "dashboard.dispatch-queue-lane-tab-panel-card-action-menu-group-shortcut-command.queued"
    );
    assert_eq!(
        shell_dashboard_dispatch_queue_lane_tab_panel_card_action_menu_group_shortcut_command_registry_payload
            ["menuGroupShortcutCommands"][0]["handlerId"],
        "handler.berkeley.app-shell.dashboard.dispatch-queue.menu-group-shortcut.queued"
    );
    assert_eq!(
        shell_dashboard_dispatch_queue_lane_tab_panel_card_action_menu_group_shortcut_command_registry_payload
            ["menuGroupShortcutCommands"][2]["disabledReason"],
        "No attention dispatches"
    );
    assert_eq!(
        shell_dashboard_dispatch_queue_lane_tab_panel_card_action_menu_group_shortcut_command_registry_payload
            ["dispatchQueueLaneTabPanelCardActionMenuGroupShortcutCommandRegistryCapabilityId"],
        "app-shell-dashboard-dispatch-queue-lane-tab-panel-card-action-menu-group-shortcut-command-registry-json"
    );

    let shell_dashboard_dispatch_queue_lane_tab_panel_card_action_menu_group_shortcut_command_palette =
        app.run_app_shell_dashboard_dispatch_queue_lane_tab_panel_card_action_menu_group_shortcut_command_palette(
            BerkeleyAppPersistedEditorState {
                selected_syntax_card_index: Some(3),
                active_command_id: Some("analysis.3.inspect-waveform".to_string()),
            },
        )
        .expect(
            "shell dashboard dispatch queue lane tab panel card action menu group shortcut command palette should execute",
        );
    assert_eq!(
        shell_dashboard_dispatch_queue_lane_tab_panel_card_action_menu_group_shortcut_command_palette
            .schema_version,
        BERKELEY_APP_SHELL_DASHBOARD_DISPATCH_QUEUE_LANE_TAB_PANEL_CARD_ACTION_MENU_GROUP_SHORTCUT_COMMAND_PALETTE_SCHEMA_VERSION
    );
    assert_eq!(
        shell_dashboard_dispatch_queue_lane_tab_panel_card_action_menu_group_shortcut_command_palette
            .palette_id,
        "dashboard.dispatch-queue.shortcut-command-palette"
    );
    assert_eq!(
        shell_dashboard_dispatch_queue_lane_tab_panel_card_action_menu_group_shortcut_command_palette
            .active_menu_group_shortcut_command_palette_item_id
            .as_deref(),
        Some("dashboard.dispatch-queue-lane-tab-panel-card-action-menu-group-shortcut-command-palette-item.queued")
    );
    assert_eq!(
        shell_dashboard_dispatch_queue_lane_tab_panel_card_action_menu_group_shortcut_command_palette
            .default_menu_group_shortcut_command_palette_item_id
            .as_deref(),
        Some("dashboard.dispatch-queue-lane-tab-panel-card-action-menu-group-shortcut-command-palette-item.queued")
    );
    assert_eq!(
        shell_dashboard_dispatch_queue_lane_tab_panel_card_action_menu_group_shortcut_command_palette
            .command_palette_item_count,
        3
    );
    assert_eq!(
        shell_dashboard_dispatch_queue_lane_tab_panel_card_action_menu_group_shortcut_command_palette
            .selectable_command_palette_item_count,
        2
    );
    assert_eq!(
        shell_dashboard_dispatch_queue_lane_tab_panel_card_action_menu_group_shortcut_command_palette
            .visible_command_palette_item_count,
        3
    );
    assert_eq!(
        shell_dashboard_dispatch_queue_lane_tab_panel_card_action_menu_group_shortcut_command_palette
            .command_palette_items[0]
            .command_entry_id,
        "dashboard.dispatch-queue-lane-tab-panel-card-action-menu-group-shortcut-command.queued"
    );
    assert_eq!(
        shell_dashboard_dispatch_queue_lane_tab_panel_card_action_menu_group_shortcut_command_palette
            .command_palette_items[0]
            .palette_id,
        "dashboard.dispatch-queue.shortcut-command-palette"
    );
    assert!(
        shell_dashboard_dispatch_queue_lane_tab_panel_card_action_menu_group_shortcut_command_palette
            .command_palette_items[0]
            .search_text
            .contains("Queued dispatches")
    );
    assert_eq!(
        shell_dashboard_dispatch_queue_lane_tab_panel_card_action_menu_group_shortcut_command_palette
            .command_palette_items[0]
            .keywords[0],
        "queued"
    );
    assert!(
        shell_dashboard_dispatch_queue_lane_tab_panel_card_action_menu_group_shortcut_command_palette
            .command_palette_items[0]
            .selectable
    );
    assert!(
        !shell_dashboard_dispatch_queue_lane_tab_panel_card_action_menu_group_shortcut_command_palette
            .command_palette_items[2]
            .selectable
    );
    assert_eq!(
        shell_dashboard_dispatch_queue_lane_tab_panel_card_action_menu_group_shortcut_command_palette
            .command_palette_items[2]
            .disabled_reason
            .as_deref(),
        Some("No attention dispatches")
    );
    assert_eq!(
        shell_dashboard_dispatch_queue_lane_tab_panel_card_action_menu_group_shortcut_command_palette
            .dispatch_queue_lane_tab_panel_card_action_menu_group_shortcut_command_palette_capability_id,
        "app-shell-dashboard-dispatch-queue-lane-tab-panel-card-action-menu-group-shortcut-command-palette-json"
    );

    let shell_dashboard_dispatch_queue_lane_tab_panel_card_action_menu_group_shortcut_command_palette_payload:
        serde_json::Value = serde_json::from_str(
        &app.run_app_shell_dashboard_dispatch_queue_lane_tab_panel_card_action_menu_group_shortcut_command_palette_json(
            BerkeleyAppPersistedEditorState {
                selected_syntax_card_index: Some(3),
                active_command_id: Some("analysis.3.inspect-waveform".to_string()),
            },
        )
        .unwrap(),
    )
    .expect(
        "shell dashboard dispatch queue lane tab panel card action menu group shortcut command palette JSON should parse",
    );
    assert_eq!(
        shell_dashboard_dispatch_queue_lane_tab_panel_card_action_menu_group_shortcut_command_palette_payload
            ["activeMenuGroupShortcutCommandPaletteItemId"],
        "dashboard.dispatch-queue-lane-tab-panel-card-action-menu-group-shortcut-command-palette-item.queued"
    );
    assert_eq!(
        shell_dashboard_dispatch_queue_lane_tab_panel_card_action_menu_group_shortcut_command_palette_payload
            ["commandPaletteItems"][0]["commandEntryId"],
        "dashboard.dispatch-queue-lane-tab-panel-card-action-menu-group-shortcut-command.queued"
    );
    assert_eq!(
        shell_dashboard_dispatch_queue_lane_tab_panel_card_action_menu_group_shortcut_command_palette_payload
            ["commandPaletteItems"][0]["keywords"][0],
        "queued"
    );
    assert_eq!(
        shell_dashboard_dispatch_queue_lane_tab_panel_card_action_menu_group_shortcut_command_palette_payload
            ["commandPaletteItems"][2]["selectable"],
        false
    );
    assert_eq!(
        shell_dashboard_dispatch_queue_lane_tab_panel_card_action_menu_group_shortcut_command_palette_payload
            ["dispatchQueueLaneTabPanelCardActionMenuGroupShortcutCommandPaletteCapabilityId"],
        "app-shell-dashboard-dispatch-queue-lane-tab-panel-card-action-menu-group-shortcut-command-palette-json"
    );

    let shell_dashboard_dispatch_queue_lane_tab_panel_card_action_menu_group_shortcut_command_palette_search_index =
        app.run_app_shell_dashboard_dispatch_queue_lane_tab_panel_card_action_menu_group_shortcut_command_palette_search_index(
            BerkeleyAppPersistedEditorState {
                selected_syntax_card_index: Some(3),
                active_command_id: Some("analysis.3.inspect-waveform".to_string()),
            },
        )
        .expect(
            "shell dashboard dispatch queue lane tab panel card action menu group shortcut command palette search index should execute",
        );
    assert_eq!(
        shell_dashboard_dispatch_queue_lane_tab_panel_card_action_menu_group_shortcut_command_palette_search_index
            .schema_version,
        BERKELEY_APP_SHELL_DASHBOARD_DISPATCH_QUEUE_LANE_TAB_PANEL_CARD_ACTION_MENU_GROUP_SHORTCUT_COMMAND_PALETTE_SEARCH_INDEX_SCHEMA_VERSION
    );
    assert_eq!(
        shell_dashboard_dispatch_queue_lane_tab_panel_card_action_menu_group_shortcut_command_palette_search_index
            .search_index_id,
        "dashboard.dispatch-queue.shortcut-command-palette-search-index"
    );
    assert_eq!(
        shell_dashboard_dispatch_queue_lane_tab_panel_card_action_menu_group_shortcut_command_palette_search_index
            .active_menu_group_shortcut_command_search_index_entry_id
            .as_deref(),
        Some("dashboard.dispatch-queue-lane-tab-panel-card-action-menu-group-shortcut-command-palette-search-index-entry.queued")
    );
    assert_eq!(
        shell_dashboard_dispatch_queue_lane_tab_panel_card_action_menu_group_shortcut_command_palette_search_index
            .search_index_entry_count,
        3
    );
    assert_eq!(
        shell_dashboard_dispatch_queue_lane_tab_panel_card_action_menu_group_shortcut_command_palette_search_index
            .selectable_search_index_entry_count,
        2
    );
    assert_eq!(
        shell_dashboard_dispatch_queue_lane_tab_panel_card_action_menu_group_shortcut_command_palette_search_index
            .visible_search_index_entry_count,
        3
    );
    assert!(
        shell_dashboard_dispatch_queue_lane_tab_panel_card_action_menu_group_shortcut_command_palette_search_index
            .search_token_count
            > 0
    );
    assert_eq!(
        shell_dashboard_dispatch_queue_lane_tab_panel_card_action_menu_group_shortcut_command_palette_search_index
            .search_index_entries[0]
            .palette_item_id,
        "dashboard.dispatch-queue-lane-tab-panel-card-action-menu-group-shortcut-command-palette-item.queued"
    );
    assert!(
        shell_dashboard_dispatch_queue_lane_tab_panel_card_action_menu_group_shortcut_command_palette_search_index
            .search_index_entries[0]
            .normalized_search_text
            .contains("queued dispatches")
    );
    assert_eq!(
        shell_dashboard_dispatch_queue_lane_tab_panel_card_action_menu_group_shortcut_command_palette_search_index
            .search_index_entries[0]
            .search_tokens[0],
        "queued"
    );
    assert!(
        shell_dashboard_dispatch_queue_lane_tab_panel_card_action_menu_group_shortcut_command_palette_search_index
            .search_index_entries[0]
            .search_tokens
            .iter()
            .any(|token| token == "mod")
    );
    assert!(
        !shell_dashboard_dispatch_queue_lane_tab_panel_card_action_menu_group_shortcut_command_palette_search_index
            .search_index_entries[2]
            .selectable
    );
    assert_eq!(
        shell_dashboard_dispatch_queue_lane_tab_panel_card_action_menu_group_shortcut_command_palette_search_index
            .dispatch_queue_lane_tab_panel_card_action_menu_group_shortcut_command_palette_search_index_capability_id,
        "app-shell-dashboard-dispatch-queue-lane-tab-panel-card-action-menu-group-shortcut-command-palette-search-index-json"
    );

    let shell_dashboard_dispatch_queue_lane_tab_panel_card_action_menu_group_shortcut_command_palette_search_index_payload:
        serde_json::Value = serde_json::from_str(
        &app.run_app_shell_dashboard_dispatch_queue_lane_tab_panel_card_action_menu_group_shortcut_command_palette_search_index_json(
            BerkeleyAppPersistedEditorState {
                selected_syntax_card_index: Some(3),
                active_command_id: Some("analysis.3.inspect-waveform".to_string()),
            },
        )
        .unwrap(),
    )
    .expect(
        "shell dashboard dispatch queue lane tab panel card action menu group shortcut command palette search index JSON should parse",
    );
    assert_eq!(
        shell_dashboard_dispatch_queue_lane_tab_panel_card_action_menu_group_shortcut_command_palette_search_index_payload
            ["activeMenuGroupShortcutCommandSearchIndexEntryId"],
        "dashboard.dispatch-queue-lane-tab-panel-card-action-menu-group-shortcut-command-palette-search-index-entry.queued"
    );
    assert_eq!(
        shell_dashboard_dispatch_queue_lane_tab_panel_card_action_menu_group_shortcut_command_palette_search_index_payload
            ["searchIndexEntries"][0]["searchTokens"][0],
        "queued"
    );
    assert_eq!(
        shell_dashboard_dispatch_queue_lane_tab_panel_card_action_menu_group_shortcut_command_palette_search_index_payload
            ["searchIndexEntries"][2]["selectable"],
        false
    );
    assert_eq!(
        shell_dashboard_dispatch_queue_lane_tab_panel_card_action_menu_group_shortcut_command_palette_search_index_payload
            ["dispatchQueueLaneTabPanelCardActionMenuGroupShortcutCommandPaletteSearchIndexCapabilityId"],
        "app-shell-dashboard-dispatch-queue-lane-tab-panel-card-action-menu-group-shortcut-command-palette-search-index-json"
    );

    let shell_dashboard_dispatch_queue_lane_tab_panel_card_action_menu_group_shortcut_command_palette_search_results =
        app.run_app_shell_dashboard_dispatch_queue_lane_tab_panel_card_action_menu_group_shortcut_command_palette_search_results(
            BerkeleyAppPersistedEditorState {
                selected_syntax_card_index: Some(3),
                active_command_id: Some("analysis.3.inspect-waveform".to_string()),
            },
            "queued dispatch",
        )
        .expect(
            "shell dashboard dispatch queue lane tab panel card action menu group shortcut command palette search results should execute",
        );
    assert_eq!(
        shell_dashboard_dispatch_queue_lane_tab_panel_card_action_menu_group_shortcut_command_palette_search_results
            .schema_version,
        BERKELEY_APP_SHELL_DASHBOARD_DISPATCH_QUEUE_LANE_TAB_PANEL_CARD_ACTION_MENU_GROUP_SHORTCUT_COMMAND_PALETTE_SEARCH_RESULTS_SCHEMA_VERSION
    );
    assert_eq!(
        shell_dashboard_dispatch_queue_lane_tab_panel_card_action_menu_group_shortcut_command_palette_search_results
            .normalized_query,
        "queued dispatch"
    );
    assert_eq!(
        shell_dashboard_dispatch_queue_lane_tab_panel_card_action_menu_group_shortcut_command_palette_search_results
            .query_token_count,
        2
    );
    assert_eq!(
        shell_dashboard_dispatch_queue_lane_tab_panel_card_action_menu_group_shortcut_command_palette_search_results
            .search_result_count,
        1
    );
    assert_eq!(
        shell_dashboard_dispatch_queue_lane_tab_panel_card_action_menu_group_shortcut_command_palette_search_results
            .active_menu_group_shortcut_command_search_result_id
            .as_deref(),
        Some("dashboard.dispatch-queue-lane-tab-panel-card-action-menu-group-shortcut-command-palette-search-result.queued")
    );
    assert!(
        !shell_dashboard_dispatch_queue_lane_tab_panel_card_action_menu_group_shortcut_command_palette_search_results
            .no_results
    );
    assert_eq!(
        shell_dashboard_dispatch_queue_lane_tab_panel_card_action_menu_group_shortcut_command_palette_search_results
            .search_results[0]
            .search_index_entry_id,
        "dashboard.dispatch-queue-lane-tab-panel-card-action-menu-group-shortcut-command-palette-search-index-entry.queued"
    );
    assert_eq!(
        shell_dashboard_dispatch_queue_lane_tab_panel_card_action_menu_group_shortcut_command_palette_search_results
            .search_results[0]
            .matched_query_tokens,
        vec!["queued".to_string(), "dispatch".to_string()]
    );
    assert_eq!(
        shell_dashboard_dispatch_queue_lane_tab_panel_card_action_menu_group_shortcut_command_palette_search_results
            .dispatch_queue_lane_tab_panel_card_action_menu_group_shortcut_command_palette_search_results_capability_id,
        "app-shell-dashboard-dispatch-queue-lane-tab-panel-card-action-menu-group-shortcut-command-palette-search-results-json"
    );

    let shell_dashboard_dispatch_queue_lane_tab_panel_card_action_menu_group_shortcut_command_palette_search_results_payload:
        serde_json::Value = serde_json::from_str(
        &app.run_app_shell_dashboard_dispatch_queue_lane_tab_panel_card_action_menu_group_shortcut_command_palette_search_results_json(
            BerkeleyAppPersistedEditorState {
                selected_syntax_card_index: Some(3),
                active_command_id: Some("analysis.3.inspect-waveform".to_string()),
            },
            "queued dispatch",
        )
        .unwrap(),
    )
    .expect(
        "shell dashboard dispatch queue lane tab panel card action menu group shortcut command palette search results JSON should parse",
    );
    assert_eq!(
        shell_dashboard_dispatch_queue_lane_tab_panel_card_action_menu_group_shortcut_command_palette_search_results_payload
            ["activeMenuGroupShortcutCommandSearchResultId"],
        "dashboard.dispatch-queue-lane-tab-panel-card-action-menu-group-shortcut-command-palette-search-result.queued"
    );
    assert_eq!(
        shell_dashboard_dispatch_queue_lane_tab_panel_card_action_menu_group_shortcut_command_palette_search_results_payload
            ["queryTokens"][1],
        "dispatch"
    );
    assert_eq!(
        shell_dashboard_dispatch_queue_lane_tab_panel_card_action_menu_group_shortcut_command_palette_search_results_payload
            ["searchResults"][0]["matchedQueryTokens"][0],
        "queued"
    );
    assert_eq!(
        shell_dashboard_dispatch_queue_lane_tab_panel_card_action_menu_group_shortcut_command_palette_search_results_payload
            ["dispatchQueueLaneTabPanelCardActionMenuGroupShortcutCommandPaletteSearchResultsCapabilityId"],
        "app-shell-dashboard-dispatch-queue-lane-tab-panel-card-action-menu-group-shortcut-command-palette-search-results-json"
    );

    let shell_dashboard_dispatch_queue_lane_tab_panel_card_action_menu_group_shortcut_command_palette_search_selection =
        app.run_app_shell_dashboard_dispatch_queue_lane_tab_panel_card_action_menu_group_shortcut_command_palette_search_selection(
            BerkeleyAppPersistedEditorState {
                selected_syntax_card_index: Some(3),
                active_command_id: Some("analysis.3.inspect-waveform".to_string()),
            },
            "queued dispatch",
            None,
        )
        .expect(
            "shell dashboard dispatch queue lane tab panel card action menu group shortcut command palette search selection should execute",
        );
    assert_eq!(
        shell_dashboard_dispatch_queue_lane_tab_panel_card_action_menu_group_shortcut_command_palette_search_selection
            .schema_version,
        BERKELEY_APP_SHELL_DASHBOARD_DISPATCH_QUEUE_LANE_TAB_PANEL_CARD_ACTION_MENU_GROUP_SHORTCUT_COMMAND_PALETTE_SEARCH_SELECTION_SCHEMA_VERSION
    );
    assert_eq!(
        shell_dashboard_dispatch_queue_lane_tab_panel_card_action_menu_group_shortcut_command_palette_search_selection
            .selection_source,
        "active"
    );
    assert_eq!(
        shell_dashboard_dispatch_queue_lane_tab_panel_card_action_menu_group_shortcut_command_palette_search_selection
            .selection_state,
        "ready"
    );
    assert!(
        shell_dashboard_dispatch_queue_lane_tab_panel_card_action_menu_group_shortcut_command_palette_search_selection
            .can_invoke
    );
    assert_eq!(
        shell_dashboard_dispatch_queue_lane_tab_panel_card_action_menu_group_shortcut_command_palette_search_selection
            .selected_search_result_id
            .as_deref(),
        Some("dashboard.dispatch-queue-lane-tab-panel-card-action-menu-group-shortcut-command-palette-search-result.queued")
    );
    assert_eq!(
        shell_dashboard_dispatch_queue_lane_tab_panel_card_action_menu_group_shortcut_command_palette_search_selection
            .selected_command_id
            .as_deref(),
        Some("berkeley.app-shell.dashboard.dispatch-queue.menu-group-shortcut.queued")
    );
    assert_eq!(
        shell_dashboard_dispatch_queue_lane_tab_panel_card_action_menu_group_shortcut_command_palette_search_selection
            .selected_handler_id
            .as_deref(),
        Some("handler.berkeley.app-shell.dashboard.dispatch-queue.menu-group-shortcut.queued")
    );
    assert_eq!(
        shell_dashboard_dispatch_queue_lane_tab_panel_card_action_menu_group_shortcut_command_palette_search_selection
            .selected_target_kind
            .as_deref(),
        Some("menu-group")
    );
    assert_eq!(
        shell_dashboard_dispatch_queue_lane_tab_panel_card_action_menu_group_shortcut_command_palette_search_selection
            .selected_matched_query_tokens,
        vec!["queued".to_string(), "dispatch".to_string()]
    );
    assert_eq!(
        shell_dashboard_dispatch_queue_lane_tab_panel_card_action_menu_group_shortcut_command_palette_search_selection
            .dispatch_queue_lane_tab_panel_card_action_menu_group_shortcut_command_palette_search_selection_capability_id,
        "app-shell-dashboard-dispatch-queue-lane-tab-panel-card-action-menu-group-shortcut-command-palette-search-selection-json"
    );

    let shell_dashboard_dispatch_queue_lane_tab_panel_card_action_menu_group_shortcut_command_palette_search_selection_payload:
        serde_json::Value = serde_json::from_str(
        &app.run_app_shell_dashboard_dispatch_queue_lane_tab_panel_card_action_menu_group_shortcut_command_palette_search_selection_json(
            BerkeleyAppPersistedEditorState {
                selected_syntax_card_index: Some(3),
                active_command_id: Some("analysis.3.inspect-waveform".to_string()),
            },
            "queued dispatch",
            Some("dashboard.dispatch-queue-lane-tab-panel-card-action-menu-group-shortcut-command-palette-search-result.queued".to_string()),
        )
        .unwrap(),
    )
    .expect(
        "shell dashboard dispatch queue lane tab panel card action menu group shortcut command palette search selection JSON should parse",
    );
    assert_eq!(
        shell_dashboard_dispatch_queue_lane_tab_panel_card_action_menu_group_shortcut_command_palette_search_selection_payload
            ["selectionSource"],
        "requested"
    );
    assert_eq!(
        shell_dashboard_dispatch_queue_lane_tab_panel_card_action_menu_group_shortcut_command_palette_search_selection_payload
            ["canInvoke"],
        true
    );
    assert_eq!(
        shell_dashboard_dispatch_queue_lane_tab_panel_card_action_menu_group_shortcut_command_palette_search_selection_payload
            ["selectedCommandId"],
        "berkeley.app-shell.dashboard.dispatch-queue.menu-group-shortcut.queued"
    );
    assert_eq!(
        shell_dashboard_dispatch_queue_lane_tab_panel_card_action_menu_group_shortcut_command_palette_search_selection_payload
            ["selectedMatchedQueryTokens"][1],
        "dispatch"
    );
    assert_eq!(
        shell_dashboard_dispatch_queue_lane_tab_panel_card_action_menu_group_shortcut_command_palette_search_selection_payload
            ["dispatchQueueLaneTabPanelCardActionMenuGroupShortcutCommandPaletteSearchSelectionCapabilityId"],
        "app-shell-dashboard-dispatch-queue-lane-tab-panel-card-action-menu-group-shortcut-command-palette-search-selection-json"
    );

    let shell_dashboard_dispatch_queue_lane_tab_panel_card_action_menu_group_shortcut_command_palette_search_invocation =
        app.run_app_shell_dashboard_dispatch_queue_lane_tab_panel_card_action_menu_group_shortcut_command_palette_search_invocation(
            BerkeleyAppPersistedEditorState {
                selected_syntax_card_index: Some(3),
                active_command_id: Some("analysis.3.inspect-waveform".to_string()),
            },
            "queued dispatch",
            None,
        )
        .expect(
            "shell dashboard dispatch queue lane tab panel card action menu group shortcut command palette search invocation should execute",
        );
    assert_eq!(
        shell_dashboard_dispatch_queue_lane_tab_panel_card_action_menu_group_shortcut_command_palette_search_invocation
            .schema_version,
        BERKELEY_APP_SHELL_DASHBOARD_DISPATCH_QUEUE_LANE_TAB_PANEL_CARD_ACTION_MENU_GROUP_SHORTCUT_COMMAND_PALETTE_SEARCH_INVOCATION_SCHEMA_VERSION
    );
    assert_eq!(
        shell_dashboard_dispatch_queue_lane_tab_panel_card_action_menu_group_shortcut_command_palette_search_invocation
            .invocation_state,
        "ready"
    );
    assert_eq!(
        shell_dashboard_dispatch_queue_lane_tab_panel_card_action_menu_group_shortcut_command_palette_search_invocation
            .invocation_action
            .as_deref(),
        Some("invoke-command")
    );
    assert!(
        shell_dashboard_dispatch_queue_lane_tab_panel_card_action_menu_group_shortcut_command_palette_search_invocation
            .can_dispatch
    );
    assert_eq!(
        shell_dashboard_dispatch_queue_lane_tab_panel_card_action_menu_group_shortcut_command_palette_search_invocation
            .selected_search_result_id
            .as_deref(),
        Some("dashboard.dispatch-queue-lane-tab-panel-card-action-menu-group-shortcut-command-palette-search-result.queued")
    );
    assert_eq!(
        shell_dashboard_dispatch_queue_lane_tab_panel_card_action_menu_group_shortcut_command_palette_search_invocation
            .selected_command_id
            .as_deref(),
        Some("berkeley.app-shell.dashboard.dispatch-queue.menu-group-shortcut.queued")
    );
    assert_eq!(
        shell_dashboard_dispatch_queue_lane_tab_panel_card_action_menu_group_shortcut_command_palette_search_invocation
            .selected_handler_id
            .as_deref(),
        Some("handler.berkeley.app-shell.dashboard.dispatch-queue.menu-group-shortcut.queued")
    );
    assert_eq!(
        shell_dashboard_dispatch_queue_lane_tab_panel_card_action_menu_group_shortcut_command_palette_search_invocation
            .selected_target
            .as_deref(),
        Some("dashboard.dispatch-queue-lane-tab-panel-card-action-menu-group.queued")
    );
    assert_eq!(
        shell_dashboard_dispatch_queue_lane_tab_panel_card_action_menu_group_shortcut_command_palette_search_invocation
            .dispatch_queue_lane_tab_panel_card_action_menu_group_shortcut_command_palette_search_invocation_capability_id,
        "app-shell-dashboard-dispatch-queue-lane-tab-panel-card-action-menu-group-shortcut-command-palette-search-invocation-json"
    );

    let shell_dashboard_dispatch_queue_lane_tab_panel_card_action_menu_group_shortcut_command_palette_search_invocation_payload:
        serde_json::Value = serde_json::from_str(
        &app.run_app_shell_dashboard_dispatch_queue_lane_tab_panel_card_action_menu_group_shortcut_command_palette_search_invocation_json(
            BerkeleyAppPersistedEditorState {
                selected_syntax_card_index: Some(3),
                active_command_id: Some("analysis.3.inspect-waveform".to_string()),
            },
            "queued dispatch",
            Some("dashboard.dispatch-queue-lane-tab-panel-card-action-menu-group-shortcut-command-palette-search-result.queued".to_string()),
        )
        .unwrap(),
    )
    .expect(
        "shell dashboard dispatch queue lane tab panel card action menu group shortcut command palette search invocation JSON should parse",
    );
    assert_eq!(
        shell_dashboard_dispatch_queue_lane_tab_panel_card_action_menu_group_shortcut_command_palette_search_invocation_payload
            ["selectionSource"],
        "requested"
    );
    assert_eq!(
        shell_dashboard_dispatch_queue_lane_tab_panel_card_action_menu_group_shortcut_command_palette_search_invocation_payload
            ["canDispatch"],
        true
    );
    assert_eq!(
        shell_dashboard_dispatch_queue_lane_tab_panel_card_action_menu_group_shortcut_command_palette_search_invocation_payload
            ["invocationAction"],
        "invoke-command"
    );
    assert_eq!(
        shell_dashboard_dispatch_queue_lane_tab_panel_card_action_menu_group_shortcut_command_palette_search_invocation_payload
            ["selectedCommandId"],
        "berkeley.app-shell.dashboard.dispatch-queue.menu-group-shortcut.queued"
    );
    assert_eq!(
        shell_dashboard_dispatch_queue_lane_tab_panel_card_action_menu_group_shortcut_command_palette_search_invocation_payload
            ["dispatchQueueLaneTabPanelCardActionMenuGroupShortcutCommandPaletteSearchInvocationCapabilityId"],
        "app-shell-dashboard-dispatch-queue-lane-tab-panel-card-action-menu-group-shortcut-command-palette-search-invocation-json"
    );

    let shell_dashboard_dispatch_queue_lane_tab_panel_card_action_menu_group_shortcut_command_palette_search_invocation_receipts =
        app.run_app_shell_dashboard_dispatch_queue_lane_tab_panel_card_action_menu_group_shortcut_command_palette_search_invocation_receipts(
            BerkeleyAppPersistedEditorState {
                selected_syntax_card_index: Some(3),
                active_command_id: Some("analysis.3.inspect-waveform".to_string()),
            },
            "queued dispatch",
            Some("dashboard.dispatch-queue-lane-tab-panel-card-action-menu-group-shortcut-command-palette-search-result.queued".to_string()),
        )
        .expect(
            "shell dashboard dispatch queue lane tab panel card action menu group shortcut command palette search invocation receipts should execute",
        );
    assert_eq!(
        shell_dashboard_dispatch_queue_lane_tab_panel_card_action_menu_group_shortcut_command_palette_search_invocation_receipts
            .schema_version,
        BERKELEY_APP_SHELL_DASHBOARD_DISPATCH_QUEUE_LANE_TAB_PANEL_CARD_ACTION_MENU_GROUP_SHORTCUT_COMMAND_PALETTE_SEARCH_INVOCATION_RECEIPTS_SCHEMA_VERSION
    );
    assert_eq!(
        shell_dashboard_dispatch_queue_lane_tab_panel_card_action_menu_group_shortcut_command_palette_search_invocation_receipts
            .receipt_count,
        1
    );
    assert_eq!(
        shell_dashboard_dispatch_queue_lane_tab_panel_card_action_menu_group_shortcut_command_palette_search_invocation_receipts
            .accepted_receipt_count,
        1
    );
    assert_eq!(
        shell_dashboard_dispatch_queue_lane_tab_panel_card_action_menu_group_shortcut_command_palette_search_invocation_receipts
            .receipts[0]
            .receipt_state,
        "accepted"
    );
    assert_eq!(
        shell_dashboard_dispatch_queue_lane_tab_panel_card_action_menu_group_shortcut_command_palette_search_invocation_receipts
            .dispatch_queue_lane_tab_panel_card_action_menu_group_shortcut_command_palette_search_invocation_receipts_capability_id,
        "app-shell-dashboard-dispatch-queue-lane-tab-panel-card-action-menu-group-shortcut-command-palette-search-invocation-receipts-json"
    );

    let shell_dashboard_dispatch_queue_lane_tab_panel_card_action_menu_group_shortcut_command_palette_search_invocation_receipts_payload:
        serde_json::Value = serde_json::from_str(
        &app.run_app_shell_dashboard_dispatch_queue_lane_tab_panel_card_action_menu_group_shortcut_command_palette_search_invocation_receipts_json(
            BerkeleyAppPersistedEditorState {
                selected_syntax_card_index: Some(3),
                active_command_id: Some("analysis.3.inspect-waveform".to_string()),
            },
            "queued dispatch",
            Some("dashboard.dispatch-queue-lane-tab-panel-card-action-menu-group-shortcut-command-palette-search-result.queued".to_string()),
        )
        .unwrap(),
    )
    .expect(
        "shell dashboard dispatch queue lane tab panel card action menu group shortcut command palette search invocation receipts JSON should parse",
    );
    assert_eq!(
        shell_dashboard_dispatch_queue_lane_tab_panel_card_action_menu_group_shortcut_command_palette_search_invocation_receipts_payload
            ["acceptedReceiptCount"],
        1
    );
    assert_eq!(
        shell_dashboard_dispatch_queue_lane_tab_panel_card_action_menu_group_shortcut_command_palette_search_invocation_receipts_payload
            ["receipts"][0]["dispatchAccepted"],
        true
    );
    assert_eq!(
        shell_dashboard_dispatch_queue_lane_tab_panel_card_action_menu_group_shortcut_command_palette_search_invocation_receipts_payload
            ["receipts"][0]["selectedCommandId"],
        "berkeley.app-shell.dashboard.dispatch-queue.menu-group-shortcut.queued"
    );
    assert_eq!(
        shell_dashboard_dispatch_queue_lane_tab_panel_card_action_menu_group_shortcut_command_palette_search_invocation_receipts_payload
            ["dispatchQueueLaneTabPanelCardActionMenuGroupShortcutCommandPaletteSearchInvocationReceiptsCapabilityId"],
        "app-shell-dashboard-dispatch-queue-lane-tab-panel-card-action-menu-group-shortcut-command-palette-search-invocation-receipts-json"
    );

    let shell_dashboard_dispatch_queue_lane_tab_panel_card_action_menu_group_shortcut_command_palette_search_invocation_receipt_summary =
        app.run_app_shell_dashboard_dispatch_queue_lane_tab_panel_card_action_menu_group_shortcut_command_palette_search_invocation_receipt_summary(
            BerkeleyAppPersistedEditorState {
                selected_syntax_card_index: Some(3),
                active_command_id: Some("analysis.3.inspect-waveform".to_string()),
            },
            "queued dispatch",
            Some("dashboard.dispatch-queue-lane-tab-panel-card-action-menu-group-shortcut-command-palette-search-result.queued".to_string()),
        )
        .expect(
            "shell dashboard dispatch queue lane tab panel card action menu group shortcut command palette search invocation receipt summary should execute",
        );
    assert_eq!(
        shell_dashboard_dispatch_queue_lane_tab_panel_card_action_menu_group_shortcut_command_palette_search_invocation_receipt_summary
            .schema_version,
        BERKELEY_APP_SHELL_DASHBOARD_DISPATCH_QUEUE_LANE_TAB_PANEL_CARD_ACTION_MENU_GROUP_SHORTCUT_COMMAND_PALETTE_SEARCH_INVOCATION_RECEIPT_SUMMARY_SCHEMA_VERSION
    );
    assert_eq!(
        shell_dashboard_dispatch_queue_lane_tab_panel_card_action_menu_group_shortcut_command_palette_search_invocation_receipt_summary
            .status_kind,
        "success"
    );
    assert_eq!(
        shell_dashboard_dispatch_queue_lane_tab_panel_card_action_menu_group_shortcut_command_palette_search_invocation_receipt_summary
            .status_title,
        "Command accepted"
    );
    assert_eq!(
        shell_dashboard_dispatch_queue_lane_tab_panel_card_action_menu_group_shortcut_command_palette_search_invocation_receipt_summary
            .selected_command_id
            .as_deref(),
        Some("berkeley.app-shell.dashboard.dispatch-queue.menu-group-shortcut.queued")
    );
    assert_eq!(
        shell_dashboard_dispatch_queue_lane_tab_panel_card_action_menu_group_shortcut_command_palette_search_invocation_receipt_summary
            .dispatch_queue_lane_tab_panel_card_action_menu_group_shortcut_command_palette_search_invocation_receipt_summary_capability_id,
        "app-shell-dashboard-dispatch-queue-lane-tab-panel-card-action-menu-group-shortcut-command-palette-search-invocation-receipt-summary-json"
    );

    let shell_dashboard_dispatch_queue_lane_tab_panel_card_action_menu_group_shortcut_command_palette_search_invocation_receipt_summary_payload:
        serde_json::Value = serde_json::from_str(
        &app.run_app_shell_dashboard_dispatch_queue_lane_tab_panel_card_action_menu_group_shortcut_command_palette_search_invocation_receipt_summary_json(
            BerkeleyAppPersistedEditorState {
                selected_syntax_card_index: Some(3),
                active_command_id: Some("analysis.3.inspect-waveform".to_string()),
            },
            "queued dispatch",
            Some("dashboard.dispatch-queue-lane-tab-panel-card-action-menu-group-shortcut-command-palette-search-result.queued".to_string()),
        )
        .unwrap(),
    )
    .expect(
        "shell dashboard dispatch queue lane tab panel card action menu group shortcut command palette search invocation receipt summary JSON should parse",
    );
    assert_eq!(
        shell_dashboard_dispatch_queue_lane_tab_panel_card_action_menu_group_shortcut_command_palette_search_invocation_receipt_summary_payload
            ["statusKind"],
        "success"
    );
    assert_eq!(
        shell_dashboard_dispatch_queue_lane_tab_panel_card_action_menu_group_shortcut_command_palette_search_invocation_receipt_summary_payload
            ["dispatchAccepted"],
        true
    );
    assert_eq!(
        shell_dashboard_dispatch_queue_lane_tab_panel_card_action_menu_group_shortcut_command_palette_search_invocation_receipt_summary_payload
            ["latestReceiptId"],
        "dashboard.dispatch-queue.shortcut-command-palette-search-invocation.receipt.accepted"
    );
    assert_eq!(
        shell_dashboard_dispatch_queue_lane_tab_panel_card_action_menu_group_shortcut_command_palette_search_invocation_receipt_summary_payload
            ["dispatchQueueLaneTabPanelCardActionMenuGroupShortcutCommandPaletteSearchInvocationReceiptSummaryCapabilityId"],
        "app-shell-dashboard-dispatch-queue-lane-tab-panel-card-action-menu-group-shortcut-command-palette-search-invocation-receipt-summary-json"
    );

    let shell_dashboard_dispatch_queue_lane_tab_panel_card_action_menu_group_shortcut_command_palette_search_invocation_receipt_notification =
        app.run_app_shell_dashboard_dispatch_queue_lane_tab_panel_card_action_menu_group_shortcut_command_palette_search_invocation_receipt_notification(
            BerkeleyAppPersistedEditorState {
                selected_syntax_card_index: Some(3),
                active_command_id: Some("analysis.3.inspect-waveform".to_string()),
            },
            "queued dispatch",
            Some("dashboard.dispatch-queue-lane-tab-panel-card-action-menu-group-shortcut-command-palette-search-result.queued".to_string()),
        )
        .expect(
            "shell dashboard dispatch queue lane tab panel card action menu group shortcut command palette search invocation receipt notification should execute",
        );
    assert_eq!(
        shell_dashboard_dispatch_queue_lane_tab_panel_card_action_menu_group_shortcut_command_palette_search_invocation_receipt_notification
            .schema_version,
        BERKELEY_APP_SHELL_DASHBOARD_DISPATCH_QUEUE_LANE_TAB_PANEL_CARD_ACTION_MENU_GROUP_SHORTCUT_COMMAND_PALETTE_SEARCH_INVOCATION_RECEIPT_NOTIFICATION_SCHEMA_VERSION
    );
    assert_eq!(
        shell_dashboard_dispatch_queue_lane_tab_panel_card_action_menu_group_shortcut_command_palette_search_invocation_receipt_notification
            .notification_kind,
        "dispatch-accepted"
    );
    assert_eq!(
        shell_dashboard_dispatch_queue_lane_tab_panel_card_action_menu_group_shortcut_command_palette_search_invocation_receipt_notification
            .notification_level,
        "success"
    );
    assert_eq!(
        shell_dashboard_dispatch_queue_lane_tab_panel_card_action_menu_group_shortcut_command_palette_search_invocation_receipt_notification
            .notification_action_label
            .as_deref(),
        Some("Open command")
    );
    assert_eq!(
        shell_dashboard_dispatch_queue_lane_tab_panel_card_action_menu_group_shortcut_command_palette_search_invocation_receipt_notification
            .notification_action_target
            .as_deref(),
        Some("dashboard.dispatch-queue-lane-tab-panel-card-action-menu-group.queued")
    );
    assert!(
        shell_dashboard_dispatch_queue_lane_tab_panel_card_action_menu_group_shortcut_command_palette_search_invocation_receipt_notification
            .should_announce
    );
    assert_eq!(
        shell_dashboard_dispatch_queue_lane_tab_panel_card_action_menu_group_shortcut_command_palette_search_invocation_receipt_notification
            .dispatch_queue_lane_tab_panel_card_action_menu_group_shortcut_command_palette_search_invocation_receipt_notification_capability_id,
        "app-shell-dashboard-dispatch-queue-lane-tab-panel-card-action-menu-group-shortcut-command-palette-search-invocation-receipt-notification-json"
    );

    let shell_dashboard_dispatch_queue_lane_tab_panel_card_action_menu_group_shortcut_command_palette_search_invocation_receipt_notification_payload:
        serde_json::Value = serde_json::from_str(
        &app.run_app_shell_dashboard_dispatch_queue_lane_tab_panel_card_action_menu_group_shortcut_command_palette_search_invocation_receipt_notification_json(
            BerkeleyAppPersistedEditorState {
                selected_syntax_card_index: Some(3),
                active_command_id: Some("analysis.3.inspect-waveform".to_string()),
            },
            "queued dispatch",
            Some("dashboard.dispatch-queue-lane-tab-panel-card-action-menu-group-shortcut-command-palette-search-result.queued".to_string()),
        )
        .unwrap(),
    )
    .expect(
        "shell dashboard dispatch queue lane tab panel card action menu group shortcut command palette search invocation receipt notification JSON should parse",
    );
    assert_eq!(
        shell_dashboard_dispatch_queue_lane_tab_panel_card_action_menu_group_shortcut_command_palette_search_invocation_receipt_notification_payload
            ["notificationKind"],
        "dispatch-accepted"
    );
    assert_eq!(
        shell_dashboard_dispatch_queue_lane_tab_panel_card_action_menu_group_shortcut_command_palette_search_invocation_receipt_notification_payload
            ["notificationActionLabel"],
        "Open command"
    );
    assert_eq!(
        shell_dashboard_dispatch_queue_lane_tab_panel_card_action_menu_group_shortcut_command_palette_search_invocation_receipt_notification_payload
            ["shouldAnnounce"],
        true
    );
    assert_eq!(
        shell_dashboard_dispatch_queue_lane_tab_panel_card_action_menu_group_shortcut_command_palette_search_invocation_receipt_notification_payload
            ["announcement"],
        "Command accepted: Accepted command \"Queued dispatches\" for dispatch."
    );
    assert_eq!(
        shell_dashboard_dispatch_queue_lane_tab_panel_card_action_menu_group_shortcut_command_palette_search_invocation_receipt_notification_payload
            ["dispatchQueueLaneTabPanelCardActionMenuGroupShortcutCommandPaletteSearchInvocationReceiptNotificationCapabilityId"],
        "app-shell-dashboard-dispatch-queue-lane-tab-panel-card-action-menu-group-shortcut-command-palette-search-invocation-receipt-notification-json"
    );

    let shell_dashboard_dispatch_queue_lane_tab_panel_card_action_menu_group_shortcut_command_palette_search_invocation_receipt_notification_stack =
        app.run_app_shell_dashboard_dispatch_queue_lane_tab_panel_card_action_menu_group_shortcut_command_palette_search_invocation_receipt_notification_stack(
            BerkeleyAppPersistedEditorState {
                selected_syntax_card_index: Some(3),
                active_command_id: Some("analysis.3.inspect-waveform".to_string()),
            },
            "queued dispatch",
            Some("dashboard.dispatch-queue-lane-tab-panel-card-action-menu-group-shortcut-command-palette-search-result.queued".to_string()),
        )
        .expect(
            "shell dashboard dispatch queue lane tab panel card action menu group shortcut command palette search invocation receipt notification stack should execute",
        );
    assert_eq!(
        shell_dashboard_dispatch_queue_lane_tab_panel_card_action_menu_group_shortcut_command_palette_search_invocation_receipt_notification_stack
            .schema_version,
        BERKELEY_APP_SHELL_DASHBOARD_DISPATCH_QUEUE_LANE_TAB_PANEL_CARD_ACTION_MENU_GROUP_SHORTCUT_COMMAND_PALETTE_SEARCH_INVOCATION_RECEIPT_NOTIFICATION_STACK_SCHEMA_VERSION
    );
    assert_eq!(
        shell_dashboard_dispatch_queue_lane_tab_panel_card_action_menu_group_shortcut_command_palette_search_invocation_receipt_notification_stack
            .notification_count,
        1
    );
    assert_eq!(
        shell_dashboard_dispatch_queue_lane_tab_panel_card_action_menu_group_shortcut_command_palette_search_invocation_receipt_notification_stack
            .visible_notification_count,
        1
    );
    assert_eq!(
        shell_dashboard_dispatch_queue_lane_tab_panel_card_action_menu_group_shortcut_command_palette_search_invocation_receipt_notification_stack
            .announce_notification_count,
        1
    );
    assert_eq!(
        shell_dashboard_dispatch_queue_lane_tab_panel_card_action_menu_group_shortcut_command_palette_search_invocation_receipt_notification_stack
            .success_notification_count,
        1
    );
    assert_eq!(
        shell_dashboard_dispatch_queue_lane_tab_panel_card_action_menu_group_shortcut_command_palette_search_invocation_receipt_notification_stack
            .active_notification_id
            .as_deref(),
        Some("dashboard.dispatch-queue.shortcut-command-palette-search-invocation-receipt-notification")
    );
    assert_eq!(
        shell_dashboard_dispatch_queue_lane_tab_panel_card_action_menu_group_shortcut_command_palette_search_invocation_receipt_notification_stack
            .latest_notification_kind
            .as_deref(),
        Some("dispatch-accepted")
    );
    assert_eq!(
        shell_dashboard_dispatch_queue_lane_tab_panel_card_action_menu_group_shortcut_command_palette_search_invocation_receipt_notification_stack
            .notifications[0]
            .notification_kind,
        "dispatch-accepted"
    );
    assert_eq!(
        shell_dashboard_dispatch_queue_lane_tab_panel_card_action_menu_group_shortcut_command_palette_search_invocation_receipt_notification_stack
            .dispatch_queue_lane_tab_panel_card_action_menu_group_shortcut_command_palette_search_invocation_receipt_notification_stack_capability_id,
        "app-shell-dashboard-dispatch-queue-lane-tab-panel-card-action-menu-group-shortcut-command-palette-search-invocation-receipt-notification-stack-json"
    );

    let shell_dashboard_dispatch_queue_lane_tab_panel_card_action_menu_group_shortcut_command_palette_search_invocation_receipt_notification_stack_payload:
        serde_json::Value = serde_json::from_str(
        &app.run_app_shell_dashboard_dispatch_queue_lane_tab_panel_card_action_menu_group_shortcut_command_palette_search_invocation_receipt_notification_stack_json(
            BerkeleyAppPersistedEditorState {
                selected_syntax_card_index: Some(3),
                active_command_id: Some("analysis.3.inspect-waveform".to_string()),
            },
            "queued dispatch",
            Some("dashboard.dispatch-queue-lane-tab-panel-card-action-menu-group-shortcut-command-palette-search-result.queued".to_string()),
        )
        .unwrap(),
    )
    .expect(
        "shell dashboard dispatch queue lane tab panel card action menu group shortcut command palette search invocation receipt notification stack JSON should parse",
    );
    assert_eq!(
        shell_dashboard_dispatch_queue_lane_tab_panel_card_action_menu_group_shortcut_command_palette_search_invocation_receipt_notification_stack_payload
            ["latestNotificationKind"],
        "dispatch-accepted"
    );
    assert_eq!(
        shell_dashboard_dispatch_queue_lane_tab_panel_card_action_menu_group_shortcut_command_palette_search_invocation_receipt_notification_stack_payload
            ["visibleNotificationCount"],
        1
    );
    assert_eq!(
        shell_dashboard_dispatch_queue_lane_tab_panel_card_action_menu_group_shortcut_command_palette_search_invocation_receipt_notification_stack_payload
            ["notifications"][0]["notificationActionLabel"],
        "Open command"
    );
    assert_eq!(
        shell_dashboard_dispatch_queue_lane_tab_panel_card_action_menu_group_shortcut_command_palette_search_invocation_receipt_notification_stack_payload
            ["dispatchQueueLaneTabPanelCardActionMenuGroupShortcutCommandPaletteSearchInvocationReceiptNotificationStackCapabilityId"],
        "app-shell-dashboard-dispatch-queue-lane-tab-panel-card-action-menu-group-shortcut-command-palette-search-invocation-receipt-notification-stack-json"
    );

    let accepted_notification_stack_summary =
        app.run_app_shell_dashboard_dispatch_queue_lane_tab_panel_card_action_menu_group_shortcut_command_palette_search_invocation_receipt_notification_stack_summary(
            BerkeleyAppPersistedEditorState {
                selected_syntax_card_index: Some(3),
                active_command_id: Some("analysis.3.inspect-waveform".to_string()),
            },
            "queued dispatch",
            Some("dashboard.dispatch-queue-lane-tab-panel-card-action-menu-group-shortcut-command-palette-search-result.queued".to_string()),
        )
        .expect(
            "shell dashboard dispatch queue lane tab panel card action menu group shortcut command palette search invocation receipt notification stack summary should execute",
        );
    assert_eq!(
        accepted_notification_stack_summary.schema_version,
        BERKELEY_APP_SHELL_DASHBOARD_DISPATCH_QUEUE_LANE_TAB_PANEL_CARD_ACTION_MENU_GROUP_SHORTCUT_COMMAND_PALETTE_SEARCH_INVOCATION_RECEIPT_NOTIFICATION_STACK_SUMMARY_SCHEMA_VERSION
    );
    assert_eq!(accepted_notification_stack_summary.summary_kind, "visible");
    assert_eq!(accepted_notification_stack_summary.summary_level, "success");
    assert!(accepted_notification_stack_summary.should_render_stack);
    assert!(accepted_notification_stack_summary.should_announce_latest);
    assert_eq!(
        accepted_notification_stack_summary
            .headline_notification_id
            .as_deref(),
        Some("dashboard.dispatch-queue.shortcut-command-palette-search-invocation-receipt-notification")
    );

    let accepted_notification_stack_summary_payload: serde_json::Value = serde_json::from_str(
        &app.run_app_shell_dashboard_dispatch_queue_lane_tab_panel_card_action_menu_group_shortcut_command_palette_search_invocation_receipt_notification_stack_summary_json(
            BerkeleyAppPersistedEditorState {
                selected_syntax_card_index: Some(3),
                active_command_id: Some("analysis.3.inspect-waveform".to_string()),
            },
            "queued dispatch",
            Some("dashboard.dispatch-queue-lane-tab-panel-card-action-menu-group-shortcut-command-palette-search-result.queued".to_string()),
        )
        .unwrap(),
    )
    .expect(
        "shell dashboard dispatch queue lane tab panel card action menu group shortcut command palette search invocation receipt notification stack summary JSON should parse",
    );
    assert_eq!(
        accepted_notification_stack_summary_payload["summaryKind"],
        "visible"
    );
    assert_eq!(
        accepted_notification_stack_summary_payload["headlineNotificationKind"],
        "dispatch-accepted"
    );
    assert_eq!(
        accepted_notification_stack_summary_payload["shouldAnnounceLatest"],
        true
    );
    assert_eq!(
        accepted_notification_stack_summary_payload
            ["dispatchQueueLaneTabPanelCardActionMenuGroupShortcutCommandPaletteSearchInvocationReceiptNotificationStackSummaryCapabilityId"],
        "app-shell-dashboard-dispatch-queue-lane-tab-panel-card-action-menu-group-shortcut-command-palette-search-invocation-receipt-notification-stack-summary-json"
    );

    let accepted_notification_stack_summary_product_handoff =
        app.run_app_shell_dashboard_dispatch_queue_lane_tab_panel_card_action_menu_group_shortcut_command_palette_search_invocation_receipt_notification_stack_summary_product_handoff(
            BerkeleyAppPersistedEditorState {
                selected_syntax_card_index: Some(3),
                active_command_id: Some("analysis.3.inspect-waveform".to_string()),
            },
            "queued dispatch",
            Some("dashboard.dispatch-queue-lane-tab-panel-card-action-menu-group-shortcut-command-palette-search-result.queued".to_string()),
        )
        .expect(
            "shell dashboard dispatch queue lane tab panel card action menu group shortcut command palette search invocation receipt notification stack summary product handoff should execute",
        );
    assert_eq!(
        accepted_notification_stack_summary_product_handoff.schema_version,
        BERKELEY_APP_SHELL_DASHBOARD_DISPATCH_QUEUE_LANE_TAB_PANEL_CARD_ACTION_MENU_GROUP_SHORTCUT_COMMAND_PALETTE_SEARCH_INVOCATION_RECEIPT_NOTIFICATION_STACK_SUMMARY_PRODUCT_HANDOFF_SCHEMA_VERSION
    );
    assert_eq!(
        accepted_notification_stack_summary_product_handoff.handoff_route,
        "notification-stack"
    );
    assert_eq!(
        accepted_notification_stack_summary_product_handoff.product_shell_action,
        "render-notification-stack"
    );
    assert_eq!(
        accepted_notification_stack_summary_product_handoff.notification_badge_label,
        "1 visible notification"
    );
    assert_eq!(
        accepted_notification_stack_summary_product_handoff
            .live_region_id
            .as_deref(),
        Some("app-shell-command-palette-receipt-live-region")
    );
    assert_eq!(
        accepted_notification_stack_summary_product_handoff
            .stack_summary
            .summary_kind,
        accepted_notification_stack_summary.summary_kind
    );

    let accepted_notification_stack_summary_product_handoff_payload: serde_json::Value = serde_json::from_str(
        &app.run_app_shell_dashboard_dispatch_queue_lane_tab_panel_card_action_menu_group_shortcut_command_palette_search_invocation_receipt_notification_stack_summary_product_handoff_json(
            BerkeleyAppPersistedEditorState {
                selected_syntax_card_index: Some(3),
                active_command_id: Some("analysis.3.inspect-waveform".to_string()),
            },
            "queued dispatch",
            Some("dashboard.dispatch-queue-lane-tab-panel-card-action-menu-group-shortcut-command-palette-search-result.queued".to_string()),
        )
        .unwrap(),
    )
    .expect(
        "shell dashboard dispatch queue lane tab panel card action menu group shortcut command palette search invocation receipt notification stack summary product handoff JSON should parse",
    );
    assert_eq!(
        accepted_notification_stack_summary_product_handoff_payload["handoffRoute"],
        "notification-stack"
    );
    assert_eq!(
        accepted_notification_stack_summary_product_handoff_payload["productShellAction"],
        "render-notification-stack"
    );
    assert_eq!(
        accepted_notification_stack_summary_product_handoff_payload["stackSummary"]["summaryKind"],
        "visible"
    );
    assert_eq!(
        accepted_notification_stack_summary_product_handoff_payload
            ["dispatchQueueLaneTabPanelCardActionMenuGroupShortcutCommandPaletteSearchInvocationReceiptNotificationStackSummaryProductHandoffCapabilityId"],
        "app-shell-dashboard-dispatch-queue-lane-tab-panel-card-action-menu-group-shortcut-command-palette-search-invocation-receipt-notification-stack-summary-product-handoff-json"
    );

    let accepted_notification_stack_summary_product_handoff_delivery_package =
        app.run_app_shell_dashboard_dispatch_queue_lane_tab_panel_card_action_menu_group_shortcut_command_palette_search_invocation_receipt_notification_stack_summary_product_handoff_delivery_package(
            BerkeleyAppPersistedEditorState {
                selected_syntax_card_index: Some(3),
                active_command_id: Some("analysis.3.inspect-waveform".to_string()),
            },
            "queued dispatch",
            Some("dashboard.dispatch-queue-lane-tab-panel-card-action-menu-group-shortcut-command-palette-search-result.queued".to_string()),
        )
        .expect(
            "shell dashboard dispatch queue lane tab panel card action menu group shortcut command palette search invocation receipt notification stack summary product handoff delivery package should execute",
        );
    assert_eq!(
        accepted_notification_stack_summary_product_handoff_delivery_package.schema_version,
        BERKELEY_APP_SHELL_DASHBOARD_DISPATCH_QUEUE_LANE_TAB_PANEL_CARD_ACTION_MENU_GROUP_SHORTCUT_COMMAND_PALETTE_SEARCH_INVOCATION_RECEIPT_NOTIFICATION_STACK_SUMMARY_PRODUCT_HANDOFF_DELIVERY_PACKAGE_SCHEMA_VERSION
    );
    assert_eq!(
        accepted_notification_stack_summary_product_handoff_delivery_package.delivery_package_kind,
        "berkeley-product-handoff-delivery-package"
    );
    assert_eq!(
        accepted_notification_stack_summary_product_handoff_delivery_package.delivery_route,
        "ready/notification-stack"
    );
    assert_eq!(
        accepted_notification_stack_summary_product_handoff_delivery_package.wasm_export_symbol,
        "berkeleyAppProductHandoffDeliveryPackage"
    );
    assert_eq!(
        accepted_notification_stack_summary_product_handoff_delivery_package.hydration_target_id,
        "berkeley-product-handoff-hydration-root"
    );
    assert!(accepted_notification_stack_summary_product_handoff_delivery_package.should_hydrate);
    assert_eq!(
        accepted_notification_stack_summary_product_handoff_delivery_package
            .product_handoff
            .handoff_route,
        accepted_notification_stack_summary_product_handoff.handoff_route
    );

    let accepted_notification_stack_summary_product_handoff_delivery_package_payload: serde_json::Value = serde_json::from_str(
        &app.run_app_shell_dashboard_dispatch_queue_lane_tab_panel_card_action_menu_group_shortcut_command_palette_search_invocation_receipt_notification_stack_summary_product_handoff_delivery_package_json(
            BerkeleyAppPersistedEditorState {
                selected_syntax_card_index: Some(3),
                active_command_id: Some("analysis.3.inspect-waveform".to_string()),
            },
            "queued dispatch",
            Some("dashboard.dispatch-queue-lane-tab-panel-card-action-menu-group-shortcut-command-palette-search-result.queued".to_string()),
        )
        .unwrap(),
    )
    .expect(
        "shell dashboard dispatch queue lane tab panel card action menu group shortcut command palette search invocation receipt notification stack summary product handoff delivery package JSON should parse",
    );
    assert_eq!(
        accepted_notification_stack_summary_product_handoff_delivery_package_payload
            ["deliveryPackageKind"],
        "berkeley-product-handoff-delivery-package"
    );
    assert_eq!(
        accepted_notification_stack_summary_product_handoff_delivery_package_payload
            ["wasmExportSymbol"],
        "berkeleyAppProductHandoffDeliveryPackage"
    );
    assert_eq!(
        accepted_notification_stack_summary_product_handoff_delivery_package_payload
            ["shouldHydrate"],
        true
    );
    assert_eq!(
        accepted_notification_stack_summary_product_handoff_delivery_package_payload
            ["productHandoff"]["handoffRoute"],
        "notification-stack"
    );
    assert_eq!(
        accepted_notification_stack_summary_product_handoff_delivery_package_payload
            ["dispatchQueueLaneTabPanelCardActionMenuGroupShortcutCommandPaletteSearchInvocationReceiptNotificationStackSummaryProductHandoffDeliveryPackageCapabilityId"],
        "app-shell-dashboard-dispatch-queue-lane-tab-panel-card-action-menu-group-shortcut-command-palette-search-invocation-receipt-notification-stack-summary-product-handoff-delivery-package-json"
    );

    let accepted_notification_stack_summary_product_handoff_delivery_package_embed_manifest =
        app.run_app_shell_dashboard_dispatch_queue_lane_tab_panel_card_action_menu_group_shortcut_command_palette_search_invocation_receipt_notification_stack_summary_product_handoff_delivery_package_embed_manifest(
            BerkeleyAppPersistedEditorState {
                selected_syntax_card_index: Some(3),
                active_command_id: Some("analysis.3.inspect-waveform".to_string()),
            },
            "queued dispatch",
            Some("dashboard.dispatch-queue-lane-tab-panel-card-action-menu-group-shortcut-command-palette-search-result.queued".to_string()),
        )
        .expect(
            "shell dashboard dispatch queue lane tab panel card action menu group shortcut command palette search invocation receipt notification stack summary product handoff delivery package embed manifest should execute",
        );
    assert_eq!(
        accepted_notification_stack_summary_product_handoff_delivery_package_embed_manifest
            .schema_version,
        BERKELEY_APP_SHELL_DASHBOARD_DISPATCH_QUEUE_LANE_TAB_PANEL_CARD_ACTION_MENU_GROUP_SHORTCUT_COMMAND_PALETTE_SEARCH_INVOCATION_RECEIPT_NOTIFICATION_STACK_SUMMARY_PRODUCT_HANDOFF_DELIVERY_PACKAGE_EMBED_MANIFEST_SCHEMA_VERSION
    );
    assert_eq!(
        accepted_notification_stack_summary_product_handoff_delivery_package_embed_manifest
            .embed_manifest_kind,
        "berkeley-product-handoff-webassembly-embed-manifest"
    );
    assert_eq!(
        accepted_notification_stack_summary_product_handoff_delivery_package_embed_manifest
            .wasm_module_name,
        "berkeley_spice_mosaic_app"
    );
    assert_eq!(
        accepted_notification_stack_summary_product_handoff_delivery_package_embed_manifest
            .wasm_import_namespace,
        "berkeley_spice_mosaic"
    );
    assert_eq!(
        accepted_notification_stack_summary_product_handoff_delivery_package_embed_manifest
            .hydration_mode,
        "hydrate"
    );
    assert!(
        accepted_notification_stack_summary_product_handoff_delivery_package_embed_manifest
            .should_preload
    );
    assert!(
        accepted_notification_stack_summary_product_handoff_delivery_package_embed_manifest
            .should_instantiate
    );
    assert!(
        accepted_notification_stack_summary_product_handoff_delivery_package_embed_manifest
            .should_mount
    );
    assert_eq!(
        accepted_notification_stack_summary_product_handoff_delivery_package_embed_manifest
            .delivery_package
            .delivery_package_id,
        accepted_notification_stack_summary_product_handoff_delivery_package.delivery_package_id
    );

    let accepted_notification_stack_summary_product_handoff_delivery_package_embed_manifest_payload: serde_json::Value = serde_json::from_str(
        &app.run_app_shell_dashboard_dispatch_queue_lane_tab_panel_card_action_menu_group_shortcut_command_palette_search_invocation_receipt_notification_stack_summary_product_handoff_delivery_package_embed_manifest_json(
            BerkeleyAppPersistedEditorState {
                selected_syntax_card_index: Some(3),
                active_command_id: Some("analysis.3.inspect-waveform".to_string()),
            },
            "queued dispatch",
            Some("dashboard.dispatch-queue-lane-tab-panel-card-action-menu-group-shortcut-command-palette-search-result.queued".to_string()),
        )
        .unwrap(),
    )
    .expect(
        "shell dashboard dispatch queue lane tab panel card action menu group shortcut command palette search invocation receipt notification stack summary product handoff delivery package embed manifest JSON should parse",
    );
    assert_eq!(
        accepted_notification_stack_summary_product_handoff_delivery_package_embed_manifest_payload
            ["embedManifestKind"],
        "berkeley-product-handoff-webassembly-embed-manifest"
    );
    assert_eq!(
        accepted_notification_stack_summary_product_handoff_delivery_package_embed_manifest_payload
            ["wasmInitializerSymbol"],
        "berkeleyAppProductHandoffEmbedManifest"
    );
    assert_eq!(
        accepted_notification_stack_summary_product_handoff_delivery_package_embed_manifest_payload
            ["hydrationMode"],
        "hydrate"
    );
    assert_eq!(
        accepted_notification_stack_summary_product_handoff_delivery_package_embed_manifest_payload
            ["deliveryPackage"]["deliveryPackageKind"],
        "berkeley-product-handoff-delivery-package"
    );
    assert_eq!(
        accepted_notification_stack_summary_product_handoff_delivery_package_embed_manifest_payload
            ["dispatchQueueLaneTabPanelCardActionMenuGroupShortcutCommandPaletteSearchInvocationReceiptNotificationStackSummaryProductHandoffDeliveryPackageEmbedManifestCapabilityId"],
        "app-shell-dashboard-dispatch-queue-lane-tab-panel-card-action-menu-group-shortcut-command-palette-search-invocation-receipt-notification-stack-summary-product-handoff-delivery-package-embed-manifest-json"
    );

    let accepted_notification_stack_summary_product_handoff_delivery_package_embed_loader_plan =
        app.run_app_shell_dashboard_dispatch_queue_lane_tab_panel_card_action_menu_group_shortcut_command_palette_search_invocation_receipt_notification_stack_summary_product_handoff_delivery_package_embed_loader_plan(
            BerkeleyAppPersistedEditorState {
                selected_syntax_card_index: Some(3),
                active_command_id: Some("analysis.3.inspect-waveform".to_string()),
            },
            "queued dispatch",
            Some("dashboard.dispatch-queue-lane-tab-panel-card-action-menu-group-shortcut-command-palette-search-result.queued".to_string()),
        )
        .expect(
            "shell dashboard dispatch queue lane tab panel card action menu group shortcut command palette search invocation receipt notification stack summary product handoff delivery package embed loader plan should execute",
        );
    assert_eq!(
        accepted_notification_stack_summary_product_handoff_delivery_package_embed_loader_plan
            .schema_version,
        BERKELEY_APP_SHELL_DASHBOARD_DISPATCH_QUEUE_LANE_TAB_PANEL_CARD_ACTION_MENU_GROUP_SHORTCUT_COMMAND_PALETTE_SEARCH_INVOCATION_RECEIPT_NOTIFICATION_STACK_SUMMARY_PRODUCT_HANDOFF_DELIVERY_PACKAGE_EMBED_LOADER_PLAN_SCHEMA_VERSION
    );
    assert_eq!(
        accepted_notification_stack_summary_product_handoff_delivery_package_embed_loader_plan
            .loader_plan_kind,
        "berkeley-product-handoff-webassembly-embed-loader-plan"
    );
    assert_eq!(
        accepted_notification_stack_summary_product_handoff_delivery_package_embed_loader_plan
            .loader_phase,
        "ready"
    );
    assert_eq!(
        accepted_notification_stack_summary_product_handoff_delivery_package_embed_loader_plan
            .load_order,
        vec![
            "preload-module".to_string(),
            "instantiate-module".to_string(),
            "mount-embed-root".to_string(),
            "hydrate-product-handoff".to_string()
        ]
    );
    assert_eq!(
        accepted_notification_stack_summary_product_handoff_delivery_package_embed_loader_plan
            .module_integrity_mode,
        "source-fingerprint"
    );
    assert_eq!(
        accepted_notification_stack_summary_product_handoff_delivery_package_embed_loader_plan
            .embed_manifest
            .embed_manifest_id,
        accepted_notification_stack_summary_product_handoff_delivery_package_embed_manifest
            .embed_manifest_id
    );

    let accepted_notification_stack_summary_product_handoff_delivery_package_embed_loader_plan_payload: serde_json::Value = serde_json::from_str(
        &app.run_app_shell_dashboard_dispatch_queue_lane_tab_panel_card_action_menu_group_shortcut_command_palette_search_invocation_receipt_notification_stack_summary_product_handoff_delivery_package_embed_loader_plan_json(
            BerkeleyAppPersistedEditorState {
                selected_syntax_card_index: Some(3),
                active_command_id: Some("analysis.3.inspect-waveform".to_string()),
            },
            "queued dispatch",
            Some("dashboard.dispatch-queue-lane-tab-panel-card-action-menu-group-shortcut-command-palette-search-result.queued".to_string()),
        )
        .unwrap(),
    )
    .expect(
        "shell dashboard dispatch queue lane tab panel card action menu group shortcut command palette search invocation receipt notification stack summary product handoff delivery package embed loader plan JSON should parse",
    );
    assert_eq!(
        accepted_notification_stack_summary_product_handoff_delivery_package_embed_loader_plan_payload
            ["loaderStrategy"],
        "preload-instantiate-mount-hydrate"
    );
    assert_eq!(
        accepted_notification_stack_summary_product_handoff_delivery_package_embed_loader_plan_payload
            ["moduleFetchPriority"],
        "auto"
    );
    assert_eq!(
        accepted_notification_stack_summary_product_handoff_delivery_package_embed_loader_plan_payload
            ["loadOrder"][3],
        "hydrate-product-handoff"
    );
    assert_eq!(
        accepted_notification_stack_summary_product_handoff_delivery_package_embed_loader_plan_payload
            ["embedManifest"]["embedManifestKind"],
        "berkeley-product-handoff-webassembly-embed-manifest"
    );
    assert_eq!(
        accepted_notification_stack_summary_product_handoff_delivery_package_embed_loader_plan_payload
            ["dispatchQueueLaneTabPanelCardActionMenuGroupShortcutCommandPaletteSearchInvocationReceiptNotificationStackSummaryProductHandoffDeliveryPackageEmbedLoaderPlanCapabilityId"],
        "app-shell-dashboard-dispatch-queue-lane-tab-panel-card-action-menu-group-shortcut-command-palette-search-invocation-receipt-notification-stack-summary-product-handoff-delivery-package-embed-loader-plan-json"
    );

    let accepted_notification_stack_summary_product_handoff_delivery_package_embed_runtime_plan =
        app.run_app_shell_dashboard_dispatch_queue_lane_tab_panel_card_action_menu_group_shortcut_command_palette_search_invocation_receipt_notification_stack_summary_product_handoff_delivery_package_embed_runtime_plan(
            BerkeleyAppPersistedEditorState {
                selected_syntax_card_index: Some(3),
                active_command_id: Some("analysis.3.inspect-waveform".to_string()),
            },
            "queued dispatch",
            Some("dashboard.dispatch-queue-lane-tab-panel-card-action-menu-group-shortcut-command-palette-search-result.queued".to_string()),
        )
        .expect(
            "shell dashboard dispatch queue lane tab panel card action menu group shortcut command palette search invocation receipt notification stack summary product handoff delivery package embed runtime plan should execute",
        );
    assert_eq!(
        accepted_notification_stack_summary_product_handoff_delivery_package_embed_runtime_plan
            .schema_version,
        BERKELEY_APP_SHELL_DASHBOARD_DISPATCH_QUEUE_LANE_TAB_PANEL_CARD_ACTION_MENU_GROUP_SHORTCUT_COMMAND_PALETTE_SEARCH_INVOCATION_RECEIPT_NOTIFICATION_STACK_SUMMARY_PRODUCT_HANDOFF_DELIVERY_PACKAGE_EMBED_RUNTIME_PLAN_SCHEMA_VERSION
    );
    assert_eq!(
        accepted_notification_stack_summary_product_handoff_delivery_package_embed_runtime_plan
            .runtime_plan_kind,
        "berkeley-product-handoff-webassembly-embed-runtime-plan"
    );
    assert_eq!(
        accepted_notification_stack_summary_product_handoff_delivery_package_embed_runtime_plan
            .runtime_phase,
        "bootstrappable"
    );
    assert_eq!(
        accepted_notification_stack_summary_product_handoff_delivery_package_embed_runtime_plan
            .runtime_strategy,
        "start-runtime-mount-and-hydrate"
    );
    assert!(
        accepted_notification_stack_summary_product_handoff_delivery_package_embed_runtime_plan
            .should_start_runtime
    );
    assert_eq!(
        accepted_notification_stack_summary_product_handoff_delivery_package_embed_runtime_plan
            .runtime_steps
            .last()
            .map(String::as_str),
        Some("publish-runtime-ready")
    );
    assert_eq!(
        accepted_notification_stack_summary_product_handoff_delivery_package_embed_runtime_plan
            .embed_loader_plan
            .loader_plan_id,
        accepted_notification_stack_summary_product_handoff_delivery_package_embed_loader_plan
            .loader_plan_id
    );

    let accepted_notification_stack_summary_product_handoff_delivery_package_embed_runtime_plan_payload: serde_json::Value = serde_json::from_str(
        &app.run_app_shell_dashboard_dispatch_queue_lane_tab_panel_card_action_menu_group_shortcut_command_palette_search_invocation_receipt_notification_stack_summary_product_handoff_delivery_package_embed_runtime_plan_json(
            BerkeleyAppPersistedEditorState {
                selected_syntax_card_index: Some(3),
                active_command_id: Some("analysis.3.inspect-waveform".to_string()),
            },
            "queued dispatch",
            Some("dashboard.dispatch-queue-lane-tab-panel-card-action-menu-group-shortcut-command-palette-search-result.queued".to_string()),
        )
        .unwrap(),
    )
    .expect(
        "shell dashboard dispatch queue lane tab panel card action menu group shortcut command palette search invocation receipt notification stack summary product handoff delivery package embed runtime plan JSON should parse",
    );
    assert_eq!(
        accepted_notification_stack_summary_product_handoff_delivery_package_embed_runtime_plan_payload
            ["runtimeHostKind"],
        "mosaic-webassembly-product-shell-host"
    );
    assert_eq!(
        accepted_notification_stack_summary_product_handoff_delivery_package_embed_runtime_plan_payload
            ["runtimeSteps"][5],
        "hydrate-product-handoff"
    );
    assert_eq!(
        accepted_notification_stack_summary_product_handoff_delivery_package_embed_runtime_plan_payload
            ["embedLoaderPlan"]["loaderPlanKind"],
        "berkeley-product-handoff-webassembly-embed-loader-plan"
    );
    assert_eq!(
        accepted_notification_stack_summary_product_handoff_delivery_package_embed_runtime_plan_payload
            ["dispatchQueueLaneTabPanelCardActionMenuGroupShortcutCommandPaletteSearchInvocationReceiptNotificationStackSummaryProductHandoffDeliveryPackageEmbedRuntimePlanCapabilityId"],
        "app-shell-dashboard-dispatch-queue-lane-tab-panel-card-action-menu-group-shortcut-command-palette-search-invocation-receipt-notification-stack-summary-product-handoff-delivery-package-embed-runtime-plan-json"
    );

    let accepted_notification_stack_summary_product_handoff_delivery_package_embed_runtime_session_plan =
        app.run_app_shell_dashboard_dispatch_queue_lane_tab_panel_card_action_menu_group_shortcut_command_palette_search_invocation_receipt_notification_stack_summary_product_handoff_delivery_package_embed_runtime_session_plan(
            BerkeleyAppPersistedEditorState {
                selected_syntax_card_index: Some(3),
                active_command_id: Some("analysis.3.inspect-waveform".to_string()),
            },
            "queued dispatch",
            Some("dashboard.dispatch-queue-lane-tab-panel-card-action-menu-group-shortcut-command-palette-search-result.queued".to_string()),
        )
        .expect(
            "shell dashboard dispatch queue lane tab panel card action menu group shortcut command palette search invocation receipt notification stack summary product handoff delivery package embed runtime session plan should execute",
        );
    assert_eq!(
        accepted_notification_stack_summary_product_handoff_delivery_package_embed_runtime_session_plan
            .schema_version,
        BERKELEY_APP_SHELL_DASHBOARD_DISPATCH_QUEUE_LANE_TAB_PANEL_CARD_ACTION_MENU_GROUP_SHORTCUT_COMMAND_PALETTE_SEARCH_INVOCATION_RECEIPT_NOTIFICATION_STACK_SUMMARY_PRODUCT_HANDOFF_DELIVERY_PACKAGE_EMBED_RUNTIME_SESSION_PLAN_SCHEMA_VERSION
    );
    assert_eq!(
        accepted_notification_stack_summary_product_handoff_delivery_package_embed_runtime_session_plan
            .runtime_session_plan_kind,
        "berkeley-product-handoff-webassembly-embed-runtime-session-plan"
    );
    assert_eq!(
        accepted_notification_stack_summary_product_handoff_delivery_package_embed_runtime_session_plan
            .runtime_session_phase,
        "activatable"
    );
    assert_eq!(
        accepted_notification_stack_summary_product_handoff_delivery_package_embed_runtime_session_plan
            .runtime_session_strategy,
        "open-session-start-runtime-publish-ready"
    );
    assert!(
        accepted_notification_stack_summary_product_handoff_delivery_package_embed_runtime_session_plan
            .should_publish_ready
    );
    assert_eq!(
        accepted_notification_stack_summary_product_handoff_delivery_package_embed_runtime_session_plan
            .session_steps
            .last()
            .map(String::as_str),
        Some("publish-runtime-session-ready")
    );
    assert_eq!(
        accepted_notification_stack_summary_product_handoff_delivery_package_embed_runtime_session_plan
            .embed_runtime_plan
            .runtime_plan_id,
        accepted_notification_stack_summary_product_handoff_delivery_package_embed_runtime_plan
            .runtime_plan_id
    );

    let accepted_notification_stack_summary_product_handoff_delivery_package_embed_runtime_session_plan_payload: serde_json::Value = serde_json::from_str(
        &app.run_app_shell_dashboard_dispatch_queue_lane_tab_panel_card_action_menu_group_shortcut_command_palette_search_invocation_receipt_notification_stack_summary_product_handoff_delivery_package_embed_runtime_session_plan_json(
            BerkeleyAppPersistedEditorState {
                selected_syntax_card_index: Some(3),
                active_command_id: Some("analysis.3.inspect-waveform".to_string()),
            },
            "queued dispatch",
            Some("dashboard.dispatch-queue-lane-tab-panel-card-action-menu-group-shortcut-command-palette-search-result.queued".to_string()),
        )
        .unwrap(),
    )
    .expect(
        "shell dashboard dispatch queue lane tab panel card action menu group shortcut command palette search invocation receipt notification stack summary product handoff delivery package embed runtime session plan JSON should parse",
    );
    assert_eq!(
        accepted_notification_stack_summary_product_handoff_delivery_package_embed_runtime_session_plan_payload
            ["runtimeOwnerKind"],
        "mosaic-product-shell-runtime-session-owner"
    );
    assert_eq!(
        accepted_notification_stack_summary_product_handoff_delivery_package_embed_runtime_session_plan_payload
            ["sessionSteps"][0],
        "allocate-runtime-session"
    );
    assert_eq!(
        accepted_notification_stack_summary_product_handoff_delivery_package_embed_runtime_session_plan_payload
            ["embedRuntimePlan"]["runtimePlanKind"],
        "berkeley-product-handoff-webassembly-embed-runtime-plan"
    );
    assert_eq!(
        accepted_notification_stack_summary_product_handoff_delivery_package_embed_runtime_session_plan_payload
            ["dispatchQueueLaneTabPanelCardActionMenuGroupShortcutCommandPaletteSearchInvocationReceiptNotificationStackSummaryProductHandoffDeliveryPackageEmbedRuntimeSessionPlanCapabilityId"],
        "app-shell-dashboard-dispatch-queue-lane-tab-panel-card-action-menu-group-shortcut-command-palette-search-invocation-receipt-notification-stack-summary-product-handoff-delivery-package-embed-runtime-session-plan-json"
    );

    let accepted_notification_stack_summary_product_handoff_delivery_package_embed_runtime_activation_plan =
        app.run_app_shell_dashboard_dispatch_queue_lane_tab_panel_card_action_menu_group_shortcut_command_palette_search_invocation_receipt_notification_stack_summary_product_handoff_delivery_package_embed_runtime_activation_plan(
            BerkeleyAppPersistedEditorState {
                selected_syntax_card_index: Some(3),
                active_command_id: Some("analysis.3.inspect-waveform".to_string()),
            },
            "queued dispatch",
            Some("dashboard.dispatch-queue-lane-tab-panel-card-action-menu-group-shortcut-command-palette-search-result.queued".to_string()),
        )
        .expect(
            "shell dashboard dispatch queue lane tab panel card action menu group shortcut command palette search invocation receipt notification stack summary product handoff delivery package embed runtime activation plan should execute",
        );
    assert_eq!(
        accepted_notification_stack_summary_product_handoff_delivery_package_embed_runtime_activation_plan
            .schema_version,
        BERKELEY_APP_SHELL_DASHBOARD_DISPATCH_QUEUE_LANE_TAB_PANEL_CARD_ACTION_MENU_GROUP_SHORTCUT_COMMAND_PALETTE_SEARCH_INVOCATION_RECEIPT_NOTIFICATION_STACK_SUMMARY_PRODUCT_HANDOFF_DELIVERY_PACKAGE_EMBED_RUNTIME_ACTIVATION_PLAN_SCHEMA_VERSION
    );
    assert_eq!(
        accepted_notification_stack_summary_product_handoff_delivery_package_embed_runtime_activation_plan
            .runtime_activation_plan_kind,
        "berkeley-product-handoff-webassembly-embed-runtime-activation-plan"
    );
    assert_eq!(
        accepted_notification_stack_summary_product_handoff_delivery_package_embed_runtime_activation_plan
            .runtime_activation_phase,
        "ready-to-activate"
    );
    assert_eq!(
        accepted_notification_stack_summary_product_handoff_delivery_package_embed_runtime_activation_plan
            .runtime_activation_strategy,
        "request-session-activation-publish-ready"
    );
    assert!(
        accepted_notification_stack_summary_product_handoff_delivery_package_embed_runtime_activation_plan
            .should_request_activation
    );
    assert_eq!(
        accepted_notification_stack_summary_product_handoff_delivery_package_embed_runtime_activation_plan
            .activation_steps
            .last()
            .map(String::as_str),
        Some("publish-runtime-activation-ready")
    );
    assert_eq!(
        accepted_notification_stack_summary_product_handoff_delivery_package_embed_runtime_activation_plan
            .embed_runtime_session_plan
            .runtime_session_plan_id,
        accepted_notification_stack_summary_product_handoff_delivery_package_embed_runtime_session_plan
            .runtime_session_plan_id
    );

    let accepted_notification_stack_summary_product_handoff_delivery_package_embed_runtime_activation_plan_payload: serde_json::Value = serde_json::from_str(
        &app.run_app_shell_dashboard_dispatch_queue_lane_tab_panel_card_action_menu_group_shortcut_command_palette_search_invocation_receipt_notification_stack_summary_product_handoff_delivery_package_embed_runtime_activation_plan_json(
            BerkeleyAppPersistedEditorState {
                selected_syntax_card_index: Some(3),
                active_command_id: Some("analysis.3.inspect-waveform".to_string()),
            },
            "queued dispatch",
            Some("dashboard.dispatch-queue-lane-tab-panel-card-action-menu-group-shortcut-command-palette-search-result.queued".to_string()),
        )
        .unwrap(),
    )
    .expect(
        "shell dashboard dispatch queue lane tab panel card action menu group shortcut command palette search invocation receipt notification stack summary product handoff delivery package embed runtime activation plan JSON should parse",
    );
    assert_eq!(
        accepted_notification_stack_summary_product_handoff_delivery_package_embed_runtime_activation_plan_payload
            ["activationChannelKind"],
        "mosaic-product-shell-runtime-activation-channel"
    );
    assert_eq!(
        accepted_notification_stack_summary_product_handoff_delivery_package_embed_runtime_activation_plan_payload
            ["activationSteps"][0],
        "prepare-runtime-activation"
    );
    assert_eq!(
        accepted_notification_stack_summary_product_handoff_delivery_package_embed_runtime_activation_plan_payload
            ["embedRuntimeSessionPlan"]["runtimeSessionPlanKind"],
        "berkeley-product-handoff-webassembly-embed-runtime-session-plan"
    );
    assert_eq!(
        accepted_notification_stack_summary_product_handoff_delivery_package_embed_runtime_activation_plan_payload
            ["dispatchQueueLaneTabPanelCardActionMenuGroupShortcutCommandPaletteSearchInvocationReceiptNotificationStackSummaryProductHandoffDeliveryPackageEmbedRuntimeActivationPlanCapabilityId"],
        "app-shell-dashboard-dispatch-queue-lane-tab-panel-card-action-menu-group-shortcut-command-palette-search-invocation-receipt-notification-stack-summary-product-handoff-delivery-package-embed-runtime-activation-plan-json"
    );

    let accepted_notification_stack_summary_product_handoff_delivery_package_embed_runtime_activation_receipt =
        app.run_app_shell_dashboard_dispatch_queue_lane_tab_panel_card_action_menu_group_shortcut_command_palette_search_invocation_receipt_notification_stack_summary_product_handoff_delivery_package_embed_runtime_activation_receipt(
            BerkeleyAppPersistedEditorState {
                selected_syntax_card_index: Some(3),
                active_command_id: Some("analysis.3.inspect-waveform".to_string()),
            },
            "queued dispatch",
            Some("dashboard.dispatch-queue-lane-tab-panel-card-action-menu-group-shortcut-command-palette-search-result.queued".to_string()),
        )
        .expect(
            "shell dashboard dispatch queue lane tab panel card action menu group shortcut command palette search invocation receipt notification stack summary product handoff delivery package embed runtime activation receipt should execute",
        );
    assert_eq!(
        accepted_notification_stack_summary_product_handoff_delivery_package_embed_runtime_activation_receipt
            .schema_version,
        BERKELEY_APP_SHELL_DASHBOARD_DISPATCH_QUEUE_LANE_TAB_PANEL_CARD_ACTION_MENU_GROUP_SHORTCUT_COMMAND_PALETTE_SEARCH_INVOCATION_RECEIPT_NOTIFICATION_STACK_SUMMARY_PRODUCT_HANDOFF_DELIVERY_PACKAGE_EMBED_RUNTIME_ACTIVATION_RECEIPT_SCHEMA_VERSION
    );
    assert_eq!(
        accepted_notification_stack_summary_product_handoff_delivery_package_embed_runtime_activation_receipt
            .runtime_activation_receipt_kind,
        "berkeley-product-handoff-webassembly-embed-runtime-activation-receipt"
    );
    assert_eq!(
        accepted_notification_stack_summary_product_handoff_delivery_package_embed_runtime_activation_receipt
            .runtime_activation_receipt_status,
        "accepted"
    );
    assert_eq!(
        accepted_notification_stack_summary_product_handoff_delivery_package_embed_runtime_activation_receipt
            .runtime_activation_receipt_outcome,
        "ready"
    );
    assert!(
        accepted_notification_stack_summary_product_handoff_delivery_package_embed_runtime_activation_receipt
            .should_record_receipt
    );
    assert!(
        !accepted_notification_stack_summary_product_handoff_delivery_package_embed_runtime_activation_receipt
            .should_defer_receipt
    );
    assert_eq!(
        accepted_notification_stack_summary_product_handoff_delivery_package_embed_runtime_activation_receipt
            .activation_receipt_steps
            .last()
            .map(String::as_str),
        Some("publish-runtime-activation-receipt-ready")
    );
    assert_eq!(
        accepted_notification_stack_summary_product_handoff_delivery_package_embed_runtime_activation_receipt
            .embed_runtime_activation_plan
            .runtime_activation_plan_id,
        accepted_notification_stack_summary_product_handoff_delivery_package_embed_runtime_activation_plan
            .runtime_activation_plan_id
    );

    let accepted_notification_stack_summary_product_handoff_delivery_package_embed_runtime_activation_receipt_payload: serde_json::Value = serde_json::from_str(
        &app.run_app_shell_dashboard_dispatch_queue_lane_tab_panel_card_action_menu_group_shortcut_command_palette_search_invocation_receipt_notification_stack_summary_product_handoff_delivery_package_embed_runtime_activation_receipt_json(
            BerkeleyAppPersistedEditorState {
                selected_syntax_card_index: Some(3),
                active_command_id: Some("analysis.3.inspect-waveform".to_string()),
            },
            "queued dispatch",
            Some("dashboard.dispatch-queue-lane-tab-panel-card-action-menu-group-shortcut-command-palette-search-result.queued".to_string()),
        )
        .unwrap(),
    )
    .expect(
        "shell dashboard dispatch queue lane tab panel card action menu group shortcut command palette search invocation receipt notification stack summary product handoff delivery package embed runtime activation receipt JSON should parse",
    );
    assert_eq!(
        accepted_notification_stack_summary_product_handoff_delivery_package_embed_runtime_activation_receipt_payload
            ["runtimeActivationReceiptStatus"],
        "accepted"
    );
    assert_eq!(
        accepted_notification_stack_summary_product_handoff_delivery_package_embed_runtime_activation_receipt_payload
            ["activationReceiptSteps"][0],
        "record-runtime-activation-receipt"
    );
    assert_eq!(
        accepted_notification_stack_summary_product_handoff_delivery_package_embed_runtime_activation_receipt_payload
            ["embedRuntimeActivationPlan"]["runtimeActivationPlanKind"],
        "berkeley-product-handoff-webassembly-embed-runtime-activation-plan"
    );
    assert_eq!(
        accepted_notification_stack_summary_product_handoff_delivery_package_embed_runtime_activation_receipt_payload
            ["dispatchQueueLaneTabPanelCardActionMenuGroupShortcutCommandPaletteSearchInvocationReceiptNotificationStackSummaryProductHandoffDeliveryPackageEmbedRuntimeActivationReceiptCapabilityId"],
        "app-shell-dashboard-dispatch-queue-lane-tab-panel-card-action-menu-group-shortcut-command-palette-search-invocation-receipt-notification-stack-summary-product-handoff-delivery-package-embed-runtime-activation-receipt-json"
    );

    let accepted_notification_stack_summary_product_handoff_delivery_package_embed_runtime_activation_receipt_journal =
        app.run_app_shell_dashboard_dispatch_queue_lane_tab_panel_card_action_menu_group_shortcut_command_palette_search_invocation_receipt_notification_stack_summary_product_handoff_delivery_package_embed_runtime_activation_receipt_journal(
            BerkeleyAppPersistedEditorState {
                selected_syntax_card_index: Some(3),
                active_command_id: Some("analysis.3.inspect-waveform".to_string()),
            },
            "queued dispatch",
            Some("dashboard.dispatch-queue-lane-tab-panel-card-action-menu-group-shortcut-command-palette-search-result.queued".to_string()),
        )
        .expect(
            "shell dashboard dispatch queue lane tab panel card action menu group shortcut command palette search invocation receipt notification stack summary product handoff delivery package embed runtime activation receipt journal should execute",
        );
    assert_eq!(
        accepted_notification_stack_summary_product_handoff_delivery_package_embed_runtime_activation_receipt_journal
            .schema_version,
        BERKELEY_APP_SHELL_DASHBOARD_DISPATCH_QUEUE_LANE_TAB_PANEL_CARD_ACTION_MENU_GROUP_SHORTCUT_COMMAND_PALETTE_SEARCH_INVOCATION_RECEIPT_NOTIFICATION_STACK_SUMMARY_PRODUCT_HANDOFF_DELIVERY_PACKAGE_EMBED_RUNTIME_ACTIVATION_RECEIPT_JOURNAL_SCHEMA_VERSION
    );
    assert_eq!(
        accepted_notification_stack_summary_product_handoff_delivery_package_embed_runtime_activation_receipt_journal
            .runtime_activation_receipt_journal_kind,
        "berkeley-product-handoff-webassembly-embed-runtime-activation-receipt-journal"
    );
    assert_eq!(
        accepted_notification_stack_summary_product_handoff_delivery_package_embed_runtime_activation_receipt_journal
            .runtime_activation_receipt_journal_status,
        "committed"
    );
    assert_eq!(
        accepted_notification_stack_summary_product_handoff_delivery_package_embed_runtime_activation_receipt_journal
            .runtime_activation_receipt_journal_outcome,
        "ready"
    );
    assert!(
        accepted_notification_stack_summary_product_handoff_delivery_package_embed_runtime_activation_receipt_journal
            .should_append_journal
    );
    assert!(
        !accepted_notification_stack_summary_product_handoff_delivery_package_embed_runtime_activation_receipt_journal
            .should_defer_journal
    );
    assert_eq!(
        accepted_notification_stack_summary_product_handoff_delivery_package_embed_runtime_activation_receipt_journal
            .activation_receipt_journal_steps
            .last()
            .map(String::as_str),
        Some("publish-runtime-activation-receipt-journal-ready")
    );
    assert_eq!(
        accepted_notification_stack_summary_product_handoff_delivery_package_embed_runtime_activation_receipt_journal
            .embed_runtime_activation_receipt
            .runtime_activation_receipt_id,
        accepted_notification_stack_summary_product_handoff_delivery_package_embed_runtime_activation_receipt
            .runtime_activation_receipt_id
    );

    let accepted_notification_stack_summary_product_handoff_delivery_package_embed_runtime_activation_receipt_journal_payload: serde_json::Value = serde_json::from_str(
        &app.run_app_shell_dashboard_dispatch_queue_lane_tab_panel_card_action_menu_group_shortcut_command_palette_search_invocation_receipt_notification_stack_summary_product_handoff_delivery_package_embed_runtime_activation_receipt_journal_json(
            BerkeleyAppPersistedEditorState {
                selected_syntax_card_index: Some(3),
                active_command_id: Some("analysis.3.inspect-waveform".to_string()),
            },
            "queued dispatch",
            Some("dashboard.dispatch-queue-lane-tab-panel-card-action-menu-group-shortcut-command-palette-search-result.queued".to_string()),
        )
        .unwrap(),
    )
    .expect(
        "shell dashboard dispatch queue lane tab panel card action menu group shortcut command palette search invocation receipt notification stack summary product handoff delivery package embed runtime activation receipt journal JSON should parse",
    );
    assert_eq!(
        accepted_notification_stack_summary_product_handoff_delivery_package_embed_runtime_activation_receipt_journal_payload
            ["runtimeActivationReceiptJournalStatus"],
        "committed"
    );
    assert_eq!(
        accepted_notification_stack_summary_product_handoff_delivery_package_embed_runtime_activation_receipt_journal_payload
            ["activationReceiptJournalSteps"][0],
        "append-runtime-activation-receipt-journal-entry"
    );
    assert_eq!(
        accepted_notification_stack_summary_product_handoff_delivery_package_embed_runtime_activation_receipt_journal_payload
            ["embedRuntimeActivationReceipt"]["runtimeActivationReceiptStatus"],
        "accepted"
    );
    assert_eq!(
        accepted_notification_stack_summary_product_handoff_delivery_package_embed_runtime_activation_receipt_journal_payload
            ["dispatchQueueLaneTabPanelCardActionMenuGroupShortcutCommandPaletteSearchInvocationReceiptNotificationStackSummaryProductHandoffDeliveryPackageEmbedRuntimeActivationReceiptJournalCapabilityId"],
        "app-shell-dashboard-dispatch-queue-lane-tab-panel-card-action-menu-group-shortcut-command-palette-search-invocation-receipt-notification-stack-summary-product-handoff-delivery-package-embed-runtime-activation-receipt-journal-json"
    );

    let accepted_notification_stack_summary_product_handoff_delivery_package_embed_runtime_activation_receipt_journal_summary =
        app.run_app_shell_dashboard_dispatch_queue_lane_tab_panel_card_action_menu_group_shortcut_command_palette_search_invocation_receipt_notification_stack_summary_product_handoff_delivery_package_embed_runtime_activation_receipt_journal_summary(
            BerkeleyAppPersistedEditorState {
                selected_syntax_card_index: Some(3),
                active_command_id: Some("analysis.3.inspect-waveform".to_string()),
            },
            "queued dispatch",
            Some("dashboard.dispatch-queue-lane-tab-panel-card-action-menu-group-shortcut-command-palette-search-result.queued".to_string()),
        )
        .expect(
            "shell dashboard dispatch queue lane tab panel card action menu group shortcut command palette search invocation receipt notification stack summary product handoff delivery package embed runtime activation receipt journal summary should execute",
        );
    assert_eq!(
        accepted_notification_stack_summary_product_handoff_delivery_package_embed_runtime_activation_receipt_journal_summary
            .schema_version,
        BERKELEY_APP_SHELL_DASHBOARD_DISPATCH_QUEUE_LANE_TAB_PANEL_CARD_ACTION_MENU_GROUP_SHORTCUT_COMMAND_PALETTE_SEARCH_INVOCATION_RECEIPT_NOTIFICATION_STACK_SUMMARY_PRODUCT_HANDOFF_DELIVERY_PACKAGE_EMBED_RUNTIME_ACTIVATION_RECEIPT_JOURNAL_SUMMARY_SCHEMA_VERSION
    );
    assert_eq!(
        accepted_notification_stack_summary_product_handoff_delivery_package_embed_runtime_activation_receipt_journal_summary
            .runtime_activation_receipt_journal_summary_kind,
        "berkeley-product-handoff-webassembly-embed-runtime-activation-receipt-journal-summary"
    );
    assert_eq!(
        accepted_notification_stack_summary_product_handoff_delivery_package_embed_runtime_activation_receipt_journal_summary
            .runtime_activation_receipt_journal_summary_status,
        "ready"
    );
    assert_eq!(
        accepted_notification_stack_summary_product_handoff_delivery_package_embed_runtime_activation_receipt_journal_summary
            .journal_entry_count,
        1
    );
    assert_eq!(
        accepted_notification_stack_summary_product_handoff_delivery_package_embed_runtime_activation_receipt_journal_summary
            .committed_journal_entry_count,
        1
    );
    assert_eq!(
        accepted_notification_stack_summary_product_handoff_delivery_package_embed_runtime_activation_receipt_journal_summary
            .deferred_journal_entry_count,
        0
    );
    assert!(
        accepted_notification_stack_summary_product_handoff_delivery_package_embed_runtime_activation_receipt_journal_summary
            .should_summarize_journal
    );
    assert_eq!(
        accepted_notification_stack_summary_product_handoff_delivery_package_embed_runtime_activation_receipt_journal_summary
            .latest_runtime_activation_receipt_id,
        accepted_notification_stack_summary_product_handoff_delivery_package_embed_runtime_activation_receipt_journal
            .runtime_activation_receipt_id
    );
    assert_eq!(
        accepted_notification_stack_summary_product_handoff_delivery_package_embed_runtime_activation_receipt_journal_summary
            .activation_receipt_journal_summary_steps
            .last()
            .map(String::as_str),
        Some("publish-runtime-activation-receipt-journal-summary-ready")
    );

    let accepted_notification_stack_summary_product_handoff_delivery_package_embed_runtime_activation_receipt_journal_summary_payload: serde_json::Value = serde_json::from_str(
        &app.run_app_shell_dashboard_dispatch_queue_lane_tab_panel_card_action_menu_group_shortcut_command_palette_search_invocation_receipt_notification_stack_summary_product_handoff_delivery_package_embed_runtime_activation_receipt_journal_summary_json(
            BerkeleyAppPersistedEditorState {
                selected_syntax_card_index: Some(3),
                active_command_id: Some("analysis.3.inspect-waveform".to_string()),
            },
            "queued dispatch",
            Some("dashboard.dispatch-queue-lane-tab-panel-card-action-menu-group-shortcut-command-palette-search-result.queued".to_string()),
        )
        .unwrap(),
    )
    .expect(
        "shell dashboard dispatch queue lane tab panel card action menu group shortcut command palette search invocation receipt notification stack summary product handoff delivery package embed runtime activation receipt journal summary JSON should parse",
    );
    assert_eq!(
        accepted_notification_stack_summary_product_handoff_delivery_package_embed_runtime_activation_receipt_journal_summary_payload
            ["runtimeActivationReceiptJournalSummaryStatus"],
        "ready"
    );
    assert_eq!(
        accepted_notification_stack_summary_product_handoff_delivery_package_embed_runtime_activation_receipt_journal_summary_payload
            ["committedJournalEntryCount"],
        1
    );
    assert_eq!(
        accepted_notification_stack_summary_product_handoff_delivery_package_embed_runtime_activation_receipt_journal_summary_payload
            ["latestRuntimeActivationReceiptOutcome"],
        "ready"
    );
    assert_eq!(
        accepted_notification_stack_summary_product_handoff_delivery_package_embed_runtime_activation_receipt_journal_summary_payload
            ["embedRuntimeActivationReceiptJournal"]["runtimeActivationReceiptJournalStatus"],
        "committed"
    );
    assert_eq!(
        accepted_notification_stack_summary_product_handoff_delivery_package_embed_runtime_activation_receipt_journal_summary_payload
            ["dispatchQueueLaneTabPanelCardActionMenuGroupShortcutCommandPaletteSearchInvocationReceiptNotificationStackSummaryProductHandoffDeliveryPackageEmbedRuntimeActivationReceiptJournalSummaryCapabilityId"],
        "app-shell-dashboard-dispatch-queue-lane-tab-panel-card-action-menu-group-shortcut-command-palette-search-invocation-receipt-notification-stack-summary-product-handoff-delivery-package-embed-runtime-activation-receipt-journal-summary-json"
    );

    let accepted_notification_stack_summary_product_handoff_delivery_package_embed_runtime_activation_receipt_journal_summary_handoff =
        app.run_app_shell_dashboard_dispatch_queue_lane_tab_panel_card_action_menu_group_shortcut_command_palette_search_invocation_receipt_notification_stack_summary_product_handoff_delivery_package_embed_runtime_activation_receipt_journal_summary_handoff(
            BerkeleyAppPersistedEditorState {
                selected_syntax_card_index: Some(3),
                active_command_id: Some("analysis.3.inspect-waveform".to_string()),
            },
            "queued dispatch",
            Some("dashboard.dispatch-queue-lane-tab-panel-card-action-menu-group-shortcut-command-palette-search-result.queued".to_string()),
        )
        .expect(
            "shell dashboard dispatch queue lane tab panel card action menu group shortcut command palette search invocation receipt notification stack summary product handoff delivery package embed runtime activation receipt journal summary handoff should execute",
        );
    assert_eq!(
        accepted_notification_stack_summary_product_handoff_delivery_package_embed_runtime_activation_receipt_journal_summary_handoff
            .schema_version,
        BERKELEY_APP_SHELL_DASHBOARD_DISPATCH_QUEUE_LANE_TAB_PANEL_CARD_ACTION_MENU_GROUP_SHORTCUT_COMMAND_PALETTE_SEARCH_INVOCATION_RECEIPT_NOTIFICATION_STACK_SUMMARY_PRODUCT_HANDOFF_DELIVERY_PACKAGE_EMBED_RUNTIME_ACTIVATION_RECEIPT_JOURNAL_SUMMARY_HANDOFF_SCHEMA_VERSION
    );
    assert_eq!(
        accepted_notification_stack_summary_product_handoff_delivery_package_embed_runtime_activation_receipt_journal_summary_handoff
            .runtime_activation_receipt_journal_summary_handoff_kind,
        "berkeley-product-handoff-webassembly-embed-runtime-activation-receipt-journal-summary-handoff"
    );
    assert_eq!(
        accepted_notification_stack_summary_product_handoff_delivery_package_embed_runtime_activation_receipt_journal_summary_handoff
            .runtime_activation_receipt_journal_summary_handoff_status,
        "published"
    );
    assert_eq!(
        accepted_notification_stack_summary_product_handoff_delivery_package_embed_runtime_activation_receipt_journal_summary_handoff
            .runtime_activation_receipt_journal_summary_handoff_action,
        "render-runtime-activation-receipt-journal-summary-handoff"
    );
    assert!(
        accepted_notification_stack_summary_product_handoff_delivery_package_embed_runtime_activation_receipt_journal_summary_handoff
            .should_publish_summary_handoff
    );
    assert_eq!(
        accepted_notification_stack_summary_product_handoff_delivery_package_embed_runtime_activation_receipt_journal_summary_handoff
            .latest_runtime_activation_receipt_id,
        accepted_notification_stack_summary_product_handoff_delivery_package_embed_runtime_activation_receipt_journal_summary
            .latest_runtime_activation_receipt_id
    );
    assert_eq!(
        accepted_notification_stack_summary_product_handoff_delivery_package_embed_runtime_activation_receipt_journal_summary_handoff
            .activation_receipt_journal_summary_handoff_steps
            .last()
            .map(String::as_str),
        Some("publish-runtime-activation-receipt-journal-summary-handoff")
    );

    let accepted_notification_stack_summary_product_handoff_delivery_package_embed_runtime_activation_receipt_journal_summary_handoff_payload: serde_json::Value = serde_json::from_str(
        &app.run_app_shell_dashboard_dispatch_queue_lane_tab_panel_card_action_menu_group_shortcut_command_palette_search_invocation_receipt_notification_stack_summary_product_handoff_delivery_package_embed_runtime_activation_receipt_journal_summary_handoff_json(
            BerkeleyAppPersistedEditorState {
                selected_syntax_card_index: Some(3),
                active_command_id: Some("analysis.3.inspect-waveform".to_string()),
            },
            "queued dispatch",
            Some("dashboard.dispatch-queue-lane-tab-panel-card-action-menu-group-shortcut-command-palette-search-result.queued".to_string()),
        )
        .unwrap(),
    )
    .expect(
        "shell dashboard dispatch queue lane tab panel card action menu group shortcut command palette search invocation receipt notification stack summary product handoff delivery package embed runtime activation receipt journal summary handoff JSON should parse",
    );
    assert_eq!(
        accepted_notification_stack_summary_product_handoff_delivery_package_embed_runtime_activation_receipt_journal_summary_handoff_payload
            ["runtimeActivationReceiptJournalSummaryHandoffStatus"],
        "published"
    );
    assert_eq!(
        accepted_notification_stack_summary_product_handoff_delivery_package_embed_runtime_activation_receipt_journal_summary_handoff_payload
            ["runtimeActivationReceiptJournalSummaryHandoffDisposition"],
        "published"
    );
    assert_eq!(
        accepted_notification_stack_summary_product_handoff_delivery_package_embed_runtime_activation_receipt_journal_summary_handoff_payload
            ["embedRuntimeActivationReceiptJournalSummary"]["runtimeActivationReceiptJournalSummaryStatus"],
        "ready"
    );
    assert_eq!(
        accepted_notification_stack_summary_product_handoff_delivery_package_embed_runtime_activation_receipt_journal_summary_handoff_payload
            ["dispatchQueueLaneTabPanelCardActionMenuGroupShortcutCommandPaletteSearchInvocationReceiptNotificationStackSummaryProductHandoffDeliveryPackageEmbedRuntimeActivationReceiptJournalSummaryHandoffCapabilityId"],
        "app-shell-dashboard-dispatch-queue-lane-tab-panel-card-action-menu-group-shortcut-command-palette-search-invocation-receipt-notification-stack-summary-product-handoff-delivery-package-embed-runtime-activation-receipt-journal-summary-handoff-json"
    );

    let accepted_notification_stack_summary_product_handoff_delivery_package_embed_runtime_activation_receipt_journal_summary_handoff_receipt =
        app.run_app_shell_dashboard_dispatch_queue_lane_tab_panel_card_action_menu_group_shortcut_command_palette_search_invocation_receipt_notification_stack_summary_product_handoff_delivery_package_embed_runtime_activation_receipt_journal_summary_handoff_receipt(
            BerkeleyAppPersistedEditorState {
                selected_syntax_card_index: Some(3),
                active_command_id: Some("analysis.3.inspect-waveform".to_string()),
            },
            "queued dispatch",
            Some("dashboard.dispatch-queue-lane-tab-panel-card-action-menu-group-shortcut-command-palette-search-result.queued".to_string()),
        )
        .expect(
            "shell dashboard dispatch queue lane tab panel card action menu group shortcut command palette search invocation receipt notification stack summary product handoff delivery package embed runtime activation receipt journal summary handoff receipt should execute",
        );
    assert_eq!(
        accepted_notification_stack_summary_product_handoff_delivery_package_embed_runtime_activation_receipt_journal_summary_handoff_receipt
            .schema_version,
        BERKELEY_APP_SHELL_DASHBOARD_DISPATCH_QUEUE_LANE_TAB_PANEL_CARD_ACTION_MENU_GROUP_SHORTCUT_COMMAND_PALETTE_SEARCH_INVOCATION_RECEIPT_NOTIFICATION_STACK_SUMMARY_PRODUCT_HANDOFF_DELIVERY_PACKAGE_EMBED_RUNTIME_ACTIVATION_RECEIPT_JOURNAL_SUMMARY_HANDOFF_RECEIPT_SCHEMA_VERSION
    );
    assert_eq!(
        accepted_notification_stack_summary_product_handoff_delivery_package_embed_runtime_activation_receipt_journal_summary_handoff_receipt
            .runtime_activation_receipt_journal_summary_handoff_receipt_kind,
        "berkeley-product-handoff-webassembly-embed-runtime-activation-receipt-journal-summary-handoff-receipt"
    );
    assert_eq!(
        accepted_notification_stack_summary_product_handoff_delivery_package_embed_runtime_activation_receipt_journal_summary_handoff_receipt
            .runtime_activation_receipt_journal_summary_handoff_receipt_status,
        "acknowledged"
    );
    assert_eq!(
        accepted_notification_stack_summary_product_handoff_delivery_package_embed_runtime_activation_receipt_journal_summary_handoff_receipt
            .runtime_activation_receipt_journal_summary_handoff_receipt_action,
        "acknowledge-runtime-activation-receipt-journal-summary-handoff"
    );
    assert!(
        accepted_notification_stack_summary_product_handoff_delivery_package_embed_runtime_activation_receipt_journal_summary_handoff_receipt
            .should_acknowledge_summary_handoff_receipt
    );
    assert_eq!(
        accepted_notification_stack_summary_product_handoff_delivery_package_embed_runtime_activation_receipt_journal_summary_handoff_receipt
            .runtime_activation_receipt_journal_summary_handoff_id,
        accepted_notification_stack_summary_product_handoff_delivery_package_embed_runtime_activation_receipt_journal_summary_handoff
            .runtime_activation_receipt_journal_summary_handoff_id
    );
    assert_eq!(
        accepted_notification_stack_summary_product_handoff_delivery_package_embed_runtime_activation_receipt_journal_summary_handoff_receipt
            .activation_receipt_journal_summary_handoff_receipt_steps
            .last()
            .map(String::as_str),
        Some("acknowledge-runtime-activation-receipt-journal-summary-handoff")
    );

    let accepted_notification_stack_summary_product_handoff_delivery_package_embed_runtime_activation_receipt_journal_summary_handoff_receipt_payload: serde_json::Value = serde_json::from_str(
        &app.run_app_shell_dashboard_dispatch_queue_lane_tab_panel_card_action_menu_group_shortcut_command_palette_search_invocation_receipt_notification_stack_summary_product_handoff_delivery_package_embed_runtime_activation_receipt_journal_summary_handoff_receipt_json(
            BerkeleyAppPersistedEditorState {
                selected_syntax_card_index: Some(3),
                active_command_id: Some("analysis.3.inspect-waveform".to_string()),
            },
            "queued dispatch",
            Some("dashboard.dispatch-queue-lane-tab-panel-card-action-menu-group-shortcut-command-palette-search-result.queued".to_string()),
        )
        .unwrap(),
    )
    .expect(
        "shell dashboard dispatch queue lane tab panel card action menu group shortcut command palette search invocation receipt notification stack summary product handoff delivery package embed runtime activation receipt journal summary handoff receipt JSON should parse",
    );
    assert_eq!(
        accepted_notification_stack_summary_product_handoff_delivery_package_embed_runtime_activation_receipt_journal_summary_handoff_receipt_payload
            ["runtimeActivationReceiptJournalSummaryHandoffReceiptStatus"],
        "acknowledged"
    );
    assert_eq!(
        accepted_notification_stack_summary_product_handoff_delivery_package_embed_runtime_activation_receipt_journal_summary_handoff_receipt_payload
            ["runtimeActivationReceiptJournalSummaryHandoffReceiptDisposition"],
        "acknowledged"
    );
    assert_eq!(
        accepted_notification_stack_summary_product_handoff_delivery_package_embed_runtime_activation_receipt_journal_summary_handoff_receipt_payload
            ["embedRuntimeActivationReceiptJournalSummaryHandoff"]["runtimeActivationReceiptJournalSummaryHandoffStatus"],
        "published"
    );
    assert_eq!(
        accepted_notification_stack_summary_product_handoff_delivery_package_embed_runtime_activation_receipt_journal_summary_handoff_receipt_payload
            ["dispatchQueueLaneTabPanelCardActionMenuGroupShortcutCommandPaletteSearchInvocationReceiptNotificationStackSummaryProductHandoffDeliveryPackageEmbedRuntimeActivationReceiptJournalSummaryHandoffReceiptCapabilityId"],
        "app-shell-dashboard-dispatch-queue-lane-tab-panel-card-action-menu-group-shortcut-command-palette-search-invocation-receipt-notification-stack-summary-product-handoff-delivery-package-embed-runtime-activation-receipt-journal-summary-handoff-receipt-json"
    );

    let accepted_notification_stack_summary_product_handoff_delivery_package_embed_runtime_activation_receipt_journal_summary_handoff_receipt_acknowledgement =
        app.run_app_shell_dashboard_dispatch_queue_lane_tab_panel_card_action_menu_group_shortcut_command_palette_search_invocation_receipt_notification_stack_summary_product_handoff_delivery_package_embed_runtime_activation_receipt_journal_summary_handoff_receipt_acknowledgement(
            BerkeleyAppPersistedEditorState {
                selected_syntax_card_index: Some(3),
                active_command_id: Some("analysis.3.inspect-waveform".to_string()),
            },
            "queued dispatch",
            Some("dashboard.dispatch-queue-lane-tab-panel-card-action-menu-group-shortcut-command-palette-search-result.queued".to_string()),
        )
        .expect(
            "shell dashboard dispatch queue lane tab panel card action menu group shortcut command palette search invocation receipt notification stack summary product handoff delivery package embed runtime activation receipt journal summary handoff receipt acknowledgement should execute",
        );
    assert_eq!(
        accepted_notification_stack_summary_product_handoff_delivery_package_embed_runtime_activation_receipt_journal_summary_handoff_receipt_acknowledgement
            .schema_version,
        BERKELEY_APP_SHELL_DASHBOARD_DISPATCH_QUEUE_LANE_TAB_PANEL_CARD_ACTION_MENU_GROUP_SHORTCUT_COMMAND_PALETTE_SEARCH_INVOCATION_RECEIPT_NOTIFICATION_STACK_SUMMARY_PRODUCT_HANDOFF_DELIVERY_PACKAGE_EMBED_RUNTIME_ACTIVATION_RECEIPT_JOURNAL_SUMMARY_HANDOFF_RECEIPT_ACKNOWLEDGEMENT_SCHEMA_VERSION
    );
    assert_eq!(
        accepted_notification_stack_summary_product_handoff_delivery_package_embed_runtime_activation_receipt_journal_summary_handoff_receipt_acknowledgement
            .runtime_activation_receipt_journal_summary_handoff_receipt_acknowledgement_kind,
        "berkeley-product-handoff-webassembly-embed-runtime-activation-receipt-journal-summary-handoff-receipt-acknowledgement"
    );
    assert_eq!(
        accepted_notification_stack_summary_product_handoff_delivery_package_embed_runtime_activation_receipt_journal_summary_handoff_receipt_acknowledgement
            .runtime_activation_receipt_journal_summary_handoff_receipt_acknowledgement_status,
        "acknowledged"
    );
    assert_eq!(
        accepted_notification_stack_summary_product_handoff_delivery_package_embed_runtime_activation_receipt_journal_summary_handoff_receipt_acknowledgement
            .runtime_activation_receipt_journal_summary_handoff_receipt_acknowledgement_action,
        "acknowledge-runtime-activation-receipt-journal-summary-handoff-receipt"
    );
    assert!(
        accepted_notification_stack_summary_product_handoff_delivery_package_embed_runtime_activation_receipt_journal_summary_handoff_receipt_acknowledgement
            .should_acknowledge_summary_handoff_receipt_acknowledgement
    );
    assert_eq!(
        accepted_notification_stack_summary_product_handoff_delivery_package_embed_runtime_activation_receipt_journal_summary_handoff_receipt_acknowledgement
            .runtime_activation_receipt_journal_summary_handoff_receipt_id,
        accepted_notification_stack_summary_product_handoff_delivery_package_embed_runtime_activation_receipt_journal_summary_handoff_receipt
            .runtime_activation_receipt_journal_summary_handoff_receipt_id
    );
    assert_eq!(
        accepted_notification_stack_summary_product_handoff_delivery_package_embed_runtime_activation_receipt_journal_summary_handoff_receipt_acknowledgement
            .activation_receipt_journal_summary_handoff_receipt_acknowledgement_steps
            .last()
            .map(String::as_str),
        Some("acknowledge-runtime-activation-receipt-journal-summary-handoff-receipt")
    );

    let accepted_notification_stack_summary_product_handoff_delivery_package_embed_runtime_activation_receipt_journal_summary_handoff_receipt_acknowledgement_payload: serde_json::Value = serde_json::from_str(
        &app.run_app_shell_dashboard_dispatch_queue_lane_tab_panel_card_action_menu_group_shortcut_command_palette_search_invocation_receipt_notification_stack_summary_product_handoff_delivery_package_embed_runtime_activation_receipt_journal_summary_handoff_receipt_acknowledgement_json(
            BerkeleyAppPersistedEditorState {
                selected_syntax_card_index: Some(3),
                active_command_id: Some("analysis.3.inspect-waveform".to_string()),
            },
            "queued dispatch",
            Some("dashboard.dispatch-queue-lane-tab-panel-card-action-menu-group-shortcut-command-palette-search-result.queued".to_string()),
        )
        .unwrap(),
    )
    .expect(
        "shell dashboard dispatch queue lane tab panel card action menu group shortcut command palette search invocation receipt notification stack summary product handoff delivery package embed runtime activation receipt journal summary handoff receipt acknowledgement JSON should parse",
    );
    assert_eq!(
        accepted_notification_stack_summary_product_handoff_delivery_package_embed_runtime_activation_receipt_journal_summary_handoff_receipt_acknowledgement_payload
            ["runtimeActivationReceiptJournalSummaryHandoffReceiptAcknowledgementStatus"],
        "acknowledged"
    );
    assert_eq!(
        accepted_notification_stack_summary_product_handoff_delivery_package_embed_runtime_activation_receipt_journal_summary_handoff_receipt_acknowledgement_payload
            ["runtimeActivationReceiptJournalSummaryHandoffReceiptAcknowledgementDisposition"],
        "acknowledged"
    );
    assert_eq!(
        accepted_notification_stack_summary_product_handoff_delivery_package_embed_runtime_activation_receipt_journal_summary_handoff_receipt_acknowledgement_payload
            ["embedRuntimeActivationReceiptJournalSummaryHandoffReceipt"]["runtimeActivationReceiptJournalSummaryHandoffReceiptStatus"],
        "acknowledged"
    );
    assert_eq!(
        accepted_notification_stack_summary_product_handoff_delivery_package_embed_runtime_activation_receipt_journal_summary_handoff_receipt_acknowledgement_payload
            ["dispatchQueueLaneTabPanelCardActionMenuGroupShortcutCommandPaletteSearchInvocationReceiptNotificationStackSummaryProductHandoffDeliveryPackageEmbedRuntimeActivationReceiptJournalSummaryHandoffReceiptAcknowledgementCapabilityId"],
        "app-shell-dashboard-dispatch-queue-lane-tab-panel-card-action-menu-group-shortcut-command-palette-search-invocation-receipt-notification-stack-summary-product-handoff-delivery-package-embed-runtime-activation-receipt-journal-summary-handoff-receipt-acknowledgement-json"
    );

    let accepted_notification_stack_summary_product_handoff_delivery_package_embed_runtime_activation_receipt_journal_summary_handoff_receipt_acknowledgement_record =
        app.run_app_shell_dashboard_dispatch_queue_lane_tab_panel_card_action_menu_group_shortcut_command_palette_search_invocation_receipt_notification_stack_summary_product_handoff_delivery_package_embed_runtime_activation_receipt_journal_summary_handoff_receipt_acknowledgement_record(
            BerkeleyAppPersistedEditorState {
                selected_syntax_card_index: Some(3),
                active_command_id: Some("analysis.3.inspect-waveform".to_string()),
            },
            "queued dispatch",
            Some("dashboard.dispatch-queue-lane-tab-panel-card-action-menu-group-shortcut-command-palette-search-result.queued".to_string()),
        )
        .expect(
            "shell dashboard dispatch queue lane tab panel card action menu group shortcut command palette search invocation receipt notification stack summary product handoff delivery package embed runtime activation receipt journal summary handoff receipt acknowledgement record should execute",
        );
    assert_eq!(
        accepted_notification_stack_summary_product_handoff_delivery_package_embed_runtime_activation_receipt_journal_summary_handoff_receipt_acknowledgement_record
            .schema_version,
        BERKELEY_APP_SHELL_DASHBOARD_DISPATCH_QUEUE_LANE_TAB_PANEL_CARD_ACTION_MENU_GROUP_SHORTCUT_COMMAND_PALETTE_SEARCH_INVOCATION_RECEIPT_NOTIFICATION_STACK_SUMMARY_PRODUCT_HANDOFF_DELIVERY_PACKAGE_EMBED_RUNTIME_ACTIVATION_RECEIPT_JOURNAL_SUMMARY_HANDOFF_RECEIPT_ACKNOWLEDGEMENT_RECORD_SCHEMA_VERSION
    );
    assert_eq!(
        accepted_notification_stack_summary_product_handoff_delivery_package_embed_runtime_activation_receipt_journal_summary_handoff_receipt_acknowledgement_record
            .runtime_activation_receipt_journal_summary_handoff_receipt_acknowledgement_record_status,
        "recorded"
    );
    assert_eq!(
        accepted_notification_stack_summary_product_handoff_delivery_package_embed_runtime_activation_receipt_journal_summary_handoff_receipt_acknowledgement_record
            .runtime_activation_receipt_journal_summary_handoff_receipt_acknowledgement_record_action,
        "record-runtime-activation-receipt-journal-summary-handoff-receipt-acknowledgement"
    );
    assert!(
        accepted_notification_stack_summary_product_handoff_delivery_package_embed_runtime_activation_receipt_journal_summary_handoff_receipt_acknowledgement_record
            .should_record_summary_handoff_receipt_acknowledgement
    );
    assert_eq!(
        accepted_notification_stack_summary_product_handoff_delivery_package_embed_runtime_activation_receipt_journal_summary_handoff_receipt_acknowledgement_record
            .runtime_activation_receipt_journal_summary_handoff_receipt_acknowledgement_id,
        accepted_notification_stack_summary_product_handoff_delivery_package_embed_runtime_activation_receipt_journal_summary_handoff_receipt_acknowledgement
            .runtime_activation_receipt_journal_summary_handoff_receipt_acknowledgement_id
    );
    assert_eq!(
        accepted_notification_stack_summary_product_handoff_delivery_package_embed_runtime_activation_receipt_journal_summary_handoff_receipt_acknowledgement_record
            .activation_receipt_journal_summary_handoff_receipt_acknowledgement_record_steps
            .last()
            .map(String::as_str),
        Some("record-runtime-activation-receipt-journal-summary-handoff-receipt-acknowledgement")
    );

    let accepted_notification_stack_summary_product_handoff_delivery_package_embed_runtime_activation_receipt_journal_summary_handoff_receipt_acknowledgement_record_payload: serde_json::Value = serde_json::from_str(
        &app.run_app_shell_dashboard_dispatch_queue_lane_tab_panel_card_action_menu_group_shortcut_command_palette_search_invocation_receipt_notification_stack_summary_product_handoff_delivery_package_embed_runtime_activation_receipt_journal_summary_handoff_receipt_acknowledgement_record_json(
            BerkeleyAppPersistedEditorState {
                selected_syntax_card_index: Some(3),
                active_command_id: Some("analysis.3.inspect-waveform".to_string()),
            },
            "queued dispatch",
            Some("dashboard.dispatch-queue-lane-tab-panel-card-action-menu-group-shortcut-command-palette-search-result.queued".to_string()),
        )
        .unwrap(),
    )
    .expect(
        "shell dashboard dispatch queue lane tab panel card action menu group shortcut command palette search invocation receipt notification stack summary product handoff delivery package embed runtime activation receipt journal summary handoff receipt acknowledgement record JSON should parse",
    );
    assert_eq!(
        accepted_notification_stack_summary_product_handoff_delivery_package_embed_runtime_activation_receipt_journal_summary_handoff_receipt_acknowledgement_record_payload
            ["runtimeActivationReceiptJournalSummaryHandoffReceiptAcknowledgementRecordStatus"],
        "recorded"
    );
    assert_eq!(
        accepted_notification_stack_summary_product_handoff_delivery_package_embed_runtime_activation_receipt_journal_summary_handoff_receipt_acknowledgement_record_payload
            ["runtimeActivationReceiptJournalSummaryHandoffReceiptAcknowledgementRecordDisposition"],
        "recorded"
    );
    assert_eq!(
        accepted_notification_stack_summary_product_handoff_delivery_package_embed_runtime_activation_receipt_journal_summary_handoff_receipt_acknowledgement_record_payload
            ["embedRuntimeActivationReceiptJournalSummaryHandoffReceiptAcknowledgement"]["runtimeActivationReceiptJournalSummaryHandoffReceiptAcknowledgementStatus"],
        "acknowledged"
    );
    assert_eq!(
        accepted_notification_stack_summary_product_handoff_delivery_package_embed_runtime_activation_receipt_journal_summary_handoff_receipt_acknowledgement_record_payload
            ["dispatchQueueLaneTabPanelCardActionMenuGroupShortcutCommandPaletteSearchInvocationReceiptNotificationStackSummaryProductHandoffDeliveryPackageEmbedRuntimeActivationReceiptJournalSummaryHandoffReceiptAcknowledgementRecordCapabilityId"],
        "app-shell-dashboard-dispatch-queue-lane-tab-panel-card-action-menu-group-shortcut-command-palette-search-invocation-receipt-notification-stack-summary-product-handoff-delivery-package-embed-runtime-activation-receipt-journal-summary-handoff-receipt-acknowledgement-record-json"
    );

    let accepted_notification_stack_summary_product_handoff_delivery_package_embed_runtime_activation_receipt_journal_summary_handoff_receipt_acknowledgement_record_receipt =
        app.run_app_shell_dashboard_dispatch_queue_lane_tab_panel_card_action_menu_group_shortcut_command_palette_search_invocation_receipt_notification_stack_summary_product_handoff_delivery_package_embed_runtime_activation_receipt_journal_summary_handoff_receipt_acknowledgement_record_receipt(
            BerkeleyAppPersistedEditorState {
                selected_syntax_card_index: Some(3),
                active_command_id: Some("analysis.3.inspect-waveform".to_string()),
            },
            "queued dispatch",
            Some("dashboard.dispatch-queue-lane-tab-panel-card-action-menu-group-shortcut-command-palette-search-result.queued".to_string()),
        )
        .expect(
            "shell dashboard dispatch queue lane tab panel card action menu group shortcut command palette search invocation receipt notification stack summary product handoff delivery package embed runtime activation receipt journal summary handoff receipt acknowledgement record receipt should execute",
        );
    assert_eq!(
        accepted_notification_stack_summary_product_handoff_delivery_package_embed_runtime_activation_receipt_journal_summary_handoff_receipt_acknowledgement_record_receipt
            .schema_version,
        BERKELEY_APP_SHELL_DASHBOARD_DISPATCH_QUEUE_LANE_TAB_PANEL_CARD_ACTION_MENU_GROUP_SHORTCUT_COMMAND_PALETTE_SEARCH_INVOCATION_RECEIPT_NOTIFICATION_STACK_SUMMARY_PRODUCT_HANDOFF_DELIVERY_PACKAGE_EMBED_RUNTIME_ACTIVATION_RECEIPT_JOURNAL_SUMMARY_HANDOFF_RECEIPT_ACKNOWLEDGEMENT_RECORD_RECEIPT_SCHEMA_VERSION
    );
    assert_eq!(
        accepted_notification_stack_summary_product_handoff_delivery_package_embed_runtime_activation_receipt_journal_summary_handoff_receipt_acknowledgement_record_receipt
            .runtime_activation_receipt_journal_summary_handoff_receipt_acknowledgement_record_receipt_status,
        "acknowledged"
    );
    assert_eq!(
        accepted_notification_stack_summary_product_handoff_delivery_package_embed_runtime_activation_receipt_journal_summary_handoff_receipt_acknowledgement_record_receipt
            .runtime_activation_receipt_journal_summary_handoff_receipt_acknowledgement_record_receipt_action,
        "acknowledge-runtime-activation-receipt-journal-summary-handoff-receipt-acknowledgement-record"
    );
    assert!(
        accepted_notification_stack_summary_product_handoff_delivery_package_embed_runtime_activation_receipt_journal_summary_handoff_receipt_acknowledgement_record_receipt
            .should_acknowledge_summary_handoff_receipt_acknowledgement_record_receipt
    );
    assert_eq!(
        accepted_notification_stack_summary_product_handoff_delivery_package_embed_runtime_activation_receipt_journal_summary_handoff_receipt_acknowledgement_record_receipt
            .runtime_activation_receipt_journal_summary_handoff_receipt_acknowledgement_record_id,
        accepted_notification_stack_summary_product_handoff_delivery_package_embed_runtime_activation_receipt_journal_summary_handoff_receipt_acknowledgement_record
            .runtime_activation_receipt_journal_summary_handoff_receipt_acknowledgement_record_id
    );
    assert_eq!(
        accepted_notification_stack_summary_product_handoff_delivery_package_embed_runtime_activation_receipt_journal_summary_handoff_receipt_acknowledgement_record_receipt
            .activation_receipt_journal_summary_handoff_receipt_acknowledgement_record_receipt_steps
            .last()
            .map(String::as_str),
        Some("acknowledge-runtime-activation-receipt-journal-summary-handoff-receipt-acknowledgement-record")
    );

    let accepted_notification_stack_summary_product_handoff_delivery_package_embed_runtime_activation_receipt_journal_summary_handoff_receipt_acknowledgement_record_receipt_payload: serde_json::Value = serde_json::from_str(
        &app.run_app_shell_dashboard_dispatch_queue_lane_tab_panel_card_action_menu_group_shortcut_command_palette_search_invocation_receipt_notification_stack_summary_product_handoff_delivery_package_embed_runtime_activation_receipt_journal_summary_handoff_receipt_acknowledgement_record_receipt_json(
            BerkeleyAppPersistedEditorState {
                selected_syntax_card_index: Some(3),
                active_command_id: Some("analysis.3.inspect-waveform".to_string()),
            },
            "queued dispatch",
            Some("dashboard.dispatch-queue-lane-tab-panel-card-action-menu-group-shortcut-command-palette-search-result.queued".to_string()),
        )
        .unwrap(),
    )
    .expect(
        "shell dashboard dispatch queue lane tab panel card action menu group shortcut command palette search invocation receipt notification stack summary product handoff delivery package embed runtime activation receipt journal summary handoff receipt acknowledgement record receipt JSON should parse",
    );
    assert_eq!(
        accepted_notification_stack_summary_product_handoff_delivery_package_embed_runtime_activation_receipt_journal_summary_handoff_receipt_acknowledgement_record_receipt_payload
            ["runtimeActivationReceiptJournalSummaryHandoffReceiptAcknowledgementRecordReceiptStatus"],
        "acknowledged"
    );
    assert_eq!(
        accepted_notification_stack_summary_product_handoff_delivery_package_embed_runtime_activation_receipt_journal_summary_handoff_receipt_acknowledgement_record_receipt_payload
            ["runtimeActivationReceiptJournalSummaryHandoffReceiptAcknowledgementRecordReceiptDisposition"],
        "acknowledged"
    );
    assert_eq!(
        accepted_notification_stack_summary_product_handoff_delivery_package_embed_runtime_activation_receipt_journal_summary_handoff_receipt_acknowledgement_record_receipt_payload
            ["embedRuntimeActivationReceiptJournalSummaryHandoffReceiptAcknowledgementRecord"]["runtimeActivationReceiptJournalSummaryHandoffReceiptAcknowledgementRecordStatus"],
        "recorded"
    );
    assert_eq!(
        accepted_notification_stack_summary_product_handoff_delivery_package_embed_runtime_activation_receipt_journal_summary_handoff_receipt_acknowledgement_record_receipt_payload
            ["dispatchQueueLaneTabPanelCardActionMenuGroupShortcutCommandPaletteSearchInvocationReceiptNotificationStackSummaryProductHandoffDeliveryPackageEmbedRuntimeActivationReceiptJournalSummaryHandoffReceiptAcknowledgementRecordReceiptCapabilityId"],
        "app-shell-dashboard-dispatch-queue-lane-tab-panel-card-action-menu-group-shortcut-command-palette-search-invocation-receipt-notification-stack-summary-product-handoff-delivery-package-embed-runtime-activation-receipt-journal-summary-handoff-receipt-acknowledgement-record-receipt-json"
    );

    let accepted_notification_stack_summary_product_handoff_delivery_package_embed_runtime_activation_receipt_journal_summary_handoff_receipt_acknowledgement_record_receipt_acknowledgement =
        app.run_app_shell_dashboard_dispatch_queue_lane_tab_panel_card_action_menu_group_shortcut_command_palette_search_invocation_receipt_notification_stack_summary_product_handoff_delivery_package_embed_runtime_activation_receipt_journal_summary_handoff_receipt_acknowledgement_record_receipt_acknowledgement(
            BerkeleyAppPersistedEditorState {
                selected_syntax_card_index: Some(3),
                active_command_id: Some("analysis.3.inspect-waveform".to_string()),
            },
            "queued dispatch",
            Some("dashboard.dispatch-queue-lane-tab-panel-card-action-menu-group-shortcut-command-palette-search-result.queued".to_string()),
        )
        .expect(
            "shell dashboard dispatch queue lane tab panel card action menu group shortcut command palette search invocation receipt notification stack summary product handoff delivery package embed runtime activation receipt journal summary handoff receipt acknowledgement record receipt acknowledgement should execute",
        );
    assert_eq!(
        accepted_notification_stack_summary_product_handoff_delivery_package_embed_runtime_activation_receipt_journal_summary_handoff_receipt_acknowledgement_record_receipt_acknowledgement
            .schema_version,
        BERKELEY_APP_SHELL_DASHBOARD_DISPATCH_QUEUE_LANE_TAB_PANEL_CARD_ACTION_MENU_GROUP_SHORTCUT_COMMAND_PALETTE_SEARCH_INVOCATION_RECEIPT_NOTIFICATION_STACK_SUMMARY_PRODUCT_HANDOFF_DELIVERY_PACKAGE_EMBED_RUNTIME_ACTIVATION_RECEIPT_JOURNAL_SUMMARY_HANDOFF_RECEIPT_ACKNOWLEDGEMENT_RECORD_RECEIPT_ACKNOWLEDGEMENT_SCHEMA_VERSION
    );
    assert_eq!(
        accepted_notification_stack_summary_product_handoff_delivery_package_embed_runtime_activation_receipt_journal_summary_handoff_receipt_acknowledgement_record_receipt_acknowledgement
            .runtime_activation_receipt_journal_summary_handoff_receipt_acknowledgement_record_receipt_acknowledgement_status,
        "acknowledged"
    );
    assert_eq!(
        accepted_notification_stack_summary_product_handoff_delivery_package_embed_runtime_activation_receipt_journal_summary_handoff_receipt_acknowledgement_record_receipt_acknowledgement
            .runtime_activation_receipt_journal_summary_handoff_receipt_acknowledgement_record_receipt_acknowledgement_action,
        "acknowledge-runtime-activation-receipt-journal-summary-handoff-receipt-acknowledgement-record-receipt"
    );
    assert!(
        accepted_notification_stack_summary_product_handoff_delivery_package_embed_runtime_activation_receipt_journal_summary_handoff_receipt_acknowledgement_record_receipt_acknowledgement
            .should_acknowledge_summary_handoff_receipt_acknowledgement_record_receipt_acknowledgement
    );
    assert_eq!(
        accepted_notification_stack_summary_product_handoff_delivery_package_embed_runtime_activation_receipt_journal_summary_handoff_receipt_acknowledgement_record_receipt_acknowledgement
            .runtime_activation_receipt_journal_summary_handoff_receipt_acknowledgement_record_receipt_id,
        accepted_notification_stack_summary_product_handoff_delivery_package_embed_runtime_activation_receipt_journal_summary_handoff_receipt_acknowledgement_record_receipt
            .runtime_activation_receipt_journal_summary_handoff_receipt_acknowledgement_record_receipt_id
    );
    assert_eq!(
        accepted_notification_stack_summary_product_handoff_delivery_package_embed_runtime_activation_receipt_journal_summary_handoff_receipt_acknowledgement_record_receipt_acknowledgement
            .activation_receipt_journal_summary_handoff_receipt_acknowledgement_record_receipt_acknowledgement_steps
            .last()
            .map(String::as_str),
        Some("acknowledge-runtime-activation-receipt-journal-summary-handoff-receipt-acknowledgement-record-receipt")
    );

    let accepted_notification_stack_summary_product_handoff_delivery_package_embed_runtime_activation_receipt_journal_summary_handoff_receipt_acknowledgement_record_receipt_acknowledgement_payload: serde_json::Value = serde_json::from_str(
        &app.run_app_shell_dashboard_dispatch_queue_lane_tab_panel_card_action_menu_group_shortcut_command_palette_search_invocation_receipt_notification_stack_summary_product_handoff_delivery_package_embed_runtime_activation_receipt_journal_summary_handoff_receipt_acknowledgement_record_receipt_acknowledgement_json(
            BerkeleyAppPersistedEditorState {
                selected_syntax_card_index: Some(3),
                active_command_id: Some("analysis.3.inspect-waveform".to_string()),
            },
            "queued dispatch",
            Some("dashboard.dispatch-queue-lane-tab-panel-card-action-menu-group-shortcut-command-palette-search-result.queued".to_string()),
        )
        .unwrap(),
    )
    .expect(
        "shell dashboard dispatch queue lane tab panel card action menu group shortcut command palette search invocation receipt notification stack summary product handoff delivery package embed runtime activation receipt journal summary handoff receipt acknowledgement record receipt acknowledgement JSON should parse",
    );
    assert_eq!(
        accepted_notification_stack_summary_product_handoff_delivery_package_embed_runtime_activation_receipt_journal_summary_handoff_receipt_acknowledgement_record_receipt_acknowledgement_payload
            ["runtimeActivationReceiptJournalSummaryHandoffReceiptAcknowledgementRecordReceiptAcknowledgementStatus"],
        "acknowledged"
    );
    assert_eq!(
        accepted_notification_stack_summary_product_handoff_delivery_package_embed_runtime_activation_receipt_journal_summary_handoff_receipt_acknowledgement_record_receipt_acknowledgement_payload
            ["runtimeActivationReceiptJournalSummaryHandoffReceiptAcknowledgementRecordReceiptAcknowledgementDisposition"],
        "acknowledged"
    );
    assert_eq!(
        accepted_notification_stack_summary_product_handoff_delivery_package_embed_runtime_activation_receipt_journal_summary_handoff_receipt_acknowledgement_record_receipt_acknowledgement_payload
            ["embedRuntimeActivationReceiptJournalSummaryHandoffReceiptAcknowledgementRecordReceipt"]["runtimeActivationReceiptJournalSummaryHandoffReceiptAcknowledgementRecordReceiptStatus"],
        "acknowledged"
    );
    assert_eq!(
        accepted_notification_stack_summary_product_handoff_delivery_package_embed_runtime_activation_receipt_journal_summary_handoff_receipt_acknowledgement_record_receipt_acknowledgement_payload
            ["dispatchQueueLaneTabPanelCardActionMenuGroupShortcutCommandPaletteSearchInvocationReceiptNotificationStackSummaryProductHandoffDeliveryPackageEmbedRuntimeActivationReceiptJournalSummaryHandoffReceiptAcknowledgementRecordReceiptAcknowledgementCapabilityId"],
        "app-shell-dashboard-dispatch-queue-lane-tab-panel-card-action-menu-group-shortcut-command-palette-search-invocation-receipt-notification-stack-summary-product-handoff-delivery-package-embed-runtime-activation-receipt-journal-summary-handoff-receipt-acknowledgement-record-receipt-acknowledgement-json"
    );

    let accepted_notification_stack_summary_product_handoff_delivery_package_embed_runtime_activation_receipt_journal_summary_handoff_receipt_acknowledgement_record_receipt_acknowledgement_record =
        app.run_app_shell_dashboard_dispatch_queue_lane_tab_panel_card_action_menu_group_shortcut_command_palette_search_invocation_receipt_notification_stack_summary_product_handoff_delivery_package_embed_runtime_activation_receipt_journal_summary_handoff_receipt_acknowledgement_record_receipt_acknowledgement_record(
            BerkeleyAppPersistedEditorState {
                selected_syntax_card_index: Some(3),
                active_command_id: Some("analysis.3.inspect-waveform".to_string()),
            },
            "queued dispatch",
            Some("dashboard.dispatch-queue-lane-tab-panel-card-action-menu-group-shortcut-command-palette-search-result.queued".to_string()),
        )
        .expect(
            "shell dashboard dispatch queue lane tab panel card action menu group shortcut command palette search invocation receipt notification stack summary product handoff delivery package embed runtime activation receipt journal summary handoff receipt acknowledgement record receipt acknowledgement record should execute",
        );
    assert_eq!(
        accepted_notification_stack_summary_product_handoff_delivery_package_embed_runtime_activation_receipt_journal_summary_handoff_receipt_acknowledgement_record_receipt_acknowledgement_record
            .schema_version,
        BERKELEY_APP_SHELL_DASHBOARD_DISPATCH_QUEUE_LANE_TAB_PANEL_CARD_ACTION_MENU_GROUP_SHORTCUT_COMMAND_PALETTE_SEARCH_INVOCATION_RECEIPT_NOTIFICATION_STACK_SUMMARY_PRODUCT_HANDOFF_DELIVERY_PACKAGE_EMBED_RUNTIME_ACTIVATION_RECEIPT_JOURNAL_SUMMARY_HANDOFF_RECEIPT_ACKNOWLEDGEMENT_RECORD_RECEIPT_ACKNOWLEDGEMENT_RECORD_SCHEMA_VERSION
    );
    assert_eq!(
        accepted_notification_stack_summary_product_handoff_delivery_package_embed_runtime_activation_receipt_journal_summary_handoff_receipt_acknowledgement_record_receipt_acknowledgement_record
            .runtime_activation_receipt_journal_summary_handoff_receipt_acknowledgement_record_receipt_acknowledgement_record_status,
        "recorded"
    );
    assert_eq!(
        accepted_notification_stack_summary_product_handoff_delivery_package_embed_runtime_activation_receipt_journal_summary_handoff_receipt_acknowledgement_record_receipt_acknowledgement_record
            .runtime_activation_receipt_journal_summary_handoff_receipt_acknowledgement_record_receipt_acknowledgement_record_action,
        "record-runtime-activation-receipt-journal-summary-handoff-receipt-acknowledgement-record-receipt-acknowledgement"
    );
    assert!(
        accepted_notification_stack_summary_product_handoff_delivery_package_embed_runtime_activation_receipt_journal_summary_handoff_receipt_acknowledgement_record_receipt_acknowledgement_record
            .should_record_summary_handoff_receipt_acknowledgement_record_receipt_acknowledgement
    );
    assert_eq!(
        accepted_notification_stack_summary_product_handoff_delivery_package_embed_runtime_activation_receipt_journal_summary_handoff_receipt_acknowledgement_record_receipt_acknowledgement_record
            .runtime_activation_receipt_journal_summary_handoff_receipt_acknowledgement_record_receipt_acknowledgement_id,
        accepted_notification_stack_summary_product_handoff_delivery_package_embed_runtime_activation_receipt_journal_summary_handoff_receipt_acknowledgement_record_receipt_acknowledgement
            .runtime_activation_receipt_journal_summary_handoff_receipt_acknowledgement_record_receipt_acknowledgement_id
    );
    assert_eq!(
        accepted_notification_stack_summary_product_handoff_delivery_package_embed_runtime_activation_receipt_journal_summary_handoff_receipt_acknowledgement_record_receipt_acknowledgement_record
            .activation_receipt_journal_summary_handoff_receipt_acknowledgement_record_receipt_acknowledgement_record_steps
            .last()
            .map(String::as_str),
        Some("record-runtime-activation-receipt-journal-summary-handoff-receipt-acknowledgement-record-receipt-acknowledgement")
    );

    let accepted_notification_stack_summary_product_handoff_delivery_package_embed_runtime_activation_receipt_journal_summary_handoff_receipt_acknowledgement_record_receipt_acknowledgement_record_payload: serde_json::Value = serde_json::from_str(
        &app.run_app_shell_dashboard_dispatch_queue_lane_tab_panel_card_action_menu_group_shortcut_command_palette_search_invocation_receipt_notification_stack_summary_product_handoff_delivery_package_embed_runtime_activation_receipt_journal_summary_handoff_receipt_acknowledgement_record_receipt_acknowledgement_record_json(
            BerkeleyAppPersistedEditorState {
                selected_syntax_card_index: Some(3),
                active_command_id: Some("analysis.3.inspect-waveform".to_string()),
            },
            "queued dispatch",
            Some("dashboard.dispatch-queue-lane-tab-panel-card-action-menu-group-shortcut-command-palette-search-result.queued".to_string()),
        )
        .unwrap(),
    )
    .expect(
        "shell dashboard dispatch queue lane tab panel card action menu group shortcut command palette search invocation receipt notification stack summary product handoff delivery package embed runtime activation receipt journal summary handoff receipt acknowledgement record receipt acknowledgement record JSON should parse",
    );
    assert_eq!(
        accepted_notification_stack_summary_product_handoff_delivery_package_embed_runtime_activation_receipt_journal_summary_handoff_receipt_acknowledgement_record_receipt_acknowledgement_record_payload
            ["runtimeActivationReceiptJournalSummaryHandoffReceiptAcknowledgementRecordReceiptAcknowledgementRecordStatus"],
        "recorded"
    );
    assert_eq!(
        accepted_notification_stack_summary_product_handoff_delivery_package_embed_runtime_activation_receipt_journal_summary_handoff_receipt_acknowledgement_record_receipt_acknowledgement_record_payload
            ["runtimeActivationReceiptJournalSummaryHandoffReceiptAcknowledgementRecordReceiptAcknowledgementRecordDisposition"],
        "recorded"
    );
    assert_eq!(
        accepted_notification_stack_summary_product_handoff_delivery_package_embed_runtime_activation_receipt_journal_summary_handoff_receipt_acknowledgement_record_receipt_acknowledgement_record_payload
            ["embedRuntimeActivationReceiptJournalSummaryHandoffReceiptAcknowledgementRecordReceiptAcknowledgement"]["runtimeActivationReceiptJournalSummaryHandoffReceiptAcknowledgementRecordReceiptAcknowledgementStatus"],
        "acknowledged"
    );
    assert_eq!(
        accepted_notification_stack_summary_product_handoff_delivery_package_embed_runtime_activation_receipt_journal_summary_handoff_receipt_acknowledgement_record_receipt_acknowledgement_record_payload
            ["dispatchQueueLaneTabPanelCardActionMenuGroupShortcutCommandPaletteSearchInvocationReceiptNotificationStackSummaryProductHandoffDeliveryPackageEmbedRuntimeActivationReceiptJournalSummaryHandoffReceiptAcknowledgementRecordReceiptAcknowledgementRecordCapabilityId"],
        "app-shell-dashboard-dispatch-queue-lane-tab-panel-card-action-menu-group-shortcut-command-palette-search-invocation-receipt-notification-stack-summary-product-handoff-delivery-package-embed-runtime-activation-receipt-journal-summary-handoff-receipt-acknowledgement-record-receipt-acknowledgement-record-json"
    );

    let accepted_notification_stack_summary_product_handoff_delivery_package_embed_runtime_activation_receipt_journal_summary_handoff_receipt_acknowledgement_record_receipt_acknowledgement_record_summary =
        app.run_app_shell_dashboard_dispatch_queue_lane_tab_panel_card_action_menu_group_shortcut_command_palette_search_invocation_receipt_notification_stack_summary_product_handoff_delivery_package_embed_runtime_activation_receipt_journal_summary_handoff_receipt_acknowledgement_record_receipt_acknowledgement_record_summary(
            BerkeleyAppPersistedEditorState {
                selected_syntax_card_index: Some(3),
                active_command_id: Some("analysis.3.inspect-waveform".to_string()),
            },
            "queued dispatch",
            Some("dashboard.dispatch-queue-lane-tab-panel-card-action-menu-group-shortcut-command-palette-search-result.queued".to_string()),
        )
        .expect(
            "shell dashboard dispatch queue lane tab panel card action menu group shortcut command palette search invocation receipt notification stack summary product handoff delivery package embed runtime activation receipt journal summary handoff receipt acknowledgement record receipt acknowledgement record summary should execute",
        );
    assert_eq!(
        accepted_notification_stack_summary_product_handoff_delivery_package_embed_runtime_activation_receipt_journal_summary_handoff_receipt_acknowledgement_record_receipt_acknowledgement_record_summary
            .schema_version,
        BERKELEY_APP_SHELL_DASHBOARD_DISPATCH_QUEUE_LANE_TAB_PANEL_CARD_ACTION_MENU_GROUP_SHORTCUT_COMMAND_PALETTE_SEARCH_INVOCATION_RECEIPT_NOTIFICATION_STACK_SUMMARY_PRODUCT_HANDOFF_DELIVERY_PACKAGE_EMBED_RUNTIME_ACTIVATION_RECEIPT_JOURNAL_SUMMARY_HANDOFF_RECEIPT_ACKNOWLEDGEMENT_RECORD_RECEIPT_ACKNOWLEDGEMENT_RECORD_SUMMARY_SCHEMA_VERSION
    );
    assert_eq!(
        accepted_notification_stack_summary_product_handoff_delivery_package_embed_runtime_activation_receipt_journal_summary_handoff_receipt_acknowledgement_record_receipt_acknowledgement_record_summary
            .runtime_activation_receipt_journal_summary_handoff_receipt_acknowledgement_record_receipt_acknowledgement_record_summary_status,
        "summarized"
    );
    assert_eq!(
        accepted_notification_stack_summary_product_handoff_delivery_package_embed_runtime_activation_receipt_journal_summary_handoff_receipt_acknowledgement_record_receipt_acknowledgement_record_summary
            .runtime_activation_receipt_journal_summary_handoff_receipt_acknowledgement_record_receipt_acknowledgement_record_summary_action,
        "summarize-runtime-activation-receipt-journal-summary-handoff-receipt-acknowledgement-record-receipt-acknowledgement-record"
    );
    assert!(
        accepted_notification_stack_summary_product_handoff_delivery_package_embed_runtime_activation_receipt_journal_summary_handoff_receipt_acknowledgement_record_receipt_acknowledgement_record_summary
            .should_summarize_summary_handoff_receipt_acknowledgement_record_receipt_acknowledgement_record
    );
    assert_eq!(
        accepted_notification_stack_summary_product_handoff_delivery_package_embed_runtime_activation_receipt_journal_summary_handoff_receipt_acknowledgement_record_receipt_acknowledgement_record_summary
            .runtime_activation_receipt_journal_summary_handoff_receipt_acknowledgement_record_receipt_acknowledgement_record_id,
        accepted_notification_stack_summary_product_handoff_delivery_package_embed_runtime_activation_receipt_journal_summary_handoff_receipt_acknowledgement_record_receipt_acknowledgement_record
            .runtime_activation_receipt_journal_summary_handoff_receipt_acknowledgement_record_receipt_acknowledgement_record_id
    );
    assert_eq!(
        accepted_notification_stack_summary_product_handoff_delivery_package_embed_runtime_activation_receipt_journal_summary_handoff_receipt_acknowledgement_record_receipt_acknowledgement_record_summary
            .summarized_acknowledgement_record_count,
        1
    );
    assert_eq!(
        accepted_notification_stack_summary_product_handoff_delivery_package_embed_runtime_activation_receipt_journal_summary_handoff_receipt_acknowledgement_record_receipt_acknowledgement_record_summary
            .deferred_acknowledgement_record_count,
        0
    );
    assert_eq!(
        accepted_notification_stack_summary_product_handoff_delivery_package_embed_runtime_activation_receipt_journal_summary_handoff_receipt_acknowledgement_record_receipt_acknowledgement_record_summary
            .summary_step_count,
        accepted_notification_stack_summary_product_handoff_delivery_package_embed_runtime_activation_receipt_journal_summary_handoff_receipt_acknowledgement_record_receipt_acknowledgement_record_summary
            .activation_receipt_journal_summary_handoff_receipt_acknowledgement_record_receipt_acknowledgement_record_summary_steps
            .len()
    );
    assert_eq!(
        accepted_notification_stack_summary_product_handoff_delivery_package_embed_runtime_activation_receipt_journal_summary_handoff_receipt_acknowledgement_record_receipt_acknowledgement_record_summary
            .activation_receipt_journal_summary_handoff_receipt_acknowledgement_record_receipt_acknowledgement_record_summary_steps
            .last()
            .map(String::as_str),
        Some("summarize-runtime-activation-receipt-journal-summary-handoff-receipt-acknowledgement-record-receipt-acknowledgement-record")
    );

    let accepted_notification_stack_summary_product_handoff_delivery_package_embed_runtime_activation_receipt_journal_summary_handoff_receipt_acknowledgement_record_receipt_acknowledgement_record_summary_payload: serde_json::Value = serde_json::from_str(
        &app.run_app_shell_dashboard_dispatch_queue_lane_tab_panel_card_action_menu_group_shortcut_command_palette_search_invocation_receipt_notification_stack_summary_product_handoff_delivery_package_embed_runtime_activation_receipt_journal_summary_handoff_receipt_acknowledgement_record_receipt_acknowledgement_record_summary_json(
            BerkeleyAppPersistedEditorState {
                selected_syntax_card_index: Some(3),
                active_command_id: Some("analysis.3.inspect-waveform".to_string()),
            },
            "queued dispatch",
            Some("dashboard.dispatch-queue-lane-tab-panel-card-action-menu-group-shortcut-command-palette-search-result.queued".to_string()),
        )
        .unwrap(),
    )
    .expect(
        "shell dashboard dispatch queue lane tab panel card action menu group shortcut command palette search invocation receipt notification stack summary product handoff delivery package embed runtime activation receipt journal summary handoff receipt acknowledgement record receipt acknowledgement record summary JSON should parse",
    );
    assert_eq!(
        accepted_notification_stack_summary_product_handoff_delivery_package_embed_runtime_activation_receipt_journal_summary_handoff_receipt_acknowledgement_record_receipt_acknowledgement_record_summary_payload
            ["runtimeActivationReceiptJournalSummaryHandoffReceiptAcknowledgementRecordReceiptAcknowledgementRecordSummaryStatus"],
        "summarized"
    );
    assert_eq!(
        accepted_notification_stack_summary_product_handoff_delivery_package_embed_runtime_activation_receipt_journal_summary_handoff_receipt_acknowledgement_record_receipt_acknowledgement_record_summary_payload
            ["runtimeActivationReceiptJournalSummaryHandoffReceiptAcknowledgementRecordReceiptAcknowledgementRecordSummaryDisposition"],
        "summarized"
    );
    assert_eq!(
        accepted_notification_stack_summary_product_handoff_delivery_package_embed_runtime_activation_receipt_journal_summary_handoff_receipt_acknowledgement_record_receipt_acknowledgement_record_summary_payload
            ["embedRuntimeActivationReceiptJournalSummaryHandoffReceiptAcknowledgementRecordReceiptAcknowledgementRecord"]["runtimeActivationReceiptJournalSummaryHandoffReceiptAcknowledgementRecordReceiptAcknowledgementRecordStatus"],
        "recorded"
    );
    assert_eq!(
        accepted_notification_stack_summary_product_handoff_delivery_package_embed_runtime_activation_receipt_journal_summary_handoff_receipt_acknowledgement_record_receipt_acknowledgement_record_summary_payload
            ["summarizedAcknowledgementRecordCount"],
        1
    );
    assert_eq!(
        accepted_notification_stack_summary_product_handoff_delivery_package_embed_runtime_activation_receipt_journal_summary_handoff_receipt_acknowledgement_record_receipt_acknowledgement_record_summary_payload
            ["dispatchQueueLaneTabPanelCardActionMenuGroupShortcutCommandPaletteSearchInvocationReceiptNotificationStackSummaryProductHandoffDeliveryPackageEmbedRuntimeActivationReceiptJournalSummaryHandoffReceiptAcknowledgementRecordReceiptAcknowledgementRecordSummaryCapabilityId"],
        "app-shell-dashboard-dispatch-queue-lane-tab-panel-card-action-menu-group-shortcut-command-palette-search-invocation-receipt-notification-stack-summary-product-handoff-delivery-package-embed-runtime-activation-receipt-journal-summary-handoff-receipt-acknowledgement-record-receipt-acknowledgement-record-summary-json"
    );

    let accepted_notification_stack_summary_product_handoff_delivery_package_embed_runtime_activation_receipt_journal_summary_handoff_receipt_acknowledgement_record_receipt_acknowledgement_record_summary_digest =
        app.run_app_shell_dashboard_dispatch_queue_lane_tab_panel_card_action_menu_group_shortcut_command_palette_search_invocation_receipt_notification_stack_summary_product_handoff_delivery_package_embed_runtime_activation_receipt_journal_summary_handoff_receipt_acknowledgement_record_receipt_acknowledgement_record_summary_digest(
            BerkeleyAppPersistedEditorState {
                selected_syntax_card_index: Some(3),
                active_command_id: Some("analysis.3.inspect-waveform".to_string()),
            },
            "queued dispatch",
            Some("dashboard.dispatch-queue-lane-tab-panel-card-action-menu-group-shortcut-command-palette-search-result.queued".to_string()),
        )
        .expect(
            "shell dashboard dispatch queue lane tab panel card action menu group shortcut command palette search invocation receipt notification stack summary product handoff delivery package embed runtime activation receipt journal summary handoff receipt acknowledgement record receipt acknowledgement record summary digest should execute",
        );
    assert_eq!(
        accepted_notification_stack_summary_product_handoff_delivery_package_embed_runtime_activation_receipt_journal_summary_handoff_receipt_acknowledgement_record_receipt_acknowledgement_record_summary_digest
            .schema_version,
        BERKELEY_APP_SHELL_DASHBOARD_DISPATCH_QUEUE_LANE_TAB_PANEL_CARD_ACTION_MENU_GROUP_SHORTCUT_COMMAND_PALETTE_SEARCH_INVOCATION_RECEIPT_NOTIFICATION_STACK_SUMMARY_PRODUCT_HANDOFF_DELIVERY_PACKAGE_EMBED_RUNTIME_ACTIVATION_RECEIPT_JOURNAL_SUMMARY_HANDOFF_RECEIPT_ACKNOWLEDGEMENT_RECORD_RECEIPT_ACKNOWLEDGEMENT_RECORD_SUMMARY_DIGEST_SCHEMA_VERSION
    );
    assert_eq!(
        accepted_notification_stack_summary_product_handoff_delivery_package_embed_runtime_activation_receipt_journal_summary_handoff_receipt_acknowledgement_record_receipt_acknowledgement_record_summary_digest
            .runtime_activation_receipt_journal_summary_handoff_receipt_acknowledgement_record_receipt_acknowledgement_record_summary_digest_status,
        "routed"
    );
    assert_eq!(
        accepted_notification_stack_summary_product_handoff_delivery_package_embed_runtime_activation_receipt_journal_summary_handoff_receipt_acknowledgement_record_receipt_acknowledgement_record_summary_digest
            .runtime_activation_receipt_journal_summary_handoff_receipt_acknowledgement_record_receipt_acknowledgement_record_summary_digest_action,
        "route-runtime-activation-receipt-journal-summary-handoff-receipt-acknowledgement-record-receipt-acknowledgement-record-summary-digest"
    );
    assert_eq!(
        accepted_notification_stack_summary_product_handoff_delivery_package_embed_runtime_activation_receipt_journal_summary_handoff_receipt_acknowledgement_record_receipt_acknowledgement_record_summary_digest
            .summary_badge_label,
        "Acknowledged"
    );
    assert_eq!(
        accepted_notification_stack_summary_product_handoff_delivery_package_embed_runtime_activation_receipt_journal_summary_handoff_receipt_acknowledgement_record_receipt_acknowledgement_record_summary_digest
            .summary_badge_tone,
        "success"
    );
    assert!(
        accepted_notification_stack_summary_product_handoff_delivery_package_embed_runtime_activation_receipt_journal_summary_handoff_receipt_acknowledgement_record_receipt_acknowledgement_record_summary_digest
            .should_route_summary_digest
    );
    assert!(
        !accepted_notification_stack_summary_product_handoff_delivery_package_embed_runtime_activation_receipt_journal_summary_handoff_receipt_acknowledgement_record_receipt_acknowledgement_record_summary_digest
            .should_hold_summary_digest
    );
    assert_eq!(
        accepted_notification_stack_summary_product_handoff_delivery_package_embed_runtime_activation_receipt_journal_summary_handoff_receipt_acknowledgement_record_receipt_acknowledgement_record_summary_digest
            .summary_route_target_id,
        accepted_notification_stack_summary_product_handoff_delivery_package_embed_runtime_activation_receipt_journal_summary_handoff_receipt_acknowledgement_record_receipt_acknowledgement_record_summary
            .runtime_activation_receipt_journal_summary_handoff_receipt_acknowledgement_record_receipt_acknowledgement_record_summary_id
    );
    assert_eq!(
        accepted_notification_stack_summary_product_handoff_delivery_package_embed_runtime_activation_receipt_journal_summary_handoff_receipt_acknowledgement_record_receipt_acknowledgement_record_summary_digest
            .artifact_capability_count,
        accepted_notification_stack_summary_product_handoff_delivery_package_embed_runtime_activation_receipt_journal_summary_handoff_receipt_acknowledgement_record_receipt_acknowledgement_record_summary
            .artifact_capability_count
            + 1
    );
    assert_eq!(
        accepted_notification_stack_summary_product_handoff_delivery_package_embed_runtime_activation_receipt_journal_summary_handoff_receipt_acknowledgement_record_receipt_acknowledgement_record_summary_digest
            .dispatch_queue_lane_tab_panel_card_action_menu_group_shortcut_command_palette_search_invocation_receipt_notification_stack_summary_product_handoff_delivery_package_embed_runtime_activation_receipt_journal_summary_handoff_receipt_acknowledgement_record_receipt_acknowledgement_record_summary_capability_id,
        accepted_notification_stack_summary_product_handoff_delivery_package_embed_runtime_activation_receipt_journal_summary_handoff_receipt_acknowledgement_record_receipt_acknowledgement_record_summary
            .dispatch_queue_lane_tab_panel_card_action_menu_group_shortcut_command_palette_search_invocation_receipt_notification_stack_summary_product_handoff_delivery_package_embed_runtime_activation_receipt_journal_summary_handoff_receipt_acknowledgement_record_receipt_acknowledgement_record_summary_capability_id
    );

    let accepted_notification_stack_summary_product_handoff_delivery_package_embed_runtime_activation_receipt_journal_summary_handoff_receipt_acknowledgement_record_receipt_acknowledgement_record_summary_digest_payload: serde_json::Value = serde_json::from_str(
        &app.run_app_shell_dashboard_dispatch_queue_lane_tab_panel_card_action_menu_group_shortcut_command_palette_search_invocation_receipt_notification_stack_summary_product_handoff_delivery_package_embed_runtime_activation_receipt_journal_summary_handoff_receipt_acknowledgement_record_receipt_acknowledgement_record_summary_digest_json(
            BerkeleyAppPersistedEditorState {
                selected_syntax_card_index: Some(3),
                active_command_id: Some("analysis.3.inspect-waveform".to_string()),
            },
            "queued dispatch",
            Some("dashboard.dispatch-queue-lane-tab-panel-card-action-menu-group-shortcut-command-palette-search-result.queued".to_string()),
        )
        .unwrap(),
    )
    .expect(
        "shell dashboard dispatch queue lane tab panel card action menu group shortcut command palette search invocation receipt notification stack summary product handoff delivery package embed runtime activation receipt journal summary handoff receipt acknowledgement record receipt acknowledgement record summary digest JSON should parse",
    );
    assert_eq!(
        accepted_notification_stack_summary_product_handoff_delivery_package_embed_runtime_activation_receipt_journal_summary_handoff_receipt_acknowledgement_record_receipt_acknowledgement_record_summary_digest_payload
            ["runtimeActivationReceiptJournalSummaryHandoffReceiptAcknowledgementRecordReceiptAcknowledgementRecordSummaryDigestStatus"],
        "routed"
    );
    assert_eq!(
        accepted_notification_stack_summary_product_handoff_delivery_package_embed_runtime_activation_receipt_journal_summary_handoff_receipt_acknowledgement_record_receipt_acknowledgement_record_summary_digest_payload
            ["summaryBadgeTone"],
        "success"
    );
    assert_eq!(
        accepted_notification_stack_summary_product_handoff_delivery_package_embed_runtime_activation_receipt_journal_summary_handoff_receipt_acknowledgement_record_receipt_acknowledgement_record_summary_digest_payload
            ["summaryRouteTargetId"],
        accepted_notification_stack_summary_product_handoff_delivery_package_embed_runtime_activation_receipt_journal_summary_handoff_receipt_acknowledgement_record_receipt_acknowledgement_record_summary
            .runtime_activation_receipt_journal_summary_handoff_receipt_acknowledgement_record_receipt_acknowledgement_record_summary_id
    );
    assert_eq!(
        accepted_notification_stack_summary_product_handoff_delivery_package_embed_runtime_activation_receipt_journal_summary_handoff_receipt_acknowledgement_record_receipt_acknowledgement_record_summary_digest_payload
            ["dispatchQueueLaneTabPanelCardActionMenuGroupShortcutCommandPaletteSearchInvocationReceiptNotificationStackSummaryProductHandoffDeliveryPackageEmbedRuntimeActivationReceiptJournalSummaryHandoffReceiptAcknowledgementRecordReceiptAcknowledgementRecordSummaryDigestCapabilityId"],
        "app-shell-dashboard-dispatch-queue-lane-tab-panel-card-action-menu-group-shortcut-command-palette-search-invocation-receipt-notification-stack-summary-product-handoff-delivery-package-embed-runtime-activation-receipt-journal-summary-handoff-receipt-acknowledgement-record-receipt-acknowledgement-record-summary-digest-json"
    );
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

    let summary = app.app_startup_summary(BerkeleyAppPersistedEditorState {
        selected_syntax_card_index: Some(2),
        active_command_id: Some("analysis.2.run".to_string()),
    });
    assert_eq!(
        summary.schema_version,
        BERKELEY_APP_STARTUP_SUMMARY_SCHEMA_VERSION
    );
    assert!(!summary.ready);
    assert!(!summary.parsed);
    assert!(!summary.execution_available);
    assert_eq!(summary.active_panel_id.as_deref(), Some("diagnostics"));
    assert_eq!(summary.diagnostic_count, 1);
    assert!(summary.blocking_message.is_some());

    let summary_payload: serde_json::Value = serde_json::from_str(&app.app_startup_summary_json(
        BerkeleyAppPersistedEditorState {
            selected_syntax_card_index: Some(2),
            active_command_id: Some("analysis.2.run".to_string()),
        },
    ))
    .expect("blocked startup summary JSON should parse");
    assert_eq!(summary_payload["ready"], false);
    assert_eq!(summary_payload["activePanelId"], "diagnostics");
    assert_eq!(summary_payload["diagnosticCount"], 1);
    assert!(summary_payload["blockingMessage"].is_string());

    let launch_plan = app.app_launch_plan(BerkeleyAppPersistedEditorState {
        selected_syntax_card_index: Some(2),
        active_command_id: Some("analysis.2.run".to_string()),
    });
    assert_eq!(
        launch_plan.schema_version,
        BERKELEY_APP_LAUNCH_PLAN_SCHEMA_VERSION
    );
    assert_eq!(launch_plan.startup_route, "blocked");
    assert!(!launch_plan.ready);
    assert_eq!(launch_plan.entry_panel_id.as_deref(), Some("diagnostics"));
    assert_eq!(launch_plan.entry_panel_kind.as_deref(), Some("diagnostics"));
    assert_eq!(launch_plan.entry_target.as_deref(), Some("diagnostics"));
    assert_eq!(launch_plan.diagnostic_count, 1);
    assert!(launch_plan.blocking_message.is_some());
    let primary_action = launch_plan
        .actions
        .iter()
        .find(|action| action.primary)
        .expect("blocked launch plan should expose diagnostics as primary");
    assert_eq!(primary_action.id, "launch.diagnostics");
    assert_eq!(primary_action.panel_id, "diagnostics");
    assert!(primary_action.enabled);

    let launch_payload: serde_json::Value =
        serde_json::from_str(&app.app_launch_plan_json(BerkeleyAppPersistedEditorState {
            selected_syntax_card_index: Some(2),
            active_command_id: Some("analysis.2.run".to_string()),
        }))
        .expect("blocked launch plan JSON should parse");
    assert_eq!(launch_payload["startupRoute"], "blocked");
    assert_eq!(launch_payload["entryPanelId"], "diagnostics");
    assert_eq!(launch_payload["entryTarget"], "diagnostics");
    assert_eq!(launch_payload["actions"][1]["id"], "launch.diagnostics");
    assert_eq!(launch_payload["actions"][1]["primary"], true);
    assert_eq!(launch_payload["diagnosticCount"], 1);

    let readiness_report = app.app_readiness_report(BerkeleyAppPersistedEditorState {
        selected_syntax_card_index: Some(2),
        active_command_id: Some("analysis.2.run".to_string()),
    });
    assert_eq!(
        readiness_report.schema_version,
        BERKELEY_APP_READINESS_REPORT_SCHEMA_VERSION
    );
    assert_eq!(readiness_report.startup_route, "blocked");
    assert!(!readiness_report.ready);
    assert!(!readiness_report.parsed);
    assert!(!readiness_report.execution_available);
    assert_eq!(
        readiness_report.entry_panel_id.as_deref(),
        Some("diagnostics")
    );
    assert_eq!(
        readiness_report.primary_action_id.as_deref(),
        Some("launch.diagnostics")
    );
    assert!(readiness_report.primary_action_enabled);
    assert_eq!(readiness_report.panel_count, 5);
    assert_eq!(readiness_report.enabled_panel_count, 3);
    assert_eq!(readiness_report.disabled_panel_count, 2);
    assert_eq!(readiness_report.action_count, 5);
    assert_eq!(readiness_report.enabled_action_count, 3);
    assert_eq!(readiness_report.disabled_action_count, 2);
    assert_eq!(readiness_report.diagnostic_count, 1);
    assert_eq!(readiness_report.error_count, 1);
    assert_eq!(readiness_report.warning_count, 0);
    assert_eq!(readiness_report.note_count, 0);
    assert!(readiness_report.blocking_message.is_some());

    let readiness_payload: serde_json::Value = serde_json::from_str(
        &app.app_readiness_report_json(BerkeleyAppPersistedEditorState {
            selected_syntax_card_index: Some(2),
            active_command_id: Some("analysis.2.run".to_string()),
        }),
    )
    .expect("blocked readiness report JSON should parse");
    assert_eq!(readiness_payload["startupRoute"], "blocked");
    assert_eq!(readiness_payload["entryPanelId"], "diagnostics");
    assert_eq!(readiness_payload["primaryActionId"], "launch.diagnostics");
    assert_eq!(readiness_payload["enabledPanelCount"], 3);
    assert_eq!(readiness_payload["disabledPanelCount"], 2);
    assert_eq!(readiness_payload["enabledActionCount"], 3);
    assert_eq!(readiness_payload["errorCount"], 1);
    assert!(readiness_payload["blockingMessage"].is_string());

    let handoff = app.app_shell_handoff(BerkeleyAppPersistedEditorState {
        selected_syntax_card_index: Some(2),
        active_command_id: Some("analysis.2.run".to_string()),
    });
    assert_eq!(
        handoff.schema_version,
        BERKELEY_APP_SHELL_HANDOFF_SCHEMA_VERSION
    );
    assert!(!handoff.startup_summary.ready);
    assert_eq!(
        handoff.launch_plan.entry_panel_id.as_deref(),
        Some("diagnostics")
    );
    assert_eq!(handoff.readiness_report.error_count, 1);
    assert!(handoff.readiness_report.blocking_message.is_some());

    let handoff_payload: serde_json::Value = serde_json::from_str(&app.app_shell_handoff_json(
        BerkeleyAppPersistedEditorState {
            selected_syntax_card_index: Some(2),
            active_command_id: Some("analysis.2.run".to_string()),
        },
    ))
    .expect("blocked shell handoff JSON should parse");
    assert_eq!(handoff_payload["startupSummary"]["ready"], false);
    assert_eq!(handoff_payload["launchPlan"]["entryPanelId"], "diagnostics");
    assert_eq!(handoff_payload["readinessReport"]["errorCount"], 1);
    assert!(handoff_payload["readinessReport"]["blockingMessage"].is_string());

    let shell_status = app.app_shell_status(BerkeleyAppPersistedEditorState {
        selected_syntax_card_index: Some(2),
        active_command_id: Some("analysis.2.run".to_string()),
    });
    assert_eq!(
        shell_status.schema_version,
        BERKELEY_APP_SHELL_STATUS_SCHEMA_VERSION
    );
    assert!(!shell_status.ready);
    assert_eq!(shell_status.startup_route, "blocked");
    assert_eq!(shell_status.severity, "error");
    assert_eq!(shell_status.entry_panel_id.as_deref(), Some("diagnostics"));
    assert_eq!(
        shell_status.primary_action_id.as_deref(),
        Some("launch.diagnostics")
    );
    assert_eq!(shell_status.error_count, 1);
    assert_eq!(
        shell_status.blocking_message.as_deref(),
        Some(shell_status.message.as_str())
    );

    let shell_status_payload: serde_json::Value =
        serde_json::from_str(&app.app_shell_status_json(BerkeleyAppPersistedEditorState {
            selected_syntax_card_index: Some(2),
            active_command_id: Some("analysis.2.run".to_string()),
        }))
        .expect("blocked shell status JSON should parse");
    assert_eq!(shell_status_payload["ready"], false);
    assert_eq!(shell_status_payload["startupRoute"], "blocked");
    assert_eq!(shell_status_payload["severity"], "error");
    assert_eq!(shell_status_payload["entryPanelId"], "diagnostics");
    assert_eq!(
        shell_status_payload["primaryActionId"],
        "launch.diagnostics"
    );
    assert_eq!(shell_status_payload["errorCount"], 1);
    assert!(shell_status_payload["message"].is_string());

    let shell_telemetry = app.app_shell_telemetry(BerkeleyAppPersistedEditorState {
        selected_syntax_card_index: Some(2),
        active_command_id: Some("analysis.2.run".to_string()),
    });
    assert_eq!(
        shell_telemetry.schema_version,
        BERKELEY_APP_SHELL_TELEMETRY_SCHEMA_VERSION
    );
    assert!(!shell_telemetry.ready);
    assert_eq!(shell_telemetry.startup_route, "blocked");
    assert_eq!(shell_telemetry.severity, "error");
    assert_eq!(
        shell_telemetry.primary_action_id.as_deref(),
        Some("launch.diagnostics")
    );
    assert_eq!(shell_telemetry.panel_count, 5);
    assert_eq!(shell_telemetry.enabled_panel_count, 3);
    assert_eq!(shell_telemetry.disabled_panel_count, 2);
    assert_eq!(shell_telemetry.action_count, 5);
    assert_eq!(shell_telemetry.enabled_action_count, 3);
    assert_eq!(shell_telemetry.disabled_action_count, 2);
    assert_eq!(shell_telemetry.diagnostic_count, 1);
    assert_eq!(shell_telemetry.error_count, 1);
    assert_eq!(
        shell_telemetry.repaired_state,
        handoff.readiness_report.repaired_state
    );
    assert_eq!(
        shell_telemetry.artifact_capability_count,
        handoff.package_manifest.artifact_capabilities.len()
    );

    let shell_telemetry_payload: serde_json::Value = serde_json::from_str(
        &app.app_shell_telemetry_json(BerkeleyAppPersistedEditorState {
            selected_syntax_card_index: Some(2),
            active_command_id: Some("analysis.2.run".to_string()),
        }),
    )
    .expect("blocked shell telemetry JSON should parse");
    assert_eq!(shell_telemetry_payload["ready"], false);
    assert_eq!(shell_telemetry_payload["startupRoute"], "blocked");
    assert_eq!(shell_telemetry_payload["severity"], "error");
    assert_eq!(shell_telemetry_payload["panelCount"], 5);
    assert_eq!(shell_telemetry_payload["enabledPanelCount"], 3);
    assert_eq!(shell_telemetry_payload["disabledActionCount"], 2);
    assert_eq!(shell_telemetry_payload["diagnosticCount"], 1);
    assert_eq!(shell_telemetry_payload["errorCount"], 1);

    let shell_events = app.app_shell_event_log(BerkeleyAppPersistedEditorState {
        selected_syntax_card_index: Some(2),
        active_command_id: Some("analysis.2.run".to_string()),
    });
    assert_eq!(
        shell_events.schema_version,
        BERKELEY_APP_SHELL_EVENT_LOG_SCHEMA_VERSION
    );
    assert!(!shell_events.ready);
    assert_eq!(shell_events.startup_route, "blocked");
    assert_eq!(shell_events.event_count, 6);
    assert_eq!(shell_events.events[0].id, "shell.status");
    assert_eq!(shell_events.events[0].severity, "error");
    assert_eq!(
        shell_events.events[0].action_id.as_deref(),
        Some("launch.diagnostics")
    );
    assert_eq!(shell_events.events[1].id, "shell.route.blocked");
    assert_eq!(shell_events.events[2].id, "shell.action.primary");
    assert_eq!(
        shell_events.events[2].action_id.as_deref(),
        Some("launch.diagnostics")
    );
    assert_eq!(shell_events.events[3].id, "shell.diagnostics");
    assert_eq!(shell_events.events[3].severity, "error");
    assert_eq!(shell_events.events[3].count, Some(1));
    assert_eq!(shell_events.events[4].id, "shell.state");
    assert_eq!(
        shell_events.events[4].count,
        Some(if handoff.readiness_report.repaired_state {
            1
        } else {
            0
        })
    );

    let shell_events_payload: serde_json::Value = serde_json::from_str(
        &app.app_shell_event_log_json(BerkeleyAppPersistedEditorState {
            selected_syntax_card_index: Some(2),
            active_command_id: Some("analysis.2.run".to_string()),
        }),
    )
    .expect("blocked shell event log JSON should parse");
    assert_eq!(shell_events_payload["ready"], false);
    assert_eq!(shell_events_payload["startupRoute"], "blocked");
    assert_eq!(shell_events_payload["eventCount"], 6);
    assert_eq!(shell_events_payload["events"][0]["severity"], "error");
    assert_eq!(
        shell_events_payload["events"][1]["id"],
        "shell.route.blocked"
    );
    assert_eq!(
        shell_events_payload["events"][2]["actionId"],
        "launch.diagnostics"
    );
    assert_eq!(shell_events_payload["events"][3]["count"], 1);

    let shell_event_summary = app.app_shell_event_summary(BerkeleyAppPersistedEditorState {
        selected_syntax_card_index: Some(2),
        active_command_id: Some("analysis.2.run".to_string()),
    });
    assert_eq!(
        shell_event_summary.schema_version,
        BERKELEY_APP_SHELL_EVENT_SUMMARY_SCHEMA_VERSION
    );
    assert!(!shell_event_summary.ready);
    assert_eq!(shell_event_summary.startup_route, "blocked");
    assert_eq!(shell_event_summary.severity, "error");
    assert_eq!(
        shell_event_summary.primary_action_id.as_deref(),
        Some("launch.diagnostics")
    );
    assert_eq!(shell_event_summary.event_count, 6);
    assert_eq!(
        shell_event_summary.ready_event_count,
        if handoff.readiness_report.primary_action_enabled {
            1
        } else {
            0
        }
    );
    assert_eq!(
        shell_event_summary.blocked_event_count,
        if handoff.readiness_report.primary_action_enabled {
            0
        } else {
            1
        }
    );
    assert_eq!(
        shell_event_summary.info_event_count,
        if handoff.readiness_report.repaired_state {
            1
        } else {
            2
        }
    );
    assert_eq!(
        shell_event_summary.warning_event_count,
        if handoff.readiness_report.repaired_state {
            1
        } else {
            0
        }
    );
    assert_eq!(shell_event_summary.error_event_count, 3);
    assert_eq!(shell_event_summary.diagnostic_count, 1);
    assert_eq!(
        shell_event_summary.repaired_state_count,
        if handoff.readiness_report.repaired_state {
            1
        } else {
            0
        }
    );
    assert_eq!(
        shell_event_summary.artifact_capability_count,
        handoff.package_manifest.artifact_capabilities.len()
    );

    let shell_event_summary_payload: serde_json::Value = serde_json::from_str(
        &app.app_shell_event_summary_json(BerkeleyAppPersistedEditorState {
            selected_syntax_card_index: Some(2),
            active_command_id: Some("analysis.2.run".to_string()),
        }),
    )
    .expect("blocked shell event summary JSON should parse");
    assert_eq!(shell_event_summary_payload["ready"], false);
    assert_eq!(shell_event_summary_payload["startupRoute"], "blocked");
    assert_eq!(shell_event_summary_payload["severity"], "error");
    assert_eq!(
        shell_event_summary_payload["blockedEventCount"].as_u64(),
        Some(if handoff.readiness_report.primary_action_enabled {
            0
        } else {
            1
        })
    );
    assert_eq!(shell_event_summary_payload["diagnosticCount"], 1);
    assert_eq!(
        shell_event_summary_payload["primaryActionId"],
        "launch.diagnostics"
    );

    let shell_event_digest = app.app_shell_event_digest(BerkeleyAppPersistedEditorState {
        selected_syntax_card_index: Some(2),
        active_command_id: Some("analysis.2.run".to_string()),
    });
    assert_eq!(
        shell_event_digest.schema_version,
        BERKELEY_APP_SHELL_EVENT_DIGEST_SCHEMA_VERSION
    );
    assert!(!shell_event_digest.ready);
    assert_eq!(shell_event_digest.startup_route, "blocked");
    assert_eq!(shell_event_digest.severity, "error");
    assert_eq!(
        shell_event_digest.headline_event_id.as_deref(),
        Some("shell.status")
    );
    assert_eq!(
        shell_event_digest.primary_action_id.as_deref(),
        Some("launch.diagnostics")
    );
    let mut expected_attention_event_ids = vec![
        "shell.status".to_string(),
        "shell.route.blocked".to_string(),
        "shell.diagnostics".to_string(),
    ];
    if handoff.readiness_report.repaired_state {
        expected_attention_event_ids.push("shell.state".to_string());
    }
    assert_eq!(
        shell_event_digest.attention_event_count,
        expected_attention_event_ids.len()
    );
    assert_eq!(
        shell_event_digest.attention_event_ids,
        expected_attention_event_ids
    );
    assert_eq!(shell_event_digest.metric_event_count, 3);
    assert_eq!(shell_event_digest.diagnostic_count, 1);
    assert_eq!(
        shell_event_digest.artifact_capability_count,
        handoff.package_manifest.artifact_capabilities.len()
    );

    let shell_event_digest_payload: serde_json::Value = serde_json::from_str(
        &app.app_shell_event_digest_json(BerkeleyAppPersistedEditorState {
            selected_syntax_card_index: Some(2),
            active_command_id: Some("analysis.2.run".to_string()),
        }),
    )
    .expect("blocked shell event digest JSON should parse");
    assert_eq!(shell_event_digest_payload["ready"], false);
    assert_eq!(shell_event_digest_payload["startupRoute"], "blocked");
    assert_eq!(shell_event_digest_payload["severity"], "error");
    assert_eq!(
        shell_event_digest_payload["attentionEventCount"].as_u64(),
        Some(shell_event_digest.attention_event_count as u64)
    );
    assert_eq!(
        shell_event_digest_payload["attentionEventIds"][1],
        "shell.route.blocked"
    );
    assert_eq!(
        shell_event_digest_payload["primaryActionId"],
        "launch.diagnostics"
    );

    let shell_event_dashboard = app.app_shell_event_dashboard(BerkeleyAppPersistedEditorState {
        selected_syntax_card_index: Some(2),
        active_command_id: Some("analysis.2.run".to_string()),
    });
    assert_eq!(
        shell_event_dashboard.schema_version,
        BERKELEY_APP_SHELL_EVENT_DASHBOARD_SCHEMA_VERSION
    );
    assert!(!shell_event_dashboard.ready);
    assert_eq!(shell_event_dashboard.startup_route, "blocked");
    assert_eq!(shell_event_dashboard.severity, "error");
    assert_eq!(
        shell_event_dashboard.headline_event_id.as_deref(),
        Some("shell.status")
    );
    assert_eq!(
        shell_event_dashboard.primary_action_id.as_deref(),
        Some("launch.diagnostics")
    );
    assert!(shell_event_dashboard.attention_required);
    assert_eq!(shell_event_dashboard.section_count, 3);
    assert_eq!(shell_event_dashboard.sections[0].id, "status");
    assert_eq!(shell_event_dashboard.sections[0].severity, "error");
    assert_eq!(shell_event_dashboard.sections[1].id, "attention");
    assert_eq!(shell_event_dashboard.sections[1].severity, "error");
    assert_eq!(
        shell_event_dashboard.sections[1].event_count,
        shell_event_digest.attention_event_count
    );
    assert_eq!(
        shell_event_dashboard.sections[1].event_ids,
        shell_event_digest.attention_event_ids
    );
    assert_eq!(shell_event_dashboard.sections[2].id, "metrics");
    assert_eq!(shell_event_dashboard.sections[2].event_count, 3);

    let shell_event_dashboard_payload: serde_json::Value = serde_json::from_str(
        &app.app_shell_event_dashboard_json(BerkeleyAppPersistedEditorState {
            selected_syntax_card_index: Some(2),
            active_command_id: Some("analysis.2.run".to_string()),
        }),
    )
    .expect("blocked shell event dashboard JSON should parse");
    assert_eq!(shell_event_dashboard_payload["ready"], false);
    assert_eq!(shell_event_dashboard_payload["startupRoute"], "blocked");
    assert_eq!(shell_event_dashboard_payload["severity"], "error");
    assert_eq!(shell_event_dashboard_payload["attentionRequired"], true);
    assert_eq!(
        shell_event_dashboard_payload["sections"][1]["eventCount"].as_u64(),
        Some(shell_event_digest.attention_event_count as u64)
    );
    assert_eq!(
        shell_event_dashboard_payload["sections"][1]["eventIds"][0],
        "shell.status"
    );

    let shell_dashboard_package =
        app.app_shell_dashboard_package(BerkeleyAppPersistedEditorState {
            selected_syntax_card_index: Some(2),
            active_command_id: Some("analysis.2.run".to_string()),
        });
    assert_eq!(
        shell_dashboard_package.schema_version,
        BERKELEY_APP_SHELL_DASHBOARD_PACKAGE_SCHEMA_VERSION
    );
    assert!(!shell_dashboard_package.ready);
    assert_eq!(shell_dashboard_package.startup_route, "blocked");
    assert_eq!(shell_dashboard_package.severity, "error");
    assert!(shell_dashboard_package.attention_required);
    assert_eq!(shell_dashboard_package.section_count, 3);
    assert_eq!(
        shell_dashboard_package
            .event_dashboard
            .primary_action_id
            .as_deref(),
        Some("launch.diagnostics")
    );
    assert_eq!(
        shell_dashboard_package.artifact_capability_count,
        shell_dashboard_package
            .package_manifest
            .artifact_capabilities
            .len()
    );

    let shell_dashboard_package_payload: serde_json::Value = serde_json::from_str(
        &app.app_shell_dashboard_package_json(BerkeleyAppPersistedEditorState {
            selected_syntax_card_index: Some(2),
            active_command_id: Some("analysis.2.run".to_string()),
        }),
    )
    .expect("blocked shell dashboard package JSON should parse");
    assert_eq!(shell_dashboard_package_payload["ready"], false);
    assert_eq!(shell_dashboard_package_payload["startupRoute"], "blocked");
    assert_eq!(
        shell_dashboard_package_payload["packageCapabilityId"],
        "app-shell-dashboard-package-json"
    );
    assert_eq!(
        shell_dashboard_package_payload["eventDashboard"]["attentionRequired"],
        true
    );

    let shell_dashboard_cards = app.app_shell_dashboard_cards(BerkeleyAppPersistedEditorState {
        selected_syntax_card_index: Some(2),
        active_command_id: Some("analysis.2.run".to_string()),
    });
    assert_eq!(
        shell_dashboard_cards.schema_version,
        BERKELEY_APP_SHELL_DASHBOARD_CARDS_SCHEMA_VERSION
    );
    assert!(!shell_dashboard_cards.ready);
    assert_eq!(shell_dashboard_cards.startup_route, "blocked");
    assert_eq!(shell_dashboard_cards.severity, "error");
    assert!(shell_dashboard_cards.attention_required);
    assert_eq!(shell_dashboard_cards.card_count, 3);
    assert_eq!(
        shell_dashboard_cards.primary_card_id.as_deref(),
        Some("dashboard.attention")
    );
    assert_eq!(shell_dashboard_cards.cards[0].id, "dashboard.status");
    assert!(!shell_dashboard_cards.cards[0].primary);
    assert_eq!(shell_dashboard_cards.cards[1].id, "dashboard.attention");
    assert!(shell_dashboard_cards.cards[1].primary);
    assert!(shell_dashboard_cards.cards[1].attention);
    assert_eq!(
        shell_dashboard_cards.cards[1].event_count,
        shell_event_digest.attention_event_count
    );
    assert_eq!(
        shell_dashboard_cards.cards[1].event_ids,
        shell_event_digest.attention_event_ids
    );
    assert_eq!(
        shell_dashboard_cards.artifact_capability_count,
        shell_dashboard_package.artifact_capability_count
    );

    let shell_dashboard_cards_payload: serde_json::Value = serde_json::from_str(
        &app.app_shell_dashboard_cards_json(BerkeleyAppPersistedEditorState {
            selected_syntax_card_index: Some(2),
            active_command_id: Some("analysis.2.run".to_string()),
        }),
    )
    .expect("blocked shell dashboard cards JSON should parse");
    assert_eq!(shell_dashboard_cards_payload["ready"], false);
    assert_eq!(
        shell_dashboard_cards_payload["primaryCardId"],
        "dashboard.attention"
    );
    assert_eq!(shell_dashboard_cards_payload["cards"][1]["attention"], true);
    assert_eq!(
        shell_dashboard_cards_payload["cards"][1]["eventIds"][0],
        "shell.status"
    );

    let shell_dashboard_view = app.app_shell_dashboard_view(BerkeleyAppPersistedEditorState {
        selected_syntax_card_index: Some(2),
        active_command_id: Some("analysis.2.run".to_string()),
    });
    assert_eq!(
        shell_dashboard_view.schema_version,
        BERKELEY_APP_SHELL_DASHBOARD_VIEW_SCHEMA_VERSION
    );
    assert!(!shell_dashboard_view.ready);
    assert_eq!(shell_dashboard_view.startup_route, "blocked");
    assert_eq!(
        shell_dashboard_view.primary_card_id.as_deref(),
        Some("dashboard.attention")
    );
    assert_eq!(
        shell_dashboard_view.primary_card_title.as_deref(),
        Some("Attention")
    );
    assert_eq!(shell_dashboard_view.card_count, 3);
    assert_eq!(shell_dashboard_view.visible_card_count, 3);
    assert_eq!(shell_dashboard_view.attention_card_count, 1);
    assert_eq!(shell_dashboard_view.metric_card_count, 1);
    assert_eq!(
        shell_dashboard_view.card_ids,
        vec![
            "dashboard.status".to_string(),
            "dashboard.attention".to_string(),
            "dashboard.metrics".to_string()
        ]
    );
    assert_eq!(
        shell_dashboard_view.attention_card_ids,
        vec!["dashboard.attention".to_string()]
    );
    assert_eq!(
        shell_dashboard_view.metric_card_ids,
        vec!["dashboard.metrics".to_string()]
    );
    assert_eq!(
        shell_dashboard_view.cards_capability_id,
        shell_dashboard_cards.cards_capability_id
    );
    assert_eq!(
        shell_dashboard_view.view_capability_id,
        "app-shell-dashboard-view-json"
    );

    let shell_dashboard_view_payload: serde_json::Value = serde_json::from_str(
        &app.app_shell_dashboard_view_json(BerkeleyAppPersistedEditorState {
            selected_syntax_card_index: Some(2),
            active_command_id: Some("analysis.2.run".to_string()),
        }),
    )
    .expect("blocked shell dashboard view JSON should parse");
    assert_eq!(shell_dashboard_view_payload["ready"], false);
    assert_eq!(
        shell_dashboard_view_payload["primaryCardId"],
        "dashboard.attention"
    );
    assert_eq!(shell_dashboard_view_payload["attentionCardCount"], 1);
    assert_eq!(
        shell_dashboard_view_payload["attentionCardIds"][0],
        "dashboard.attention"
    );
    assert_eq!(
        shell_dashboard_view_payload["viewCapabilityId"],
        "app-shell-dashboard-view-json"
    );

    let shell_dashboard_layout = app.app_shell_dashboard_layout(BerkeleyAppPersistedEditorState {
        selected_syntax_card_index: Some(2),
        active_command_id: Some("analysis.2.run".to_string()),
    });
    assert_eq!(
        shell_dashboard_layout.schema_version,
        BERKELEY_APP_SHELL_DASHBOARD_LAYOUT_SCHEMA_VERSION
    );
    assert!(!shell_dashboard_layout.ready);
    assert_eq!(
        shell_dashboard_layout.primary_card_id.as_deref(),
        Some("dashboard.attention")
    );
    assert_eq!(
        shell_dashboard_layout.primary_region_id.as_deref(),
        Some("dashboard.layout.attention")
    );
    assert_eq!(shell_dashboard_layout.region_count, 3);
    assert_eq!(shell_dashboard_layout.visible_region_count, 3);
    assert_eq!(
        shell_dashboard_layout.attention_card_count,
        shell_dashboard_view.attention_card_count
    );
    assert_eq!(shell_dashboard_layout.regions[0].role, "status");
    assert!(shell_dashboard_layout.regions[0].visible);
    assert!(!shell_dashboard_layout.regions[0].primary);
    assert_eq!(shell_dashboard_layout.regions[1].role, "attention");
    assert!(shell_dashboard_layout.regions[1].primary);
    assert!(shell_dashboard_layout.regions[1].visible);
    assert_eq!(
        shell_dashboard_layout.regions[1].card_ids,
        vec!["dashboard.attention".to_string()]
    );
    assert_eq!(shell_dashboard_layout.regions[2].role, "metrics");
    assert!(shell_dashboard_layout.regions[2].visible);
    assert_eq!(
        shell_dashboard_layout.layout_capability_id,
        "app-shell-dashboard-layout-json"
    );

    let shell_dashboard_layout_payload: serde_json::Value = serde_json::from_str(
        &app.app_shell_dashboard_layout_json(BerkeleyAppPersistedEditorState {
            selected_syntax_card_index: Some(2),
            active_command_id: Some("analysis.2.run".to_string()),
        }),
    )
    .expect("blocked shell dashboard layout JSON should parse");
    assert_eq!(
        shell_dashboard_layout_payload["primaryRegionId"],
        "dashboard.layout.attention"
    );
    assert_eq!(shell_dashboard_layout_payload["visibleRegionCount"], 3);
    assert_eq!(
        shell_dashboard_layout_payload["regions"][1]["primary"],
        true
    );
    assert_eq!(
        shell_dashboard_layout_payload["regions"][1]["cardIds"][0],
        "dashboard.attention"
    );
    assert_eq!(
        shell_dashboard_layout_payload["layoutCapabilityId"],
        "app-shell-dashboard-layout-json"
    );

    let shell_dashboard_navigation =
        app.app_shell_dashboard_navigation(BerkeleyAppPersistedEditorState {
            selected_syntax_card_index: Some(2),
            active_command_id: Some("analysis.2.run".to_string()),
        });
    assert_eq!(
        shell_dashboard_navigation.schema_version,
        BERKELEY_APP_SHELL_DASHBOARD_NAVIGATION_SCHEMA_VERSION
    );
    assert!(!shell_dashboard_navigation.ready);
    assert_eq!(
        shell_dashboard_navigation.active_item_id.as_deref(),
        Some("dashboard.nav.attention")
    );
    assert_eq!(
        shell_dashboard_navigation.primary_region_id.as_deref(),
        Some("dashboard.layout.attention")
    );
    assert_eq!(shell_dashboard_navigation.item_count, 3);
    assert_eq!(shell_dashboard_navigation.visible_item_count, 3);
    assert_eq!(shell_dashboard_navigation.enabled_item_count, 3);
    assert_eq!(shell_dashboard_navigation.items[0].role, "status");
    assert!(!shell_dashboard_navigation.items[0].active);
    assert!(shell_dashboard_navigation.items[0].enabled);
    assert_eq!(
        shell_dashboard_navigation.items[1].id,
        "dashboard.nav.attention"
    );
    assert!(shell_dashboard_navigation.items[1].active);
    assert!(shell_dashboard_navigation.items[1].visible);
    assert!(shell_dashboard_navigation.items[1].enabled);
    assert_eq!(
        shell_dashboard_navigation.items[1].card_ids,
        vec!["dashboard.attention".to_string()]
    );
    assert_eq!(
        shell_dashboard_navigation.navigation_capability_id,
        "app-shell-dashboard-navigation-json"
    );

    let shell_dashboard_navigation_payload: serde_json::Value = serde_json::from_str(
        &app.app_shell_dashboard_navigation_json(BerkeleyAppPersistedEditorState {
            selected_syntax_card_index: Some(2),
            active_command_id: Some("analysis.2.run".to_string()),
        }),
    )
    .expect("blocked shell dashboard navigation JSON should parse");
    assert_eq!(
        shell_dashboard_navigation_payload["activeItemId"],
        "dashboard.nav.attention"
    );
    assert_eq!(shell_dashboard_navigation_payload["enabledItemCount"], 3);
    assert_eq!(
        shell_dashboard_navigation_payload["items"][1]["active"],
        true
    );
    assert_eq!(
        shell_dashboard_navigation_payload["items"][1]["regionId"],
        "dashboard.layout.attention"
    );
    assert_eq!(
        shell_dashboard_navigation_payload["navigationCapabilityId"],
        "app-shell-dashboard-navigation-json"
    );

    let shell_dashboard_routes = app.app_shell_dashboard_routes(BerkeleyAppPersistedEditorState {
        selected_syntax_card_index: Some(2),
        active_command_id: Some("analysis.2.run".to_string()),
    });
    assert_eq!(
        shell_dashboard_routes.schema_version,
        BERKELEY_APP_SHELL_DASHBOARD_ROUTES_SCHEMA_VERSION
    );
    assert!(!shell_dashboard_routes.ready);
    assert_eq!(
        shell_dashboard_routes.active_route_id.as_deref(),
        Some("dashboard.route.attention")
    );
    assert_eq!(
        shell_dashboard_routes.default_route_id.as_deref(),
        Some("dashboard.route.attention")
    );
    assert_eq!(
        shell_dashboard_routes.default_route_path.as_deref(),
        Some("/dashboard/attention")
    );
    assert_eq!(shell_dashboard_routes.route_count, 3);
    assert_eq!(shell_dashboard_routes.visible_route_count, 3);
    assert_eq!(shell_dashboard_routes.enabled_route_count, 3);
    assert_eq!(shell_dashboard_routes.routes[0].role, "status");
    assert!(!shell_dashboard_routes.routes[0].active);
    assert!(shell_dashboard_routes.routes[0].enabled);
    assert_eq!(
        shell_dashboard_routes.routes[1].id,
        "dashboard.route.attention"
    );
    assert_eq!(
        shell_dashboard_routes.routes[1].item_id,
        "dashboard.nav.attention"
    );
    assert!(shell_dashboard_routes.routes[1].active);
    assert!(shell_dashboard_routes.routes[1].default_route);
    assert_eq!(
        shell_dashboard_routes.routes[1].path,
        "/dashboard/attention"
    );
    assert_eq!(
        shell_dashboard_routes.routes_capability_id,
        "app-shell-dashboard-routes-json"
    );

    let shell_dashboard_routes_payload: serde_json::Value = serde_json::from_str(
        &app.app_shell_dashboard_routes_json(BerkeleyAppPersistedEditorState {
            selected_syntax_card_index: Some(2),
            active_command_id: Some("analysis.2.run".to_string()),
        }),
    )
    .expect("blocked shell dashboard routes JSON should parse");
    assert_eq!(
        shell_dashboard_routes_payload["activeRouteId"],
        "dashboard.route.attention"
    );
    assert_eq!(shell_dashboard_routes_payload["visibleRouteCount"], 3);
    assert_eq!(shell_dashboard_routes_payload["routes"][1]["default"], true);
    assert_eq!(
        shell_dashboard_routes_payload["routes"][1]["itemId"],
        "dashboard.nav.attention"
    );
    assert_eq!(
        shell_dashboard_routes_payload["routesCapabilityId"],
        "app-shell-dashboard-routes-json"
    );

    let shell_dashboard_breadcrumbs =
        app.app_shell_dashboard_breadcrumbs(BerkeleyAppPersistedEditorState {
            selected_syntax_card_index: Some(2),
            active_command_id: Some("analysis.2.run".to_string()),
        });
    assert_eq!(
        shell_dashboard_breadcrumbs.schema_version,
        BERKELEY_APP_SHELL_DASHBOARD_BREADCRUMBS_SCHEMA_VERSION
    );
    assert!(!shell_dashboard_breadcrumbs.ready);
    assert_eq!(
        shell_dashboard_breadcrumbs.active_breadcrumb_id.as_deref(),
        Some("dashboard.breadcrumb.attention")
    );
    assert_eq!(
        shell_dashboard_breadcrumbs.default_breadcrumb_id.as_deref(),
        Some("dashboard.breadcrumb.attention")
    );
    assert_eq!(
        shell_dashboard_breadcrumbs
            .default_breadcrumb_path
            .as_deref(),
        Some("/dashboard/attention")
    );
    assert_eq!(shell_dashboard_breadcrumbs.breadcrumb_count, 3);
    assert_eq!(shell_dashboard_breadcrumbs.visible_breadcrumb_count, 3);
    assert_eq!(shell_dashboard_breadcrumbs.enabled_breadcrumb_count, 3);
    assert_eq!(shell_dashboard_breadcrumbs.breadcrumbs[0].role, "status");
    assert!(!shell_dashboard_breadcrumbs.breadcrumbs[0].active);
    assert!(shell_dashboard_breadcrumbs.breadcrumbs[0].enabled);
    assert_eq!(
        shell_dashboard_breadcrumbs.breadcrumbs[1].id,
        "dashboard.breadcrumb.attention"
    );
    assert_eq!(
        shell_dashboard_breadcrumbs.breadcrumbs[1].route_id,
        "dashboard.route.attention"
    );
    assert!(shell_dashboard_breadcrumbs.breadcrumbs[1].active);
    assert!(shell_dashboard_breadcrumbs.breadcrumbs[1].default_route);
    assert_eq!(shell_dashboard_breadcrumbs.breadcrumbs[1].position, 2);
    assert_eq!(
        shell_dashboard_breadcrumbs.breadcrumbs_capability_id,
        "app-shell-dashboard-breadcrumbs-json"
    );

    let shell_dashboard_breadcrumbs_payload: serde_json::Value = serde_json::from_str(
        &app.app_shell_dashboard_breadcrumbs_json(BerkeleyAppPersistedEditorState {
            selected_syntax_card_index: Some(2),
            active_command_id: Some("analysis.2.run".to_string()),
        }),
    )
    .expect("blocked shell dashboard breadcrumbs JSON should parse");
    assert_eq!(
        shell_dashboard_breadcrumbs_payload["activeBreadcrumbId"],
        "dashboard.breadcrumb.attention"
    );
    assert_eq!(
        shell_dashboard_breadcrumbs_payload["enabledBreadcrumbCount"],
        3
    );
    assert_eq!(
        shell_dashboard_breadcrumbs_payload["breadcrumbs"][1]["default"],
        true
    );
    assert_eq!(
        shell_dashboard_breadcrumbs_payload["breadcrumbs"][1]["itemId"],
        "dashboard.nav.attention"
    );
    assert_eq!(
        shell_dashboard_breadcrumbs_payload["breadcrumbsCapabilityId"],
        "app-shell-dashboard-breadcrumbs-json"
    );

    let shell_dashboard_tabs = app.app_shell_dashboard_tabs(BerkeleyAppPersistedEditorState {
        selected_syntax_card_index: Some(2),
        active_command_id: Some("analysis.2.run".to_string()),
    });
    assert_eq!(
        shell_dashboard_tabs.schema_version,
        BERKELEY_APP_SHELL_DASHBOARD_TABS_SCHEMA_VERSION
    );
    assert!(!shell_dashboard_tabs.ready);
    assert_eq!(
        shell_dashboard_tabs.selected_tab_id.as_deref(),
        Some("dashboard.tab.attention")
    );
    assert_eq!(
        shell_dashboard_tabs.default_tab_id.as_deref(),
        Some("dashboard.tab.attention")
    );
    assert_eq!(
        shell_dashboard_tabs.default_tab_path.as_deref(),
        Some("/dashboard/attention")
    );
    assert_eq!(shell_dashboard_tabs.tab_count, 3);
    assert_eq!(shell_dashboard_tabs.visible_tab_count, 3);
    assert_eq!(shell_dashboard_tabs.enabled_tab_count, 3);
    assert_eq!(shell_dashboard_tabs.tabs[0].role, "status");
    assert!(!shell_dashboard_tabs.tabs[0].selected);
    assert!(shell_dashboard_tabs.tabs[0].enabled);
    assert_eq!(shell_dashboard_tabs.tabs[1].id, "dashboard.tab.attention");
    assert_eq!(
        shell_dashboard_tabs.tabs[1].breadcrumb_id,
        "dashboard.breadcrumb.attention"
    );
    assert_eq!(
        shell_dashboard_tabs.tabs[1].route_id,
        "dashboard.route.attention"
    );
    assert!(shell_dashboard_tabs.tabs[1].selected);
    assert!(shell_dashboard_tabs.tabs[1].default_tab);
    assert_eq!(shell_dashboard_tabs.tabs[1].position, 2);
    assert_eq!(
        shell_dashboard_tabs.tabs_capability_id,
        "app-shell-dashboard-tabs-json"
    );

    let shell_dashboard_tabs_payload: serde_json::Value = serde_json::from_str(
        &app.app_shell_dashboard_tabs_json(BerkeleyAppPersistedEditorState {
            selected_syntax_card_index: Some(2),
            active_command_id: Some("analysis.2.run".to_string()),
        }),
    )
    .expect("blocked shell dashboard tabs JSON should parse");
    assert_eq!(
        shell_dashboard_tabs_payload["selectedTabId"],
        "dashboard.tab.attention"
    );
    assert_eq!(shell_dashboard_tabs_payload["enabledTabCount"], 3);
    assert_eq!(shell_dashboard_tabs_payload["tabs"][1]["default"], true);
    assert_eq!(
        shell_dashboard_tabs_payload["tabs"][1]["breadcrumbId"],
        "dashboard.breadcrumb.attention"
    );
    assert_eq!(
        shell_dashboard_tabs_payload["tabsCapabilityId"],
        "app-shell-dashboard-tabs-json"
    );

    let shell_dashboard_tab_panels =
        app.app_shell_dashboard_tab_panels(BerkeleyAppPersistedEditorState {
            selected_syntax_card_index: Some(2),
            active_command_id: Some("analysis.2.run".to_string()),
        });
    assert_eq!(
        shell_dashboard_tab_panels.schema_version,
        BERKELEY_APP_SHELL_DASHBOARD_TAB_PANELS_SCHEMA_VERSION
    );
    assert!(!shell_dashboard_tab_panels.ready);
    assert_eq!(
        shell_dashboard_tab_panels.selected_panel_id.as_deref(),
        Some("dashboard.tab-panel.attention")
    );
    assert_eq!(
        shell_dashboard_tab_panels.default_panel_id.as_deref(),
        Some("dashboard.tab-panel.attention")
    );
    assert_eq!(
        shell_dashboard_tab_panels.default_panel_path.as_deref(),
        Some("/dashboard/attention")
    );
    assert_eq!(shell_dashboard_tab_panels.panel_count, 3);
    assert_eq!(shell_dashboard_tab_panels.visible_panel_count, 3);
    assert_eq!(shell_dashboard_tab_panels.enabled_panel_count, 3);
    assert_eq!(shell_dashboard_tab_panels.panels[0].role, "status");
    assert!(!shell_dashboard_tab_panels.panels[0].selected);
    assert!(shell_dashboard_tab_panels.panels[0].enabled);
    assert_eq!(
        shell_dashboard_tab_panels.panels[1].id,
        "dashboard.tab-panel.attention"
    );
    assert_eq!(
        shell_dashboard_tab_panels.panels[1].tab_id,
        "dashboard.tab.attention"
    );
    assert_eq!(
        shell_dashboard_tab_panels.panels[1].region_id,
        "dashboard.layout.attention"
    );
    assert!(shell_dashboard_tab_panels.panels[1].selected);
    assert!(shell_dashboard_tab_panels.panels[1].default_panel);
    assert_eq!(shell_dashboard_tab_panels.panels[1].position, 2);
    assert_eq!(
        shell_dashboard_tab_panels.tab_panels_capability_id,
        "app-shell-dashboard-tab-panels-json"
    );

    let shell_dashboard_tab_panels_payload: serde_json::Value = serde_json::from_str(
        &app.app_shell_dashboard_tab_panels_json(BerkeleyAppPersistedEditorState {
            selected_syntax_card_index: Some(2),
            active_command_id: Some("analysis.2.run".to_string()),
        }),
    )
    .expect("blocked shell dashboard tab panels JSON should parse");
    assert_eq!(
        shell_dashboard_tab_panels_payload["selectedPanelId"],
        "dashboard.tab-panel.attention"
    );
    assert_eq!(shell_dashboard_tab_panels_payload["enabledPanelCount"], 3);
    assert_eq!(
        shell_dashboard_tab_panels_payload["panels"][1]["default"],
        true
    );
    assert_eq!(
        shell_dashboard_tab_panels_payload["panels"][1]["tabId"],
        "dashboard.tab.attention"
    );
    assert_eq!(
        shell_dashboard_tab_panels_payload["tabPanelsCapabilityId"],
        "app-shell-dashboard-tab-panels-json"
    );

    let shell_dashboard_panel_cards =
        app.app_shell_dashboard_panel_cards(BerkeleyAppPersistedEditorState {
            selected_syntax_card_index: Some(2),
            active_command_id: Some("analysis.2.run".to_string()),
        });
    assert_eq!(
        shell_dashboard_panel_cards.schema_version,
        BERKELEY_APP_SHELL_DASHBOARD_PANEL_CARDS_SCHEMA_VERSION
    );
    assert!(!shell_dashboard_panel_cards.ready);
    assert_eq!(
        shell_dashboard_panel_cards
            .selected_panel_card_id
            .as_deref(),
        Some("dashboard.panel-card.attention")
    );
    assert_eq!(
        shell_dashboard_panel_cards.selected_card_id.as_deref(),
        Some("dashboard.attention")
    );
    assert_eq!(
        shell_dashboard_panel_cards.default_panel_card_id.as_deref(),
        Some("dashboard.panel-card.attention")
    );
    assert_eq!(
        shell_dashboard_panel_cards.default_card_id.as_deref(),
        Some("dashboard.attention")
    );
    assert_eq!(shell_dashboard_panel_cards.panel_card_count, 3);
    assert_eq!(shell_dashboard_panel_cards.visible_panel_card_count, 3);
    assert_eq!(shell_dashboard_panel_cards.enabled_panel_card_count, 3);
    assert_eq!(shell_dashboard_panel_cards.panel_cards[0].role, "status");
    assert!(!shell_dashboard_panel_cards.panel_cards[0].selected);
    assert!(shell_dashboard_panel_cards.panel_cards[0].enabled);
    assert_eq!(
        shell_dashboard_panel_cards.panel_cards[1].id,
        "dashboard.panel-card.attention"
    );
    assert_eq!(
        shell_dashboard_panel_cards.panel_cards[1].panel_id,
        "dashboard.tab-panel.attention"
    );
    assert_eq!(
        shell_dashboard_panel_cards.panel_cards[1].card_id,
        "dashboard.attention"
    );
    assert!(shell_dashboard_panel_cards.panel_cards[1].selected);
    assert!(shell_dashboard_panel_cards.panel_cards[1].default_panel);
    assert!(shell_dashboard_panel_cards.panel_cards[1].attention);
    assert_eq!(shell_dashboard_panel_cards.panel_cards[1].position, 2);
    assert_eq!(
        shell_dashboard_panel_cards.panel_cards_capability_id,
        "app-shell-dashboard-panel-cards-json"
    );

    let shell_dashboard_panel_cards_payload: serde_json::Value = serde_json::from_str(
        &app.app_shell_dashboard_panel_cards_json(BerkeleyAppPersistedEditorState {
            selected_syntax_card_index: Some(2),
            active_command_id: Some("analysis.2.run".to_string()),
        }),
    )
    .expect("blocked shell dashboard panel cards JSON should parse");
    assert_eq!(
        shell_dashboard_panel_cards_payload["selectedPanelCardId"],
        "dashboard.panel-card.attention"
    );
    assert_eq!(
        shell_dashboard_panel_cards_payload["selectedCardId"],
        "dashboard.attention"
    );
    assert_eq!(
        shell_dashboard_panel_cards_payload["enabledPanelCardCount"],
        3
    );
    assert_eq!(
        shell_dashboard_panel_cards_payload["panelCards"][1]["attention"],
        true
    );
    assert_eq!(
        shell_dashboard_panel_cards_payload["panelCards"][1]["cardId"],
        "dashboard.attention"
    );
    assert_eq!(
        shell_dashboard_panel_cards_payload["panelCardsCapabilityId"],
        "app-shell-dashboard-panel-cards-json"
    );

    let shell_dashboard_panel_card_actions =
        app.app_shell_dashboard_panel_card_actions(BerkeleyAppPersistedEditorState {
            selected_syntax_card_index: Some(2),
            active_command_id: Some("analysis.2.run".to_string()),
        });
    assert_eq!(
        shell_dashboard_panel_card_actions.schema_version,
        BERKELEY_APP_SHELL_DASHBOARD_PANEL_CARD_ACTIONS_SCHEMA_VERSION
    );
    assert!(!shell_dashboard_panel_card_actions.ready);
    assert_eq!(
        shell_dashboard_panel_card_actions
            .selected_panel_card_action_id
            .as_deref(),
        Some("dashboard.panel-card-action.attention")
    );
    assert_eq!(
        shell_dashboard_panel_card_actions
            .selected_action_id
            .as_deref(),
        Some("launch.diagnostics")
    );
    assert_eq!(
        shell_dashboard_panel_card_actions
            .default_panel_card_action_id
            .as_deref(),
        Some("dashboard.panel-card-action.attention")
    );
    assert_eq!(
        shell_dashboard_panel_card_actions
            .default_action_id
            .as_deref(),
        Some("launch.diagnostics")
    );
    assert_eq!(
        shell_dashboard_panel_card_actions.panel_card_action_count,
        3
    );
    assert_eq!(
        shell_dashboard_panel_card_actions.visible_panel_card_action_count,
        3
    );
    assert_eq!(
        shell_dashboard_panel_card_actions.enabled_panel_card_action_count,
        3
    );
    assert_eq!(
        shell_dashboard_panel_card_actions.panel_card_actions[0].role,
        "status"
    );
    assert_eq!(
        shell_dashboard_panel_card_actions.panel_card_actions[0].action_id,
        "launch.source"
    );
    assert_eq!(
        shell_dashboard_panel_card_actions.panel_card_actions[1].id,
        "dashboard.panel-card-action.attention"
    );
    assert_eq!(
        shell_dashboard_panel_card_actions.panel_card_actions[1].action_id,
        "launch.diagnostics"
    );
    assert!(shell_dashboard_panel_card_actions.panel_card_actions[1].selected);
    assert!(shell_dashboard_panel_card_actions.panel_card_actions[1].primary);
    assert!(shell_dashboard_panel_card_actions.panel_card_actions[1].attention);
    assert_eq!(
        shell_dashboard_panel_card_actions.panel_card_actions_capability_id,
        "app-shell-dashboard-panel-card-actions-json"
    );

    let shell_dashboard_panel_card_actions_payload: serde_json::Value = serde_json::from_str(
        &app.app_shell_dashboard_panel_card_actions_json(BerkeleyAppPersistedEditorState {
            selected_syntax_card_index: Some(2),
            active_command_id: Some("analysis.2.run".to_string()),
        }),
    )
    .expect("blocked shell dashboard panel-card actions JSON should parse");
    assert_eq!(
        shell_dashboard_panel_card_actions_payload["selectedPanelCardActionId"],
        "dashboard.panel-card-action.attention"
    );
    assert_eq!(
        shell_dashboard_panel_card_actions_payload["selectedActionId"],
        "launch.diagnostics"
    );
    assert_eq!(
        shell_dashboard_panel_card_actions_payload["enabledPanelCardActionCount"],
        3
    );
    assert_eq!(
        shell_dashboard_panel_card_actions_payload["panelCardActions"][1]["attention"],
        true
    );
    assert_eq!(
        shell_dashboard_panel_card_actions_payload["panelCardActions"][1]["actionId"],
        "launch.diagnostics"
    );
    assert_eq!(
        shell_dashboard_panel_card_actions_payload["panelCardActionsCapabilityId"],
        "app-shell-dashboard-panel-card-actions-json"
    );

    let shell_dashboard_action_dispatch =
        app.app_shell_dashboard_action_dispatch(BerkeleyAppPersistedEditorState {
            selected_syntax_card_index: Some(2),
            active_command_id: Some("analysis.2.run".to_string()),
        });
    assert_eq!(
        shell_dashboard_action_dispatch.schema_version,
        BERKELEY_APP_SHELL_DASHBOARD_ACTION_DISPATCH_SCHEMA_VERSION
    );
    assert!(!shell_dashboard_action_dispatch.ready);
    assert_eq!(
        shell_dashboard_action_dispatch
            .selected_action_dispatch_id
            .as_deref(),
        Some("dashboard.action-dispatch.attention")
    );
    assert_eq!(
        shell_dashboard_action_dispatch
            .selected_action_id
            .as_deref(),
        Some("launch.diagnostics")
    );
    assert_eq!(
        shell_dashboard_action_dispatch
            .default_action_dispatch_id
            .as_deref(),
        Some("dashboard.action-dispatch.attention")
    );
    assert_eq!(
        shell_dashboard_action_dispatch.default_action_id.as_deref(),
        Some("launch.diagnostics")
    );
    assert_eq!(shell_dashboard_action_dispatch.action_dispatch_count, 3);
    assert_eq!(
        shell_dashboard_action_dispatch.visible_action_dispatch_count,
        3
    );
    assert_eq!(
        shell_dashboard_action_dispatch.enabled_action_dispatch_count,
        3
    );
    assert_eq!(
        shell_dashboard_action_dispatch.action_dispatches[0].action_id,
        "launch.source"
    );
    assert_eq!(
        shell_dashboard_action_dispatch.action_dispatches[1].id,
        "dashboard.action-dispatch.attention"
    );
    assert_eq!(
        shell_dashboard_action_dispatch.action_dispatches[1].panel_card_action_id,
        "dashboard.panel-card-action.attention"
    );
    assert!(shell_dashboard_action_dispatch.action_dispatches[1].dispatchable);
    assert!(shell_dashboard_action_dispatch.action_dispatches[1].attention);
    assert_eq!(
        shell_dashboard_action_dispatch.action_dispatch_capability_id,
        "app-shell-dashboard-action-dispatch-json"
    );

    let shell_dashboard_action_dispatch_payload: serde_json::Value = serde_json::from_str(
        &app.app_shell_dashboard_action_dispatch_json(BerkeleyAppPersistedEditorState {
            selected_syntax_card_index: Some(2),
            active_command_id: Some("analysis.2.run".to_string()),
        }),
    )
    .expect("blocked shell dashboard action dispatch JSON should parse");
    assert_eq!(
        shell_dashboard_action_dispatch_payload["selectedActionDispatchId"],
        "dashboard.action-dispatch.attention"
    );
    assert_eq!(
        shell_dashboard_action_dispatch_payload["selectedActionId"],
        "launch.diagnostics"
    );
    assert_eq!(
        shell_dashboard_action_dispatch_payload["enabledActionDispatchCount"],
        3
    );
    assert_eq!(
        shell_dashboard_action_dispatch_payload["actionDispatches"][1]["dispatchable"],
        true
    );
    assert_eq!(
        shell_dashboard_action_dispatch_payload["actionDispatches"][1]["actionId"],
        "launch.diagnostics"
    );
    assert_eq!(
        shell_dashboard_action_dispatch_payload["actionDispatchCapabilityId"],
        "app-shell-dashboard-action-dispatch-json"
    );

    let shell_dashboard_dispatch_events =
        app.app_shell_dashboard_dispatch_events(BerkeleyAppPersistedEditorState {
            selected_syntax_card_index: Some(2),
            active_command_id: Some("analysis.2.run".to_string()),
        });
    assert_eq!(
        shell_dashboard_dispatch_events.schema_version,
        BERKELEY_APP_SHELL_DASHBOARD_DISPATCH_EVENTS_SCHEMA_VERSION
    );
    assert!(!shell_dashboard_dispatch_events.ready);
    assert_eq!(
        shell_dashboard_dispatch_events
            .selected_dispatch_event_id
            .as_deref(),
        Some("dashboard.dispatch-event.attention")
    );
    assert_eq!(
        shell_dashboard_dispatch_events
            .default_dispatch_event_id
            .as_deref(),
        Some("dashboard.dispatch-event.attention")
    );
    assert_eq!(shell_dashboard_dispatch_events.dispatch_event_count, 3);
    assert_eq!(
        shell_dashboard_dispatch_events.dispatch_ready_event_count,
        3
    );
    assert_eq!(
        shell_dashboard_dispatch_events.dispatch_blocked_event_count,
        0
    );
    assert_eq!(
        shell_dashboard_dispatch_events.attention_dispatch_event_count,
        1
    );
    assert!(shell_dashboard_dispatch_events.selected_dispatchable);
    assert!(shell_dashboard_dispatch_events.default_dispatchable);
    assert_eq!(
        shell_dashboard_dispatch_events.dispatch_events[1].id,
        "dashboard.dispatch-event.attention"
    );
    assert_eq!(
        shell_dashboard_dispatch_events.dispatch_events[1].kind,
        "dispatch-ready"
    );
    assert_eq!(
        shell_dashboard_dispatch_events.dispatch_events[1].severity,
        "warning"
    );
    assert!(shell_dashboard_dispatch_events.dispatch_events[1].attention);
    assert_eq!(
        shell_dashboard_dispatch_events.dispatch_events_capability_id,
        "app-shell-dashboard-dispatch-events-json"
    );

    let shell_dashboard_dispatch_events_payload: serde_json::Value = serde_json::from_str(
        &app.app_shell_dashboard_dispatch_events_json(BerkeleyAppPersistedEditorState {
            selected_syntax_card_index: Some(2),
            active_command_id: Some("analysis.2.run".to_string()),
        }),
    )
    .expect("blocked shell dashboard dispatch events JSON should parse");
    assert_eq!(
        shell_dashboard_dispatch_events_payload["selectedDispatchEventId"],
        "dashboard.dispatch-event.attention"
    );
    assert_eq!(
        shell_dashboard_dispatch_events_payload["dispatchReadyEventCount"],
        3
    );
    assert_eq!(
        shell_dashboard_dispatch_events_payload["dispatchBlockedEventCount"],
        0
    );
    assert_eq!(
        shell_dashboard_dispatch_events_payload["dispatchEvents"][1]["severity"],
        "warning"
    );
    assert_eq!(
        shell_dashboard_dispatch_events_payload["dispatchEvents"][1]["actionDispatchId"],
        "dashboard.action-dispatch.attention"
    );
    assert_eq!(
        shell_dashboard_dispatch_events_payload["dispatchEventsCapabilityId"],
        "app-shell-dashboard-dispatch-events-json"
    );

    let shell_dashboard_dispatch_queue =
        app.app_shell_dashboard_dispatch_queue(BerkeleyAppPersistedEditorState {
            selected_syntax_card_index: Some(2),
            active_command_id: Some("analysis.2.run".to_string()),
        });
    assert_eq!(
        shell_dashboard_dispatch_queue.schema_version,
        BERKELEY_APP_SHELL_DASHBOARD_DISPATCH_QUEUE_SCHEMA_VERSION
    );
    assert!(!shell_dashboard_dispatch_queue.ready);
    assert_eq!(
        shell_dashboard_dispatch_queue
            .selected_dispatch_queue_item_id
            .as_deref(),
        Some("dashboard.dispatch-queue.attention")
    );
    assert_eq!(
        shell_dashboard_dispatch_queue
            .default_dispatch_queue_item_id
            .as_deref(),
        Some("dashboard.dispatch-queue.attention")
    );
    assert_eq!(shell_dashboard_dispatch_queue.dispatch_queue_item_count, 3);
    assert_eq!(shell_dashboard_dispatch_queue.queued_dispatch_count, 3);
    assert_eq!(shell_dashboard_dispatch_queue.blocked_dispatch_count, 0);
    assert_eq!(
        shell_dashboard_dispatch_queue.attention_dispatch_queue_item_count,
        1
    );
    assert!(shell_dashboard_dispatch_queue.selected_queued);
    assert!(shell_dashboard_dispatch_queue.default_queued);
    assert_eq!(
        shell_dashboard_dispatch_queue.dispatch_queue_items[1].id,
        "dashboard.dispatch-queue.attention"
    );
    assert_eq!(
        shell_dashboard_dispatch_queue.dispatch_queue_items[1].queue_state,
        "queued"
    );
    assert_eq!(
        shell_dashboard_dispatch_queue.dispatch_queue_items[1].severity,
        "warning"
    );
    assert!(shell_dashboard_dispatch_queue.dispatch_queue_items[1].attention);
    assert_eq!(
        shell_dashboard_dispatch_queue.dispatch_queue_capability_id,
        "app-shell-dashboard-dispatch-queue-json"
    );

    let shell_dashboard_dispatch_queue_payload: serde_json::Value = serde_json::from_str(
        &app.app_shell_dashboard_dispatch_queue_json(BerkeleyAppPersistedEditorState {
            selected_syntax_card_index: Some(2),
            active_command_id: Some("analysis.2.run".to_string()),
        }),
    )
    .expect("blocked shell dashboard dispatch queue JSON should parse");
    assert_eq!(
        shell_dashboard_dispatch_queue_payload["selectedDispatchQueueItemId"],
        "dashboard.dispatch-queue.attention"
    );
    assert_eq!(
        shell_dashboard_dispatch_queue_payload["queuedDispatchCount"],
        3
    );
    assert_eq!(
        shell_dashboard_dispatch_queue_payload["blockedDispatchCount"],
        0
    );
    assert_eq!(
        shell_dashboard_dispatch_queue_payload["dispatchQueueItems"][1]["severity"],
        "warning"
    );
    assert_eq!(
        shell_dashboard_dispatch_queue_payload["dispatchQueueItems"][1]["dispatchEventId"],
        "dashboard.dispatch-event.attention"
    );
    assert_eq!(
        shell_dashboard_dispatch_queue_payload["dispatchQueueCapabilityId"],
        "app-shell-dashboard-dispatch-queue-json"
    );

    let shell_dashboard_dispatch_queue_summary =
        app.app_shell_dashboard_dispatch_queue_summary(BerkeleyAppPersistedEditorState {
            selected_syntax_card_index: Some(2),
            active_command_id: Some("analysis.2.run".to_string()),
        });
    assert_eq!(
        shell_dashboard_dispatch_queue_summary.schema_version,
        BERKELEY_APP_SHELL_DASHBOARD_DISPATCH_QUEUE_SUMMARY_SCHEMA_VERSION
    );
    assert!(!shell_dashboard_dispatch_queue_summary.ready);
    assert_eq!(
        shell_dashboard_dispatch_queue_summary
            .selected_dispatch_queue_item_id
            .as_deref(),
        Some("dashboard.dispatch-queue.attention")
    );
    assert_eq!(
        shell_dashboard_dispatch_queue_summary
            .default_dispatch_queue_item_id
            .as_deref(),
        Some("dashboard.dispatch-queue.attention")
    );
    assert_eq!(
        shell_dashboard_dispatch_queue_summary.dispatch_queue_item_count,
        3
    );
    assert_eq!(
        shell_dashboard_dispatch_queue_summary.queued_dispatch_count,
        3
    );
    assert_eq!(
        shell_dashboard_dispatch_queue_summary.blocked_dispatch_count,
        0
    );
    assert_eq!(
        shell_dashboard_dispatch_queue_summary.attention_dispatch_queue_item_count,
        1
    );
    assert!(shell_dashboard_dispatch_queue_summary.selected_queued);
    assert!(shell_dashboard_dispatch_queue_summary.default_queued);
    assert_eq!(
        shell_dashboard_dispatch_queue_summary
            .first_queued_dispatch_queue_item_id
            .as_deref(),
        Some("dashboard.dispatch-queue.status")
    );
    assert_eq!(
        shell_dashboard_dispatch_queue_summary
            .first_blocked_dispatch_queue_item_id
            .as_deref(),
        None
    );
    assert_eq!(
        shell_dashboard_dispatch_queue_summary
            .first_attention_dispatch_queue_item_id
            .as_deref(),
        Some("dashboard.dispatch-queue.attention")
    );
    assert_eq!(
        shell_dashboard_dispatch_queue_summary.queued_dispatch_queue_item_ids,
        vec![
            "dashboard.dispatch-queue.status".to_string(),
            "dashboard.dispatch-queue.attention".to_string(),
            "dashboard.dispatch-queue.metrics".to_string()
        ]
    );
    assert_eq!(
        shell_dashboard_dispatch_queue_summary.attention_dispatch_queue_item_ids,
        vec!["dashboard.dispatch-queue.attention".to_string()]
    );
    assert_eq!(
        shell_dashboard_dispatch_queue_summary.dispatch_queue_summary_capability_id,
        "app-shell-dashboard-dispatch-queue-summary-json"
    );

    let shell_dashboard_dispatch_queue_summary_payload: serde_json::Value = serde_json::from_str(
        &app.app_shell_dashboard_dispatch_queue_summary_json(BerkeleyAppPersistedEditorState {
            selected_syntax_card_index: Some(2),
            active_command_id: Some("analysis.2.run".to_string()),
        }),
    )
    .expect("blocked shell dashboard dispatch queue summary JSON should parse");
    assert_eq!(
        shell_dashboard_dispatch_queue_summary_payload["selectedDispatchQueueItemId"],
        "dashboard.dispatch-queue.attention"
    );
    assert_eq!(
        shell_dashboard_dispatch_queue_summary_payload["queuedDispatchQueueItemIds"][1],
        "dashboard.dispatch-queue.attention"
    );
    assert!(
        shell_dashboard_dispatch_queue_summary_payload["firstBlockedDispatchQueueItemId"].is_null()
    );
    assert_eq!(
        shell_dashboard_dispatch_queue_summary_payload["firstAttentionDispatchQueueItemId"],
        "dashboard.dispatch-queue.attention"
    );
    assert_eq!(
        shell_dashboard_dispatch_queue_summary_payload["dispatchQueueSummaryCapabilityId"],
        "app-shell-dashboard-dispatch-queue-summary-json"
    );

    let shell_dashboard_dispatch_queue_digest =
        app.app_shell_dashboard_dispatch_queue_digest(BerkeleyAppPersistedEditorState {
            selected_syntax_card_index: Some(2),
            active_command_id: Some("analysis.2.run".to_string()),
        });
    assert_eq!(
        shell_dashboard_dispatch_queue_digest.schema_version,
        BERKELEY_APP_SHELL_DASHBOARD_DISPATCH_QUEUE_DIGEST_SCHEMA_VERSION
    );
    assert!(!shell_dashboard_dispatch_queue_digest.ready);
    assert_eq!(
        shell_dashboard_dispatch_queue_digest
            .headline_dispatch_queue_item_id
            .as_deref(),
        Some("dashboard.dispatch-queue.attention")
    );
    assert_eq!(
        shell_dashboard_dispatch_queue_digest
            .headline_dispatch_event_id
            .as_deref(),
        Some("dashboard.dispatch-event.attention")
    );
    assert_eq!(
        shell_dashboard_dispatch_queue_digest
            .headline_action_dispatch_id
            .as_deref(),
        Some("dashboard.action-dispatch.attention")
    );
    assert_eq!(
        shell_dashboard_dispatch_queue_digest
            .headline_queue_state
            .as_deref(),
        Some("queued")
    );
    assert_eq!(
        shell_dashboard_dispatch_queue_digest.headline_message,
        "Diagnostics queued for dispatch"
    );
    assert!(shell_dashboard_dispatch_queue_digest.headline_selected);
    assert!(shell_dashboard_dispatch_queue_digest.headline_default_dispatch);
    assert!(shell_dashboard_dispatch_queue_digest.headline_attention);
    assert_eq!(
        shell_dashboard_dispatch_queue_digest
            .first_attention_dispatch_queue_item_id
            .as_deref(),
        Some("dashboard.dispatch-queue.attention")
    );
    assert_eq!(
        shell_dashboard_dispatch_queue_digest.attention_dispatch_queue_item_count,
        1
    );
    assert_eq!(
        shell_dashboard_dispatch_queue_digest.dispatch_queue_digest_capability_id,
        "app-shell-dashboard-dispatch-queue-digest-json"
    );

    let shell_dashboard_dispatch_queue_digest_payload: serde_json::Value = serde_json::from_str(
        &app.app_shell_dashboard_dispatch_queue_digest_json(BerkeleyAppPersistedEditorState {
            selected_syntax_card_index: Some(2),
            active_command_id: Some("analysis.2.run".to_string()),
        }),
    )
    .expect("blocked shell dashboard dispatch queue digest JSON should parse");
    assert_eq!(
        shell_dashboard_dispatch_queue_digest_payload["headlineDispatchQueueItemId"],
        "dashboard.dispatch-queue.attention"
    );
    assert_eq!(
        shell_dashboard_dispatch_queue_digest_payload["headlineMessage"],
        "Diagnostics queued for dispatch"
    );
    assert_eq!(
        shell_dashboard_dispatch_queue_digest_payload["headlineAttention"],
        true
    );
    assert_eq!(
        shell_dashboard_dispatch_queue_digest_payload["firstAttentionDispatchQueueItemId"],
        "dashboard.dispatch-queue.attention"
    );
    assert_eq!(
        shell_dashboard_dispatch_queue_digest_payload["dispatchQueueDigestCapabilityId"],
        "app-shell-dashboard-dispatch-queue-digest-json"
    );

    let shell_dashboard_dispatch_queue_lanes =
        app.app_shell_dashboard_dispatch_queue_lanes(BerkeleyAppPersistedEditorState {
            selected_syntax_card_index: Some(2),
            active_command_id: Some("analysis.2.run".to_string()),
        });
    assert_eq!(
        shell_dashboard_dispatch_queue_lanes.schema_version,
        BERKELEY_APP_SHELL_DASHBOARD_DISPATCH_QUEUE_LANES_SCHEMA_VERSION
    );
    assert!(!shell_dashboard_dispatch_queue_lanes.ready);
    assert_eq!(
        shell_dashboard_dispatch_queue_lanes
            .headline_dispatch_queue_item_id
            .as_deref(),
        Some("dashboard.dispatch-queue.attention")
    );
    assert_eq!(
        shell_dashboard_dispatch_queue_lanes
            .active_lane_id
            .as_deref(),
        Some("dashboard.dispatch-queue-lane.attention")
    );
    assert_eq!(
        shell_dashboard_dispatch_queue_lanes
            .attention_lane_id
            .as_deref(),
        Some("dashboard.dispatch-queue-lane.attention")
    );
    assert_eq!(shell_dashboard_dispatch_queue_lanes.lane_count, 3);
    assert_eq!(
        shell_dashboard_dispatch_queue_lanes.lanes[0].dispatch_queue_item_ids,
        vec![
            "dashboard.dispatch-queue.status".to_string(),
            "dashboard.dispatch-queue.attention".to_string(),
            "dashboard.dispatch-queue.metrics".to_string()
        ]
    );
    assert!(shell_dashboard_dispatch_queue_lanes.lanes[0].selected);
    assert!(shell_dashboard_dispatch_queue_lanes.lanes[0].default_dispatch);
    assert_eq!(
        shell_dashboard_dispatch_queue_lanes.lanes[1].dispatch_queue_item_count,
        0
    );
    assert_eq!(
        shell_dashboard_dispatch_queue_lanes.lanes[2].dispatch_queue_item_ids,
        vec!["dashboard.dispatch-queue.attention".to_string()]
    );
    assert!(shell_dashboard_dispatch_queue_lanes.lanes[2].attention);
    assert!(shell_dashboard_dispatch_queue_lanes.lanes[2].selected);
    assert!(shell_dashboard_dispatch_queue_lanes.lanes[2].default_dispatch);
    assert_eq!(
        shell_dashboard_dispatch_queue_lanes.dispatch_queue_digest_capability_id,
        "app-shell-dashboard-dispatch-queue-digest-json"
    );

    let shell_dashboard_dispatch_queue_lanes_payload: serde_json::Value = serde_json::from_str(
        &app.app_shell_dashboard_dispatch_queue_lanes_json(BerkeleyAppPersistedEditorState {
            selected_syntax_card_index: Some(2),
            active_command_id: Some("analysis.2.run".to_string()),
        }),
    )
    .expect("blocked shell dashboard dispatch queue lanes JSON should parse");
    assert_eq!(
        shell_dashboard_dispatch_queue_lanes_payload["activeLaneId"],
        "dashboard.dispatch-queue-lane.attention"
    );
    assert_eq!(
        shell_dashboard_dispatch_queue_lanes_payload["attentionLaneId"],
        "dashboard.dispatch-queue-lane.attention"
    );
    assert_eq!(
        shell_dashboard_dispatch_queue_lanes_payload["lanes"][2]["dispatchQueueItemIds"][0],
        "dashboard.dispatch-queue.attention"
    );
    assert_eq!(
        shell_dashboard_dispatch_queue_lanes_payload["lanes"][2]["attention"],
        true
    );
    assert_eq!(
        shell_dashboard_dispatch_queue_lanes_payload["dispatchQueueLanesCapabilityId"],
        "app-shell-dashboard-dispatch-queue-lanes-json"
    );

    let shell_dashboard_dispatch_queue_lane_tabs = app
        .app_shell_dashboard_dispatch_queue_lane_tabs(BerkeleyAppPersistedEditorState {
            selected_syntax_card_index: Some(2),
            active_command_id: Some("analysis.2.run".to_string()),
        });
    assert_eq!(
        shell_dashboard_dispatch_queue_lane_tabs.schema_version,
        BERKELEY_APP_SHELL_DASHBOARD_DISPATCH_QUEUE_LANE_TABS_SCHEMA_VERSION
    );
    assert!(!shell_dashboard_dispatch_queue_lane_tabs.ready);
    assert_eq!(
        shell_dashboard_dispatch_queue_lane_tabs
            .active_tab_id
            .as_deref(),
        Some("dashboard.dispatch-queue-lane-tab.attention")
    );
    assert_eq!(
        shell_dashboard_dispatch_queue_lane_tabs
            .attention_tab_id
            .as_deref(),
        Some("dashboard.dispatch-queue-lane-tab.attention")
    );
    assert_eq!(shell_dashboard_dispatch_queue_lane_tabs.tab_count, 3);
    assert_eq!(
        shell_dashboard_dispatch_queue_lane_tabs.enabled_tab_count,
        2
    );
    assert_eq!(
        shell_dashboard_dispatch_queue_lane_tabs.disabled_tab_count,
        1
    );
    assert!(shell_dashboard_dispatch_queue_lane_tabs.tabs[2].active);
    assert!(shell_dashboard_dispatch_queue_lane_tabs.tabs[2].attention);
    assert!(shell_dashboard_dispatch_queue_lane_tabs.tabs[2].selected);
    assert!(shell_dashboard_dispatch_queue_lane_tabs.tabs[2].default_dispatch);
    assert!(shell_dashboard_dispatch_queue_lane_tabs.tabs[1].disabled);
    assert_eq!(
        shell_dashboard_dispatch_queue_lane_tabs.dispatch_queue_lane_tabs_capability_id,
        "app-shell-dashboard-dispatch-queue-lane-tabs-json"
    );

    let shell_dashboard_dispatch_queue_lane_tabs_payload: serde_json::Value = serde_json::from_str(
        &app.app_shell_dashboard_dispatch_queue_lane_tabs_json(BerkeleyAppPersistedEditorState {
            selected_syntax_card_index: Some(2),
            active_command_id: Some("analysis.2.run".to_string()),
        }),
    )
    .expect("blocked shell dashboard dispatch queue lane tabs JSON should parse");
    assert_eq!(
        shell_dashboard_dispatch_queue_lane_tabs_payload["activeTabId"],
        "dashboard.dispatch-queue-lane-tab.attention"
    );
    assert_eq!(
        shell_dashboard_dispatch_queue_lane_tabs_payload["attentionTabId"],
        "dashboard.dispatch-queue-lane-tab.attention"
    );
    assert_eq!(
        shell_dashboard_dispatch_queue_lane_tabs_payload["tabs"][1]["disabled"],
        true
    );
    assert_eq!(
        shell_dashboard_dispatch_queue_lane_tabs_payload["tabs"][2]["attention"],
        true
    );
    assert_eq!(
        shell_dashboard_dispatch_queue_lane_tabs_payload["dispatchQueueLaneTabsCapabilityId"],
        "app-shell-dashboard-dispatch-queue-lane-tabs-json"
    );

    let shell_dashboard_dispatch_queue_lane_tab_panels = app
        .app_shell_dashboard_dispatch_queue_lane_tab_panels(BerkeleyAppPersistedEditorState {
            selected_syntax_card_index: Some(2),
            active_command_id: Some("analysis.2.run".to_string()),
        });
    assert_eq!(
        shell_dashboard_dispatch_queue_lane_tab_panels.schema_version,
        BERKELEY_APP_SHELL_DASHBOARD_DISPATCH_QUEUE_LANE_TAB_PANELS_SCHEMA_VERSION
    );
    assert!(!shell_dashboard_dispatch_queue_lane_tab_panels.ready);
    assert_eq!(
        shell_dashboard_dispatch_queue_lane_tab_panels
            .active_panel_id
            .as_deref(),
        Some("dashboard.dispatch-queue-lane-tab-panel.attention")
    );
    assert_eq!(
        shell_dashboard_dispatch_queue_lane_tab_panels
            .attention_panel_id
            .as_deref(),
        Some("dashboard.dispatch-queue-lane-tab-panel.attention")
    );
    assert_eq!(
        shell_dashboard_dispatch_queue_lane_tab_panels.panel_count,
        3
    );
    assert_eq!(
        shell_dashboard_dispatch_queue_lane_tab_panels.enabled_panel_count,
        2
    );
    assert_eq!(
        shell_dashboard_dispatch_queue_lane_tab_panels.disabled_panel_count,
        1
    );
    assert_eq!(
        shell_dashboard_dispatch_queue_lane_tab_panels.empty_panel_count,
        1
    );
    assert!(shell_dashboard_dispatch_queue_lane_tab_panels.panels[2].active);
    assert!(shell_dashboard_dispatch_queue_lane_tab_panels.panels[2].attention);
    assert!(shell_dashboard_dispatch_queue_lane_tab_panels.panels[1].disabled);
    assert!(shell_dashboard_dispatch_queue_lane_tab_panels.panels[1].empty);
    assert_eq!(
        shell_dashboard_dispatch_queue_lane_tab_panels.panels[1]
            .empty_message
            .as_deref(),
        Some("No blocked dispatches")
    );
    assert_eq!(
        shell_dashboard_dispatch_queue_lane_tab_panels.dispatch_queue_lane_tab_panels_capability_id,
        "app-shell-dashboard-dispatch-queue-lane-tab-panels-json"
    );

    let shell_dashboard_dispatch_queue_lane_tab_panels_payload: serde_json::Value =
        serde_json::from_str(
            &app.app_shell_dashboard_dispatch_queue_lane_tab_panels_json(
                BerkeleyAppPersistedEditorState {
                    selected_syntax_card_index: Some(2),
                    active_command_id: Some("analysis.2.run".to_string()),
                },
            ),
        )
        .expect("blocked shell dashboard dispatch queue lane tab panels JSON should parse");
    assert_eq!(
        shell_dashboard_dispatch_queue_lane_tab_panels_payload["activePanelId"],
        "dashboard.dispatch-queue-lane-tab-panel.attention"
    );
    assert_eq!(
        shell_dashboard_dispatch_queue_lane_tab_panels_payload["attentionPanelId"],
        "dashboard.dispatch-queue-lane-tab-panel.attention"
    );
    assert_eq!(
        shell_dashboard_dispatch_queue_lane_tab_panels_payload["panels"][1]["empty"],
        true
    );
    assert_eq!(
        shell_dashboard_dispatch_queue_lane_tab_panels_payload["panels"][2]["attention"],
        true
    );
    assert_eq!(
        shell_dashboard_dispatch_queue_lane_tab_panels_payload
            ["dispatchQueueLaneTabPanelsCapabilityId"],
        "app-shell-dashboard-dispatch-queue-lane-tab-panels-json"
    );

    let shell_dashboard_dispatch_queue_lane_tab_panel_cards = app
        .app_shell_dashboard_dispatch_queue_lane_tab_panel_cards(BerkeleyAppPersistedEditorState {
            selected_syntax_card_index: Some(2),
            active_command_id: Some("analysis.2.run".to_string()),
        });
    assert_eq!(
        shell_dashboard_dispatch_queue_lane_tab_panel_cards.schema_version,
        BERKELEY_APP_SHELL_DASHBOARD_DISPATCH_QUEUE_LANE_TAB_PANEL_CARDS_SCHEMA_VERSION
    );
    assert!(!shell_dashboard_dispatch_queue_lane_tab_panel_cards.ready);
    assert_eq!(
        shell_dashboard_dispatch_queue_lane_tab_panel_cards
            .active_panel_card_id
            .as_deref(),
        Some("dashboard.dispatch-queue-lane-tab-panel-card.attention")
    );
    assert_eq!(
        shell_dashboard_dispatch_queue_lane_tab_panel_cards
            .attention_panel_card_id
            .as_deref(),
        Some("dashboard.dispatch-queue-lane-tab-panel-card.attention")
    );
    assert_eq!(
        shell_dashboard_dispatch_queue_lane_tab_panel_cards.panel_card_count,
        3
    );
    assert_eq!(
        shell_dashboard_dispatch_queue_lane_tab_panel_cards.enabled_panel_card_count,
        2
    );
    assert_eq!(
        shell_dashboard_dispatch_queue_lane_tab_panel_cards.disabled_panel_card_count,
        1
    );
    assert_eq!(
        shell_dashboard_dispatch_queue_lane_tab_panel_cards.empty_panel_card_count,
        1
    );
    assert!(shell_dashboard_dispatch_queue_lane_tab_panel_cards.panel_cards[2].active);
    assert!(shell_dashboard_dispatch_queue_lane_tab_panel_cards.panel_cards[2].attention);
    assert!(shell_dashboard_dispatch_queue_lane_tab_panel_cards.panel_cards[1].empty);
    assert_eq!(
        shell_dashboard_dispatch_queue_lane_tab_panel_cards.panel_cards[1].summary,
        "No blocked dispatches"
    );
    assert_eq!(
        shell_dashboard_dispatch_queue_lane_tab_panel_cards.panel_cards[2].summary,
        "1 attention dispatch"
    );
    assert_eq!(
        shell_dashboard_dispatch_queue_lane_tab_panel_cards
            .dispatch_queue_lane_tab_panel_cards_capability_id,
        "app-shell-dashboard-dispatch-queue-lane-tab-panel-cards-json"
    );

    let shell_dashboard_dispatch_queue_lane_tab_panel_cards_payload: serde_json::Value =
        serde_json::from_str(
            &app.app_shell_dashboard_dispatch_queue_lane_tab_panel_cards_json(
                BerkeleyAppPersistedEditorState {
                    selected_syntax_card_index: Some(2),
                    active_command_id: Some("analysis.2.run".to_string()),
                },
            ),
        )
        .expect("blocked shell dashboard dispatch queue lane tab panel cards JSON should parse");
    assert_eq!(
        shell_dashboard_dispatch_queue_lane_tab_panel_cards_payload["activePanelCardId"],
        "dashboard.dispatch-queue-lane-tab-panel-card.attention"
    );
    assert_eq!(
        shell_dashboard_dispatch_queue_lane_tab_panel_cards_payload["attentionPanelCardId"],
        "dashboard.dispatch-queue-lane-tab-panel-card.attention"
    );
    assert_eq!(
        shell_dashboard_dispatch_queue_lane_tab_panel_cards_payload["panelCards"][1]["empty"],
        true
    );
    assert_eq!(
        shell_dashboard_dispatch_queue_lane_tab_panel_cards_payload["panelCards"][2]["summary"],
        "1 attention dispatch"
    );
    assert_eq!(
        shell_dashboard_dispatch_queue_lane_tab_panel_cards_payload
            ["dispatchQueueLaneTabPanelCardsCapabilityId"],
        "app-shell-dashboard-dispatch-queue-lane-tab-panel-cards-json"
    );

    let shell_dashboard_dispatch_queue_lane_tab_panel_card_actions = app
        .app_shell_dashboard_dispatch_queue_lane_tab_panel_card_actions(
            BerkeleyAppPersistedEditorState {
                selected_syntax_card_index: Some(2),
                active_command_id: Some("analysis.2.run".to_string()),
            },
        );
    assert_eq!(
        shell_dashboard_dispatch_queue_lane_tab_panel_card_actions.schema_version,
        BERKELEY_APP_SHELL_DASHBOARD_DISPATCH_QUEUE_LANE_TAB_PANEL_CARD_ACTIONS_SCHEMA_VERSION
    );
    assert!(!shell_dashboard_dispatch_queue_lane_tab_panel_card_actions.ready);
    assert_eq!(
        shell_dashboard_dispatch_queue_lane_tab_panel_card_actions
            .active_panel_card_action_id
            .as_deref(),
        Some("dashboard.dispatch-queue-lane-tab-panel-card-action.attention")
    );
    assert_eq!(
        shell_dashboard_dispatch_queue_lane_tab_panel_card_actions
            .attention_panel_card_action_id
            .as_deref(),
        Some("dashboard.dispatch-queue-lane-tab-panel-card-action.attention")
    );
    assert_eq!(
        shell_dashboard_dispatch_queue_lane_tab_panel_card_actions.panel_card_action_count,
        3
    );
    assert_eq!(
        shell_dashboard_dispatch_queue_lane_tab_panel_card_actions.enabled_panel_card_action_count,
        2
    );
    assert_eq!(
        shell_dashboard_dispatch_queue_lane_tab_panel_card_actions.disabled_panel_card_action_count,
        1
    );
    assert!(
        shell_dashboard_dispatch_queue_lane_tab_panel_card_actions.panel_card_actions[2].active
    );
    assert!(
        shell_dashboard_dispatch_queue_lane_tab_panel_card_actions.panel_card_actions[2].attention
    );
    assert_eq!(
        shell_dashboard_dispatch_queue_lane_tab_panel_card_actions.panel_card_actions[1].label,
        "View blocked dispatches"
    );
    assert_eq!(
        shell_dashboard_dispatch_queue_lane_tab_panel_card_actions.panel_card_actions[1]
            .disabled_reason
            .as_deref(),
        Some("No blocked dispatches")
    );
    assert_eq!(
        shell_dashboard_dispatch_queue_lane_tab_panel_card_actions.panel_card_actions[2].label,
        "Open attention dispatches"
    );
    assert_eq!(
        shell_dashboard_dispatch_queue_lane_tab_panel_card_actions
            .dispatch_queue_lane_tab_panel_card_actions_capability_id,
        "app-shell-dashboard-dispatch-queue-lane-tab-panel-card-actions-json"
    );

    let shell_dashboard_dispatch_queue_lane_tab_panel_card_actions_payload: serde_json::Value =
        serde_json::from_str(
            &app.app_shell_dashboard_dispatch_queue_lane_tab_panel_card_actions_json(
                BerkeleyAppPersistedEditorState {
                    selected_syntax_card_index: Some(2),
                    active_command_id: Some("analysis.2.run".to_string()),
                },
            ),
        )
        .expect(
            "blocked shell dashboard dispatch queue lane tab panel card actions JSON should parse",
        );
    assert_eq!(
        shell_dashboard_dispatch_queue_lane_tab_panel_card_actions_payload
            ["activePanelCardActionId"],
        "dashboard.dispatch-queue-lane-tab-panel-card-action.attention"
    );
    assert_eq!(
        shell_dashboard_dispatch_queue_lane_tab_panel_card_actions_payload
            ["attentionPanelCardActionId"],
        "dashboard.dispatch-queue-lane-tab-panel-card-action.attention"
    );
    assert_eq!(
        shell_dashboard_dispatch_queue_lane_tab_panel_card_actions_payload["panelCardActions"][1]
            ["disabledReason"],
        "No blocked dispatches"
    );
    assert_eq!(
        shell_dashboard_dispatch_queue_lane_tab_panel_card_actions_payload["panelCardActions"][2]
            ["label"],
        "Open attention dispatches"
    );
    assert_eq!(
        shell_dashboard_dispatch_queue_lane_tab_panel_card_actions_payload
            ["dispatchQueueLaneTabPanelCardActionsCapabilityId"],
        "app-shell-dashboard-dispatch-queue-lane-tab-panel-card-actions-json"
    );

    let shell_dashboard_dispatch_queue_lane_tab_panel_card_action_menu = app
        .app_shell_dashboard_dispatch_queue_lane_tab_panel_card_action_menu(
            BerkeleyAppPersistedEditorState {
                selected_syntax_card_index: Some(2),
                active_command_id: Some("analysis.2.run".to_string()),
            },
        );
    assert_eq!(
        shell_dashboard_dispatch_queue_lane_tab_panel_card_action_menu.schema_version,
        BERKELEY_APP_SHELL_DASHBOARD_DISPATCH_QUEUE_LANE_TAB_PANEL_CARD_ACTION_MENU_SCHEMA_VERSION
    );
    assert!(!shell_dashboard_dispatch_queue_lane_tab_panel_card_action_menu.ready);
    assert_eq!(
        shell_dashboard_dispatch_queue_lane_tab_panel_card_action_menu
            .active_menu_item_id
            .as_deref(),
        Some("dashboard.dispatch-queue-lane-tab-panel-card-action-menu-item.attention")
    );
    assert_eq!(
        shell_dashboard_dispatch_queue_lane_tab_panel_card_action_menu
            .attention_menu_item_id
            .as_deref(),
        Some("dashboard.dispatch-queue-lane-tab-panel-card-action-menu-item.attention")
    );
    assert_eq!(
        shell_dashboard_dispatch_queue_lane_tab_panel_card_action_menu.menu_item_count,
        3
    );
    assert_eq!(
        shell_dashboard_dispatch_queue_lane_tab_panel_card_action_menu.enabled_menu_item_count,
        2
    );
    assert_eq!(
        shell_dashboard_dispatch_queue_lane_tab_panel_card_action_menu.disabled_menu_item_count,
        1
    );
    assert!(shell_dashboard_dispatch_queue_lane_tab_panel_card_action_menu.menu_items[2].active);
    assert!(shell_dashboard_dispatch_queue_lane_tab_panel_card_action_menu.menu_items[2].attention);
    assert_eq!(
        shell_dashboard_dispatch_queue_lane_tab_panel_card_action_menu.menu_items[1].label,
        "View blocked dispatches"
    );
    assert_eq!(
        shell_dashboard_dispatch_queue_lane_tab_panel_card_action_menu.menu_items[1]
            .disabled_reason
            .as_deref(),
        Some("No blocked dispatches")
    );
    assert_eq!(
        shell_dashboard_dispatch_queue_lane_tab_panel_card_action_menu
            .dispatch_queue_lane_tab_panel_card_action_menu_capability_id,
        "app-shell-dashboard-dispatch-queue-lane-tab-panel-card-action-menu-json"
    );

    let shell_dashboard_dispatch_queue_lane_tab_panel_card_action_menu_payload: serde_json::Value =
        serde_json::from_str(
            &app.app_shell_dashboard_dispatch_queue_lane_tab_panel_card_action_menu_json(
                BerkeleyAppPersistedEditorState {
                    selected_syntax_card_index: Some(2),
                    active_command_id: Some("analysis.2.run".to_string()),
                },
            ),
        )
        .expect(
            "blocked shell dashboard dispatch queue lane tab panel card action menu JSON should parse",
        );
    assert_eq!(
        shell_dashboard_dispatch_queue_lane_tab_panel_card_action_menu_payload["activeMenuItemId"],
        "dashboard.dispatch-queue-lane-tab-panel-card-action-menu-item.attention"
    );
    assert_eq!(
        shell_dashboard_dispatch_queue_lane_tab_panel_card_action_menu_payload
            ["attentionMenuItemId"],
        "dashboard.dispatch-queue-lane-tab-panel-card-action-menu-item.attention"
    );
    assert_eq!(
        shell_dashboard_dispatch_queue_lane_tab_panel_card_action_menu_payload["menuItems"][1]
            ["disabledReason"],
        "No blocked dispatches"
    );
    assert_eq!(
        shell_dashboard_dispatch_queue_lane_tab_panel_card_action_menu_payload
            ["dispatchQueueLaneTabPanelCardActionMenuCapabilityId"],
        "app-shell-dashboard-dispatch-queue-lane-tab-panel-card-action-menu-json"
    );

    let shell_dashboard_dispatch_queue_lane_tab_panel_card_action_menu_groups = app
        .app_shell_dashboard_dispatch_queue_lane_tab_panel_card_action_menu_groups(
            BerkeleyAppPersistedEditorState {
                selected_syntax_card_index: Some(2),
                active_command_id: Some("analysis.2.run".to_string()),
            },
        );
    assert_eq!(
        shell_dashboard_dispatch_queue_lane_tab_panel_card_action_menu_groups.schema_version,
        BERKELEY_APP_SHELL_DASHBOARD_DISPATCH_QUEUE_LANE_TAB_PANEL_CARD_ACTION_MENU_GROUPS_SCHEMA_VERSION
    );
    assert!(!shell_dashboard_dispatch_queue_lane_tab_panel_card_action_menu_groups.ready);
    assert_eq!(
        shell_dashboard_dispatch_queue_lane_tab_panel_card_action_menu_groups
            .active_menu_group_id
            .as_deref(),
        Some("dashboard.dispatch-queue-lane-tab-panel-card-action-menu-group.attention")
    );
    assert_eq!(
        shell_dashboard_dispatch_queue_lane_tab_panel_card_action_menu_groups
            .attention_menu_group_id
            .as_deref(),
        Some("dashboard.dispatch-queue-lane-tab-panel-card-action-menu-group.attention")
    );
    assert_eq!(
        shell_dashboard_dispatch_queue_lane_tab_panel_card_action_menu_groups.menu_group_count,
        3
    );
    assert_eq!(
        shell_dashboard_dispatch_queue_lane_tab_panel_card_action_menu_groups
            .enabled_menu_group_count,
        2
    );
    assert_eq!(
        shell_dashboard_dispatch_queue_lane_tab_panel_card_action_menu_groups
            .disabled_menu_group_count,
        1
    );
    assert!(
        shell_dashboard_dispatch_queue_lane_tab_panel_card_action_menu_groups.menu_groups[2].active
    );
    assert!(
        shell_dashboard_dispatch_queue_lane_tab_panel_card_action_menu_groups.menu_groups[2]
            .attention
    );
    assert_eq!(
        shell_dashboard_dispatch_queue_lane_tab_panel_card_action_menu_groups.menu_groups[1].label,
        "Blocked dispatches"
    );
    assert_eq!(
        shell_dashboard_dispatch_queue_lane_tab_panel_card_action_menu_groups.menu_groups[1]
            .disabled_reason
            .as_deref(),
        Some("No blocked dispatches")
    );
    assert_eq!(
        shell_dashboard_dispatch_queue_lane_tab_panel_card_action_menu_groups
            .dispatch_queue_lane_tab_panel_card_action_menu_groups_capability_id,
        "app-shell-dashboard-dispatch-queue-lane-tab-panel-card-action-menu-groups-json"
    );

    let shell_dashboard_dispatch_queue_lane_tab_panel_card_action_menu_groups_payload:
        serde_json::Value = serde_json::from_str(
        &app.app_shell_dashboard_dispatch_queue_lane_tab_panel_card_action_menu_groups_json(
            BerkeleyAppPersistedEditorState {
                selected_syntax_card_index: Some(2),
                active_command_id: Some("analysis.2.run".to_string()),
            },
        ),
    )
    .expect(
        "blocked shell dashboard dispatch queue lane tab panel card action menu groups JSON should parse",
    );
    assert_eq!(
        shell_dashboard_dispatch_queue_lane_tab_panel_card_action_menu_groups_payload
            ["activeMenuGroupId"],
        "dashboard.dispatch-queue-lane-tab-panel-card-action-menu-group.attention"
    );
    assert_eq!(
        shell_dashboard_dispatch_queue_lane_tab_panel_card_action_menu_groups_payload
            ["attentionMenuGroupId"],
        "dashboard.dispatch-queue-lane-tab-panel-card-action-menu-group.attention"
    );
    assert_eq!(
        shell_dashboard_dispatch_queue_lane_tab_panel_card_action_menu_groups_payload["menuGroups"]
            [1]["disabledReason"],
        "No blocked dispatches"
    );
    assert_eq!(
        shell_dashboard_dispatch_queue_lane_tab_panel_card_action_menu_groups_payload
            ["dispatchQueueLaneTabPanelCardActionMenuGroupsCapabilityId"],
        "app-shell-dashboard-dispatch-queue-lane-tab-panel-card-action-menu-groups-json"
    );

    let shell_dashboard_dispatch_queue_lane_tab_panel_card_action_menu_group_shortcuts = app
        .app_shell_dashboard_dispatch_queue_lane_tab_panel_card_action_menu_group_shortcuts(
            BerkeleyAppPersistedEditorState {
                selected_syntax_card_index: Some(2),
                active_command_id: Some("analysis.2.run".to_string()),
            },
        );
    assert_eq!(
        shell_dashboard_dispatch_queue_lane_tab_panel_card_action_menu_group_shortcuts
            .schema_version,
        BERKELEY_APP_SHELL_DASHBOARD_DISPATCH_QUEUE_LANE_TAB_PANEL_CARD_ACTION_MENU_GROUP_SHORTCUTS_SCHEMA_VERSION
    );
    assert!(!shell_dashboard_dispatch_queue_lane_tab_panel_card_action_menu_group_shortcuts.ready);
    assert_eq!(
        shell_dashboard_dispatch_queue_lane_tab_panel_card_action_menu_group_shortcuts
            .active_menu_group_shortcut_id
            .as_deref(),
        Some("dashboard.dispatch-queue-lane-tab-panel-card-action-menu-group-shortcut.attention")
    );
    assert_eq!(
        shell_dashboard_dispatch_queue_lane_tab_panel_card_action_menu_group_shortcuts
            .attention_menu_group_shortcut_id
            .as_deref(),
        Some("dashboard.dispatch-queue-lane-tab-panel-card-action-menu-group-shortcut.attention")
    );
    assert_eq!(
        shell_dashboard_dispatch_queue_lane_tab_panel_card_action_menu_group_shortcuts
            .menu_group_shortcut_count,
        3
    );
    assert_eq!(
        shell_dashboard_dispatch_queue_lane_tab_panel_card_action_menu_group_shortcuts
            .enabled_menu_group_shortcut_count,
        2
    );
    assert!(
        shell_dashboard_dispatch_queue_lane_tab_panel_card_action_menu_group_shortcuts
            .menu_group_shortcuts[2]
            .active
    );
    assert_eq!(
        shell_dashboard_dispatch_queue_lane_tab_panel_card_action_menu_group_shortcuts
            .menu_group_shortcuts[2]
            .accelerator,
        "mod+3"
    );
    assert_eq!(
        shell_dashboard_dispatch_queue_lane_tab_panel_card_action_menu_group_shortcuts
            .menu_group_shortcuts[1]
            .disabled_reason
            .as_deref(),
        Some("No blocked dispatches")
    );
    assert_eq!(
        shell_dashboard_dispatch_queue_lane_tab_panel_card_action_menu_group_shortcuts
            .dispatch_queue_lane_tab_panel_card_action_menu_group_shortcuts_capability_id,
        "app-shell-dashboard-dispatch-queue-lane-tab-panel-card-action-menu-group-shortcuts-json"
    );

    let shell_dashboard_dispatch_queue_lane_tab_panel_card_action_menu_group_shortcuts_payload:
        serde_json::Value = serde_json::from_str(
        &app.app_shell_dashboard_dispatch_queue_lane_tab_panel_card_action_menu_group_shortcuts_json(
            BerkeleyAppPersistedEditorState {
                selected_syntax_card_index: Some(2),
                active_command_id: Some("analysis.2.run".to_string()),
            },
        ),
    )
    .expect(
        "blocked shell dashboard dispatch queue lane tab panel card action menu group shortcuts JSON should parse",
    );
    assert_eq!(
        shell_dashboard_dispatch_queue_lane_tab_panel_card_action_menu_group_shortcuts_payload
            ["activeMenuGroupShortcutId"],
        "dashboard.dispatch-queue-lane-tab-panel-card-action-menu-group-shortcut.attention"
    );
    assert_eq!(
        shell_dashboard_dispatch_queue_lane_tab_panel_card_action_menu_group_shortcuts_payload
            ["menuGroupShortcuts"][2]["accelerator"],
        "mod+3"
    );
    assert_eq!(
        shell_dashboard_dispatch_queue_lane_tab_panel_card_action_menu_group_shortcuts_payload
            ["menuGroupShortcuts"][1]["disabledReason"],
        "No blocked dispatches"
    );
    assert_eq!(
        shell_dashboard_dispatch_queue_lane_tab_panel_card_action_menu_group_shortcuts_payload
            ["dispatchQueueLaneTabPanelCardActionMenuGroupShortcutsCapabilityId"],
        "app-shell-dashboard-dispatch-queue-lane-tab-panel-card-action-menu-group-shortcuts-json"
    );

    let shell_dashboard_dispatch_queue_lane_tab_panel_card_action_menu_group_shortcut_bindings =
        app.app_shell_dashboard_dispatch_queue_lane_tab_panel_card_action_menu_group_shortcut_bindings(
            BerkeleyAppPersistedEditorState {
                selected_syntax_card_index: Some(2),
                active_command_id: Some("analysis.2.run".to_string()),
            },
        );
    assert_eq!(
        shell_dashboard_dispatch_queue_lane_tab_panel_card_action_menu_group_shortcut_bindings
            .schema_version,
        BERKELEY_APP_SHELL_DASHBOARD_DISPATCH_QUEUE_LANE_TAB_PANEL_CARD_ACTION_MENU_GROUP_SHORTCUT_BINDINGS_SCHEMA_VERSION
    );
    assert!(
        !shell_dashboard_dispatch_queue_lane_tab_panel_card_action_menu_group_shortcut_bindings
            .ready
    );
    assert_eq!(
        shell_dashboard_dispatch_queue_lane_tab_panel_card_action_menu_group_shortcut_bindings
            .active_menu_group_shortcut_binding_id
            .as_deref(),
        Some(
            "dashboard.dispatch-queue-lane-tab-panel-card-action-menu-group-shortcut-binding.attention"
        )
    );
    assert_eq!(
        shell_dashboard_dispatch_queue_lane_tab_panel_card_action_menu_group_shortcut_bindings
            .attention_menu_group_shortcut_binding_id
            .as_deref(),
        Some(
            "dashboard.dispatch-queue-lane-tab-panel-card-action-menu-group-shortcut-binding.attention"
        )
    );
    assert_eq!(
        shell_dashboard_dispatch_queue_lane_tab_panel_card_action_menu_group_shortcut_bindings
            .menu_group_shortcut_binding_count,
        3
    );
    assert_eq!(
        shell_dashboard_dispatch_queue_lane_tab_panel_card_action_menu_group_shortcut_bindings
            .enabled_menu_group_shortcut_binding_count,
        2
    );
    assert!(
        shell_dashboard_dispatch_queue_lane_tab_panel_card_action_menu_group_shortcut_bindings
            .menu_group_shortcut_bindings[2]
            .active
    );
    assert_eq!(
        shell_dashboard_dispatch_queue_lane_tab_panel_card_action_menu_group_shortcut_bindings
            .menu_group_shortcut_bindings[2]
            .accelerator,
        "mod+3"
    );
    assert_eq!(
        shell_dashboard_dispatch_queue_lane_tab_panel_card_action_menu_group_shortcut_bindings
            .menu_group_shortcut_bindings[2]
            .command_id,
        "berkeley.app-shell.dashboard.dispatch-queue.menu-group-shortcut.attention"
    );
    assert_eq!(
        shell_dashboard_dispatch_queue_lane_tab_panel_card_action_menu_group_shortcut_bindings
            .menu_group_shortcut_bindings[1]
            .disabled_reason
            .as_deref(),
        Some("No blocked dispatches")
    );
    assert_eq!(
        shell_dashboard_dispatch_queue_lane_tab_panel_card_action_menu_group_shortcut_bindings
            .dispatch_queue_lane_tab_panel_card_action_menu_group_shortcut_bindings_capability_id,
        "app-shell-dashboard-dispatch-queue-lane-tab-panel-card-action-menu-group-shortcut-bindings-json"
    );

    let shell_dashboard_dispatch_queue_lane_tab_panel_card_action_menu_group_shortcut_bindings_payload:
        serde_json::Value = serde_json::from_str(
        &app.app_shell_dashboard_dispatch_queue_lane_tab_panel_card_action_menu_group_shortcut_bindings_json(
            BerkeleyAppPersistedEditorState {
                selected_syntax_card_index: Some(2),
                active_command_id: Some("analysis.2.run".to_string()),
            },
        ),
    )
    .expect(
        "blocked shell dashboard dispatch queue lane tab panel card action menu group shortcut bindings JSON should parse",
    );
    assert_eq!(
        shell_dashboard_dispatch_queue_lane_tab_panel_card_action_menu_group_shortcut_bindings_payload
            ["activeMenuGroupShortcutBindingId"],
        "dashboard.dispatch-queue-lane-tab-panel-card-action-menu-group-shortcut-binding.attention"
    );
    assert_eq!(
        shell_dashboard_dispatch_queue_lane_tab_panel_card_action_menu_group_shortcut_bindings_payload
            ["menuGroupShortcutBindings"][2]["commandId"],
        "berkeley.app-shell.dashboard.dispatch-queue.menu-group-shortcut.attention"
    );
    assert_eq!(
        shell_dashboard_dispatch_queue_lane_tab_panel_card_action_menu_group_shortcut_bindings_payload
            ["menuGroupShortcutBindings"][1]["disabledReason"],
        "No blocked dispatches"
    );
    assert_eq!(
        shell_dashboard_dispatch_queue_lane_tab_panel_card_action_menu_group_shortcut_bindings_payload
            ["dispatchQueueLaneTabPanelCardActionMenuGroupShortcutBindingsCapabilityId"],
        "app-shell-dashboard-dispatch-queue-lane-tab-panel-card-action-menu-group-shortcut-bindings-json"
    );

    let shell_dashboard_dispatch_queue_lane_tab_panel_card_action_menu_group_shortcut_command_registry =
        app.app_shell_dashboard_dispatch_queue_lane_tab_panel_card_action_menu_group_shortcut_command_registry(
            BerkeleyAppPersistedEditorState {
                selected_syntax_card_index: Some(2),
                active_command_id: Some("analysis.2.run".to_string()),
            },
        );
    assert_eq!(
        shell_dashboard_dispatch_queue_lane_tab_panel_card_action_menu_group_shortcut_command_registry
            .schema_version,
        BERKELEY_APP_SHELL_DASHBOARD_DISPATCH_QUEUE_LANE_TAB_PANEL_CARD_ACTION_MENU_GROUP_SHORTCUT_COMMAND_REGISTRY_SCHEMA_VERSION
    );
    assert!(
        !shell_dashboard_dispatch_queue_lane_tab_panel_card_action_menu_group_shortcut_command_registry
            .ready
    );
    assert_eq!(
        shell_dashboard_dispatch_queue_lane_tab_panel_card_action_menu_group_shortcut_command_registry
            .active_menu_group_shortcut_command_id
            .as_deref(),
        Some("dashboard.dispatch-queue-lane-tab-panel-card-action-menu-group-shortcut-command.attention")
    );
    assert_eq!(
        shell_dashboard_dispatch_queue_lane_tab_panel_card_action_menu_group_shortcut_command_registry
            .attention_menu_group_shortcut_command_id
            .as_deref(),
        Some("dashboard.dispatch-queue-lane-tab-panel-card-action-menu-group-shortcut-command.attention")
    );
    assert_eq!(
        shell_dashboard_dispatch_queue_lane_tab_panel_card_action_menu_group_shortcut_command_registry
            .menu_group_shortcut_command_count,
        3
    );
    assert_eq!(
        shell_dashboard_dispatch_queue_lane_tab_panel_card_action_menu_group_shortcut_command_registry
            .enabled_menu_group_shortcut_command_count,
        2
    );
    assert!(
        shell_dashboard_dispatch_queue_lane_tab_panel_card_action_menu_group_shortcut_command_registry
            .menu_group_shortcut_commands[2]
            .active
    );
    assert_eq!(
        shell_dashboard_dispatch_queue_lane_tab_panel_card_action_menu_group_shortcut_command_registry
            .menu_group_shortcut_commands[2]
            .handler_id,
        "handler.berkeley.app-shell.dashboard.dispatch-queue.menu-group-shortcut.attention"
    );
    assert_eq!(
        shell_dashboard_dispatch_queue_lane_tab_panel_card_action_menu_group_shortcut_command_registry
            .menu_group_shortcut_commands[1]
            .disabled_reason
            .as_deref(),
        Some("No blocked dispatches")
    );
    assert_eq!(
        shell_dashboard_dispatch_queue_lane_tab_panel_card_action_menu_group_shortcut_command_registry
            .dispatch_queue_lane_tab_panel_card_action_menu_group_shortcut_command_registry_capability_id,
        "app-shell-dashboard-dispatch-queue-lane-tab-panel-card-action-menu-group-shortcut-command-registry-json"
    );

    let shell_dashboard_dispatch_queue_lane_tab_panel_card_action_menu_group_shortcut_command_registry_payload:
        serde_json::Value = serde_json::from_str(
        &app.app_shell_dashboard_dispatch_queue_lane_tab_panel_card_action_menu_group_shortcut_command_registry_json(
            BerkeleyAppPersistedEditorState {
                selected_syntax_card_index: Some(2),
                active_command_id: Some("analysis.2.run".to_string()),
            },
        ),
    )
    .expect(
        "blocked shell dashboard dispatch queue lane tab panel card action menu group shortcut command registry JSON should parse",
    );
    assert_eq!(
        shell_dashboard_dispatch_queue_lane_tab_panel_card_action_menu_group_shortcut_command_registry_payload
            ["activeMenuGroupShortcutCommandId"],
        "dashboard.dispatch-queue-lane-tab-panel-card-action-menu-group-shortcut-command.attention"
    );
    assert_eq!(
        shell_dashboard_dispatch_queue_lane_tab_panel_card_action_menu_group_shortcut_command_registry_payload
            ["menuGroupShortcutCommands"][2]["handlerId"],
        "handler.berkeley.app-shell.dashboard.dispatch-queue.menu-group-shortcut.attention"
    );
    assert_eq!(
        shell_dashboard_dispatch_queue_lane_tab_panel_card_action_menu_group_shortcut_command_registry_payload
            ["menuGroupShortcutCommands"][1]["disabledReason"],
        "No blocked dispatches"
    );
    assert_eq!(
        shell_dashboard_dispatch_queue_lane_tab_panel_card_action_menu_group_shortcut_command_registry_payload
            ["dispatchQueueLaneTabPanelCardActionMenuGroupShortcutCommandRegistryCapabilityId"],
        "app-shell-dashboard-dispatch-queue-lane-tab-panel-card-action-menu-group-shortcut-command-registry-json"
    );

    let shell_dashboard_dispatch_queue_lane_tab_panel_card_action_menu_group_shortcut_command_palette =
        app.app_shell_dashboard_dispatch_queue_lane_tab_panel_card_action_menu_group_shortcut_command_palette(
            BerkeleyAppPersistedEditorState {
                selected_syntax_card_index: Some(2),
                active_command_id: Some("analysis.2.run".to_string()),
            },
        );
    assert_eq!(
        shell_dashboard_dispatch_queue_lane_tab_panel_card_action_menu_group_shortcut_command_palette
            .schema_version,
        BERKELEY_APP_SHELL_DASHBOARD_DISPATCH_QUEUE_LANE_TAB_PANEL_CARD_ACTION_MENU_GROUP_SHORTCUT_COMMAND_PALETTE_SCHEMA_VERSION
    );
    assert!(
        !shell_dashboard_dispatch_queue_lane_tab_panel_card_action_menu_group_shortcut_command_palette
            .ready
    );
    assert_eq!(
        shell_dashboard_dispatch_queue_lane_tab_panel_card_action_menu_group_shortcut_command_palette
            .active_menu_group_shortcut_command_palette_item_id
            .as_deref(),
        Some("dashboard.dispatch-queue-lane-tab-panel-card-action-menu-group-shortcut-command-palette-item.attention")
    );
    assert_eq!(
        shell_dashboard_dispatch_queue_lane_tab_panel_card_action_menu_group_shortcut_command_palette
            .attention_menu_group_shortcut_command_palette_item_id
            .as_deref(),
        Some("dashboard.dispatch-queue-lane-tab-panel-card-action-menu-group-shortcut-command-palette-item.attention")
    );
    assert_eq!(
        shell_dashboard_dispatch_queue_lane_tab_panel_card_action_menu_group_shortcut_command_palette
            .command_palette_item_count,
        3
    );
    assert_eq!(
        shell_dashboard_dispatch_queue_lane_tab_panel_card_action_menu_group_shortcut_command_palette
            .selectable_command_palette_item_count,
        2
    );
    assert!(
        shell_dashboard_dispatch_queue_lane_tab_panel_card_action_menu_group_shortcut_command_palette
            .command_palette_items[2]
            .active
    );
    assert!(
        shell_dashboard_dispatch_queue_lane_tab_panel_card_action_menu_group_shortcut_command_palette
            .command_palette_items[2]
            .selectable
    );
    assert_eq!(
        shell_dashboard_dispatch_queue_lane_tab_panel_card_action_menu_group_shortcut_command_palette
            .command_palette_items[2]
            .handler_id,
        "handler.berkeley.app-shell.dashboard.dispatch-queue.menu-group-shortcut.attention"
    );
    assert_eq!(
        shell_dashboard_dispatch_queue_lane_tab_panel_card_action_menu_group_shortcut_command_palette
            .command_palette_items[2]
            .rank,
        2
    );
    assert_eq!(
        shell_dashboard_dispatch_queue_lane_tab_panel_card_action_menu_group_shortcut_command_palette
            .command_palette_items[1]
            .disabled_reason
            .as_deref(),
        Some("No blocked dispatches")
    );
    assert_eq!(
        shell_dashboard_dispatch_queue_lane_tab_panel_card_action_menu_group_shortcut_command_palette
            .dispatch_queue_lane_tab_panel_card_action_menu_group_shortcut_command_palette_capability_id,
        "app-shell-dashboard-dispatch-queue-lane-tab-panel-card-action-menu-group-shortcut-command-palette-json"
    );

    let shell_dashboard_dispatch_queue_lane_tab_panel_card_action_menu_group_shortcut_command_palette_payload:
        serde_json::Value = serde_json::from_str(
        &app.app_shell_dashboard_dispatch_queue_lane_tab_panel_card_action_menu_group_shortcut_command_palette_json(
            BerkeleyAppPersistedEditorState {
                selected_syntax_card_index: Some(2),
                active_command_id: Some("analysis.2.run".to_string()),
            },
        ),
    )
    .expect(
        "blocked shell dashboard dispatch queue lane tab panel card action menu group shortcut command palette JSON should parse",
    );
    assert_eq!(
        shell_dashboard_dispatch_queue_lane_tab_panel_card_action_menu_group_shortcut_command_palette_payload
            ["activeMenuGroupShortcutCommandPaletteItemId"],
        "dashboard.dispatch-queue-lane-tab-panel-card-action-menu-group-shortcut-command-palette-item.attention"
    );
    assert_eq!(
        shell_dashboard_dispatch_queue_lane_tab_panel_card_action_menu_group_shortcut_command_palette_payload
            ["commandPaletteItems"][2]["handlerId"],
        "handler.berkeley.app-shell.dashboard.dispatch-queue.menu-group-shortcut.attention"
    );
    assert_eq!(
        shell_dashboard_dispatch_queue_lane_tab_panel_card_action_menu_group_shortcut_command_palette_payload
            ["commandPaletteItems"][2]["rank"],
        2
    );
    assert_eq!(
        shell_dashboard_dispatch_queue_lane_tab_panel_card_action_menu_group_shortcut_command_palette_payload
            ["commandPaletteItems"][1]["disabledReason"],
        "No blocked dispatches"
    );
    assert_eq!(
        shell_dashboard_dispatch_queue_lane_tab_panel_card_action_menu_group_shortcut_command_palette_payload
            ["dispatchQueueLaneTabPanelCardActionMenuGroupShortcutCommandPaletteCapabilityId"],
        "app-shell-dashboard-dispatch-queue-lane-tab-panel-card-action-menu-group-shortcut-command-palette-json"
    );

    let shell_dashboard_dispatch_queue_lane_tab_panel_card_action_menu_group_shortcut_command_palette_search_index =
        app.app_shell_dashboard_dispatch_queue_lane_tab_panel_card_action_menu_group_shortcut_command_palette_search_index(
            BerkeleyAppPersistedEditorState {
                selected_syntax_card_index: Some(2),
                active_command_id: Some("analysis.2.run".to_string()),
            },
        );
    assert_eq!(
        shell_dashboard_dispatch_queue_lane_tab_panel_card_action_menu_group_shortcut_command_palette_search_index
            .schema_version,
        BERKELEY_APP_SHELL_DASHBOARD_DISPATCH_QUEUE_LANE_TAB_PANEL_CARD_ACTION_MENU_GROUP_SHORTCUT_COMMAND_PALETTE_SEARCH_INDEX_SCHEMA_VERSION
    );
    assert!(
        !shell_dashboard_dispatch_queue_lane_tab_panel_card_action_menu_group_shortcut_command_palette_search_index
            .ready
    );
    assert_eq!(
        shell_dashboard_dispatch_queue_lane_tab_panel_card_action_menu_group_shortcut_command_palette_search_index
            .active_menu_group_shortcut_command_search_index_entry_id
            .as_deref(),
        Some("dashboard.dispatch-queue-lane-tab-panel-card-action-menu-group-shortcut-command-palette-search-index-entry.attention")
    );
    assert_eq!(
        shell_dashboard_dispatch_queue_lane_tab_panel_card_action_menu_group_shortcut_command_palette_search_index
            .attention_menu_group_shortcut_command_search_index_entry_id
            .as_deref(),
        Some("dashboard.dispatch-queue-lane-tab-panel-card-action-menu-group-shortcut-command-palette-search-index-entry.attention")
    );
    assert_eq!(
        shell_dashboard_dispatch_queue_lane_tab_panel_card_action_menu_group_shortcut_command_palette_search_index
            .search_index_entry_count,
        3
    );
    assert_eq!(
        shell_dashboard_dispatch_queue_lane_tab_panel_card_action_menu_group_shortcut_command_palette_search_index
            .selectable_search_index_entry_count,
        2
    );
    assert!(
        shell_dashboard_dispatch_queue_lane_tab_panel_card_action_menu_group_shortcut_command_palette_search_index
            .search_index_entries[2]
            .active
    );
    assert!(
        shell_dashboard_dispatch_queue_lane_tab_panel_card_action_menu_group_shortcut_command_palette_search_index
            .search_index_entries[2]
            .selectable
    );
    assert!(
        shell_dashboard_dispatch_queue_lane_tab_panel_card_action_menu_group_shortcut_command_palette_search_index
            .search_index_entries[2]
            .search_tokens
            .iter()
            .any(|token| token == "attention")
    );
    assert_eq!(
        shell_dashboard_dispatch_queue_lane_tab_panel_card_action_menu_group_shortcut_command_palette_search_index
            .search_index_entries[1]
            .disabled_reason
            .as_deref(),
        Some("No blocked dispatches")
    );
    assert_eq!(
        shell_dashboard_dispatch_queue_lane_tab_panel_card_action_menu_group_shortcut_command_palette_search_index
            .dispatch_queue_lane_tab_panel_card_action_menu_group_shortcut_command_palette_search_index_capability_id,
        "app-shell-dashboard-dispatch-queue-lane-tab-panel-card-action-menu-group-shortcut-command-palette-search-index-json"
    );

    let shell_dashboard_dispatch_queue_lane_tab_panel_card_action_menu_group_shortcut_command_palette_search_index_payload:
        serde_json::Value = serde_json::from_str(
        &app.app_shell_dashboard_dispatch_queue_lane_tab_panel_card_action_menu_group_shortcut_command_palette_search_index_json(
            BerkeleyAppPersistedEditorState {
                selected_syntax_card_index: Some(2),
                active_command_id: Some("analysis.2.run".to_string()),
            },
        ),
    )
    .expect(
        "blocked shell dashboard dispatch queue lane tab panel card action menu group shortcut command palette search index JSON should parse",
    );
    assert_eq!(
        shell_dashboard_dispatch_queue_lane_tab_panel_card_action_menu_group_shortcut_command_palette_search_index_payload
            ["activeMenuGroupShortcutCommandSearchIndexEntryId"],
        "dashboard.dispatch-queue-lane-tab-panel-card-action-menu-group-shortcut-command-palette-search-index-entry.attention"
    );
    assert_eq!(
        shell_dashboard_dispatch_queue_lane_tab_panel_card_action_menu_group_shortcut_command_palette_search_index_payload
            ["searchIndexEntries"][2]["searchTokens"][0],
        "attention"
    );
    assert_eq!(
        shell_dashboard_dispatch_queue_lane_tab_panel_card_action_menu_group_shortcut_command_palette_search_index_payload
            ["searchIndexEntries"][1]["disabledReason"],
        "No blocked dispatches"
    );
    assert_eq!(
        shell_dashboard_dispatch_queue_lane_tab_panel_card_action_menu_group_shortcut_command_palette_search_index_payload
            ["dispatchQueueLaneTabPanelCardActionMenuGroupShortcutCommandPaletteSearchIndexCapabilityId"],
        "app-shell-dashboard-dispatch-queue-lane-tab-panel-card-action-menu-group-shortcut-command-palette-search-index-json"
    );

    let shell_dashboard_dispatch_queue_lane_tab_panel_card_action_menu_group_shortcut_command_palette_search_results =
        app.app_shell_dashboard_dispatch_queue_lane_tab_panel_card_action_menu_group_shortcut_command_palette_search_results(
            BerkeleyAppPersistedEditorState {
                selected_syntax_card_index: Some(2),
                active_command_id: Some("analysis.2.run".to_string()),
            },
            "attention",
        );
    assert_eq!(
        shell_dashboard_dispatch_queue_lane_tab_panel_card_action_menu_group_shortcut_command_palette_search_results
            .schema_version,
        BERKELEY_APP_SHELL_DASHBOARD_DISPATCH_QUEUE_LANE_TAB_PANEL_CARD_ACTION_MENU_GROUP_SHORTCUT_COMMAND_PALETTE_SEARCH_RESULTS_SCHEMA_VERSION
    );
    assert!(!shell_dashboard_dispatch_queue_lane_tab_panel_card_action_menu_group_shortcut_command_palette_search_results.ready);
    assert_eq!(
        shell_dashboard_dispatch_queue_lane_tab_panel_card_action_menu_group_shortcut_command_palette_search_results
            .search_result_count,
        1
    );
    assert_eq!(
        shell_dashboard_dispatch_queue_lane_tab_panel_card_action_menu_group_shortcut_command_palette_search_results
            .active_menu_group_shortcut_command_search_result_id
            .as_deref(),
        Some("dashboard.dispatch-queue-lane-tab-panel-card-action-menu-group-shortcut-command-palette-search-result.attention")
    );
    assert_eq!(
        shell_dashboard_dispatch_queue_lane_tab_panel_card_action_menu_group_shortcut_command_palette_search_results
            .attention_menu_group_shortcut_command_search_result_id
            .as_deref(),
        Some("dashboard.dispatch-queue-lane-tab-panel-card-action-menu-group-shortcut-command-palette-search-result.attention")
    );
    assert!(
        shell_dashboard_dispatch_queue_lane_tab_panel_card_action_menu_group_shortcut_command_palette_search_results
            .search_results[0]
            .attention
    );
    assert_eq!(
        shell_dashboard_dispatch_queue_lane_tab_panel_card_action_menu_group_shortcut_command_palette_search_results
            .search_results[0]
            .matched_query_tokens,
        vec!["attention".to_string()]
    );

    let shell_dashboard_dispatch_queue_lane_tab_panel_card_action_menu_group_shortcut_command_palette_empty_search_results =
        app.app_shell_dashboard_dispatch_queue_lane_tab_panel_card_action_menu_group_shortcut_command_palette_search_results(
            BerkeleyAppPersistedEditorState {
                selected_syntax_card_index: Some(2),
                active_command_id: Some("analysis.2.run".to_string()),
            },
            "missing",
        );
    assert_eq!(
        shell_dashboard_dispatch_queue_lane_tab_panel_card_action_menu_group_shortcut_command_palette_empty_search_results
            .search_result_count,
        0
    );
    assert!(shell_dashboard_dispatch_queue_lane_tab_panel_card_action_menu_group_shortcut_command_palette_empty_search_results.no_results);
    assert_eq!(
        shell_dashboard_dispatch_queue_lane_tab_panel_card_action_menu_group_shortcut_command_palette_empty_search_results
            .active_menu_group_shortcut_command_search_result_id,
        None
    );
    assert_eq!(
        shell_dashboard_dispatch_queue_lane_tab_panel_card_action_menu_group_shortcut_command_palette_empty_search_results
            .empty_state_title
            .as_deref(),
        Some("No command matches")
    );
    assert_eq!(
        shell_dashboard_dispatch_queue_lane_tab_panel_card_action_menu_group_shortcut_command_palette_empty_search_results
            .empty_state_message
            .as_deref(),
        Some("No command palette entries match \"missing\".")
    );

    let shell_dashboard_dispatch_queue_lane_tab_panel_card_action_menu_group_shortcut_command_palette_empty_search_results_payload:
        serde_json::Value = serde_json::from_str(
        &app.app_shell_dashboard_dispatch_queue_lane_tab_panel_card_action_menu_group_shortcut_command_palette_search_results_json(
            BerkeleyAppPersistedEditorState {
                selected_syntax_card_index: Some(2),
                active_command_id: Some("analysis.2.run".to_string()),
            },
            "missing",
        ),
    )
    .expect(
        "blocked shell dashboard dispatch queue lane tab panel card action menu group shortcut command palette empty search results JSON should parse",
    );
    assert_eq!(
        shell_dashboard_dispatch_queue_lane_tab_panel_card_action_menu_group_shortcut_command_palette_empty_search_results_payload
            ["noResults"],
        true
    );
    assert_eq!(
        shell_dashboard_dispatch_queue_lane_tab_panel_card_action_menu_group_shortcut_command_palette_empty_search_results_payload
            ["searchResultCount"],
        0
    );
    assert_eq!(
        shell_dashboard_dispatch_queue_lane_tab_panel_card_action_menu_group_shortcut_command_palette_empty_search_results_payload
            ["emptyStateMessage"],
        "No command palette entries match \"missing\"."
    );
    assert_eq!(
        shell_dashboard_dispatch_queue_lane_tab_panel_card_action_menu_group_shortcut_command_palette_empty_search_results_payload
            ["dispatchQueueLaneTabPanelCardActionMenuGroupShortcutCommandPaletteSearchResultsCapabilityId"],
        "app-shell-dashboard-dispatch-queue-lane-tab-panel-card-action-menu-group-shortcut-command-palette-search-results-json"
    );

    let shell_dashboard_dispatch_queue_lane_tab_panel_card_action_menu_group_shortcut_command_palette_attention_search_selection =
        app.app_shell_dashboard_dispatch_queue_lane_tab_panel_card_action_menu_group_shortcut_command_palette_search_selection(
            BerkeleyAppPersistedEditorState {
                selected_syntax_card_index: Some(2),
                active_command_id: Some("analysis.2.run".to_string()),
            },
            "attention",
            None,
        );
    assert_eq!(
        shell_dashboard_dispatch_queue_lane_tab_panel_card_action_menu_group_shortcut_command_palette_attention_search_selection
            .schema_version,
        BERKELEY_APP_SHELL_DASHBOARD_DISPATCH_QUEUE_LANE_TAB_PANEL_CARD_ACTION_MENU_GROUP_SHORTCUT_COMMAND_PALETTE_SEARCH_SELECTION_SCHEMA_VERSION
    );
    assert!(!shell_dashboard_dispatch_queue_lane_tab_panel_card_action_menu_group_shortcut_command_palette_attention_search_selection.ready);
    assert_eq!(
        shell_dashboard_dispatch_queue_lane_tab_panel_card_action_menu_group_shortcut_command_palette_attention_search_selection
            .selection_source,
        "active"
    );
    assert_eq!(
        shell_dashboard_dispatch_queue_lane_tab_panel_card_action_menu_group_shortcut_command_palette_attention_search_selection
            .selected_search_result_id
            .as_deref(),
        Some("dashboard.dispatch-queue-lane-tab-panel-card-action-menu-group-shortcut-command-palette-search-result.attention")
    );
    assert!(
        shell_dashboard_dispatch_queue_lane_tab_panel_card_action_menu_group_shortcut_command_palette_attention_search_selection
            .selection_attention
    );
    assert!(
        shell_dashboard_dispatch_queue_lane_tab_panel_card_action_menu_group_shortcut_command_palette_attention_search_selection
            .can_invoke
    );

    let shell_dashboard_dispatch_queue_lane_tab_panel_card_action_menu_group_shortcut_command_palette_disabled_search_selection =
        app.app_shell_dashboard_dispatch_queue_lane_tab_panel_card_action_menu_group_shortcut_command_palette_search_selection(
            BerkeleyAppPersistedEditorState {
                selected_syntax_card_index: Some(2),
                active_command_id: Some("analysis.2.run".to_string()),
            },
            "blocked",
            Some("dashboard.dispatch-queue-lane-tab-panel-card-action-menu-group-shortcut-command-palette-search-result.blocked".to_string()),
        );
    assert_eq!(
        shell_dashboard_dispatch_queue_lane_tab_panel_card_action_menu_group_shortcut_command_palette_disabled_search_selection
            .selection_source,
        "requested"
    );
    assert_eq!(
        shell_dashboard_dispatch_queue_lane_tab_panel_card_action_menu_group_shortcut_command_palette_disabled_search_selection
            .selection_state,
        "disabled"
    );
    assert!(
        !shell_dashboard_dispatch_queue_lane_tab_panel_card_action_menu_group_shortcut_command_palette_disabled_search_selection
            .can_invoke
    );
    assert_eq!(
        shell_dashboard_dispatch_queue_lane_tab_panel_card_action_menu_group_shortcut_command_palette_disabled_search_selection
            .blocked_reason
            .as_deref(),
        Some("No blocked dispatches")
    );

    let shell_dashboard_dispatch_queue_lane_tab_panel_card_action_menu_group_shortcut_command_palette_empty_search_selection_payload:
        serde_json::Value = serde_json::from_str(
        &app.app_shell_dashboard_dispatch_queue_lane_tab_panel_card_action_menu_group_shortcut_command_palette_search_selection_json(
            BerkeleyAppPersistedEditorState {
                selected_syntax_card_index: Some(2),
                active_command_id: Some("analysis.2.run".to_string()),
            },
            "missing",
            None,
        ),
    )
    .expect(
        "blocked shell dashboard dispatch queue lane tab panel card action menu group shortcut command palette empty search selection JSON should parse",
    );
    assert_eq!(
        shell_dashboard_dispatch_queue_lane_tab_panel_card_action_menu_group_shortcut_command_palette_empty_search_selection_payload
            ["selectionSource"],
        "none"
    );
    assert_eq!(
        shell_dashboard_dispatch_queue_lane_tab_panel_card_action_menu_group_shortcut_command_palette_empty_search_selection_payload
            ["canInvoke"],
        false
    );
    assert_eq!(
        shell_dashboard_dispatch_queue_lane_tab_panel_card_action_menu_group_shortcut_command_palette_empty_search_selection_payload
            ["blockedReason"],
        "No command palette entries match \"missing\"."
    );
    assert_eq!(
        shell_dashboard_dispatch_queue_lane_tab_panel_card_action_menu_group_shortcut_command_palette_empty_search_selection_payload
            ["dispatchQueueLaneTabPanelCardActionMenuGroupShortcutCommandPaletteSearchSelectionCapabilityId"],
        "app-shell-dashboard-dispatch-queue-lane-tab-panel-card-action-menu-group-shortcut-command-palette-search-selection-json"
    );

    let shell_dashboard_dispatch_queue_lane_tab_panel_card_action_menu_group_shortcut_command_palette_attention_search_invocation =
        app.app_shell_dashboard_dispatch_queue_lane_tab_panel_card_action_menu_group_shortcut_command_palette_search_invocation(
            BerkeleyAppPersistedEditorState {
                selected_syntax_card_index: Some(2),
                active_command_id: Some("analysis.2.run".to_string()),
            },
            "attention",
            None,
        );
    assert!(!shell_dashboard_dispatch_queue_lane_tab_panel_card_action_menu_group_shortcut_command_palette_attention_search_invocation.ready);
    assert_eq!(
        shell_dashboard_dispatch_queue_lane_tab_panel_card_action_menu_group_shortcut_command_palette_attention_search_invocation
            .invocation_state,
        "ready"
    );
    assert!(
        shell_dashboard_dispatch_queue_lane_tab_panel_card_action_menu_group_shortcut_command_palette_attention_search_invocation
            .can_dispatch
    );
    assert!(
        shell_dashboard_dispatch_queue_lane_tab_panel_card_action_menu_group_shortcut_command_palette_attention_search_invocation
            .selection_attention
    );

    let shell_dashboard_dispatch_queue_lane_tab_panel_card_action_menu_group_shortcut_command_palette_disabled_search_invocation =
        app.app_shell_dashboard_dispatch_queue_lane_tab_panel_card_action_menu_group_shortcut_command_palette_search_invocation(
            BerkeleyAppPersistedEditorState {
                selected_syntax_card_index: Some(2),
                active_command_id: Some("analysis.2.run".to_string()),
            },
            "blocked",
            Some("dashboard.dispatch-queue-lane-tab-panel-card-action-menu-group-shortcut-command-palette-search-result.blocked".to_string()),
        );
    assert_eq!(
        shell_dashboard_dispatch_queue_lane_tab_panel_card_action_menu_group_shortcut_command_palette_disabled_search_invocation
            .invocation_state,
        "blocked"
    );
    assert!(
        !shell_dashboard_dispatch_queue_lane_tab_panel_card_action_menu_group_shortcut_command_palette_disabled_search_invocation
            .can_dispatch
    );
    assert_eq!(
        shell_dashboard_dispatch_queue_lane_tab_panel_card_action_menu_group_shortcut_command_palette_disabled_search_invocation
            .blocked_reason
            .as_deref(),
        Some("No blocked dispatches")
    );

    let shell_dashboard_dispatch_queue_lane_tab_panel_card_action_menu_group_shortcut_command_palette_empty_search_invocation_payload:
        serde_json::Value = serde_json::from_str(
        &app.app_shell_dashboard_dispatch_queue_lane_tab_panel_card_action_menu_group_shortcut_command_palette_search_invocation_json(
            BerkeleyAppPersistedEditorState {
                selected_syntax_card_index: Some(2),
                active_command_id: Some("analysis.2.run".to_string()),
            },
            "missing",
            None,
        ),
    )
    .expect(
        "blocked shell dashboard dispatch queue lane tab panel card action menu group shortcut command palette empty search invocation JSON should parse",
    );
    assert_eq!(
        shell_dashboard_dispatch_queue_lane_tab_panel_card_action_menu_group_shortcut_command_palette_empty_search_invocation_payload
            ["invocationState"],
        "empty"
    );
    assert_eq!(
        shell_dashboard_dispatch_queue_lane_tab_panel_card_action_menu_group_shortcut_command_palette_empty_search_invocation_payload
            ["canDispatch"],
        false
    );
    assert_eq!(
        shell_dashboard_dispatch_queue_lane_tab_panel_card_action_menu_group_shortcut_command_palette_empty_search_invocation_payload
            ["blockedReason"],
        "No command palette entries match \"missing\"."
    );
    assert_eq!(
        shell_dashboard_dispatch_queue_lane_tab_panel_card_action_menu_group_shortcut_command_palette_empty_search_invocation_payload
            ["dispatchQueueLaneTabPanelCardActionMenuGroupShortcutCommandPaletteSearchInvocationCapabilityId"],
        "app-shell-dashboard-dispatch-queue-lane-tab-panel-card-action-menu-group-shortcut-command-palette-search-invocation-json"
    );

    let shell_dashboard_dispatch_queue_lane_tab_panel_card_action_menu_group_shortcut_command_palette_attention_search_invocation_receipts =
        app.app_shell_dashboard_dispatch_queue_lane_tab_panel_card_action_menu_group_shortcut_command_palette_search_invocation_receipts(
            BerkeleyAppPersistedEditorState {
                selected_syntax_card_index: Some(2),
                active_command_id: Some("analysis.2.run".to_string()),
            },
            "attention",
            None,
        );
    assert_eq!(
        shell_dashboard_dispatch_queue_lane_tab_panel_card_action_menu_group_shortcut_command_palette_attention_search_invocation_receipts
            .schema_version,
        BERKELEY_APP_SHELL_DASHBOARD_DISPATCH_QUEUE_LANE_TAB_PANEL_CARD_ACTION_MENU_GROUP_SHORTCUT_COMMAND_PALETTE_SEARCH_INVOCATION_RECEIPTS_SCHEMA_VERSION
    );
    assert_eq!(
        shell_dashboard_dispatch_queue_lane_tab_panel_card_action_menu_group_shortcut_command_palette_attention_search_invocation_receipts
            .receipt_count,
        1
    );
    assert_eq!(
        shell_dashboard_dispatch_queue_lane_tab_panel_card_action_menu_group_shortcut_command_palette_attention_search_invocation_receipts
            .accepted_receipt_count,
        1
    );
    assert_eq!(
        shell_dashboard_dispatch_queue_lane_tab_panel_card_action_menu_group_shortcut_command_palette_attention_search_invocation_receipts
            .blocked_receipt_count,
        0
    );
    assert_eq!(
        shell_dashboard_dispatch_queue_lane_tab_panel_card_action_menu_group_shortcut_command_palette_attention_search_invocation_receipts
            .latest_receipt_id
            .as_deref(),
        Some("dashboard.dispatch-queue.shortcut-command-palette-search-invocation.receipt.accepted")
    );
    assert!(
        shell_dashboard_dispatch_queue_lane_tab_panel_card_action_menu_group_shortcut_command_palette_attention_search_invocation_receipts
            .receipts[0]
            .dispatch_accepted
    );
    assert_eq!(
        shell_dashboard_dispatch_queue_lane_tab_panel_card_action_menu_group_shortcut_command_palette_attention_search_invocation_receipts
            .receipts[0]
            .selected_handler_id
            .as_deref(),
        Some("handler.berkeley.app-shell.dashboard.dispatch-queue.menu-group-shortcut.attention")
    );

    let shell_dashboard_dispatch_queue_lane_tab_panel_card_action_menu_group_shortcut_command_palette_disabled_search_invocation_receipts =
        app.app_shell_dashboard_dispatch_queue_lane_tab_panel_card_action_menu_group_shortcut_command_palette_search_invocation_receipts(
            BerkeleyAppPersistedEditorState {
                selected_syntax_card_index: Some(2),
                active_command_id: Some("analysis.2.run".to_string()),
            },
            "blocked",
            Some("dashboard.dispatch-queue-lane-tab-panel-card-action-menu-group-shortcut-command-palette-search-result.blocked".to_string()),
        );
    assert_eq!(
        shell_dashboard_dispatch_queue_lane_tab_panel_card_action_menu_group_shortcut_command_palette_disabled_search_invocation_receipts
            .blocked_receipt_count,
        1
    );
    assert_eq!(
        shell_dashboard_dispatch_queue_lane_tab_panel_card_action_menu_group_shortcut_command_palette_disabled_search_invocation_receipts
            .receipts[0]
            .receipt_state,
        "blocked"
    );
    assert!(
        shell_dashboard_dispatch_queue_lane_tab_panel_card_action_menu_group_shortcut_command_palette_disabled_search_invocation_receipts
            .receipts[0]
            .dispatch_blocked
    );
    assert_eq!(
        shell_dashboard_dispatch_queue_lane_tab_panel_card_action_menu_group_shortcut_command_palette_disabled_search_invocation_receipts
            .receipts[0]
            .blocked_reason
            .as_deref(),
        Some("No blocked dispatches")
    );

    let shell_dashboard_dispatch_queue_lane_tab_panel_card_action_menu_group_shortcut_command_palette_disabled_search_invocation_receipt_summary =
        app.app_shell_dashboard_dispatch_queue_lane_tab_panel_card_action_menu_group_shortcut_command_palette_search_invocation_receipt_summary(
            BerkeleyAppPersistedEditorState {
                selected_syntax_card_index: Some(2),
                active_command_id: Some("analysis.2.run".to_string()),
            },
            "blocked",
            Some("dashboard.dispatch-queue-lane-tab-panel-card-action-menu-group-shortcut-command-palette-search-result.blocked".to_string()),
        );
    assert_eq!(
        shell_dashboard_dispatch_queue_lane_tab_panel_card_action_menu_group_shortcut_command_palette_disabled_search_invocation_receipt_summary
            .status_kind,
        "blocked"
    );
    assert_eq!(
        shell_dashboard_dispatch_queue_lane_tab_panel_card_action_menu_group_shortcut_command_palette_disabled_search_invocation_receipt_summary
            .status_title,
        "Command blocked"
    );
    assert!(
        shell_dashboard_dispatch_queue_lane_tab_panel_card_action_menu_group_shortcut_command_palette_disabled_search_invocation_receipt_summary
            .dispatch_blocked
    );
    assert!(
        shell_dashboard_dispatch_queue_lane_tab_panel_card_action_menu_group_shortcut_command_palette_disabled_search_invocation_receipt_summary
            .attention
    );
    assert_eq!(
        shell_dashboard_dispatch_queue_lane_tab_panel_card_action_menu_group_shortcut_command_palette_disabled_search_invocation_receipt_summary
            .blocked_reason
            .as_deref(),
        Some("No blocked dispatches")
    );

    let shell_dashboard_dispatch_queue_lane_tab_panel_card_action_menu_group_shortcut_command_palette_empty_search_invocation_receipts_payload:
        serde_json::Value = serde_json::from_str(
        &app.app_shell_dashboard_dispatch_queue_lane_tab_panel_card_action_menu_group_shortcut_command_palette_search_invocation_receipts_json(
            BerkeleyAppPersistedEditorState {
                selected_syntax_card_index: Some(2),
                active_command_id: Some("analysis.2.run".to_string()),
            },
            "missing",
            None,
        ),
    )
    .expect(
        "blocked shell dashboard dispatch queue lane tab panel card action menu group shortcut command palette empty search invocation receipts JSON should parse",
    );
    assert_eq!(
        shell_dashboard_dispatch_queue_lane_tab_panel_card_action_menu_group_shortcut_command_palette_empty_search_invocation_receipts_payload
            ["receiptCount"],
        1
    );
    assert_eq!(
        shell_dashboard_dispatch_queue_lane_tab_panel_card_action_menu_group_shortcut_command_palette_empty_search_invocation_receipts_payload
            ["emptyReceiptCount"],
        1
    );
    assert_eq!(
        shell_dashboard_dispatch_queue_lane_tab_panel_card_action_menu_group_shortcut_command_palette_empty_search_invocation_receipts_payload
            ["receipts"][0]["receiptState"],
        "empty"
    );
    assert_eq!(
        shell_dashboard_dispatch_queue_lane_tab_panel_card_action_menu_group_shortcut_command_palette_empty_search_invocation_receipts_payload
            ["dispatchQueueLaneTabPanelCardActionMenuGroupShortcutCommandPaletteSearchInvocationReceiptsCapabilityId"],
        "app-shell-dashboard-dispatch-queue-lane-tab-panel-card-action-menu-group-shortcut-command-palette-search-invocation-receipts-json"
    );

    let shell_dashboard_dispatch_queue_lane_tab_panel_card_action_menu_group_shortcut_command_palette_empty_search_invocation_receipt_summary_payload:
        serde_json::Value = serde_json::from_str(
        &app.app_shell_dashboard_dispatch_queue_lane_tab_panel_card_action_menu_group_shortcut_command_palette_search_invocation_receipt_summary_json(
            BerkeleyAppPersistedEditorState {
                selected_syntax_card_index: Some(2),
                active_command_id: Some("analysis.2.run".to_string()),
            },
            "missing",
            None,
        ),
    )
    .expect(
        "blocked shell dashboard dispatch queue lane tab panel card action menu group shortcut command palette empty search invocation receipt summary JSON should parse",
    );
    assert_eq!(
        shell_dashboard_dispatch_queue_lane_tab_panel_card_action_menu_group_shortcut_command_palette_empty_search_invocation_receipt_summary_payload
            ["statusKind"],
        "empty"
    );
    assert_eq!(
        shell_dashboard_dispatch_queue_lane_tab_panel_card_action_menu_group_shortcut_command_palette_empty_search_invocation_receipt_summary_payload
            ["statusTitle"],
        "No command selected"
    );
    assert_eq!(
        shell_dashboard_dispatch_queue_lane_tab_panel_card_action_menu_group_shortcut_command_palette_empty_search_invocation_receipt_summary_payload
            ["emptyReceiptCount"],
        1
    );
    assert_eq!(
        shell_dashboard_dispatch_queue_lane_tab_panel_card_action_menu_group_shortcut_command_palette_empty_search_invocation_receipt_summary_payload
            ["dispatchQueueLaneTabPanelCardActionMenuGroupShortcutCommandPaletteSearchInvocationReceiptSummaryCapabilityId"],
        "app-shell-dashboard-dispatch-queue-lane-tab-panel-card-action-menu-group-shortcut-command-palette-search-invocation-receipt-summary-json"
    );

    let shell_dashboard_dispatch_queue_lane_tab_panel_card_action_menu_group_shortcut_command_palette_disabled_search_invocation_receipt_notification =
        app.app_shell_dashboard_dispatch_queue_lane_tab_panel_card_action_menu_group_shortcut_command_palette_search_invocation_receipt_notification(
            BerkeleyAppPersistedEditorState {
                selected_syntax_card_index: Some(2),
                active_command_id: Some("analysis.2.run".to_string()),
            },
            "blocked",
            Some("dashboard.dispatch-queue-lane-tab-panel-card-action-menu-group-shortcut-command-palette-search-result.blocked".to_string()),
        );
    assert_eq!(
        shell_dashboard_dispatch_queue_lane_tab_panel_card_action_menu_group_shortcut_command_palette_disabled_search_invocation_receipt_notification
            .notification_kind,
        "dispatch-blocked"
    );
    assert_eq!(
        shell_dashboard_dispatch_queue_lane_tab_panel_card_action_menu_group_shortcut_command_palette_disabled_search_invocation_receipt_notification
            .notification_level,
        "error"
    );
    assert_eq!(
        shell_dashboard_dispatch_queue_lane_tab_panel_card_action_menu_group_shortcut_command_palette_disabled_search_invocation_receipt_notification
            .notification_action_label
            .as_deref(),
        Some("Review command")
    );
    assert!(
        shell_dashboard_dispatch_queue_lane_tab_panel_card_action_menu_group_shortcut_command_palette_disabled_search_invocation_receipt_notification
            .dispatch_blocked
    );
    assert!(
        shell_dashboard_dispatch_queue_lane_tab_panel_card_action_menu_group_shortcut_command_palette_disabled_search_invocation_receipt_notification
            .attention
    );
    assert_eq!(
        shell_dashboard_dispatch_queue_lane_tab_panel_card_action_menu_group_shortcut_command_palette_disabled_search_invocation_receipt_notification
            .blocked_reason
            .as_deref(),
        Some("No blocked dispatches")
    );

    let shell_dashboard_dispatch_queue_lane_tab_panel_card_action_menu_group_shortcut_command_palette_disabled_search_invocation_receipt_notification_stack =
        app.app_shell_dashboard_dispatch_queue_lane_tab_panel_card_action_menu_group_shortcut_command_palette_search_invocation_receipt_notification_stack(
            BerkeleyAppPersistedEditorState {
                selected_syntax_card_index: Some(2),
                active_command_id: Some("analysis.2.run".to_string()),
            },
            "blocked",
            Some("dashboard.dispatch-queue-lane-tab-panel-card-action-menu-group-shortcut-command-palette-search-result.blocked".to_string()),
        );
    assert_eq!(
        shell_dashboard_dispatch_queue_lane_tab_panel_card_action_menu_group_shortcut_command_palette_disabled_search_invocation_receipt_notification_stack
            .latest_notification_kind
            .as_deref(),
        Some("dispatch-blocked")
    );
    assert_eq!(
        shell_dashboard_dispatch_queue_lane_tab_panel_card_action_menu_group_shortcut_command_palette_disabled_search_invocation_receipt_notification_stack
            .error_notification_count,
        1
    );
    assert_eq!(
        shell_dashboard_dispatch_queue_lane_tab_panel_card_action_menu_group_shortcut_command_palette_disabled_search_invocation_receipt_notification_stack
            .attention_notification_count,
        1
    );
    assert_eq!(
        shell_dashboard_dispatch_queue_lane_tab_panel_card_action_menu_group_shortcut_command_palette_disabled_search_invocation_receipt_notification_stack
            .attention_notification_id
            .as_deref(),
        Some("dashboard.dispatch-queue.shortcut-command-palette-search-invocation-receipt-notification")
    );

    let blocked_notification_stack_summary =
        app.app_shell_dashboard_dispatch_queue_lane_tab_panel_card_action_menu_group_shortcut_command_palette_search_invocation_receipt_notification_stack_summary(
            BerkeleyAppPersistedEditorState {
                selected_syntax_card_index: Some(2),
                active_command_id: Some("analysis.2.run".to_string()),
            },
            "blocked",
            Some("dashboard.dispatch-queue-lane-tab-panel-card-action-menu-group-shortcut-command-palette-search-result.blocked".to_string()),
        );
    assert_eq!(blocked_notification_stack_summary.summary_kind, "attention");
    assert_eq!(blocked_notification_stack_summary.summary_level, "error");
    assert!(blocked_notification_stack_summary.has_attention_notifications);
    assert!(blocked_notification_stack_summary.has_error_notifications);
    assert_eq!(
        blocked_notification_stack_summary
            .headline_notification_kind
            .as_deref(),
        Some("dispatch-blocked")
    );

    let shell_dashboard_dispatch_queue_lane_tab_panel_card_action_menu_group_shortcut_command_palette_empty_search_invocation_receipt_notification_payload:
        serde_json::Value = serde_json::from_str(
        &app.app_shell_dashboard_dispatch_queue_lane_tab_panel_card_action_menu_group_shortcut_command_palette_search_invocation_receipt_notification_json(
            BerkeleyAppPersistedEditorState {
                selected_syntax_card_index: Some(2),
                active_command_id: Some("analysis.2.run".to_string()),
            },
            "missing",
            None,
        ),
    )
    .expect(
        "blocked shell dashboard dispatch queue lane tab panel card action menu group shortcut command palette empty search invocation receipt notification JSON should parse",
    );
    assert_eq!(
        shell_dashboard_dispatch_queue_lane_tab_panel_card_action_menu_group_shortcut_command_palette_empty_search_invocation_receipt_notification_payload
            ["notificationKind"],
        "dispatch-empty"
    );
    assert_eq!(
        shell_dashboard_dispatch_queue_lane_tab_panel_card_action_menu_group_shortcut_command_palette_empty_search_invocation_receipt_notification_payload
            ["notificationLevel"],
        "info"
    );
    assert_eq!(
        shell_dashboard_dispatch_queue_lane_tab_panel_card_action_menu_group_shortcut_command_palette_empty_search_invocation_receipt_notification_payload
            ["notificationActionLabel"],
        serde_json::Value::Null
    );
    assert_eq!(
        shell_dashboard_dispatch_queue_lane_tab_panel_card_action_menu_group_shortcut_command_palette_empty_search_invocation_receipt_notification_payload
            ["shouldAnnounce"],
        false
    );
    assert_eq!(
        shell_dashboard_dispatch_queue_lane_tab_panel_card_action_menu_group_shortcut_command_palette_empty_search_invocation_receipt_notification_payload
            ["dispatchQueueLaneTabPanelCardActionMenuGroupShortcutCommandPaletteSearchInvocationReceiptNotificationCapabilityId"],
        "app-shell-dashboard-dispatch-queue-lane-tab-panel-card-action-menu-group-shortcut-command-palette-search-invocation-receipt-notification-json"
    );

    let shell_dashboard_dispatch_queue_lane_tab_panel_card_action_menu_group_shortcut_command_palette_empty_search_invocation_receipt_notification_stack_payload:
        serde_json::Value = serde_json::from_str(
        &app.app_shell_dashboard_dispatch_queue_lane_tab_panel_card_action_menu_group_shortcut_command_palette_search_invocation_receipt_notification_stack_json(
            BerkeleyAppPersistedEditorState {
                selected_syntax_card_index: Some(2),
                active_command_id: Some("analysis.2.run".to_string()),
            },
            "missing",
            None,
        ),
    )
    .expect(
        "blocked shell dashboard dispatch queue lane tab panel card action menu group shortcut command palette empty search invocation receipt notification stack JSON should parse",
    );
    assert_eq!(
        shell_dashboard_dispatch_queue_lane_tab_panel_card_action_menu_group_shortcut_command_palette_empty_search_invocation_receipt_notification_stack_payload
            ["latestNotificationKind"],
        "dispatch-empty"
    );
    assert_eq!(
        shell_dashboard_dispatch_queue_lane_tab_panel_card_action_menu_group_shortcut_command_palette_empty_search_invocation_receipt_notification_stack_payload
            ["visibleNotificationCount"],
        0
    );
    assert_eq!(
        shell_dashboard_dispatch_queue_lane_tab_panel_card_action_menu_group_shortcut_command_palette_empty_search_invocation_receipt_notification_stack_payload
            ["activeNotificationId"],
        serde_json::Value::Null
    );
    assert_eq!(
        shell_dashboard_dispatch_queue_lane_tab_panel_card_action_menu_group_shortcut_command_palette_empty_search_invocation_receipt_notification_stack_payload
            ["infoNotificationCount"],
        1
    );
    assert_eq!(
        shell_dashboard_dispatch_queue_lane_tab_panel_card_action_menu_group_shortcut_command_palette_empty_search_invocation_receipt_notification_stack_payload
            ["notifications"][0]["shouldAnnounce"],
        false
    );
    assert_eq!(
        shell_dashboard_dispatch_queue_lane_tab_panel_card_action_menu_group_shortcut_command_palette_empty_search_invocation_receipt_notification_stack_payload
            ["dispatchQueueLaneTabPanelCardActionMenuGroupShortcutCommandPaletteSearchInvocationReceiptNotificationStackCapabilityId"],
        "app-shell-dashboard-dispatch-queue-lane-tab-panel-card-action-menu-group-shortcut-command-palette-search-invocation-receipt-notification-stack-json"
    );

    let empty_notification_stack_summary_payload: serde_json::Value = serde_json::from_str(
        &app.app_shell_dashboard_dispatch_queue_lane_tab_panel_card_action_menu_group_shortcut_command_palette_search_invocation_receipt_notification_stack_summary_json(
            BerkeleyAppPersistedEditorState {
                selected_syntax_card_index: Some(2),
                active_command_id: Some("analysis.2.run".to_string()),
            },
            "missing",
            None,
        ),
    )
    .expect(
        "blocked shell dashboard dispatch queue lane tab panel card action menu group shortcut command palette empty search invocation receipt notification stack summary JSON should parse",
    );
    assert_eq!(
        empty_notification_stack_summary_payload["summaryKind"],
        "empty"
    );
    assert_eq!(
        empty_notification_stack_summary_payload["summaryLevel"],
        "info"
    );
    assert_eq!(
        empty_notification_stack_summary_payload["shouldRenderStack"],
        false
    );
    assert_eq!(
        empty_notification_stack_summary_payload["headlineNotificationKind"],
        "dispatch-empty"
    );
    assert_eq!(
        empty_notification_stack_summary_payload["hasInfoNotifications"],
        true
    );
    assert_eq!(
        empty_notification_stack_summary_payload
            ["dispatchQueueLaneTabPanelCardActionMenuGroupShortcutCommandPaletteSearchInvocationReceiptNotificationStackSummaryCapabilityId"],
        "app-shell-dashboard-dispatch-queue-lane-tab-panel-card-action-menu-group-shortcut-command-palette-search-invocation-receipt-notification-stack-summary-json"
    );

    let empty_notification_stack_summary_product_handoff_payload: serde_json::Value = serde_json::from_str(
        &app.app_shell_dashboard_dispatch_queue_lane_tab_panel_card_action_menu_group_shortcut_command_palette_search_invocation_receipt_notification_stack_summary_product_handoff_json(
            BerkeleyAppPersistedEditorState {
                selected_syntax_card_index: Some(2),
                active_command_id: Some("analysis.2.run".to_string()),
            },
            "missing",
            None,
        ),
    )
    .expect(
        "blocked shell dashboard dispatch queue lane tab panel card action menu group shortcut command palette empty search invocation receipt notification stack summary product handoff JSON should parse",
    );
    assert_eq!(
        empty_notification_stack_summary_product_handoff_payload["handoffRoute"],
        "dispatch-status"
    );
    assert_eq!(
        empty_notification_stack_summary_product_handoff_payload["productShellAction"],
        "render-empty-dispatch-status"
    );
    assert_eq!(
        empty_notification_stack_summary_product_handoff_payload["notificationBadgeCount"],
        0
    );
    assert_eq!(
        empty_notification_stack_summary_product_handoff_payload["liveRegionId"],
        serde_json::Value::Null
    );
    assert_eq!(
        empty_notification_stack_summary_product_handoff_payload["stackSummary"]["summaryKind"],
        "empty"
    );
    assert_eq!(
        empty_notification_stack_summary_product_handoff_payload
            ["dispatchQueueLaneTabPanelCardActionMenuGroupShortcutCommandPaletteSearchInvocationReceiptNotificationStackSummaryProductHandoffCapabilityId"],
        "app-shell-dashboard-dispatch-queue-lane-tab-panel-card-action-menu-group-shortcut-command-palette-search-invocation-receipt-notification-stack-summary-product-handoff-json"
    );

    let empty_notification_stack_summary_product_handoff_delivery_package_payload: serde_json::Value = serde_json::from_str(
        &app.app_shell_dashboard_dispatch_queue_lane_tab_panel_card_action_menu_group_shortcut_command_palette_search_invocation_receipt_notification_stack_summary_product_handoff_delivery_package_json(
            BerkeleyAppPersistedEditorState {
                selected_syntax_card_index: Some(2),
                active_command_id: Some("analysis.2.run".to_string()),
            },
            "missing",
            None,
        ),
    )
    .expect(
        "blocked shell dashboard dispatch queue lane tab panel card action menu group shortcut command palette empty search invocation receipt notification stack summary product handoff delivery package JSON should parse",
    );
    assert_eq!(
        empty_notification_stack_summary_product_handoff_delivery_package_payload["deliveryRoute"],
        "blocked/dispatch-status"
    );
    assert_eq!(
        empty_notification_stack_summary_product_handoff_delivery_package_payload["shouldHydrate"],
        false
    );
    assert_eq!(
        empty_notification_stack_summary_product_handoff_delivery_package_payload["productHandoff"]
            ["stackSummary"]["summaryKind"],
        "empty"
    );
    assert_eq!(
        empty_notification_stack_summary_product_handoff_delivery_package_payload
            ["dispatchQueueLaneTabPanelCardActionMenuGroupShortcutCommandPaletteSearchInvocationReceiptNotificationStackSummaryProductHandoffDeliveryPackageCapabilityId"],
        "app-shell-dashboard-dispatch-queue-lane-tab-panel-card-action-menu-group-shortcut-command-palette-search-invocation-receipt-notification-stack-summary-product-handoff-delivery-package-json"
    );

    let empty_notification_stack_summary_product_handoff_delivery_package_embed_manifest_payload: serde_json::Value = serde_json::from_str(
        &app.app_shell_dashboard_dispatch_queue_lane_tab_panel_card_action_menu_group_shortcut_command_palette_search_invocation_receipt_notification_stack_summary_product_handoff_delivery_package_embed_manifest_json(
            BerkeleyAppPersistedEditorState {
                selected_syntax_card_index: Some(2),
                active_command_id: Some("analysis.2.run".to_string()),
            },
            "missing",
            None,
        ),
    )
    .expect(
        "blocked shell dashboard dispatch queue lane tab panel card action menu group shortcut command palette empty search invocation receipt notification stack summary product handoff delivery package embed manifest JSON should parse",
    );
    assert_eq!(
        empty_notification_stack_summary_product_handoff_delivery_package_embed_manifest_payload
            ["hydrationMode"],
        "defer"
    );
    assert_eq!(
        empty_notification_stack_summary_product_handoff_delivery_package_embed_manifest_payload
            ["shouldPreload"],
        true
    );
    assert_eq!(
        empty_notification_stack_summary_product_handoff_delivery_package_embed_manifest_payload
            ["shouldInstantiate"],
        false
    );
    assert_eq!(
        empty_notification_stack_summary_product_handoff_delivery_package_embed_manifest_payload
            ["shouldMount"],
        false
    );
    assert_eq!(
        empty_notification_stack_summary_product_handoff_delivery_package_embed_manifest_payload
            ["deliveryPackage"]["deliveryRoute"],
        "blocked/dispatch-status"
    );
    assert_eq!(
        empty_notification_stack_summary_product_handoff_delivery_package_embed_manifest_payload
            ["dispatchQueueLaneTabPanelCardActionMenuGroupShortcutCommandPaletteSearchInvocationReceiptNotificationStackSummaryProductHandoffDeliveryPackageEmbedManifestCapabilityId"],
        "app-shell-dashboard-dispatch-queue-lane-tab-panel-card-action-menu-group-shortcut-command-palette-search-invocation-receipt-notification-stack-summary-product-handoff-delivery-package-embed-manifest-json"
    );

    let empty_notification_stack_summary_product_handoff_delivery_package_embed_loader_plan_payload: serde_json::Value = serde_json::from_str(
        &app.app_shell_dashboard_dispatch_queue_lane_tab_panel_card_action_menu_group_shortcut_command_palette_search_invocation_receipt_notification_stack_summary_product_handoff_delivery_package_embed_loader_plan_json(
            BerkeleyAppPersistedEditorState {
                selected_syntax_card_index: Some(2),
                active_command_id: Some("analysis.2.run".to_string()),
            },
            "missing",
            None,
        ),
    )
    .expect(
        "blocked shell dashboard dispatch queue lane tab panel card action menu group shortcut command palette empty search invocation receipt notification stack summary product handoff delivery package embed loader plan JSON should parse",
    );
    assert_eq!(
        empty_notification_stack_summary_product_handoff_delivery_package_embed_loader_plan_payload
            ["loaderPhase"],
        "blocked"
    );
    assert_eq!(
        empty_notification_stack_summary_product_handoff_delivery_package_embed_loader_plan_payload
            ["loaderStrategy"],
        "preload-defer"
    );
    assert_eq!(
        empty_notification_stack_summary_product_handoff_delivery_package_embed_loader_plan_payload
            ["shouldInstantiate"],
        false
    );
    assert_eq!(
        empty_notification_stack_summary_product_handoff_delivery_package_embed_loader_plan_payload
            ["loadOrder"][1],
        "defer-product-handoff-hydration"
    );
    assert_eq!(
        empty_notification_stack_summary_product_handoff_delivery_package_embed_loader_plan_payload
            ["embedManifest"]["hydrationMode"],
        "defer"
    );
    assert_eq!(
        empty_notification_stack_summary_product_handoff_delivery_package_embed_loader_plan_payload
            ["dispatchQueueLaneTabPanelCardActionMenuGroupShortcutCommandPaletteSearchInvocationReceiptNotificationStackSummaryProductHandoffDeliveryPackageEmbedLoaderPlanCapabilityId"],
        "app-shell-dashboard-dispatch-queue-lane-tab-panel-card-action-menu-group-shortcut-command-palette-search-invocation-receipt-notification-stack-summary-product-handoff-delivery-package-embed-loader-plan-json"
    );

    let empty_notification_stack_summary_product_handoff_delivery_package_embed_runtime_plan_payload: serde_json::Value = serde_json::from_str(
        &app.app_shell_dashboard_dispatch_queue_lane_tab_panel_card_action_menu_group_shortcut_command_palette_search_invocation_receipt_notification_stack_summary_product_handoff_delivery_package_embed_runtime_plan_json(
            BerkeleyAppPersistedEditorState {
                selected_syntax_card_index: Some(2),
                active_command_id: Some("analysis.2.run".to_string()),
            },
            "missing",
            None,
        ),
    )
    .expect(
        "blocked shell dashboard dispatch queue lane tab panel card action menu group shortcut command palette empty search invocation receipt notification stack summary product handoff delivery package embed runtime plan JSON should parse",
    );
    assert_eq!(
        empty_notification_stack_summary_product_handoff_delivery_package_embed_runtime_plan_payload
            ["runtimePhase"],
        "blocked"
    );
    assert_eq!(
        empty_notification_stack_summary_product_handoff_delivery_package_embed_runtime_plan_payload
            ["runtimeStrategy"],
        "start-runtime-defer-hydration"
    );
    assert_eq!(
        empty_notification_stack_summary_product_handoff_delivery_package_embed_runtime_plan_payload
            ["shouldStartRuntime"],
        true
    );
    assert_eq!(
        empty_notification_stack_summary_product_handoff_delivery_package_embed_runtime_plan_payload
            ["runtimeSteps"][3],
        "defer-product-handoff-hydration"
    );
    assert_eq!(
        empty_notification_stack_summary_product_handoff_delivery_package_embed_runtime_plan_payload
            ["runtimeSteps"][5],
        "publish-runtime-blocked"
    );
    assert_eq!(
        empty_notification_stack_summary_product_handoff_delivery_package_embed_runtime_plan_payload
            ["embedLoaderPlan"]["loaderPhase"],
        "blocked"
    );
    assert_eq!(
        empty_notification_stack_summary_product_handoff_delivery_package_embed_runtime_plan_payload
            ["dispatchQueueLaneTabPanelCardActionMenuGroupShortcutCommandPaletteSearchInvocationReceiptNotificationStackSummaryProductHandoffDeliveryPackageEmbedRuntimePlanCapabilityId"],
        "app-shell-dashboard-dispatch-queue-lane-tab-panel-card-action-menu-group-shortcut-command-palette-search-invocation-receipt-notification-stack-summary-product-handoff-delivery-package-embed-runtime-plan-json"
    );

    let empty_notification_stack_summary_product_handoff_delivery_package_embed_runtime_session_plan_payload: serde_json::Value = serde_json::from_str(
        &app.app_shell_dashboard_dispatch_queue_lane_tab_panel_card_action_menu_group_shortcut_command_palette_search_invocation_receipt_notification_stack_summary_product_handoff_delivery_package_embed_runtime_session_plan_json(
            BerkeleyAppPersistedEditorState {
                selected_syntax_card_index: Some(2),
                active_command_id: Some("analysis.2.run".to_string()),
            },
            "missing",
            None,
        ),
    )
    .expect(
        "blocked shell dashboard dispatch queue lane tab panel card action menu group shortcut command palette empty search invocation receipt notification stack summary product handoff delivery package embed runtime session plan JSON should parse",
    );
    assert_eq!(
        empty_notification_stack_summary_product_handoff_delivery_package_embed_runtime_session_plan_payload
            ["runtimeSessionPhase"],
        "blocked"
    );
    assert_eq!(
        empty_notification_stack_summary_product_handoff_delivery_package_embed_runtime_session_plan_payload
            ["runtimeSessionStrategy"],
        "open-session-defer-runtime"
    );
    assert_eq!(
        empty_notification_stack_summary_product_handoff_delivery_package_embed_runtime_session_plan_payload
            ["shouldPublishReady"],
        false
    );
    assert_eq!(
        empty_notification_stack_summary_product_handoff_delivery_package_embed_runtime_session_plan_payload
            ["shouldDeferSession"],
        true
    );
    assert_eq!(
        empty_notification_stack_summary_product_handoff_delivery_package_embed_runtime_session_plan_payload
            ["sessionSteps"][8],
        "publish-runtime-session-blocked"
    );
    assert_eq!(
        empty_notification_stack_summary_product_handoff_delivery_package_embed_runtime_session_plan_payload
            ["embedRuntimePlan"]["runtimePhase"],
        "blocked"
    );
    assert_eq!(
        empty_notification_stack_summary_product_handoff_delivery_package_embed_runtime_session_plan_payload
            ["dispatchQueueLaneTabPanelCardActionMenuGroupShortcutCommandPaletteSearchInvocationReceiptNotificationStackSummaryProductHandoffDeliveryPackageEmbedRuntimeSessionPlanCapabilityId"],
        "app-shell-dashboard-dispatch-queue-lane-tab-panel-card-action-menu-group-shortcut-command-palette-search-invocation-receipt-notification-stack-summary-product-handoff-delivery-package-embed-runtime-session-plan-json"
    );

    let empty_notification_stack_summary_product_handoff_delivery_package_embed_runtime_activation_plan_payload: serde_json::Value = serde_json::from_str(
        &app.app_shell_dashboard_dispatch_queue_lane_tab_panel_card_action_menu_group_shortcut_command_palette_search_invocation_receipt_notification_stack_summary_product_handoff_delivery_package_embed_runtime_activation_plan_json(
            BerkeleyAppPersistedEditorState {
                selected_syntax_card_index: Some(2),
                active_command_id: Some("analysis.2.run".to_string()),
            },
            "missing",
            None,
        ),
    )
    .expect(
        "blocked shell dashboard dispatch queue lane tab panel card action menu group shortcut command palette empty search invocation receipt notification stack summary product handoff delivery package embed runtime activation plan JSON should parse",
    );
    assert_eq!(
        empty_notification_stack_summary_product_handoff_delivery_package_embed_runtime_activation_plan_payload
            ["runtimeActivationPhase"],
        "blocked"
    );
    assert_eq!(
        empty_notification_stack_summary_product_handoff_delivery_package_embed_runtime_activation_plan_payload
            ["runtimeActivationStrategy"],
        "request-session-activation-defer"
    );
    assert_eq!(
        empty_notification_stack_summary_product_handoff_delivery_package_embed_runtime_activation_plan_payload
            ["shouldDeferActivation"],
        true
    );
    assert_eq!(
        empty_notification_stack_summary_product_handoff_delivery_package_embed_runtime_activation_plan_payload
            ["activationSteps"][11],
        "publish-runtime-activation-blocked"
    );
    assert_eq!(
        empty_notification_stack_summary_product_handoff_delivery_package_embed_runtime_activation_plan_payload
            ["embedRuntimeSessionPlan"]["runtimeSessionPhase"],
        "blocked"
    );
    assert_eq!(
        empty_notification_stack_summary_product_handoff_delivery_package_embed_runtime_activation_plan_payload
            ["dispatchQueueLaneTabPanelCardActionMenuGroupShortcutCommandPaletteSearchInvocationReceiptNotificationStackSummaryProductHandoffDeliveryPackageEmbedRuntimeActivationPlanCapabilityId"],
        "app-shell-dashboard-dispatch-queue-lane-tab-panel-card-action-menu-group-shortcut-command-palette-search-invocation-receipt-notification-stack-summary-product-handoff-delivery-package-embed-runtime-activation-plan-json"
    );

    let empty_notification_stack_summary_product_handoff_delivery_package_embed_runtime_activation_receipt_payload: serde_json::Value = serde_json::from_str(
        &app.app_shell_dashboard_dispatch_queue_lane_tab_panel_card_action_menu_group_shortcut_command_palette_search_invocation_receipt_notification_stack_summary_product_handoff_delivery_package_embed_runtime_activation_receipt_json(
            BerkeleyAppPersistedEditorState {
                selected_syntax_card_index: Some(2),
                active_command_id: Some("analysis.2.run".to_string()),
            },
            "missing",
            None,
        ),
    )
    .expect(
        "blocked shell dashboard dispatch queue lane tab panel card action menu group shortcut command palette empty search invocation receipt notification stack summary product handoff delivery package embed runtime activation receipt JSON should parse",
    );
    assert_eq!(
        empty_notification_stack_summary_product_handoff_delivery_package_embed_runtime_activation_receipt_payload
            ["runtimeActivationReceiptStatus"],
        "deferred"
    );
    assert_eq!(
        empty_notification_stack_summary_product_handoff_delivery_package_embed_runtime_activation_receipt_payload
            ["runtimeActivationReceiptOutcome"],
        "blocked"
    );
    assert_eq!(
        empty_notification_stack_summary_product_handoff_delivery_package_embed_runtime_activation_receipt_payload
            ["shouldDeferReceipt"],
        true
    );
    assert_eq!(
        empty_notification_stack_summary_product_handoff_delivery_package_embed_runtime_activation_receipt_payload
            ["activationReceiptSteps"][14],
        "publish-runtime-activation-receipt-blocked"
    );
    assert_eq!(
        empty_notification_stack_summary_product_handoff_delivery_package_embed_runtime_activation_receipt_payload
            ["embedRuntimeActivationPlan"]["runtimeActivationPhase"],
        "blocked"
    );
    assert_eq!(
        empty_notification_stack_summary_product_handoff_delivery_package_embed_runtime_activation_receipt_payload
            ["dispatchQueueLaneTabPanelCardActionMenuGroupShortcutCommandPaletteSearchInvocationReceiptNotificationStackSummaryProductHandoffDeliveryPackageEmbedRuntimeActivationReceiptCapabilityId"],
        "app-shell-dashboard-dispatch-queue-lane-tab-panel-card-action-menu-group-shortcut-command-palette-search-invocation-receipt-notification-stack-summary-product-handoff-delivery-package-embed-runtime-activation-receipt-json"
    );

    let empty_notification_stack_summary_product_handoff_delivery_package_embed_runtime_activation_receipt_journal_payload: serde_json::Value = serde_json::from_str(
        &app.app_shell_dashboard_dispatch_queue_lane_tab_panel_card_action_menu_group_shortcut_command_palette_search_invocation_receipt_notification_stack_summary_product_handoff_delivery_package_embed_runtime_activation_receipt_journal_json(
            BerkeleyAppPersistedEditorState {
                selected_syntax_card_index: Some(2),
                active_command_id: Some("analysis.2.run".to_string()),
            },
            "missing",
            None,
        ),
    )
    .expect(
        "blocked shell dashboard dispatch queue lane tab panel card action menu group shortcut command palette empty search invocation receipt notification stack summary product handoff delivery package embed runtime activation receipt journal JSON should parse",
    );
    assert_eq!(
        empty_notification_stack_summary_product_handoff_delivery_package_embed_runtime_activation_receipt_journal_payload
            ["runtimeActivationReceiptJournalStatus"],
        "deferred"
    );
    assert_eq!(
        empty_notification_stack_summary_product_handoff_delivery_package_embed_runtime_activation_receipt_journal_payload
            ["runtimeActivationReceiptJournalOutcome"],
        "blocked"
    );
    assert_eq!(
        empty_notification_stack_summary_product_handoff_delivery_package_embed_runtime_activation_receipt_journal_payload
            ["shouldDeferJournal"],
        true
    );
    assert_eq!(
        empty_notification_stack_summary_product_handoff_delivery_package_embed_runtime_activation_receipt_journal_payload
            ["activationReceiptJournalSteps"][17],
        "publish-runtime-activation-receipt-journal-blocked"
    );
    assert_eq!(
        empty_notification_stack_summary_product_handoff_delivery_package_embed_runtime_activation_receipt_journal_payload
            ["embedRuntimeActivationReceipt"]["runtimeActivationReceiptOutcome"],
        "blocked"
    );
    assert_eq!(
        empty_notification_stack_summary_product_handoff_delivery_package_embed_runtime_activation_receipt_journal_payload
            ["dispatchQueueLaneTabPanelCardActionMenuGroupShortcutCommandPaletteSearchInvocationReceiptNotificationStackSummaryProductHandoffDeliveryPackageEmbedRuntimeActivationReceiptJournalCapabilityId"],
        "app-shell-dashboard-dispatch-queue-lane-tab-panel-card-action-menu-group-shortcut-command-palette-search-invocation-receipt-notification-stack-summary-product-handoff-delivery-package-embed-runtime-activation-receipt-journal-json"
    );

    let empty_notification_stack_summary_product_handoff_delivery_package_embed_runtime_activation_receipt_journal_summary_payload: serde_json::Value = serde_json::from_str(
        &app.app_shell_dashboard_dispatch_queue_lane_tab_panel_card_action_menu_group_shortcut_command_palette_search_invocation_receipt_notification_stack_summary_product_handoff_delivery_package_embed_runtime_activation_receipt_journal_summary_json(
            BerkeleyAppPersistedEditorState {
                selected_syntax_card_index: Some(2),
                active_command_id: Some("analysis.2.run".to_string()),
            },
            "missing",
            None,
        ),
    )
    .expect(
        "blocked shell dashboard dispatch queue lane tab panel card action menu group shortcut command palette empty search invocation receipt notification stack summary product handoff delivery package embed runtime activation receipt journal summary JSON should parse",
    );
    assert_eq!(
        empty_notification_stack_summary_product_handoff_delivery_package_embed_runtime_activation_receipt_journal_summary_payload
            ["runtimeActivationReceiptJournalSummaryStatus"],
        "deferred"
    );
    assert_eq!(
        empty_notification_stack_summary_product_handoff_delivery_package_embed_runtime_activation_receipt_journal_summary_payload
            ["runtimeActivationReceiptJournalSummaryOutcome"],
        "blocked"
    );
    assert_eq!(
        empty_notification_stack_summary_product_handoff_delivery_package_embed_runtime_activation_receipt_journal_summary_payload
            ["journalEntryCount"],
        1
    );
    assert_eq!(
        empty_notification_stack_summary_product_handoff_delivery_package_embed_runtime_activation_receipt_journal_summary_payload
            ["committedJournalEntryCount"],
        0
    );
    assert_eq!(
        empty_notification_stack_summary_product_handoff_delivery_package_embed_runtime_activation_receipt_journal_summary_payload
            ["deferredJournalEntryCount"],
        1
    );
    assert_eq!(
        empty_notification_stack_summary_product_handoff_delivery_package_embed_runtime_activation_receipt_journal_summary_payload
            ["shouldDeferJournal"],
        true
    );
    assert_eq!(
        empty_notification_stack_summary_product_handoff_delivery_package_embed_runtime_activation_receipt_journal_summary_payload
            ["activationReceiptJournalSummarySteps"][20],
        "publish-runtime-activation-receipt-journal-summary-blocked"
    );
    assert_eq!(
        empty_notification_stack_summary_product_handoff_delivery_package_embed_runtime_activation_receipt_journal_summary_payload
            ["embedRuntimeActivationReceiptJournal"]["runtimeActivationReceiptJournalOutcome"],
        "blocked"
    );
    assert_eq!(
        empty_notification_stack_summary_product_handoff_delivery_package_embed_runtime_activation_receipt_journal_summary_payload
            ["dispatchQueueLaneTabPanelCardActionMenuGroupShortcutCommandPaletteSearchInvocationReceiptNotificationStackSummaryProductHandoffDeliveryPackageEmbedRuntimeActivationReceiptJournalSummaryCapabilityId"],
        "app-shell-dashboard-dispatch-queue-lane-tab-panel-card-action-menu-group-shortcut-command-palette-search-invocation-receipt-notification-stack-summary-product-handoff-delivery-package-embed-runtime-activation-receipt-journal-summary-json"
    );

    let empty_notification_stack_summary_product_handoff_delivery_package_embed_runtime_activation_receipt_journal_summary_handoff_payload: serde_json::Value = serde_json::from_str(
        &app.app_shell_dashboard_dispatch_queue_lane_tab_panel_card_action_menu_group_shortcut_command_palette_search_invocation_receipt_notification_stack_summary_product_handoff_delivery_package_embed_runtime_activation_receipt_journal_summary_handoff_json(
            BerkeleyAppPersistedEditorState {
                selected_syntax_card_index: Some(2),
                active_command_id: Some("analysis.2.run".to_string()),
            },
            "missing",
            None,
        ),
    )
    .expect(
        "blocked shell dashboard dispatch queue lane tab panel card action menu group shortcut command palette empty search invocation receipt notification stack summary product handoff delivery package embed runtime activation receipt journal summary handoff JSON should parse",
    );
    assert_eq!(
        empty_notification_stack_summary_product_handoff_delivery_package_embed_runtime_activation_receipt_journal_summary_handoff_payload
            ["runtimeActivationReceiptJournalSummaryHandoffStatus"],
        "deferred"
    );
    assert_eq!(
        empty_notification_stack_summary_product_handoff_delivery_package_embed_runtime_activation_receipt_journal_summary_handoff_payload
            ["runtimeActivationReceiptJournalSummaryHandoffOutcome"],
        "blocked"
    );
    assert_eq!(
        empty_notification_stack_summary_product_handoff_delivery_package_embed_runtime_activation_receipt_journal_summary_handoff_payload
            ["runtimeActivationReceiptJournalSummaryHandoffAction"],
        "defer-runtime-activation-receipt-journal-summary-handoff"
    );
    assert_eq!(
        empty_notification_stack_summary_product_handoff_delivery_package_embed_runtime_activation_receipt_journal_summary_handoff_payload
            ["shouldPublishSummaryHandoff"],
        false
    );
    assert_eq!(
        empty_notification_stack_summary_product_handoff_delivery_package_embed_runtime_activation_receipt_journal_summary_handoff_payload
            ["shouldDeferSummaryHandoff"],
        true
    );
    assert_eq!(
        empty_notification_stack_summary_product_handoff_delivery_package_embed_runtime_activation_receipt_journal_summary_handoff_payload
            ["embedRuntimeActivationReceiptJournalSummary"]["runtimeActivationReceiptJournalSummaryStatus"],
        "deferred"
    );
    assert_eq!(
        empty_notification_stack_summary_product_handoff_delivery_package_embed_runtime_activation_receipt_journal_summary_handoff_payload
            ["dispatchQueueLaneTabPanelCardActionMenuGroupShortcutCommandPaletteSearchInvocationReceiptNotificationStackSummaryProductHandoffDeliveryPackageEmbedRuntimeActivationReceiptJournalSummaryHandoffCapabilityId"],
        "app-shell-dashboard-dispatch-queue-lane-tab-panel-card-action-menu-group-shortcut-command-palette-search-invocation-receipt-notification-stack-summary-product-handoff-delivery-package-embed-runtime-activation-receipt-journal-summary-handoff-json"
    );

    let empty_notification_stack_summary_product_handoff_delivery_package_embed_runtime_activation_receipt_journal_summary_handoff_receipt_payload: serde_json::Value = serde_json::from_str(
        &app.app_shell_dashboard_dispatch_queue_lane_tab_panel_card_action_menu_group_shortcut_command_palette_search_invocation_receipt_notification_stack_summary_product_handoff_delivery_package_embed_runtime_activation_receipt_journal_summary_handoff_receipt_json(
            BerkeleyAppPersistedEditorState {
                selected_syntax_card_index: Some(2),
                active_command_id: Some("analysis.2.run".to_string()),
            },
            "missing",
            None,
        ),
    )
    .expect(
        "blocked shell dashboard dispatch queue lane tab panel card action menu group shortcut command palette empty search invocation receipt notification stack summary product handoff delivery package embed runtime activation receipt journal summary handoff receipt JSON should parse",
    );
    assert_eq!(
        empty_notification_stack_summary_product_handoff_delivery_package_embed_runtime_activation_receipt_journal_summary_handoff_receipt_payload
            ["runtimeActivationReceiptJournalSummaryHandoffReceiptStatus"],
        "deferred"
    );
    assert_eq!(
        empty_notification_stack_summary_product_handoff_delivery_package_embed_runtime_activation_receipt_journal_summary_handoff_receipt_payload
            ["runtimeActivationReceiptJournalSummaryHandoffReceiptOutcome"],
        "blocked"
    );
    assert_eq!(
        empty_notification_stack_summary_product_handoff_delivery_package_embed_runtime_activation_receipt_journal_summary_handoff_receipt_payload
            ["runtimeActivationReceiptJournalSummaryHandoffReceiptAction"],
        "defer-runtime-activation-receipt-journal-summary-handoff-receipt"
    );
    assert_eq!(
        empty_notification_stack_summary_product_handoff_delivery_package_embed_runtime_activation_receipt_journal_summary_handoff_receipt_payload
            ["shouldAcknowledgeSummaryHandoffReceipt"],
        false
    );
    assert_eq!(
        empty_notification_stack_summary_product_handoff_delivery_package_embed_runtime_activation_receipt_journal_summary_handoff_receipt_payload
            ["shouldDeferSummaryHandoffReceipt"],
        true
    );
    assert_eq!(
        empty_notification_stack_summary_product_handoff_delivery_package_embed_runtime_activation_receipt_journal_summary_handoff_receipt_payload
            ["embedRuntimeActivationReceiptJournalSummaryHandoff"]["runtimeActivationReceiptJournalSummaryHandoffStatus"],
        "deferred"
    );
    assert_eq!(
        empty_notification_stack_summary_product_handoff_delivery_package_embed_runtime_activation_receipt_journal_summary_handoff_receipt_payload
            ["dispatchQueueLaneTabPanelCardActionMenuGroupShortcutCommandPaletteSearchInvocationReceiptNotificationStackSummaryProductHandoffDeliveryPackageEmbedRuntimeActivationReceiptJournalSummaryHandoffReceiptCapabilityId"],
        "app-shell-dashboard-dispatch-queue-lane-tab-panel-card-action-menu-group-shortcut-command-palette-search-invocation-receipt-notification-stack-summary-product-handoff-delivery-package-embed-runtime-activation-receipt-journal-summary-handoff-receipt-json"
    );

    let empty_notification_stack_summary_product_handoff_delivery_package_embed_runtime_activation_receipt_journal_summary_handoff_receipt_acknowledgement_payload: serde_json::Value = serde_json::from_str(
        &app.app_shell_dashboard_dispatch_queue_lane_tab_panel_card_action_menu_group_shortcut_command_palette_search_invocation_receipt_notification_stack_summary_product_handoff_delivery_package_embed_runtime_activation_receipt_journal_summary_handoff_receipt_acknowledgement_json(
            BerkeleyAppPersistedEditorState {
                selected_syntax_card_index: Some(2),
                active_command_id: Some("analysis.2.run".to_string()),
            },
            "missing",
            None,
        ),
    )
    .expect(
        "blocked shell dashboard dispatch queue lane tab panel card action menu group shortcut command palette empty search invocation receipt notification stack summary product handoff delivery package embed runtime activation receipt journal summary handoff receipt acknowledgement JSON should parse",
    );
    assert_eq!(
        empty_notification_stack_summary_product_handoff_delivery_package_embed_runtime_activation_receipt_journal_summary_handoff_receipt_acknowledgement_payload
            ["runtimeActivationReceiptJournalSummaryHandoffReceiptAcknowledgementStatus"],
        "deferred"
    );
    assert_eq!(
        empty_notification_stack_summary_product_handoff_delivery_package_embed_runtime_activation_receipt_journal_summary_handoff_receipt_acknowledgement_payload
            ["runtimeActivationReceiptJournalSummaryHandoffReceiptAcknowledgementOutcome"],
        "blocked"
    );
    assert_eq!(
        empty_notification_stack_summary_product_handoff_delivery_package_embed_runtime_activation_receipt_journal_summary_handoff_receipt_acknowledgement_payload
            ["runtimeActivationReceiptJournalSummaryHandoffReceiptAcknowledgementAction"],
        "defer-runtime-activation-receipt-journal-summary-handoff-receipt-acknowledgement"
    );
    assert_eq!(
        empty_notification_stack_summary_product_handoff_delivery_package_embed_runtime_activation_receipt_journal_summary_handoff_receipt_acknowledgement_payload
            ["shouldAcknowledgeSummaryHandoffReceiptAcknowledgement"],
        false
    );
    assert_eq!(
        empty_notification_stack_summary_product_handoff_delivery_package_embed_runtime_activation_receipt_journal_summary_handoff_receipt_acknowledgement_payload
            ["shouldDeferSummaryHandoffReceiptAcknowledgement"],
        true
    );
    assert_eq!(
        empty_notification_stack_summary_product_handoff_delivery_package_embed_runtime_activation_receipt_journal_summary_handoff_receipt_acknowledgement_payload
            ["embedRuntimeActivationReceiptJournalSummaryHandoffReceipt"]["runtimeActivationReceiptJournalSummaryHandoffReceiptStatus"],
        "deferred"
    );
    assert_eq!(
        empty_notification_stack_summary_product_handoff_delivery_package_embed_runtime_activation_receipt_journal_summary_handoff_receipt_acknowledgement_payload
            ["dispatchQueueLaneTabPanelCardActionMenuGroupShortcutCommandPaletteSearchInvocationReceiptNotificationStackSummaryProductHandoffDeliveryPackageEmbedRuntimeActivationReceiptJournalSummaryHandoffReceiptAcknowledgementCapabilityId"],
        "app-shell-dashboard-dispatch-queue-lane-tab-panel-card-action-menu-group-shortcut-command-palette-search-invocation-receipt-notification-stack-summary-product-handoff-delivery-package-embed-runtime-activation-receipt-journal-summary-handoff-receipt-acknowledgement-json"
    );

    let empty_notification_stack_summary_product_handoff_delivery_package_embed_runtime_activation_receipt_journal_summary_handoff_receipt_acknowledgement_record_payload: serde_json::Value = serde_json::from_str(
        &app.app_shell_dashboard_dispatch_queue_lane_tab_panel_card_action_menu_group_shortcut_command_palette_search_invocation_receipt_notification_stack_summary_product_handoff_delivery_package_embed_runtime_activation_receipt_journal_summary_handoff_receipt_acknowledgement_record_json(
            BerkeleyAppPersistedEditorState {
                selected_syntax_card_index: Some(2),
                active_command_id: Some("analysis.2.run".to_string()),
            },
            "missing",
            None,
        ),
    )
    .expect(
        "blocked shell dashboard dispatch queue lane tab panel card action menu group shortcut command palette empty search invocation receipt notification stack summary product handoff delivery package embed runtime activation receipt journal summary handoff receipt acknowledgement record JSON should parse",
    );
    assert_eq!(
        empty_notification_stack_summary_product_handoff_delivery_package_embed_runtime_activation_receipt_journal_summary_handoff_receipt_acknowledgement_record_payload
            ["runtimeActivationReceiptJournalSummaryHandoffReceiptAcknowledgementRecordStatus"],
        "deferred"
    );
    assert_eq!(
        empty_notification_stack_summary_product_handoff_delivery_package_embed_runtime_activation_receipt_journal_summary_handoff_receipt_acknowledgement_record_payload
            ["runtimeActivationReceiptJournalSummaryHandoffReceiptAcknowledgementRecordOutcome"],
        "blocked"
    );
    assert_eq!(
        empty_notification_stack_summary_product_handoff_delivery_package_embed_runtime_activation_receipt_journal_summary_handoff_receipt_acknowledgement_record_payload
            ["runtimeActivationReceiptJournalSummaryHandoffReceiptAcknowledgementRecordAction"],
        "defer-runtime-activation-receipt-journal-summary-handoff-receipt-acknowledgement-record"
    );
    assert_eq!(
        empty_notification_stack_summary_product_handoff_delivery_package_embed_runtime_activation_receipt_journal_summary_handoff_receipt_acknowledgement_record_payload
            ["shouldRecordSummaryHandoffReceiptAcknowledgement"],
        false
    );
    assert_eq!(
        empty_notification_stack_summary_product_handoff_delivery_package_embed_runtime_activation_receipt_journal_summary_handoff_receipt_acknowledgement_record_payload
            ["shouldDeferSummaryHandoffReceiptAcknowledgementRecord"],
        true
    );
    assert_eq!(
        empty_notification_stack_summary_product_handoff_delivery_package_embed_runtime_activation_receipt_journal_summary_handoff_receipt_acknowledgement_record_payload
            ["embedRuntimeActivationReceiptJournalSummaryHandoffReceiptAcknowledgement"]["runtimeActivationReceiptJournalSummaryHandoffReceiptAcknowledgementStatus"],
        "deferred"
    );
    assert_eq!(
        empty_notification_stack_summary_product_handoff_delivery_package_embed_runtime_activation_receipt_journal_summary_handoff_receipt_acknowledgement_record_payload
            ["dispatchQueueLaneTabPanelCardActionMenuGroupShortcutCommandPaletteSearchInvocationReceiptNotificationStackSummaryProductHandoffDeliveryPackageEmbedRuntimeActivationReceiptJournalSummaryHandoffReceiptAcknowledgementRecordCapabilityId"],
        "app-shell-dashboard-dispatch-queue-lane-tab-panel-card-action-menu-group-shortcut-command-palette-search-invocation-receipt-notification-stack-summary-product-handoff-delivery-package-embed-runtime-activation-receipt-journal-summary-handoff-receipt-acknowledgement-record-json"
    );

    let empty_notification_stack_summary_product_handoff_delivery_package_embed_runtime_activation_receipt_journal_summary_handoff_receipt_acknowledgement_record_receipt_payload: serde_json::Value = serde_json::from_str(
        &app.app_shell_dashboard_dispatch_queue_lane_tab_panel_card_action_menu_group_shortcut_command_palette_search_invocation_receipt_notification_stack_summary_product_handoff_delivery_package_embed_runtime_activation_receipt_journal_summary_handoff_receipt_acknowledgement_record_receipt_json(
            BerkeleyAppPersistedEditorState {
                selected_syntax_card_index: Some(2),
                active_command_id: Some("analysis.2.run".to_string()),
            },
            "missing",
            None,
        ),
    )
    .expect(
        "blocked shell dashboard dispatch queue lane tab panel card action menu group shortcut command palette empty search invocation receipt notification stack summary product handoff delivery package embed runtime activation receipt journal summary handoff receipt acknowledgement record receipt JSON should parse",
    );
    assert_eq!(
        empty_notification_stack_summary_product_handoff_delivery_package_embed_runtime_activation_receipt_journal_summary_handoff_receipt_acknowledgement_record_receipt_payload
            ["runtimeActivationReceiptJournalSummaryHandoffReceiptAcknowledgementRecordReceiptStatus"],
        "deferred"
    );
    assert_eq!(
        empty_notification_stack_summary_product_handoff_delivery_package_embed_runtime_activation_receipt_journal_summary_handoff_receipt_acknowledgement_record_receipt_payload
            ["runtimeActivationReceiptJournalSummaryHandoffReceiptAcknowledgementRecordReceiptOutcome"],
        "blocked"
    );
    assert_eq!(
        empty_notification_stack_summary_product_handoff_delivery_package_embed_runtime_activation_receipt_journal_summary_handoff_receipt_acknowledgement_record_receipt_payload
            ["runtimeActivationReceiptJournalSummaryHandoffReceiptAcknowledgementRecordReceiptAction"],
        "defer-runtime-activation-receipt-journal-summary-handoff-receipt-acknowledgement-record-receipt"
    );
    assert_eq!(
        empty_notification_stack_summary_product_handoff_delivery_package_embed_runtime_activation_receipt_journal_summary_handoff_receipt_acknowledgement_record_receipt_payload
            ["shouldAcknowledgeSummaryHandoffReceiptAcknowledgementRecordReceipt"],
        false
    );
    assert_eq!(
        empty_notification_stack_summary_product_handoff_delivery_package_embed_runtime_activation_receipt_journal_summary_handoff_receipt_acknowledgement_record_receipt_payload
            ["shouldDeferSummaryHandoffReceiptAcknowledgementRecordReceipt"],
        true
    );
    assert_eq!(
        empty_notification_stack_summary_product_handoff_delivery_package_embed_runtime_activation_receipt_journal_summary_handoff_receipt_acknowledgement_record_receipt_payload
            ["embedRuntimeActivationReceiptJournalSummaryHandoffReceiptAcknowledgementRecord"]["runtimeActivationReceiptJournalSummaryHandoffReceiptAcknowledgementRecordStatus"],
        "deferred"
    );
    assert_eq!(
        empty_notification_stack_summary_product_handoff_delivery_package_embed_runtime_activation_receipt_journal_summary_handoff_receipt_acknowledgement_record_receipt_payload
            ["dispatchQueueLaneTabPanelCardActionMenuGroupShortcutCommandPaletteSearchInvocationReceiptNotificationStackSummaryProductHandoffDeliveryPackageEmbedRuntimeActivationReceiptJournalSummaryHandoffReceiptAcknowledgementRecordReceiptCapabilityId"],
        "app-shell-dashboard-dispatch-queue-lane-tab-panel-card-action-menu-group-shortcut-command-palette-search-invocation-receipt-notification-stack-summary-product-handoff-delivery-package-embed-runtime-activation-receipt-journal-summary-handoff-receipt-acknowledgement-record-receipt-json"
    );

    let empty_notification_stack_summary_product_handoff_delivery_package_embed_runtime_activation_receipt_journal_summary_handoff_receipt_acknowledgement_record_receipt_acknowledgement_payload: serde_json::Value = serde_json::from_str(
        &app.app_shell_dashboard_dispatch_queue_lane_tab_panel_card_action_menu_group_shortcut_command_palette_search_invocation_receipt_notification_stack_summary_product_handoff_delivery_package_embed_runtime_activation_receipt_journal_summary_handoff_receipt_acknowledgement_record_receipt_acknowledgement_json(
            BerkeleyAppPersistedEditorState {
                selected_syntax_card_index: Some(2),
                active_command_id: Some("analysis.2.run".to_string()),
            },
            "missing",
            None,
        ),
    )
    .expect(
        "blocked shell dashboard dispatch queue lane tab panel card action menu group shortcut command palette empty search invocation receipt notification stack summary product handoff delivery package embed runtime activation receipt journal summary handoff receipt acknowledgement record receipt acknowledgement JSON should parse",
    );
    assert_eq!(
        empty_notification_stack_summary_product_handoff_delivery_package_embed_runtime_activation_receipt_journal_summary_handoff_receipt_acknowledgement_record_receipt_acknowledgement_payload
            ["runtimeActivationReceiptJournalSummaryHandoffReceiptAcknowledgementRecordReceiptAcknowledgementStatus"],
        "deferred"
    );
    assert_eq!(
        empty_notification_stack_summary_product_handoff_delivery_package_embed_runtime_activation_receipt_journal_summary_handoff_receipt_acknowledgement_record_receipt_acknowledgement_payload
            ["runtimeActivationReceiptJournalSummaryHandoffReceiptAcknowledgementRecordReceiptAcknowledgementOutcome"],
        "blocked"
    );
    assert_eq!(
        empty_notification_stack_summary_product_handoff_delivery_package_embed_runtime_activation_receipt_journal_summary_handoff_receipt_acknowledgement_record_receipt_acknowledgement_payload
            ["runtimeActivationReceiptJournalSummaryHandoffReceiptAcknowledgementRecordReceiptAcknowledgementAction"],
        "defer-runtime-activation-receipt-journal-summary-handoff-receipt-acknowledgement-record-receipt-acknowledgement"
    );
    assert_eq!(
        empty_notification_stack_summary_product_handoff_delivery_package_embed_runtime_activation_receipt_journal_summary_handoff_receipt_acknowledgement_record_receipt_acknowledgement_payload
            ["shouldAcknowledgeSummaryHandoffReceiptAcknowledgementRecordReceiptAcknowledgement"],
        false
    );
    assert_eq!(
        empty_notification_stack_summary_product_handoff_delivery_package_embed_runtime_activation_receipt_journal_summary_handoff_receipt_acknowledgement_record_receipt_acknowledgement_payload
            ["shouldDeferSummaryHandoffReceiptAcknowledgementRecordReceiptAcknowledgement"],
        true
    );
    assert_eq!(
        empty_notification_stack_summary_product_handoff_delivery_package_embed_runtime_activation_receipt_journal_summary_handoff_receipt_acknowledgement_record_receipt_acknowledgement_payload
            ["embedRuntimeActivationReceiptJournalSummaryHandoffReceiptAcknowledgementRecordReceipt"]["runtimeActivationReceiptJournalSummaryHandoffReceiptAcknowledgementRecordReceiptStatus"],
        "deferred"
    );
    assert_eq!(
        empty_notification_stack_summary_product_handoff_delivery_package_embed_runtime_activation_receipt_journal_summary_handoff_receipt_acknowledgement_record_receipt_acknowledgement_payload
            ["dispatchQueueLaneTabPanelCardActionMenuGroupShortcutCommandPaletteSearchInvocationReceiptNotificationStackSummaryProductHandoffDeliveryPackageEmbedRuntimeActivationReceiptJournalSummaryHandoffReceiptAcknowledgementRecordReceiptAcknowledgementCapabilityId"],
        "app-shell-dashboard-dispatch-queue-lane-tab-panel-card-action-menu-group-shortcut-command-palette-search-invocation-receipt-notification-stack-summary-product-handoff-delivery-package-embed-runtime-activation-receipt-journal-summary-handoff-receipt-acknowledgement-record-receipt-acknowledgement-json"
    );

    let empty_notification_stack_summary_product_handoff_delivery_package_embed_runtime_activation_receipt_journal_summary_handoff_receipt_acknowledgement_record_receipt_acknowledgement_record_payload: serde_json::Value = serde_json::from_str(
        &app.app_shell_dashboard_dispatch_queue_lane_tab_panel_card_action_menu_group_shortcut_command_palette_search_invocation_receipt_notification_stack_summary_product_handoff_delivery_package_embed_runtime_activation_receipt_journal_summary_handoff_receipt_acknowledgement_record_receipt_acknowledgement_record_json(
            BerkeleyAppPersistedEditorState {
                selected_syntax_card_index: Some(2),
                active_command_id: Some("analysis.2.run".to_string()),
            },
            "missing",
            None,
        ),
    )
    .expect(
        "blocked shell dashboard dispatch queue lane tab panel card action menu group shortcut command palette empty search invocation receipt notification stack summary product handoff delivery package embed runtime activation receipt journal summary handoff receipt acknowledgement record receipt acknowledgement record JSON should parse",
    );
    assert_eq!(
        empty_notification_stack_summary_product_handoff_delivery_package_embed_runtime_activation_receipt_journal_summary_handoff_receipt_acknowledgement_record_receipt_acknowledgement_record_payload
            ["runtimeActivationReceiptJournalSummaryHandoffReceiptAcknowledgementRecordReceiptAcknowledgementRecordStatus"],
        "deferred"
    );
    assert_eq!(
        empty_notification_stack_summary_product_handoff_delivery_package_embed_runtime_activation_receipt_journal_summary_handoff_receipt_acknowledgement_record_receipt_acknowledgement_record_payload
            ["runtimeActivationReceiptJournalSummaryHandoffReceiptAcknowledgementRecordReceiptAcknowledgementRecordOutcome"],
        "blocked"
    );
    assert_eq!(
        empty_notification_stack_summary_product_handoff_delivery_package_embed_runtime_activation_receipt_journal_summary_handoff_receipt_acknowledgement_record_receipt_acknowledgement_record_payload
            ["runtimeActivationReceiptJournalSummaryHandoffReceiptAcknowledgementRecordReceiptAcknowledgementRecordAction"],
        "defer-runtime-activation-receipt-journal-summary-handoff-receipt-acknowledgement-record-receipt-acknowledgement-record"
    );
    assert_eq!(
        empty_notification_stack_summary_product_handoff_delivery_package_embed_runtime_activation_receipt_journal_summary_handoff_receipt_acknowledgement_record_receipt_acknowledgement_record_payload
            ["shouldRecordSummaryHandoffReceiptAcknowledgementRecordReceiptAcknowledgement"],
        false
    );
    assert_eq!(
        empty_notification_stack_summary_product_handoff_delivery_package_embed_runtime_activation_receipt_journal_summary_handoff_receipt_acknowledgement_record_receipt_acknowledgement_record_payload
            ["shouldDeferSummaryHandoffReceiptAcknowledgementRecordReceiptAcknowledgementRecord"],
        true
    );
    assert_eq!(
        empty_notification_stack_summary_product_handoff_delivery_package_embed_runtime_activation_receipt_journal_summary_handoff_receipt_acknowledgement_record_receipt_acknowledgement_record_payload
            ["embedRuntimeActivationReceiptJournalSummaryHandoffReceiptAcknowledgementRecordReceiptAcknowledgement"]["runtimeActivationReceiptJournalSummaryHandoffReceiptAcknowledgementRecordReceiptAcknowledgementStatus"],
        "deferred"
    );
    assert_eq!(
        empty_notification_stack_summary_product_handoff_delivery_package_embed_runtime_activation_receipt_journal_summary_handoff_receipt_acknowledgement_record_receipt_acknowledgement_record_payload
            ["dispatchQueueLaneTabPanelCardActionMenuGroupShortcutCommandPaletteSearchInvocationReceiptNotificationStackSummaryProductHandoffDeliveryPackageEmbedRuntimeActivationReceiptJournalSummaryHandoffReceiptAcknowledgementRecordReceiptAcknowledgementRecordCapabilityId"],
        "app-shell-dashboard-dispatch-queue-lane-tab-panel-card-action-menu-group-shortcut-command-palette-search-invocation-receipt-notification-stack-summary-product-handoff-delivery-package-embed-runtime-activation-receipt-journal-summary-handoff-receipt-acknowledgement-record-receipt-acknowledgement-record-json"
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
