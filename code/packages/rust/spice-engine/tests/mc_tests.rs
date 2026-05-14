use spice_engine::{
    mc_dc, Circuit, CurrentSource, Element, McDistribution, McOptions, Resistor, SpiceError, Vccs,
    Vcvs, VoltageSource,
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
