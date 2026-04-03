"""Tests for the affine2d package."""
import math
import pytest
from affine2d import Affine2D
from point2d import Point

EPS = 1e-9

def approx_eq(a, b): return abs(a - b) < EPS
def pt_eq(a, b): return approx_eq(a.x, b.x) and approx_eq(a.y, b.y)
def aff_eq(a, b):
    return all(approx_eq(x, y) for x, y in zip(a.to_array(), b.to_array()))


class TestFactories:
    def test_identity_components(self):
        id_ = Affine2D.identity()
        assert id_.a == 1.0 and id_.d == 1.0
        assert id_.b == 0.0 and id_.c == 0.0
        assert id_.e == 0.0 and id_.f == 0.0

    def test_identity_leaves_point_unchanged(self):
        p = Point(3.0, 4.0)
        assert pt_eq(Affine2D.identity().apply_to_point(p), p)

    def test_translate(self):
        q = Affine2D.translate(5.0, -3.0).apply_to_point(Point(1.0, 2.0))
        assert approx_eq(q.x, 6.0) and approx_eq(q.y, -1.0)

    def test_translate_does_not_affect_vector(self):
        v = Point(1.0, 1.0)
        assert pt_eq(Affine2D.translate(100.0, 200.0).apply_to_vector(v), v)

    def test_rotate_90(self):
        q = Affine2D.rotate(math.pi / 2).apply_to_point(Point(1.0, 0.0))
        assert approx_eq(q.x, 0.0) and approx_eq(q.y, 1.0)

    def test_rotate_360_is_identity(self):
        assert Affine2D.rotate(2 * math.pi).is_identity()

    def test_rotate_around_keeps_center(self):
        center = Point(1.0, 0.0)
        q = Affine2D.rotate_around(center, math.pi / 2).apply_to_point(center)
        assert approx_eq(q.x, 1.0) and approx_eq(q.y, 0.0)

    def test_scale(self):
        q = Affine2D.scale(2.0, 3.0).apply_to_point(Point(1.0, 1.0))
        assert approx_eq(q.x, 2.0) and approx_eq(q.y, 3.0)

    def test_scale_uniform(self):
        q = Affine2D.scale_uniform(5.0).apply_to_point(Point(2.0, 3.0))
        assert approx_eq(q.x, 10.0) and approx_eq(q.y, 15.0)

    def test_skew_x(self):
        q = Affine2D.skew_x(math.pi / 4).apply_to_point(Point(0.0, 1.0))
        assert approx_eq(q.x, 1.0) and approx_eq(q.y, 1.0)

    def test_skew_y(self):
        q = Affine2D.skew_y(math.pi / 4).apply_to_point(Point(1.0, 0.0))
        assert approx_eq(q.x, 1.0) and approx_eq(q.y, 1.0)


class TestComposition:
    def test_multiply_by_identity(self):
        m = Affine2D.translate(3.0, 4.0)
        assert aff_eq(m.multiply(Affine2D.identity()), m)

    def test_two_90_rotations_equal_180(self):
        r90 = Affine2D.rotate(math.pi / 2)
        r180 = Affine2D.rotate(math.pi)
        assert aff_eq(r90.multiply(r90), r180)


class TestDeterminantAndInvert:
    def test_determinant_identity(self):
        assert approx_eq(Affine2D.identity().determinant(), 1.0)

    def test_determinant_scale(self):
        assert approx_eq(Affine2D.scale(2.0, 3.0).determinant(), 6.0)

    def test_invert_gives_identity(self):
        t = Affine2D.translate(3.0, -7.0)
        composed = t.multiply(t.invert())
        assert composed.is_identity()

    def test_invert_singular_returns_none(self):
        singular = Affine2D(0.0, 0.0, 0.0, 0.0, 0.0, 0.0)
        assert singular.invert() is None


class TestPredicates:
    def test_is_identity(self):
        assert Affine2D.identity().is_identity()
        assert not Affine2D.translate(1.0, 0.0).is_identity()

    def test_is_translation_only(self):
        assert Affine2D.identity().is_translation_only()
        assert Affine2D.translate(5.0, 3.0).is_translation_only()
        assert not Affine2D.rotate(0.1).is_translation_only()

    def test_to_array(self):
        m = Affine2D(1.0, 2.0, 3.0, 4.0, 5.0, 6.0)
        assert m.to_array() == (1.0, 2.0, 3.0, 4.0, 5.0, 6.0)
