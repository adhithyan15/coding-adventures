"""Tests for bezier2d."""
import pytest
from bezier2d import QuadraticBezier, CubicBezier
from point2d import Point

EPS = 1e-9
def approx_eq(a, b): return abs(a - b) < EPS
def pt_eq(a, b): return approx_eq(a.x, b.x) and approx_eq(a.y, b.y)


class TestQuadraticBezier:
    q = QuadraticBezier(Point(0, 0), Point(1, 2), Point(2, 0))

    def test_endpoints(self):
        assert pt_eq(self.q.evaluate(0), self.q.p0)
        assert pt_eq(self.q.evaluate(1), self.q.p2)

    def test_midpoint(self):
        mid = self.q.evaluate(0.5)
        assert approx_eq(mid.x, 1.0) and approx_eq(mid.y, 1.0)

    def test_derivative_at_0(self):
        d = self.q.derivative(0)
        assert approx_eq(d.x, 2.0) and approx_eq(d.y, 4.0)

    def test_split_endpoints(self):
        left, right = self.q.split(0.5)
        split_pt = self.q.evaluate(0.5)
        assert pt_eq(left.p2, split_pt) and pt_eq(right.p0, split_pt)
        assert pt_eq(left.p0, self.q.p0) and pt_eq(right.p2, self.q.p2)

    def test_polyline_straight(self):
        q = QuadraticBezier(Point(0, 0), Point(1, 0), Point(2, 0))
        assert len(q.to_polyline(0.1)) == 2

    def test_polyline_endpoints(self):
        pts = self.q.to_polyline(0.1)
        assert pt_eq(pts[0], self.q.p0) and pt_eq(pts[-1], self.q.p2)

    def test_bounding_box_contains_endpoints(self):
        bb = self.q.bounding_box()
        assert bb.x <= 0 and bb.x + bb.width >= 2

    def test_elevate_equivalent(self):
        c = self.q.elevate()
        for t in [0, 0.25, 0.5, 0.75, 1]:
            assert abs(self.q.evaluate(t).x - c.evaluate(t).x) < 1e-9
            assert abs(self.q.evaluate(t).y - c.evaluate(t).y) < 1e-9


class TestCubicBezier:
    c = CubicBezier(Point(0, 0), Point(1, 2), Point(3, 2), Point(4, 0))

    def test_endpoints(self):
        assert pt_eq(self.c.evaluate(0), self.c.p0)
        assert pt_eq(self.c.evaluate(1), self.c.p3)

    def test_symmetric_midpoint(self):
        assert approx_eq(self.c.evaluate(0.5).x, 2.0)

    def test_derivative_straight(self):
        c = CubicBezier(Point(0, 0), Point(1, 0), Point(2, 0), Point(3, 0))
        d = c.derivative(0)
        assert approx_eq(d.x, 3.0) and approx_eq(d.y, 0.0)

    def test_split_endpoints(self):
        left, right = self.c.split(0.5)
        split_pt = self.c.evaluate(0.5)
        assert pt_eq(left.p3, split_pt) and pt_eq(right.p0, split_pt)

    def test_polyline_straight(self):
        c = CubicBezier(Point(0, 0), Point(1, 0), Point(2, 0), Point(3, 0))
        assert len(c.to_polyline(0.1)) == 2

    def test_polyline_curved(self):
        pts = self.c.to_polyline(0.1)
        assert len(pts) > 2
        assert pt_eq(pts[0], self.c.p0) and pt_eq(pts[-1], self.c.p3)

    def test_bounding_box_contains_samples(self):
        bb = self.c.bounding_box()
        for i in range(21):
            p = self.c.evaluate(i / 20)
            assert p.x >= bb.x - 1e-6 and p.x <= bb.x + bb.width + 1e-6
            assert p.y >= bb.y - 1e-6 and p.y <= bb.y + bb.height + 1e-6
