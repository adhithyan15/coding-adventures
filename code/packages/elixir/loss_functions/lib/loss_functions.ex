defmodule CodingAdventures.LossFunctions do
  @moduledoc """
  Provides pure, composable mathematical operations for calculating standard 
  machine learning loss (error) metrics.

  ## Literate Programming Notes
  As a teaching toolkit, this package eschews complex object-oriented patterns
  in favor of pure functional programming paradigms, utilizing data pipelines
  and operating directly on foundational Elixir lists.
  """

  @epsilon 1.0e-7

  @doc """
  Calculates Mean Squared Error (MSE).

  MSE measures the average of the squares of the errors—that is, the average squared
  difference between the estimated values (`y_pred`) and the actual value (`y_true`).
  It is typically used for regression tasks, and heavily penalizes outliers.

  ## Equation
      MSE = (1/N) * sum_{i=1}^{N} (y_true_i - y_pred_i)^2

  ## Examples

      iex> CodingAdventures.LossFunctions.mse([1.0, 0.0], [0.9, 0.1])
      0.01

  """
  def mse(y_true, y_pred) when length(y_true) == length(y_pred) and length(y_true) > 0 do
    y_true
    |> Enum.zip(y_pred)
    |> Enum.map(fn {t, p} -> (t - p) * (t - p) end)
    |> Enum.sum()
    |> Kernel./(length(y_true))
  end
  def mse(_, _), do: {:error, :length_mismatch}

  @doc """
  Calculates Mean Absolute Error (MAE).

  MAE measures the absolute magnitude of the errors without considering direction.
  It is widely used in Robust Regression to ignore extreme outliers.

  ## Equation
      MAE = (1/N) * sum_{i=1}^{N} |y_true_i - y_pred_i|

  ## Examples

      iex> CodingAdventures.LossFunctions.mae([1.0, 0.0], [0.9, 0.1])
      0.1

  """
  def mae(y_true, y_pred) when length(y_true) == length(y_pred) and length(y_true) > 0 do
    y_true
    |> Enum.zip(y_pred)
    |> Enum.map(fn {t, p} -> abs(t - p) end)
    |> Enum.sum()
    |> Kernel./(length(y_true))
  end
  def mae(_, _), do: {:error, :length_mismatch}

  @doc """
  Calculates Binary Cross-Entropy (BCE).

  BCE is used for binary classification tasks. It quantifies the difference
  between two probability distributions. Predictions must be between 0 and 1.
  It utilizes an epsilon clamp to prevent taking the logarithm of 0.

  ## Equation
      BCE = -(1/n) * sum_{i=1}^{N} [y_true_i * log(y_pred_i) + (1 - y_true_i) * log(1 - y_pred_i)]

  ## Examples

      iex> CodingAdventures.LossFunctions.bce([1.0, 0.0], [0.9, 0.1])
      0.1053605

  """
  def bce(y_true, y_pred) when length(y_true) == length(y_pred) and length(y_true) > 0 do
    sum =
      y_true
      |> Enum.zip(y_pred)
      |> Enum.map(fn {t, p} ->
        p = clamp(p, @epsilon, 1.0 - @epsilon)
        t * :math.log(p) + (1.0 - t) * :math.log(1.0 - p)
      end)
      |> Enum.sum()

    -sum / length(y_true)
  end
  def bce(_, _), do: {:error, :length_mismatch}

  @doc """
  Calculates Categorical Cross-Entropy (CCE).

  CCE is used for multi-class classification tasks where only one class is correct.
  It assumes the true labels are one-hot encoded.
  It utilizes an epsilon clamp to prevent taking the logarithm of 0.

  ## Equation
      CCE = -(1/n) * sum_{i=1}^{N} [y_true_i * log(y_pred_i)]

  ## Examples

      iex> CodingAdventures.LossFunctions.cce([1.0, 0.0], [0.9, 0.1])
      0.0526802

  """
  def cce(y_true, y_pred) when length(y_true) == length(y_pred) and length(y_true) > 0 do
    sum =
      y_true
      |> Enum.zip(y_pred)
      |> Enum.map(fn {t, p} ->
        p = clamp(p, @epsilon, 1.0 - @epsilon)
        t * :math.log(p)
      end)
      |> Enum.sum()

    -sum / length(y_true)
  end
  def cce(_, _), do: {:error, :length_mismatch}

  defp clamp(val, min, max) do
    if val < min do
      min
    else
      if val > max do
        max
      else
        val
      end
    end
  end
end
