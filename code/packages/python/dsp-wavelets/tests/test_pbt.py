"""Property-based testing for dsp_wavelets using deterministic SplitMix64 PRNG."""

from __future__ import annotations

import math

from dsp_wavelets.boundary import Periodic
from dsp_wavelets.dwt import dwt, dwt_1d, dwt_2d, idwt, idwt_1d, idwt_2d
from dsp_wavelets.swt import iswt, swt
from dsp_wavelets.wavedec import wavedec, waverec

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


def _assert_close(a: list[float], b: list[float], tol: float = 1e-3) -> None:
    assert len(a) == len(b), f"length mismatch: {len(a)} vs {len(b)}"
    for idx, (x, y) in enumerate(zip(a, b, strict=True)):
        scale = max(abs(x), abs(y), 1.0)
        assert abs(x - y) <= scale * tol, f"mismatch at {idx}: {x} vs {y} (tol {tol})"


def test_pbt_haar_energy_conservation() -> None:
    prng = SplitMix64(0xCAFE1234)
    for _ in range(25):
        n = 16
        sig = [_next_f64(prng) for _ in range(n)]
        ca, cd = dwt(sig, wavelet="haar", boundary=Periodic)

        energy_sig = sum(x * x for x in sig)
        energy_coeffs = sum(a * a for a in ca) + sum(d * d for d in cd)
        assert math.isclose(energy_sig, energy_coeffs, rel_tol=1e-4)


def test_pbt_dwt_idwt_roundtrip() -> None:
    prng = SplitMix64(0x5678ABCD)
    for _ in range(25):
        n = 32
        sig = [_next_f64(prng) for _ in range(n)]
        for w_name in ["haar", "db2", "db4"]:
            ca, cd = dwt(sig, wavelet=w_name, boundary=Periodic)
            rec = idwt(ca, cd, wavelet=w_name, boundary=Periodic, target_len=n)
            _assert_close(sig, rec, tol=1e-4)


def test_pbt_dwt_1d_multilevel() -> None:
    prng = SplitMix64(0x9ABCDEF0)
    for _ in range(15):
        n = 32
        sig = [_next_f64(prng) for _ in range(n)]
        coeffs = dwt_1d(sig, wavelet="db2", levels=3, boundary=Periodic)
        rec = idwt_1d(
            coeffs,
            wavelet="db2",
            levels=3,
            boundary=Periodic,
            output_length=n,
        )
        _assert_close(sig, rec, tol=1e-4)


def test_pbt_wavedec_waverec() -> None:
    prng = SplitMix64(0xFEEDFACE)
    for _ in range(15):
        n = 32
        sig = [_next_f64(prng) for _ in range(n)]
        decomp = wavedec(sig, wavelet="haar", boundary=Periodic, level=3)
        rec = waverec(decomp, wavelet="haar", boundary=Periodic)
        _assert_close(sig, rec, tol=1e-4)


def test_pbt_swt_iswt() -> None:
    prng = SplitMix64(0xDEADBEEF)
    for _ in range(15):
        n = 16
        sig = [_next_f64(prng) for _ in range(n)]
        coeffs = swt(sig, wavelet="haar", level=2, boundary=Periodic)
        rec = iswt(coeffs, wavelet="haar", boundary=Periodic)
        _assert_close(sig, rec, tol=1e-4)


def test_pbt_2d_dwt_roundtrip() -> None:
    prng = SplitMix64(0x1337C0DE)
    for _ in range(10):
        rows, cols = 8, 8
        image = [_next_f64(prng) for _ in range(rows * cols)]
        coeffs = dwt_2d(
            image, rows, cols, wavelet="haar", levels=2, boundary=Periodic
        )
        rec = idwt_2d(
            coeffs, rows, cols, wavelet="haar", levels=2, boundary=Periodic
        )
        _assert_close(image, rec, tol=1e-4)
