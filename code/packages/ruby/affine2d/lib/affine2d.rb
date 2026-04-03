# frozen_string_literal: true

# =============================================================================
# Affine2D — 2D Affine Transformation Matrix
# =============================================================================
#
# An affine transformation combines linear transformation (rotation, scale,
# skew) with translation. In 2D we represent it as a 3×3 matrix in homogeneous
# coordinates, but since the last row is always [0 0 1] we only store 6 values:
#
#   [ a  c  e ]
#   [ b  d  f ]
#   [ 0  0  1 ]
#
# This is the same layout as SVG's `matrix(a,b,c,d,e,f)` and the Canvas 2D API.
#
# The mapping is: x' = a*x + c*y + e
#                 y' = b*x + d*y + f
#
# =============================================================================

require_relative "../../trig/lib/trig"
require_relative "../../point2d/lib/point2d"

module Affine2D
  # Affine2D matrix stored as [a, b, c, d, e, f].
  Affine = Struct.new(:a, :b, :c, :d, :e, :f) do
    # -------------------------------------------------------------------------
    # Factory methods
    # -------------------------------------------------------------------------

    # identity returns the do-nothing transform.
    def self.identity
      new(1, 0, 0, 1, 0, 0)
    end

    # translate returns a pure translation by (tx, ty).
    def self.translate(tx, ty)
      new(1, 0, 0, 1, tx, ty)
    end

    # rotate returns a CCW rotation by angle_rad about the origin.
    # Uses the standard rotation matrix:
    #   [ cos  -sin  0 ]
    #   [ sin   cos  0 ]
    #   [ 0     0    1 ]
    def self.rotate(angle_rad)
      c = Trig.cos(angle_rad)
      s = Trig.sin(angle_rad)
      new(c, s, -s, c, 0, 0)
    end

    # rotate_around rotates CCW by angle_rad about pivot point (px, py).
    # Equivalent to: translate(px,py) · rotate(angle) · translate(-px,-py)
    def self.rotate_around(angle_rad, px, py)
      translate(px, py).then(rotate(angle_rad)).then(translate(-px, -py))
    end

    # scale returns a non-uniform scale by (sx, sy).
    def self.scale(sx, sy)
      new(sx, 0, 0, sy, 0, 0)
    end

    # scale_uniform returns a uniform scale by factor s.
    def self.scale_uniform(s)
      scale(s, s)
    end

    # skew_x shears along the x-axis by angle_rad.
    def self.skew_x(angle_rad)
      new(1, 0, Trig.tan(angle_rad), 1, 0, 0)
    end

    # skew_y shears along the y-axis by angle_rad.
    def self.skew_y(angle_rad)
      new(1, Trig.tan(angle_rad), 0, 1, 0, 0)
    end

    # -------------------------------------------------------------------------
    # Composition
    # -------------------------------------------------------------------------

    # then composes self with other: applies self first, then other.
    # Matrix multiplication for 2×3 affine matrices (with implicit [0 0 1] row):
    #   [a1 c1 e1]   [a2 c2 e2]   [a1*a2+c1*b2  a1*c2+c1*d2  a1*e2+c1*f2+e1]
    #   [b1 d1 f1] × [b2 d2 f2] = [b1*a2+d1*b2  b1*c2+d1*d2  b1*e2+d1*f2+f1]
    def then(other)
      Affine.new(
        a * other.a + c * other.b,
        b * other.a + d * other.b,
        a * other.c + c * other.d,
        b * other.c + d * other.d,
        a * other.e + c * other.f + e,
        b * other.e + d * other.f + f
      )
    end

    # -------------------------------------------------------------------------
    # Application
    # -------------------------------------------------------------------------

    # apply_to_point transforms a Point (position): includes translation.
    def apply_to_point(pt)
      Point2D::Point.new(
        a * pt.x + c * pt.y + e,
        b * pt.x + d * pt.y + f
      )
    end

    # apply_to_vector transforms a direction vector: excludes translation.
    def apply_to_vector(v)
      Point2D::Point.new(
        a * v.x + c * v.y,
        b * v.x + d * v.y
      )
    end

    # -------------------------------------------------------------------------
    # Matrix properties
    # -------------------------------------------------------------------------

    # determinant is ad - bc. Non-zero means the transform is invertible.
    def determinant
      a * d - b * c
    end

    # invert returns the inverse matrix, or nil if singular (det ≈ 0).
    # For the 2×3 matrix the inverse is:
    #   a' =  d/det   c' = -c/det   e' = (c*f - d*e) / det
    #   b' = -b/det   d' =  a/det   f' = (b*e - a*f) / det
    def invert
      det = determinant
      return nil if det.abs < 1e-12
      inv = 1.0 / det
      Affine.new(
        d * inv,
        -b * inv,
        -c * inv,
        a * inv,
        (c * f - d * e) * inv,
        (b * e - a * f) * inv
      )
    end

    # is_identity? returns true when the transform is the identity.
    def is_identity?
      a == 1 && b == 0 && c == 0 && d == 1 && e == 0 && f == 0
    end

    # is_translation_only? returns true when only translation is present.
    def is_translation_only?
      a == 1 && b == 0 && c == 0 && d == 1
    end

    # to_array returns [a, b, c, d, e, f] (SVG matrix order).
    def to_array
      [a, b, c, d, e, f]
    end
  end
end
