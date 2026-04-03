"""
point2d — 2D Point/Vector and Axis-Aligned Bounding Box
=========================================================

Two fundamental types for 2D geometry:

- ``Point`` — a 2D position *and* a 2D vector. A position (where is
  something?) and a direction+magnitude (how far and which way?) are both
  described by two floats (x, y). Treating them as the same type eliminates
  entire classes of representation-mismatch bugs.

- ``Rect`` — an axis-aligned bounding box (AABB), given by an origin corner
  (x, y) and size (width, height). Used everywhere for hit-testing, clipping,
  and conservative intersection tests.

All operations produce **new** values — immutable/frozen dataclass semantics.

Dependencies
------------
The ``angle()`` method calls ``trig.atan2`` from the PHY00 ``trig`` package.
No other trig function is needed here.
"""

from __future__ import annotations

import math
from dataclasses import dataclass
from typing import Optional

import trig as _trig


# ============================================================================
# Point
# ============================================================================

@dataclass(frozen=True)
class Point:
    """
    A 2D point (position) and 2D vector (direction + magnitude).

    The two interpretations share the same underlying data: a pair of floats
    (x, y). Which interpretation applies depends on context.

    All methods return new Point values — this class is frozen (immutable).
    """

    x: float
    """The horizontal coordinate."""
    y: float
    """The vertical coordinate."""

    # -----------------------------------------------------------------------
    # Construction helpers
    # -----------------------------------------------------------------------

    @classmethod
    def origin(cls) -> "Point":
        """The point at the origin (0, 0) — the additive identity."""
        return cls(0.0, 0.0)

    # -----------------------------------------------------------------------
    # Arithmetic
    # -----------------------------------------------------------------------

    def add(self, other: "Point") -> "Point":
        """Element-wise addition: (x1+x2, y1+y2)."""
        return Point(self.x + other.x, self.y + other.y)

    def subtract(self, other: "Point") -> "Point":
        """Element-wise subtraction: (x1-x2, y1-y2)."""
        return Point(self.x - other.x, self.y - other.y)

    def scale(self, s: float) -> "Point":
        """Scalar multiplication: (s*x, s*y)."""
        return Point(self.x * s, self.y * s)

    def negate(self) -> "Point":
        """Additive inverse: (-x, -y). Same as scale(-1)."""
        return Point(-self.x, -self.y)

    # -----------------------------------------------------------------------
    # Vector operations
    # -----------------------------------------------------------------------

    def dot(self, other: "Point") -> float:
        """
        Dot product: x1*x2 + y1*y2.

        Encodes the angle θ between two vectors: u·v = |u||v|cos(θ).
        Zero → perpendicular. Positive → same direction. Negative → opposite.
        """
        return self.x * other.x + self.y * other.y

    def cross(self, other: "Point") -> float:
        """
        2D cross product (scalar): x1*y2 - y1*x2.

        Positive → other is to the LEFT of self (CCW turn).
        Negative → other is to the RIGHT of self (CW turn).
        Zero → collinear.
        """
        return self.x * other.y - self.y * other.x

    def magnitude(self) -> float:
        """
        Euclidean length: sqrt(x²+y²).

        Uses trig.sqrt from PHY00. Prefer magnitude_squared() for comparisons.
        """
        return _trig.sqrt(self.x * self.x + self.y * self.y)

    def magnitude_squared(self) -> float:
        """
        Squared magnitude: x²+y². No square root.

        Cheaper than magnitude(). Use for distance comparisons.
        """
        return self.x * self.x + self.y * self.y

    def normalize(self) -> "Point":
        """
        Unit vector in the same direction.

        Returns origin if the magnitude is zero.
        """
        m = self.magnitude()
        if m < 1e-12:
            # Zero vector has no direction; return origin by convention.
            return Point.origin()
        return Point(self.x / m, self.y / m)

    def distance(self, other: "Point") -> float:
        """Euclidean distance to another point."""
        return self.subtract(other).magnitude()

    def distance_squared(self, other: "Point") -> float:
        """Squared distance to another point. No sqrt."""
        return self.subtract(other).magnitude_squared()

    # -----------------------------------------------------------------------
    # Interpolation and direction
    # -----------------------------------------------------------------------

    def lerp(self, other: "Point", t: float) -> "Point":
        """
        Linear interpolation: self + t*(other-self).

        t=0 → self; t=1 → other; t=0.5 → midpoint.
        """
        dx = other.x - self.x
        dy = other.y - self.y
        return Point(self.x + t * dx, self.y + t * dy)

    def perpendicular(self) -> "Point":
        """
        Rotate 90° counterclockwise: (-y, x).

        Same magnitude as self. Calling twice gives negate().
        """
        return Point(-self.y, self.x)

    def angle(self) -> float:
        """
        Direction angle in radians: atan2(y, x).

        Counterclockwise from positive X axis. Result in (-π, π].
        Always calls trig.atan2 from PHY00.
        """
        return _trig.atan2(self.y, self.x)


# ============================================================================
# Rect
# ============================================================================

@dataclass(frozen=True)
class Rect:
    """
    An axis-aligned bounding box (AABB).

    The ``x`` and ``y`` fields give the top-left corner; ``width`` and
    ``height`` give the extent. All are floats.

    Coordinate convention: Y increases downward (screen space), matching SVG,
    HTML Canvas, and Core Graphics.
    """

    x: float
    """X coordinate of the top-left corner."""
    y: float
    """Y coordinate of the top-left corner."""
    width: float
    """Width (extent in X direction)."""
    height: float
    """Height (extent in Y direction)."""

    # -----------------------------------------------------------------------
    # Construction helpers
    # -----------------------------------------------------------------------

    @classmethod
    def from_points(cls, min_pt: Point, max_pt: Point) -> "Rect":
        """Construct from two corner points: min (top-left), max (bottom-right)."""
        return cls(
            min_pt.x, min_pt.y,
            max_pt.x - min_pt.x,
            max_pt.y - min_pt.y,
        )

    @classmethod
    def zero(cls) -> "Rect":
        """The empty rect at the origin: {0, 0, 0, 0}."""
        return cls(0.0, 0.0, 0.0, 0.0)

    # -----------------------------------------------------------------------
    # Corner accessors
    # -----------------------------------------------------------------------

    def min_point(self) -> Point:
        """Top-left corner: Point(x, y)."""
        return Point(self.x, self.y)

    def max_point(self) -> Point:
        """Bottom-right corner: Point(x+width, y+height)."""
        return Point(self.x + self.width, self.y + self.height)

    def center(self) -> Point:
        """Center point: Point(x+width/2, y+height/2)."""
        return Point(self.x + self.width / 2.0, self.y + self.height / 2.0)

    # -----------------------------------------------------------------------
    # Geometric predicates
    # -----------------------------------------------------------------------

    def is_empty(self) -> bool:
        """True if width ≤ 0 or height ≤ 0 (zero-area rect)."""
        return self.width <= 0.0 or self.height <= 0.0

    def contains_point(self, p: Point) -> bool:
        """
        True if p is inside this rect.

        Half-open interval [x, x+width) × [y, y+height): the top-left edge
        is inclusive, the bottom-right is exclusive.
        """
        return (
            self.x <= p.x < self.x + self.width
            and self.y <= p.y < self.y + self.height
        )

    # -----------------------------------------------------------------------
    # Set operations
    # -----------------------------------------------------------------------

    def union(self, other: "Rect") -> "Rect":
        """
        Smallest rect containing both self and other.

        If either is empty, returns the other.
        """
        if self.is_empty():
            return other
        if other.is_empty():
            return self
        min_x = min(self.x, other.x)
        min_y = min(self.y, other.y)
        max_x = max(self.x + self.width, other.x + other.width)
        max_y = max(self.y + self.height, other.y + other.height)
        return Rect(min_x, min_y, max_x - min_x, max_y - min_y)

    def intersection(self, other: "Rect") -> Optional["Rect"]:
        """
        The overlap region of self and other, or None if no overlap.

        Returns None if the overlap would have zero or negative area.
        """
        ix = max(self.x, other.x)
        iy = max(self.y, other.y)
        iw = min(self.x + self.width, other.x + other.width) - ix
        ih = min(self.y + self.height, other.y + other.height) - iy
        if iw <= 0.0 or ih <= 0.0:
            return None
        return Rect(ix, iy, iw, ih)

    def expand_by(self, amount: float) -> "Rect":
        """
        Grow all four edges outward by amount.

        Origin shifts by (-amount, -amount); dimensions grow by 2*amount each.
        Negative amount shrinks the rect.
        """
        return Rect(
            self.x - amount,
            self.y - amount,
            self.width + 2.0 * amount,
            self.height + 2.0 * amount,
        )
