"""
Binary Classification: Space Launch Predictor
-------------------------------------------
Predicting Absolute Launch Viability using our native Activation frameworks.
"""
import sys
from matrix import Matrix
from loss_functions import bce, bce_derivative
from activation_functions import sigmoid, sigmoid_derivative

def train():
    print("\n--- Booting Binary Classifier: Space Launch Telemetry ---\n")
    
    # 1. Dataset (Features: X)
    # Col 0: Wind Speed (MPH)
    # Col 1: Cloud Cover (%)
    launch_telemetry_data = [
        [12.0, 15.0], # Safe
        [35.0, 85.0], # Abort
        [5.0, 5.0],   # Safe
        [40.0, 95.0], # Abort
        [15.0, 30.0], # Safe
        [28.0, 60.0]  # Abort
    ]
    
    # Target Labels (1.0 = Safe, 0.0 = Abort)
    launch_decisions_data = [
        [1.0], [0.0], [1.0], [0.0], [1.0], [0.0]
    ]
    
    telemetry = Matrix(launch_telemetry_data)
    decisions = Matrix(launch_decisions_data)
    
    # Parameters
    weights = Matrix([[0.0], [0.0]])
    bias = 0.0
    learning_rate = 0.01
    training_epochs = 3000
    
    for epoch in range(training_epochs + 1):
        # FORWARD PASS
        raw = telemetry.dot(weights) + bias
        probs = [[sigmoid(raw.data[i][0])] for i in range(raw.rows)]
        prob_matrix = Matrix(probs)
        
        # LOSS
        linear_labels = [row[0] for row in decisions.data]
        linear_probs = [row[0] for row in prob_matrix.data]
        log_loss = bce(linear_labels, linear_probs)
        
        # BACKWARD PASS
        loss_grad = bce_derivative(linear_labels, linear_probs)
        act_grad = [sigmoid_derivative(raw.data[i][0]) for i in range(raw.rows)]
        
        combined_grad = [[loss_grad[i] * act_grad[i]] for i in range(raw.rows)]
        grad_matrix = Matrix(combined_grad)
        
        weight_gradients = telemetry.transpose().dot(grad_matrix)
        bias_gradient = sum(grad_matrix.data[i][0] for i in range(grad_matrix.rows))
        
        weights = weights - (weight_gradients * learning_rate)
        bias = bias - (bias_gradient * learning_rate)
        
        if epoch % 500 == 0:
             print(f"Epoch {epoch:4d} | Global Log Loss: {log_loss:8.4f} | Weights [Wind: {weights.data[0][0]:5.4f}, Cloud: {weights.data[1][0]:5.4f}] | Bias: {bias:5.2f}")

    print("\n--- Final Launch Probabilities ---")
    final_raw = telemetry.dot(weights) + bias
    for i in range(decisions.rows):
        prob = sigmoid(final_raw.data[i][0])
        target = "Safe" if launch_decisions_data[i][0] == 1.0 else "Abort"
        guess = "Safe" if prob > 0.5 else "Abort"
        print(f"Scenario {i+1} (Truth: {target}) -> System: {guess} ({prob*100:.2f}%)")

if __name__ == "__main__":
    train()
