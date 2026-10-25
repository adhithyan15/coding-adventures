//! Seed and train XOR, or resume a portable checkpoint supplied on stdin.
use coding_adventures_bounded_json::{parse, serialize, JsonNumber, JsonValue as J};
use neural_framework::{Activation as A, DenseNetwork, MAX_CHECKPOINT_BYTES};
use std::io::{self, Read};

fn run() -> Result<(), Box<dyn std::error::Error>> {
    let arguments: Vec<String> = std::env::args().skip(1).collect();
    let resume = match arguments.as_slice() {
        [] => false,
        [flag] if flag == "--resume" => true,
        _ => return Err("usage: xor [--resume]".into()),
    };
    let mut model = if resume {
        let mut text = String::new();
        io::stdin()
            .take((MAX_CHECKPOINT_BYTES + 1) as u64)
            .read_to_string(&mut text)?;
        DenseNetwork::from_json(&text)?
    } else {
        DenseNetwork::seeded(&[2, 4, 1], &[A::Tanh, A::Sigmoid], 7)?
    };
    let inputs = vec![
        vec![0.0, 0.0],
        vec![0.0, 1.0],
        vec![1.0, 0.0],
        vec![1.0, 1.0],
    ];
    let targets = vec![vec![0.0], vec![1.0], vec![1.0], vec![0.0]];
    let losses = model.fit(&inputs, &targets, if resume { 1 } else { 3000 }, 1.0)?;
    let predictions = J::Array(
        model
            .predict(&inputs)?
            .into_iter()
            .map(|row| {
                J::Array(
                    row.into_iter()
                        .map(|v| J::Number(JsonNumber::Float(v)))
                        .collect(),
                )
            })
            .collect(),
    );
    println!(
        "{}",
        serialize(&J::Object(vec![
            ("checkpoint".into(), parse(&model.to_json()?)?),
            ("predictions".into(), predictions),
            (
                "initial_loss".into(),
                J::Number(JsonNumber::Float(losses[0]))
            ),
            (
                "final_loss".into(),
                J::Number(JsonNumber::Float(losses[losses.len() - 1]))
            ),
        ]))?
    );
    Ok(())
}

fn main() {
    if let Err(error) = run() {
        eprintln!("{error}");
        std::process::exit(1);
    }
}
