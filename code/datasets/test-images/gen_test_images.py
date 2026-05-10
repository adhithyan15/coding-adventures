#!/usr/bin/env python3
"""Generate the synthetic test image suite for `code/datasets/test-images/`.

All images are 256×256 RGB PPM (P6).  They are deterministic — running
this script reproduces byte-for-byte identical files — and intentionally
small so they can be checked into git without bloating the repo.

Run from anywhere::

    python3 code/datasets/test-images/gen_test_images.py

The images this script produces are dedicated to the public domain
under CC0 1.0 (see ``LICENSE``).  Each image is designed to stress one
specific aspect of an image-processing pipeline:

* ``gradient_quad.ppm``     — saturated R/G/B quadrants + a yellow disc on
                               near-black background.  General sanity
                               check.  The "before" picture in our
                               instagram-filters demos.
* ``peppers_synthetic.ppm`` — overlapping organic blobs in saturated
                               red, yellow, and green.  Stand-in for the
                               classic "Peppers" image.  Useful for
                               sepia / colour-matrix work because it has
                               broad, smoothly-shaded surfaces in three
                               saturated hues.
* ``zone_plate.ppm``        — radial sinusoid of increasing frequency
                               (grayscale).  Aliasing / resampling /
                               anti-alias filter test.
* ``gamma_ramp.ppm``        — a column of 16 calibrated grayscale steps
                               next to a smooth horizontal gradient.
                               Posterize and gamma show banding clearly
                               here.
* ``mandrill_proxy.ppm``    — high-frequency procedural fur-like noise
                               in mixed warm hues.  Stand-in for the
                               classic "Mandrill" image, kept entirely
                               original (procedural simplex-style noise
                               rather than a recognisable face).

Why synthesise rather than ship a real photo?

The classical image-processing test images (Lenna, Peppers, Mandrill)
all have at-best-fuzzy licensing.  Synthetic images sidestep the
question entirely: they're produced by deterministic code we own, are
trivially CC0, and we can tune them to exercise specific filter
properties.
"""

from __future__ import annotations

import math
import os
import struct
from pathlib import Path

W = H = 256


# ─────────────────────────── PPM writer ───────────────────────────


def write_ppm(path: Path, pixels: bytes) -> None:
    """Write `pixels` (W*H*3 raw RGB bytes) as a P6 PPM file at `path`."""
    assert len(pixels) == W * H * 3, f"expected {W*H*3} bytes, got {len(pixels)}"
    header = f"P6\n{W} {H}\n255\n".encode("ascii")
    path.write_bytes(header + pixels)


# ─────────────────────────── Helpers ───────────────────────────


def clamp(v: float) -> int:
    if v < 0:
        return 0
    if v > 255:
        return 255
    return int(v)


# Cheap deterministic value-noise: build a coarse grid of pseudo-random
# scalars, then bilinearly interpolate.  Mulberry32 PRNG keeps the
# output stable across machines.


def mulberry32(seed: int):
    state = [seed & 0xFFFFFFFF]

    def step() -> float:
        state[0] = (state[0] + 0x6D2B79F5) & 0xFFFFFFFF
        t = state[0]
        t = ((t ^ (t >> 15)) * (t | 1)) & 0xFFFFFFFF
        t ^= (t + (((t ^ (t >> 7)) * (t | 61)) & 0xFFFFFFFF)) & 0xFFFFFFFF
        return ((t ^ (t >> 14)) & 0xFFFFFFFF) / 4294967296.0

    return step


def value_noise(seed: int, freq: int, w: int, h: int) -> list[float]:
    """Bilinear-interpolated value noise.  Returns w*h floats in [0, 1]."""
    rng = mulberry32(seed)
    grid_w = freq + 1
    grid_h = freq + 1
    grid = [rng() for _ in range(grid_w * grid_h)]
    out = []
    for y in range(h):
        gy = (y / (h - 1)) * freq
        y0 = int(gy)
        y1 = min(y0 + 1, freq)
        ty = gy - y0
        for x in range(w):
            gx = (x / (w - 1)) * freq
            x0 = int(gx)
            x1 = min(x0 + 1, freq)
            tx = gx - x0
            v00 = grid[y0 * grid_w + x0]
            v10 = grid[y0 * grid_w + x1]
            v01 = grid[y1 * grid_w + x0]
            v11 = grid[y1 * grid_w + x1]
            # Smoothstep on tx, ty for fewer grid artifacts.
            sx = tx * tx * (3 - 2 * tx)
            sy = ty * ty * (3 - 2 * ty)
            top = v00 * (1 - sx) + v10 * sx
            bot = v01 * (1 - sx) + v11 * sx
            out.append(top * (1 - sy) + bot * sy)
    return out


# ─────────────────────────── Images ───────────────────────────


def gradient_quad() -> bytes:
    """RGB quadrants + yellow disc on near-black background."""
    cx, cy = 192, 192
    r2 = 60 * 60
    out = bytearray()
    for y in range(H):
        for x in range(W):
            if x < 128 and y < 128:
                r, g, b = (x * 2), 30, 30
            elif x >= 128 and y < 128:
                r, g, b = 30, ((x - 128) * 2), 30
            elif x < 128 and y >= 128:
                r, g, b = 30, 30, ((y - 128) * 2)
            else:
                d2 = (x - cx) ** 2 + (y - cy) ** 2
                if d2 < r2:
                    r, g, b = 240, 220, 60
                else:
                    r, g, b = 20, 20, 30
            out.extend(bytes([clamp(r), clamp(g), clamp(b)]))
    return bytes(out)


def peppers_synthetic() -> bytes:
    """Saturated colour blobs over a dark background.

    Each blob is a Gaussian-falloff disc with a saturated body colour
    that fades to the background at the edges.  We composite via a
    proper "lerp toward body colour by Gaussian weight" so overlapping
    blobs don't blow out to white — instead the *strongest* blob at a
    given pixel wins (winner-take-all), giving a clear "five distinct
    peppers in a bowl" look rather than one big mushy blob.
    """
    bg = (25, 28, 35)  # near-black bluish background
    blobs = [
        # (cx,  cy,  sigma, (R, G, B))
        (70,  80, 32, (215,  55,  55)),   # red pepper
        (170, 75, 30, (240, 195,  60)),   # yellow pepper
        (90, 175, 34, (70, 175,  75)),   # green pepper
        (190, 175, 28, (215, 110,  45)),  # orange pepper
        (135, 130, 25, (160,  35,  90)),  # crimson pepper centre-ish
    ]
    out = bytearray()
    for y in range(H):
        for x in range(W):
            # Find the strongest contributing blob at this pixel.  Its
            # Gaussian weight tells us how much of its body colour to
            # lerp in over the background.
            best_w = 0.0
            best_col = bg
            for cx, cy, sigma, body in blobs:
                dx = x - cx
                dy = y - cy
                d2 = dx * dx + dy * dy
                w = math.exp(-d2 / (2 * sigma * sigma))
                if w > best_w:
                    best_w = w
                    best_col = body
            r = bg[0] * (1 - best_w) + best_col[0] * best_w
            g = bg[1] * (1 - best_w) + best_col[1] * best_w
            b = bg[2] * (1 - best_w) + best_col[2] * best_w
            out.extend(bytes([clamp(r), clamp(g), clamp(b)]))
    return bytes(out)


def zone_plate() -> bytes:
    """Radial sinusoidal pattern, frequency rising from centre.

    Standard test for resampling, anti-aliasing, demosaicing.  The
    instantaneous frequency at radius r is roughly k*r, so high-radius
    rings approach the Nyquist limit of the 256×256 grid and reveal
    aliasing in any filter that accidentally downsamples.
    """
    out = bytearray()
    cx = (W - 1) / 2.0
    cy = (H - 1) / 2.0
    # k chosen so the outer corner has ~50 cycles, well below the
    # Nyquist limit of 128 but enough to show ringing.
    k = 0.0008
    for y in range(H):
        for x in range(W):
            dx = x - cx
            dy = y - cy
            r2 = dx * dx + dy * dy
            v = math.cos(k * r2)
            g = clamp(127.5 + 127.5 * v)
            out.extend(bytes([g, g, g]))
    return bytes(out)


def gamma_ramp() -> bytes:
    """Discrete grayscale steps + smooth gradient.

    Left half: 16 horizontal stripes at evenly-spaced grey levels (0,
    17, 34, ..., 255).  Banding is *expected* here; this is the
    reference.  Right half: smooth horizontal gradient.  Posterise
    should turn the gradient into bands and leave the steps essentially
    unchanged.
    """
    out = bytearray()
    for y in range(H):
        step = (y // 16) * 17  # 0, 17, 34, ..., 255
        for x in range(W):
            if x < 128:
                v = step
            else:
                # smooth gradient based on x in [128, 255]
                t = (x - 128) / 127
                v = clamp(t * 255)
            out.extend(bytes([v, v, v]))
    return bytes(out)


def mandrill_proxy() -> bytes:
    """High-frequency procedural texture in mixed warm hues.

    Sums three octaves of value noise and modulates per-channel
    differently to produce a fur-like appearance suitable for the same
    purposes Mandrill is normally used for (sharpening / posterise /
    contrast tests where high-frequency content matters).
    """
    n1 = value_noise(seed=0xC0FFEE, freq=32, w=W, h=H)   # fine
    n2 = value_noise(seed=0xBADF00D, freq=8, w=W, h=H)   # medium
    n3 = value_noise(seed=0xFEEDFACE, freq=4, w=W, h=H)  # coarse
    out = bytearray()
    for i in range(W * H):
        a = n1[i]
        b = n2[i]
        c = n3[i]
        # Warm-toned palette: lots of red, less green, very little blue.
        r = 80 + 175 * (0.5 * a + 0.3 * b + 0.2 * c)
        g = 50 + 130 * (0.4 * a + 0.4 * b + 0.2 * c)
        bch = 30 + 100 * (0.6 * a + 0.2 * b + 0.2 * c)
        out.extend(bytes([clamp(r), clamp(g), clamp(bch)]))
    return bytes(out)


# ─────────────────────────── Driver ───────────────────────────


IMAGES = [
    ("gradient_quad.ppm", gradient_quad),
    ("peppers_synthetic.ppm", peppers_synthetic),
    ("zone_plate.ppm", zone_plate),
    ("gamma_ramp.ppm", gamma_ramp),
    ("mandrill_proxy.ppm", mandrill_proxy),
]


def main() -> None:
    here = Path(__file__).resolve().parent
    for name, fn in IMAGES:
        target = here / name
        target.write_bytes(b"")  # truncate first so we don't ship stale partials
        pixels = fn()
        write_ppm(target, pixels)
        print(f"  wrote {target.relative_to(here.parent.parent)}  ({target.stat().st_size} bytes)")


if __name__ == "__main__":
    main()
