defmodule ActivationFunctions do
  def sigmoid(x) do
    cond do
      x < -709 -> 0.0
      x > 709 -> 1.0
      true -> 1.0 / (1.0 + :math.exp(-x))
    end
  end

  def sigmoid_derivative(x) do
    sig = sigmoid(x)
    sig * (1.0 - sig)
  end

  def relu(x) do
    if x > 0.0, do: x, else: 0.0
  end

  def relu_derivative(x) do
    if x > 0.0, do: 1.0, else: 0.0
  end

  def tanh(x) do
    :math.tanh(x)
  end

  def tanh_derivative(x) do
    t = :math.tanh(x)
    1.0 - (t * t)
  end
end
