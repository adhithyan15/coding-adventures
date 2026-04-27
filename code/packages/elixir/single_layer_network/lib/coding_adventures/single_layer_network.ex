defmodule CodingAdventures.SingleLayerNetwork do
  @moduledoc """
  Single-layer multi-input multi-output neural network primitives.
  """

  @version "0.1.0"
  def version, do: @version

  defstruct weights: [], biases: [], activation: :linear

  def new(input_count, output_count, activation \\ :linear) do
    %__MODULE__{
      weights: List.duplicate(List.duplicate(0.0, output_count), input_count),
      biases: List.duplicate(0.0, output_count),
      activation: activation
    }
  end

  def predict(%__MODULE__{} = model, inputs) do
    predict_with_parameters(inputs, model.weights, model.biases, model.activation)
  end

  def fit(%__MODULE__{} = model, inputs, targets, learning_rate \\ 0.05, epochs \\ 100) do
    Enum.reduce(1..epochs, {model, []}, fn _, {current, history} ->
      step =
        train_one_epoch_with_matrices(
          inputs,
          targets,
          current.weights,
          current.biases,
          learning_rate,
          current.activation
        )

      next = %{current | weights: step.next_weights, biases: step.next_biases}
      {next, [step | history]}
    end)
    |> then(fn {model, history} -> {model, Enum.reverse(history)} end)
  end

  def fit_single_layer_network(inputs, targets, learning_rate \\ 0.05, epochs \\ 100, activation \\ :linear) do
    {_sample_count, input_count} = validate_matrix!("inputs", inputs)
    {_target_count, output_count} = validate_matrix!("targets", targets)
    new(input_count, output_count, activation)
    |> fit(inputs, targets, learning_rate, epochs)
  end

  def predict_with_parameters(inputs, weights, biases, activation \\ :linear) do
    {sample_count, input_count} = validate_matrix!("inputs", inputs)
    {weight_rows, output_count} = validate_matrix!("weights", weights)
    if input_count != weight_rows, do: raise(ArgumentError, "input column count must match weight row count")
    if length(biases) != output_count, do: raise(ArgumentError, "bias count must match output count")

    for row <- 0..(sample_count - 1) do
      for output <- 0..(output_count - 1) do
        total =
          Enum.reduce(0..(input_count - 1), Enum.at(biases, output), fn input, sum ->
            sum + Enum.at(Enum.at(inputs, row), input) * Enum.at(Enum.at(weights, input), output)
          end)

        activate(total, activation)
      end
    end
  end

  def train_one_epoch_with_matrices(inputs, targets, weights, biases, learning_rate, activation \\ :linear) do
    {sample_count, input_count} = validate_matrix!("inputs", inputs)
    {target_rows, output_count} = validate_matrix!("targets", targets)
    {weight_rows, weight_cols} = validate_matrix!("weights", weights)
    if target_rows != sample_count, do: raise(ArgumentError, "inputs and targets must have the same row count")
    if weight_rows != input_count or weight_cols != output_count, do: raise(ArgumentError, "weights must be shaped input_count x output_count")
    if length(biases) != output_count, do: raise(ArgumentError, "bias count must match output count")

    predictions = predict_with_parameters(inputs, weights, biases, activation)
    scale = 2.0 / (sample_count * output_count)

    {errors, deltas, loss_total} =
      Enum.reduce(0..(sample_count - 1), {[], [], 0.0}, fn row, {error_rows, delta_rows, loss} ->
        {error_row, delta_row, row_loss} =
          Enum.reduce(0..(output_count - 1), {[], [], 0.0}, fn output, {er, dr, rl} ->
            error = Enum.at(Enum.at(predictions, row), output) - Enum.at(Enum.at(targets, row), output)
            delta = scale * error * derivative_from_output(Enum.at(Enum.at(predictions, row), output), activation)
            {[error | er], [delta | dr], rl + error * error}
          end)

        {[Enum.reverse(error_row) | error_rows], [Enum.reverse(delta_row) | delta_rows], loss + row_loss}
      end)

    errors = Enum.reverse(errors)
    deltas = Enum.reverse(deltas)

    weight_gradients =
      for input <- 0..(input_count - 1) do
        for output <- 0..(output_count - 1) do
          Enum.reduce(0..(sample_count - 1), 0.0, fn row, sum ->
            sum + Enum.at(Enum.at(inputs, row), input) * Enum.at(Enum.at(deltas, row), output)
          end)
        end
      end

    bias_gradients =
      for output <- 0..(output_count - 1) do
        Enum.reduce(0..(sample_count - 1), 0.0, fn row, sum ->
          sum + Enum.at(Enum.at(deltas, row), output)
        end)
      end

    next_weights =
      for input <- 0..(input_count - 1) do
        for output <- 0..(output_count - 1) do
          Enum.at(Enum.at(weights, input), output) - learning_rate * Enum.at(Enum.at(weight_gradients, input), output)
        end
      end

    next_biases =
      for output <- 0..(output_count - 1) do
        Enum.at(biases, output) - learning_rate * Enum.at(bias_gradients, output)
      end

    %{
      predictions: predictions,
      errors: errors,
      weight_gradients: weight_gradients,
      bias_gradients: bias_gradients,
      next_weights: next_weights,
      next_biases: next_biases,
      loss: loss_total / (sample_count * output_count)
    }
  end

  defp validate_matrix!(name, matrix) do
    if matrix == [], do: raise(ArgumentError, "#{name} must contain at least one row")
    width = matrix |> hd() |> length()
    if width == 0, do: raise(ArgumentError, "#{name} must contain at least one column")
    if Enum.any?(matrix, &(length(&1) != width)), do: raise(ArgumentError, "#{name} must be rectangular")
    {length(matrix), width}
  end

  defp activate(value, :linear), do: value
  defp activate(value, :sigmoid) when value >= 0.0 do
    z = :math.exp(-value)
    1.0 / (1.0 + z)
  end
  defp activate(value, :sigmoid) do
    z = :math.exp(value)
    z / (1.0 + z)
  end
  defp activate(_value, activation), do: raise(ArgumentError, "unsupported activation: #{activation}")

  defp derivative_from_output(_output, :linear), do: 1.0
  defp derivative_from_output(output, :sigmoid), do: output * (1.0 - output)
  defp derivative_from_output(_output, activation), do: raise(ArgumentError, "unsupported activation: #{activation}")
end
