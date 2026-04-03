"""
bezier2d — Quadratic and Cubic Bezier Curves
=============================================

Pure polynomial arithmetic on 2D points.
No trig dependency for curve evaluation (only trig.sqrt for bounding box).
"""

from __future__ import annotations

from dataclasses import dataclass
from typing import Optional

import trig as _trig
from point2d import Point, Rect


@dataclass(frozen=True)
class QuadraticBezier:
    """Quadratic Bezier curve with three control points."""

    p0: Point
    p1: Point
    p2: Point

    def evaluate(self, t: float) -> Point:
        """Evaluate at t ∈ [0,1] via de Casteljau."""
        q0 = self.p0.lerp(self.p1, t)
        q1 = self.p1.lerp(self.p2, t)
        return q0.lerp(q1, t)

    def derivative(self, t: float) -> Point:
        """Tangent vector at t: 2*lerp(p1-p0, p2-p1, t)."""
        d0 = self.p1.subtract(self.p0)
        d1 = self.p2.subtract(self.p1)
        return d0.lerp(d1, t).scale(2.0)

    def split(self, t: float) -> tuple["QuadraticBezier", "QuadraticBezier"]:
        """Split at t into (left, right) sub-curves."""
        q0 = self.p0.lerp(self.p1, t)
        q1 = self.p1.lerp(self.p2, t)
        m = q0.lerp(q1, t)
        return QuadraticBezier(self.p0, q0, m), QuadraticBezier(m, q1, self.p2)

    def to_polyline(self, tolerance: float) -> list[Point]:
        """Adaptive polyline approximation within tolerance."""
        chord_mid = self.p0.lerp(self.p2, 0.5)
        curve_mid = self.evaluate(0.5)
        if chord_mid.distance(curve_mid) <= tolerance:
            return [self.p0, self.p2]
        left, right = self.split(0.5)
        pts = left.to_polyline(tolerance)
        pts.extend(right.to_polyline(tolerance)[1:])
        return pts

    def bounding_box(self) -> Rect:
        """Tight axis-aligned bounding box."""
        min_x = min(self.p0.x, self.p2.x)
        max_x = max(self.p0.x, self.p2.x)
        min_y = min(self.p0.y, self.p2.y)
        max_y = max(self.p0.y, self.p2.y)

        denom_x = self.p0.x - 2 * self.p1.x + self.p2.x
        if abs(denom_x) > 1e-12:
            tx = (self.p0.x - self.p1.x) / denom_x
            if 0.0 < tx < 1.0:
                px = self.evaluate(tx)
                min_x, max_x = min(min_x, px.x), max(max_x, px.x)

        denom_y = self.p0.y - 2 * self.p1.y + self.p2.y
        if abs(denom_y) > 1e-12:
            ty = (self.p0.y - self.p1.y) / denom_y
            if 0.0 < ty < 1.0:
                py = self.evaluate(ty)
                min_y, max_y = min(min_y, py.y), max(max_y, py.y)

        return Rect(min_x, min_y, max_x - min_x, max_y - min_y)

    def elevate(self) -> "CubicBezier":
        """Degree elevation: convert to equivalent cubic."""
        q1 = self.p0.scale(1 / 3).add(self.p1.scale(2 / 3))
        q2 = self.p1.scale(2 / 3).add(self.p2.scale(1 / 3))
        return CubicBezier(self.p0, q1, q2, self.p2)


@dataclass(frozen=True)
class CubicBezier:
    """Cubic Bezier curve with four control points."""

    p0: Point
    p1: Point
    p2: Point
    p3: Point

    def evaluate(self, t: float) -> Point:
        """Evaluate at t ∈ [0,1] via de Casteljau (3 levels)."""
        p01 = self.p0.lerp(self.p1, t)
        p12 = self.p1.lerp(self.p2, t)
        p23 = self.p2.lerp(self.p3, t)
        p012 = p01.lerp(p12, t)
        p123 = p12.lerp(p23, t)
        return p012.lerp(p123, t)

    def derivative(self, t: float) -> Point:
        """Tangent vector at t."""
        d0 = self.p1.subtract(self.p0)
        d1 = self.p2.subtract(self.p1)
        d2 = self.p3.subtract(self.p2)
        one_t = 1.0 - t
        r = (d0.scale(one_t * one_t)
             .add(d1.scale(2.0 * one_t * t))
             .add(d2.scale(t * t)))
        return r.scale(3.0)

    def split(self, t: float) -> tuple["CubicBezier", "CubicBezier"]:
        """Split at t via de Casteljau."""
        p01 = self.p0.lerp(self.p1, t)
        p12 = self.p1.lerp(self.p2, t)
        p23 = self.p2.lerp(self.p3, t)
        p012 = p01.lerp(p12, t)
        p123 = p12.lerp(p23, t)
        p0123 = p012.lerp(p123, t)
        return (
            CubicBezier(self.p0, p01, p012, p0123),
            CubicBezier(p0123, p123, p23, self.p3),
        )

    def to_polyline(self, tolerance: float) -> list[Point]:
        """Adaptive polyline approximation."""
        chord_mid = self.p0.lerp(self.p3, 0.5)
        curve_mid = self.evaluate(0.5)
        if chord_mid.distance(curve_mid) <= tolerance:
            return [self.p0, self.p3]
        left, right = self.split(0.5)
        pts = left.to_polyline(tolerance)
        pts.extend(right.to_polyline(tolerance)[1:])
        return pts

    def bounding_box(self) -> Rect:
        """Tight bounding box via derivative root finding."""
        min_x = min(self.p0.x, self.p3.x)
        max_x = max(self.p0.x, self.p3.x)
        min_y = min(self.p0.y, self.p3.y)
        max_y = max(self.p0.y, self.p3.y)

        for t in _extrema(self.p0.x, self.p1.x, self.p2.x, self.p3.x):
            px = self.evaluate(t)
            min_x, max_x = min(min_x, px.x), max(max_x, px.x)
        for t in _extrema(self.p0.y, self.p1.y, self.p2.y, self.p3.y):
            py = self.evaluate(t)
            min_y, max_y = min(min_y, py.y), max(max_y, py.y)

        return Rect(min_x, min_y, max_x - min_x, max_y - min_y)


def _extrema(v0: float, v1: float, v2: float, v3: float) -> list[float]:
    """Find t in (0,1) where the cubic's derivative in this coordinate is zero."""
    a = -3*v0 + 9*v1 - 9*v2 + 3*v3
    b = 6*v0 - 12*v1 + 6*v2
    c = -3*v0 + 3*v1
    roots: list[float] = []
    if abs(a) < 1e-12:
        if abs(b) > 1e-12:
            t = -c / b
            if 0.0 < t < 1.0:
                roots.append(t)
    else:
        disc = b*b - 4*a*c
        if disc >= 0:
            sq = _trig.sqrt(disc)
            for t in [(-b + sq) / (2*a), (-b - sq) / (2*a)]:
                if 0.0 < t < 1.0:
                    roots.append(t)
    return roots
