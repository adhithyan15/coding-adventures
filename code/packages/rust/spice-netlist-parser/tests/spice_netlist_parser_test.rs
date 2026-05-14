use spice_engine::{dc_op, Element};
use spice_netlist_parser::{
    parse_netlist, parse_value, AcAnalysis, Analysis, DcAnalysis, NetlistParseError, OpAnalysis,
    TranAnalysis,
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
Cload out 0 10p
L1 out 0 1u
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
fn parses_engineering_suffixes() {
    assert_eq!(parse_value("1k").unwrap(), 1.0e3);
    assert_eq!(parse_value("2.2meg").unwrap(), 2.2e6);
    assert_eq!(parse_value("3u").unwrap(), 3.0e-6);
    assert_eq!(parse_value("4n").unwrap(), 4.0e-9);
}

#[test]
fn rejects_unsupported_elements_with_line_numbers() {
    let err = parse_netlist("\nD1 a 0 diode\n").unwrap_err();

    assert!(err.to_string().contains("line 2: unsupported element"));
    assert_error_type(err);
}

#[test]
fn rejects_unbalanced_waveform_parentheses() {
    let err = parse_netlist("V1 in 0 PULSE(0 1\n").unwrap_err();

    assert!(err.to_string().contains("unclosed parenthesis"));
}

fn assert_error_type(_: NetlistParseError) {}
