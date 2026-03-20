"""
Multi-Variable Linear Regression: House Price Predictor
-------------------------------------------------------
This standalone program demonstrates how to utilize our foundational 'matrix' 
and 'loss-functions' packages to execute gradient descent over multiple variables dynamically.

Unlike our previous Celsius predictor (which only had 1 variable limit), this architecture
maps an entire Data Grid (X) against a vector of Weights (W).

Formula:
    Y_pred = X • W + b
"""
import sys
from matrix import Matrix
from loss_functions import mse as mse_loss

def train():
    print("\n--- Booting Multi-Variable Predictor: House Prices ---\n")
    
    # 1. The Dataset (Inputs)
    # X represents our features. Each row is a specific house in the layout.
    # Column 0: Square Footage (in 1000s)
    # Column 1: Number of Bedrooms
    X_data = [
        [2.0, 3.0], # House 1: 2000 SqFt, 3 Beds
        [1.5, 2.0], # House 2: 1500 SqFt, 2 Beds
        [2.5, 4.0], # House 3: 2500 SqFt, 4 Beds
        [1.0, 1.0]  # House 4: 1000 SqFt, 1 Bed
    ]
    
    # 2. The Target Values (Outputs)
    # Y represents the actual property value (in $1000s).
    Y_data = [
        [400.0],
        [300.0],
        [500.0],
        [200.0]
    ]
    
    # Instantiate them safely using our natively built OOP Matrix SDK!
    X = Matrix(X_data)
    Y = Matrix(Y_data)
    
    # 3. Model Parameters
    # We initialize a 2x1 Weights column vector dynamically.
    # W[0] tracks SqFt impact, W[1] tracks Bedroom impact.
    W = Matrix([[0.5], [0.5]])
    b = 0.5  # Base Bias (represents the base price of land unconditionally)
    
    # Learning Rate controls step-size down the gradient slope mathematical bounds.
    lr = 0.01 
    epochs = 1500
    
    print("Beginning Training Epochs...")
    for epoch in range(epochs + 1):
        
        # --- THE FORWARD PASS --- #
        # We multiply the entire dataset mathematically simultaneously.
        # Y_pred = (4x2 Matrix) DOT (2x1 Vector) => Result is a 4x1 Prediction Vector!
        # We can dynamically use the `+` operator overloads handled explicitly by Python's __add__ hooks!
        Y_pred = X.dot(W) + b
        
        # --- LOSS CALCULATION --- #
        # We flatten the (4x1) vectors out into 1D lists to feed our pure MSE mathematical mapper smoothly.
        y_true_list = [row[0] for row in Y.data]
        y_pred_list = [row[0] for row in Y_pred.data]
        loss = mse_loss(y_true_list, y_pred_list)
        
        # --- BACKPROPAGATION (CALCULATING GRADIENTS) --- #
        # How do we figure out exactly how much the SqFt Weight vs Bedroom Weight was responsible for the error?
        # 1. We take our original (N BY 2) Data Grid (X) and physically flip it on its side to become (2 BY N). 
        #    - Row 1 now contains only SqFt values. Row 2 contains only Bedroom values.
        # 2. We Dot Product this (2 BY N) grid against our (N BY 1) Error Vector!
        #    - This multiplies every single SqFt value by its respective Error, collapsing into a (2 BY 1) Gradient Vector.
        error = Y_pred - Y
        
        # We multiply by (2 / N) because of the Mean Squared Error derivative scaling.
        dW = X.transpose().dot(error) * (2.0 / Y.rows)
        
        # For the Bias (b), because it shifts the prediction unconditionally for every house,
        # its "share" of the blame is simply the average of all the mistakes combined!
        # We take the raw (N BY 1) Error array, sum up the N values, and scale it by 2/N.
        db = sum(error.data[i][0] for i in range(error.rows)) * (2.0 / Y.rows)
        
        # --- OPTIMIZATION STEP --- #
        # Finally, we take our original Weights and Bias and nudge them against the slope.
        # We multiply by our Learning Rate (0.01) which acts as a safety brake so we don't 
        # overshoot the target and cause the math to explode into infinity!
        W = W - (dW * lr)
        b = b - (db * lr)
        
        if epoch % 150 == 0:
            print(f"Epoch {epoch:4d} | Global Loss: {loss:10.4f} | Weights [SqFt W: {W.data[0][0]:6.2f}, Bedrm W: {W.data[1][0]:6.2f}] | Bias: {b:6.2f}")
            
    print("\nFinal Optimal Mapping Achieved!")
    prediction = (X.dot(W) + b).data[0][0]
    print(f"Prediction for House 1 (Target $400k): ${prediction:.2f}k")

if __name__ == "__main__":
    train()
