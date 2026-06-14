module TwoLayerNetwork
  VERSION = "0.1.0"

  Parameters = Struct.new(
    :input_to_hidden_weights,
    :hidden_biases,
    :hidden_to_output_weights,
    :output_biases,
    keyword_init: true
  )

  ForwardPass = Struct.new(
    :hidden_raw,
    :hidden_activations,
    :output_raw,
    :predictions,
    keyword_init: true
  )

  TrainingStep = Struct.new(
    :predictions,
    :errors,
    :output_deltas,
    :hidden_deltas,
    :hidden_to_output_weight_gradients,
    :output_bias_gradients,
    :input_to_hidden_weight_gradients,
    :hidden_bias_gradients,
    :next_parameters,
    :loss,
    keyword_init: true
  )

  class Model
    attr_reader :parameters

    def initialize(parameters:, learning_rate: 0.5, hidden_activation: :sigmoid, output_activation: :sigmoid)
      @parameters = parameters
      @learning_rate = learning_rate
      @hidden_activation = hidden_activation
      @output_activation = output_activation
    end

    def predict(inputs)
      TwoLayerNetwork.forward(inputs, @parameters, @hidden_activation, @output_activation).predictions
    end

    def inspect(inputs)
      TwoLayerNetwork.forward(inputs, @parameters, @hidden_activation, @output_activation)
    end

    def fit(inputs, targets, epochs: 100)
      history = []
      epochs.times do
        step = TwoLayerNetwork.train_one_epoch(inputs, targets, @parameters, @learning_rate, @hidden_activation, @output_activation)
        @parameters = step.next_parameters
        history << step
      end
      history
    end
  end

  module_function

  def xor_warm_start_parameters
    Parameters.new(
      input_to_hidden_weights: [[4.0, -4.0], [4.0, -4.0]],
      hidden_biases: [-2.0, 6.0],
      hidden_to_output_weights: [[4.0], [4.0]],
      output_biases: [-6.0]
    )
  end

  def validate_matrix(name, matrix)
    raise ArgumentError, "#{name} must contain at least one row" if matrix.empty?

    width = matrix.first.length
    raise ArgumentError, "#{name} must contain at least one column" if width.zero?
    raise ArgumentError, "#{name} must be rectangular" unless matrix.all? { |row| row.length == width }

    [matrix.length, width]
  end

  def activate(value, activation)
    case activation.to_sym
    when :linear then value
    when :sigmoid
      if value >= 0.0
        z = Math.exp(-value)
        1.0 / (1.0 + z)
      else
        z = Math.exp(value)
        z / (1.0 + z)
      end
    else raise ArgumentError, "unsupported activation: #{activation}"
    end
  end

  def derivative(raw, activated, activation)
    case activation.to_sym
    when :linear then 1.0
    when :sigmoid then activated * (1.0 - activated)
    else raise ArgumentError, "unsupported activation: #{activation}"
    end
  end

  def dot(left, right)
    rows, width = validate_matrix("left", left)
    right_rows, cols = validate_matrix("right", right)
    raise ArgumentError, "matrix shapes do not align" unless width == right_rows

    Array.new(rows) do |row|
      Array.new(cols) do |col|
        (0...width).sum { |k| left[row][k] * right[k][col] }
      end
    end
  end

  def transpose(matrix)
    rows, cols = validate_matrix("matrix", matrix)
    Array.new(cols) { |col| Array.new(rows) { |row| matrix[row][col] } }
  end

  def add_biases(matrix, biases)
    matrix.map { |row| row.each_with_index.map { |value, col| value + biases[col] } }
  end

  def apply_activation(matrix, activation)
    matrix.map { |row| row.map { |value| activate(value, activation) } }
  end

  def column_sums(matrix)
    _rows, cols = validate_matrix("matrix", matrix)
    Array.new(cols) { |col| matrix.sum { |row| row[col] } }
  end

  def mean_squared_error(errors)
    values = errors.flatten
    values.sum { |value| value * value } / values.length
  end

  def subtract_scaled(matrix, gradients, learning_rate)
    matrix.each_with_index.map do |row, row_index|
      row.each_with_index.map { |value, col| value - learning_rate * gradients[row_index][col] }
    end
  end

  def forward(inputs, parameters, hidden_activation = :sigmoid, output_activation = :sigmoid)
    hidden_raw = add_biases(dot(inputs, parameters.input_to_hidden_weights), parameters.hidden_biases)
    hidden_activations = apply_activation(hidden_raw, hidden_activation)
    output_raw = add_biases(dot(hidden_activations, parameters.hidden_to_output_weights), parameters.output_biases)
    predictions = apply_activation(output_raw, output_activation)

    ForwardPass.new(
      hidden_raw: hidden_raw,
      hidden_activations: hidden_activations,
      output_raw: output_raw,
      predictions: predictions
    )
  end

  def train_one_epoch(inputs, targets, parameters, learning_rate, hidden_activation = :sigmoid, output_activation = :sigmoid)
    sample_count, = validate_matrix("inputs", inputs)
    _target_rows, output_count = validate_matrix("targets", targets)
    passed = forward(inputs, parameters, hidden_activation, output_activation)
    scale = 2.0 / (sample_count * output_count)
    errors = Array.new(sample_count) { Array.new(output_count, 0.0) }
    output_deltas = Array.new(sample_count) { Array.new(output_count, 0.0) }

    sample_count.times do |row|
      output_count.times do |output|
        error = passed.predictions[row][output] - targets[row][output]
        errors[row][output] = error
        output_deltas[row][output] = scale * error * derivative(passed.output_raw[row][output], passed.predictions[row][output], output_activation)
      end
    end

    h2o_gradients = dot(transpose(passed.hidden_activations), output_deltas)
    output_bias_gradients = column_sums(output_deltas)
    hidden_errors = dot(output_deltas, transpose(parameters.hidden_to_output_weights))
    hidden_width = parameters.hidden_biases.length
    hidden_deltas = Array.new(sample_count) { Array.new(hidden_width, 0.0) }
    sample_count.times do |row|
      hidden_width.times do |hidden|
        hidden_deltas[row][hidden] = hidden_errors[row][hidden] * derivative(passed.hidden_raw[row][hidden], passed.hidden_activations[row][hidden], hidden_activation)
      end
    end
    i2h_gradients = dot(transpose(inputs), hidden_deltas)
    hidden_bias_gradients = column_sums(hidden_deltas)

    TrainingStep.new(
      predictions: passed.predictions,
      errors: errors,
      output_deltas: output_deltas,
      hidden_deltas: hidden_deltas,
      hidden_to_output_weight_gradients: h2o_gradients,
      output_bias_gradients: output_bias_gradients,
      input_to_hidden_weight_gradients: i2h_gradients,
      hidden_bias_gradients: hidden_bias_gradients,
      next_parameters: Parameters.new(
        input_to_hidden_weights: subtract_scaled(parameters.input_to_hidden_weights, i2h_gradients, learning_rate),
        hidden_biases: parameters.hidden_biases.each_with_index.map { |bias, index| bias - learning_rate * hidden_bias_gradients[index] },
        hidden_to_output_weights: subtract_scaled(parameters.hidden_to_output_weights, h2o_gradients, learning_rate),
        output_biases: parameters.output_biases.each_with_index.map { |bias, index| bias - learning_rate * output_bias_gradients[index] }
      ),
      loss: mean_squared_error(errors)
    )
  end
end
