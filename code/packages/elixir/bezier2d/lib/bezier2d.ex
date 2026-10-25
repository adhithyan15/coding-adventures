defmodule Bezier2D do
  @moduledoc """
  Quadratic and cubic Bezier curves.

  Curves are represented as tuples:
  - Quadratic: `{:quad, p0, p1, p2}`
  - Cubic:     `{:cubic, p0, p1, p2, p3}`

  Points are `{x, y}` tuples from the Point2D module.

  All evaluation uses de Casteljau's algorithm for numerical stability.
  """

  @max_flattening_depth 32
  @max_flattening_subdivisions 65_535

  # ---------------------------------------------------------------------------
  # Quadratic Bezier
  # ---------------------------------------------------------------------------

  @doc "Create a QuadraticBezier."
  def new_quad(p0, p1, p2), do: {:quad, p0, p1, p2}

  @doc "Evaluate a quadratic Bezier at t ∈ [0,1] via de Casteljau."
  def eval_quad({:quad, p0, p1, p2}, t) do
    q0 = Point2D.lerp(p0, p1, t)
    q1 = Point2D.lerp(p1, p2, t)
    Point2D.lerp(q0, q1, t)
  end

  @doc "Tangent vector of a quadratic Bezier at t."
  def deriv_quad({:quad, p0, p1, p2}, t) do
    d0 = Point2D.subtract(p1, p0)
    d1 = Point2D.subtract(p2, p1)
    Point2D.scale(Point2D.lerp(d0, d1, t), 2)
  end

  @doc "Split a quadratic Bezier at t into two quadratic Beziers."
  def split_quad({:quad, p0, p1, p2}, t) do
    q0 = Point2D.lerp(p0, p1, t)
    q1 = Point2D.lerp(p1, p2, t)
    m  = Point2D.lerp(q0, q1, t)
    {new_quad(p0, q0, m), new_quad(m, q1, p2)}
  end

  @doc """
  Adaptively subdivide a quadratic Bezier into a polyline within `tolerance`.
  Returns a list of `{x, y}` points.
  """
  def polyline_quad({:quad, p0, p1, p2} = q, tolerance) do
    flatten(q, tolerance, [p0, p1, p2])
  end

  @doc "Tight bounding box of a quadratic Bezier."
  def bbox_quad({:quad, p0, p1, p2} = q) do
    {min_x, max_x} = Enum.min_max([elem(p0, 0), elem(p2, 0)])
    {min_y, max_y} = Enum.min_max([elem(p0, 1), elem(p2, 1)])

    {min_x, max_x} = quad_extremum(min_x, max_x, elem(p0, 0), elem(p1, 0), elem(p2, 0),
      fn t -> elem(eval_quad(q, t), 0) end)
    {min_y, max_y} = quad_extremum(min_y, max_y, elem(p0, 1), elem(p1, 1), elem(p2, 1),
      fn t -> elem(eval_quad(q, t), 1) end)

    Point2D.new_rect(min_x, min_y, max_x - min_x, max_y - min_y)
  end

  @doc "Elevate a quadratic Bezier to an equivalent cubic."
  def elevate_quad({:quad, p0, p1, p2}) do
    q1 = Point2D.add(Point2D.scale(p0, 1.0 / 3), Point2D.scale(p1, 2.0 / 3))
    q2 = Point2D.add(Point2D.scale(p1, 2.0 / 3), Point2D.scale(p2, 1.0 / 3))
    new_cubic(p0, q1, q2, p2)
  end

  # ---------------------------------------------------------------------------
  # Cubic Bezier
  # ---------------------------------------------------------------------------

  @doc "Create a CubicBezier."
  def new_cubic(p0, p1, p2, p3), do: {:cubic, p0, p1, p2, p3}

  @doc "Evaluate a cubic Bezier at t via de Casteljau."
  def eval_cubic({:cubic, p0, p1, p2, p3}, t) do
    p01  = Point2D.lerp(p0, p1, t)
    p12  = Point2D.lerp(p1, p2, t)
    p23  = Point2D.lerp(p2, p3, t)
    p012 = Point2D.lerp(p01, p12, t)
    p123 = Point2D.lerp(p12, p23, t)
    Point2D.lerp(p012, p123, t)
  end

  @doc "Tangent vector of a cubic Bezier at t."
  def deriv_cubic({:cubic, p0, p1, p2, p3}, t) do
    d0 = Point2D.subtract(p1, p0)
    d1 = Point2D.subtract(p2, p1)
    d2 = Point2D.subtract(p3, p2)
    one_t = 1 - t
    r = Point2D.add(
          Point2D.add(
            Point2D.scale(d0, one_t * one_t),
            Point2D.scale(d1, 2 * one_t * t)
          ),
          Point2D.scale(d2, t * t)
        )
    Point2D.scale(r, 3)
  end

  @doc "Split a cubic Bezier at t into two cubics."
  def split_cubic({:cubic, p0, p1, p2, p3}, t) do
    p01   = Point2D.lerp(p0, p1, t)
    p12   = Point2D.lerp(p1, p2, t)
    p23   = Point2D.lerp(p2, p3, t)
    p012  = Point2D.lerp(p01, p12, t)
    p123  = Point2D.lerp(p12, p23, t)
    p0123 = Point2D.lerp(p012, p123, t)
    {new_cubic(p0, p01, p012, p0123), new_cubic(p0123, p123, p23, p3)}
  end

  @doc "Adaptively subdivide a cubic Bezier into a polyline within `tolerance`."
  def polyline_cubic({:cubic, p0, p1, p2, p3} = c, tolerance) do
    flatten(c, tolerance, [p0, p1, p2, p3])
  end

  @doc "Tight bounding box of a cubic Bezier."
  def bbox_cubic({:cubic, p0, p1, p2, p3} = c) do
    {min_x, max_x} = Enum.min_max([elem(p0, 0), elem(p3, 0)])
    {min_y, max_y} = Enum.min_max([elem(p0, 1), elem(p3, 1)])

    {min_x, max_x} = cubic_extrema(min_x, max_x, elem(p0, 0), elem(p1, 0), elem(p2, 0), elem(p3, 0),
      fn t -> elem(eval_cubic(c, t), 0) end)
    {min_y, max_y} = cubic_extrema(min_y, max_y, elem(p0, 1), elem(p1, 1), elem(p2, 1), elem(p3, 1),
      fn t -> elem(eval_cubic(c, t), 1) end)

    Point2D.new_rect(min_x, min_y, max_x - min_x, max_y - min_y)
  end

  # ---------------------------------------------------------------------------
  # Private helpers
  # ---------------------------------------------------------------------------

  defp flatten(curve, tolerance, controls) do
    if not is_number(tolerance) or tolerance <= 0.0,
      do: raise(ArgumentError, "tolerance must be finite and positive")

    unless Enum.all?(controls, fn {x, y} -> is_number(x) and is_number(y) end),
      do: raise(ArgumentError, "control points must be finite")

    try do
      curve
      |> flatten_pending(tolerance, 0, [{curve, 0}], [hd(controls)])
      |> Enum.reverse()
    rescue
      ArithmeticError -> raise RuntimeError, "Bezier subdivision produced nonfinite geometry"
    end
  end

  defp flatten_pending(_curve, _tolerance, _subdivisions, [], points), do: points

  defp flatten_pending(curve, tolerance, subdivisions, [{current, depth} | pending], points) do
    {endpoint, off_curve} =
      case current do
        {:quad, _p0, p1, p2} -> {p2, [p1]}
        {:cubic, _p0, p1, p2, p3} -> {p3, [p1, p2]}
      end

    start_point = elem(current, 1)
    error = off_curve |> Enum.map(&distance_to_segment(&1, start_point, endpoint)) |> Enum.max()

    if error <= tolerance do
      flatten_pending(curve, tolerance, subdivisions, pending, [endpoint | points])
    else
      if depth >= @max_flattening_depth or subdivisions >= @max_flattening_subdivisions,
        do: raise(RuntimeError, "Bezier flattening exceeded the depth or subdivision budget")

      {left, right} =
        case current do
          {:quad, _, _, _} -> split_quad(current, 0.5)
          {:cubic, _, _, _, _} -> split_cubic(current, 0.5)
        end

      flatten_pending(
        curve,
        tolerance,
        subdivisions + 1,
        [{left, depth + 1}, {right, depth + 1} | pending],
        points
      )
    end
  end

  defp distance_to_segment({px, py}, {sx, sy} = start_point, {ex, ey}) do
    dx = ex - sx
    dy = ey - sy
    scale = max(abs(dx), abs(dy))

    if scale == 0.0 do
      scaled_distance({px, py}, start_point)
    else
      nx = dx / scale
      ny = dy / scale
      denominator = nx * nx + ny * ny
      projection =
        min(1.0, max(0.0, ((px - sx) / scale * nx + (py - sy) / scale * ny) / denominator))

      scaled_distance({px, py}, {sx + projection * dx, sy + projection * dy})
    end
  end

  defp scaled_distance({x1, y1}, {x2, y2}) do
    dx = x1 - x2
    dy = y1 - y2
    scale = max(abs(dx), abs(dy))
    if scale == 0.0, do: 0.0, else: scale * :math.sqrt((dx / scale) ** 2 + (dy / scale) ** 2)
  end

  defp quad_extremum(min_v, max_v, v0, v1, v2, eval_fn) do
    denom = v0 - 2 * v1 + v2
    if abs(denom) < 1.0e-12 do
      {min_v, max_v}
    else
      tx = (v0 - v1) / denom
      if tx > 0 and tx < 1 do
        vt = eval_fn.(tx)
        {min(min_v, vt), max(max_v, vt)}
      else
        {min_v, max_v}
      end
    end
  end

  defp cubic_extrema(min_v, max_v, v0, v1, v2, v3, eval_fn) do
    a = -3 * v0 + 9 * v1 - 9 * v2 + 3 * v3
    b = 6 * v0 - 12 * v1 + 6 * v2
    c = -3 * v0 + 3 * v1
    roots = solve_quadratic(a, b, c)
    Enum.reduce(roots, {min_v, max_v}, fn tx, {mn, mx} ->
      vt = eval_fn.(tx)
      {min(mn, vt), max(mx, vt)}
    end)
  end

  defp solve_quadratic(a, b, c) do
    cond do
      abs(a) < 1.0e-12 and abs(b) > 1.0e-12 ->
        tx = -c / b
        if tx > 0 and tx < 1, do: [tx], else: []
      abs(a) < 1.0e-12 ->
        []
      true ->
        disc = b * b - 4 * a * c
        if disc < 0 do
          []
        else
          sq = Trig.sqrt(disc)
          [(-b + sq) / (2 * a), (-b - sq) / (2 * a)]
          |> Enum.filter(fn tx -> tx > 0 and tx < 1 end)
        end
    end
  end
end
