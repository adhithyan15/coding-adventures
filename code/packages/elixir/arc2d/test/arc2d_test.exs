defmodule Arc2DTest do
  use ExUnit.Case

  @delta 1.0e-6
  defp approx(a, b), do: abs(a - b) < @delta
  defp pt_approx({x1, y1}, {x2, y2}), do: approx(x1, x2) and approx(y1, y2)

  @unit_arc Arc2D.new_center_arc({0, 0}, 1, 1, 0, :math.pi() / 2, 0)

  test "eval at 0" do
    assert pt_approx(Arc2D.eval_arc(@unit_arc, 0), {1, 0})
  end

  test "eval at 1" do
    assert pt_approx(Arc2D.eval_arc(@unit_arc, 1), {0, 1})
  end

  test "eval midpoint" do
    {x, y} = Arc2D.eval_arc(@unit_arc, 0.5)
    expected = 1.0 / :math.sqrt(2)
    assert approx(x, expected) and approx(y, expected)
  end

  test "bbox full circle contains unit circle" do
    full = Arc2D.new_center_arc({0, 0}, 1, 1, 0, 2 * Trig.pi(), 0)
    {bx, _by, bw, _bh} = Arc2D.bbox_arc(full)
    assert bx <= -0.99 and bx + bw >= 0.99
  end

  test "to_cubic_beziers quarter circle endpoints" do
    curves = Arc2D.to_cubic_beziers(@unit_arc)
    assert length(curves) > 0
    {:cubic, p0, _p1, _p2, _p3} = hd(curves)
    assert pt_approx(p0, {1, 0})
    {:cubic, _p0, _p1, _p2, p3} = List.last(curves)
    assert pt_approx(p3, {0, 1})
  end

  test "degenerate same point" do
    arc = Arc2D.new_svg_arc({1, 1}, {1, 1}, 1, 1, 0, false, false)
    assert :degenerate == Arc2D.to_center_arc(arc)
  end

  test "degenerate zero radius" do
    arc = Arc2D.new_svg_arc({0, 0}, {1, 0}, 0, 1, 0, false, false)
    assert :degenerate == Arc2D.to_center_arc(arc)
  end

  test "semicircle center" do
    arc = Arc2D.new_svg_arc({1, 0}, {-1, 0}, 1, 1, 0, false, true)
    {:ok, ca} = Arc2D.to_center_arc(arc)
    {:center_arc, {cx, cy}, _, _, _, _, _} = ca
    assert approx(cx, 0) and approx(cy, 0)
  end

  test "semicircle endpoints reproduced" do
    arc = Arc2D.new_svg_arc({1, 0}, {-1, 0}, 1, 1, 0, false, true)
    {:ok, ca} = Arc2D.to_center_arc(arc)
    assert pt_approx(Arc2D.eval_arc(ca, 0), {1, 0})
    assert pt_approx(Arc2D.eval_arc(ca, 1), {-1, 0})
  end
end
