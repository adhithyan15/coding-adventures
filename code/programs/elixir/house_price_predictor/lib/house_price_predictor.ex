# Multi-Variable Linear Regression: House Price Predictor
# -------------------------------------------------------
# Demonstrates n input features -> one output with feature normalization and a
# short learning-rate sweep before the full training run.

defmodule HousePricePredictor do
  alias CodingAdventures.FeatureNormalization

  @house_features_data [
    [2000.0, 3.0],
    [1500.0, 2.0],
    [2500.0, 4.0],
    [1000.0, 1.0]
  ]

  @true_prices_data [
    [400.0],
    [300.0],
    [500.0],
    [200.0]
  ]

  def run do
    IO.puts("\n--- Booting Multi-Variable Predictor: House Prices ---")
    IO.puts("Features: square footage and bedroom count. Target: price in $1000s.")

    {:ok, scaler} = FeatureNormalization.fit_standard_scaler(@house_features_data)

    {:ok, normalized_features} =
      FeatureNormalization.transform_standard(@house_features_data, scaler)

    best_trial = find_learning_rate(normalized_features, @true_prices_data)

    IO.puts("\nSelected learning rate: #{best_trial.learning_rate}")
    IO.puts("Beginning full training run...")

    final_result =
      run_training(normalized_features, @true_prices_data, best_trial.learning_rate, 1500, 150)

    IO.puts("\nFinal Optimal Mapping Achieved!")

    {:ok, normalized_test_house} =
      FeatureNormalization.transform_standard([[2000.0, 3.0]], scaler)

    prediction =
      Matrix.add_scalar(
        Matrix.dot(Matrix.new(normalized_test_house), final_result.weights),
        final_result.bias
      )
      |> then(& &1.data)
      |> hd()
      |> hd()

    IO.puts(:io_lib.format("Prediction for House 1 (Target $400k): $~.2fk", [prediction]))
  end

  defp find_learning_rate(features_data, prices_data) do
    candidates = [0.001, 0.003, 0.01, 0.03, 0.1, 0.3, 0.6]
    results = Enum.map(candidates, &run_training(features_data, prices_data, &1, 120, nil))

    IO.puts("\nShort learning-rate sweep over normalized features:")

    Enum.each(results, fn result ->
      loss_text = if result.diverged, do: "diverged", else: :io_lib.format("~.4f", [result.loss])
      IO.puts(["  lr=", to_string(result.learning_rate), " -> loss=", loss_text])
    end)

    results
    |> Enum.reject(& &1.diverged)
    |> Enum.min_by(& &1.loss)
  end

  defp run_training(features_data, prices_data, learning_rate, epochs, log_every) do
    house_features = Matrix.new(features_data)
    true_prices = Matrix.new(prices_data)
    feature_weights = Matrix.new([[0.5], [0.5]])

    train_loop(
      0,
      epochs,
      house_features,
      true_prices,
      feature_weights,
      0.0,
      learning_rate,
      log_every,
      :infinity
    )
  end

  defp train_loop(
         epoch,
         epochs,
         _features,
         _prices,
         weights,
         bias,
         learning_rate,
         _log_every,
         loss
       )
       when epoch > epochs do
    %{learning_rate: learning_rate, loss: loss, diverged: false, weights: weights, bias: bias}
  end

  defp train_loop(
         epoch,
         epochs,
         house_features,
         true_prices,
         feature_weights,
         bias,
         learning_rate,
         log_every,
         _loss
       ) do
    final_predictions =
      house_features
      |> Matrix.dot(feature_weights)
      |> Matrix.add_scalar(bias)

    linear_true_prices = Enum.map(true_prices.data, fn [value] -> value end)
    linear_predictions = Enum.map(final_predictions.data, fn [value] -> value end)
    mse_loss = CodingAdventures.LossFunctions.mse(linear_true_prices, linear_predictions)

    if mse_loss != mse_loss or mse_loss > 1.0e12 do
      %{
        learning_rate: learning_rate,
        loss: :infinity,
        diverged: true,
        weights: feature_weights,
        bias: bias
      }
    else
      if log_every && rem(epoch, log_every) == 0 do
        [[w1], [w2]] = feature_weights.data

        IO.puts(
          :io_lib.format(
            "Epoch ~4w | Loss: ~12.4f | Weights [SqFt: ~7.3f, Beds: ~7.3f] | Bias: ~7.3f",
            [epoch, mse_loss, w1, w2, bias]
          )
        )
      end

      prediction_errors = Matrix.subtract(final_predictions, true_prices)

      weight_gradients =
        house_features
        |> Matrix.transpose()
        |> Matrix.dot(prediction_errors)
        |> Matrix.scale(2.0 / true_prices.rows)

      bias_gradient_total =
        prediction_errors.data |> Enum.map(fn [value] -> value end) |> Enum.sum()

      bias_gradient = bias_gradient_total * (2.0 / true_prices.rows)

      new_feature_weights =
        Matrix.subtract(feature_weights, Matrix.scale(weight_gradients, learning_rate))

      new_bias = bias - bias_gradient * learning_rate

      train_loop(
        epoch + 1,
        epochs,
        house_features,
        true_prices,
        new_feature_weights,
        new_bias,
        learning_rate,
        log_every,
        mse_loss
      )
    end
  end
end
