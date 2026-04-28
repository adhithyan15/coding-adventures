defmodule CodingAdventures.TwoLayerNetwork do
  @moduledoc """
  Two-layer neural network primitives for hidden-layer examples.
  """

  @version "0.1.0"
  def version, do: @version

  defstruct parameters: nil, learning_rate: 0.5, hidden_activation: :sigmoid, output_activation: :sigmoid

  def xor_warm_start_parameters do
    %{
      input_to_hidden_weights: [[4.0, -4.0], [4.0, -4.0]],
      hidden_biases: [-2.0, 6.0],
      hidden_to_output_weights: [[4.0], [4.0]],
      output_biases: [-6.0]
    }
  end

  def new(parameters, learning_rate \\ 0.5) do
    %__MODULE__{parameters: parameters, learning_rate: learning_rate}
  end

  def predict(%__MODULE__{} = model, inputs) do
    forward(inputs, model.parameters, model.hidden_activation, model.output_activation).predictions
  end

  def inspect(%__MODULE__{} = model, inputs) do
    forward(inputs, model.parameters, model.hidden_activation, model.output_activation)
  end

  def forward(inputs, parameters, hidden_activation \\ :sigmoid, output_activation \\ :sigmoid) do
    hidden_raw = dot(inputs, parameters.input_to_hidden_weights) |> add_biases(parameters.hidden_biases)
    hidden_activations = apply_activation(hidden_raw, hidden_activation)
    output_raw = dot(hidden_activations, parameters.hidden_to_output_weights) |> add_biases(parameters.output_biases)
    predictions = apply_activation(output_raw, output_activation)

    %{
      hidden_raw: hidden_raw,
      hidden_activations: hidden_activations,
      output_raw: output_raw,
      predictions: predictions
    }
  end

  def train_one_epoch(inputs, targets, parameters, learning_rate, hidden_activation \\ :sigmoid, output_activation \\ :sigmoid) do
    {sample_count, _input_count} = validate_matrix!("inputs", inputs)
    {_target_rows, output_count} = validate_matrix!("targets", targets)
    pass = forward(inputs, parameters, hidden_activation, output_activation)
    scale = 2.0 / (sample_count * output_count)

    errors =
      for row <- 0..(sample_count - 1) do
        for output <- 0..(output_count - 1) do
          at2(pass.predictions, row, output) - at2(targets, row, output)
        end
      end

    output_deltas =
      for row <- 0..(sample_count - 1) do
        for output <- 0..(output_count - 1) do
          scale * at2(errors, row, output) *
            derivative(at2(pass.output_raw, row, output), at2(pass.predictions, row, output), output_activation)
        end
      end

    h2o_gradients = dot(transpose(pass.hidden_activations), output_deltas)
    output_bias_gradients = column_sums(output_deltas)
    hidden_errors = dot(output_deltas, transpose(parameters.hidden_to_output_weights))
    hidden_width = length(parameters.hidden_biases)

    hidden_deltas =
      for row <- 0..(sample_count - 1) do
        for hidden <- 0..(hidden_width - 1) do
          at2(hidden_errors, row, hidden) *
            derivative(at2(pass.hidden_raw, row, hidden), at2(pass.hidden_activations, row, hidden), hidden_activation)
        end
      end

    i2h_gradients = dot(transpose(inputs), hidden_deltas)
    hidden_bias_gradients = column_sums(hidden_deltas)

    %{
      predictions: pass.predictions,
      errors: errors,
      output_deltas: output_deltas,
      hidden_deltas: hidden_deltas,
      hidden_to_output_weight_gradients: h2o_gradients,
      output_bias_gradients: output_bias_gradients,
      input_to_hidden_weight_gradients: i2h_gradients,
      hidden_bias_gradients: hidden_bias_gradients,
      next_parameters: %{
        input_to_hidden_weights: subtract_scaled(parameters.input_to_hidden_weights, i2h_gradients, learning_rate),
        hidden_biases: subtract_scaled_vector(parameters.hidden_biases, hidden_bias_gradients, learning_rate),
        hidden_to_output_weights: subtract_scaled(parameters.hidden_to_output_weights, h2o_gradients, learning_rate),
        output_biases: subtract_scaled_vector(parameters.output_biases, output_bias_gradients, learning_rate)
      },
      loss: mse(errors)
    }
  end

  def fit(%__MODULE__{} = model, inputs, targets, epochs \\ 100) do
    Enum.reduce(1..epochs, {model, []}, fn _, {current, history} ->
      step =
        train_one_epoch(
          inputs,
          targets,
          current.parameters,
          current.learning_rate,
          current.hidden_activation,
          current.output_activation
        )

      {%{current | parameters: step.next_parameters}, [step | history]}
    end)
    |> then(fn {model, history} -> {model, Enum.reverse(history)} end)
  end

  defp validate_matrix!(name, matrix) do
    if matrix == [], do: raise(ArgumentError, "#{name} must contain at least one row")
    width = matrix |> hd() |> length()
    if width == 0, do: raise(ArgumentError, "#{name} must contain at least one column")
    if Enum.any?(matrix, &(length(&1) != width)), do: raise(ArgumentError, "#{name} must be rectangular")
    {length(matrix), width}
  end

  defp at2(matrix, row, col), do: matrix |> Enum.at(row) |> Enum.at(col)

  defp dot(left, right) do
    {rows, width} = validate_matrix!("left", left)
    {right_rows, cols} = validate_matrix!("right", right)
    if width != right_rows, do: raise(ArgumentError, "matrix shapes do not align")

    for row <- 0..(rows - 1) do
      for col <- 0..(cols - 1) do
        Enum.reduce(0..(width - 1), 0.0, fn k, sum -> sum + at2(left, row, k) * at2(right, k, col) end)
      end
    end
  end

  defp transpose(matrix) do
    {rows, cols} = validate_matrix!("matrix", matrix)
    for col <- 0..(cols - 1), do: for(row <- 0..(rows - 1), do: at2(matrix, row, col))
  end

  defp add_biases(matrix, biases) do
    Enum.map(matrix, fn row ->
      Enum.with_index(row) |> Enum.map(fn {value, col} -> value + Enum.at(biases, col) end)
    end)
  end

  defp apply_activation(matrix, activation) do
    Enum.map(matrix, fn row -> Enum.map(row, &activate(&1, activation)) end)
  end

  defp column_sums(matrix) do
    {_rows, cols} = validate_matrix!("matrix", matrix)
    for col <- 0..(cols - 1), do: Enum.reduce(matrix, 0.0, fn row, sum -> sum + Enum.at(row, col) end)
  end

  defp subtract_scaled(matrix, gradients, learning_rate) do
    Enum.with_index(matrix)
    |> Enum.map(fn {row, row_index} ->
      Enum.with_index(row)
      |> Enum.map(fn {value, col} -> value - learning_rate * at2(gradients, row_index, col) end)
    end)
  end

  defp subtract_scaled_vector(values, gradients, learning_rate) do
    Enum.with_index(values) |> Enum.map(fn {value, index} -> value - learning_rate * Enum.at(gradients, index) end)
  end

  defp mse(errors) do
    values = List.flatten(errors)
    Enum.reduce(values, 0.0, fn value, sum -> sum + value * value end) / length(values)
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

  defp derivative(_raw, _activated, :linear), do: 1.0
  defp derivative(_raw, activated, :sigmoid), do: activated * (1.0 - activated)
  defp derivative(_raw, _activated, activation), do: raise(ArgumentError, "unsupported activation: #{activation}")
end
