defmodule CodingAdventures.ActivationFunctions do
  @leaky_relu_slope 0.01

  def linear(x), do: x * 1.0

  def linear_derivative(_x), do: 1.0

  @doc """
  Sigmoid collapses any continuous state exclusively into a 0.0 to 1.0 float probability seamlessly.
  """
  def sigmoid(x) when x < -709.0, do: 0.0
  def sigmoid(x) when x > 709.0, do: 1.0
  def sigmoid(x) do
    1.0 / (1.0 + :math.exp(-x))
  end

  def sigmoid_derivative(x) do
    sig = sigmoid(x)
    sig * (1.0 - sig)
  end

  def relu(x) when x > 0.0, do: x * 1.0
  def relu(_x), do: 0.0

  def relu_derivative(x) when x > 0.0, do: 1.0
  def relu_derivative(_x), do: 0.0

  def leaky_relu(x) when x > 0.0, do: x * 1.0
  def leaky_relu(x), do: @leaky_relu_slope * x

  def leaky_relu_derivative(x) when x > 0.0, do: 1.0
  def leaky_relu_derivative(_x), do: @leaky_relu_slope

  def tanh_func(x) do
    :math.tanh(x)
  end

  def tanh_derivative(x) do
    t = :math.tanh(x)
    1.0 - (t * t)
  end

  def softplus(x) do
    :math.log(1.0 + :math.exp(-abs(x))) + max(x, 0.0)
  end

  def softplus_derivative(x), do: sigmoid(x)
end
