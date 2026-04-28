//! Multi-Variable Linear Regression: House Price Predictor
//! -------------------------------------------------------
//! Demonstrates n input features -> one output with feature normalization and
//! a short learning-rate sweep before the full training run.

use feature_normalization::{fit_standard_scaler, transform_standard};
use loss_functions::mse as mean_squared_error;
use matrix::Matrix;

#[derive(Clone)]
struct TrainingResult {
    learning_rate: f64,
    loss: f64,
    diverged: bool,
    weights: Matrix,
    bias: f64,
}

fn house_features_data() -> Vec<Vec<f64>> {
    vec![
        vec![2000.0, 3.0],
        vec![1500.0, 2.0],
        vec![2500.0, 4.0],
        vec![1000.0, 1.0],
    ]
}

fn true_prices_data() -> Vec<Vec<f64>> {
    vec![vec![400.0], vec![300.0], vec![500.0], vec![200.0]]
}

fn run_training(
    features_data: &[Vec<f64>],
    prices_data: &[Vec<f64>],
    learning_rate: f64,
    epochs: usize,
    log_every: Option<usize>,
) -> TrainingResult {
    let house_features = Matrix::new_2d(features_data.to_vec());
    let true_prices = Matrix::new_2d(prices_data.to_vec());
    let mut feature_weights = Matrix::new_2d(vec![vec![0.5], vec![0.5]]);
    let mut base_price_bias = 0.0;
    let mut last_loss = f64::INFINITY;

    for epoch in 0..=epochs {
        let final_predictions = house_features
            .dot(&feature_weights)
            .unwrap()
            .add_scalar(base_price_bias);
        let linear_true_prices: Vec<f64> = true_prices.data.iter().map(|row| row[0]).collect();
        let linear_predictions: Vec<f64> =
            final_predictions.data.iter().map(|row| row[0]).collect();
        last_loss = mean_squared_error(&linear_true_prices, &linear_predictions).unwrap();

        if !last_loss.is_finite() || last_loss > 1.0e12 {
            return TrainingResult {
                learning_rate,
                loss: f64::INFINITY,
                diverged: true,
                weights: feature_weights,
                bias: base_price_bias,
            };
        }

        if log_every.is_some_and(|every| epoch % every == 0) {
            println!(
                "Epoch {:4} | Loss: {:10.4} | Weights [SqFt: {:7.3}, Beds: {:7.3}] | Bias: {:7.3}",
                epoch,
                last_loss,
                feature_weights.data[0][0],
                feature_weights.data[1][0],
                base_price_bias
            );
        }

        let prediction_errors = final_predictions.subtract(&true_prices).unwrap();
        let weight_gradients = house_features
            .transpose()
            .dot(&prediction_errors)
            .unwrap()
            .scale(2.0 / true_prices.rows as f64);

        let bias_gradient_total: f64 = prediction_errors.data.iter().map(|row| row[0]).sum();
        let bias_gradient = bias_gradient_total * (2.0 / true_prices.rows as f64);

        feature_weights = feature_weights
            .subtract(&weight_gradients.scale(learning_rate))
            .unwrap();
        base_price_bias -= bias_gradient * learning_rate;
    }

    TrainingResult {
        learning_rate,
        loss: last_loss,
        diverged: false,
        weights: feature_weights,
        bias: base_price_bias,
    }
}

fn find_learning_rate(features_data: &[Vec<f64>], prices_data: &[Vec<f64>]) -> TrainingResult {
    let candidates = [0.001, 0.003, 0.01, 0.03, 0.1, 0.3, 0.6];
    let results: Vec<TrainingResult> = candidates
        .iter()
        .map(|learning_rate| run_training(features_data, prices_data, *learning_rate, 120, None))
        .collect();

    println!("\nShort learning-rate sweep over normalized features:");
    for result in &results {
        if result.diverged {
            println!("  lr={:<6} -> loss=diverged", result.learning_rate);
        } else {
            println!(
                "  lr={:<6} -> loss={:.4}",
                result.learning_rate, result.loss
            );
        }
    }

    results
        .into_iter()
        .filter(|result| !result.diverged)
        .min_by(|a, b| a.loss.total_cmp(&b.loss))
        .unwrap()
}

fn main() {
    println!("\n--- Booting Multi-Variable Predictor: House Prices ---");
    println!("Features: square footage and bedroom count. Target: price in $1000s.");

    let raw_features = house_features_data();
    let true_prices = true_prices_data();
    let scaler = fit_standard_scaler(&raw_features).unwrap();
    let normalized_features = transform_standard(&raw_features, &scaler).unwrap();
    let best_trial = find_learning_rate(&normalized_features, &true_prices);

    println!("\nSelected learning rate: {}", best_trial.learning_rate);
    println!("Beginning full training run...");
    let final_result = run_training(
        &normalized_features,
        &true_prices,
        best_trial.learning_rate,
        1500,
        Some(150),
    );

    println!("\nFinal Optimal Mapping Achieved!");
    let normalized_test_house = transform_standard(&[vec![2000.0, 3.0]], &scaler).unwrap();
    let prediction = Matrix::new_2d(normalized_test_house)
        .dot(&final_result.weights)
        .unwrap()
        .add_scalar(final_result.bias)
        .data[0][0];
    println!("Prediction for House 1 (Target $400k): ${:.2}k", prediction);
}
