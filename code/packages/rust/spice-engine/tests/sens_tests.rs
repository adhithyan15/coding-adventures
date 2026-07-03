use spice_engine::{
    format_corner_sens_table, format_sens_table, sens_dc, sens_dc_corners,
    sens_dc_corners_parallel, Cccs, Ccvs, Circuit, CornerOverride, CornerSpec, CurrentSource,
    Element, Resistor, SensResult, SpiceError, Vccs, Vcvs, VoltageSource,
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
fn sens_text_output_table_is_stable() {
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

    assert_eq!(
        format_sens_table(&result),
        "OutputNode\tNominalVoltage\tElement\tParameter\tNominalValue\tSensitivity\tRelativeSensitivity\n\
out\t5.000000e+00\tVin\tvoltage\t1.000000e+01\t5.000000e-01\t1.000000e+00\n\
out\t5.000000e+00\tRbot\tresistance_ohms\t1.000000e+03\t2.499999e-03\t4.999998e-01\n\
out\t5.000000e+00\tRtop\tresistance_ohms\t1.000000e+03\t-2.499999e-03\t-4.999998e-01\n"
    );
}

#[test]
fn sens_dc_corners_runs_analysis_per_corner() {
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

    let result = sens_dc_corners(
        &circuit,
        "out",
        &[
            CornerSpec::new("nominal", Vec::new()),
            CornerSpec::new(
                "rbot-fast",
                vec![CornerOverride::new("Rbot", "resistance", 500.0)],
            ),
            CornerSpec::new(
                "vin-high",
                vec![CornerOverride::new("Vin", "voltage", 12.0)],
            ),
        ],
    )
    .unwrap();

    assert_eq!(result.output_node, "out");
    assert_eq!(result.points.len(), 3);
    assert_eq!(result.points[0].corner_name, "nominal");
    assert_eq!(result.points[1].corner_name, "rbot-fast");
    assert_eq!(result.points[2].corner_name, "vin-high");
    assert_close(result.points[0].result.nominal_voltage, 5.0);
    assert_close(result.points[1].result.nominal_voltage, 10.0 / 3.0);
    assert_close(result.points[2].result.nominal_voltage, 6.0);
    assert_close(
        entry(&result.points[0].result, "Rbot", "resistance_ohms").sensitivity,
        0.0025,
    );
    assert_close(
        entry(&result.points[1].result, "Rbot", "resistance_ohms").nominal_value,
        500.0,
    );
    assert_close(
        entry(&result.points[2].result, "Vin", "voltage").sensitivity,
        0.5,
    );
}

#[test]
fn corner_sens_text_output_table_is_stable() {
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

    let result = sens_dc_corners(
        &circuit,
        "out",
        &[
            CornerSpec::new("nominal", Vec::new()),
            CornerSpec::new(
                "rbot-fast",
                vec![CornerOverride::new("Rbot", "resistance", 500.0)],
            ),
        ],
    )
    .unwrap();

    assert_eq!(
        format_corner_sens_table(&result),
        "Corner\tOutputNode\tNominalVoltage\tElement\tParameter\tNominalValue\tSensitivity\tRelativeSensitivity\n\
nominal\tout\t5.000000e+00\tVin\tvoltage\t1.000000e+01\t5.000000e-01\t1.000000e+00\n\
nominal\tout\t5.000000e+00\tRbot\tresistance_ohms\t1.000000e+03\t2.499999e-03\t4.999998e-01\n\
nominal\tout\t5.000000e+00\tRtop\tresistance_ohms\t1.000000e+03\t-2.499999e-03\t-4.999998e-01\n\
rbot-fast\tout\t3.333333e+00\tVin\tvoltage\t1.000000e+01\t3.333333e-01\t1.000000e+00\n\
rbot-fast\tout\t3.333333e+00\tRbot\tresistance_ohms\t5.000000e+02\t4.444443e-03\t6.666664e-01\n\
rbot-fast\tout\t3.333333e+00\tRtop\tresistance_ohms\t1.000000e+03\t-2.222221e-03\t-6.666662e-01\n"
    );
}

#[test]
fn sens_dc_corners_parallel_matches_ordered_sequential_results() {
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
    let corners = [
        CornerSpec::new("nominal", Vec::new()),
        CornerSpec::new(
            "rbot-fast",
            vec![CornerOverride::new("Rbot", "resistance", 500.0)],
        ),
        CornerSpec::new(
            "vin-high",
            vec![CornerOverride::new("Vin", "voltage", 12.0)],
        ),
    ];

    let sequential = sens_dc_corners(&circuit, "out", &corners).unwrap();
    let parallel = sens_dc_corners_parallel(&circuit, "out", &corners).unwrap();

    assert_eq!(parallel.output_node, sequential.output_node);
    assert_eq!(parallel.points.len(), sequential.points.len());
    for (parallel_corner, sequential_corner) in parallel.points.iter().zip(sequential.points.iter())
    {
        assert_eq!(parallel_corner.corner_name, sequential_corner.corner_name);
        assert_close(
            parallel_corner.result.nominal_voltage,
            sequential_corner.result.nominal_voltage,
        );
        assert_eq!(
            parallel_corner.result.entries.len(),
            sequential_corner.result.entries.len()
        );
        for (parallel_entry, sequential_entry) in parallel_corner
            .result
            .entries
            .iter()
            .zip(sequential_corner.result.entries.iter())
        {
            assert_eq!(parallel_entry.element_name, sequential_entry.element_name);
            assert_eq!(parallel_entry.parameter, sequential_entry.parameter);
            assert_close(parallel_entry.nominal_value, sequential_entry.nominal_value);
            assert_close(parallel_entry.sensitivity, sequential_entry.sensitivity);
            assert_close(
                parallel_entry.relative_sensitivity,
                sequential_entry.relative_sensitivity,
            );
        }
    }
    assert_eq!(
        format_corner_sens_table(&parallel),
        "Corner\tOutputNode\tNominalVoltage\tElement\tParameter\tNominalValue\tSensitivity\tRelativeSensitivity\n\
nominal\tout\t5.000000e+00\tVin\tvoltage\t1.000000e+01\t5.000000e-01\t1.000000e+00\n\
nominal\tout\t5.000000e+00\tRbot\tresistance_ohms\t1.000000e+03\t2.499999e-03\t4.999998e-01\n\
nominal\tout\t5.000000e+00\tRtop\tresistance_ohms\t1.000000e+03\t-2.499999e-03\t-4.999998e-01\n\
rbot-fast\tout\t3.333333e+00\tVin\tvoltage\t1.000000e+01\t3.333333e-01\t1.000000e+00\n\
rbot-fast\tout\t3.333333e+00\tRbot\tresistance_ohms\t5.000000e+02\t4.444443e-03\t6.666664e-01\n\
rbot-fast\tout\t3.333333e+00\tRtop\tresistance_ohms\t1.000000e+03\t-2.222221e-03\t-6.666662e-01\n\
vin-high\tout\t6.000000e+00\tVin\tvoltage\t1.200000e+01\t5.000000e-01\t1.000000e+00\n\
vin-high\tout\t6.000000e+00\tRbot\tresistance_ohms\t1.000000e+03\t2.999999e-03\t4.999998e-01\n\
vin-high\tout\t6.000000e+00\tRtop\tresistance_ohms\t1.000000e+03\t-2.999998e-03\t-4.999997e-01\n"
    );
}

#[test]
fn sens_dc_corners_parallel_reports_corner_override_errors() {
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
    let corners = [CornerSpec::new(
        "missing",
        vec![CornerOverride::new("Rmissing", "resistance", 500.0)],
    )];

    assert!(matches!(
        sens_dc_corners_parallel(&circuit, "out", &corners),
        Err(SpiceError::InvalidElement { name, reason })
            if name == "dc_corners" && reason.contains("Rmissing")
    ));
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
fn sens_dc_reports_ccvs_transresistance_sensitivity() {
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
        "H1", "out", "0", "Vsense", 2_000.0,
    )));
    circuit.add(Element::Resistor(Resistor::new(
        "Rload", "out", "0", 1_000.0,
    )));

    let result = sens_dc(&circuit, "out").unwrap();

    assert_close(result.nominal_voltage, 2.0);
    assert_close(
        entry(&result, "H1", "transresistance_ohms").sensitivity,
        1.0e-3,
    );
    assert_close(
        entry(&result, "H1", "transresistance_ohms").relative_sensitivity,
        1.0,
    );
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
