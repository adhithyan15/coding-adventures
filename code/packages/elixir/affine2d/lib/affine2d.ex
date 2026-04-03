defmodule Affine2D do
  @moduledoc """
  2D affine transformation matrix in SVG/Canvas convention.

  Stored as `{a, b, c, d, e, f}` matching the SVG `matrix(a,b,c,d,e,f)` format:

      [ a  c  e ]
      [ b  d  f ]
      [ 0  0  1 ]

  Mapping: x' = a*x + c*y + e
           y' = b*x + d*y + f
  """

  @doc "Identity transform."
  def identity, do: {1.0, 0.0, 0.0, 1.0, 0.0, 0.0}

  @doc "Pure translation by (tx, ty)."
  def translate(tx, ty), do: {1.0, 0.0, 0.0, 1.0, tx, ty}

  @doc "CCW rotation by `angle_rad` about the origin."
  def rotate(angle_rad) do
    c = Trig.cos(angle_rad)
    s = Trig.sin(angle_rad)
    {c, s, -s, c, 0.0, 0.0}
  end

  @doc "CCW rotation about pivot `{px, py}`."
  def rotate_around(angle_rad, px, py) do
    compose(compose(translate(px, py), rotate(angle_rad)), translate(-px, -py))
  end

  @doc "Non-uniform scale."
  def scale(sx, sy), do: {sx, 0.0, 0.0, sy, 0.0, 0.0}

  @doc "Uniform scale."
  def scale_uniform(s), do: scale(s, s)

  @doc "Shear along the x-axis."
  def skew_x(angle_rad), do: {1.0, 0.0, Trig.tan(angle_rad), 1.0, 0.0, 0.0}

  @doc "Shear along the y-axis."
  def skew_y(angle_rad), do: {1.0, Trig.tan(angle_rad), 0.0, 1.0, 0.0, 0.0}

  @doc """
  Compose `a` then `b` (apply `a` first, then `b`).

  Matrix multiplication for 2×3 affine matrices:

      [a1 c1 e1]   [a2 c2 e2]   [a1*a2+c1*b2  a1*c2+c1*d2  a1*e2+c1*f2+e1]
      [b1 d1 f1] × [b2 d2 f2] = [b1*a2+d1*b2  b1*c2+d1*d2  b1*e2+d1*f2+f1]
  """
  def compose({a1, b1, c1, d1, e1, f1}, {a2, b2, c2, d2, e2, f2}) do
    {
      a1 * a2 + c1 * b2,
      b1 * a2 + d1 * b2,
      a1 * c2 + c1 * d2,
      b1 * c2 + d1 * d2,
      a1 * e2 + c1 * f2 + e1,
      b1 * e2 + d1 * f2 + f1
    }
  end

  @doc "Apply transform to a position point (includes translation)."
  def apply_to_point({a, b, c, d, e, f}, {x, y}) do
    {a * x + c * y + e, b * x + d * y + f}
  end

  @doc "Apply transform to a direction vector (excludes translation)."
  def apply_to_vector({a, b, c, d, _e, _f}, {x, y}) do
    {a * x + c * y, b * x + d * y}
  end

  @doc "Determinant: ad - bc."
  def determinant({a, b, c, d, _e, _f}), do: a * d - b * c

  @doc "Inverse matrix, or nil if singular."
  def invert(m) do
    det = determinant(m)
    if abs(det) < 1.0e-12 do
      nil
    else
      {a, b, c, d, e, f} = m
      inv = 1.0 / det
      {
        d * inv,
        -b * inv,
        -c * inv,
        a * inv,
        (c * f - d * e) * inv,
        (b * e - a * f) * inv
      }
    end
  end

  @doc "True if transform is the identity."
  def identity?(m), do: m == {1.0, 0.0, 0.0, 1.0, 0.0, 0.0}

  @doc "True if only translation is present (no rotation/scale/skew)."
  def translation_only?({a, b, c, d, _e, _f}), do: a == 1.0 and b == 0.0 and c == 0.0 and d == 1.0

  @doc "Return `[a, b, c, d, e, f]` as a list (SVG matrix order)."
  def to_list({a, b, c, d, e, f}), do: [a, b, c, d, e, f]
end
