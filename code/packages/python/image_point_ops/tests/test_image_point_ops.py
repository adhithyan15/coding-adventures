"""Tests for image_point_ops — IMG03 point operations."""
from __future__ import annotations

import pytest
from pixel_container import create_pixel_container, pixel_at, set_pixel

from image_point_ops import (
    GreyscaleMethod,
    apply_lut1d_u8,
    brightness,
    build_gamma_lut,
    build_lut1d_u8,
    colour_matrix,
    contrast,
    exposure,
    extract_channel,
    gamma,
    greyscale,
    hue_rotate,
    invert,
    linear_to_srgb_image,
    posterize,
    saturate,
    sepia,
    srgb_to_linear_image,
    swap_rgb_bgr,
    threshold,
    threshold_luminance,
)


# ── helpers ────────────────────────────────────────────────────────────────

def solid(r: int, g: int, b: int, a: int):
    img = create_pixel_container(1, 1)
    set_pixel(img, 0, 0, r, g, b, a)
    return img


# ── dimensions ────────────────────────────────────────────────────────────

def test_dimensions_preserved():
    img = create_pixel_container(3, 5)
    out = invert(img)
    assert out.width == 3
    assert out.height == 5


# ── invert ────────────────────────────────────────────────────────────────

def test_invert_rgb():
    out = invert(solid(10, 100, 200, 255))
    assert pixel_at(out, 0, 0) == (245, 155, 55, 255)


def test_invert_preserves_alpha():
    out = invert(solid(10, 100, 200, 128))
    assert pixel_at(out, 0, 0)[3] == 128


def test_double_invert_identity():
    img = solid(30, 80, 180, 255)
    assert pixel_at(invert(invert(img)), 0, 0) == pixel_at(img, 0, 0)


# ── threshold ─────────────────────────────────────────────────────────────

def test_threshold_above():
    out = threshold(solid(200, 200, 200, 255), 128)
    assert pixel_at(out, 0, 0) == (255, 255, 255, 255)


def test_threshold_below():
    out = threshold(solid(50, 50, 50, 255), 128)
    assert pixel_at(out, 0, 0) == (0, 0, 0, 255)


def test_threshold_luminance_white():
    out = threshold_luminance(solid(255, 255, 255, 255), 128)
    assert pixel_at(out, 0, 0) == (255, 255, 255, 255)


# ── posterize ─────────────────────────────────────────────────────────────

def test_posterize_two_levels():
    out = posterize(solid(50, 50, 50, 255), 2)
    r, _, _, _ = pixel_at(out, 0, 0)
    assert r in (0, 255)


# ── swapRgbBgr ────────────────────────────────────────────────────────────

def test_swap_rgb_bgr():
    out = swap_rgb_bgr(solid(255, 0, 0, 255))
    assert pixel_at(out, 0, 0) == (0, 0, 255, 255)


# ── extractChannel ────────────────────────────────────────────────────────

def test_extract_channel_red():
    out = extract_channel(solid(100, 150, 200, 255), 0)
    assert pixel_at(out, 0, 0) == (100, 0, 0, 255)


def test_extract_channel_green():
    out = extract_channel(solid(100, 150, 200, 255), 1)
    assert pixel_at(out, 0, 0) == (0, 150, 0, 255)


# ── brightness ────────────────────────────────────────────────────────────

def test_brightness_clamps_high():
    out = brightness(solid(250, 10, 10, 255), 20)
    r, g, _, _ = pixel_at(out, 0, 0)
    assert r == 255
    assert g == 30


def test_brightness_clamps_low():
    out = brightness(solid(5, 10, 10, 255), -20)
    r, _, _, _ = pixel_at(out, 0, 0)
    assert r == 0


# ── contrast ──────────────────────────────────────────────────────────────

def test_contrast_identity():
    img = solid(100, 150, 200, 255)
    out = contrast(img, 1.0)
    orig = pixel_at(img, 0, 0)
    result = pixel_at(out, 0, 0)
    assert abs(result[0] - orig[0]) <= 1
    assert abs(result[1] - orig[1]) <= 1
    assert abs(result[2] - orig[2]) <= 1


# ── gamma ─────────────────────────────────────────────────────────────────

def test_gamma_identity():
    img = solid(100, 150, 200, 255)
    out = gamma(img, 1.0)
    orig = pixel_at(img, 0, 0)
    result = pixel_at(out, 0, 0)
    assert abs(result[0] - orig[0]) <= 1


def test_gamma_brightens_midtones():
    img = solid(128, 128, 128, 255)
    out = gamma(img, 0.5)
    r, _, _, _ = pixel_at(out, 0, 0)
    assert r > 128


# ── exposure ──────────────────────────────────────────────────────────────

def test_exposure_plus_one():
    img = solid(100, 100, 100, 255)
    out = exposure(img, 1.0)
    r, _, _, _ = pixel_at(out, 0, 0)
    orig_r, _, _, _ = pixel_at(img, 0, 0)
    assert r > orig_r


# ── greyscale ─────────────────────────────────────────────────────────────

def test_greyscale_white_stays_white():
    for method in GreyscaleMethod:
        out = greyscale(solid(255, 255, 255, 255), method)
        assert pixel_at(out, 0, 0) == (255, 255, 255, 255)


def test_greyscale_black_stays_black():
    out = greyscale(solid(0, 0, 0, 255))
    assert pixel_at(out, 0, 0) == (0, 0, 0, 255)


def test_greyscale_equal_channels():
    out = greyscale(solid(100, 100, 100, 255))
    r, g, b, _ = pixel_at(out, 0, 0)
    assert r == g == b


# ── sepia ─────────────────────────────────────────────────────────────────

def test_sepia_preserves_alpha():
    out = sepia(solid(128, 128, 128, 200))
    assert pixel_at(out, 0, 0)[3] == 200


# ── colour_matrix ─────────────────────────────────────────────────────────

def test_colour_matrix_identity():
    img = solid(80, 120, 200, 255)
    out = colour_matrix(img, [[1, 0, 0], [0, 1, 0], [0, 0, 1]])
    orig = pixel_at(img, 0, 0)
    result = pixel_at(out, 0, 0)
    assert abs(result[0] - orig[0]) <= 1
    assert abs(result[1] - orig[1]) <= 1
    assert abs(result[2] - orig[2]) <= 1


# ── saturate ──────────────────────────────────────────────────────────────

def test_saturate_zero_gives_grey():
    out = saturate(solid(200, 100, 50, 255), 0.0)
    r, g, b, _ = pixel_at(out, 0, 0)
    assert r == g == b


# ── hue_rotate ────────────────────────────────────────────────────────────

def test_hue_rotate_360_identity():
    img = solid(200, 80, 40, 255)
    out = hue_rotate(img, 360.0)
    orig = pixel_at(img, 0, 0)
    result = pixel_at(out, 0, 0)
    assert abs(result[0] - orig[0]) <= 2
    assert abs(result[1] - orig[1]) <= 2
    assert abs(result[2] - orig[2]) <= 2


# ── colorspace ────────────────────────────────────────────────────────────

def test_srgb_linear_roundtrip():
    img = solid(100, 150, 200, 255)
    out = linear_to_srgb_image(srgb_to_linear_image(img))
    orig = pixel_at(img, 0, 0)
    result = pixel_at(out, 0, 0)
    assert abs(result[0] - orig[0]) <= 2
    assert abs(result[1] - orig[1]) <= 2
    assert abs(result[2] - orig[2]) <= 2


# ── LUTs ──────────────────────────────────────────────────────────────────

def test_apply_lut1d_invert():
    invert_lut = bytes(255 - i for i in range(256))
    out = apply_lut1d_u8(solid(100, 0, 200, 255), invert_lut, invert_lut, invert_lut)
    assert pixel_at(out, 0, 0) == (155, 255, 55, 255)


def test_build_lut1d_u8_identity():
    lut = build_lut1d_u8(lambda v: v)
    for i in range(256):
        assert abs(lut[i] - i) <= 1


def test_build_gamma_lut_identity():
    lut = build_gamma_lut(1.0)
    for i in range(256):
        assert abs(lut[i] - i) <= 1
