module SingleLayerNetwork
  VERSION = "0.1.0"

  TrainingStep = Struct.new(
    :predictions,
    :errors,
    :weight_gradients,
    :bias_gradients,
    :next_weights,
    :next_biases,
    :loss,
    keyword_init: true
  )

  class Model
    attr_reader :weights, :biases, :activation

    def initialize(input_count:, output_count:, activation: :linear)
      @weights = Array.new(input_count) { Array.new(output_count, 0.0) }
      @biases = Array.new(output_count, 0.0)
      @activation = activation
    end

    def predict(inputs)
      SingleLayerNetwork.predict_with_parameters(inputs, @weights, @biases, @activation)
    end

    def fit(inputs, targets, learning_rate: 0.05, epochs: 100)
      history = []
      epochs.times do
        step = SingleLayerNetwork.train_one_epoch_with_matrices(
          inputs,
          targets,
          @weights,
          @biases,
          learning_rate,
          @activation
        )
        @weights = step.next_weights
        @biases = step.next_biases
        history << step
      end
      history
    end
  end

  module_function

  def validate_matrix(name, matrix)
    raise ArgumentError, "#{name} must contain at least one row" if matrix.empty?

    width = matrix.first.length
    raise ArgumentError, "#{name} must contain at least one column" if width.zero?
    raise ArgumentError, "#{name} must be rectangular" unless matrix.all? { |row| row.length == width }

    [matrix.length, width]
  end

  def activate(value, activation)
    case activation.to_sym
    when :linear
      value
    when :sigmoid
      if value >= 0.0
        z = Math.exp(-value)
        1.0 / (1.0 + z)
      else
        z = Math.exp(value)
        z / (1.0 + z)
      end
    else
      raise ArgumentError, "unsupported activation: #{activation}"
    end
  end

  def derivative_from_output(output, activation)
    case activation.to_sym
    when :linear then 1.0
    when :sigmoid then output * (1.0 - output)
    else raise ArgumentError, "unsupported activation: #{activation}"
    end
  end

  def predict_with_parameters(inputs, weights, biases, activation = :linear)
    sample_count, input_count = validate_matrix("inputs", inputs)
    weight_rows, output_count = validate_matrix("weights", weights)
    raise ArgumentError, "input column count must match weight row count" unless input_count == weight_rows
    raise ArgumentError, "bias count must match output count" unless biases.length == output_count

    Array.new(sample_count) do |row|
      Array.new(output_count) do |output|
        total = biases[output]
        input_count.times do |input|
          total += inputs[row][input] * weights[input][output]
        end
        activate(total, activation)
      end
    end
  end

  def train_one_epoch_with_matrices(inputs, targets, weights, biases, learning_rate, activation = :linear)
    sample_count, input_count = validate_matrix("inputs", inputs)
    target_rows, output_count = validate_matrix("targets", targets)
    weight_rows, weight_cols = validate_matrix("weights", weights)
    raise ArgumentError, "inputs and targets must have the same row count" unless target_rows == sample_count
    raise ArgumentError, "weights must be shaped input_count x output_count" unless weight_rows == input_count && weight_cols == output_count
    raise ArgumentError, "bias count must match output count" unless biases.length == output_count

    predictions = predict_with_parameters(inputs, weights, biases, activation)
    scale = 2.0 / (sample_count * output_count)
    errors = Array.new(sample_count) { Array.new(output_count, 0.0) }
    deltas = Array.new(sample_count) { Array.new(output_count, 0.0) }
    loss_total = 0.0

    sample_count.times do |row|
      output_count.times do |output|
        error = predictions[row][output] - targets[row][output]
        errors[row][output] = error
        deltas[row][output] = scale * error * derivative_from_output(predictions[row][output], activation)
        loss_total += error * error
      end
    end

    weight_gradients = Array.new(input_count) { Array.new(output_count, 0.0) }
    next_weights = Array.new(input_count) { Array.new(output_count, 0.0) }
    input_count.times do |input|
      output_count.times do |output|
        sample_count.times do |row|
          weight_gradients[input][output] += inputs[row][input] * deltas[row][output]
        end
        next_weights[input][output] = weights[input][output] - learning_rate * weight_gradients[input][output]
      end
    end

    bias_gradients = Array.new(output_count, 0.0)
    next_biases = Array.new(output_count, 0.0)
    output_count.times do |output|
      sample_count.times { |row| bias_gradients[output] += deltas[row][output] }
      next_biases[output] = biases[output] - learning_rate * bias_gradients[output]
    end

    TrainingStep.new(
      predictions: predictions,
      errors: errors,
      weight_gradients: weight_gradients,
      bias_gradients: bias_gradients,
      next_weights: next_weights,
      next_biases: next_biases,
      loss: loss_total / (sample_count * output_count)
    )
  end

  def fit_single_layer_network(inputs, targets, learning_rate: 0.05, epochs: 100, activation: :linear)
    _, input_count = validate_matrix("inputs", inputs)
    _, output_count = validate_matrix("targets", targets)
    model = Model.new(input_count: input_count, output_count: output_count, activation: activation)
    model.fit(inputs, targets, learning_rate: learning_rate, epochs: epochs)
    model
  end
end
