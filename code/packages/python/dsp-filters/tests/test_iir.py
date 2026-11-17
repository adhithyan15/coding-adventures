"""Unit tests for IIR filtering via Direct-Form-II Transposed."""

from __future__ import annotations

from dsp_filters import iir


def _assert_close(a: list[float], b: list[float], tol: float = 1e-4) -> None:
    assert len(a) == len(b), f"length mismatch: {len(a)} vs {len(b)}"
    for idx, (x, y) in enumerate(zip(a, b, strict=True)):
        scale = max(abs(x), abs(y), 1.0)
        assert abs(x - y) <= scale * tol, f"mismatch at {idx}: {x} vs {y} (tol {tol})"


def test_iir_pure_gain() -> None:
    sig = [1.0, 2.0, 3.0, 4.0]
    out = iir(sig, b=[2.5], a=[1.0])
    _assert_close(out, [2.5, 5.0, 7.5, 10.0])


def test_iir_exponential_decay() -> None:
    # First-order IIR: y[n] = x[n] + 0.5 y[n-1]
    # a = [1.0, -0.5], b = [1.0]
    # Impulse input -> 1.0, 0.5, 0.25, 0.125, 0.0625
    impulse = [1.0, 0.0, 0.0, 0.0, 0.0]
    out = iir(impulse, b=[1.0], a=[1.0, -0.5])
    expected = [1.0, 0.5, 0.25, 0.125, 0.0625]
    _assert_close(out, expected)


def test_iir_non_unity_a0() -> None:
    # Dividing by a[0] = 2.0
    sig = [2.0, 4.0, 6.0]
    out = iir(sig, b=[1.0], a=[2.0])
    _assert_close(out, [1.0, 2.0, 3.0])
