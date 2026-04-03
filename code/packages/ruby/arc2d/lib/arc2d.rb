# frozen_string_literal: true

# =============================================================================
# Arc2D — Elliptical Arc (Center Form and SVG Endpoint Form)
# =============================================================================
#
# An elliptical arc is a portion of an ellipse. We support two representations:
#
#   CenterArc — defined by center, radii, rotation, start angle, sweep angle.
#                Natural for rendering.
#
#   SvgArc    — defined by two endpoints + flags (the SVG "A" path command).
#                Natural for parsing SVG paths.
#
# Conversion from SvgArc to CenterArc uses the W3C SVG Specification §B.2.4
# algorithm (endpoint-to-center parameterization).
#
# =============================================================================

require_relative "../../trig/lib/trig"
require_relative "../../point2d/lib/point2d"
require_relative "../../bezier2d/lib/bezier2d"

module Arc2D
  # ===========================================================================
  # CenterArc
  # ===========================================================================
  CenterArc = Struct.new(:center, :rx, :ry, :start_angle, :sweep_angle, :x_rotation) do
    # eval returns the arc point at parameter t ∈ [0,1].
    # θ = start_angle + t * sweep_angle
    # local: (rx*cos(θ), ry*sin(θ)) then rotate by x_rotation, translate by center.
    def eval(t)
      theta = start_angle + t * sweep_angle
      cos_t = Trig.cos(theta)
      sin_t = Trig.sin(theta)
      lx = rx * cos_t
      ly = ry * sin_t
      cos_r = Trig.cos(x_rotation)
      sin_r = Trig.sin(x_rotation)
      Point2D::Point.new(
        center.x + cos_r * lx - sin_r * ly,
        center.y + sin_r * lx + cos_r * ly
      )
    end

    # tangent returns the unnormalized tangent direction at t.
    # d/dt of eval(t), chain rule through the rotation matrix.
    def tangent(t)
      theta = start_angle + t * sweep_angle
      cos_t = Trig.cos(theta)
      sin_t = Trig.sin(theta)
      cos_r = Trig.cos(x_rotation)
      sin_r = Trig.sin(x_rotation)
      # d/dtheta: (-rx*sin_t, ry*cos_t) rotated by x_rotation, times sweep_angle
      dlx = -rx * sin_t
      dly =  ry * cos_t
      Point2D::Point.new(
        sweep_angle * (cos_r * dlx - sin_r * dly),
        sweep_angle * (sin_r * dlx + cos_r * dly)
      )
    end

    # bbox returns an approximate bounding box via 100-point sampling.
    def bbox
      p0 = eval(0)
      min_x, max_x = p0.x, p0.x
      min_y, max_y = p0.y, p0.y
      101.times do |i|
        pt = eval(i / 100.0)
        min_x = [min_x, pt.x].min
        max_x = [max_x, pt.x].max
        min_y = [min_y, pt.y].min
        max_y = [max_y, pt.y].max
      end
      Point2D::Rect.new(min_x, min_y, max_x - min_x, max_y - min_y)
    end

    # to_cubic_beziers approximates the arc as a sequence of cubic Bezier curves.
    # Each segment spans at most π/2. The approximation uses:
    #   k = (4/3) * tan(segment_sweep / 4)
    # to place the control points at the correct distance along the tangents.
    def to_cubic_beziers
      half_pi = Trig::PI / 2
      n_seg = (sweep_angle.abs / half_pi).ceil
      n_seg = 1 if n_seg < 1
      seg_sweep = sweep_angle / n_seg.to_f
      cos_r = Trig.cos(x_rotation)
      sin_r = Trig.sin(x_rotation)

      local_to_world = lambda do |lx, ly|
        Point2D::Point.new(
          center.x + cos_r * lx - sin_r * ly,
          center.y + sin_r * lx + cos_r * ly
        )
      end

      (0...n_seg).map do |i|
        t0 = start_angle + i * seg_sweep
        t1 = t0 + seg_sweep
        k  = (4.0 / 3.0) * Trig.tan(seg_sweep / 4)

        cos0, sin0 = Trig.cos(t0), Trig.sin(t0)
        cos1, sin1 = Trig.cos(t1), Trig.sin(t1)

        # Endpoints in local ellipse space
        p0l = [rx * cos0, ry * sin0]
        p3l = [rx * cos1, ry * sin1]

        # Control points: tangent at t0 scaled by k
        p1l = [p0l[0] - k * rx * sin0, p0l[1] + k * ry * cos0]
        p2l = [p3l[0] + k * rx * sin1, p3l[1] - k * ry * cos1]

        Bezier2D::CubicBezier.new(
          local_to_world.call(*p0l),
          local_to_world.call(*p1l),
          local_to_world.call(*p2l),
          local_to_world.call(*p3l)
        )
      end
    end
  end

  # ===========================================================================
  # SvgArc
  # ===========================================================================
  #
  # Represents the SVG endpoint arc command: A rx ry x-rotation large-arc sweep x y
  SvgArc = Struct.new(:from_pt, :to_pt, :rx, :ry, :x_rotation, :large_arc, :sweep) do
    # to_center_arc converts this SvgArc to a CenterArc using the W3C algorithm.
    # Returns nil if degenerate (same endpoints or zero radii).
    def to_center_arc
      # Degenerate cases
      return nil if from_pt == to_pt
      rx_abs = rx.abs
      ry_abs = ry.abs
      return nil if rx_abs < 1e-12 || ry_abs < 1e-12

      cos_r = Trig.cos(x_rotation)
      sin_r = Trig.sin(x_rotation)

      # Step 1: midpoint in rotated frame
      dx2 = (from_pt.x - to_pt.x) / 2.0
      dy2 = (from_pt.y - to_pt.y) / 2.0
      x1p =  cos_r * dx2 + sin_r * dy2
      y1p = -sin_r * dx2 + cos_r * dy2

      # Step 2: scale radii if too small
      lambda_sq = (x1p / rx_abs)**2 + (y1p / ry_abs)**2
      if lambda_sq > 1
        lam = Trig.sqrt(lambda_sq)
        rx_abs *= lam
        ry_abs *= lam
      end

      # Step 3: center in rotated frame
      rx2, ry2 = rx_abs * rx_abs, ry_abs * ry_abs
      x1p2, y1p2 = x1p * x1p, y1p * y1p
      num = rx2 * ry2 - rx2 * y1p2 - ry2 * x1p2
      den = rx2 * y1p2 + ry2 * x1p2
      return nil if den.abs < 1e-24

      sq = num / den > 0 ? Trig.sqrt(num / den) : 0.0
      # Sign: large_arc == sweep → negative factor
      sq = -sq if large_arc == sweep

      cxp =  sq * rx_abs * y1p / ry_abs
      cyp = -sq * ry_abs * x1p / rx_abs

      # Step 4: center in world frame
      mx = (from_pt.x + to_pt.x) / 2.0
      my = (from_pt.y + to_pt.y) / 2.0
      cx = cos_r * cxp - sin_r * cyp + mx
      cy = sin_r * cxp + cos_r * cyp + my

      # Step 5: start and sweep angles
      ux = (x1p - cxp) / rx_abs
      uy = (y1p - cyp) / ry_abs
      vx = (-x1p - cxp) / rx_abs
      vy = (-y1p - cyp) / ry_abs

      start_angle = Trig.atan2(uy, ux)
      sweep_angle = angle_between(ux, uy, vx, vy)

      if !sweep && sweep_angle > 0
        sweep_angle -= Trig::TWO_PI
      elsif sweep && sweep_angle < 0
        sweep_angle += Trig::TWO_PI
      end

      CenterArc.new(
        Point2D::Point.new(cx, cy),
        rx_abs, ry_abs,
        start_angle, sweep_angle,
        x_rotation
      )
    end

    private

    # angle_between returns the signed angle from (ux,uy) to (vx,vy).
    # Uses acos via atan2(sqrt(1-c^2), c) to stay within our trig module.
    def angle_between(ux, uy, vx, vy)
      dot  = ux * vx + uy * vy
      mag_u = Trig.sqrt(ux * ux + uy * uy)
      mag_v = Trig.sqrt(vx * vx + vy * vy)
      return 0 if mag_u < 1e-12 || mag_v < 1e-12

      cos_a = (dot / (mag_u * mag_v)).clamp(-1.0, 1.0)
      sin_a = Trig.sqrt(1 - cos_a * cos_a)
      angle = Trig.atan2(sin_a, cos_a)
      angle = -angle if ux * vy - uy * vx < 0
      angle
    end
  end
end
