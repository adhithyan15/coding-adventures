//! Multi-Variable Linear Regression: House Price Predictor
//! -------------------------------------------------------
//! Written thoroughly inside pure native Rust executing flawless functional evaluation 
//! maps explicitly using descriptive variable designations natively accurately!

use matrix::Matrix;
use loss_functions::mse as mean_squared_error;

fn main() {
    println!("\n--- Booting Multi-Variable Predictor: House Prices ---\n");

    // 1. Defining standard dimensional nodes natively using Vector grids!
    let house_features = Matrix::new_2d(vec![
        vec![2.0, 3.0],
        vec![1.5, 2.0],
        vec![2.5, 4.0],
        vec![1.0, 1.0],
    ]);

    let true_prices = Matrix::new_2d(vec![
        vec![400.0],
        vec![300.0],
        vec![500.0],
        vec![200.0],
    ]);

    // 2. Setting weight properties descriptively natively.
    let mut feature_weights = Matrix::new_2d(vec![vec![0.5], vec![0.5]]);
    let mut base_price_bias: f64 = 0.5;
    let learning_rate: f64 = 0.01;

    println!("Beginning Training Epochs...");
    for epoch in 0..=1500 {
        
        // --- FORWARD MATRICES --
        let raw_predictions = house_features.dot(&feature_weights).unwrap();
        let final_predictions = raw_predictions.add_scalar(base_price_bias);

        let linear_true_prices: Vec<f64> = true_prices.data.iter().map(|r| r[0]).collect();
        let linear_predictions: Vec<f64> = final_predictions.data.iter().map(|r| r[0]).collect();
        let mse_loss = mean_squared_error(&linear_true_prices, &linear_predictions).unwrap();

        // --- BACKPROPAGATION (CALCULATING GRADIENTS) ---
        // How do we figure out exactly how much the SqFt Weight vs Bedroom Weight was responsible for the error?
        // 1. We take our original (N BY 2) Data Grid and physically flip it on its side to become (2 BY N). 
        //    - Row 1 now contains only SqFt values. Row 2 contains only Bedroom values.
        // 2. We Dot Product this (2 BY N) grid against our (N BY 1) Error Vector!
        let prediction_errors = final_predictions.subtract(&true_prices).unwrap();
        let transposed_features = house_features.transpose();
        let features_dot_errors = transposed_features.dot(&prediction_errors).unwrap();
        
        // We multiply by (2 / N) because of the Mean Squared Error derivative scaling.
        let weight_gradients = features_dot_errors.scale(2.0 / true_prices.rows as f64);

        // For the Bias, because it shifts the prediction unconditionally for every house,
        // its "share" of the blame is simply the average of all the mistakes combined!
        let mut bias_gradient_total = 0.0;
        for i in 0..prediction_errors.rows {
            bias_gradient_total += prediction_errors.data[i][0];
        }
        let bias_gradient = bias_gradient_total * (2.0 / true_prices.rows as f64);

        // --- OPTIMIZATION STEP ---
        // Finally, we take our original Weights and Bias and nudge them against the slope.
        // We multiply by our Learning Rate (0.01) which acts as a safety brake so we don't explode.
        let scaled_weight_gradients = weight_gradients.scale(learning_rate);
        feature_weights = feature_weights.subtract(&scaled_weight_gradients).unwrap();
        base_price_bias -= bias_gradient * learning_rate;

        if epoch % 150 == 0 {
            println!(
                "Epoch {:4} | Global Loss: {:10.4} | Weights [SqFt: {:5.2}, Bed: {:5.2}] | Bias: {:5.2}",
                epoch, mse_loss, feature_weights.data[0][0], feature_weights.data[1][0], base_price_bias
            );
        }
    }
    println!("\nFinal Optimal Mapping Achieved!");
    let prediction = house_features.dot(&feature_weights).unwrap().add_scalar(base_price_bias).data[0][0];
    println!("Prediction for House 1 (Target $400k): ${:.2}k", prediction);
}
