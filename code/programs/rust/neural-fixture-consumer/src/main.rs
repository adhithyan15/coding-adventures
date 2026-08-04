use serde::{Deserialize, Serialize};
use std::env;
use std::fs::File;
use std::io::{self, Read, Write};
use std::path::Path;

const LANE_ID: &str = "rust-native";
const MAXIMUM_BYTES: u64 = 1 << 20;

#[derive(Debug, Deserialize)]
#[serde(deny_unknown_fields)]
struct Layer {
    name: String,
    weights: Vec<Vec<f64>>,
    biases: Vec<f64>,
    activation: String,
}

#[derive(Debug, Deserialize)]
#[serde(deny_unknown_fields)]
struct Model {
    kind: String,
    input_count: usize,
    layers: Vec<Layer>,
}

#[derive(Debug, Deserialize)]
#[serde(deny_unknown_fields)]
struct DatasetRow {
    label: String,
    input: Vec<f64>,
    target: Vec<f64>,
}

#[derive(Debug, Deserialize)]
#[serde(deny_unknown_fields)]
struct Dataset {
    input_labels: Vec<String>,
    target_labels: Vec<String>,
    rows: Vec<DatasetRow>,
}

#[derive(Debug, Deserialize)]
#[serde(deny_unknown_fields)]
struct ForwardExpectation {
    row: String,
    prediction: Vec<f64>,
}

#[derive(Debug, Deserialize)]
#[serde(deny_unknown_fields)]
struct Expected {
    absolute_tolerance: f64,
    forward: Vec<ForwardExpectation>,
    first_step: Option<serde_json::Value>,
}

#[derive(Debug, Deserialize)]
#[serde(deny_unknown_fields)]
struct Fixture {
    schema_version: u64,
    id: String,
    title: String,
    stage: String,
    question: String,
    concepts: Vec<String>,
    model: Model,
    dataset: Dataset,
    training: Option<serde_json::Value>,
    expected: Expected,
}

#[derive(Debug, Serialize)]
struct Receipt {
    schema_version: u64,
    lane_id: &'static str,
    fixture_id: String,
    row: String,
    contributions: Vec<f64>,
    bias: f64,
    preactivation: f64,
    prediction: Vec<f64>,
    maximum_absolute_error: f64,
    passes: bool,
}

fn parse_fixture(payload: &str) -> Result<Fixture, String> {
    serde_json::from_str(payload).map_err(|error| format!("decode fixture: {error}"))
}

fn load_fixture(path: &Path) -> Result<Fixture, String> {
    let file = File::open(path).map_err(|error| format!("open fixture: {error}"))?;
    let metadata = file
        .metadata()
        .map_err(|error| format!("inspect fixture: {error}"))?;
    if !metadata.is_file() || metadata.len() == 0 || metadata.len() > MAXIMUM_BYTES {
        return Err("fixture must be a non-empty regular file no larger than 1 MiB".to_string());
    }
    let mut payload = Vec::new();
    file.take(MAXIMUM_BYTES + 1)
        .read_to_end(&mut payload)
        .map_err(|error| format!("read fixture: {error}"))?;
    if payload.is_empty() || payload.len() as u64 > MAXIMUM_BYTES {
        return Err("fixture must be a non-empty regular file no larger than 1 MiB".to_string());
    }
    let payload =
        String::from_utf8(payload).map_err(|error| format!("read fixture as UTF-8: {error}"))?;
    parse_fixture(&payload)
}

fn evaluate(document: Fixture) -> Result<Receipt, String> {
    if document.schema_version != 1
        || document.id != "weighted-neuron-forward"
        || document.stage != "forward"
    {
        return Err("unsupported fixture identity".to_string());
    }
    if document.title.is_empty()
        || document.question.is_empty()
        || document.concepts.is_empty()
        || document.training.is_some()
        || document.expected.first_step.is_some()
    {
        return Err("invalid forward-only fixture metadata".to_string());
    }
    if document.model.kind != "single-neuron"
        || document.model.input_count != 2
        || document.model.layers.len() != 1
    {
        return Err("expected one two-input neuron".to_string());
    }
    let layer = &document.model.layers[0];
    if layer.name != "output"
        || layer.activation != "identity"
        || layer.weights.len() != 2
        || layer.biases.len() != 1
    {
        return Err("unsupported layer contract".to_string());
    }
    if document.dataset.input_labels.len() != 2
        || document.dataset.target_labels.len() != 1
        || document.dataset.rows.len() != 1
        || document.expected.forward.len() != 1
    {
        return Err("expected one data row and one forward expectation".to_string());
    }
    let row = &document.dataset.rows[0];
    let expected = &document.expected.forward[0];
    if row.input.len() != 2
        || row.target.len() != 1
        || expected.prediction.len() != 1
        || expected.row != row.label
        || !document.expected.absolute_tolerance.is_finite()
        || document.expected.absolute_tolerance <= 0.0
    {
        return Err("invalid row or expectation shape".to_string());
    }

    let mut contributions = Vec::with_capacity(2);
    let mut preactivation = layer.biases[0];
    for (index, input) in row.input.iter().enumerate() {
        if layer.weights[index].len() != 1 {
            return Err("each input must have one output weight".to_string());
        }
        let contribution = input * layer.weights[index][0];
        contributions.push(contribution);
        preactivation += contribution;
    }
    let maximum_absolute_error = (preactivation - expected.prediction[0]).abs();
    if !layer.biases[0].is_finite()
        || !contributions.iter().all(|value| value.is_finite())
        || !preactivation.is_finite()
        || !maximum_absolute_error.is_finite()
    {
        return Err("non-finite arithmetic result".to_string());
    }
    Ok(Receipt {
        schema_version: 1,
        lane_id: LANE_ID,
        fixture_id: document.id,
        row: row.label.clone(),
        contributions,
        bias: layer.biases[0],
        preactivation,
        prediction: vec![preactivation],
        maximum_absolute_error,
        passes: maximum_absolute_error <= document.expected.absolute_tolerance,
    })
}

fn run(arguments: &[String], mut stdout: impl Write) -> Result<(), String> {
    if arguments.len() != 2 || arguments[0] != "--fixture" || arguments[1].is_empty() {
        return Err("usage: neural-fixture-consumer --fixture PATH".to_string());
    }
    let receipt = evaluate(load_fixture(Path::new(&arguments[1]))?)?;
    if !receipt.passes {
        return Err("prediction exceeded the fixture tolerance".to_string());
    }
    serde_json::to_writer(&mut stdout, &receipt)
        .map_err(|error| format!("encode receipt: {error}"))?;
    writeln!(stdout).map_err(|error| format!("write receipt: {error}"))
}

fn main() {
    let arguments: Vec<String> = env::args().skip(1).collect();
    if let Err(error) = run(&arguments, io::stdout()) {
        eprintln!("{error}");
        std::process::exit(1);
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    const FIXTURE: &str =
        include_str!("../../../../specs/fixtures/neural-learning-v1/labs/00-weighted-neuron.json");

    #[test]
    fn run_emits_passing_receipt() {
        let path = Path::new(env!("CARGO_MANIFEST_DIR"))
            .join("../../../specs/fixtures/neural-learning-v1/labs/00-weighted-neuron.json");
        let mut output = Vec::new();
        run(
            &["--fixture".to_string(), path.display().to_string()],
            &mut output,
        )
        .unwrap();
        let receipt: serde_json::Value = serde_json::from_slice(&output).unwrap();
        assert_eq!(receipt["lane_id"], "rust-native");
        assert_eq!(receipt["contributions"], serde_json::json!([1.0, 0.25]));
        assert_eq!(receipt["preactivation"], 1.35);
        assert_eq!(receipt["passes"], true);
    }

    #[test]
    fn parser_rejects_unknown_fields() {
        let mutated = FIXTURE.replacen(
            "\"schema_version\": 1,",
            "\"schema_version\": 1, \"surprise\": true,",
            1,
        );
        let error = parse_fixture(&mutated).unwrap_err();
        assert!(error.contains("unknown field"), "{error}");
    }

    #[test]
    fn parser_rejects_duplicate_fields() {
        let mutated = FIXTURE.replacen(
            "\"schema_version\": 1,",
            "\"schema_version\": 1, \"schema_version\": 1,",
            1,
        );
        let error = parse_fixture(&mutated).unwrap_err();
        assert!(error.contains("duplicate field"), "{error}");
    }

    #[test]
    fn run_requires_exact_arguments() {
        let error = run(&[], Vec::new()).unwrap_err();
        assert!(error.contains("usage"));
    }
}
