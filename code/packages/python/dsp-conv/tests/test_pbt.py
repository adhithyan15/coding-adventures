"""Property-based testing for dsp_conv using deterministic SplitMix64 PRNG."""

from __future__ import annotations

from dsp_conv import (
    BoundaryMode,
    ConvMode,
    conv2d,
    convolve,
    fft_convolve,
    overlap_add,
    sep_conv2d,
)

try:
    from coding_adventures_test_bootstrap.prng import SplitMix64
except ImportError:
    class SplitMix64:  # type: ignore[no-redef]
        def __init__(self, seed: int = 0x123456789ABCDEF0) -> None:
            self.state: int = seed & 0xFFFFFFFFFFFFFFFF

        def next_u64(self) -> int:
            self.state = (self.state + 0x9E3779B97F4A7C15) & 0xFFFFFFFFFFFFFFFF
            z = self.state
            z = ((z ^ (z >> 30)) * 0xBF58476D1CE4E5B9) & 0xFFFFFFFFFFFFFFFF
            z = ((z ^ (z >> 27)) * 0x94D049BB133111EB) & 0xFFFFFFFFFFFFFFFF
            return (z ^ (z >> 31)) & 0xFFFFFFFFFFFFFFFF

        def next_float(self) -> float:
            return (self.next_u64() >> 11) * (1.0 / (1 << 53))


def _next_f64(prng: SplitMix64) -> float:
    return (prng.next_float() * 20.0) - 10.0


def _assert_close(a: list[float], b: list[float], tol: float = 1e-4) -> None:
    assert len(a) == len(b), f"length mismatch: {len(a)} vs {len(b)}"
    for idx, (x, y) in enumerate(zip(a, b, strict=True)):
        scale = max(abs(x), abs(y), 1.0)
        assert abs(x - y) <= scale * tol, f"mismatch at {idx}: {x} vs {y} (tol {tol})"


def test_pbt_convolve_commutativity() -> None:
    prng = SplitMix64(0x11223344)
    for n, k in [(6, 4), (10, 7), (16, 9), (25, 12)]:
        x = [_next_f64(prng) for _ in range(n)]
        h = [_next_f64(prng) for _ in range(k)]
        out1 = convolve(x, h, mode=ConvMode.FULL)
        out2 = convolve(h, x, mode=ConvMode.FULL)
        _assert_close(out1, out2, tol=1e-5)


def test_pbt_convolve_linearity() -> None:
    prng = SplitMix64(0x55667788)
    n, k = 16, 5
    x = [_next_f64(prng) for _ in range(n)]
    y = [_next_f64(prng) for _ in range(n)]
    h = [_next_f64(prng) for _ in range(k)]
    a_scalar = 2.1
    b_scalar = -1.4

    ax_plus_by = [a_scalar * x[i] + b_scalar * y[i] for i in range(n)]
    lhs = convolve(ax_plus_by, h, mode=ConvMode.FULL)

    conv_x = convolve(x, h, mode=ConvMode.FULL)
    conv_y = convolve(y, h, mode=ConvMode.FULL)
    rhs = [a_scalar * conv_x[i] + b_scalar * conv_y[i] for i in range(len(lhs))]
    _assert_close(lhs, rhs, tol=1e-4)


def test_pbt_fft_convolve_equivalence() -> None:
    prng = SplitMix64(0x99AABBCC)
    for n, k in [(8, 4), (17, 9), (32, 11), (65, 23)]:
        x = [_next_f64(prng) for _ in range(n)]
        h = [_next_f64(prng) for _ in range(k)]
        direct_out = convolve(x, h, mode=ConvMode.FULL)
        fft_out = fft_convolve(x, h, mode=ConvMode.FULL)
        _assert_close(direct_out, fft_out, tol=1e-4)


def test_pbt_overlap_add_equivalence() -> None:
    prng = SplitMix64(0xDDEEFF00)
    for n, k in [(30, 5), (60, 10), (120, 16)]:
        x = [_next_f64(prng) for _ in range(n)]
        h = [_next_f64(prng) for _ in range(k)]
        direct_out = convolve(x, h, mode=ConvMode.FULL)
        ola_out = overlap_add(x, h, block_size=16, mode=ConvMode.FULL)
        _assert_close(direct_out, ola_out, tol=1e-4)


def test_pbt_separable_2d_equivalence() -> None:
    prng = SplitMix64(0x13579BDF)
    h, w = 6, 7
    img = [_next_f64(prng) for _ in range(h * w)]
    h_ker = [_next_f64(prng) for _ in range(3)]
    v_ker = [_next_f64(prng) for _ in range(3)]
    outer_ker = [v * u for v in v_ker for u in h_ker]

    direct = conv2d(img, outer_ker, h, w, 3, 3, mode=BoundaryMode.ZERO)
    sep = sep_conv2d(img, h_ker, v_ker, h, w, mode=BoundaryMode.ZERO)
    _assert_close(direct, sep, tol=1e-4)
