# frozen_string_literal: true

# =============================================================================
# Bezier2D — Quadratic and Cubic Bezier Curves
# =============================================================================
#
# Bezier curves are smooth polynomial curves defined by control points. They
# are the foundation of vector graphics: every font glyph, SVG path, and CSS
# animation curve is built from Bezier segments.
#
# ## De Casteljau Algorithm
#
# The core insight: to evaluate a Bezier curve at parameter t, repeatedly
# linearly interpolate ("lerp") between consecutive control points until only
# one point remains. This algorithm is numerically stable and elegant.
#
# For a quadratic (degree 2) with points P0, P1, P2:
#   Level 1: Q0 = lerp(P0,P1,t),  Q1 = lerp(P1,P2,t)
#   Level 2: R  = lerp(Q0,Q1,t)   ← the curve point
#
# For a cubic (degree 3) with P0..P3: three levels of lerp.
#
# =============================================================================

require_relative "../../trig/lib/trig"
require_relative "../../point2d/lib/point2d"

module Bezier2D
  include Point2D

  # ===========================================================================
  # QuadraticBezier
  # ===========================================================================
  QuadraticBezier = Struct.new(:p0, :p1, :p2) do
    # eval computes the curve point at t ∈ [0,1] via de Casteljau.
    def eval(t)
      q0 = p0.lerp(p1, t)
      q1 = p1.lerp(p2, t)
      q0.lerp(q1, t)
    end

    # deriv returns the tangent vector at t.
    # Derivative of a quadratic Bezier is a linear Bezier of differences × 2:
    #   B'(t) = 2 * lerp(P1-P0, P2-P1, t)
    def deriv(t)
      d0 = p1.subtract(p0)
      d1 = p2.subtract(p1)
      d0.lerp(d1, t).scale(2)
    end

    # split divides the curve at t into two quadratic segments.
    # De Casteljau at t gives us all the intermediate points we need.
    def split(t)
      q0 = p0.lerp(p1, t)
      q1 = p1.lerp(p2, t)
      m  = q0.lerp(q1, t)
      [QuadraticBezier.new(p0, q0, m), QuadraticBezier.new(m, q1, p2)]
    end

    # polyline adaptively subdivides the curve to within tolerance.
    # If the midpoint of the chord (straight line) is close to the actual
    # curve midpoint, the segment is flat enough to represent as a line.
    def polyline(tolerance)
      chord_mid = p0.lerp(p2, 0.5)
      curve_mid = eval(0.5)
      if chord_mid.distance(curve_mid) <= tolerance
        [p0, p2]
      else
        left, right = split(0.5)
        lpts = left.polyline(tolerance)
        rpts = right.polyline(tolerance)
        lpts + rpts[1..]
      end
    end

    # bbox returns the tight axis-aligned bounding box.
    # For a quadratic, the derivative is linear in t, so extrema occur at most
    # at one interior t per axis (where derivative is zero), plus the endpoints.
    def bbox
      min_x, max_x = [p0.x, p2.x].minmax
      min_y, max_y = [p0.y, p2.y].minmax

      # X extremum: derivative x(t) = 2*(p1.x - p0.x) + 2*(p0.x - 2*p1.x + p2.x)*t = 0
      # → t = (p0.x - p1.x) / (p0.x - 2*p1.x + p2.x)
      dx = p0.x - 2 * p1.x + p2.x
      unless dx.abs < 1e-12
        tx = (p0.x - p1.x) / dx
        if tx > 0 && tx < 1
          px = eval(tx).x
          min_x = [min_x, px].min
          max_x = [max_x, px].max
        end
      end

      dy = p0.y - 2 * p1.y + p2.y
      unless dy.abs < 1e-12
        ty = (p0.y - p1.y) / dy
        if ty > 0 && ty < 1
          py = eval(ty).y
          min_y = [min_y, py].min
          max_y = [max_y, py].max
        end
      end

      Point2D::Rect.new(min_x, min_y, max_x - min_x, max_y - min_y)
    end

    # elevate converts this quadratic to an equivalent cubic.
    # Degree elevation formula: the new control points Q0..Q3 satisfy
    #   Q0 = P0
    #   Q1 = (1/3)*P0 + (2/3)*P1
    #   Q2 = (2/3)*P1 + (1/3)*P2
    #   Q3 = P2
    def elevate
      q1 = p0.scale(1.0 / 3).add(p1.scale(2.0 / 3))
      q2 = p1.scale(2.0 / 3).add(p2.scale(1.0 / 3))
      CubicBezier.new(p0, q1, q2, p2)
    end
  end

  # ===========================================================================
  # CubicBezier
  # ===========================================================================
  CubicBezier = Struct.new(:p0, :p1, :p2, :p3) do
    # eval computes the curve point at t via de Casteljau (3 levels).
    def eval(t)
      p01  = p0.lerp(p1, t)
      p12  = p1.lerp(p2, t)
      p23  = p2.lerp(p3, t)
      p012 = p01.lerp(p12, t)
      p123 = p12.lerp(p23, t)
      p012.lerp(p123, t)
    end

    # deriv returns the tangent vector at t.
    # Cubic derivative is: 3 * quadratic_bezier(P1-P0, P2-P1, P3-P2)(t)
    # Expanded: 3*[(1-t)^2*(P1-P0) + 2*(1-t)*t*(P2-P1) + t^2*(P3-P2)]
    def deriv(t)
      d0 = p1.subtract(p0)
      d1 = p2.subtract(p1)
      d2 = p3.subtract(p2)
      one_t = 1 - t
      r = d0.scale(one_t * one_t)
        .add(d1.scale(2 * one_t * t))
        .add(d2.scale(t * t))
      r.scale(3)
    end

    # split divides the cubic at t into two cubic segments via de Casteljau.
    def split(t)
      p01   = p0.lerp(p1, t)
      p12   = p1.lerp(p2, t)
      p23   = p2.lerp(p3, t)
      p012  = p01.lerp(p12, t)
      p123  = p12.lerp(p23, t)
      p0123 = p012.lerp(p123, t)
      [
        CubicBezier.new(p0, p01, p012, p0123),
        CubicBezier.new(p0123, p123, p23, p3)
      ]
    end

    # polyline adaptively subdivides to within tolerance.
    def polyline(tolerance)
      chord_mid = p0.lerp(p3, 0.5)
      curve_mid = eval(0.5)
      if chord_mid.distance(curve_mid) <= tolerance
        [p0, p3]
      else
        left, right = split(0.5)
        lpts = left.polyline(tolerance)
        rpts = right.polyline(tolerance)
        lpts + rpts[1..]
      end
    end

    # bbox returns the tight axis-aligned bounding box.
    # Extrema are at t where the derivative is zero. The derivative of a cubic
    # is a quadratic in t, so we solve the quadratic equation for each axis.
    def bbox
      min_x, max_x = [p0.x, p3.x].minmax
      min_y, max_y = [p0.y, p3.y].minmax

      extrema(p0.x, p1.x, p2.x, p3.x).each do |tx|
        px = eval(tx).x
        min_x = [min_x, px].min
        max_x = [max_x, px].max
      end
      extrema(p0.y, p1.y, p2.y, p3.y).each do |ty|
        py = eval(ty).y
        min_y = [min_y, py].min
        max_y = [max_y, py].max
      end

      Point2D::Rect.new(min_x, min_y, max_x - min_x, max_y - min_y)
    end

    private

    # extrema finds t in (0,1) where the cubic derivative in one coordinate is zero.
    # The cubic derivative in one coordinate is the quadratic:
    #   a*t^2 + b*t + c = 0
    # where the quadratic coefficients come from expanding the derivative formula.
    def extrema(v0, v1, v2, v3)
      a = -3 * v0 + 9 * v1 - 9 * v2 + 3 * v3
      b =  6 * v0 - 12 * v1 + 6 * v2
      c = -3 * v0 + 3 * v1
      roots = []
      if a.abs < 1e-12
        # Degenerate to linear: b*t + c = 0
        if b.abs > 1e-12
          tx = -c / b
          roots << tx if tx > 0 && tx < 1
        end
      else
        disc = b * b - 4 * a * c
        if disc >= 0
          sq = Trig.sqrt(disc)
          [(-b + sq) / (2 * a), (-b - sq) / (2 * a)].each do |tx|
            roots << tx if tx > 0 && tx < 1
          end
        end
      end
      roots
    end
  end
end
