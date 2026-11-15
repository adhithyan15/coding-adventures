"""Property-based testing for dsp_filters using deterministic SplitMix64 PRNG."""

from __future__ import annotations

from dsp_filters import fir, fir_fft, iir


class SplitMix64:
    """Pure-Python deterministic 64-bit PRNG."""

    def __init__(self, seed: int = 0x123456789ABCDEF0) -> None:
        self.state: int = seed & 0xFFFFFFFFFFFFFFFF

    def next_u64(self) -> int:
        self.state = (self.state + 0x9E3779B97F4A7C15) & 0xFFFFFFFFFFFFFFFF
        z = self.state
        z = ((z ^ (z >> 30)) * 0xBF58476D1CE4E5B9) & 0xFFFFFFFFFFFFFFFF
        z = ((z ^ (z >> 27)) * 0x94D049BB133111EB) & 0xFFFFFFFFFFFFFFFF
        return (z ^ (z >> 31)) & 0xFFFFFFFFFFFFFFFF

    def next_f64(self) -> float:
        u = (self.next_u64() >> 11) / (1 << 53)
        return (u * 20.0) - 10.0


def _assert_close(a: list[float], b: list[float], tol: float = 1e-4) -> None:
    assert len(a) == len(b), f"length mismatch: {len(a)} vs {len(b)}"
    for idx, (x, y) in enumerate(zip(a, b, strict=True)):
        scale = max(abs(x), abs(y), 1.0)
        assert abs(x - y) <= scale * tol, f"mismatch at {idx}: {x} vs {y} (tol {tol})"


def test_pbt_fir_commutativity() -> None:
    prng = SplitMix64(0x11223344)
    for n, k in [(5, 3), (8, 6), (15, 7), (20, 10)]:
        x = [prng.next_f64() for _ in range(n)]
        h = [prng.next_f64() for _ in range(k)]
        out1 = fir(x, h)
        out2 = fir(h, x)
        _assert_close(out1, out2, tol=1e-5)


def test_pbt_fir_fft_equivalence() -> None:
    prng = SplitMix64(0x55667788)
    for n, k in [(4, 2), (10, 5), (32, 11), (64, 21)]:
        x = [prng.next_f64() for _ in range(n)]
        h = [prng.next_f64() for _ in range(k)]
        direct_out = fir(x, h)
        fft_out = fir_fft(x, h)
        _assert_close(direct_out, fft_out, tol=1e-4)


def test_pbt_fir_linearity() -> None:
    prng = SplitMix64(0x99AABBCC)
    n = 16
    k = 5
    x = [prng.next_f64() for _ in range(n)]
    y = [prng.next_f64() for _ in range(n)]
    h = [prng.next_f64() for _ in range(k)]
    a_scalar = 1.7
    b_scalar = -2.3

    ax_plus_by = [a_scalar * x[i] + b_scalar * y[i] for i in range(n)]
    lhs = fir(ax_plus_by, h)

    fir_x = fir(x, h)
    fir_y = fir(y, h)
    rhs = [a_scalar * fir_x[i] + b_scalar * fir_y[i] for i in range(len(lhs))]
    _assert_close(lhs, rhs, tol=1e-4)


def test_pbt_iir_linearity() -> None:
    prng = SplitMix64(0xDDEEFF00)
    n = 20
    x = [prng.next_f64() for _ in range(n)]
    y = [prng.next_f64() for _ in range(n)]
    b_coef = [0.2, 0.4, 0.2]
    a_coef = [1.0, -0.6, 0.2]
    a_scalar = 0.8
    b_scalar = 1.5

    ax_plus_by = [a_scalar * x[i] + b_scalar * y[i] for i in range(n)]
    lhs = iir(ax_plus_by, b_coef, a_coef)

    iir_x = iir(x, b_coef, a_coef)
    iir_y = iir(y, b_coef, a_coef)
    rhs = [a_scalar * iir_x[i] + b_scalar * iir_y[i] for i in range(n)]
    _assert_close(lhs, rhs, tol=1e-4)
