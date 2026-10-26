//! NN38: build and train dense networks natively above the Rust matrix layer.
//!
//! Each layer computes `activation(input * weights + bias)`. Backpropagation
//! walks those layers in reverse; it does not call an external ML framework.
//! Checkpoints contain only owned numbers and activation names, never code.
#![forbid(unsafe_code)]
#![doc = include_str!("../README.md")]

use coding_adventures_bounded_json::{self as json, JsonNumber, JsonValue};
use matrix::Matrix;
use std::fmt;

/// Row-major, sample-by-feature data (weights instead use input-by-output).
pub type Grid = Vec<Vec<f64>>;
/// NN37 step counts must survive a JavaScript JSON round trip exactly.
pub const MAX_STEP: u64 = (1 << 53) - 1;
/// The text transport is bounded; the owned-record API has no text envelope.
pub const MAX_CHECKPOINT_BYTES: usize = 4 * 1024 * 1024;

/// Closed, input-free errors. No checkpoint bytes or dataset values leak here.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Error {
    /// Empty, ragged, disconnected, or dimensionally incompatible arrays.
    Shape,
    /// A number or intermediate computation is not finite.
    NonFinite,
    /// Unknown, missing, duplicated, or incorrectly typed checkpoint metadata.
    Metadata,
    /// Malformed JSON or a text/depth envelope violation.
    Json,
    /// Learning rates must be finite and strictly positive.
    Rate,
    /// A requested count cannot be represented by this host.
    Count,
    /// The next successful step would exceed the portable safe-integer bound.
    StepExhausted,
}

impl fmt::Display for Error {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "neural framework {:?}", self)
    }
}
impl std::error::Error for Error {}

/// Nonlinearities supported by the portable dense contract.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Activation {
    /// Identity; derivative is one.
    Linear,
    /// Stable logistic function; derivative is a * (1 - a).
    Sigmoid,
    /// Hyperbolic tangent; derivative is 1 - a squared.
    Tanh,
    /// Positive part; derivative at zero is defined as zero.
    Relu,
}

impl Activation {
    fn apply(self, z: f64) -> f64 {
        match self {
            Self::Linear => z,
            Self::Relu => z.max(0.0),
            Self::Tanh => z.tanh(),
            Self::Sigmoid => {
                // exp(nonpositive) cannot overflow, even for a very negative z.
                let factor = (-z.abs()).exp();
                if z >= 0.0 {
                    1.0 / (1.0 + factor)
                } else {
                    factor / (1.0 + factor)
                }
            }
        }
    }
    fn derivative(self, raw: f64, activated: f64) -> f64 {
        match self {
            Self::Linear => 1.0,
            Self::Relu => {
                if raw > 0.0 {
                    1.0
                } else {
                    0.0
                }
            }
            Self::Tanh => 1.0 - activated * activated,
            Self::Sigmoid => activated * (1.0 - activated),
        }
    }
    fn name(self) -> &'static str {
        match self {
            Self::Linear => "linear",
            Self::Sigmoid => "sigmoid",
            Self::Tanh => "tanh",
            Self::Relu => "relu",
        }
    }
    fn parse(value: &JsonValue) -> Result<Self, Error> {
        match value {
            JsonValue::String(s) => match s.as_str() {
                "linear" => Ok(Self::Linear),
                "sigmoid" => Ok(Self::Sigmoid),
                "tanh" => Ok(Self::Tanh),
                "relu" => Ok(Self::Relu),
                _ => Err(Error::Metadata),
            },
            _ => Err(Error::Metadata),
        }
    }
}

/// An owned layer record. Construction/loading validates its raw arrays.
#[derive(Clone, Debug, PartialEq)]
pub struct DenseLayer {
    /// Input-by-output weights, matching A * W.
    pub weights: Grid,
    /// One bias per output column.
    pub biases: Vec<f64>,
    /// Activation applied after affine transformation.
    pub activation: Activation,
}

/// Owned, closed NN37 checkpoint data; validate it with `from_checkpoint`.
#[derive(Clone, Debug, PartialEq)]
pub struct Checkpoint {
    /// Must be `coding-adventures-dense`.
    pub format: String,
    /// Must be integer one.
    pub version: u32,
    /// Successful SGD updates, at most MAX_STEP.
    pub step: u64,
    /// Ordered dense layers.
    pub layers: Vec<DenseLayer>,
}

/// Analytical MSE gradients at the model state before an optimizer update.
#[derive(Clone, Debug, PartialEq)]
pub struct GradientTrace {
    /// Mean over samples times output coordinates, not just samples.
    pub loss: f64,
    /// Owned forward predictions.
    pub predictions: Grid,
    /// One input-by-output gradient matrix per layer.
    pub weight_gradients: Vec<Grid>,
    /// One bias gradient vector per layer.
    pub bias_gradients: Vec<Vec<f64>>,
}

/// Immutable observations with transactional, full-batch SGD updates.
#[derive(Clone, Debug, PartialEq)]
pub struct DenseNetwork {
    layers: Vec<DenseLayer>,
    step: u64,
}

struct Forward {
    saved: Vec<Grid>,
    raw: Vec<Grid>,
}

fn finite(value: f64) -> Result<f64, Error> {
    if value.is_finite() {
        Ok(value)
    } else {
        Err(Error::NonFinite)
    }
}
fn grid(values: &[Vec<f64>]) -> Result<(usize, usize), Error> {
    let columns = values.first().ok_or(Error::Shape)?.len();
    if columns == 0 || values.iter().any(|row| row.len() != columns) {
        return Err(Error::Shape);
    }
    for value in values.iter().flatten() {
        finite(*value)?;
    }
    Ok((values.len(), columns))
}
fn rate(value: f64) -> Result<(), Error> {
    if value.is_finite() && value > 0.0 {
        Ok(())
    } else {
        Err(Error::Rate)
    }
}
fn dot(a: &Grid, b: &Grid) -> Result<Grid, Error> {
    grid(a)?;
    grid(b)?;
    let out = Matrix::new_2d(a.clone())
        .dot(&Matrix::new_2d(b.clone()))
        .map_err(|_| Error::Shape)?
        .data;
    grid(&out)?;
    Ok(out)
}
fn transpose(a: &Grid) -> Grid {
    Matrix::new_2d(a.clone()).transpose().data
}

impl DenseNetwork {
    /// Validate all shapes and numbers before exposing a usable model.
    pub fn new(layers: Vec<DenseLayer>) -> Result<Self, Error> {
        if layers.is_empty() {
            return Err(Error::Shape);
        }
        let mut previous = None;
        for layer in &layers {
            let (inputs, outputs) = grid(&layer.weights)?;
            if outputs != layer.biases.len() || previous.is_some_and(|p| p != inputs) {
                return Err(Error::Shape);
            }
            for bias in &layer.biases {
                finite(*bias)?;
            }
            previous = Some(outputs);
        }
        Ok(Self { layers, step: 0 })
    }

    /// Reproducible NN37 initialization; this RNG is not cryptographic.
    pub fn seeded(widths: &[usize], activations: &[Activation], seed: u32) -> Result<Self, Error> {
        if widths.len() < 2 || activations.len() != widths.len() - 1 || widths.contains(&0) {
            return Err(Error::Shape);
        }
        let mut state = seed;
        let mut layers = Vec::new();
        for (pair, activation) in widths.windows(2).zip(activations) {
            // Vec rejects byte capacities above isize::MAX even when the
            // element-count product fits usize. Reject that boundary as data
            // before asking the allocator for an impossible matrix.
            for (count, size) in [
                (
                    pair[0].checked_mul(pair[1]).ok_or(Error::Count)?,
                    std::mem::size_of::<f64>(),
                ),
                (pair[0], std::mem::size_of::<Vec<f64>>()),
                (pair[1], std::mem::size_of::<f64>()),
            ] {
                count
                    .checked_mul(size)
                    .filter(|n| *n <= isize::MAX as usize)
                    .ok_or(Error::Count)?;
            }
            let mut weights = vec![vec![0.0; pair[1]]; pair[0]];
            for weight in weights.iter_mut().flatten() {
                state = state.wrapping_mul(1_664_525).wrapping_add(1_013_904_223);
                *weight =
                    (2.0 * f64::from(state) / 4_294_967_296.0 - 1.0) / (pair[0] as f64).sqrt();
            }
            layers.push(DenseLayer {
                weights,
                biases: vec![0.0; pair[1]],
                activation: *activation,
            });
        }
        Self::new(layers)
    }

    /// Successful optimizer updates since construction/loading.
    pub fn step(&self) -> u64 {
        self.step
    }

    fn forward(&self, inputs: &[Vec<f64>]) -> Result<Forward, Error> {
        let (_, columns) = grid(inputs)?;
        if columns != self.layers[0].weights.len() {
            return Err(Error::Shape);
        }
        let mut current = inputs.to_vec();
        let mut saved = vec![current.clone()];
        let mut raw_layers = Vec::new();
        for layer in &self.layers {
            let products = dot(&current, &layer.weights)?;
            let raw: Grid = products
                .iter()
                .map(|row| row.iter().zip(&layer.biases).map(|(v, b)| v + b).collect())
                .collect();
            grid(&raw)?;
            current = raw
                .iter()
                .map(|row| row.iter().map(|z| layer.activation.apply(*z)).collect())
                .collect();
            grid(&current)?;
            raw_layers.push(raw);
            saved.push(current.clone());
        }
        Ok(Forward {
            saved,
            raw: raw_layers,
        })
    }

    /// Return a fresh sample-by-output prediction matrix.
    pub fn predict(&self, inputs: &[Vec<f64>]) -> Result<Grid, Error> {
        self.forward(inputs)?.saved.pop().ok_or(Error::Shape)
    }

    /// Reverse the chain of affine maps and activation derivatives.
    pub fn gradients(
        &self,
        inputs: &[Vec<f64>],
        targets: &[Vec<f64>],
    ) -> Result<GradientTrace, Error> {
        let forward = self.forward(inputs)?;
        let predictions = &forward.saved[self.layers.len()];
        if grid(targets)? != grid(predictions)? {
            return Err(Error::Shape);
        }
        let count = predictions
            .len()
            .checked_mul(predictions[0].len())
            .ok_or(Error::Count)? as f64;
        let mut loss = 0.0;
        let mut upstream = predictions.clone();
        for ((row, predicted), target) in upstream.iter_mut().zip(predictions).zip(targets) {
            for ((g, p), t) in row.iter_mut().zip(predicted).zip(target) {
                let error = finite(p - t)?;
                loss += error * error;
                *g = 2.0 * error / count;
            }
        }
        loss = finite(loss / count)?;
        grid(&upstream)?;
        let mut dw = Vec::new();
        let mut db = Vec::new();
        for index in (0..self.layers.len()).rev() {
            let layer = &self.layers[index];
            let mut delta = upstream;
            for ((row, raw), activated) in delta
                .iter_mut()
                .zip(&forward.raw[index])
                .zip(&forward.saved[index + 1])
            {
                for ((g, z), a) in row.iter_mut().zip(raw).zip(activated) {
                    *g *= layer.activation.derivative(*z, *a);
                }
            }
            grid(&delta)?;
            dw.push(dot(&transpose(&forward.saved[index]), &delta)?);
            let mut biases = vec![0.0; layer.biases.len()];
            for row in &delta {
                for (sum, value) in biases.iter_mut().zip(row) {
                    *sum += value;
                }
            }
            for b in &biases {
                finite(*b)?;
            }
            db.push(biases);
            upstream = if index > 0 {
                dot(&delta, &transpose(&layer.weights))?
            } else {
                delta
            };
        }
        dw.reverse();
        db.reverse();
        Ok(GradientTrace {
            loss,
            predictions: predictions.clone(),
            weight_gradients: dw,
            bias_gradients: db,
        })
    }

    /// Exactly one successful SGD update; return the pre-update trace.
    pub fn train_step(
        &mut self,
        inputs: &[Vec<f64>],
        targets: &[Vec<f64>],
        learning_rate: f64,
    ) -> Result<GradientTrace, Error> {
        rate(learning_rate)?;
        if self.step == MAX_STEP {
            return Err(Error::StepExhausted);
        }
        let trace = self.gradients(inputs, targets)?;
        // Work only on an owned candidate. Even a post-update forward overflow
        // must leave every original weight, bias, and step unchanged.
        let mut candidate = self.clone();
        for ((layer, dw), db) in candidate
            .layers
            .iter_mut()
            .zip(&trace.weight_gradients)
            .zip(&trace.bias_gradients)
        {
            for (row, gs) in layer.weights.iter_mut().zip(dw) {
                for (w, g) in row.iter_mut().zip(gs) {
                    *w = finite(*w - learning_rate * g)?;
                }
            }
            for (b, g) in layer.biases.iter_mut().zip(db) {
                *b = finite(*b - learning_rate * g)?;
            }
        }
        candidate.gradients(inputs, targets)?;
        candidate.step += 1;
        *self = candidate;
        Ok(trace)
    }

    /// Initial loss followed by one post-update loss per requested epoch.
    /// Earlier successful steps remain committed if a later step fails.
    pub fn fit(
        &mut self,
        inputs: &[Vec<f64>],
        targets: &[Vec<f64>],
        epochs: usize,
        learning_rate: f64,
    ) -> Result<Vec<f64>, Error> {
        rate(learning_rate)?;
        epochs.checked_add(1).ok_or(Error::Count)?;
        let mut losses = vec![self.gradients(inputs, targets)?.loss];
        for _ in 0..epochs {
            self.train_step(inputs, targets, learning_rate)?;
            losses.push(self.gradients(inputs, targets)?.loss);
        }
        Ok(losses)
    }

    /// Fresh owned checkpoint records, never aliases into live model state.
    pub fn to_checkpoint(&self) -> Checkpoint {
        Checkpoint {
            format: "coding-adventures-dense".into(),
            version: 1,
            step: self.step,
            layers: self.layers.clone(),
        }
    }

    /// Validate an owned checkpoint without file I/O or dynamic code loading.
    pub fn from_checkpoint(checkpoint: Checkpoint) -> Result<Self, Error> {
        if checkpoint.format != "coding-adventures-dense"
            || checkpoint.version != 1
            || checkpoint.step > MAX_STEP
        {
            return Err(Error::Metadata);
        }
        let mut model = Self::new(checkpoint.layers)?;
        model.step = checkpoint.step;
        Ok(model)
    }

    /// Decode bounded-depth JSON with closed objects and exact integer metadata.
    pub fn from_json(text: &str) -> Result<Self, Error> {
        if text.len() > MAX_CHECKPOINT_BYTES {
            return Err(Error::Json);
        }
        let value = json::parse(text).map_err(|_| Error::Json)?;
        let fields = object(&value, &["format", "version", "step", "layers"])?;
        if fields[0] != &JsonValue::String("coding-adventures-dense".into())
            || integer(fields[1])? != 1
        {
            return Err(Error::Metadata);
        }
        let layers = array(fields[3])?
            .iter()
            .map(|value| {
                let f = object(value, &["weights", "biases", "activation"])?;
                Ok(DenseLayer {
                    weights: json_grid(f[0])?,
                    biases: json_vector(f[1])?,
                    activation: Activation::parse(f[2])?,
                })
            })
            .collect::<Result<Vec<_>, Error>>()?;
        Self::from_checkpoint(Checkpoint {
            format: "coding-adventures-dense".into(),
            version: 1,
            step: integer(fields[2])?,
            layers,
        })
    }

    /// Serialize only validated, finite NN37 checkpoint data.
    pub fn to_json(&self) -> Result<String, Error> {
        let layers = self
            .layers
            .iter()
            .map(|layer| {
                JsonValue::Object(vec![
                    ("weights".into(), json_grid_value(&layer.weights)),
                    ("biases".into(), json_vector_value(&layer.biases)),
                    (
                        "activation".into(),
                        JsonValue::String(layer.activation.name().into()),
                    ),
                ])
            })
            .collect();
        json::serialize(&JsonValue::Object(vec![
            (
                "format".into(),
                JsonValue::String("coding-adventures-dense".into()),
            ),
            ("version".into(), JsonValue::Number(JsonNumber::Integer(1))),
            (
                "step".into(),
                JsonValue::Number(JsonNumber::Integer(self.step as i64)),
            ),
            ("layers".into(), JsonValue::Array(layers)),
        ]))
        .map_err(|_| Error::Json)
    }
}

// Objects are intentionally source-ordered vectors in bounded-json. Checking
// cardinality AND exactly one occurrence of each name rejects duplicate keys
// rather than accepting a parser's arbitrary last-wins interpretation.
fn object<'a>(value: &'a JsonValue, keys: &[&str]) -> Result<Vec<&'a JsonValue>, Error> {
    let JsonValue::Object(fields) = value else {
        return Err(Error::Metadata);
    };
    if fields.len() != keys.len() {
        return Err(Error::Metadata);
    }
    keys.iter()
        .map(|key| {
            let mut matches = fields.iter().filter(|(name, _)| name == key);
            let (_, value) = matches.next().ok_or(Error::Metadata)?;
            if matches.next().is_some() {
                return Err(Error::Metadata);
            }
            Ok(value)
        })
        .collect()
}
fn integer(value: &JsonValue) -> Result<u64, Error> {
    match value {
        JsonValue::Number(JsonNumber::Integer(n)) if *n >= 0 && *n as u64 <= MAX_STEP => {
            Ok(*n as u64)
        }
        _ => Err(Error::Metadata),
    }
}
fn array(value: &JsonValue) -> Result<&[JsonValue], Error> {
    match value {
        JsonValue::Array(values) => Ok(values),
        _ => Err(Error::Shape),
    }
}
fn json_vector(value: &JsonValue) -> Result<Vec<f64>, Error> {
    array(value)?
        .iter()
        .map(|v| match v {
            JsonValue::Number(JsonNumber::Integer(n)) => Ok(*n as f64),
            JsonValue::Number(JsonNumber::Float(n)) => finite(*n),
            _ => Err(Error::Metadata),
        })
        .collect()
}
fn json_grid(value: &JsonValue) -> Result<Grid, Error> {
    array(value)?.iter().map(json_vector).collect()
}
fn json_vector_value(value: &[f64]) -> JsonValue {
    JsonValue::Array(
        value
            .iter()
            .map(|n| JsonValue::Number(JsonNumber::Float(*n)))
            .collect(),
    )
}
fn json_grid_value(value: &Grid) -> JsonValue {
    JsonValue::Array(value.iter().map(|row| json_vector_value(row)).collect())
}
