defmodule Perceptron do
  alias CodingAdventures.ActivationFunctions
  alias CodingAdventures.LossFunctions

  defstruct learning_rate: 0.1, epochs: 2000, weights: nil, bias: 0.0

  def new(lr \\ 0.1, epochs \\ 2000) do
    %Perceptron{learning_rate: lr, epochs: epochs}
  end

  def fit(model, x_data, y_data, log_steps) do
    features = Matrix.new(x_data)
    true_labels = Matrix.new(y_data)

    w_data = List.duplicate([0.0], features.cols)
    weights = Matrix.new(w_data)
    bias = 0.0

    {final_weights, final_bias} =
      Enum.reduce(0..model.epochs, {weights, bias}, fn epoch, {curr_weights, curr_bias} ->
        raw = features |> Matrix.dot(curr_weights) |> Matrix.add_scalar(curr_bias)

        probs = Enum.map(raw.data, fn [val] -> [ActivationFunctions.sigmoid(val)] end)

        linear_truth = Enum.map(true_labels.data, fn [v] -> v end)
        linear_probs = Enum.map(probs, fn [v] -> v end)

        log_loss = LossFunctions.bce(linear_truth, linear_probs)
        loss_grad = LossFunctions.bce_derivative(linear_truth, linear_probs)

        {grad_data, bias_grad} =
          Enum.reduce(0..(features.rows - 1), {[], 0.0}, fn i, {acc_grad, acc_bias} ->
            act_grad = ActivationFunctions.sigmoid_derivative(Enum.at(Enum.at(raw.data, i), 0))
            combined = Enum.at(loss_grad, i) * act_grad
            {acc_grad ++ [[combined]], acc_bias + combined}
          end)

        grad_matrix = Matrix.new(grad_data)
        weight_grads = features |> Matrix.transpose() |> Matrix.dot(grad_matrix)

        scaled_weights = Matrix.scale(weight_grads, model.learning_rate)
        new_weights = Matrix.subtract(curr_weights, scaled_weights)
        new_bias = curr_bias - bias_grad * model.learning_rate

        if rem(epoch, log_steps) == 0 do
          :io.format("Epoch ~4w | BCE Loss: ~.4f | Bias: ~.2f~n", [epoch, log_loss, new_bias])
        end

        {new_weights, new_bias}
      end)

    %{model | weights: final_weights, bias: final_bias}
  end

  def predict(model, x_data) do
    if is_nil(model.weights) do
      raise "Error: Predict called before fit/4"
    end

    features = Matrix.new(x_data)
    raw = features |> Matrix.dot(model.weights) |> Matrix.add_scalar(model.bias)

    Enum.map(raw.data, fn [val] -> ActivationFunctions.sigmoid(val) end)
  end
end
