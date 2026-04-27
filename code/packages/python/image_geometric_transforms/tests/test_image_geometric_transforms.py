"""
Tests for image_geometric_transforms (IMG04).

Each test section covers a specific transform or behaviour.
We create small PixelContainers with known pixel values and verify
properties like dimension changes, pixel identity, round-trips, and
boundary/OOB correctness.
"""
from __future__ import annotations

import math

from pixel_container import PixelContainer, create_pixel_container, pixel_at, set_pixel

from image_geometric_transforms import (
    Interpolation,
    OutOfBounds,
    RotateBounds,
    affine,
    crop,
    flip_horizontal,
    flip_vertical,
    pad,
    perspective_warp,
    rotate,
    rotate_90_ccw,
    rotate_90_cw,
    rotate_180,
    scale,
)

# ── Helpers ───────────────────────────────────────────────────────────────────


def make_solid(w: int, h: int, r: int, g: int, b: int, a: int) -> PixelContainer:
    """Create a w×h image filled with a single colour."""
    c = create_pixel_container(w, h)
    for y in range(h):
        for x in range(w):
            set_pixel(c, x, y, r, g, b, a)
    return c


def make_gradient_h(w: int, h: int) -> PixelContainer:
    """Horizontal gradient: column x → (x*255//(w-1), 0, 0, 255)."""
    c = create_pixel_container(w, h)
    for y in range(h):
        for x in range(w):
            v = x * 255 // (w - 1) if w > 1 else 0
            set_pixel(c, x, y, v, 0, 0, 255)
    return c


def make_checkerboard(w: int, h: int) -> PixelContainer:
    """2×2 checkerboard: white on even cells, black on odd."""
    c = create_pixel_container(w, h)
    for y in range(h):
        for x in range(w):
            v = 255 if (x + y) % 2 == 0 else 0
            set_pixel(c, x, y, v, v, v, 255)
    return c


def images_close(
    a: PixelContainer,
    b: PixelContainer,
    tol: int = 2,
) -> bool:
    """True if every channel of every pixel differs by at most tol."""
    if a.width != b.width or a.height != b.height:
        return False
    for y in range(a.height):
        for x in range(a.width):
            for ca, cb in zip(pixel_at(a, x, y), pixel_at(b, x, y)):
                if abs(ca - cb) > tol:
                    return False
    return True


# ── flip_horizontal ───────────────────────────────────────────────────────────


def test_flip_horizontal_reverses_columns() -> None:
    """The leftmost pixel of the source appears at the rightmost column of the output."""
    src = create_pixel_container(4, 1)
    for x in range(4):
        set_pixel(src, x, 0, x * 50, 0, 0, 255)

    out = flip_horizontal(src)
    assert out.width == 4
    assert out.height == 1
    for x in range(4):
        assert pixel_at(out, x, 0) == pixel_at(src, 3 - x, 0)


def test_flip_horizontal_double_is_identity() -> None:
    """Flipping horizontally twice must return the original image exactly."""
    src = make_checkerboard(5, 3)
    result = flip_horizontal(flip_horizontal(src))
    assert images_close(result, src, tol=0)


def test_flip_horizontal_preserves_dimensions() -> None:
    src = make_solid(7, 3, 10, 20, 30, 255)
    out = flip_horizontal(src)
    assert out.width == 7 and out.height == 3


# ── flip_vertical ─────────────────────────────────────────────────────────────


def test_flip_vertical_reverses_rows() -> None:
    """The top row of the source appears at the bottom row of the output."""
    src = create_pixel_container(3, 4)
    for y in range(4):
        for x in range(3):
            set_pixel(src, x, y, y * 50, 0, 0, 255)

    out = flip_vertical(src)
    assert out.width == 3
    assert out.height == 4
    for y in range(4):
        for x in range(3):
            assert pixel_at(out, x, y) == pixel_at(src, x, 3 - y)


def test_flip_vertical_double_is_identity() -> None:
    """Flipping vertically twice must return the original image exactly."""
    src = make_gradient_h(6, 4)
    result = flip_vertical(flip_vertical(src))
    assert images_close(result, src, tol=0)


# ── rotate_90_cw / rotate_90_ccw ─────────────────────────────────────────────


def test_rotate_90_cw_swaps_dimensions() -> None:
    """A 90° CW rotation of a W×H image produces an H×W image."""
    src = make_solid(4, 6, 1, 2, 3, 255)
    out = rotate_90_cw(src)
    assert out.width == 6 and out.height == 4


def test_rotate_90_ccw_swaps_dimensions() -> None:
    src = make_solid(5, 3, 1, 2, 3, 255)
    out = rotate_90_ccw(src)
    assert out.width == 3 and out.height == 5


def test_rotate_90_cw_then_ccw_is_identity() -> None:
    """CW followed by CCW must be the identity transform."""
    src = make_checkerboard(4, 4)
    result = rotate_90_ccw(rotate_90_cw(src))
    assert images_close(result, src, tol=0)


def test_rotate_90_four_times_is_identity() -> None:
    """Four 90° CW rotations must return the original image."""
    src = make_checkerboard(4, 4)
    r = rotate_90_cw(rotate_90_cw(rotate_90_cw(rotate_90_cw(src))))
    assert images_close(r, src, tol=0)


def test_rotate_90_cw_correct_pixel_mapping() -> None:
    """
    For a 3×2 image (W=3, H=2), rotating CW produces a 2×3 output.

    Mapping: out(x_out, y_out) <- src(y_out, H-1-x_out)
    The top-left of src (0,0) goes to out(x_out=H-1=1, y_out=0) = top-right of output.
    """
    # 3 wide × 2 tall
    src = create_pixel_container(3, 2)
    set_pixel(src, 0, 0, 255, 0, 0, 255)  # top-left is red
    out = rotate_90_cw(src)
    # out.width = H = 2, out.height = W = 3
    # src(0,0) → out(H-1=1, 0): the top-right of the output
    assert pixel_at(out, out.width - 1, 0) == (255, 0, 0, 255)


# ── rotate_180 ────────────────────────────────────────────────────────────────


def test_rotate_180_dimensions_unchanged() -> None:
    src = make_solid(5, 7, 1, 2, 3, 255)
    out = rotate_180(src)
    assert out.width == 5 and out.height == 7


def test_rotate_180_double_is_identity() -> None:
    src = make_checkerboard(6, 4)
    result = rotate_180(rotate_180(src))
    assert images_close(result, src, tol=0)


def test_rotate_180_pixel_mapping() -> None:
    """Verify that the top-left pixel appears at the bottom-right after 180°."""
    src = create_pixel_container(4, 3)
    set_pixel(src, 0, 0, 100, 200, 50, 255)
    out = rotate_180(src)
    # top-left (0,0) → bottom-right (W-1, H-1) = (3, 2)
    assert pixel_at(out, 3, 2) == (100, 200, 50, 255)


# ── crop ──────────────────────────────────────────────────────────────────────


def test_crop_dimensions() -> None:
    src = make_solid(10, 10, 1, 2, 3, 255)
    out = crop(src, 2, 3, 4, 5)
    assert out.width == 4 and out.height == 5


def test_crop_extracts_correct_region() -> None:
    """Pixel values in the cropped region match the source."""
    src = create_pixel_container(8, 8)
    for y in range(8):
        for x in range(8):
            set_pixel(src, x, y, x * 10, y * 10, 0, 255)

    out = crop(src, 2, 3, 4, 3)  # crop starting at (2,3), size 4×3
    for dy in range(3):
        for dx in range(4):
            expected = pixel_at(src, 2 + dx, 3 + dy)
            assert pixel_at(out, dx, dy) == expected


def test_crop_oob_fills_transparent() -> None:
    """Cropping outside the image boundary produces transparent black pixels."""
    src = make_solid(4, 4, 255, 0, 0, 255)
    # Request a crop that extends past the right/bottom edges
    out = crop(src, 2, 2, 6, 6)
    # Top-left 2×2 of crop is inside source (red), rest is OOB (transparent)
    assert pixel_at(out, 0, 0) == (255, 0, 0, 255)  # inside
    assert pixel_at(out, 4, 4) == (0, 0, 0, 0)      # outside


# ── pad ───────────────────────────────────────────────────────────────────────


def test_pad_dimensions() -> None:
    src = make_solid(4, 4, 1, 2, 3, 255)
    out = pad(src, top=1, right=2, bottom=3, left=4)
    assert out.width == 4 + 2 + 4 and out.height == 4 + 1 + 3


def test_pad_interior_matches_source() -> None:
    """Source pixels appear at the correct offset inside the padded image."""
    src = create_pixel_container(3, 3)
    for y in range(3):
        for x in range(3):
            set_pixel(src, x, y, x * 80, y * 80, 0, 255)

    out = pad(src, top=2, right=1, bottom=1, left=3)
    for y in range(3):
        for x in range(3):
            assert pixel_at(out, x + 3, y + 2) == pixel_at(src, x, y)


def test_pad_border_fill_colour() -> None:
    """Padding pixels outside the source area carry the fill colour."""
    src = make_solid(2, 2, 100, 100, 100, 255)
    fill = (0, 255, 0, 128)  # translucent green
    out = pad(src, top=1, right=1, bottom=1, left=1, fill=fill)
    # Top-left corner is part of the border
    assert pixel_at(out, 0, 0) == fill
    # Bottom-right corner is part of the border
    assert pixel_at(out, out.width - 1, out.height - 1) == fill


def test_pad_zero_padding_is_identity() -> None:
    """Padding with all zeros is equivalent to copying the source."""
    src = make_checkerboard(4, 4)
    out = pad(src, 0, 0, 0, 0)
    assert images_close(out, src, tol=0)


# ── scale ─────────────────────────────────────────────────────────────────────


def test_scale_up_doubles_dimensions() -> None:
    src = make_solid(4, 3, 50, 100, 150, 255)
    out = scale(src, 8, 6)
    assert out.width == 8 and out.height == 6


def test_scale_down_halves_dimensions() -> None:
    src = make_solid(8, 6, 10, 20, 30, 255)
    out = scale(src, 4, 3)
    assert out.width == 4 and out.height == 3


def test_scale_solid_colour_is_identical() -> None:
    """Scaling a solid-colour image produces a solid-colour image (all modes)."""
    src = make_solid(3, 3, 123, 45, 67, 255)
    for mode in Interpolation:
        out = scale(src, 6, 6, mode=mode)
        for y in range(6):
            for x in range(6):
                r, g, b, a = pixel_at(out, x, y)
                assert abs(r - 123) <= 1 and abs(g - 45) <= 1 and abs(b - 67) <= 1


def test_scale_nearest_1to1_is_identity() -> None:
    """Nearest-neighbour scaling to the same size must be pixel-perfect."""
    src = make_checkerboard(5, 5)
    out = scale(src, 5, 5, mode=Interpolation.NEAREST)
    assert images_close(out, src, tol=0)


# ── rotate ────────────────────────────────────────────────────────────────────


def test_rotate_zero_radians_is_identity() -> None:
    """Rotating by 0 radians should reproduce the source (within interpolation error)."""
    src = make_solid(8, 8, 200, 100, 50, 255)
    out = rotate(src, 0.0, mode=Interpolation.BILINEAR, bounds=RotateBounds.CROP)
    assert out.width == 8 and out.height == 8
    # Centre pixel should be very close to original
    cx, cy = 4, 4
    r, g, b, _ = pixel_at(out, cx, cy)
    assert abs(r - 200) <= 2 and abs(g - 100) <= 2 and abs(b - 50) <= 2


def test_rotate_fit_expands_canvas() -> None:
    """FIT mode must produce a canvas at least as large as the source for non-zero angles."""
    src = make_solid(10, 10, 1, 2, 3, 255)
    out = rotate(src, math.pi / 4, bounds=RotateBounds.FIT)
    # Diagonal of a 10×10 square ≈ 14.14 → canvas > 10
    assert out.width > 10 and out.height > 10


def test_rotate_crop_preserves_dimensions() -> None:
    src = make_solid(8, 6, 1, 2, 3, 255)
    out = rotate(src, math.pi / 3, bounds=RotateBounds.CROP)
    assert out.width == 8 and out.height == 6


def test_rotate_90_via_rotate_swaps_dimensions_approx() -> None:
    """rotate(π/2, FIT) should produce a nearly square canvas when src is square."""
    src = make_solid(8, 8, 1, 2, 3, 255)
    out = rotate(src, math.pi / 2, bounds=RotateBounds.FIT)
    assert out.width == out.height == 8


# ── affine ────────────────────────────────────────────────────────────────────


def test_affine_identity_matrix_is_identity() -> None:
    """A 2×3 identity affine matrix must reproduce the source image."""
    identity = [[1.0, 0.0, 0.0], [0.0, 1.0, 0.0]]
    src = make_checkerboard(6, 6)
    out = affine(src, identity, 6, 6, mode=Interpolation.NEAREST, oob=OutOfBounds.REPLICATE)
    assert images_close(out, src, tol=0)


def test_affine_translation() -> None:
    """An affine with a +1,+1 translation shifts the image by one pixel."""
    shift = [[1.0, 0.0, -1.0], [0.0, 1.0, -1.0]]  # inverse: shift source by +1,+1
    src = create_pixel_container(5, 5)
    set_pixel(src, 1, 1, 255, 0, 0, 255)
    # Output at (1,1) should read source at (1-1, 1-1) = (0,0) → black
    # Output at (2,2) should read source at (1,1) → red
    out = affine(src, shift, 5, 5, mode=Interpolation.NEAREST, oob=OutOfBounds.ZERO)
    assert pixel_at(out, 2, 2) == (255, 0, 0, 255)


def test_affine_output_dimensions() -> None:
    identity = [[1.0, 0.0, 0.0], [0.0, 1.0, 0.0]]
    src = make_solid(4, 3, 0, 0, 0, 255)
    out = affine(src, identity, 7, 9)
    assert out.width == 7 and out.height == 9


# ── perspective_warp ──────────────────────────────────────────────────────────


def test_perspective_warp_identity_matrix() -> None:
    """A 3×3 identity homography must reproduce the source image."""
    h = [[1.0, 0.0, 0.0], [0.0, 1.0, 0.0], [0.0, 0.0, 1.0]]
    src = make_checkerboard(6, 6)
    out = perspective_warp(src, h, 6, 6, mode=Interpolation.NEAREST, oob=OutOfBounds.REPLICATE)
    assert images_close(out, src, tol=0)


def test_perspective_warp_output_dimensions() -> None:
    h = [[1.0, 0.0, 0.0], [0.0, 1.0, 0.0], [0.0, 0.0, 1.0]]
    src = make_solid(4, 4, 0, 0, 0, 255)
    out = perspective_warp(src, h, 8, 8)
    assert out.width == 8 and out.height == 8


# ── Interpolation modes ───────────────────────────────────────────────────────


def test_nearest_returns_exact_pixel_values() -> None:
    """Nearest-neighbour sampling of integer coordinates returns exact pixel values."""
    src = create_pixel_container(4, 4)
    set_pixel(src, 2, 2, 100, 150, 200, 255)
    # scale 1:1 with nearest should give exact pixel
    out = scale(src, 4, 4, mode=Interpolation.NEAREST)
    assert pixel_at(out, 2, 2) == (100, 150, 200, 255)


def test_bilinear_midpoint_blend_horizontal_gradient() -> None:
    """
    On a horizontal gradient from 0 to 255, bilinear scaling should
    produce a value between 0 and 255 at the midpoint (not exactly 0 or 255).
    """
    # 2-pixel wide: left=0, right=255
    src = create_pixel_container(2, 1)
    set_pixel(src, 0, 0, 0, 0, 0, 255)
    set_pixel(src, 1, 0, 255, 0, 0, 255)
    out = scale(src, 4, 1, mode=Interpolation.BILINEAR)
    # The middle output pixels should be blended, not pure black or pure red
    r_mid, _, _, _ = pixel_at(out, 2, 0)
    assert 0 < r_mid < 255


def test_all_oob_modes_run_without_error() -> None:
    """All OutOfBounds modes complete without raising an exception."""
    # An affine that shifts source well outside the image
    shift = [[1.0, 0.0, -100.0], [0.0, 1.0, -100.0]]
    src = make_solid(4, 4, 50, 100, 150, 255)
    for oob in OutOfBounds:
        out = affine(src, shift, 4, 4, mode=Interpolation.BILINEAR, oob=oob)
        assert out.width == 4 and out.height == 4


def test_oob_replicate_extends_border() -> None:
    """REPLICATE OOB: all output pixels outside the source get the edge colour."""
    src = make_solid(2, 2, 200, 100, 50, 255)
    # Shift source image 10 pixels left (so all output sees OOB on the right)
    shift = [[1.0, 0.0, 10.0], [0.0, 1.0, 0.0]]
    out = affine(src, shift, 2, 2, mode=Interpolation.NEAREST, oob=OutOfBounds.REPLICATE)
    # All output pixels should be the edge colour (replicated)
    for y in range(2):
        for x in range(2):
            r, g, b, _ = pixel_at(out, x, y)
            assert abs(r - 200) <= 2 and abs(g - 100) <= 2 and abs(b - 50) <= 2


def test_oob_reflect_and_wrap_run() -> None:
    """REFLECT and WRAP modes produce output without error."""
    src = make_gradient_h(4, 4)
    shift = [[1.0, 0.0, -2.0], [0.0, 1.0, -2.0]]
    out_r = affine(src, shift, 8, 8, oob=OutOfBounds.REFLECT)
    out_w = affine(src, shift, 8, 8, oob=OutOfBounds.WRAP)
    assert out_r.width == 8 and out_w.width == 8


def test_bicubic_mode_runs() -> None:
    """Bicubic interpolation completes and returns a correctly-sized result."""
    src = make_gradient_h(8, 8)
    out = scale(src, 16, 16, mode=Interpolation.BICUBIC)
    assert out.width == 16 and out.height == 16


# ── Additional edge-case tests ─────────────────────────────────────────────────


def test_flip_then_rotate_180_equivalence() -> None:
    """flip_horizontal(flip_vertical(x)) should equal rotate_180(x)."""
    src = make_checkerboard(6, 4)
    via_flips = flip_horizontal(flip_vertical(src))
    via_rotate = rotate_180(src)
    assert images_close(via_flips, via_rotate, tol=0)


def test_crop_then_pad_recovers_source() -> None:
    """Cropping then padding back with the correct offset should equal the original."""
    src = make_solid(6, 6, 80, 160, 240, 255)
    cropped = crop(src, 1, 1, 4, 4)
    recovered = pad(cropped, top=1, right=1, bottom=1, left=1, fill=(80, 160, 240, 255))
    # The interior of the recovered image should match the original
    for y in range(1, 5):
        for x in range(1, 5):
            assert pixel_at(recovered, x, y) == pixel_at(src, x, y)


def test_scale_all_interpolation_modes() -> None:
    """All three interpolation modes produce correctly dimensioned output."""
    src = make_gradient_h(4, 4)
    for mode in Interpolation:
        out = scale(src, 8, 8, mode=mode)
        assert out.width == 8 and out.height == 8


def test_rotate_full_circle_is_identity() -> None:
    """Rotating by 2π should approximate the identity (within interpolation error)."""
    src = make_solid(8, 8, 150, 100, 200, 255)
    out = rotate(src, 2 * math.pi, bounds=RotateBounds.CROP)
    assert images_close(out, src, tol=3)
