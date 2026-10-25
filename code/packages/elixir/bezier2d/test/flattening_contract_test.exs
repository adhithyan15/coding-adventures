defmodule Bezier2DFlatteningContractTest do
  use ExUnit.Case, async: true

  @fixture Path.expand("../../../../specs/fixtures/bezier2d-flattening-v1/cases.json", __DIR__)
           |> File.read!()
           |> :json.decode()

  defp curve(test_case) do
    points = Enum.map(test_case["control_points"], fn [x, y] -> {x, y} end)
    case test_case["degree"] do
      2 -> apply(Bezier2D, :new_quad, points)
      3 -> apply(Bezier2D, :new_cubic, points)
    end
  end

  defp flatten({:quad, _, _, _} = value, tolerance), do: Bezier2D.polyline_quad(value, tolerance)
  defp flatten({:cubic, _, _, _, _} = value, tolerance), do: Bezier2D.polyline_cubic(value, tolerance)

  test "dynamic neutral flattening cases" do
    Enum.each(@fixture["cases"], fn test_case ->
      value = curve(test_case)
      cond do
        test_case["expected_disposition"] == "invalid-tolerance" ->
          assert_raise ArgumentError, ~r/tolerance/, fn -> flatten(value, test_case["tolerance"]) end

        test_case["expected_termination"] == "budget-error" ->
          assert_raise RuntimeError, ~r/budget/, fn -> flatten(value, test_case["tolerance"]) end

        true ->
          points = flatten(value, test_case["tolerance"])
          assert hd(points) == elem(value, 1)
          assert List.last(points) == elem(value, tuple_size(value) - 1)
          assert Enum.all?(points, fn {x, y} -> is_number(x) and is_number(y) end)
          if test_case["expected_disposition"] == "chord", do: assert(length(points) == 2), else: assert(length(points) > 2)
      end
    end)
  end

  test "invalid native inputs and derived arithmetic fail closed" do
    straight = Bezier2D.new_quad({0.0, 0.0}, {0.5, 0.0}, {1.0, 0.0})
    for tolerance <- [0.0, -1.0] do
      assert_raise ArgumentError, ~r/tolerance/, fn -> Bezier2D.polyline_quad(straight, tolerance) end
    end
    overflow = Bezier2D.new_quad({1.0e308, 0.0}, {0.0, 1.0}, {-1.0e308, 0.0})
    assert_raise RuntimeError, ~r/finite/, fn -> Bezier2D.polyline_quad(overflow, 0.1) end
  end

  test "subnormal geometry cannot underflow into a chord" do
    curve = Bezier2D.new_quad({0.0, 0.0}, {0.0, 1.0e-200}, {1.0e-200, 0.0})
    assert_raise RuntimeError, ~r/budget/, fn -> Bezier2D.polyline_quad(curve, 5.0e-324) end
  end
end
