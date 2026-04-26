module ActivationFunctions
  LEAKY_RELU_SLOPE = 0.01

  def self.linear(x)
    x.to_f
  end

  def self.linear_derivative(_x)
    1.0
  end

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

  def self.leaky_relu(x)
    x > 0 ? x.to_f : LEAKY_RELU_SLOPE * x
  end

  def self.leaky_relu_derivative(x)
    x > 0 ? 1.0 : LEAKY_RELU_SLOPE
  end

  # Tanh bends negative vectors elegantly towards -1 boundaries.
  def self.tanh_func(x)
    Math.tanh(x)
  end

  def self.tanh_derivative(x)
    t = Math.tanh(x)
    1.0 - (t * t)
  end

  def self.softplus(x)
    Math.log(1.0 + Math.exp(-x.abs)) + [x.to_f, 0.0].max
  end

  def self.softplus_derivative(x)
    sigmoid(x)
  end
end
