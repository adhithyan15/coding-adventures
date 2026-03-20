class LengthMismatchError < StandardError; end

module GradientDescent
  def self.sgd(weights, gradients, learning_rate)
    if weights.length != gradients.length || weights.empty?
      raise LengthMismatchError, "Arrays must have the same non-zero length"
    end

    weights.zip(gradients).map do |w, g|
      w - (learning_rate * g)
    end
  end
end
