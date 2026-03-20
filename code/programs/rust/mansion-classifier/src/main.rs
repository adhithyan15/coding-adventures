use matrix::Matrix;
use loss_functions::{bce, bce_derivative};
use activation_functions::{sigmoid, sigmoid_derivative};

fn main() {
    println!("\n--- Booting Rust Mansion Classifier ---");

    let house_data = vec![
        vec![4.5, 6.0], vec![3.8, 5.0], vec![1.5, 2.0],
        vec![0.9, 1.0], vec![5.5, 7.0], vec![2.0, 3.0]
    ];
    let target_data = vec![
        vec![1.0], vec![1.0], vec![0.0], vec![0.0], vec![1.0], vec![0.0]
    ];

    let features = Matrix::new_2d(house_data);
    let true_labels = Matrix::new_2d(target_data);

    let mut weights = Matrix::new_2d(vec![vec![0.0], vec![0.0]]);
    let mut bias = 0.0;
    let lr = 0.1;
    let epochs = 2000;

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

        if epoch % 400 == 0 {
            println!("Epoch {:4} | BCE Loss: {:.4} | Bias: {:.2}", epoch, log_loss, bias);
        }
    }

    println!("\n--- Final Matrix Probability Inferences ---");
    let mut final_raw = features.dot(&weights).unwrap();
    final_raw.add_scalar(bias);
    for i in 0..true_labels.rows {
        let prob = sigmoid(final_raw.data[i][0]);
        let guess = if prob > 0.5 { "Mansion" } else { "Normal" };
        let truth = if true_labels.data[i][0] == 1.0 { "Mansion" } else { "Normal" };
        println!("House {} (Truth: {}) -> System: {} ({:.2}%)", i+1, truth, guess, prob * 100.0);
    }
}
