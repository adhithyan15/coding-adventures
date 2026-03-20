use loss_functions::{mse, mse_derivative, mae, mae_derivative};
use gradient_descent::sgd;

fn train(
    loss_name: &str,
    loss_fn: fn(&[f64], &[f64]) -> Result<f64, &'static str>,
    deriv_fn: fn(&[f64], &[f64]) -> Result<Vec<f64>, &'static str>,
    learning_rate: f64,
    epochs: usize,
) {
    let celsius = &[-40.0, -10.0, 0.0, 8.0, 15.0, 22.0, 38.0];
    let fahrenheit = &[-40.0, 14.0, 32.0, 46.4, 59.0, 71.6, 100.4];

    let mut w = 0.5;
    let mut b = 0.5;

    println!("\n--- Celsius to Fahrenheit Predictor: Training with {} ---", loss_name);

    for epoch in 0..epochs {
        let mut y_pred = Vec::with_capacity(celsius.len());
        for c in celsius {
            y_pred.push(w * c + b);
        }

        let err = loss_fn(fahrenheit, &y_pred).unwrap();

        if err < 0.5 {
            println!("Converged beautifully in {} epochs! (Loss: {:.6})", epoch + 1, err);
            println!("Final Formula: F = C * {:.6} + {:.6}", w, b);
            break;
        }

        let gradients = deriv_fn(fahrenheit, &y_pred).unwrap();

        let mut grad_w = 0.0;
        let mut grad_b = 0.0;
        for i in 0..gradients.len() {
            grad_w += gradients[i] * celsius[i];
            grad_b += gradients[i];
        }

        let new_params = sgd(&[w, b], &[grad_w, grad_b], learning_rate).unwrap();
        w = new_params[0];
        b = new_params[1];

        if (epoch + 1) % 1000 == 0 {
            println!("Epoch {:04} -> Loss: {:.6} | w: {:.4} | b: {:.4}", epoch + 1, err, w, b);
        }
    }

    let pred_f = w * 100.0 + b;
    println!("Prediction for 100.0 C -> {:.2} F (Expected ~212.00 F)", pred_f);
}

fn main() {
    train("Mean Squared Error (MSE)", mse, mse_derivative, 0.0005, 10000);
    train("Mean Absolute Error (MAE)", mae, mae_derivative, 0.01, 10000);
}
