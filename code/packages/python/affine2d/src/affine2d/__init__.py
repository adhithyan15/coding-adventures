"""
affine2d — 2D Affine Transformation Matrix
==========================================

Provides ``Affine2D``: the standard 6-float representation used by SVG,
HTML Canvas, PDF, Cairo, and Core Graphics.

Transform formula:
  x' = a*x + c*y + e
  y' = b*x + d*y + f
"""

from __future__ import annotations

import math
from dataclasses import dataclass
from typing import Optional

import trig as _trig
from point2d import Point


@dataclass(frozen=True)
class Affine2D:
    """
    A 2D affine transformation stored as [a, b, c, d, e, f].

    The SVG/Canvas/PDF convention:
      x' = a*x + c*y + e
      y' = b*x + d*y + f
    """

    a: float
    b: float
    c: float
    d: float
    e: float
    f: float

    # -----------------------------------------------------------------------
    # Factory functions
    # -----------------------------------------------------------------------

    @classmethod
    def identity(cls) -> "Affine2D":
        """The identity transform: [1, 0, 0, 1, 0, 0]."""
        return cls(1.0, 0.0, 0.0, 1.0, 0.0, 0.0)

    @classmethod
    def translate(cls, tx: float, ty: float) -> "Affine2D":
        """Pure translation by (tx, ty)."""
        return cls(1.0, 0.0, 0.0, 1.0, tx, ty)

    @classmethod
    def rotate(cls, angle: float) -> "Affine2D":
        """CCW rotation by angle radians."""
        c = _trig.cos(angle)
        s = _trig.sin(angle)
        return cls(c, s, -s, c, 0.0, 0.0)

    @classmethod
    def rotate_around(cls, center: Point, angle: float) -> "Affine2D":
        """Rotation about center."""
        return (
            cls.translate(-center.x, -center.y)
            .then(cls.rotate(angle))
            .then(cls.translate(center.x, center.y))
        )

    @classmethod
    def scale(cls, sx: float, sy: float) -> "Affine2D":
        """Non-uniform scale."""
        return cls(sx, 0.0, 0.0, sy, 0.0, 0.0)

    @classmethod
    def scale_uniform(cls, s: float) -> "Affine2D":
        """Uniform scale."""
        return cls.scale(s, s)

    @classmethod
    def skew_x(cls, angle: float) -> "Affine2D":
        """Horizontal skew."""
        return cls(1.0, 0.0, _trig.tan(angle), 1.0, 0.0, 0.0)

    @classmethod
    def skew_y(cls, angle: float) -> "Affine2D":
        """Vertical skew."""
        return cls(1.0, _trig.tan(angle), 0.0, 1.0, 0.0, 0.0)

    # -----------------------------------------------------------------------
    # Composition
    # -----------------------------------------------------------------------

    def then(self, next_t: "Affine2D") -> "Affine2D":
        """Apply next_t after self."""
        return next_t.multiply(self)

    def multiply(self, other: "Affine2D") -> "Affine2D":
        """self applied after other: result = self · other."""
        return Affine2D(
            self.a * other.a + self.c * other.b,
            self.b * other.a + self.d * other.b,
            self.a * other.c + self.c * other.d,
            self.b * other.c + self.d * other.d,
            self.a * other.e + self.c * other.f + self.e,
            self.b * other.e + self.d * other.f + self.f,
        )

    # -----------------------------------------------------------------------
    # Application
    # -----------------------------------------------------------------------

    def apply_to_point(self, p: Point) -> Point:
        """Apply transform to a point (including translation)."""
        return Point(
            self.a * p.x + self.c * p.y + self.e,
            self.b * p.x + self.d * p.y + self.f,
        )

    def apply_to_vector(self, v: Point) -> Point:
        """Apply transform to a vector (ignoring translation)."""
        return Point(
            self.a * v.x + self.c * v.y,
            self.b * v.x + self.d * v.y,
        )

    # -----------------------------------------------------------------------
    # Properties
    # -----------------------------------------------------------------------

    def determinant(self) -> float:
        """Determinant of the 2×2 linear part: a*d - b*c."""
        return self.a * self.d - self.b * self.c

    def invert(self) -> Optional["Affine2D"]:
        """Inverse, or None if singular."""
        det = self.determinant()
        if abs(det) < 1e-12:
            return None
        return Affine2D(
            self.d / det,
            -self.b / det,
            -self.c / det,
            self.a / det,
            (self.c * self.f - self.d * self.e) / det,
            (self.b * self.e - self.a * self.f) / det,
        )

    def is_identity(self) -> bool:
        """True if approximately identity (within 1e-10)."""
        eps = 1e-10
        return (
            abs(self.a - 1.0) < eps and abs(self.b) < eps
            and abs(self.c) < eps and abs(self.d - 1.0) < eps
            and abs(self.e) < eps and abs(self.f) < eps
        )

    def is_translation_only(self) -> bool:
        """True if a≈1, b≈0, c≈0, d≈1 (pure translation)."""
        eps = 1e-10
        return (
            abs(self.a - 1.0) < eps and abs(self.b) < eps
            and abs(self.c) < eps and abs(self.d - 1.0) < eps
        )

    def to_array(self) -> tuple[float, float, float, float, float, float]:
        """Return the six components as a tuple [a, b, c, d, e, f]."""
        return (self.a, self.b, self.c, self.d, self.e, self.f)
