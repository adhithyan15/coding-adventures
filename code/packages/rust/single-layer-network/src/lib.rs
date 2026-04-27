pub const VERSION: &str = "0.1.0";

pub type Matrix = Vec<Vec<f64>>;

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum ActivationName {
    Linear,
    Sigmoid,
}

#[derive(Clone, Debug, PartialEq)]
pub struct TrainingStep {
    pub predictions: Matrix,
    pub errors: Matrix,
    pub weight_gradients: Matrix,
    pub bias_gradients: Vec<f64>,
    pub next_weights: Matrix,
    pub next_biases: Vec<f64>,
    pub loss: f64,
}

#[derive(Clone, Debug, PartialEq)]
pub struct SingleLayerNetwork {
    pub weights: Matrix,
    pub biases: Vec<f64>,
    pub activation: ActivationName,
}

fn validate_matrix(name: &str, matrix: &Matrix) -> Result<(usize, usize), String> {
    if matrix.is_empty() {
        return Err(format!("{name} must contain at least one row"));
    }
    let width = matrix[0].len();
    if width == 0 {
        return Err(format!("{name} must contain at least one column"));
    }
    if matrix.iter().any(|row| row.len() != width) {
        return Err(format!("{name} must be rectangular"));
    }
    Ok((matrix.len(), width))
}

fn activate(value: f64, activation: ActivationName) -> f64 {
    match activation {
        ActivationName::Linear => value,
        ActivationName::Sigmoid => {
            if value >= 0.0 {
                let z = (-value).exp();
                1.0 / (1.0 + z)
            } else {
                let z = value.exp();
                z / (1.0 + z)
            }
        }
    }
}

fn derivative_from_output(output: f64, activation: ActivationName) -> f64 {
    match activation {
        ActivationName::Linear => 1.0,
        ActivationName::Sigmoid => output * (1.0 - output),
    }
}

pub fn predict_with_parameters(
    inputs: &Matrix,
    weights: &Matrix,
    biases: &[f64],
    activation: ActivationName,
) -> Result<Matrix, String> {
    let (samples, input_count) = validate_matrix("inputs", inputs)?;
    let (weight_rows, output_count) = validate_matrix("weights", weights)?;
    if input_count != weight_rows {
        return Err("input column count must match weight row count".to_string());
    }
    if biases.len() != output_count {
        return Err("bias count must match output count".to_string());
    }

    let mut predictions = vec![vec![0.0; output_count]; samples];
    for row in 0..samples {
        for output in 0..output_count {
            let mut total = biases[output];
            for input in 0..input_count {
                total += inputs[row][input] * weights[input][output];
            }
            predictions[row][output] = activate(total, activation);
        }
    }
    Ok(predictions)
}

pub fn train_one_epoch_with_matrices(
    inputs: &Matrix,
    targets: &Matrix,
    weights: &Matrix,
    biases: &[f64],
    learning_rate: f64,
    activation: ActivationName,
) -> Result<TrainingStep, String> {
    let (samples, input_count) = validate_matrix("inputs", inputs)?;
    let (target_rows, output_count) = validate_matrix("targets", targets)?;
    let (weight_rows, weight_cols) = validate_matrix("weights", weights)?;
    if target_rows != samples {
        return Err("inputs and targets must have the same row count".to_string());
    }
    if weight_rows != input_count || weight_cols != output_count {
        return Err("weights must be shaped input_count x output_count".to_string());
    }
    if biases.len() != output_count {
        return Err("bias count must match output count".to_string());
    }

    let predictions = predict_with_parameters(inputs, weights, biases, activation)?;
    let scale = 2.0 / ((samples * output_count) as f64);
    let mut errors = vec![vec![0.0; output_count]; samples];
    let mut deltas = vec![vec![0.0; output_count]; samples];
    let mut loss_total = 0.0;
    for row in 0..samples {
        for output in 0..output_count {
            let error = predictions[row][output] - targets[row][output];
            errors[row][output] = error;
            deltas[row][output] =
                scale * error * derivative_from_output(predictions[row][output], activation);
            loss_total += error * error;
        }
    }

    let mut weight_gradients = vec![vec![0.0; output_count]; input_count];
    let mut next_weights = vec![vec![0.0; output_count]; input_count];
    for input in 0..input_count {
        for output in 0..output_count {
            for row in 0..samples {
                weight_gradients[input][output] += inputs[row][input] * deltas[row][output];
            }
            next_weights[input][output] =
                weights[input][output] - learning_rate * weight_gradients[input][output];
        }
    }

    let mut bias_gradients = vec![0.0; output_count];
    let mut next_biases = vec![0.0; output_count];
    for output in 0..output_count {
        for row in 0..samples {
            bias_gradients[output] += deltas[row][output];
        }
        next_biases[output] = biases[output] - learning_rate * bias_gradients[output];
    }

    Ok(TrainingStep {
        predictions,
        errors,
        weight_gradients,
        bias_gradients,
        next_weights,
        next_biases,
        loss: loss_total / ((samples * output_count) as f64),
    })
}

impl SingleLayerNetwork {
    pub fn new(input_count: usize, output_count: usize, activation: ActivationName) -> Self {
        Self {
            weights: vec![vec![0.0; output_count]; input_count],
            biases: vec![0.0; output_count],
            activation,
        }
    }

    pub fn predict(&self, inputs: &Matrix) -> Result<Matrix, String> {
        predict_with_parameters(inputs, &self.weights, &self.biases, self.activation)
    }

    pub fn fit(
        &mut self,
        inputs: &Matrix,
        targets: &Matrix,
        learning_rate: f64,
        epochs: usize,
    ) -> Result<Vec<TrainingStep>, String> {
        let mut history = Vec::with_capacity(epochs);
        for _ in 0..epochs {
            let step = train_one_epoch_with_matrices(
                inputs,
                targets,
                &self.weights,
                &self.biases,
                learning_rate,
                self.activation,
            )?;
            self.weights = step.next_weights.clone();
            self.biases = step.next_biases.clone();
            history.push(step);
        }
        Ok(history)
    }
}

pub fn fit_single_layer_network(
    inputs: &Matrix,
    targets: &Matrix,
    learning_rate: f64,
    epochs: usize,
    activation: ActivationName,
) -> Result<(SingleLayerNetwork, Vec<TrainingStep>), String> {
    let (_, input_count) = validate_matrix("inputs", inputs)?;
    let (_, output_count) = validate_matrix("targets", targets)?;
    let mut network = SingleLayerNetwork::new(input_count, output_count, activation);
    let history = network.fit(inputs, targets, learning_rate, epochs)?;
    Ok((network, history))
}

#[cfg(test)]
mod tests {
    use super::*;

    fn near(actual: f64, expected: f64) {
        assert!((actual - expected).abs() <= 1e-6, "{actual} != {expected}");
    }

    #[test]
    fn one_epoch_exposes_matrix_gradients() {
        let step = train_one_epoch_with_matrices(
            &vec![vec![1.0, 2.0]],
            &vec![vec![3.0, 5.0]],
            &vec![vec![0.0, 0.0], vec![0.0, 0.0]],
            &[0.0, 0.0],
            0.1,
            ActivationName::Linear,
        )
        .unwrap();

        assert_eq!(step.predictions, vec![vec![0.0, 0.0]]);
        assert_eq!(step.errors, vec![vec![-3.0, -5.0]]);
        assert_eq!(step.weight_gradients, vec![vec![-3.0, -5.0], vec![-6.0, -10.0]]);
        near(step.next_weights[0][0], 0.3);
        near(step.next_weights[1][1], 1.0);
    }

    #[test]
    fn fit_learns_m_inputs_to_n_outputs() {
        let mut network = SingleLayerNetwork::new(3, 2, ActivationName::Linear);
        let history = network
            .fit(
                &vec![vec![0.0, 0.0, 1.0], vec![1.0, 2.0, 1.0], vec![2.0, 1.0, 1.0]],
                &vec![vec![1.0, -1.0], vec![3.0, 2.0], vec![4.0, 1.0]],
                0.05,
                500,
            )
            .unwrap();
        assert!(history.last().unwrap().loss < history.first().unwrap().loss);
        assert_eq!(network.predict(&vec![vec![1.0, 1.0, 1.0]]).unwrap()[0].len(), 2);
    }
}
