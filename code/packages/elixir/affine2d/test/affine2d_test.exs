defmodule Affine2DTest do
  use ExUnit.Case

  @delta 1.0e-9
  defp approx(a, b), do: abs(a - b) < @delta
  defp pt_approx({x1, y1}, {x2, y2}), do: approx(x1, x2) and approx(y1, y2)

  test "identity apply" do
    pt = Affine2D.apply_to_point(Affine2D.identity(), {3, 4})
    assert pt_approx(pt, {3, 4})
  end

  test "translate" do
    pt = Affine2D.apply_to_point(Affine2D.translate(2, 3), {1, 1})
    assert pt_approx(pt, {3, 4})
  end

  test "rotate 90 degrees" do
    pt = Affine2D.apply_to_point(Affine2D.rotate(Trig.pi() / 2), {1, 0})
    assert approx(elem(pt, 0), 0) and approx(elem(pt, 1), 1)
  end

  test "scale non-uniform" do
    pt = Affine2D.apply_to_point(Affine2D.scale(2, 3), {1, 1})
    assert pt_approx(pt, {2, 3})
  end

  test "scale uniform" do
    pt = Affine2D.apply_to_point(Affine2D.scale_uniform(5), {2, 3})
    assert pt_approx(pt, {10, 15})
  end

  test "compose translations" do
    a = Affine2D.compose(Affine2D.translate(1, 0), Affine2D.translate(0, 2))
    pt = Affine2D.apply_to_point(a, {0, 0})
    assert pt_approx(pt, {1, 2})
  end

  test "determinant identity" do
    assert approx(Affine2D.determinant(Affine2D.identity()), 1)
  end

  test "determinant scale" do
    assert approx(Affine2D.determinant(Affine2D.scale(2, 3)), 6)
  end

  test "invert identity" do
    inv = Affine2D.invert(Affine2D.identity())
    assert inv != nil
    assert Affine2D.identity?(inv)
  end

  test "invert translate roundtrip" do
    a = Affine2D.translate(3, -5)
    inv = Affine2D.invert(a)
    assert inv != nil
    pt = Affine2D.apply_to_point(inv, Affine2D.apply_to_point(a, {1, 2}))
    assert pt_approx(pt, {1, 2})
  end

  test "invert singular" do
    singular = Affine2D.scale(0, 1)
    assert nil == Affine2D.invert(singular)
  end

  test "is identity" do
    assert Affine2D.identity?(Affine2D.identity())
    refute Affine2D.identity?(Affine2D.translate(1, 0))
  end

  test "translation only" do
    assert Affine2D.translation_only?(Affine2D.translate(5, -3))
    refute Affine2D.translation_only?(Affine2D.rotate(0.1))
  end

  test "to_list" do
    assert [1.0, 0.0, 0.0, 1.0, 0.0, 0.0] == Affine2D.to_list(Affine2D.identity())
  end

  test "apply_to_vector excludes translation" do
    v = Affine2D.apply_to_vector(Affine2D.translate(100, 100), {1, 0})
    assert pt_approx(v, {1, 0})
  end

  test "rotate_around pivot stays fixed" do
    a = Affine2D.rotate_around(Trig.pi() / 2, 1, 0)
    pt = Affine2D.apply_to_point(a, {1, 0})
    assert approx(elem(pt, 0), 1) and approx(elem(pt, 1), 0)
  end
end
