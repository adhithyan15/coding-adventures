"""
Multi-Variable Linear Regression: House Price Predictor
-------------------------------------------------------
This standalone program demonstrates how to utilize our foundational 'matrix' 
and 'loss-functions' packages to execute gradient descent over multiple variables dynamically.

Formula:
    Final Predictions = House Features • Feature Weights + Base Price Bias
"""
import sys
from matrix import Matrix
from loss_functions import mse as mse_loss

def train():
    print("\n--- Booting Multi-Variable Predictor: House Prices ---\n")
    
    # 1. The Dataset (Inputs)
    # Column 0: Square Footage (in 1000s)
    # Column 1: Number of Bedrooms
    house_features_data = [
        [2.0, 3.0], # House 1: 2000 SqFt, 3 Beds
        [1.5, 2.0], # House 2: 1500 SqFt, 2 Beds
        [2.5, 4.0], # House 3: 2500 SqFt, 4 Beds
        [1.0, 1.0]  # House 4: 1000 SqFt, 1 Bed
    ]
    
    # 2. The Target Values (Outputs) represent the actual property value (in $1000s).
    true_prices_data = [
        [400.0],
        [300.0],
        [500.0],
        [200.0]
    ]
    
    # Instantiate them safely using our natively built OOP Matrix SDK!
    house_features = Matrix(house_features_data)
    true_prices = Matrix(true_prices_data)
    
    # 3. Model Parameters
    # We initialize a 2x1 Weights column vector dynamically.
    feature_weights = Matrix([[0.5], [0.5]])
    base_price_bias = 0.5  # Represents the base price of land unconditionally
    
    # Learning Rate controls step-size down the gradient slope explicitly.
    learning_rate = 0.01 
    training_epochs = 1500
    
    print("Beginning Training Epochs...")
    for epoch in range(training_epochs + 1):
        
        # --- THE FORWARD PASS --- #
        # house_features (4x2 Matrix) DOT feature_weights (2x1 Vector) => Result is a 4x1 Prediction Vector!
        final_predictions = house_features.dot(feature_weights) + base_price_bias
        
        # --- LOSS CALCULATION --- #
        # We flatten the vectors out into 1D lists to feed our pure MSE mathematical mapper smoothly.
        linear_true_prices = [row[0] for row in true_prices.data]
        linear_predictions = [row[0] for row in final_predictions.data]
        mean_squared_error = mse_loss(linear_true_prices, linear_predictions)
        
        # --- BACKPROPAGATION (CALCULATING GRADIENTS) --- #
        # How do we figure out exactly how much the SqFt Weight vs Bedroom Weight was responsible for the error?
        # 1. We take our original (4 BY 2) Data Grid and physically flip it on its side to become (2 BY 4). 
        #    - Row 1 now contains only SqFt values. Row 2 contains only Bedroom values.
        # 2. We Dot Product this (2 BY 4) transposed grid against our (4 BY 1) Error Vector!
        #    - This multiplies every single SqFt value by its respective Error, collapsing into a (2 BY 1) Gradient Vector.
        prediction_errors = final_predictions - true_prices
        
        # We multiply by (2 / N) because of the Mean Squared Error derivative scaling.
        transposed_features = house_features.transpose()
        weight_gradients = transposed_features.dot(prediction_errors) * (2.0 / true_prices.rows)
        
        # For the Bias, because it shifts the prediction unconditionally for every house,
        # its "share" of the blame is simply the average of all the mistakes combined!
        bias_gradient = sum(prediction_errors.data[i][0] for i in range(prediction_errors.rows)) * (2.0 / true_prices.rows)
        
        # --- OPTIMIZATION STEP --- #
        # Finally, we take our original Weights and Bias and nudge them against the slope.
        # We multiply by our Learning Rate (0.01) which acts as a safety brake so we don't explode to infinity.
        feature_weights = feature_weights - (weight_gradients * learning_rate)
        base_price_bias = base_price_bias - (bias_gradient * learning_rate)
        
        if epoch % 150 == 0:
            print(f"Epoch {epoch:4d} | Global Loss: {mean_squared_error:10.4f} | Weights [SqFt W: {feature_weights.data[0][0]:6.2f}, Bedrm W: {feature_weights.data[1][0]:6.2f}] | Bias: {base_price_bias:6.2f}")
            
    print("\nFinal Optimal Mapping Achieved!")
    final_prediction = (house_features.dot(feature_weights) + base_price_bias).data[0][0]
    print(f"Prediction for House 1 (Target $400k): ${final_prediction:.2f}k")

if __name__ == "__main__":
    train()
