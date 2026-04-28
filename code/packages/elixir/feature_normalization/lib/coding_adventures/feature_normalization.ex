defmodule CodingAdventures.FeatureNormalization do
  @moduledoc """
  Feature scaling utilities for machine-learning examples.
  """

  defmodule StandardScaler do
    defstruct means: [], standard_deviations: []
  end

  defmodule MinMaxScaler do
    defstruct minimums: [], maximums: []
  end

  def fit_standard_scaler(rows) do
    with {:ok, matrix, width} <- validate_matrix(rows) do
      count = length(matrix)

      means =
        for col <- 0..(width - 1) do
          matrix |> Enum.map(&Enum.at(&1, col)) |> Enum.sum() |> Kernel./(count)
        end

      standard_deviations =
        for col <- 0..(width - 1) do
          variance =
            matrix
            |> Enum.map(fn row ->
              diff = Enum.at(row, col) - Enum.at(means, col)
              diff * diff
            end)
            |> Enum.sum()
            |> Kernel./(count)

          :math.sqrt(variance)
        end

      {:ok, %StandardScaler{means: means, standard_deviations: standard_deviations}}
    end
  end

  def transform_standard(rows, %StandardScaler{} = scaler) do
    with {:ok, matrix, width} <- validate_matrix(rows),
         :ok <- require_width(width, length(scaler.means)) do
      {:ok,
       Enum.map(matrix, fn row ->
         row
         |> Enum.with_index()
         |> Enum.map(fn {value, col} ->
           std = Enum.at(scaler.standard_deviations, col)
           if std == 0.0, do: 0.0, else: (value - Enum.at(scaler.means, col)) / std
         end)
       end)}
    end
  end

  def fit_min_max_scaler(rows) do
    with {:ok, matrix, width} <- validate_matrix(rows) do
      minimums =
        for col <- 0..(width - 1), do: matrix |> Enum.map(&Enum.at(&1, col)) |> Enum.min()

      maximums =
        for col <- 0..(width - 1), do: matrix |> Enum.map(&Enum.at(&1, col)) |> Enum.max()

      {:ok, %MinMaxScaler{minimums: minimums, maximums: maximums}}
    end
  end

  def transform_min_max(rows, %MinMaxScaler{} = scaler) do
    with {:ok, matrix, width} <- validate_matrix(rows),
         :ok <- require_width(width, length(scaler.minimums)) do
      {:ok,
       Enum.map(matrix, fn row ->
         row
         |> Enum.with_index()
         |> Enum.map(fn {value, col} ->
           min = Enum.at(scaler.minimums, col)
           span = Enum.at(scaler.maximums, col) - min
           if span == 0.0, do: 0.0, else: (value - min) / span
         end)
       end)}
    end
  end

  defp validate_matrix([]), do: {:error, :empty_matrix}

  defp validate_matrix(rows) do
    matrix = Enum.map(rows, fn row -> Enum.map(row, &(&1 * 1.0)) end)
    width = matrix |> hd() |> length()

    cond do
      width == 0 -> {:error, :empty_matrix}
      Enum.any?(matrix, &(length(&1) != width)) -> {:error, :ragged_matrix}
      true -> {:ok, matrix, width}
    end
  end

  defp require_width(actual, expected) when actual == expected, do: :ok
  defp require_width(_actual, _expected), do: {:error, :width_mismatch}
end
