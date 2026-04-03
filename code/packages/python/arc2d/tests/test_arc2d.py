"""Tests for arc2d."""
import math
import pytest
from arc2d import CenterArc, SvgArc
from point2d import Point
import trig

EPS = 1e-5
def approx_eq(a, b): return abs(a - b) < EPS
def pt_eq(a, b): return approx_eq(a.x, b.x) and approx_eq(a.y, b.y)


class TestCenterArc:
    def test_quarter_circle_endpoints(self):
        arc = CenterArc(Point.origin(), 1, 1, 0, trig.PI/2, 0)
        assert pt_eq(arc.evaluate(0), Point(1, 0))
        assert pt_eq(arc.evaluate(1), Point(0, 1))

    def test_full_circle_midpoint(self):
        arc = CenterArc(Point.origin(), 2, 2, 0, 2*trig.PI, 0)
        mid = arc.evaluate(0.5)
        assert approx_eq(mid.x, -2) and approx_eq(mid.y, 0)

    def test_tangent_points_upward(self):
        arc = CenterArc(Point.origin(), 1, 1, 0, trig.PI/2, 0)
        t0 = arc.tangent(0)
        assert approx_eq(t0.x, 0) and t0.y > 0

    def test_bounding_box_unit_circle(self):
        arc = CenterArc(Point.origin(), 1, 1, 0, 2*trig.PI, 0)
        bb = arc.bounding_box()
        assert abs(bb.x + 1) < 0.05 and abs(bb.width - 2) < 0.05

    def test_quarter_circle_one_bezier(self):
        arc = CenterArc(Point.origin(), 1, 1, 0, trig.PI/2, 0)
        assert len(arc.to_cubic_beziers()) == 1

    def test_full_circle_four_beziers(self):
        arc = CenterArc(Point.origin(), 1, 1, 0, 2*trig.PI, 0)
        assert len(arc.to_cubic_beziers()) == 4

    def test_bezier_accuracy(self):
        arc = CenterArc(Point.origin(), 1, 1, 0, trig.PI/2, 0)
        b = arc.to_cubic_beziers()[0]
        arc_mid = arc.evaluate(0.5)
        bez_mid = b.evaluate(0.5)
        assert arc_mid.distance(bez_mid) < 0.001

    def test_bezier_continuity(self):
        arc = CenterArc(Point.origin(), 1, 1, 0, 2*trig.PI, 0)
        bz = arc.to_cubic_beziers()
        for i in range(len(bz) - 1):
            assert bz[i].p3.distance(bz[i+1].p0) < 1e-6


class TestSvgArc:
    def test_degenerate_same_endpoints(self):
        arc = SvgArc(Point.origin(), Point.origin(), 1, 1, 0, False, True)
        assert arc.to_center_arc() is None

    def test_degenerate_zero_radius(self):
        arc = SvgArc(Point(0, 0), Point(1, 0), 0, 1, 0, False, True)
        assert arc.to_center_arc() is None

    def test_quarter_circle_center(self):
        arc = SvgArc(Point(1, 0), Point(0, 1), 1, 1, 0, False, True)
        ca = arc.to_center_arc()
        assert ca is not None
        assert approx_eq(ca.center.x, 0) and approx_eq(ca.center.y, 0)

    def test_ccw_positive_sweep(self):
        arc = SvgArc(Point(1, 0), Point(0, 1), 1, 1, 0, False, True)
        assert arc.to_center_arc().sweep_angle > 0

    def test_cw_negative_sweep(self):
        arc = SvgArc(Point(1, 0), Point(0, 1), 1, 1, 0, False, False)
        assert arc.to_center_arc().sweep_angle < 0

    def test_large_arc_bigger_than_pi(self):
        arc = SvgArc(Point(1, 0), Point(-1, 0), 1, 1, 0, True, True)
        ca = arc.to_center_arc()
        assert abs(ca.sweep_angle) > trig.PI - 1e-6

    def test_evaluate_start(self):
        arc = SvgArc(Point(1, 0), Point(0, 1), 1, 1, 0, False, True)
        p = arc.evaluate(0)
        assert p is not None and approx_eq(p.x, 1) and approx_eq(p.y, 0)

    def test_degenerate_returns_empty_beziers(self):
        arc = SvgArc(Point.origin(), Point.origin(), 1, 1, 0, False, True)
        assert arc.to_cubic_beziers() == []

    def test_bounding_box_not_none(self):
        arc = SvgArc(Point(1, 0), Point(-1, 0), 1, 1, 0, True, True)
        assert arc.bounding_box() is not None
