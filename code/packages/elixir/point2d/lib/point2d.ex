defmodule Point2D do
  @moduledoc """
  Immutable 2D point/vector and axis-aligned bounding rectangle.

  ## Point

  A `Point` is a pair `{x, y}` representing a position or direction vector.
  All arithmetic operations return new values (pure functional style).

  ## Rect

  A `Rect` is `{x, y, width, height}` — the same convention as CSS and SVG.
  Containment is half-open: `[x, x+w) × [y, y+h)`.
  """

  # ---------------------------------------------------------------------------
  # Point
  # ---------------------------------------------------------------------------

  @doc "Create a point."
  def new_point(x, y), do: {x, y}

  @doc "Add two points: (a+c, b+d)."
  def add({x1, y1}, {x2, y2}), do: {x1 + x2, y1 + y2}

  @doc "Subtract: self - other."
  def subtract({x1, y1}, {x2, y2}), do: {x1 - x2, y1 - y2}

  @doc "Scale by scalar s."
  def scale({x, y}, s), do: {x * s, y * s}

  @doc "Negate: (-x, -y)."
  def negate({x, y}), do: {-x, -y}

  @doc "Dot product: x1*x2 + y1*y2."
  def dot({x1, y1}, {x2, y2}), do: x1 * x2 + y1 * y2

  @doc "2D cross product: x1*y2 - y1*x2."
  def cross({x1, y1}, {x2, y2}), do: x1 * y2 - y1 * x2

  @doc "Squared magnitude: x^2 + y^2."
  def magnitude_squared({x, y}), do: x * x + y * y

  @doc "Euclidean magnitude: sqrt(x^2 + y^2)."
  def magnitude(p), do: Trig.sqrt(magnitude_squared(p))

  @doc """
  Normalize to unit length. Returns the original point if magnitude is zero
  (avoids division by zero).
  """
  def normalize(p) do
    m = magnitude(p)
    if m < 1.0e-15, do: p, else: scale(p, 1.0 / m)
  end

  @doc "Squared distance between two points."
  def distance_squared(a, b), do: magnitude_squared(subtract(a, b))

  @doc "Euclidean distance between two points."
  def distance(a, b), do: Trig.sqrt(distance_squared(a, b))

  @doc """
  Linear interpolation from `a` to `b` at parameter `t`.
  At t=0 returns a; at t=1 returns b.
  """
  def lerp({x1, y1}, {x2, y2}, t), do: {x1 + t * (x2 - x1), y1 + t * (y2 - y1)}

  @doc "Perpendicular vector (90° CCW): (-y, x)."
  def perpendicular({x, y}), do: {-y, x}

  @doc "Angle of vector from +X axis, in radians [-π, π]."
  def angle({x, y}), do: Trig.atan2(y, x)

  # ---------------------------------------------------------------------------
  # Rect
  # ---------------------------------------------------------------------------

  @doc "Create a rect from (x, y, width, height)."
  def new_rect(x, y, w, h), do: {x, y, w, h}

  @doc "True if point `pt` is inside the half-open rect `[x, x+w) × [y, y+h)`."
  def contains_point?({rx, ry, rw, rh}, {px, py}) do
    px >= rx and px < rx + rw and py >= ry and py < ry + rh
  end

  @doc "Smallest rect that contains both `r1` and `r2`."
  def rect_union({x1, y1, w1, h1}, {x2, y2, w2, h2}) do
    x0 = min(x1, x2)
    y0 = min(y1, y2)
    x1e = max(x1 + w1, x2 + w2)
    y1e = max(y1 + h1, y2 + h2)
    {x0, y0, x1e - x0, y1e - y0}
  end

  @doc "Intersection of two rects, or nil if disjoint."
  def rect_intersection({x1, y1, w1, h1}, {x2, y2, w2, h2}) do
    x0 = max(x1, x2)
    y0 = max(y1, y2)
    xe = min(x1 + w1, x2 + w2)
    ye = min(y1 + h1, y2 + h2)
    if xe > x0 and ye > y0, do: {x0, y0, xe - x0, ye - y0}, else: nil
  end

  @doc "Expand rect by `margin` on all four sides."
  def rect_expand({x, y, w, h}, margin) do
    {x - margin, y - margin, w + 2 * margin, h + 2 * margin}
  end
end
