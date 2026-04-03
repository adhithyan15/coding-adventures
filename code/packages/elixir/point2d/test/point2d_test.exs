defmodule Point2DTest do
  use ExUnit.Case

  @delta 1.0e-9

  defp approx(a, b), do: abs(a - b) < @delta

  test "add" do
    {x, y} = Point2D.add({1, 2}, {3, 4})
    assert approx(x, 4) and approx(y, 6)
  end

  test "subtract" do
    {x, y} = Point2D.subtract({5, 3}, {2, 1})
    assert approx(x, 3) and approx(y, 2)
  end

  test "scale" do
    {x, y} = Point2D.scale({2, 3}, 2)
    assert approx(x, 4) and approx(y, 6)
  end

  test "negate" do
    {x, y} = Point2D.negate({1, -2})
    assert approx(x, -1) and approx(y, 2)
  end

  test "dot" do
    assert approx(Point2D.dot({1, 2}, {3, 4}), 11)
  end

  test "cross" do
    assert approx(Point2D.cross({1, 2}, {3, 4}), -2)
  end

  test "magnitude" do
    assert approx(Point2D.magnitude({3, 4}), 5)
  end

  test "magnitude_squared" do
    assert approx(Point2D.magnitude_squared({3, 4}), 25)
  end

  test "normalize" do
    n = Point2D.normalize({3, 4})
    assert approx(Point2D.magnitude(n), 1)
  end

  test "normalize zero vector" do
    n = Point2D.normalize({0, 0})
    assert n == {0, 0}
  end

  test "distance" do
    assert approx(Point2D.distance({0, 0}, {3, 4}), 5)
  end

  test "lerp at 0" do
    {x, _} = Point2D.lerp({0, 0}, {10, 10}, 0)
    assert approx(x, 0)
  end

  test "lerp at 1" do
    {x, _} = Point2D.lerp({0, 0}, {10, 10}, 1)
    assert approx(x, 10)
  end

  test "lerp at 0.5" do
    {x, _} = Point2D.lerp({0, 0}, {10, 0}, 0.5)
    assert approx(x, 5)
  end

  test "perpendicular" do
    {x, y} = Point2D.perpendicular({1, 0})
    assert approx(x, 0) and approx(y, 1)
  end

  test "angle" do
    assert approx(Point2D.angle({1, 1}), Trig.pi() / 4)
  end

  test "contains_point inside" do
    assert Point2D.contains_point?({0, 0, 10, 10}, {5, 5})
  end

  test "contains_point outside" do
    refute Point2D.contains_point?({0, 0, 10, 10}, {10, 5})
  end

  test "rect_union" do
    {x, y, w, h} = Point2D.rect_union({0, 0, 10, 10}, {5, 5, 10, 10})
    assert approx(x, 0) and approx(y, 0)
    assert approx(w, 15) and approx(h, 15)
  end

  test "rect_intersection overlap" do
    result = Point2D.rect_intersection({0, 0, 10, 10}, {5, 5, 10, 10})
    assert result != nil
    {x, y, w, h} = result
    assert approx(x, 5) and approx(y, 5)
    assert approx(w, 5) and approx(h, 5)
  end

  test "rect_intersection disjoint" do
    assert nil == Point2D.rect_intersection({0, 0, 5, 5}, {10, 10, 5, 5})
  end

  test "rect_expand" do
    {x, _y, w, _h} = Point2D.rect_expand({0, 0, 10, 10}, 2)
    assert approx(x, -2) and approx(w, 14)
  end
end
