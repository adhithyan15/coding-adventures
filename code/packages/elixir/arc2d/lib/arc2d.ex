defmodule Arc2D do
  @moduledoc """
  Elliptical arc in center form and SVG endpoint form.

  ## CenterArc

  Represented as `{:center_arc, center, rx, ry, start_angle, sweep_angle, x_rotation}`.

  ## SvgArc

  Represented as `{:svg_arc, from_pt, to_pt, rx, ry, x_rotation, large_arc, sweep}`.
  """

  # ---------------------------------------------------------------------------
  # CenterArc
  # ---------------------------------------------------------------------------

  @doc "Create a CenterArc."
  def new_center_arc(center, rx, ry, start_angle, sweep_angle, x_rotation) do
    {:center_arc, center, rx, ry, start_angle, sweep_angle, x_rotation}
  end

  @doc """
  Evaluate the arc at t ∈ [0,1].

  θ = start_angle + t * sweep_angle.
  Local point: (rx*cos(θ), ry*sin(θ)).
  Rotate by x_rotation and translate by center.
  """
  def eval_arc({:center_arc, {cx, cy}, rx, ry, start_angle, sweep_angle, x_rotation}, t) do
    theta = start_angle + t * sweep_angle
    cos_t = Trig.cos(theta)
    sin_t = Trig.sin(theta)
    lx = rx * cos_t
    ly = ry * sin_t
    cos_r = Trig.cos(x_rotation)
    sin_r = Trig.sin(x_rotation)
    {cx + cos_r * lx - sin_r * ly, cy + sin_r * lx + cos_r * ly}
  end

  @doc "Tangent direction at t (unnormalized)."
  def tangent_arc({:center_arc, _center, rx, ry, start_angle, sweep_angle, x_rotation}, t) do
    theta = start_angle + t * sweep_angle
    cos_t = Trig.cos(theta)
    sin_t = Trig.sin(theta)
    cos_r = Trig.cos(x_rotation)
    sin_r = Trig.sin(x_rotation)
    dlx = -rx * sin_t
    dly =  ry * cos_t
    {
      sweep_angle * (cos_r * dlx - sin_r * dly),
      sweep_angle * (sin_r * dlx + cos_r * dly)
    }
  end

  @doc "Bounding box via 100-point sampling."
  def bbox_arc(arc) do
    {x0, y0} = eval_arc(arc, 0)
    {min_x, max_x, min_y, max_y} =
      Enum.reduce(1..100, {x0, x0, y0, y0}, fn i, {mnx, mxx, mny, mxy} ->
        {px, py} = eval_arc(arc, i / 100.0)
        {min(mnx, px), max(mxx, px), min(mny, py), max(mxy, py)}
      end)
    Point2D.new_rect(min_x, min_y, max_x - min_x, max_y - min_y)
  end

  @doc """
  Approximate the arc as cubic Bezier segments.
  Each segment spans at most π/2. Uses k = (4/3)*tan(s/4).
  """
  def to_cubic_beziers({:center_arc, {cx, cy}, rx, ry, start_angle, sweep_angle, x_rotation}) do
    half_pi = Trig.pi() / 2
    n_seg = max(1, ceil(abs(sweep_angle) / half_pi))
    seg_sweep = sweep_angle / n_seg
    cos_r = Trig.cos(x_rotation)
    sin_r = Trig.sin(x_rotation)

    local_to_world = fn lx, ly ->
      {cx + cos_r * lx - sin_r * ly, cy + sin_r * lx + cos_r * ly}
    end

    Enum.map(0..(n_seg - 1), fn i ->
      t0 = start_angle + i * seg_sweep
      t1 = t0 + seg_sweep
      k  = 4.0 / 3.0 * Trig.tan(seg_sweep / 4)

      cos0 = Trig.cos(t0); sin0 = Trig.sin(t0)
      cos1 = Trig.cos(t1); sin1 = Trig.sin(t1)

      p0 = local_to_world.(rx * cos0, ry * sin0)
      p3 = local_to_world.(rx * cos1, ry * sin1)
      p1 = local_to_world.(rx * cos0 - k * rx * sin0, ry * sin0 + k * ry * cos0)
      p2 = local_to_world.(rx * cos1 + k * rx * sin1, ry * sin1 - k * ry * cos1)

      Bezier2D.new_cubic(p0, p1, p2, p3)
    end)
  end

  # ---------------------------------------------------------------------------
  # SvgArc
  # ---------------------------------------------------------------------------

  @doc "Create an SvgArc."
  def new_svg_arc(from_pt, to_pt, rx, ry, x_rotation, large_arc, sweep) do
    {:svg_arc, from_pt, to_pt, rx, ry, x_rotation, large_arc, sweep}
  end

  @doc """
  Convert an SvgArc to a CenterArc using the W3C SVG §B.2.4 algorithm.
  Returns `{:ok, center_arc}` or `:degenerate`.
  """
  def to_center_arc({:svg_arc, from_pt, to_pt, rx, ry, x_rotation, large_arc, sweep}) do
    if from_pt == to_pt do
      :degenerate
    else
      rx_abs = abs(rx)
      ry_abs = abs(ry)
      if rx_abs < 1.0e-12 or ry_abs < 1.0e-12 do
        :degenerate
      else
        do_to_center_arc(from_pt, to_pt, rx_abs, ry_abs, x_rotation, large_arc, sweep)
      end
    end
  end

  defp do_to_center_arc({fx, fy}, {tx, ty}, rx, ry, x_rotation, large_arc, sweep) do
    cos_r = Trig.cos(x_rotation)
    sin_r = Trig.sin(x_rotation)

    # Step 1: midpoint in rotated frame
    dx2 = (fx - tx) / 2.0
    dy2 = (fy - ty) / 2.0
    x1p =  cos_r * dx2 + sin_r * dy2
    y1p = -sin_r * dx2 + cos_r * dy2

    # Step 2: scale radii if too small
    lambda_sq = (x1p / rx) * (x1p / rx) + (y1p / ry) * (y1p / ry)
    {rx, ry} =
      if lambda_sq > 1 do
        lam = Trig.sqrt(lambda_sq)
        {rx * lam, ry * lam}
      else
        {rx, ry}
      end

    # Step 3: center in rotated frame
    rx2 = rx * rx; ry2 = ry * ry
    x1p2 = x1p * x1p; y1p2 = y1p * y1p
    num = rx2 * ry2 - rx2 * y1p2 - ry2 * x1p2
    den = rx2 * y1p2 + ry2 * x1p2

    if den < 1.0e-24 do
      :degenerate
    else
      sq_val = if num / den > 0, do: Trig.sqrt(num / den), else: 0.0
      sq = if large_arc == sweep, do: -sq_val, else: sq_val

      cxp =  sq * rx * y1p / ry
      cyp = -sq * ry * x1p / rx

      # Step 4: world frame center
      mx = (fx + tx) / 2.0
      my = (fy + ty) / 2.0
      center_x = cos_r * cxp - sin_r * cyp + mx
      center_y = sin_r * cxp + cos_r * cyp + my

      # Step 5: angles
      ux = (x1p - cxp) / rx
      uy = (y1p - cyp) / ry
      vx = (-x1p - cxp) / rx
      vy = (-y1p - cyp) / ry

      start_angle = Trig.atan2(uy, ux)
      sweep_angle = angle_between(ux, uy, vx, vy)

      sweep_angle =
        cond do
          not sweep and sweep_angle > 0 -> sweep_angle - 2 * Trig.pi()
          sweep and sweep_angle < 0     -> sweep_angle + 2 * Trig.pi()
          true                          -> sweep_angle
        end

      {:ok, new_center_arc({center_x, center_y}, rx, ry, start_angle, sweep_angle, x_rotation)}
    end
  end

  defp angle_between(ux, uy, vx, vy) do
    dot   = ux * vx + uy * vy
    mag_u = Trig.sqrt(ux * ux + uy * uy)
    mag_v = Trig.sqrt(vx * vx + vy * vy)
    if mag_u < 1.0e-12 or mag_v < 1.0e-12 do
      0.0
    else
      cos_a = max(-1.0, min(1.0, dot / (mag_u * mag_v)))
      sin_a = Trig.sqrt(1 - cos_a * cos_a)
      angle = Trig.atan2(sin_a, cos_a)
      if ux * vy - uy * vx < 0, do: -angle, else: angle
    end
  end
end
