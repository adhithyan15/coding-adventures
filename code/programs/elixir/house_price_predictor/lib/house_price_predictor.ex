# Multi-Variable Linear Regression: House Price Predictor
# -------------------------------------------------------
# Harnesses functional pipeline states sequentially traversing native module mappings
# mapped cleanly purely efficiently accurately via detailed explicit literal variables!

defmodule HousePricePredictor do
  def run do
    IO.puts("\n--- Booting Multi-Variable Predictor: House Prices ---\n")

    house_features = Matrix.new([
      [2.0, 3.0],
      [1.5, 2.0],
      [2.5, 4.0],
      [1.0, 1.0]
    ])

    true_prices = Matrix.new([
      [400.0],
      [300.0],
      [500.0],
      [200.0]
    ])

    feature_weights = Matrix.new([[0.5], [0.5]])
    base_price_bias = 0.5
    learning_rate = 0.01

    IO.puts("Beginning Training Epochs...")
    train(house_features, true_prices, feature_weights, base_price_bias, learning_rate, 0)
  end

  defp train(_hf, _tp, _fw, _bpb, _lr, epoch) when epoch > 1500, do: :ok
  defp train(house_features, true_prices, feature_weights, base_price_bias, learning_rate, epoch) do
    
    # --- PURE FUNCTIONAL FORWARD MATRIX EVALUATION ---
    raw_predictions = Matrix.dot(house_features, feature_weights)
    final_predictions = Matrix.add_scalar(raw_predictions, base_price_bias)

    linear_true_prices = Enum.map(true_prices.data, fn [v] -> v end)
    linear_predictions = Enum.map(final_predictions.data, fn [v] -> v end)
    mse_loss = CodingAdventures.LossFunctions.mse(linear_true_prices, linear_predictions)

    # --- BACKPROPAGATION (CALCULATING GRADIENTS) ---
    # How do we figure out exactly how much the SqFt Weight vs Bedroom Weight was responsible for the error?
    # 1. We take our original (N BY 2) Data Grid and physically flip it on its side to become (2 BY N). 
    # 2. We Dot Product this (2 BY N) grid against our (N BY 1) Error Vector!
    prediction_errors = Matrix.subtract(final_predictions, true_prices)
    transposed_features = Matrix.transpose(house_features)
    features_dot_errors = Matrix.dot(transposed_features, prediction_errors)
    
    # We multiply by (2 / N) because of the Mean Squared Error derivative scaling.
    weight_gradients = Matrix.scale(features_dot_errors, 2.0 / true_prices.rows)

    # For the Bias, because it shifts the prediction unconditionally for every house,
    # its "share" of the blame is simply the average of all the mistakes combined!
    bias_gradient_total = Enum.map(prediction_errors.data, fn [v] -> v end) |> Enum.sum()
    bias_gradient = bias_gradient_total * (2.0 / true_prices.rows)

    # --- TAIL RECURSION ---
    # Finally, we take our original Weights and Bias and nudge them against the slope.
    # We multiply by our Learning Rate (0.01) which acts as a safety brake so we don't explode.
    new_feature_weights = Matrix.subtract(feature_weights, Matrix.scale(weight_gradients, learning_rate))
    new_base_price_bias = base_price_bias - (bias_gradient * learning_rate)

    if rem(epoch, 150) == 0 do
      [[w1], [w2]] = new_feature_weights.data
      IO.puts(:io_lib.format("Epoch ~4w | Global Loss: ~9.4f | Weights [SqFt: ~5.2f, Bed: ~5.2f] | Bias: ~5.2f", [epoch, mse_loss, w1, w2, new_base_price_bias]))
    end
    
    if epoch == 1500 do
        IO.puts("\nFinal Optimal Mapping Achieved!")
        prediction = Matrix.add_scalar(Matrix.dot(house_features, new_feature_weights), new_base_price_bias).data |> hd() |> hd()
        IO.puts(:io_lib.format("Prediction for House 1 (Target $400k): $~.2fk", [prediction]))
    end

    train(house_features, true_prices, new_feature_weights, new_base_price_bias, learning_rate, epoch + 1)
  end
end
