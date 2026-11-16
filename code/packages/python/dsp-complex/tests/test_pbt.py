"""Property-based testing for dsp_complex using deterministic SplitMix64 PRNG."""

from __future__ import annotations

import math

from dsp_complex import ComplexTensor


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
        # Uniform float in [-1000.0, 1000.0]
        u = (self.next_u64() >> 11) / (1 << 53)
        return (u * 2000.0) - 1000.0


def test_pbt_conjugate_twice_is_identity() -> None:
    prng = SplitMix64(0xFEEDFACE)
    for length in [0, 1, 2, 7, 16, 64, 128]:
        real = [prng.next_f64() for _ in range(length)]
        imag = [prng.next_f64() for _ in range(length)]
        t = ComplexTensor.from_real_imag(real, imag)
        assert t.conjugate().conjugate() == t


def test_pbt_component_reconstruction() -> None:
    prng = SplitMix64(0xCAFEBABE)
    for length in [1, 5, 32, 100]:
        real = [prng.next_f64() for _ in range(length)]
        imag = [prng.next_f64() for _ in range(length)]
        t = ComplexTensor.from_real_imag(real, imag)
        assert t.real() == real
        assert t.imag() == imag
        reconstructed = ComplexTensor.from_real_imag(t.real(), t.imag())
        assert reconstructed == t


def test_pbt_magnitude_phase_properties() -> None:
    prng = SplitMix64(0xDEADBEEF)
    for _ in range(50):
        r = prng.next_f64()
        i = prng.next_f64()
        t = ComplexTensor.from_real_imag([r], [i])
        mag = t.magnitude()[0]
        phase = t.phase()[0]
        # mag >= 0
        assert mag >= 0.0
        # Recover real and imag from polar form
        recovered_r = mag * math.cos(phase)
        recovered_i = mag * math.sin(phase)
        assert math.isclose(r, recovered_r, rel_tol=1e-5, abs_tol=1e-5)
        assert math.isclose(i, recovered_i, rel_tol=1e-5, abs_tol=1e-5)
