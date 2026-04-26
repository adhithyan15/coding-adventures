"""
Multi-Variable Linear Regression: House Price Predictor
-------------------------------------------------------
This program demonstrates the practical training loop for n input features and
one output: normalize the feature columns, sweep a few learning rates on a short
run, then train with the best stable candidate.
"""

import math

from feature_normalization import fit_standard_scaler, transform_standard
from loss_functions import mse as mse_loss
from matrix import Matrix


HOUSE_FEATURES_DATA = [
    [2000.0, 3.0],  # House 1: 2000 SqFt, 3 Beds
    [1500.0, 2.0],  # House 2: 1500 SqFt, 2 Beds
    [2500.0, 4.0],  # House 3: 2500 SqFt, 4 Beds
    [1000.0, 1.0],  # House 4: 1000 SqFt, 1 Bed
]

TRUE_PRICES_DATA = [
    [400.0],
    [300.0],
    [500.0],
    [200.0],
]


def run_training(features_data, true_prices_data, learning_rate, epochs, log_every=None):
    house_features = Matrix(features_data)
    true_prices = Matrix(true_prices_data)
    feature_weights = Matrix([[0.5], [0.5]])
    base_price_bias = 0.0
    last_loss = math.inf

    for epoch in range(epochs + 1):
        final_predictions = house_features.dot(feature_weights) + base_price_bias
        linear_true_prices = [row[0] for row in true_prices.data]
        linear_predictions = [row[0] for row in final_predictions.data]
        last_loss = mse_loss(linear_true_prices, linear_predictions)

        if not math.isfinite(last_loss) or last_loss > 1.0e12:
            return {
                "learning_rate": learning_rate,
                "loss": math.inf,
                "diverged": True,
                "weights": feature_weights,
                "bias": base_price_bias,
            }

        if log_every is not None and epoch % log_every == 0:
            print(
                f"Epoch {epoch:4d} | Loss: {last_loss:10.4f} | "
                f"Weights [SqFt: {feature_weights.data[0][0]:7.3f}, "
                f"Beds: {feature_weights.data[1][0]:7.3f}] | Bias: {base_price_bias:7.3f}"
            )

        prediction_errors = final_predictions - true_prices
        transposed_features = house_features.transpose()
        weight_gradients = transposed_features.dot(prediction_errors) * (2.0 / true_prices.rows)
        bias_gradient = sum(
            prediction_errors.data[i][0] for i in range(prediction_errors.rows)
        ) * (2.0 / true_prices.rows)

        feature_weights = feature_weights - (weight_gradients * learning_rate)
        base_price_bias = base_price_bias - (bias_gradient * learning_rate)

    return {
        "learning_rate": learning_rate,
        "loss": last_loss,
        "diverged": False,
        "weights": feature_weights,
        "bias": base_price_bias,
    }


def find_learning_rate(features_data, true_prices_data):
    candidates = [0.001, 0.003, 0.01, 0.03, 0.1, 0.3, 0.6]
    results = [
        run_training(features_data, true_prices_data, learning_rate, epochs=120)
        for learning_rate in candidates
    ]

    print("\nShort learning-rate sweep over normalized features:")
    for result in results:
        loss_text = "diverged" if result["diverged"] else f"{result['loss']:.4f}"
        print(f"  lr={result['learning_rate']:<6} -> loss={loss_text}")

    stable_results = [result for result in results if not result["diverged"]]
    return min(stable_results, key=lambda result: result["loss"])


def train():
    print("\n--- Booting Multi-Variable Predictor: House Prices ---")
    print("Features: square footage and bedroom count. Target: price in $1000s.")

    scaler = fit_standard_scaler(HOUSE_FEATURES_DATA)
    normalized_features = transform_standard(HOUSE_FEATURES_DATA, scaler)
    best_trial = find_learning_rate(normalized_features, TRUE_PRICES_DATA)

    print(f"\nSelected learning rate: {best_trial['learning_rate']}")
    print("Beginning full training run...")
    final_result = run_training(
        normalized_features,
        TRUE_PRICES_DATA,
        best_trial["learning_rate"],
        epochs=1500,
        log_every=150,
    )

    print("\nFinal Optimal Mapping Achieved!")
    test_house = [[2000.0, 3.0]]
    normalized_test_house = transform_standard(test_house, scaler)
    final_prediction = (
        Matrix(normalized_test_house).dot(final_result["weights"]) + final_result["bias"]
    ).data[0][0]
    print(f"Prediction for House 1 (Target $400k): ${final_prediction:.2f}k")


if __name__ == "__main__":
    train()
