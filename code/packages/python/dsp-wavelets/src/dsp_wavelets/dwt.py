"""1-D and 2-D Discrete Wavelet Transform (Mallat pyramid and synthesis)."""

from __future__ import annotations

from collections.abc import Sequence

from dsp_wavelets.boundary import (
    WaveletBoundary,
    parse_wavelet_boundary,
    sample_with_boundary,
)
from dsp_wavelets.errors import (
    MAX_SAMPLES,
    MAX_WAVELET_LEVELS,
    EmptySignalError,
    InvalidCoefficientsError,
    InvalidParamError,
    SignalTooShortError,
    validate_floats,
)
from dsp_wavelets.wavelet import Wavelet, WaveletFamily, parse_wavelet


def _require_orthogonal(w: Wavelet) -> None:
    """Ensure wavelet belongs to supported orthogonal families."""
    if w.family not in (
        WaveletFamily.HAAR,
        WaveletFamily.DAUBECHIES,
        WaveletFamily.SYMLETS,
        WaveletFamily.COIFLETS,
    ):
        raise InvalidParamError(
            f"DWT supports orthogonal wavelets only; got {w.name}"
        )


def _filter_and_downsample(
    signal: Sequence[float],
    h: list[float],
    g: list[float],
    boundary: WaveletBoundary,
) -> tuple[list[float], list[float]]:
    """Compute one level forward DWT: filter and downsample by 2."""
    n = len(signal)
    len_h = len(h)
    len_g = len(g)
    out_len = (n + 1) // 2

    ca = [0.0] * out_len
    cd = [0.0] * out_len

    for out_idx in range(out_len):
        k = 2 * out_idx + 1
        acc_h = 0.0
        for i in range(len_h):
            s = sample_with_boundary(signal, k - i, boundary)
            acc_h += h[i] * s
        acc_g = 0.0
        for i in range(len_g):
            s = sample_with_boundary(signal, k - i, boundary)
            acc_g += g[i] * s
        ca[out_idx] = acc_h
        cd[out_idx] = acc_g

    return ca, cd


def _synthesize_one_level(
    ca: Sequence[float],
    cd: Sequence[float],
    h: list[float],
    g: list[float],
    target_len: int,
    boundary: WaveletBoundary,
) -> list[float]:
    """Generic synthesis for any orthogonal/biorthogonal filter pair."""
    len_h = len(h)
    len_g = len(g)
    out = [0.0] * target_len

    for n in range(target_len):
        acc = 0.0
        for i in range(len_h):
            numerator = n + i - 1
            if numerator % 2 == 0:
                m = numerator // 2
                ca_val = sample_with_boundary(ca, m, boundary)
                acc += h[i] * ca_val
        for i in range(len_g):
            numerator = n + i - 1
            if numerator % 2 == 0:
                m = numerator // 2
                cd_val = sample_with_boundary(cd, m, boundary)
                acc += g[i] * cd_val
        out[n] = acc

    return out


def dwt(
    signal: Sequence[float],
    wavelet: Wavelet | str = "haar",
    boundary: WaveletBoundary | str = WaveletBoundary.SYMMETRIC,
) -> tuple[list[float], list[float]]:
    """Compute single-level 1-D discrete wavelet transform: returns (cA, cD)."""
    if len(signal) == 0:
        raise EmptySignalError("signal must be non-empty")
    if len(signal) > MAX_SAMPLES:
        raise InvalidParamError(f"signal length exceeds MAX_SAMPLES {MAX_SAMPLES}")
    validate_floats(signal, "signal")

    w = parse_wavelet(wavelet)
    _require_orthogonal(w)
    b = parse_wavelet_boundary(boundary)
    return _filter_and_downsample(signal, w.dec_lo, w.dec_hi, b)


def idwt(
    ca: Sequence[float],
    cd: Sequence[float],
    wavelet: Wavelet | str = "haar",
    boundary: WaveletBoundary | str = WaveletBoundary.SYMMETRIC,
    target_len: int | None = None,
) -> list[float]:
    """Compute single-level 1-D inverse discrete wavelet transform."""
    if len(ca) == 0 or len(cd) == 0:
        raise EmptySignalError("cA and cD must be non-empty")
    if len(ca) != len(cd):
        raise InvalidCoefficientsError("cA and cD must have equal lengths")
    validate_floats(ca, "ca", exc_type=InvalidCoefficientsError)
    validate_floats(cd, "cd", exc_type=InvalidCoefficientsError)

    t_len = target_len if target_len is not None else 2 * len(ca)
    if t_len <= 0 or t_len > MAX_SAMPLES:
        raise InvalidParamError(
            f"target_len exceeds MAX_SAMPLES {MAX_SAMPLES} or is non-positive"
        )

    w = parse_wavelet(wavelet)
    _require_orthogonal(w)
    b = parse_wavelet_boundary(boundary)
    return _synthesize_one_level(ca, cd, w.rec_lo, w.rec_hi, t_len, b)


def forward_level_lengths(initial_len: int, levels: int) -> list[int]:
    """Compute lengths [len_0, len_1, ..., len_levels] where len_0 = initial_len."""
    lens = [initial_len]
    curr = initial_len
    for _ in range(levels):
        curr = (curr + 1) // 2
        lens.append(curr)
    return lens


def dwt_1d(
    signal: Sequence[float],
    wavelet: Wavelet | str = "haar",
    levels: int = 1,
    boundary: WaveletBoundary | str = WaveletBoundary.SYMMETRIC,
) -> list[float]:
    """Forward 1-D DWT returning flattened [cA_J | cD_J | ... | cD_1] layout."""
    if len(signal) == 0:
        raise EmptySignalError("signal must be non-empty")
    if levels <= 0:
        raise InvalidParamError(f"levels must be > 0; got {levels}")
    if levels > MAX_WAVELET_LEVELS:
        raise InvalidParamError(
            f"levels {levels} exceeds MAX_WAVELET_LEVELS {MAX_WAVELET_LEVELS}"
        )
    if len(signal) > MAX_SAMPLES:
        raise InvalidParamError(f"signal length exceeds MAX_SAMPLES {MAX_SAMPLES}")
    validate_floats(signal, "signal")

    w = parse_wavelet(wavelet)
    _require_orthogonal(w)
    b = parse_wavelet_boundary(boundary)

    # Check signal has enough samples for levels
    if len(signal) < (1 << (levels - 1)):
        raise SignalTooShortError(
            f"signal length {len(signal)} too short for {levels} levels"
        )

    current = list(signal)
    details_reversed: list[list[float]] = []

    for _ in range(levels):
        ca, cd = _filter_and_downsample(current, w.dec_lo, w.dec_hi, b)
        details_reversed.append(cd)
        current = ca

    out = list(current)
    for cd_band in reversed(details_reversed):
        out.extend(cd_band)

    return out


def idwt_1d(
    coeffs: Sequence[float],
    wavelet: Wavelet | str = "haar",
    levels: int = 1,
    boundary: WaveletBoundary | str = WaveletBoundary.SYMMETRIC,
    output_length: int = 0,
) -> list[float]:
    """Inverse 1-D DWT reversing dwt_1d via synthesis filter bank."""
    if len(coeffs) == 0:
        raise EmptySignalError("coeffs must be non-empty")
    if output_length <= 0:
        raise InvalidParamError("output_length must be > 0")
    if output_length > MAX_SAMPLES or len(coeffs) > MAX_SAMPLES:
        raise InvalidParamError("length exceeds MAX_SAMPLES")
    if levels <= 0 or levels > MAX_WAVELET_LEVELS:
        raise InvalidParamError(f"invalid levels {levels}")
    validate_floats(coeffs, "coeffs", exc_type=InvalidCoefficientsError)

    w = parse_wavelet(wavelet)
    _require_orthogonal(w)
    b = parse_wavelet_boundary(boundary)

    level_lens = forward_level_lengths(output_length, levels)
    coarsest_ca_len = level_lens[levels]
    expected_total = coarsest_ca_len + sum(level_lens[1 : levels + 1])
    if len(coeffs) != expected_total:
        raise InvalidCoefficientsError(
            f"coeffs length {len(coeffs)} != expected {expected_total} "
            f"for output_length={output_length}, levels={levels}"
        )

    offset = 0
    current = list(coeffs[offset : offset + coarsest_ca_len])
    offset += coarsest_ca_len

    for j in range(levels, 0, -1):
        cd_len = level_lens[j]
        cd = coeffs[offset : offset + cd_len]
        offset += cd_len
        target_len = level_lens[j - 1]
        current = _synthesize_one_level(current, cd, w.rec_lo, w.rec_hi, target_len, b)

    return current


def dwt_2d(
    image: Sequence[float],
    n_rows: int,
    n_cols: int,
    wavelet: Wavelet | str = "haar",
    levels: int = 1,
    boundary: WaveletBoundary | str = WaveletBoundary.PERIODIC,
) -> list[float]:
    """Forward 2-D DWT quad-tree decomposition for row-major flattened images."""
    if len(image) == 0:
        raise EmptySignalError("image must be non-empty")
    if n_rows <= 0 or n_cols <= 0:
        raise InvalidParamError("image dimensions must be > 0")
    total_pixels = n_rows * n_cols
    if total_pixels > MAX_SAMPLES or len(image) != total_pixels:
        raise InvalidParamError("image size mismatch or exceeds MAX_SAMPLES")
    if levels <= 0 or levels > MAX_WAVELET_LEVELS:
        raise InvalidParamError(f"invalid levels {levels}")
    validate_floats(image, "image")

    w = parse_wavelet(wavelet)
    _require_orthogonal(w)
    b = parse_wavelet_boundary(boundary)

    current = list(image)
    cur_rows, cur_cols = n_rows, n_cols
    details_reversed: list[tuple[list[float], list[float], list[float]]] = []

    for _ in range(levels):
        half_rows = (cur_rows + 1) // 2
        half_cols = (cur_cols + 1) // 2

        # Row pass
        l_rows = [0.0] * (cur_rows * half_cols)
        h_rows = [0.0] * (cur_rows * half_cols)
        for r in range(cur_rows):
            row_slice = current[r * cur_cols : (r + 1) * cur_cols]
            ca, cd = _filter_and_downsample(row_slice, w.dec_lo, w.dec_hi, b)
            l_rows[r * half_cols : (r + 1) * half_cols] = ca
            h_rows[r * half_cols : (r + 1) * half_cols] = cd

        # Column pass
        ll = [0.0] * (half_rows * half_cols)
        hl = [0.0] * (half_rows * half_cols)
        lh = [0.0] * (half_rows * half_cols)
        hh = [0.0] * (half_rows * half_cols)

        col_buf = [0.0] * cur_rows
        for c in range(half_cols):
            # L columns -> LL and LH
            for r in range(cur_rows):
                col_buf[r] = l_rows[r * half_cols + c]
            ca_l, cd_l = _filter_and_downsample(col_buf, w.dec_lo, w.dec_hi, b)
            for r in range(half_rows):
                ll[r * half_cols + c] = ca_l[r]
                lh[r * half_cols + c] = cd_l[r]

            # H columns -> HL and HH
            for r in range(cur_rows):
                col_buf[r] = h_rows[r * half_cols + c]
            ca_h, cd_h = _filter_and_downsample(col_buf, w.dec_lo, w.dec_hi, b)
            for r in range(half_rows):
                hl[r * half_cols + c] = ca_h[r]
                hh[r * half_cols + c] = cd_h[r]

        details_reversed.append((hl, lh, hh))
        cur_rows, cur_cols = half_rows, half_cols
        current = ll

    out = list(current)  # LL_J
    for hl, lh, hh in reversed(details_reversed):
        out.extend(hl)
        out.extend(lh)
        out.extend(hh)
    return out


def idwt_2d(
    coeffs: Sequence[float],
    n_rows: int,
    n_cols: int,
    wavelet: Wavelet | str = "haar",
    levels: int = 1,
    boundary: WaveletBoundary | str = WaveletBoundary.PERIODIC,
) -> list[float]:
    """Inverse 2-D DWT quad-tree reconstruction."""
    if len(coeffs) == 0:
        raise EmptySignalError("coeffs must be non-empty")
    if n_rows <= 0 or n_cols <= 0:
        raise InvalidParamError("image dimensions must be > 0")
    total_pixels = n_rows * n_cols
    if total_pixels > MAX_SAMPLES:
        raise InvalidParamError("image dimensions exceed MAX_SAMPLES")
    if levels <= 0 or levels > MAX_WAVELET_LEVELS:
        raise InvalidParamError(f"invalid levels {levels}")
    validate_floats(coeffs, "coeffs", exc_type=InvalidCoefficientsError)

    w = parse_wavelet(wavelet)
    _require_orthogonal(w)
    b = parse_wavelet_boundary(boundary)

    # Compute per-level dimensions
    level_dims: list[tuple[int, int]] = [(n_rows, n_cols)]
    r_dim, c_dim = n_rows, n_cols
    for _ in range(levels):
        r_dim = (r_dim + 1) // 2
        c_dim = (c_dim + 1) // 2
        level_dims.append((r_dim, c_dim))

    ll_rows, ll_cols = level_dims[levels]
    expected_total = ll_rows * ll_cols + sum(
        3 * r * c for r, c in level_dims[1 : levels + 1]
    )
    if len(coeffs) != expected_total:
        raise InvalidCoefficientsError(
            f"coeffs length {len(coeffs)} != expected {expected_total}"
        )

    offset = 0
    ll = list(coeffs[offset : offset + ll_rows * ll_cols])
    offset += ll_rows * ll_cols

    for j in range(levels, 0, -1):
        rj, cj = level_dims[j]
        band_len = rj * cj
        hl = coeffs[offset : offset + band_len]
        offset += band_len
        lh = coeffs[offset : offset + band_len]
        offset += band_len
        hh = coeffs[offset : offset + band_len]
        offset += band_len

        target_rows, target_cols = level_dims[j - 1]

        # Inverse column pass
        l_rows = [0.0] * (target_rows * cj)
        h_rows = [0.0] * (target_rows * cj)
        ca_buf = [0.0] * rj
        cd_buf = [0.0] * rj

        for c in range(cj):
            for r in range(rj):
                ca_buf[r] = ll[r * cj + c]
                cd_buf[r] = lh[r * cj + c]
            col_l = _synthesize_one_level(
                ca_buf, cd_buf, w.rec_lo, w.rec_hi, target_rows, b
            )
            for r in range(target_rows):
                l_rows[r * cj + c] = col_l[r]

            for r in range(rj):
                ca_buf[r] = hl[r * cj + c]
                cd_buf[r] = hh[r * cj + c]
            col_h = _synthesize_one_level(
                ca_buf, cd_buf, w.rec_lo, w.rec_hi, target_rows, b
            )
            for r in range(target_rows):
                h_rows[r * cj + c] = col_h[r]

        # Inverse row pass
        reconstructed_band = [0.0] * (target_rows * target_cols)
        row_ca = [0.0] * cj
        row_cd = [0.0] * cj
        for r in range(target_rows):
            for c in range(cj):
                row_ca[c] = l_rows[r * cj + c]
                row_cd[c] = h_rows[r * cj + c]
            row_rec = _synthesize_one_level(
                row_ca, row_cd, w.rec_lo, w.rec_hi, target_cols, b
            )
            reconstructed_band[r * target_cols : (r + 1) * target_cols] = row_rec

        ll = reconstructed_band

    return ll
