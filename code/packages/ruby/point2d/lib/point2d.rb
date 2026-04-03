# frozen_string_literal: true

# =============================================================================
# Point2D — Immutable 2D Point/Vector and Axis-Aligned Bounding Rectangle
# =============================================================================
#
# This module provides two value types:
#
#   Point  — an (x, y) pair used as both a position and a 2D vector.
#   Rect   — an axis-aligned bounding box stored as (x, y, width, height).
#
# All operations are pure: every method returns a new object rather than
# mutating the receiver. This makes Point and Rect safe to share and compose.
#
# ## Why a single module?
#
# Point and Rect are always used together — Rect is defined in terms of
# Point containment, and Rect operations often return Point values
# (e.g. the origin corner). Keeping them in one file avoids circular requires.
#
# =============================================================================

require_relative "../../trig/lib/trig"

module Point2D
  # ===========================================================================
  # Point
  # ===========================================================================
  #
  # A Point is a pair (x, y) representing either a position in the plane or a
  # 2D vector (direction + magnitude). Mathematically these are the same object;
  # we call it Point when we mean a location and use vector language when we mean
  # a displacement.
  #
  # Ruby uses Struct for simple value objects — it gives us attr_readers,
  # equality (==), and a nice #to_s for free.
  Point = Struct.new(:x, :y) do
    # -------------------------------------------------------------------------
    # Arithmetic
    # -------------------------------------------------------------------------

    # add returns a new Point that is self + other.
    # Vector addition: (a+c, b+d) where self=(a,b), other=(c,d)
    def add(other)
      Point.new(x + other.x, y + other.y)
    end

    # subtract returns self - other.
    def subtract(other)
      Point.new(x - other.x, y - other.y)
    end

    # scale multiplies both coordinates by scalar s.
    # This stretches (or shrinks) the vector: (s*x, s*y)
    def scale(s)
      Point.new(x * s, y * s)
    end

    # negate returns the additive inverse: (-x, -y)
    def negate
      Point.new(-x, -y)
    end

    # -------------------------------------------------------------------------
    # Inner products
    # -------------------------------------------------------------------------

    # dot returns the dot product: x1*x2 + y1*y2.
    # Geometrically: |a||b|cos(θ). Zero when vectors are perpendicular.
    def dot(other)
      x * other.x + y * other.y
    end

    # cross returns the 2D "cross product" (the z-component of the 3D cross):
    # x1*y2 - y1*x2.
    # Positive when other is CCW from self, negative when CW.
    def cross(other)
      x * other.y - y * other.x
    end

    # -------------------------------------------------------------------------
    # Magnitudes
    # -------------------------------------------------------------------------

    # magnitude_squared avoids the square root — use this when comparing distances.
    def magnitude_squared
      x * x + y * y
    end

    # magnitude is the Euclidean length: sqrt(x^2 + y^2).
    def magnitude
      Trig.sqrt(magnitude_squared)
    end

    # normalize returns a unit vector (magnitude 1) in the same direction.
    # If magnitude is zero, returns self unchanged (avoids division by zero).
    def normalize
      m = magnitude
      return self if m < 1e-15
      scale(1.0 / m)
    end

    # -------------------------------------------------------------------------
    # Distance
    # -------------------------------------------------------------------------

    # distance_squared to another point: avoids sqrt.
    def distance_squared(other)
      subtract(other).magnitude_squared
    end

    # distance to another point via Euclidean metric.
    def distance(other)
      Trig.sqrt(distance_squared(other))
    end

    # -------------------------------------------------------------------------
    # Interpolation and rotation
    # -------------------------------------------------------------------------

    # lerp linearly interpolates from self to other.
    # At t=0 returns self; at t=1 returns other.
    # Formula: self + t*(other - self) = (1-t)*self + t*other
    def lerp(other, t)
      Point.new(x + t * (other.x - x), y + t * (other.y - y))
    end

    # perpendicular returns a vector 90° CCW: (-y, x).
    def perpendicular
      Point.new(-y, x)
    end

    # angle returns the angle of the vector from the +X axis (radians, CCW).
    # Uses Trig.atan2 so the result is in (-π, π].
    def angle
      Trig.atan2(y, x)
    end
  end

  # ===========================================================================
  # Rect
  # ===========================================================================
  #
  # Rect is an axis-aligned bounding box (AABB) stored as
  # (x, y, width, height) — the same convention as CSS, SVG, and Canvas.
  #
  # Containment uses a half-open interval [x, x+w) × [y, y+h), which means
  # grid cells can be counted without overlap.
  Rect = Struct.new(:x, :y, :width, :height) do
    # contains_point? returns true if pt is inside the half-open rectangle.
    def contains_point?(pt)
      pt.x >= x && pt.x < x + width && pt.y >= y && pt.y < y + height
    end

    # union returns the smallest Rect that contains both self and other.
    def union(other)
      x0 = [x, other.x].min
      y0 = [y, other.y].min
      x1 = [x + width, other.x + other.width].max
      y1 = [y + height, other.y + other.height].max
      Rect.new(x0, y0, x1 - x0, y1 - y0)
    end

    # intersection returns the overlap of self and other, or nil if disjoint.
    def intersection(other)
      x0 = [x, other.x].max
      y0 = [y, other.y].max
      x1 = [x + width, other.x + other.width].min
      y1 = [y + height, other.y + other.height].min
      return nil if x1 <= x0 || y1 <= y0
      Rect.new(x0, y0, x1 - x0, y1 - y0)
    end

    # expand_by returns a new Rect enlarged by margin on all four sides.
    def expand_by(margin)
      Rect.new(x - margin, y - margin, width + 2 * margin, height + 2 * margin)
    end
  end
end
