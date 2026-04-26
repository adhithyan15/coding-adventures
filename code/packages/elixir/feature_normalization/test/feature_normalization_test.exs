defmodule CodingAdventures.FeatureNormalizationTest do
  use ExUnit.Case

  alias CodingAdventures.FeatureNormalization

  @rows [
    [1000.0, 3.0, 1.0],
    [1500.0, 4.0, 0.0],
    [2000.0, 5.0, 1.0]
  ]

  defp assert_close(expected, actual) do
    assert abs(expected - actual) <= 1.0e-9
  end

  test "standard scaler centers and scales columns" do
    {:ok, scaler} = FeatureNormalization.fit_standard_scaler(@rows)
    assert_close(1500.0, Enum.at(scaler.means, 0))
    assert_close(4.0, Enum.at(scaler.means, 1))

    {:ok, transformed} = FeatureNormalization.transform_standard(@rows, scaler)
    assert_close(-1.224744871391589, transformed |> Enum.at(0) |> Enum.at(0))
    assert_close(0.0, transformed |> Enum.at(1) |> Enum.at(0))
    assert_close(1.224744871391589, transformed |> Enum.at(2) |> Enum.at(0))
  end

  test "min-max scaler maps columns to unit range" do
    {:ok, scaler} = FeatureNormalization.fit_min_max_scaler(@rows)
    {:ok, transformed} = FeatureNormalization.transform_min_max(@rows, scaler)

    assert transformed == [
             [0.0, 0.0, 1.0],
             [0.5, 0.5, 0.0],
             [1.0, 1.0, 1.0]
           ]
  end

  test "constant columns map to zero" do
    rows = [[1.0, 7.0], [2.0, 7.0]]

    {:ok, standard_scaler} = FeatureNormalization.fit_standard_scaler(rows)
    {:ok, min_max_scaler} = FeatureNormalization.fit_min_max_scaler(rows)
    {:ok, standard} = FeatureNormalization.transform_standard(rows, standard_scaler)
    {:ok, min_max} = FeatureNormalization.transform_min_max(rows, min_max_scaler)

    assert 0.0 == standard |> Enum.at(0) |> Enum.at(1)
    assert 0.0 == min_max |> Enum.at(0) |> Enum.at(1)
  end
end
