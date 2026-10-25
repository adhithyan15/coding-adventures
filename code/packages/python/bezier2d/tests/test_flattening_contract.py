"""Dynamic conformance tests for the G2D02 safe-flattening fixture."""

import json
import math
from pathlib import Path

import pytest
from bezier2d import CubicBezier, QuadraticBezier
from point2d import Point


FIXTURE_PATH = (
    Path(__file__).resolve().parents[4]
    / "specs"
    / "fixtures"
    / "bezier2d-flattening-v1"
    / "cases.json"
)
CASES = json.loads(FIXTURE_PATH.read_text(encoding="utf-8"))["cases"]


def _curve(case):
    points = [Point(*coordinates) for coordinates in case["control_points"]]
    if case["degree"] == 2:
        return QuadraticBezier(*points)
    return CubicBezier(*points)


@pytest.mark.parametrize("case", CASES, ids=lambda case: case["id"])
def test_neutral_flattening_cases(case):
    curve = _curve(case)
    disposition = case["expected_disposition"]
    if disposition == "invalid-tolerance":
        with pytest.raises(ValueError, match="tolerance"):
            curve.to_polyline(case["tolerance"])
        return
    if case.get("expected_termination") == "budget-error":
        with pytest.raises(RuntimeError, match="budget"):
            curve.to_polyline(case["tolerance"])
        return

    points = curve.to_polyline(case["tolerance"])
    assert points[0] == curve.p0
    assert points[-1] == (curve.p2 if case["degree"] == 2 else curve.p3)
    assert all(math.isfinite(point.x) and math.isfinite(point.y) for point in points)
    assert len(points) == 2 if disposition == "chord" else len(points) > 2


@pytest.mark.parametrize("tolerance", [0.0, -1.0, math.inf, -math.inf, math.nan])
def test_tolerance_must_be_finite_and_positive(tolerance):
    curve = QuadraticBezier(Point(0, 0), Point(0.5, 0), Point(1, 0))
    with pytest.raises(ValueError, match="tolerance"):
        curve.to_polyline(tolerance)


@pytest.mark.parametrize("coordinate", [math.inf, -math.inf, math.nan])
def test_control_points_must_be_finite(coordinate):
    curve = CubicBezier(Point(0, 0), Point(coordinate, 1), Point(0, -1), Point(0, 0))
    with pytest.raises(ValueError, match="control"):
        curve.to_polyline(0.1)


def test_nonfinite_derived_geometry_fails_without_a_result():
    curve = QuadraticBezier(Point(1e308, 0), Point(0, 1), Point(-1e308, 0))
    result = None
    with pytest.raises(RuntimeError, match="finite"):
        result = curve.to_polyline(0.1)
    assert result is None
