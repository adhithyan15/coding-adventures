defmodule CodingAdventures.GradientDescent do
  def sgd(weights, gradients, learning_rate) when length(weights) == length(gradients) and length(weights) > 0 do
    weights
    |> Enum.zip(gradients)
    |> Enum.map(fn {w, g} -> w - (learning_rate * g) end)
  end
  def sgd(_, _, _), do: {:error, :length_mismatch}
end
