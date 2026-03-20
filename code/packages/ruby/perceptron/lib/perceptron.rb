require_relative "../../matrix/lib/matrix_ml"
require_relative "../../loss-functions/lib/loss_functions"
require_relative "../../activation-functions/lib/activation_functions"

module Perceptron_ML
  class Perceptron
    attr_accessor :learning_rate, :epochs, :weights, :bias

    def initialize(learning_rate = 0.1, epochs = 2000)
      @learning_rate = learning_rate
      @epochs = epochs
      @weights = nil
      @bias = 0.0
    end

    def fit(x_data, y_data, log_steps)
      features = Matrix.new(x_data)
      true_labels = Matrix.new(y_data)

      w_data = []
      features.cols.times { w_data << [0.0] }
      @weights = Matrix.new(w_data)
      @bias = 0.0

      (0..@epochs).each do |epoch|
        raw = features.dot(@weights)
        raw = raw + @bias

        probs = raw.data.map { |r| [ActivationFunctions.sigmoid(r[0])] }
        linear_truth = true_labels.data.map { |r| r[0] }
        linear_probs = probs.map { |r| r[0] }

        log_loss = LossFunctions.bce(linear_truth, linear_probs)
        loss_grad = LossFunctions.bce_derivative(linear_truth, linear_probs)

        grad_data = []
        bias_grad = 0.0
        features.rows.times do |i|
          act_grad = ActivationFunctions.sigmoid_derivative(raw.data[i][0])
          combined = loss_grad[i] * act_grad
          grad_data << [combined]
          bias_grad += combined
        end

        grad_matrix = Matrix.new(grad_data)
        weight_grads = features.transpose.dot(grad_matrix)

        scaled_weights = weight_grads * @learning_rate
        @weights = @weights - scaled_weights
        @bias -= bias_grad * @learning_rate

        if epoch % log_steps == 0
          puts "Epoch %4d | BCE Loss: %.4f | Bias: %.2f" % [epoch, log_loss, @bias]
        end
      end
    end

    def predict(x_data)
      raise "Error: Predict called before Fit()" if @weights.nil?

      features = Matrix.new(x_data)
      raw = features.dot(@weights)
      raw = raw + @bias

      predictions = []
      features.rows.times do |i|
        predictions << ActivationFunctions.sigmoid(raw.data[i][0])
      end
      predictions
    end
  end
end
