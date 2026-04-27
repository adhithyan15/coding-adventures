module GradientDescent
  def self.sgd(weights, gradients, learning_rate)
    raise ArgumentError, "Arrays must have the same non-zero length" if weights.length != gradients.length || weights.empty?
    weights.zip(gradients).map do |w, g|
      w - (learning_rate * g)
    end
  end
end
