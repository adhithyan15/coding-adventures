use spice_engine::{
    sens_dc, Cccs, Circuit, CurrentSource, Element, Resistor, SensResult, SpiceError, Vccs, Vcvs,
    VoltageSource,
};

fn assert_close(actual: f64, expected: f64) {
    assert!(
        (actual - expected).abs() < 1.0e-6,
        "expected {expected}, got {actual}"
    );
}

fn entry<'a>(
    result: &'a SensResult,
    element_name: &str,
    parameter: &str,
) -> &'a spice_engine::SensEntry {
    result
        .entry(element_name, parameter)
        .unwrap_or_else(|| panic!("missing sensitivity entry for {element_name}.{parameter}"))
}

#[test]
fn sens_dc_reports_divider_source_and_resistor_sensitivities() {
    let mut circuit = Circuit::new();
    circuit.add(Element::VoltageSource(VoltageSource::new(
        "Vin", "vin", "0", 10.0,
    )));
    circuit.add(Element::Resistor(Resistor::new(
        "Rtop", "vin", "out", 1_000.0,
    )));
    circuit.add(Element::Resistor(Resistor::new(
        "Rbot", "out", "0", 1_000.0,
    )));

    let result = sens_dc(&circuit, "out").unwrap();

    assert_eq!(result.output_node, "out");
    assert_close(result.nominal_voltage, 5.0);
    assert_eq!(result.entries.len(), 3);
    assert_close(entry(&result, "Vin", "voltage").sensitivity, 0.5);
    assert_close(entry(&result, "Vin", "voltage").relative_sensitivity, 1.0);
    assert_close(
        entry(&result, "Rtop", "resistance_ohms").sensitivity,
        -0.0025,
    );
    assert_close(
        entry(&result, "Rtop", "resistance_ohms").relative_sensitivity,
        -0.5,
    );
    assert_close(
        entry(&result, "Rbot", "resistance_ohms").sensitivity,
        0.0025,
    );
    assert_close(
        entry(&result, "Rbot", "resistance_ohms").relative_sensitivity,
        0.5,
    );
}

#[test]
fn sens_dc_reports_current_source_and_load_resistance_sensitivities() {
    let mut circuit = Circuit::new();
    circuit.add(Element::CurrentSource(CurrentSource::new(
        "Iin", "0", "out", 1.0e-3,
    )));
    circuit.add(Element::Resistor(Resistor::new(
        "Rload", "out", "0", 1_000.0,
    )));

    let result = sens_dc(&circuit, "out").unwrap();

    assert_close(result.nominal_voltage, 1.0);
    assert_close(entry(&result, "Iin", "current").sensitivity, 1_000.0);
    assert_close(entry(&result, "Iin", "current").relative_sensitivity, 1.0);
    assert_close(
        entry(&result, "Rload", "resistance_ohms").sensitivity,
        1.0e-3,
    );
    assert_close(
        entry(&result, "Rload", "resistance_ohms").relative_sensitivity,
        1.0,
    );
}

#[test]
fn sens_dc_reports_vccs_transconductance_sensitivity() {
    let mut circuit = Circuit::new();
    circuit.add(Element::VoltageSource(VoltageSource::new(
        "Vin", "in", "0", 1.0,
    )));
    circuit.add(Element::Vccs(Vccs::new(
        "Gm", "0", "out", "in", "0", 2.0e-3,
    )));
    circuit.add(Element::Resistor(Resistor::new(
        "Rload", "out", "0", 1_000.0,
    )));

    let result = sens_dc(&circuit, "out").unwrap();

    assert_close(result.nominal_voltage, 2.0);
    assert_close(
        entry(&result, "Gm", "transconductance_siemens").sensitivity,
        1_000.0,
    );
    assert_close(
        entry(&result, "Gm", "transconductance_siemens").relative_sensitivity,
        1.0,
    );
}

#[test]
fn sens_dc_reports_vcvs_gain_sensitivity() {
    let mut circuit = Circuit::new();
    circuit.add(Element::VoltageSource(VoltageSource::new(
        "Vin", "in", "0", 1.0,
    )));
    circuit.add(Element::Vcvs(Vcvs::new("E1", "out", "0", "in", "0", 3.0)));
    circuit.add(Element::Resistor(Resistor::new(
        "Rload", "out", "0", 1_000.0,
    )));

    let result = sens_dc(&circuit, "out").unwrap();

    assert_close(result.nominal_voltage, 3.0);
    assert_close(entry(&result, "E1", "gain").sensitivity, 1.0);
    assert_close(entry(&result, "E1", "gain").relative_sensitivity, 1.0);
}

#[test]
fn sens_dc_reports_cccs_gain_sensitivity() {
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
    circuit.add(Element::Cccs(Cccs::new("F1", "0", "out", "Vsense", 2.0)));
    circuit.add(Element::Resistor(Resistor::new(
        "Rload", "out", "0", 1_000.0,
    )));

    let result = sens_dc(&circuit, "out").unwrap();

    assert_close(result.nominal_voltage, 2.0);
    assert_close(entry(&result, "F1", "gain").sensitivity, 1.0);
    assert_close(entry(&result, "F1", "gain").relative_sensitivity, 1.0);
}

#[test]
fn sens_dc_sorts_entries_by_absolute_relative_sensitivity() {
    let mut circuit = Circuit::new();
    circuit.add(Element::VoltageSource(VoltageSource::new(
        "Vin", "vin", "0", 10.0,
    )));
    circuit.add(Element::Resistor(Resistor::new(
        "Rtop", "vin", "out", 1_000.0,
    )));
    circuit.add(Element::Resistor(Resistor::new(
        "Rbot", "out", "0", 1_000.0,
    )));

    let result = sens_dc(&circuit, "out").unwrap();

    assert_eq!(result.entries[0].element_name, "Vin");
    assert!(
        result.entries[0].relative_sensitivity.abs()
            >= result.entries[1].relative_sensitivity.abs()
    );
}

#[test]
fn sens_dc_rejects_missing_output_node() {
    let circuit = Circuit::new();

    assert!(matches!(
        sens_dc(&circuit, "missing"),
        Err(SpiceError::InvalidElement { name, .. }) if name == "missing"
    ));
}
