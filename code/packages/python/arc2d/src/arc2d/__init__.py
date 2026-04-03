"""
arc2d — Elliptical Arcs
=======================

Two arc parameterizations and conversion between them.

- ``CenterArc``: center, rx, ry, start_angle, sweep_angle, x_rotation
- ``SvgArc``: SVG endpoint form (from, to, rx, ry, x_rotation, large_arc, sweep)
"""

from __future__ import annotations

import math
from dataclasses import dataclass
from typing import Optional

import trig as _trig
from point2d import Point, Rect
from bezier2d import CubicBezier


@dataclass(frozen=True)
class CenterArc:
    """Elliptical arc in center form."""

    center: Point
    rx: float
    ry: float
    start_angle: float
    sweep_angle: float
    x_rotation: float

    def evaluate(self, t: float) -> Point:
        """Evaluate at t ∈ [0,1]."""
        angle = self.start_angle + t * self.sweep_angle
        xp = self.rx * _trig.cos(angle)
        yp = self.ry * _trig.sin(angle)
        cos_r = _trig.cos(self.x_rotation)
        sin_r = _trig.sin(self.x_rotation)
        return Point(
            cos_r * xp - sin_r * yp + self.center.x,
            sin_r * xp + cos_r * yp + self.center.y,
        )

    def tangent(self, t: float) -> Point:
        """Tangent vector at t (not normalized)."""
        angle = self.start_angle + t * self.sweep_angle
        dxp = -self.rx * _trig.sin(angle) * self.sweep_angle
        dyp = self.ry * _trig.cos(angle) * self.sweep_angle
        cos_r = _trig.cos(self.x_rotation)
        sin_r = _trig.sin(self.x_rotation)
        return Point(
            cos_r * dxp - sin_r * dyp,
            sin_r * dxp + cos_r * dyp,
        )

    def bounding_box(self) -> Rect:
        """Bounding box by sampling 100 points."""
        n = 100
        min_x = min_y = float("inf")
        max_x = max_y = float("-inf")
        for i in range(n + 1):
            p = self.evaluate(i / n)
            min_x, max_x = min(min_x, p.x), max(max_x, p.x)
            min_y, max_y = min(min_y, p.y), max(max_y, p.y)
        return Rect(min_x, min_y, max_x - min_x, max_y - min_y)

    def to_cubic_beziers(self) -> list[CubicBezier]:
        """Approximate with cubic Bezier segments (≤90° each)."""
        max_seg = _trig.PI / 2
        n_segs = max(1, math.ceil(abs(self.sweep_angle) / max_seg))
        seg_sweep = self.sweep_angle / n_segs
        cos_r = _trig.cos(self.x_rotation)
        sin_r = _trig.sin(self.x_rotation)
        k = (4.0 / 3.0) * _trig.tan(seg_sweep / 4.0)

        beziers: list[CubicBezier] = []
        for i in range(n_segs):
            alpha = self.start_angle + i * seg_sweep
            beta = alpha + seg_sweep
            cos_a, sin_a = _trig.cos(alpha), _trig.sin(alpha)
            cos_b, sin_b = _trig.cos(beta), _trig.sin(beta)

            p0l = (self.rx * cos_a, self.ry * sin_a)
            p3l = (self.rx * cos_b, self.ry * sin_b)
            p1l = (p0l[0] + k * (-self.rx * sin_a), p0l[1] + k * (self.ry * cos_a))
            p2l = (p3l[0] - k * (-self.rx * sin_b), p3l[1] - k * (self.ry * cos_b))

            def rt(lx: float, ly: float) -> Point:
                return Point(
                    cos_r * lx - sin_r * ly + self.center.x,
                    sin_r * lx + cos_r * ly + self.center.y,
                )

            beziers.append(CubicBezier(
                rt(*p0l), rt(*p1l), rt(*p2l), rt(*p3l),
            ))
        return beziers


@dataclass(frozen=True)
class SvgArc:
    """Elliptical arc in SVG endpoint form."""

    from_pt: Point
    to_pt: Point
    rx: float
    ry: float
    x_rotation: float
    large_arc: bool
    sweep: bool

    def to_center_arc(self) -> Optional[CenterArc]:
        """Convert to center form using the W3C SVG algorithm."""
        if (abs(self.from_pt.x - self.to_pt.x) < 1e-12
                and abs(self.from_pt.y - self.to_pt.y) < 1e-12):
            return None
        if abs(self.rx) < 1e-12 or abs(self.ry) < 1e-12:
            return None

        cos_r = _trig.cos(self.x_rotation)
        sin_r = _trig.sin(self.x_rotation)

        dx = (self.from_pt.x - self.to_pt.x) / 2.0
        dy = (self.from_pt.y - self.to_pt.y) / 2.0
        x1p = cos_r * dx + sin_r * dy
        y1p = -sin_r * dx + cos_r * dy

        rx, ry = abs(self.rx), abs(self.ry)
        lam = (x1p / rx) ** 2 + (y1p / ry) ** 2
        if lam > 1.0:
            sq_lam = _trig.sqrt(lam)
            rx *= sq_lam
            ry *= sq_lam

        rx2, ry2 = rx * rx, ry * ry
        x1p2, y1p2 = x1p * x1p, y1p * y1p
        num = rx2 * ry2 - rx2 * y1p2 - ry2 * x1p2
        den = rx2 * y1p2 + ry2 * x1p2

        sq = 0.0 if abs(den) < 1e-12 else _trig.sqrt(max(0.0, num / den))
        sign = -1.0 if (self.large_arc == self.sweep) else 1.0

        cxp = sign * sq * (rx * y1p / ry)
        cyp = sign * sq * -(ry * x1p / rx)

        mid_x = (self.from_pt.x + self.to_pt.x) / 2.0
        mid_y = (self.from_pt.y + self.to_pt.y) / 2.0
        cx = cos_r * cxp - sin_r * cyp + mid_x
        cy = sin_r * cxp + cos_r * cyp + mid_y

        ux = (x1p - cxp) / rx
        uy = (y1p - cyp) / ry
        vx = (-x1p - cxp) / rx
        vy = (-y1p - cyp) / ry

        start_angle = _angle_between(1.0, 0.0, ux, uy)
        sweep_angle = _angle_between(ux, uy, vx, vy)

        if not self.sweep and sweep_angle > 0:
            sweep_angle -= 2.0 * _trig.PI
        if self.sweep and sweep_angle < 0:
            sweep_angle += 2.0 * _trig.PI

        return CenterArc(Point(cx, cy), rx, ry, start_angle, sweep_angle, self.x_rotation)

    def to_cubic_beziers(self) -> list[CubicBezier]:
        ca = self.to_center_arc()
        return ca.to_cubic_beziers() if ca is not None else []

    def evaluate(self, t: float) -> Optional[Point]:
        ca = self.to_center_arc()
        return ca.evaluate(t) if ca is not None else None

    def bounding_box(self) -> Optional[Rect]:
        ca = self.to_center_arc()
        return ca.bounding_box() if ca is not None else None


def _angle_between(ux: float, uy: float, vx: float, vy: float) -> float:
    """Signed angle from (ux,uy) to (vx,vy)."""
    return _trig.atan2(ux * vy - uy * vx, ux * vx + uy * vy)
