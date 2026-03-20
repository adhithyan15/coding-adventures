defmodule MansionClassifier do
  alias CodingAdventures.LossFunctions
  alias CodingAdventures.ActivationFunctions

  def run do
    IO.puts("\n--- Booting Elixir Mansion Classifier ---")

    house_data = [
      [4.5, 6.0], [3.8, 5.0], [1.5, 2.0],
      [0.9, 1.0], [5.5, 7.0], [2.0, 3.0]
    ]
    target_data = [
      [1.0], [1.0], [0.0], [0.0], [1.0], [0.0]
    ]

    features = Matrix.new(house_data)
    true_labels = Matrix.new(target_data)

    weights = Matrix.new([[0.0], [0.0]])
    bias = 0.0
    lr = 0.1
    epochs = 2000

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

        if rem(epoch, 400) == 0 do
          :io.format("Epoch ~4w | BCE Loss: ~.4f | Bias: ~.2f~n", [epoch, log_loss, new_bias])
        end

        {new_weights, new_bias}
      end)

    IO.puts("\n--- Final Matrix Probability Inferences ---")
    final_raw_dot = Matrix.dot(features, final_weights)
    final_raw = Matrix.add_scalar(final_raw_dot, final_bias)

    final_raw.data
    |> Enum.with_index()
    |> Enum.each(fn {[val], idx} ->
      prob = ActivationFunctions.sigmoid(val)
      truth_val = Enum.at(target_data, idx) |> hd()
      truth = if truth_val == 1.0, do: "Mansion", else: "Normal"
      guess = if prob > 0.5, do: "Mansion", else: "Normal"
      :io.format("House ~w (Truth: ~s) -> System: ~s (~.2f%)~n", [idx + 1, truth, guess, prob * 100.0])
    end)
  end
end
