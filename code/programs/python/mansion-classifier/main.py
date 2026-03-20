"""
Binary Classification: Mansion Classifier
-----------------------------------------
A foundational Machine Learning loop explicitly composed of 3 modular library components:
1. `matrix` SDK for pure Matrix structures and dimension matching
2. `activation-functions` SDK to map Matrix projections onto strict 1.0 - 0.0 Probabilities
3. `loss-functions` SDK to mathematically derive Binary Cross Entropy errors.

Goal: Predict whether a house is a Mansion (1) or Normal Home (0) based purely on features.
"""
import sys
from matrix import Matrix
from loss_functions import bce, bce_derivative
from activation_functions import sigmoid, sigmoid_derivative

def train():
    print("\n--- Booting Binary Classifier: Identifying Mansions ---\n")
    
    # 1. The Dataset (Features: X)
    # Column 0: Square Footage (in 1000s)
    # Column 1: Number of Bedrooms
    house_features_data = [
        [4.5, 6.0], # 4500 SqFt, 6 Beds -> Mansion
        [3.8, 5.0], # 3800 SqFt, 5 Beds -> Mansion
        [1.5, 2.0], # 1500 SqFt, 2 Beds -> Normal
        [0.9, 1.0], # 900 SqFt, 1 Bed   -> Normal
        [5.5, 7.0], # 5500 SqFt, 7 Beds -> Mansion
        [2.0, 3.0]  # 2000 SqFt, 3 Beds -> Normal
    ]
    
    # 2. The Target Labels (Outputs: Y)
    # 1.0 = Mansion, 0.0 = Normal Home
    true_labels_data = [
        [1.0], [1.0], [0.0], [0.0], [1.0], [0.0]
    ]
    
    # Instantiate OOP Matrices!
    house_features = Matrix(house_features_data)
    true_labels = Matrix(true_labels_data)
    
    # 3. Mathematical Parameters
    feature_weights = Matrix([[0.0], [0.0]])
    base_probability_bias = 0.0
    learning_rate = 0.1 # Increased slightly for classification dynamics
    training_epochs = 2000
    
    print("Beginning Training Epochs...")
    for epoch in range(training_epochs + 1):
        
        # --- THE FORWARD PASS ---
        # Matrix Dot Product produces raw, infinite numeric bounds
        raw_predictions = house_features.dot(feature_weights) + base_probability_bias
        
        # We natively map our Sigmoid pure-functional module onto the raw outputs!
        # This elegantly clamps the predictions specifically between 0.0 (0%) and 1.0 (100%).
        final_probabilities = [
            [sigmoid(raw_predictions.data[i][0])] for i in range(raw_predictions.rows)
        ]
        prob_matrix = Matrix(final_probabilities)
        
        # --- LOSS EVALUATION ---
        # Flattens structures to feed our stateless BCE error parser securely
        linear_true_labels = [row[0] for row in true_labels.data]
        linear_probabilities = [row[0] for row in prob_matrix.data]
        log_loss = bce(linear_true_labels, linear_probabilities)
        
        # --- THE BACKWARD PASS (CHAIN RULE CALCULATION) ---
        # 1. How fiercely should we panic? Calculate the Log Loss gradient relative to probabilities.
        # This maps cleanly inside our `loss-functions` BCE algorithm implicitly!
        loss_gradients = bce_derivative(linear_true_labels, linear_probabilities)
        
        # 2. How much of that panic belongs to the Sigmoid structural clamp itself?
        activation_gradients = [
            sigmoid_derivative(raw_predictions.data[i][0]) for i in range(raw_predictions.rows)
        ]
        
        # 3. Multiplying them together generates our final "Combined Prediction Gradient" Array (Nx1)
        # Note: Due to mathematical magic, BCE (y - t)/(y * (1 - y)) multiplied by Sigmoid y * (1 - y)
        # perfectly cancels out the non-linear denominator globally!
        combined_gradients = [
            [loss_gradients[i] * activation_gradients[i]] for i in range(raw_predictions.rows)
        ]
        gradient_matrix = Matrix(combined_gradients)
        
        # --- GRADIENT DESCENT ---
        # Executing standard Transposition mapping using the unified Chain Rule error values!
        transposed_features = house_features.transpose()
        weight_gradients = transposed_features.dot(gradient_matrix) # Note: Our BCE inherently divides by N optimally!
        
        bias_gradient = sum(gradient_matrix.data[i][0] for i in range(gradient_matrix.rows))
        
        # Optimization Jump natively
        feature_weights = feature_weights - (weight_gradients * learning_rate)
        base_probability_bias = base_probability_bias - (bias_gradient * learning_rate)
        
        if epoch % 400 == 0:
             print(f"Epoch {epoch:4d} | Global Log Loss: {log_loss:8.4f} | Weights [SqFt: {feature_weights.data[0][0]:5.2f}, Bed: {feature_weights.data[1][0]:5.2f}] | Bias: {base_probability_bias:5.2f}")

    print("\n--- Final Matrix Probability Inferences ---")
    final_raw = house_features.dot(feature_weights) + base_probability_bias
    for i in range(true_labels.rows):
        prob = sigmoid(final_raw.data[i][0])
        target = "Mansion" if true_labels_data[i][0] == 1.0 else "Normal"
        guess = "Mansion" if prob > 0.5 else "Normal"
        confidence = prob * 100 if prob > 0.5 else (1.0 - prob) * 100
        print(f"House {i+1} (Truth: {target}) -> Predicted: {guess} ({confidence:.2f}% Confidence)")

if __name__ == "__main__":
    train()
