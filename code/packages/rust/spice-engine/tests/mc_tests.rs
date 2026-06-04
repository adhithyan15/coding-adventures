use spice_engine::{
    format_corner_mc_table, format_mc_table, mc_dc, mc_dc_corners, mc_dc_corners_parallel, Cccs,
    Ccvs, Circuit, CornerOverride, CornerSpec, CurrentSource, Element, McDistribution, McOptions,
    Resistor, SpiceError, Vccs, Vcvs, VoltageSource,
};

fn assert_close(actual: f64, expected: f64) {
    assert!(
        (actual - expected).abs() < 1.0e-9,
        "expected {expected}, got {actual}"
    );
}

fn divider() -> Circuit {
    let mut circuit = Circuit::new();
    circuit.add(Element::VoltageSource(VoltageSource::new(
        "Vin", "in", "0", 10.0,
    )));
    circuit.add(Element::Resistor(Resistor::new(
        "Rtop", "in", "mid", 1_000.0,
    )));
    circuit.add(Element::Resistor(Resistor::new(
        "Rbot", "mid", "0", 1_000.0,
    )));
    circuit
}

fn mid_voltages(result: &spice_engine::McResult) -> Vec<f64> {
    result
        .points
        .iter()
        .map(|point| point.voltage("mid").unwrap())
        .collect()
}

#[test]
fn mc_dc_returns_trial_points_and_zero_spread_at_zero_tolerance() {
    let result = mc_dc(
        &divider(),
        "mid",
        8,
        McOptions {
            tolerance: 0.0,
            seed: Some(7),
            ..McOptions::default()
        },
    )
    .unwrap();

    assert_eq!(result.output_node, "mid");
    assert_eq!(result.n_trials, 8);
    assert_eq!(result.points.len(), 8);
    assert_eq!(
        result
            .points
            .iter()
            .map(|point| point.trial)
            .collect::<Vec<_>>(),
        vec![0, 1, 2, 3, 4, 5, 6, 7]
    );
    assert!(result.points.iter().all(|point| point.converged));
    assert_close(result.mean, 5.0);
    assert_close(result.std_dev, 0.0);
    for point in &result.points {
        assert_close(point.voltage("mid").unwrap(), 5.0);
        assert_close(point.voltage("0").unwrap(), 0.0);
        assert!(point.branch_current("Vin").is_some());
    }
}

#[test]
fn mc_dc_reproduces_with_same_seed() {
    let options = McOptions {
        tolerance: 0.05,
        distribution: McDistribution::Uniform,
        seed: Some(42),
    };

    let left = mc_dc(&divider(), "mid", 20, options).unwrap();
    let right = mc_dc(&divider(), "mid", 20, options).unwrap();

    assert_eq!(left.mean, right.mean);
    assert_eq!(left.std_dev, right.std_dev);
    assert_eq!(mid_voltages(&left), mid_voltages(&right));
}

#[test]
fn mc_dc_uses_different_seeds_for_different_trial_vectors() {
    let left = mc_dc(
        &divider(),
        "mid",
        20,
        McOptions {
            tolerance: 0.05,
            distribution: McDistribution::Uniform,
            seed: Some(1),
        },
    )
    .unwrap();
    let right = mc_dc(
        &divider(),
        "mid",
        20,
        McOptions {
            tolerance: 0.05,
            distribution: McDistribution::Uniform,
            seed: Some(2),
        },
    )
    .unwrap();

    assert_ne!(mid_voltages(&left), mid_voltages(&right));
}

#[test]
fn mc_dc_reports_spread_near_nominal_divider_voltage() {
    let result = mc_dc(
        &divider(),
        "mid",
        200,
        McOptions {
            tolerance: 0.05,
            seed: Some(3),
            ..McOptions::default()
        },
    )
    .unwrap();

    assert!(result.points.iter().all(|point| point.converged));
    assert!(result.mean > 4.5, "mean was {}", result.mean);
    assert!(result.mean < 5.5, "mean was {}", result.mean);
    assert!(result.std_dev > 0.0, "std_dev was {}", result.std_dev);
}

#[test]
fn mc_dc_corners_runs_trials_per_corner() {
    let result = mc_dc_corners(
        &divider(),
        "mid",
        8,
        McOptions {
            tolerance: 0.0,
            seed: Some(7),
            ..McOptions::default()
        },
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

    assert_eq!(result.output_node, "mid");
    assert_eq!(result.points.len(), 3);
    assert_eq!(result.points[0].corner_name, "nominal");
    assert_eq!(result.points[1].corner_name, "rbot-fast");
    assert_eq!(result.points[2].corner_name, "vin-high");
    assert_eq!(result.points[0].result.n_trials, 8);
    assert_eq!(result.points[1].result.n_trials, 8);
    assert_eq!(result.points[2].result.n_trials, 8);
    assert_close(result.points[0].result.mean, 5.0);
    assert_close(result.points[1].result.mean, 10.0 / 3.0);
    assert_close(result.points[2].result.mean, 6.0);
    assert!(result.points.iter().all(|point| point
        .result
        .points
        .iter()
        .all(|trial| trial.converged)));
}

#[test]
fn mc_dc_text_output_table_is_stable() {
    let result = mc_dc(
        &divider(),
        "mid",
        2,
        McOptions {
            tolerance: 0.0,
            seed: Some(7),
            ..McOptions::default()
        },
    )
    .unwrap();

    assert_eq!(
        format_mc_table(&result),
        "Trial\tOutputNode\tOutputValue\tMean\tStdDev\tConverged\n0\tmid\t5.000000e+00\t5.000000e+00\t0.000000e+00\ttrue\n1\tmid\t5.000000e+00\t5.000000e+00\t0.000000e+00\ttrue\n"
    );
}

#[test]
fn corner_mc_dc_text_output_table_is_stable() {
    let result = mc_dc_corners(
        &divider(),
        "mid",
        2,
        McOptions {
            tolerance: 0.0,
            seed: Some(7),
            ..McOptions::default()
        },
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
        format_corner_mc_table(&result),
        "Corner\tTrial\tOutputNode\tOutputValue\tMean\tStdDev\tConverged\nnominal\t0\tmid\t5.000000e+00\t5.000000e+00\t0.000000e+00\ttrue\nnominal\t1\tmid\t5.000000e+00\t5.000000e+00\t0.000000e+00\ttrue\nrbot-fast\t0\tmid\t3.333333e+00\t3.333333e+00\t0.000000e+00\ttrue\nrbot-fast\t1\tmid\t3.333333e+00\t3.333333e+00\t0.000000e+00\ttrue\n"
    );
}

#[test]
fn mc_dc_corners_parallel_matches_ordered_sequential_results() {
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
    let options = McOptions {
        tolerance: 0.0,
        seed: Some(7),
        ..McOptions::default()
    };

    let sequential = mc_dc_corners(&divider(), "mid", 2, options, &corners).unwrap();
    let parallel = mc_dc_corners_parallel(&divider(), "mid", 2, options, &corners).unwrap();

    assert_eq!(parallel.output_node, sequential.output_node);
    assert_eq!(parallel.points.len(), sequential.points.len());
    for (parallel_corner, sequential_corner) in parallel.points.iter().zip(sequential.points.iter())
    {
        assert_eq!(parallel_corner.corner_name, sequential_corner.corner_name);
        assert_eq!(
            parallel_corner.result.n_trials,
            sequential_corner.result.n_trials
        );
        assert_close(parallel_corner.result.mean, sequential_corner.result.mean);
        assert_close(
            parallel_corner.result.std_dev,
            sequential_corner.result.std_dev,
        );
        assert_eq!(
            parallel_corner.result.points.len(),
            sequential_corner.result.points.len()
        );
        for (parallel_trial, sequential_trial) in parallel_corner
            .result
            .points
            .iter()
            .zip(sequential_corner.result.points.iter())
        {
            assert_eq!(parallel_trial.trial, sequential_trial.trial);
            assert_eq!(parallel_trial.converged, sequential_trial.converged);
            assert_close(
                parallel_trial.voltage("mid").unwrap(),
                sequential_trial.voltage("mid").unwrap(),
            );
            assert_close(
                parallel_trial.branch_current("Vin").unwrap(),
                sequential_trial.branch_current("Vin").unwrap(),
            );
        }
    }
    assert_eq!(
        format_corner_mc_table(&parallel),
        "Corner\tTrial\tOutputNode\tOutputValue\tMean\tStdDev\tConverged\nnominal\t0\tmid\t5.000000e+00\t5.000000e+00\t0.000000e+00\ttrue\nnominal\t1\tmid\t5.000000e+00\t5.000000e+00\t0.000000e+00\ttrue\nrbot-fast\t0\tmid\t3.333333e+00\t3.333333e+00\t0.000000e+00\ttrue\nrbot-fast\t1\tmid\t3.333333e+00\t3.333333e+00\t0.000000e+00\ttrue\nvin-high\t0\tmid\t6.000000e+00\t6.000000e+00\t0.000000e+00\ttrue\nvin-high\t1\tmid\t6.000000e+00\t6.000000e+00\t0.000000e+00\ttrue\n"
    );
}

#[test]
fn mc_dc_corners_parallel_reports_corner_override_errors() {
    let corners = [CornerSpec::new(
        "missing",
        vec![CornerOverride::new("Rmissing", "resistance", 500.0)],
    )];

    assert!(matches!(
        mc_dc_corners_parallel(&divider(), "mid", 2, McOptions::default(), &corners),
        Err(SpiceError::InvalidElement { name, reason })
            if name == "dc_corners" && reason.contains("Rmissing")
    ));
}

#[test]
fn mc_dc_varies_current_source_and_vccs_parameters() {
    let mut circuit = Circuit::new();
    circuit.add(Element::VoltageSource(VoltageSource::new(
        "Vctrl", "ctrl", "0", 1.0,
    )));
    circuit.add(Element::Vccs(Vccs::new(
        "Gm", "0", "out", "ctrl", "0", 1.0e-3,
    )));
    circuit.add(Element::CurrentSource(CurrentSource::new(
        "Ibias", "0", "out", 1.0e-3,
    )));
    circuit.add(Element::Resistor(Resistor::new(
        "Rload", "out", "0", 1_000.0,
    )));

    let result = mc_dc(
        &circuit,
        "out",
        40,
        McOptions {
            tolerance: 0.05,
            distribution: McDistribution::Uniform,
            seed: Some(9),
        },
    )
    .unwrap();

    assert!(result.mean > 1.5, "mean was {}", result.mean);
    assert!(result.mean < 2.5, "mean was {}", result.mean);
    assert!(result.std_dev > 0.0, "std_dev was {}", result.std_dev);
}

#[test]
fn mc_dc_varies_vcvs_gain() {
    let mut circuit = Circuit::new();
    circuit.add(Element::VoltageSource(VoltageSource::new(
        "Vin", "in", "0", 1.0,
    )));
    circuit.add(Element::Vcvs(Vcvs::new("E1", "out", "0", "in", "0", 2.0)));
    circuit.add(Element::Resistor(Resistor::new(
        "Rload", "out", "0", 1_000.0,
    )));

    let result = mc_dc(
        &circuit,
        "out",
        40,
        McOptions {
            tolerance: 0.05,
            distribution: McDistribution::Uniform,
            seed: Some(11),
        },
    )
    .unwrap();

    assert!(result.mean > 1.8, "mean was {}", result.mean);
    assert!(result.mean < 2.2, "mean was {}", result.mean);
    assert!(result.std_dev > 0.0, "std_dev was {}", result.std_dev);
}

#[test]
fn mc_dc_varies_cccs_gain() {
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

    let result = mc_dc(
        &circuit,
        "out",
        40,
        McOptions {
            tolerance: 0.05,
            distribution: McDistribution::Uniform,
            seed: Some(13),
        },
    )
    .unwrap();

    assert!(result.mean > 1.8, "mean was {}", result.mean);
    assert!(result.mean < 2.2, "mean was {}", result.mean);
    assert!(result.std_dev > 0.0, "std_dev was {}", result.std_dev);
}

#[test]
fn mc_dc_varies_ccvs_transresistance() {
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

    let result = mc_dc(
        &circuit,
        "out",
        40,
        McOptions {
            tolerance: 0.05,
            distribution: McDistribution::Uniform,
            seed: Some(15),
        },
    )
    .unwrap();

    assert!(result.mean > 1.8, "mean was {}", result.mean);
    assert!(result.mean < 2.2, "mean was {}", result.mean);
    assert!(result.std_dev > 0.0, "std_dev was {}", result.std_dev);
}

#[test]
fn mc_dc_rejects_invalid_inputs() {
    let circuit = divider();

    assert!(matches!(
        mc_dc(&circuit, "missing", 1, McOptions::default()),
        Err(SpiceError::InvalidElement { name, .. }) if name == "missing"
    ));
    assert!(matches!(
        mc_dc(&circuit, "mid", 0, McOptions::default()),
        Err(SpiceError::InvalidElement { name, .. }) if name == "mc_dc"
    ));
    assert!(matches!(
        mc_dc(
            &circuit,
            "mid",
            1,
            McOptions {
                tolerance: f64::NAN,
                ..McOptions::default()
            },
        ),
        Err(SpiceError::InvalidElement { name, .. }) if name == "mc_dc"
    ));
}
