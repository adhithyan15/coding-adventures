defmodule GradientDescent do
  def sgd(weights, gradients, learning_rate) do
    if length(weights) != length(gradients) or length(weights) == 0 do
      raise ArgumentError, "Arrays must have the same non-zero length"
    end
    Enum.zip(weights, gradients)
    |> Enum.map(fn {w, g} -> w - (learning_rate * g) end)
  end
end
