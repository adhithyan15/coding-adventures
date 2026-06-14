use activation_functions::{sigmoid, sigmoid_derivative};
use loss_functions::{bce, bce_derivative};
use matrix::Matrix;

pub struct Perceptron {
    pub learning_rate: f64,
    pub epochs: usize,
    pub weights: Option<Matrix>,
    pub bias: f64,
}

impl Perceptron {
    pub fn new(lr: f64, epochs: usize) -> Self {
        Self {
            learning_rate: lr,
            epochs: epochs,
            weights: None,
            bias: 0.0,
        }
    }

    pub fn fit(&mut self, x_data: Vec<Vec<f64>>, y_data: Vec<Vec<f64>>, log_steps: usize) {
        let features = Matrix::new_2d(x_data);
        let true_labels = Matrix::new_2d(y_data);

        let mut w_data = vec![];
        for _ in 0..features.cols {
            w_data.push(vec![0.0]);
        }
        self.weights = Some(Matrix::new_2d(w_data));
        self.bias = 0.0;

        for epoch in 0..=self.epochs {
            let raw = features
                .dot(self.weights.as_ref().unwrap())
                .unwrap()
                .add_scalar(self.bias);

            let mut linear_probs = vec![0.0; features.rows];
            let mut linear_truth = vec![0.0; features.rows];

            for i in 0..features.rows {
                linear_probs[i] = sigmoid(raw.data[i][0]);
                linear_truth[i] = true_labels.data[i][0];
            }

            let log_loss = bce(&linear_truth, &linear_probs).unwrap();
            let loss_grad = bce_derivative(&linear_truth, &linear_probs).unwrap();

            let mut grad_data = vec![];
            let mut bias_grad = 0.0;

            for i in 0..features.rows {
                let act_grad = sigmoid_derivative(raw.data[i][0]);
                let combined = loss_grad[i] * act_grad;
                grad_data.push(vec![combined]);
                bias_grad += combined;
            }

            let grad_matrix = Matrix::new_2d(grad_data);
            let weight_grads = features.transpose().dot(&grad_matrix).unwrap();

            let scaled_weights = weight_grads.scale(self.learning_rate);
            self.weights = Some(
                self.weights
                    .as_ref()
                    .unwrap()
                    .subtract(&scaled_weights)
                    .unwrap(),
            );
            self.bias -= bias_grad * self.learning_rate;

            if epoch % log_steps == 0 {
                println!(
                    "Epoch {:4} | BCE Loss: {:.4} | Bias: {:.2}",
                    epoch, log_loss, self.bias
                );
            }
        }
    }

    pub fn predict(&self, x_data: Vec<Vec<f64>>) -> Vec<f64> {
        let features = Matrix::new_2d(x_data);
        let raw = features
            .dot(self.weights.as_ref().unwrap())
            .unwrap()
            .add_scalar(self.bias);

        let mut predictions = vec![0.0; features.rows];
        for i in 0..features.rows {
            predictions[i] = sigmoid(raw.data[i][0]);
        }
        predictions
    }
}
