pub const VERSION: &str = "0.1.0";

pub type Matrix = Vec<Vec<f64>>;

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum ActivationName {
    Linear,
    Sigmoid,
}

#[derive(Clone, Debug, PartialEq)]
pub struct Parameters {
    pub input_to_hidden_weights: Matrix,
    pub hidden_biases: Vec<f64>,
    pub hidden_to_output_weights: Matrix,
    pub output_biases: Vec<f64>,
}

#[derive(Clone, Debug, PartialEq)]
pub struct ForwardPass {
    pub hidden_raw: Matrix,
    pub hidden_activations: Matrix,
    pub output_raw: Matrix,
    pub predictions: Matrix,
}

#[derive(Clone, Debug, PartialEq)]
pub struct TrainingStep {
    pub predictions: Matrix,
    pub errors: Matrix,
    pub output_deltas: Matrix,
    pub hidden_deltas: Matrix,
    pub hidden_to_output_weight_gradients: Matrix,
    pub output_bias_gradients: Vec<f64>,
    pub input_to_hidden_weight_gradients: Matrix,
    pub hidden_bias_gradients: Vec<f64>,
    pub next_parameters: Parameters,
    pub loss: f64,
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

fn derivative(_raw: f64, activated: f64, activation: ActivationName) -> f64 {
    match activation {
        ActivationName::Linear => 1.0,
        ActivationName::Sigmoid => activated * (1.0 - activated),
    }
}

fn dot(left: &Matrix, right: &Matrix) -> Result<Matrix, String> {
    let (rows, width) = validate_matrix("left", left)?;
    let (right_rows, cols) = validate_matrix("right", right)?;
    if width != right_rows {
        return Err("matrix shapes do not align".to_string());
    }
    Ok((0..rows)
        .map(|row| {
            (0..cols)
                .map(|col| (0..width).map(|k| left[row][k] * right[k][col]).sum())
                .collect()
        })
        .collect())
}

fn transpose(matrix: &Matrix) -> Matrix {
    let (rows, cols) = validate_matrix("matrix", matrix).unwrap();
    (0..cols)
        .map(|col| (0..rows).map(|row| matrix[row][col]).collect())
        .collect()
}

fn add_biases(matrix: Matrix, biases: &[f64]) -> Matrix {
    matrix
        .into_iter()
        .map(|row| {
            row.into_iter()
                .enumerate()
                .map(|(col, value)| value + biases[col])
                .collect()
        })
        .collect()
}

fn apply_activation(matrix: &Matrix, activation: ActivationName) -> Matrix {
    matrix
        .iter()
        .map(|row| {
            row.iter()
                .map(|value| activate(*value, activation))
                .collect()
        })
        .collect()
}

fn column_sums(matrix: &Matrix) -> Vec<f64> {
    let (_, cols) = validate_matrix("matrix", matrix).unwrap();
    (0..cols)
        .map(|col| matrix.iter().map(|row| row[col]).sum())
        .collect()
}

fn mean_squared_error(errors: &Matrix) -> f64 {
    let values: Vec<f64> = errors.iter().flat_map(|row| row.iter().copied()).collect();
    values.iter().map(|value| value * value).sum::<f64>() / values.len() as f64
}

fn subtract_scaled(matrix: &Matrix, gradients: &Matrix, learning_rate: f64) -> Matrix {
    matrix
        .iter()
        .enumerate()
        .map(|(row_index, row)| {
            row.iter()
                .enumerate()
                .map(|(col, value)| value - learning_rate * gradients[row_index][col])
                .collect()
        })
        .collect()
}

pub fn xor_warm_start_parameters() -> Parameters {
    Parameters {
        input_to_hidden_weights: vec![vec![4.0, -4.0], vec![4.0, -4.0]],
        hidden_biases: vec![-2.0, 6.0],
        hidden_to_output_weights: vec![vec![4.0], vec![4.0]],
        output_biases: vec![-6.0],
    }
}

pub fn forward(
    inputs: &Matrix,
    parameters: &Parameters,
    hidden_activation: ActivationName,
    output_activation: ActivationName,
) -> Result<ForwardPass, String> {
    let hidden_raw = add_biases(
        dot(inputs, &parameters.input_to_hidden_weights)?,
        &parameters.hidden_biases,
    );
    let hidden_activations = apply_activation(&hidden_raw, hidden_activation);
    let output_raw = add_biases(
        dot(&hidden_activations, &parameters.hidden_to_output_weights)?,
        &parameters.output_biases,
    );
    let predictions = apply_activation(&output_raw, output_activation);
    Ok(ForwardPass {
        hidden_raw,
        hidden_activations,
        output_raw,
        predictions,
    })
}

pub fn train_one_epoch(
    inputs: &Matrix,
    targets: &Matrix,
    parameters: &Parameters,
    learning_rate: f64,
    hidden_activation: ActivationName,
    output_activation: ActivationName,
) -> Result<TrainingStep, String> {
    let (sample_count, _) = validate_matrix("inputs", inputs)?;
    let (_, output_count) = validate_matrix("targets", targets)?;
    let passed = forward(inputs, parameters, hidden_activation, output_activation)?;
    let scale = 2.0 / (sample_count * output_count) as f64;
    let mut errors = vec![vec![0.0; output_count]; sample_count];
    let mut output_deltas = vec![vec![0.0; output_count]; sample_count];
    for row in 0..sample_count {
        for output in 0..output_count {
            let error = passed.predictions[row][output] - targets[row][output];
            errors[row][output] = error;
            output_deltas[row][output] = scale
                * error
                * derivative(
                    passed.output_raw[row][output],
                    passed.predictions[row][output],
                    output_activation,
                );
        }
    }
    let h2o_gradients = dot(&transpose(&passed.hidden_activations), &output_deltas)?;
    let output_bias_gradients = column_sums(&output_deltas);
    let hidden_errors = dot(
        &output_deltas,
        &transpose(&parameters.hidden_to_output_weights),
    )?;
    let hidden_width = parameters.hidden_biases.len();
    let mut hidden_deltas = vec![vec![0.0; hidden_width]; sample_count];
    for row in 0..sample_count {
        for hidden in 0..hidden_width {
            hidden_deltas[row][hidden] = hidden_errors[row][hidden]
                * derivative(
                    passed.hidden_raw[row][hidden],
                    passed.hidden_activations[row][hidden],
                    hidden_activation,
                );
        }
    }
    let i2h_gradients = dot(&transpose(inputs), &hidden_deltas)?;
    let hidden_bias_gradients = column_sums(&hidden_deltas);
    Ok(TrainingStep {
        predictions: passed.predictions,
        errors: errors.clone(),
        output_deltas,
        hidden_deltas,
        hidden_to_output_weight_gradients: h2o_gradients.clone(),
        output_bias_gradients: output_bias_gradients.clone(),
        input_to_hidden_weight_gradients: i2h_gradients.clone(),
        hidden_bias_gradients: hidden_bias_gradients.clone(),
        next_parameters: Parameters {
            input_to_hidden_weights: subtract_scaled(
                &parameters.input_to_hidden_weights,
                &i2h_gradients,
                learning_rate,
            ),
            hidden_biases: parameters
                .hidden_biases
                .iter()
                .enumerate()
                .map(|(i, bias)| bias - learning_rate * hidden_bias_gradients[i])
                .collect(),
            hidden_to_output_weights: subtract_scaled(
                &parameters.hidden_to_output_weights,
                &h2o_gradients,
                learning_rate,
            ),
            output_biases: parameters
                .output_biases
                .iter()
                .enumerate()
                .map(|(i, bias)| bias - learning_rate * output_bias_gradients[i])
                .collect(),
        },
        loss: mean_squared_error(&errors),
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    fn xor_inputs() -> Matrix {
        vec![
            vec![0.0, 0.0],
            vec![0.0, 1.0],
            vec![1.0, 0.0],
            vec![1.0, 1.0],
        ]
    }
    fn xor_targets() -> Matrix {
        vec![vec![0.0], vec![1.0], vec![1.0], vec![0.0]]
    }

    #[test]
    fn forward_pass_exposes_hidden_activations() {
        let passed = forward(
            &xor_inputs(),
            &xor_warm_start_parameters(),
            ActivationName::Sigmoid,
            ActivationName::Sigmoid,
        )
        .unwrap();
        assert_eq!(passed.hidden_activations.len(), 4);
        assert_eq!(passed.hidden_activations[0].len(), 2);
        assert!(passed.predictions[1][0] > 0.7);
        assert!(passed.predictions[0][0] < 0.3);
    }

    #[test]
    fn training_step_exposes_both_layer_gradients() {
        let step = train_one_epoch(
            &xor_inputs(),
            &xor_targets(),
            &xor_warm_start_parameters(),
            0.5,
            ActivationName::Sigmoid,
            ActivationName::Sigmoid,
        )
        .unwrap();
        assert_eq!(step.input_to_hidden_weight_gradients.len(), 2);
        assert_eq!(step.input_to_hidden_weight_gradients[0].len(), 2);
        assert_eq!(step.hidden_to_output_weight_gradients.len(), 2);
        assert_eq!(step.hidden_to_output_weight_gradients[0].len(), 1);
    }
}
