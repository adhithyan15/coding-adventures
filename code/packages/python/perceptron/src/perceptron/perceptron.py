from matrix.matrix import Matrix
from loss_functions.functions import bce, bce_derivative
from activation_functions.activations import sigmoid, sigmoid_derivative

class Perceptron:
    def __init__(self, learning_rate: float = 0.1, epochs: int = 2000):
        self.lr = learning_rate
        self.epochs = epochs
        self.weights = None
        self.bias = 0.0

    def fit(self, X_train: list, Y_train: list, log_steps: int = 400):
        features = Matrix(X_train)
        true_labels = Matrix(Y_train)
        
        # Initialize Weights (Columns match Feature Dimension, Rows=1 per neuron mapping)
        self.weights = Matrix.zeros(features.cols, 1)
        self.bias = 0.0

        for epoch in range(self.epochs + 1):
            # Forward Pass
            raw_preds = features.dot(self.weights) + self.bias
            linear_probs = [sigmoid(raw_preds.data[i][0]) for i in range(features.rows)]
            linear_truth = [true_labels.data[i][0] for i in range(features.rows)]

            # Loss computation
            log_loss = bce(linear_truth, linear_probs)
            loss_grad = bce_derivative(linear_truth, linear_probs)

            # Combined Gradients (d_loss * d_act)
            combined_grad = []
            bias_grad = 0.0
            for i in range(features.rows):
                act_grad = sigmoid_derivative(raw_preds.data[i][0])
                grad_val = loss_grad[i] * act_grad
                combined_grad.append([grad_val])
                bias_grad += grad_val

            grad_matrix = Matrix(combined_grad)
            weight_gradients = features.transpose().dot(grad_matrix)

            # Update Rules
            scaled_weights = weight_gradients * self.lr
            self.weights = self.weights - scaled_weights
            self.bias -= bias_grad * self.lr

            if epoch % log_steps == 0:
                print(f"Epoch {epoch:4} | BCE Loss: {log_loss:.4f} | Bias: {self.bias:.2f}")

    def predict(self, X_data: list) -> list:
        if self.weights is None:
            raise ValueError("Perceptron has not been trained yet. Call .fit() first.")
        
        features = Matrix(X_data)
        raw_preds = features.dot(self.weights) + self.bias
        return [sigmoid(raw_preds.data[i][0]) for i in range(features.rows)]
