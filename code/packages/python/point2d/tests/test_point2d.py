"""Tests for the point2d package."""

import math
import pytest
from point2d import Point, Rect

EPS = 1e-9


def approx_eq(a: float, b: float) -> bool:
    return abs(a - b) < EPS


def pt_eq(a: Point, b: Point) -> bool:
    return approx_eq(a.x, b.x) and approx_eq(a.y, b.y)


# ============================================================================
# Point tests
# ============================================================================

class TestPointConstruction:
    def test_origin(self):
        o = Point.origin()
        assert o.x == 0.0
        assert o.y == 0.0

    def test_new(self):
        p = Point(3.0, -5.0)
        assert p.x == 3.0
        assert p.y == -5.0


class TestPointArithmetic:
    def test_add(self):
        a = Point(1.0, 2.0)
        b = Point(3.0, 4.0)
        assert a.add(b) == Point(4.0, 6.0)

    def test_subtract(self):
        a = Point(5.0, 7.0)
        b = Point(2.0, 3.0)
        assert a.subtract(b) == Point(3.0, 4.0)

    def test_scale(self):
        p = Point(3.0, 4.0)
        assert pt_eq(p.scale(2.0), Point(6.0, 8.0))
        assert pt_eq(p.scale(0.0), Point.origin())
        assert pt_eq(p.scale(-1.0), Point(-3.0, -4.0))

    def test_negate(self):
        p = Point(3.0, -4.0)
        assert p.negate() == Point(-3.0, 4.0)


class TestPointVectorOps:
    def test_dot_perpendicular(self):
        x = Point(1.0, 0.0)
        y = Point(0.0, 1.0)
        assert x.dot(y) == 0.0

    def test_dot_parallel(self):
        p = Point(3.0, 0.0)
        q = Point(5.0, 0.0)
        assert p.dot(q) == 15.0

    def test_cross_ccw(self):
        x = Point(1.0, 0.0)
        y = Point(0.0, 1.0)
        assert x.cross(y) == 1.0

    def test_cross_cw(self):
        x = Point(1.0, 0.0)
        y = Point(0.0, 1.0)
        assert y.cross(x) == -1.0

    def test_magnitude_3_4_5(self):
        p = Point(3.0, 4.0)
        assert approx_eq(p.magnitude(), 5.0)

    def test_magnitude_zero(self):
        assert Point.origin().magnitude() == 0.0

    def test_magnitude_squared(self):
        p = Point(3.0, 4.0)
        assert p.magnitude_squared() == 25.0

    def test_normalize_unit(self):
        p = Point(3.0, 4.0)
        n = p.normalize()
        assert approx_eq(n.x, 0.6)
        assert approx_eq(n.y, 0.8)
        assert approx_eq(n.magnitude(), 1.0)

    def test_normalize_zero(self):
        n = Point.origin().normalize()
        assert n == Point.origin()

    def test_distance(self):
        a = Point.origin()
        b = Point(3.0, 4.0)
        assert approx_eq(a.distance(b), 5.0)

    def test_distance_squared(self):
        a = Point.origin()
        b = Point(3.0, 4.0)
        assert a.distance_squared(b) == 25.0


class TestPointInterpolation:
    def test_lerp_endpoints(self):
        a = Point(1.0, 2.0)
        b = Point(5.0, 6.0)
        assert pt_eq(a.lerp(b, 0.0), a)
        assert pt_eq(a.lerp(b, 1.0), b)

    def test_lerp_midpoint(self):
        a = Point(0.0, 0.0)
        b = Point(10.0, 10.0)
        assert pt_eq(a.lerp(b, 0.5), Point(5.0, 5.0))

    def test_perpendicular(self):
        assert pt_eq(Point(1.0, 0.0).perpendicular(), Point(0.0, 1.0))
        assert pt_eq(Point(0.0, 1.0).perpendicular(), Point(-1.0, 0.0))

    def test_perpendicular_twice_is_negate(self):
        p = Point(3.0, 4.0)
        assert pt_eq(p.perpendicular().perpendicular(), p.negate())

    def test_angle_right(self):
        assert approx_eq(Point(1.0, 0.0).angle(), 0.0)

    def test_angle_up(self):
        assert approx_eq(Point(0.0, 1.0).angle(), math.pi / 2)

    def test_angle_left(self):
        assert approx_eq(abs(Point(-1.0, 0.0).angle()), math.pi)

    def test_angle_down(self):
        assert approx_eq(Point(0.0, -1.0).angle(), -math.pi / 2)


# ============================================================================
# Rect tests
# ============================================================================

class TestRectConstruction:
    def test_new(self):
        r = Rect(1.0, 2.0, 10.0, 5.0)
        assert r.x == 1.0
        assert r.y == 2.0
        assert r.width == 10.0
        assert r.height == 5.0

    def test_from_points(self):
        r = Rect.from_points(Point(1.0, 2.0), Point(11.0, 7.0))
        assert r.x == 1.0
        assert r.width == 10.0
        assert r.height == 5.0

    def test_zero(self):
        r = Rect.zero()
        assert r.x == 0.0
        assert r.width == 0.0


class TestRectAccessors:
    def test_min_max_center(self):
        r = Rect(2.0, 3.0, 8.0, 4.0)
        assert pt_eq(r.min_point(), Point(2.0, 3.0))
        assert pt_eq(r.max_point(), Point(10.0, 7.0))
        assert pt_eq(r.center(), Point(6.0, 5.0))


class TestRectPredicates:
    def test_is_empty_zero(self):
        assert Rect.zero().is_empty()

    def test_is_empty_negative(self):
        assert Rect(0.0, 0.0, -1.0, 5.0).is_empty()

    def test_not_empty(self):
        assert not Rect(0.0, 0.0, 5.0, 5.0).is_empty()

    def test_contains_inside(self):
        r = Rect(0.0, 0.0, 10.0, 10.0)
        assert r.contains_point(Point(5.0, 5.0))

    def test_contains_top_left_inclusive(self):
        r = Rect(0.0, 0.0, 10.0, 10.0)
        assert r.contains_point(Point(0.0, 0.0))

    def test_contains_right_exclusive(self):
        r = Rect(0.0, 0.0, 10.0, 10.0)
        assert not r.contains_point(Point(10.0, 5.0))

    def test_contains_bottom_exclusive(self):
        r = Rect(0.0, 0.0, 10.0, 10.0)
        assert not r.contains_point(Point(5.0, 10.0))

    def test_contains_outside(self):
        r = Rect(0.0, 0.0, 10.0, 10.0)
        assert not r.contains_point(Point(-1.0, 5.0))


class TestRectSetOps:
    def test_union_non_overlapping(self):
        a = Rect(0.0, 0.0, 5.0, 5.0)
        b = Rect(10.0, 10.0, 5.0, 5.0)
        u = a.union(b)
        assert approx_eq(u.x, 0.0)
        assert approx_eq(u.width, 15.0)
        assert approx_eq(u.height, 15.0)

    def test_union_with_empty(self):
        a = Rect(1.0, 2.0, 5.0, 5.0)
        u = a.union(Rect.zero())
        assert u == a

    def test_intersection_overlap(self):
        a = Rect(0.0, 0.0, 10.0, 10.0)
        b = Rect(5.0, 5.0, 10.0, 10.0)
        i = a.intersection(b)
        assert i is not None
        assert approx_eq(i.x, 5.0)
        assert approx_eq(i.width, 5.0)

    def test_intersection_no_overlap(self):
        a = Rect(0.0, 0.0, 5.0, 5.0)
        b = Rect(10.0, 10.0, 5.0, 5.0)
        assert a.intersection(b) is None

    def test_intersection_touching_edge_is_none(self):
        a = Rect(0.0, 0.0, 5.0, 5.0)
        b = Rect(5.0, 0.0, 5.0, 5.0)
        assert a.intersection(b) is None

    def test_expand_by_positive(self):
        r = Rect(1.0, 1.0, 8.0, 8.0)
        e = r.expand_by(1.0)
        assert approx_eq(e.x, 0.0)
        assert approx_eq(e.width, 10.0)

    def test_expand_by_negative(self):
        r = Rect(0.0, 0.0, 10.0, 10.0)
        s = r.expand_by(-1.0)
        assert approx_eq(s.x, 1.0)
        assert approx_eq(s.width, 8.0)
