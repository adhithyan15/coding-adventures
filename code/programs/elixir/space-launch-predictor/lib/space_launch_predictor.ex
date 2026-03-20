defmodule SpaceLaunchPredictor do
  alias CodingAdventures.LossFunctions
  alias CodingAdventures.ActivationFunctions

  def run do
    IO.puts("\n--- Booting Elixir Space Launch Predictor ---")

    shuttle_data = [
      [12.0, 15.0], [35.0, 85.0], [5.0, 5.0],
      [40.0, 95.0], [15.0, 30.0], [28.0, 60.0]
    ]
    target_data = [
      [1.0], [0.0], [1.0], [0.0], [1.0], [0.0]
    ]

    features = Matrix.new(shuttle_data)
    true_labels = Matrix.new(target_data)

    weights = Matrix.new([[0.0], [0.0]])
    bias = 0.0
    lr = 0.01
    epochs = 3000

    {final_weights, final_bias} =
      Enum.reduce(0..epochs, {weights, bias}, fn epoch, {w, b} ->
        raw_preds = Matrix.dot(features, w)
        raw = Matrix.add_scalar(raw_preds, b)

        linear_probs = Enum.map(raw.data, fn [v] -> ActivationFunctions.sigmoid(v) end)
        linear_truth = Enum.map(true_labels.data, fn [v] -> v end)

        log_loss = LossFunctions.bce(linear_truth, linear_probs)
        loss_grad = LossFunctions.bce_derivative(linear_truth, linear_probs)

        # Zip to calculate combined gradients
        grad_data =
          Enum.zip(raw.data, loss_grad)
          |> Enum.map(fn {[val], lg} ->
            act_grad = ActivationFunctions.sigmoid_derivative(val)
            [lg * act_grad]
          end)

        bias_grad = Enum.map(grad_data, fn [v] -> v end) |> Enum.sum()

        grad_matrix = Matrix.new(grad_data)
        transposed = Matrix.transpose(features)
        weight_grads = Matrix.dot(transposed, grad_matrix)

        scaled_weights = Matrix.scale(weight_grads, lr)
        new_weights = Matrix.subtract(w, scaled_weights)
        new_bias = b - (bias_grad * lr)

        if rem(epoch, 500) == 0 do
          :io.format("Epoch ~4w | BCE Loss: ~.4f | Bias: ~.2f~n", [epoch, log_loss, new_bias])
        end

        {new_weights, new_bias}
      end)

    IO.puts("\n--- Final Inference ---")
    final_raw_dot = Matrix.dot(features, final_weights)
    final_raw = Matrix.add_scalar(final_raw_dot, final_bias)

    final_raw.data
    |> Enum.with_index()
    |> Enum.each(fn {[val], idx} ->
      prob = ActivationFunctions.sigmoid(val)
      truth_val = Enum.at(target_data, idx) |> hd()
      truth = if truth_val == 1.0, do: "Safe", else: "Abort"
      guess = if prob > 0.5, do: "Safe", else: "Abort"
      :io.format("Scenario ~w (Truth: ~s) -> System: ~s (~.2f%)~n", [idx + 1, truth, guess, prob * 100.0])
    end)
  end
end
