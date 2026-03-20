use matrix::Matrix;
use loss_functions::{bce, bce_derivative};
use activation_functions::{sigmoid, sigmoid_derivative};

fn main() {
    println!("\n--- Booting Rust Space Launch Predictor ---");

    let shuttle_data = vec![
        vec![12.0, 15.0], vec![35.0, 85.0], vec![5.0, 5.0],
        vec![40.0, 95.0], vec![15.0, 30.0], vec![28.0, 60.0]
    ];
    let target_data = vec![
        vec![1.0], vec![0.0], vec![1.0], vec![0.0], vec![1.0], vec![0.0]
    ];

    let features = Matrix::new_2d(shuttle_data);
    let true_labels = Matrix::new_2d(target_data);

    let mut weights = Matrix::new_2d(vec![vec![0.0], vec![0.0]]);
    let mut bias = 0.0;
    let lr = 0.01;
    let epochs = 3000;

    for epoch in 0..=epochs {
        let mut raw = features.dot(&weights).unwrap();
        raw.add_scalar(bias);

        let mut linear_probs = Vec::with_capacity(features.rows);
        let mut linear_truth = Vec::with_capacity(features.rows);
        let mut grad_data = Vec::with_capacity(features.rows);

        for i in 0..features.rows {
            let p = sigmoid(raw.data[i][0]);
            linear_probs.push(p);
            linear_truth.push(true_labels.data[i][0]);
        }

        let log_loss = bce(&linear_truth, &linear_probs).unwrap();
        let loss_grad = bce_derivative(&linear_truth, &linear_probs).unwrap();

        let mut bias_grad = 0.0;
        for i in 0..features.rows {
            let act_grad = sigmoid_derivative(raw.data[i][0]);
            let combined = loss_grad[i] * act_grad;
            grad_data.push(vec![combined]);
            bias_grad += combined;
        }

        let grad_matrix = Matrix::new_2d(grad_data);
        let transposed = features.transpose();
        let weight_grads = transposed.dot(&grad_matrix).unwrap();

        let scaled_weights = weight_grads.scale(lr);
        weights = weights.subtract(&scaled_weights).unwrap();
        bias -= bias_grad * lr;

        if epoch % 500 == 0 {
            println!("Epoch {:4} | BCE Loss: {:.4} | Bias: {:.2}", epoch, log_loss, bias);
        }
    }

    println!("\n--- Final Inference ---");
    let mut final_raw = features.dot(&weights).unwrap();
    final_raw.add_scalar(bias);
    for i in 0..true_labels.rows {
        let prob = sigmoid(final_raw.data[i][0]);
        let guess = if prob > 0.5 { "Safe" } else { "Abort" };
        let truth = if true_labels.data[i][0] == 1.0 { "Safe" } else { "Abort" };
        println!("Scenario {} (Truth: {}) -> System: {} ({:.2}%)", i+1, truth, guess, prob * 100.0);
    }
}
