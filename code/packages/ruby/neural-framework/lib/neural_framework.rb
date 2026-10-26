# frozen_string_literal: true

require "json"
require "matrix_ml"

module CodingAdventures
  # Independent Ruby reverse-mode differentiation above the pure-Ruby matrix
  # foundation. Nothing here delegates training to a binding or another runtime.
  module NeuralFramework
    VERSION = "0.1.0"
    MAX_STEP = (1 << 53) - 1
    MAX_JSON_BYTES = 4 * 1024 * 1024
    ACTIVATIONS = %w[linear sigmoid tanh relu].freeze
    DenseLayer = Struct.new(:weights, :biases, :activation)
    GradientTrace = Struct.new(:loss, :predictions, :weight_gradients, :bias_gradients)

    # The matrix foundation deliberately exposes mutable arrays. Admission is
    # therefore our responsibility, BEFORE invoking its constructor or dot().
    module Internal
      module_function

      def number(value)
        raise ArgumentError, "expected finite integer/float" unless value.instance_of?(Integer) || value.instance_of?(Float)
        converted = value.to_f
        raise ArgumentError, "expected finite number" unless converted.finite?
        converted
      end

      def count(value, minimum = 0, maximum = MAX_STEP)
        raise ArgumentError, "integer out of range" unless value.instance_of?(Integer) && value.between?(minimum, maximum)
        value
      end

      def capacity(rows, cols)
        # Every Array coordinate occupies at least one native pointer. Refuse
        # unrepresentable byte/element counts without attempting allocation.
        maximum = ((1 << (0.size * 8 - 1)) - 1) / 8
        raise ArgumentError, "unrepresentable capacity" unless rows > 0 && cols > 0 && rows <= maximum / cols
      end

      def vector(value)
        raise ArgumentError, "expected nonempty vector" unless value.instance_of?(Array) && !value.empty?
        value.map { |cell| number(cell) }
      end

      def grid(value)
        raise ArgumentError, "expected nonempty matrix" unless value.instance_of?(Array) && !value.empty?
        rows = value.map { |row| vector(row) }
        raise ArgumentError, "ragged matrix" unless rows.all? { |row| row.length == rows[0].length }
        capacity(rows.length, rows[0].length)
        rows
      end

      def closed(value, keys)
        raise ArgumentError, "unknown or missing fields" unless value.instance_of?(Hash) && value.keys.sort_by(&:to_s) == keys.sort
      end

      def activate(value, activation)
        case activation
        when "linear" then value
        when "relu" then [0.0, value].max
        when "tanh" then Math.tanh(value)
        else
          factor = Math.exp(-value.abs) # Never exponentiate a positive input.
          (value >= 0) ? 1.0 / (1 + factor) : factor / (1 + factor)
        end
      end

      def derivative(raw, value, activation)
        case activation
        when "linear" then 1.0
        when "relu" then (raw > 0) ? 1.0 : 0.0
        when "tanh" then 1 - value * value
        else value * (1 - value)
        end
      end
    end

    class DenseNetwork
      attr_reader :step

      def initialize(layers, step: 0)
        raise ArgumentError, "network requires layers" unless layers.instance_of?(Array) && !layers.empty?
        @step = Internal.count(step)
        @layers = []
        layers.each do |layer|
          raise ArgumentError, "unsupported layer" unless layer.instance_of?(DenseLayer) && ACTIVATIONS.include?(layer.activation)
          weights = Internal.grid(layer.weights)
          biases = Internal.vector(layer.biases)
          raise ArgumentError, "bias width mismatch" unless weights[0].length == biases.length
          raise ArgumentError, "disconnected layers" if !@layers.empty? && weights.length != @layers[-1].biases.length
          @layers << DenseLayer.new(weights: weights, biases: biases, activation: layer.activation.dup.freeze)
        end
      end

      def self.seeded(widths, activations, seed: 7)
        raise ArgumentError, "widths require input and output" unless widths.instance_of?(Array) && widths.length >= 2
        sizes = widths.map { |width| Internal.count(width, 1) }
        raise ArgumentError, "activation count mismatch" unless activations.instance_of?(Array) && activations.length == sizes.length - 1
        raise ArgumentError, "unsupported activation" unless activations.all? { |a| ACTIVATIONS.include?(a) }
        state = Internal.count(seed, 0, (1 << 32) - 1)
        layers = sizes.each_cons(2).with_index.map do |(fan_in, fan_out), index|
          Internal.capacity(fan_in, fan_out)
          weights = Array.new(fan_in) do
            Array.new(fan_out) do
              state = (1664525 * state + 1013904223) & 0xffffffff
              (2.0 * state / (1 << 32) - 1) / Math.sqrt(fan_in)
            end
          end
          DenseLayer.new(weights: weights, biases: Array.new(fan_out, 0.0), activation: activations[index])
        end
        new(layers)
      end

      def predict(inputs)
        forward(inputs)[0][-1]
      end

      # Reverse-mode MSE: average over samples * outputs, then apply each
      # activation's local derivative while walking the saved layers backward.
      def gradients(inputs, targets)
        saved, raws = forward(inputs)
        wanted = Internal.grid(targets)
        predictions = saved[-1]
        raise ArgumentError, "target shape mismatch" unless wanted.length == predictions.length && wanted[0].length == predictions[0].length
        errors = Internal.grid(predictions.each_with_index.map { |row, i| row.each_with_index.map { |v, j| v - wanted[i][j] } })
        count = errors.length * errors[0].length
        loss = Internal.number(errors.sum { |row| row.sum { |e| e * e } } / count)
        upstream = Internal.grid(errors.map { |row| row.map { |e| 2 * e / count } })
        dw = Array.new(@layers.length)
        db = Array.new(@layers.length)
        (@layers.length - 1).downto(0) do |index|
          layer = @layers[index]
          delta = Internal.grid(upstream.each_with_index.map do |row, i|
            row.each_with_index.map { |g, j| g * Internal.derivative(raws[index][i][j], saved[index + 1][i][j], layer.activation) }
          end)
          dw[index] = Internal.grid(Matrix.new(saved[index]).transpose.dot(Matrix.new(delta)).data)
          db[index] = Internal.vector(Array.new(delta[0].length) { |j| delta.sum { |row| row[j] } })
          upstream = Internal.grid(Matrix.new(delta).dot(Matrix.new(layer.weights).transpose).data) if index > 0
        end
        GradientTrace.new(loss: loss, predictions: predictions, weight_gradients: dw, bias_gradients: db)
      end

      def train_step(inputs, targets, learning_rate)
        rate = positive_rate(learning_rate)
        next_step = Internal.count(@step + 1)
        trace = gradients(inputs, targets)
        layers = @layers.each_with_index.map do |layer, i|
          weights = layer.weights.each_with_index.map do |row, r|
            row.each_with_index.map { |v, c| v - rate * trace.weight_gradients[i][r][c] }
          end
          biases = layer.biases.each_with_index.map { |v, j| v - rate * trace.bias_gradients[i][j] }
          DenseLayer.new(weights: weights, biases: biases, activation: layer.activation)
        end
        candidate = self.class.new(layers, step: next_step)
        candidate.gradients(inputs, targets) # Loss/gradient overflow must fail BEFORE adoption.
        @layers = candidate.instance_variable_get(:@layers)
        @step = next_step
        trace
      end

      def fit(inputs, targets, epochs, learning_rate)
        count = Internal.count(epochs, 0, MAX_STEP - @step)
        rate = positive_rate(learning_rate)
        Internal.capacity(count + 1, 1)
        losses = [gradients(inputs, targets).loss]
        count.times do
          train_step(inputs, targets, rate)
          losses << gradients(inputs, targets).loss
        end
        losses
      end

      def to_checkpoint
        {"format" => "coding-adventures-dense", "version" => 1, "step" => @step,
         "layers" => @layers.map do |layer|
           {"activation" => layer.activation.dup, "weights" => layer.weights.map(&:dup), "biases" => layer.biases.dup}
         end}
      end

      def self.from_checkpoint(document)
        Internal.closed(document, %w[format version step layers])
        raise ArgumentError, "unsupported format/version" unless document["format"] == "coding-adventures-dense" && document["version"].instance_of?(Integer) && document["version"] == 1
        raise ArgumentError, "invalid layers" unless document["layers"].instance_of?(Array)
        layers = document["layers"].map do |layer|
          Internal.closed(layer, %w[activation weights biases])
          DenseLayer.new(weights: layer["weights"], biases: layer["biases"], activation: layer["activation"])
        end
        new(layers, step: document["step"])
      end

      def self.from_json(text)
        raise ArgumentError, "JSON byte limit/type" unless text.instance_of?(String) && text.bytesize <= MAX_JSON_BYTES
        from_checkpoint(JSON.parse(text, max_nesting: 128, allow_nan: false))
      rescue JSON::ParserError => error
        raise ArgumentError, "invalid checkpoint JSON: #{error.class}"
      end

      def to_json
        JSON.generate(to_checkpoint, allow_nan: false, max_nesting: 128)
      end

      private

      def positive_rate(value)
        rate = Internal.number(value)
        raise ArgumentError, "rate must be positive" unless rate > 0
        rate
      end

      def forward(inputs)
        current = Internal.grid(inputs)
        raise ArgumentError, "input width mismatch" unless current[0].length == @layers[0].weights.length
        saved = [current]
        raws = []
        @layers.each do |layer|
          products = Matrix.new(current).dot(Matrix.new(layer.weights)).data
          raw = Internal.grid(products.map { |row| row.each_with_index.map { |v, j| v + layer.biases[j] } })
          current = Internal.grid(raw.map { |row| row.map { |v| Internal.activate(v, layer.activation) } })
          raws << raw
          saved << current
        end
        [saved, raws]
      end
    end
  end
end
