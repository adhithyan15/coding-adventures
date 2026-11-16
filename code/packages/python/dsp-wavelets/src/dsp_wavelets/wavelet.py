"""Wavelet families, representations, and filter coefficients."""

from __future__ import annotations

import math
from enum import StrEnum

from dsp_wavelets.errors import InvalidParamError

FRAC_1_SQRT_2: float = 1.0 / math.sqrt(2.0)
SQRT_2: float = math.sqrt(2.0)


class WaveletFamily(StrEnum):
    """Supported wavelet families."""

    HAAR = "haar"
    DAUBECHIES = "db"
    SYMLETS = "sym"
    COIFLETS = "coif"
    BIORTHOGONAL = "bior"
    MORLET = "morlet"
    RICKER = "ricker"


# Tabulated orthogonal lowpass filter coefficients (sum = sqrt(2))
# Source: Daubechies 1992, PyWavelets v1.4, Rust dsp-wavelets filters.rs
DB2_DEC_LO = [
    -0.12940952,
     0.22414386,
     0.8365163,
     0.4829629,
]

DB4_DEC_LO = [
    -0.010597402,
     0.03288301,
     0.030841382,
    -0.18703482,
    -0.02798377,
     0.6308808,
     0.71484655,
     0.23037781,
]

DB6_DEC_LO = [
     0.11154074,
     0.4946239,
     0.7511339,
     0.31525034,
    -0.2262647,
    -0.12976687,
     0.097501606,
     0.027522866,
    -0.03158204,
     0.0005538422,
     0.0047772573,
    -0.0010773011,
]

DB8_DEC_LO = [
     0.05441584,
     0.3128716,
     0.67563075,
     0.5853547,
    -0.015829105,
    -0.28401554,
     0.00047248456,
     0.12874743,
    -0.0173693,
    -0.044088256,
     0.0139810275,
     0.008746094,
    -0.004870353,
    -0.00039174038,
     0.0006754494,
    -0.00011747678,
]

SYM4_DEC_LO = [
    -0.075765714,
    -0.029635528,
     0.49761868,
     0.8037388,
     0.2978578,
    -0.099219546,
    -0.012603967,
     0.0322231,
]

COIF1_DEC_LO = [
    -0.015655728,
    -0.07273262,
     0.38486484,
     0.852572,
     0.33789766,
    -0.07273262,
]

# JPEG 2000 Le Gall 5/3 (Biorthogonal 2.2 / 5.3)
BIOR2_2_DEC_LO = [
    -0.125 * SQRT_2,
     0.25 * SQRT_2,
     0.75 * SQRT_2,
     0.25 * SQRT_2,
    -0.125 * SQRT_2,
]

BIOR2_2_REC_LO = [
    0.5 * FRAC_1_SQRT_2,
    1.0 * FRAC_1_SQRT_2,
    0.5 * FRAC_1_SQRT_2,
]


def _qmf_highpass(h: list[float]) -> list[float]:
    """Derive QMF analysis highpass g from lowpass h: g[i] = (-1)^i * h[L - 1 - i]."""
    l_len = len(h)
    out = [0.0] * l_len
    for i in range(l_len):
        rev = h[l_len - 1 - i]
        out[i] = rev if (i % 2 == 0) else -rev
    return out


class Wavelet:
    """Wavelet filter bank definition."""

    def __init__(
        self,
        name: str,
        family: WaveletFamily,
        dec_lo: list[float],
        dec_hi: list[float],
        rec_lo: list[float] | None = None,
        rec_hi: list[float] | None = None,
    ) -> None:
        self.name: str = name
        self.family: WaveletFamily = family
        self.dec_lo: list[float] = list(dec_lo)
        self.dec_hi: list[float] = list(dec_hi)
        self.rec_lo: list[float] = list(rec_lo) if rec_lo is not None else list(dec_lo)
        self.rec_hi: list[float] = list(rec_hi) if rec_hi is not None else list(dec_hi)

    @property
    def filter_length(self) -> int:
        return len(self.dec_lo)

    @classmethod
    def haar(cls) -> Wavelet:
        h = [FRAC_1_SQRT_2, FRAC_1_SQRT_2]
        g = [FRAC_1_SQRT_2, -FRAC_1_SQRT_2]
        return cls("haar", WaveletFamily.HAAR, h, g)

    @classmethod
    def db(cls, n: int) -> Wavelet:
        if n == 1:
            return cls.haar()
        tables = {
            2: DB2_DEC_LO,
            4: DB4_DEC_LO,
            6: DB6_DEC_LO,
            8: DB8_DEC_LO,
        }
        if n not in tables:
            raise InvalidParamError(
                f"unsupported Daubechies order {n}; choose from [1, 2, 4, 6, 8]"
            )
        h = tables[n]
        g = _qmf_highpass(h)
        return cls(f"db{n}", WaveletFamily.DAUBECHIES, h, g)

    @classmethod
    def sym(cls, n: int) -> Wavelet:
        if n == 2:
            return cls.db(2)
        if n == 4:
            h = SYM4_DEC_LO
            g = _qmf_highpass(h)
            return cls("sym4", WaveletFamily.SYMLETS, h, g)
        raise InvalidParamError(f"unsupported Symlet order {n}; choose from [2, 4]")

    @classmethod
    def coif(cls, n: int) -> Wavelet:
        if n == 1:
            h = COIF1_DEC_LO
            g = _qmf_highpass(h)
            return cls("coif1", WaveletFamily.COIFLETS, h, g)
        raise InvalidParamError(f"unsupported Coiflet order {n}; choose from [1]")

    @classmethod
    def bior(cls, decomp: int = 2, recon: int = 2) -> Wavelet:
        if (decomp, recon) in ((2, 2), (5, 3)):
            h = BIOR2_2_DEC_LO
            g = _qmf_highpass(BIOR2_2_REC_LO)
            rec_h = BIOR2_2_REC_LO
            rec_g = _qmf_highpass(h)
            return cls("bior2.2", WaveletFamily.BIORTHOGONAL, h, g, rec_h, rec_g)
        raise InvalidParamError(f"unsupported Biorthogonal pair ({decomp}, {recon})")

    @classmethod
    def morlet(cls) -> Wavelet:
        return cls("morlet", WaveletFamily.MORLET, [], [])

    @classmethod
    def ricker(cls) -> Wavelet:
        return cls("ricker", WaveletFamily.RICKER, [], [])

    @classmethod
    def mexican_hat(cls) -> Wavelet:
        return cls.ricker()


def parse_wavelet(w: Wavelet | str) -> Wavelet:
    """Resolve a Wavelet instance from an existing object or string name."""
    if isinstance(w, Wavelet):
        return w
    if isinstance(w, str):
        s = w.strip().lower()
        if s == "haar":
            return Wavelet.haar()
        if s.startswith("db"):
            num_str = s[2:]
            if num_str.isdigit():
                return Wavelet.db(int(num_str))
        elif s.startswith("sym"):
            num_str = s[3:]
            if num_str.isdigit():
                return Wavelet.sym(int(num_str))
        elif s.startswith("coif"):
            num_str = s[4:]
            if num_str.isdigit():
                return Wavelet.coif(int(num_str))
        elif s in ("bior2.2", "bior22", "bior5.3", "bior53"):
            return Wavelet.bior(2, 2)
        elif s == "morlet":
            return Wavelet.morlet()
        elif s in ("ricker", "mexican_hat", "mexicanhat"):
            return Wavelet.ricker()
    raise InvalidParamError(f"unsupported wavelet identifier: {w}")
