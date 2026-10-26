//! Independent paper, finite-difference, ownership, and hostile-data oracles.
use coding_adventures_bounded_json::{parse, serialize, JsonNumber, JsonValue as J};
use neural_framework::{
    Activation as A, Checkpoint, DenseLayer, DenseNetwork, Error, Grid, MAX_CHECKPOINT_BYTES,
    MAX_STEP,
};

fn field<'a>(value: &'a J, key: &str) -> &'a J {
    let J::Object(fields) = value else {
        panic!("fixture object")
    };
    &fields.iter().find(|(name, _)| name == key).unwrap().1
}

/// The shared parsed-document corpus is deliberately not a Rust mutation list.
/// Every lane receives the same data; accepted controls rule out reject-all.
#[test]
fn shared_nn40_checkpoint_decisions() {
    let corpus = parse(include_str!(
        "../../../../specs/fixtures/neural-framework-v1/checkpoints.json"
    ))
    .unwrap();
    let J::Object(root) = &corpus else {
        panic!("corpus object")
    };
    assert_eq!(root.len(), 2);
    assert_eq!(field(&corpus, "contract"), &J::String("NN40".into()));
    let cases = array(field(&corpus, "cases"));
    assert_eq!(cases.len(), 55);
    let mut ids = std::collections::HashSet::new();
    let mut accepted = 0;
    let null_ids = [
        "null-weight-cell",
        "null-weight-cell-mixed",
        "null-bias-cell",
        "null-bias-cell-mixed",
        "null-weight-cell-later-layer",
        "null-bias-cell-later-layer",
    ];
    for case in cases {
        let J::Object(fields) = case else {
            panic!("case object")
        };
        assert_eq!(fields.len(), 3);
        let J::String(id) = field(case, "id") else {
            panic!("case id")
        };
        assert!(!id.is_empty() && ids.insert(id));
        let J::Bool(want) = field(case, "accept") else {
            panic!("accept boolean")
        };
        if null_ids.contains(&id.as_str()) {
            assert!(!want, "NN43 numeric null must reject: {id}");
        }
        let document = serialize(field(case, "checkpoint")).unwrap();
        let result = DenseNetwork::from_json(&document);
        assert_eq!(result.is_ok(), *want, "NN40 {id}: {result:?}");
        if *want {
            accepted += 1;
            let model = result.unwrap();
            let checkpoint = model.to_checkpoint();
            assert_eq!(
                DenseNetwork::from_checkpoint(checkpoint.clone())
                    .unwrap()
                    .to_checkpoint(),
                checkpoint
            );
            assert_eq!(
                DenseNetwork::from_json(&model.to_json().unwrap()).unwrap(),
                model
            );
        }
    }
    assert_eq!(accepted, 3);
    for id in null_ids {
        assert!(
            ids.iter().any(|found| found.as_str() == id),
            "missing NN43 {id}"
        );
    }
}
fn number(value: &J) -> f64 {
    match value {
        J::Number(JsonNumber::Float(n)) => *n,
        J::Number(JsonNumber::Integer(n)) => *n as f64,
        _ => panic!("fixture number"),
    }
}
fn array(value: &J) -> &[J] {
    let J::Array(a) = value else {
        panic!("fixture array")
    };
    a
}
fn vector(value: &J) -> Vec<f64> {
    array(value).iter().map(number).collect()
}
fn grid(value: &J) -> Grid {
    array(value).iter().map(vector).collect()
}
fn fixture() -> J {
    parse(include_str!(
        "../../../../specs/fixtures/neural-framework-v1/dense.json"
    ))
    .unwrap()
}
fn close(a: f64, b: f64, tolerance: f64) {
    assert!((a - b).abs() <= tolerance, "{a} != {b}");
}
fn close_grid(a: &Grid, b: &Grid, tolerance: f64) {
    assert_eq!(a.len(), b.len());
    for (row, want) in a.iter().zip(b) {
        assert_eq!(row.len(), want.len());
        for (x, y) in row.iter().zip(want) {
            close(*x, *y, tolerance);
        }
    }
}
fn paper() -> DenseNetwork {
    DenseNetwork::from_json(&serialize(field(field(&fixture(), "paper"), "checkpoint")).unwrap())
        .unwrap()
}
fn one(weight: f64, bias: f64, activation: A) -> DenseNetwork {
    DenseNetwork::new(vec![DenseLayer {
        weights: vec![vec![weight]],
        biases: vec![bias],
        activation,
    }])
    .unwrap()
}
fn input() -> Grid {
    vec![vec![1.0, 2.0]]
}
fn target() -> Grid {
    vec![vec![1.0]]
}

#[test]
fn shared_paper_asserts_every_gradient_and_update() {
    let fixture = fixture();
    let p = field(&fixture, "paper");
    let mut model = paper();
    let before = model.to_checkpoint();
    let trace = model.gradients(&input(), &target()).unwrap();
    close(trace.loss, number(field(p, "loss")), 1e-12);
    close_grid(&trace.predictions, &grid(field(p, "predictions")), 1e-12);
    for (got, want) in trace
        .weight_gradients
        .iter()
        .zip(array(field(p, "weight_gradients")))
    {
        close_grid(got, &grid(want), 1e-12);
    }
    for (got, want) in trace
        .bias_gradients
        .iter()
        .zip(array(field(p, "bias_gradients")))
    {
        close_grid(&vec![got.clone()], &vec![vector(want)], 1e-12);
    }
    assert_eq!(model.to_checkpoint(), before);
    assert_eq!(model.train_step(&input(), &target(), 0.01).unwrap(), trace);
    assert_eq!(model.step(), 1);
    close_grid(
        &model.predict(&input()).unwrap(),
        &grid(field(p, "updated_predictions")),
        1e-12,
    );
    close(
        model.gradients(&input(), &target()).unwrap().loss,
        number(field(p, "updated_loss")),
        1e-12,
    );
    let expected =
        DenseNetwork::from_json(&serialize(field(p, "updated_checkpoint")).unwrap()).unwrap();
    for (got, want) in model
        .to_checkpoint()
        .layers
        .iter()
        .zip(expected.to_checkpoint().layers)
    {
        close_grid(&got.weights, &want.weights, 1e-12);
        close_grid(&vec![got.biases.clone()], &vec![want.biases], 1e-12);
    }
}

#[test]
fn every_activation_and_parameter_matches_finite_differences() {
    for activation in [A::Linear, A::Sigmoid, A::Tanh, A::Relu] {
        let model =
            DenseNetwork::seeded(&[2, 3, 2, 2], &[activation, activation, activation], 7).unwrap();
        let x = vec![vec![0.2, -0.7], vec![0.8, 0.4]];
        let y = vec![vec![0.1, 0.8], vec![0.6, -0.2]];
        let trace = model.gradients(&x, &y).unwrap();
        let h = 1e-6;
        for (l, layer) in model.to_checkpoint().layers.iter().enumerate() {
            for i in 0..layer.weights.len() {
                for j in 0..layer.biases.len() {
                    let mut plus = model.to_checkpoint();
                    let mut minus = plus.clone();
                    plus.layers[l].weights[i][j] += h;
                    minus.layers[l].weights[i][j] -= h;
                    let numerical = (DenseNetwork::from_checkpoint(plus)
                        .unwrap()
                        .gradients(&x, &y)
                        .unwrap()
                        .loss
                        - DenseNetwork::from_checkpoint(minus)
                            .unwrap()
                            .gradients(&x, &y)
                            .unwrap()
                            .loss)
                        / (2.0 * h);
                    close(trace.weight_gradients[l][i][j], numerical, 2e-6);
                }
            }
            for j in 0..layer.biases.len() {
                let mut plus = model.to_checkpoint();
                let mut minus = plus.clone();
                plus.layers[l].biases[j] += h;
                minus.layers[l].biases[j] -= h;
                let numerical = (DenseNetwork::from_checkpoint(plus)
                    .unwrap()
                    .gradients(&x, &y)
                    .unwrap()
                    .loss
                    - DenseNetwork::from_checkpoint(minus)
                        .unwrap()
                        .gradients(&x, &y)
                        .unwrap()
                        .loss)
                    / (2.0 * h);
                close(trace.bias_gradients[l][j], numerical, 2e-6);
            }
        }
    }
}

#[test]
fn seeded_draws_and_exact_xor_training_are_native() {
    let f = fixture();
    let seed = field(&f, "seed");
    let model = DenseNetwork::seeded(&[2, 2], &[A::Tanh], 7).unwrap();
    close_grid(
        &model.to_checkpoint().layers[0].weights,
        &grid(field(seed, "first_weights")),
        1e-12,
    );
    let xor = field(&f, "xor");
    let x = grid(field(xor, "inputs"));
    let y = grid(field(xor, "targets"));
    let mut model = DenseNetwork::seeded(&[2, 4, 1], &[A::Tanh, A::Sigmoid], 7).unwrap();
    let initial = model.to_checkpoint();
    assert_eq!(model.fit(&x, &y, 0, 1.0).unwrap().len(), 1);
    assert_eq!(model.to_checkpoint(), initial);
    let losses = model.fit(&x, &y, 3000, 1.0).unwrap();
    assert_eq!(losses.len(), 3001);
    assert_eq!(model.step(), 3000);
    assert!(losses[3000] < number(field(xor, "maximum_final_loss")));
    close(losses[3000], 0.00023307123378282047, 1e-9);
    // Checkpoint text is ordinary portable data. Resume and uninterrupted
    // training must arrive at identical parameters, predictions and count.
    let mut resumed = DenseNetwork::from_json(&model.to_json().unwrap()).unwrap();
    model.fit(&x, &y, 2, 1.0).unwrap();
    resumed.fit(&x, &y, 2, 1.0).unwrap();
    assert_eq!(model.step(), 3002);
    close_grid(
        &model.predict(&x).unwrap(),
        &resumed.predict(&x).unwrap(),
        1e-12,
    );
    for (a, b) in model
        .to_checkpoint()
        .layers
        .iter()
        .zip(resumed.to_checkpoint().layers)
    {
        close_grid(&a.weights, &b.weights, 1e-12);
    }
}

#[test]
fn owned_snapshots_do_not_mutate_the_network() {
    let model = paper();
    let original = model.to_checkpoint();
    let mut checkpoint = model.to_checkpoint();
    checkpoint.layers[0].weights[0][0] = 100.0;
    let mut predictions = model.predict(&input()).unwrap();
    predictions[0][0] = 100.0;
    let mut trace = model.gradients(&input(), &target()).unwrap();
    trace.weight_gradients[0][0][0] = 100.0;
    let mut detached = model.clone();
    detached.train_step(&input(), &target(), 0.01).unwrap();
    assert_eq!(model.to_checkpoint(), original);
    assert_ne!(detached.to_checkpoint(), original);
}

#[test]
fn constructors_reject_all_invalid_shapes_and_nonfinite_values() {
    assert_eq!(DenseNetwork::new(vec![]), Err(Error::Shape));
    for weights in [
        vec![],
        vec![vec![]],
        vec![vec![1.0], vec![1.0, 2.0]],
        vec![vec![f64::NAN]],
        vec![vec![f64::INFINITY]],
    ] {
        assert!(DenseNetwork::new(vec![DenseLayer {
            weights,
            biases: vec![0.0],
            activation: A::Linear
        }])
        .is_err());
    }
    for biases in [vec![], vec![0.0, 0.0], vec![f64::NEG_INFINITY]] {
        assert!(DenseNetwork::new(vec![DenseLayer {
            weights: vec![vec![1.0]],
            biases,
            activation: A::Linear
        }])
        .is_err());
    }
    let mut layers = paper().to_checkpoint().layers;
    layers[1].weights.push(vec![1.0]);
    assert_eq!(DenseNetwork::new(layers), Err(Error::Shape));
    for widths in [&[][..], &[1][..], &[0, 1][..], &[1, 0][..]] {
        assert!(DenseNetwork::seeded(widths, &[A::Linear], 0).is_err());
    }
    assert!(DenseNetwork::seeded(&[1, 2], &[], 0).is_err());
    assert_eq!(
        DenseNetwork::seeded(&[usize::MAX, 2], &[A::Linear], 0),
        Err(Error::Count)
    );
    for widths in [&[usize::MAX, 1][..], &[1, usize::MAX][..]] {
        assert_eq!(
            DenseNetwork::seeded(widths, &[A::Linear], 0),
            Err(Error::Count)
        );
    }
    let a = DenseNetwork::seeded(&[1, 1], &[A::Linear], u32::MAX).unwrap();
    assert_eq!(
        a,
        DenseNetwork::seeded(&[1, 1], &[A::Linear], u32::MAX).unwrap()
    );
}

#[test]
fn batches_rates_and_counts_fail_without_mutation() {
    let mut model = paper();
    let before = model.to_checkpoint();
    for bad in [
        vec![],
        vec![vec![]],
        vec![vec![1.0]],
        vec![vec![1.0, 2.0], vec![1.0]],
        vec![vec![f64::NAN, 1.0]],
    ] {
        assert!(model.predict(&bad).is_err());
        assert!(model.train_step(&bad, &target(), 0.1).is_err());
    }
    for bad in [
        vec![],
        vec![vec![]],
        vec![vec![1.0, 2.0]],
        vec![vec![1.0], vec![2.0]],
        vec![vec![f64::INFINITY]],
    ] {
        assert!(model.gradients(&input(), &bad).is_err());
    }
    for r in [0.0, -1.0, f64::NAN, f64::INFINITY, f64::NEG_INFINITY] {
        assert_eq!(model.train_step(&input(), &target(), r), Err(Error::Rate));
        assert_eq!(model.fit(&input(), &target(), 0, r), Err(Error::Rate));
    }
    assert_eq!(
        model.fit(&input(), &target(), usize::MAX, 0.1),
        Err(Error::Count)
    );
    assert_eq!(model.to_checkpoint(), before);
    let mut exhausted = before.clone();
    exhausted.step = MAX_STEP;
    let mut exhausted = DenseNetwork::from_checkpoint(exhausted).unwrap();
    assert!(exhausted.fit(&input(), &target(), 0, 0.1).is_ok());
    let before = exhausted.to_checkpoint();
    assert_eq!(
        exhausted.train_step(&input(), &target(), 0.1),
        Err(Error::StepExhausted)
    );
    assert_eq!(exhausted.to_checkpoint(), before);
}

#[test]
fn nonfinite_arithmetic_and_candidate_loss_are_atomic() {
    let mut model = one(1.0, 0.0, A::Linear);
    let before = model.to_checkpoint();
    assert_eq!(
        model.train_step(&[vec![1.0]], &[vec![0.0]], f64::MAX),
        Err(Error::NonFinite)
    );
    assert_eq!(model.to_checkpoint(), before);
    // Candidate weights stay finite at this rate, but its squared loss does not.
    assert_eq!(
        model.train_step(&[vec![1.0]], &[vec![0.0]], 1e155),
        Err(Error::NonFinite)
    );
    assert_eq!(model.to_checkpoint(), before);
    for m in [one(f64::MAX, 0.0, A::Sigmoid), one(1.0, f64::MAX, A::Tanh)] {
        assert_eq!(m.predict(&[vec![f64::MAX]]), Err(Error::NonFinite));
    }
    assert_eq!(
        one(1.0, 0.0, A::Linear).gradients(&[vec![1e200]], &[vec![0.0]]),
        Err(Error::NonFinite)
    );
    assert_eq!(
        one(1.0, 0.0, A::Linear).gradients(&[vec![f64::MAX]], &[vec![-f64::MAX]]),
        Err(Error::NonFinite)
    );
    // Multiplication and reduction overflow are separately observable backward
    // boundaries, even when the forward prediction and loss remain finite.
    assert_eq!(
        one(0.0, 0.0, A::Linear).gradients(&[vec![1e250]], &[vec![1e100]]),
        Err(Error::NonFinite)
    );
    let model = DenseNetwork::new(vec![
        DenseLayer {
            weights: vec![vec![0.0]],
            biases: vec![0.0],
            activation: A::Linear,
        },
        DenseLayer {
            weights: vec![vec![1e300]],
            biases: vec![0.0],
            activation: A::Linear,
        },
    ])
    .unwrap();
    assert_eq!(
        model.gradients(&[vec![1e100]], &[vec![1.0]]),
        Err(Error::NonFinite)
    );
}

#[test]
fn stable_sigmoid_relu_zero_and_multiple_outputs() {
    for activation in [A::Linear, A::Tanh, A::Sigmoid, A::Relu] {
        let model = one(1.0, 0.0, activation);
        assert!(model
            .predict(&[vec![-1000.0], vec![0.0], vec![1000.0]])
            .is_ok());
    }
    let zero = one(1.0, 0.0, A::Relu)
        .gradients(&[vec![0.0]], &[vec![1.0]])
        .unwrap();
    assert_eq!(zero.weight_gradients, vec![vec![vec![0.0]]]);
    assert_eq!(zero.bias_gradients, vec![vec![0.0]]);
    let model = DenseNetwork::new(vec![DenseLayer {
        weights: vec![vec![1.0, 2.0]],
        biases: vec![0.0, 0.0],
        activation: A::Linear,
    }])
    .unwrap();
    close(
        model
            .gradients(&[vec![1.0]], &[vec![0.0, 0.0]])
            .unwrap()
            .loss,
        2.5,
        1e-12,
    );
}

#[test]
fn checkpoint_metadata_shapes_and_json_types_are_closed() {
    let valid = paper().to_json().unwrap();
    assert_eq!(DenseNetwork::from_json(&valid).unwrap(), paper());
    for text in [
        "null".to_owned(),
        "[]".into(),
        "{".into(),
        "{}".into(),
        valid.replace("\"version\":1", "\"version\":2"),
        valid.replace("\"version\":1", "\"version\":1.0"),
        valid.replace("\"step\":0", "\"step\":-1"),
        valid.replace("\"step\":0", "\"step\":true"),
        valid.replace("\"step\":0", "\"step\":9007199254740992"),
        valid.replace("coding-adventures-dense", "other"),
        valid.replace("\"linear\"", "\"unknown\""),
        valid.replace("\"linear\"", "false"),
        valid.replacen("0.5", "true", 1),
        valid.replacen("0.5", "1e999", 1),
        valid.replacen("\"weights\":", "\"unknown\":", 1),
        valid.replacen("\"weights\":", "\"biases\":[],\"weights\":", 1),
        valid.replace("\"step\":0", "\"step\":0,\"step\":0"),
        valid.replace("\"step\":0", "\"format\":\"coding-adventures-dense\""),
        valid.replace("\"layers\":[", "\"layers\":{} ,\"unused\":["),
        " ".repeat(MAX_CHECKPOINT_BYTES + 1),
        format!("{}0{}", "[".repeat(130), "]".repeat(130)),
    ] {
        assert!(DenseNetwork::from_json(&text).is_err(), "accepted {text}");
    }
    for mutate in [0, 1, 2, 3, 4] {
        let mut cp = paper().to_checkpoint();
        match mutate {
            0 => cp.format = "unknown".into(),
            1 => cp.version = 0,
            2 => cp.step = MAX_STEP + 1,
            3 => cp.layers.clear(),
            _ => cp.layers[0].weights[0][0] = f64::NAN,
        }
        assert!(DenseNetwork::from_checkpoint(cp).is_err());
    }
    let cp = Checkpoint {
        format: "coding-adventures-dense".into(),
        version: 1,
        step: 0,
        layers: vec![],
    };
    assert_eq!(DenseNetwork::from_checkpoint(cp), Err(Error::Shape));
    assert_eq!(Error::Shape.to_string(), "neural framework Shape");
    let _: &dyn std::error::Error = &Error::Metadata;
}
