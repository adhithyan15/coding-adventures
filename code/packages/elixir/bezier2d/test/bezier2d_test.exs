defmodule Bezier2DTest do
  use ExUnit.Case

  @delta 1.0e-9
  defp approx(a, b), do: abs(a - b) < @delta
  defp pt_approx({x1, y1}, {x2, y2}), do: approx(x1, x2) and approx(y1, y2)

  @q Bezier2D.new_quad({0, 0}, {1, 2}, {2, 0})

  test "quad eval at 0" do
    assert pt_approx(Bezier2D.eval_quad(@q, 0), {0, 0})
  end

  test "quad eval at 1" do
    assert pt_approx(Bezier2D.eval_quad(@q, 1), {2, 0})
  end

  test "quad eval midpoint" do
    assert pt_approx(Bezier2D.eval_quad(@q, 0.5), {1, 1})
  end

  test "quad split midpoints match" do
    {left, right} = Bezier2D.split_quad(@q, 0.5)
    m = Bezier2D.eval_quad(@q, 0.5)
    assert pt_approx(elem(left, 3), m)
    assert pt_approx(elem(right, 1), m)
  end

  test "quad polyline straight" do
    straight = Bezier2D.new_quad({0, 0}, {1, 0}, {2, 0})
    pts = Bezier2D.polyline_quad(straight, 0.1)
    assert length(pts) == 2
  end

  test "quad bbox contains endpoints" do
    {rx, _ry, rw, _rh} = Bezier2D.bbox_quad(@q)
    assert rx <= 0 and rx + rw >= 2
  end

  test "quad elevate equivalent" do
    c = Bezier2D.elevate_quad(@q)
    Enum.each([0, 0.25, 0.5, 0.75, 1], fn t ->
      qp = Bezier2D.eval_quad(@q, t)
      cp = Bezier2D.eval_cubic(c, t)
      assert pt_approx(qp, cp)
    end)
  end

  @c Bezier2D.new_cubic({0, 0}, {1, 2}, {3, 2}, {4, 0})

  test "cubic eval at 0" do
    assert pt_approx(Bezier2D.eval_cubic(@c, 0), {0, 0})
  end

  test "cubic eval at 1" do
    assert pt_approx(Bezier2D.eval_cubic(@c, 1), {4, 0})
  end

  test "cubic symmetric midpoint x" do
    {x, _} = Bezier2D.eval_cubic(@c, 0.5)
    assert approx(x, 2)
  end

  test "cubic split midpoints match" do
    {left, right} = Bezier2D.split_cubic(@c, 0.5)
    m = Bezier2D.eval_cubic(@c, 0.5)
    assert pt_approx(elem(left, 4), m)
    assert pt_approx(elem(right, 1), m)
  end

  test "cubic polyline straight" do
    straight = Bezier2D.new_cubic({0, 0}, {1, 0}, {2, 0}, {3, 0})
    pts = Bezier2D.polyline_cubic(straight, 0.1)
    assert length(pts) == 2
  end

  test "cubic bbox contains samples" do
    {bx, by, bw, bh} = Bezier2D.bbox_cubic(@c)
    Enum.each(0..20, fn i ->
      {px, _py} = Bezier2D.eval_cubic(@c, i / 20.0)
      assert px >= bx - 1.0e-6 and px <= bx + bw + 1.0e-6
    end)
  end
end
