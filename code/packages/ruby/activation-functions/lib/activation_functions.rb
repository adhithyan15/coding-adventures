module ActivationFunctions
  def self.sigmoid(x)
    return 0.0 if x < -709
    return 1.0 if x > 709
    1.0 / (1.0 + Math.exp(-x))
  end

  def self.sigmoid_derivative(x)
    sig = sigmoid(x)
    sig * (1.0 - sig)
  end

  def self.relu(x)
    [0.0, x].max
  end

  def self.relu_derivative(x)
    x > 0.0 ? 1.0 : 0.0
  end

  def self.tanh(x)
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
