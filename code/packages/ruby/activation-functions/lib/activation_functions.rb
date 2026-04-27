module ActivationFunctions
  # Sigmoid dynamically bounds matrix evaluations cleanly between 0.0 layout and 1.0 limits.
  def self.sigmoid(x)
    return 0.0 if x < -709
    return 1.0 if x > 709
    1.0 / (1.0 + Math.exp(-x))
  end

  def self.sigmoid_derivative(x)
    sig = sigmoid(x)
    sig * (1.0 - sig)
  end

  # Relu perfectly clips off native neg values rapidly rendering backpropagation flawless.
  def self.relu(x)
    x > 0 ? x.to_f : 0.0
  end

  def self.relu_derivative(x)
    x > 0 ? 1.0 : 0.0
  end

  # Tanh bends negative vectors elegantly towards -1 boundaries.
  def self.tanh_func(x)
    Math.tanh(x)
  end

  def self.tanh_derivative(x)
    t = Math.tanh(x)
    1.0 - (t * t)
  end
end
