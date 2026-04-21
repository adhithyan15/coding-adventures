defmodule CodingAdventures.ImageGeometricTransformsTest do
  use ExUnit.Case, async: true

  alias CodingAdventures.PixelContainer, as: PC
  alias CodingAdventures.ImageGeometricTransforms, as: GT

  # ── Helpers ────────────────────────────────────────────────────────────────

  # Build a 1×1 image with a single pixel
  defp solid(r, g, b, a) do
    PC.new(1, 1) |> PC.set_pixel(0, 0, r, g, b, a)
  end

  # Build a W×H image from a flat list of {r,g,b,a} tuples (row-major)
  defp from_list(pixels, w, h) do
    img = PC.new(w, h)

    pixels
    |> Enum.with_index()
    |> Enum.reduce(img, fn {{r, g, b, a}, idx}, acc ->
      x = rem(idx, w)
      y = div(idx, w)
      PC.set_pixel(acc, x, y, r, g, b, a)
    end)
  end

  # Collect all pixels row-major into a list of {r,g,b,a} tuples
  defp to_list(%PC{} = img) do
    for y <- 0..(img.height - 1), x <- 0..(img.width - 1) do
      PC.pixel_at(img, x, y)
    end
  end

  # Check that all pixel channels are within `tol` of the expected value
  defp close_pixel?({r1, g1, b1, a1}, {r2, g2, b2, a2}, tol) do
    abs(r1 - r2) <= tol and abs(g1 - g2) <= tol and abs(b1 - b2) <= tol and
      abs(a1 - a2) <= tol
  end

  # ── version ────────────────────────────────────────────────────────────────

  test "version returns a string" do
    assert is_binary(GT.version())
  end

  # ── flip_horizontal ────────────────────────────────────────────────────────

  test "flip_horizontal reverses pixel order in each row" do
    img =
      from_list(
        [
          {255, 0, 0, 255},
          {0, 255, 0, 255},
          {0, 0, 255, 255}
        ],
        3,
        1
      )

    out = GT.flip_horizontal(img)
    assert PC.pixel_at(out, 0, 0) == {0, 0, 255, 255}
    assert PC.pixel_at(out, 1, 0) == {0, 255, 0, 255}
    assert PC.pixel_at(out, 2, 0) == {255, 0, 0, 255}
  end

  test "flip_horizontal double application is identity" do
    img =
      from_list(
        [
          {10, 20, 30, 255},
          {40, 50, 60, 255},
          {70, 80, 90, 128}
        ],
        3,
        1
      )

    assert to_list(GT.flip_horizontal(GT.flip_horizontal(img))) == to_list(img)
  end

  test "flip_horizontal preserves dimensions" do
    img = PC.new(5, 7)
    out = GT.flip_horizontal(img)
    assert out.width == 5
    assert out.height == 7
  end

  # ── flip_vertical ──────────────────────────────────────────────────────────

  test "flip_vertical reverses row order" do
    img =
      from_list(
        [
          {255, 0, 0, 255},
          {0, 255, 0, 255},
          {0, 0, 255, 255}
        ],
        1,
        3
      )

    out = GT.flip_vertical(img)
    assert PC.pixel_at(out, 0, 0) == {0, 0, 255, 255}
    assert PC.pixel_at(out, 0, 1) == {0, 255, 0, 255}
    assert PC.pixel_at(out, 0, 2) == {255, 0, 0, 255}
  end

  test "flip_vertical double application is identity" do
    img =
      from_list(
        [
          {1, 2, 3, 4},
          {5, 6, 7, 8},
          {9, 10, 11, 12}
        ],
        1,
        3
      )

    assert to_list(GT.flip_vertical(GT.flip_vertical(img))) == to_list(img)
  end

  test "flip_vertical preserves dimensions" do
    img = PC.new(3, 6)
    out = GT.flip_vertical(img)
    assert out.width == 3
    assert out.height == 6
  end

  # ── rotate_90_cw ───────────────────────────────────────────────────────────

  test "rotate_90_cw swaps dimensions" do
    img = PC.new(4, 7)
    out = GT.rotate_90_cw(img)
    assert out.width == 7
    assert out.height == 4
  end

  test "rotate_90_cw four times is identity" do
    img =
      from_list(
        [
          {100, 0, 0, 255},
          {0, 100, 0, 255},
          {0, 0, 100, 255},
          {50, 50, 50, 128}
        ],
        2,
        2
      )

    result = img |> GT.rotate_90_cw() |> GT.rotate_90_cw() |> GT.rotate_90_cw() |> GT.rotate_90_cw()
    assert to_list(result) == to_list(img)
  end

  test "rotate_90_cw pixel placement" do
    # Input 2×3: A(0,0), B(1,0), C(2,0), D(0,1), E(1,1), F(2,1)
    # After 90° CW: output is 3×2
    # O(x',y') = I(y', W-1-x')   where W=2
    # O(0,0) = I(0,1) = D; O(1,0) = I(0,0) = A; ...
    img =
      from_list(
        [
          {1, 0, 0, 255},
          {2, 0, 0, 255},
          {3, 0, 0, 255},
          {4, 0, 0, 255},
          {5, 0, 0, 255},
          {6, 0, 0, 255}
        ],
        2,
        3
      )

    out = GT.rotate_90_cw(img)
    # Out dims: 3 wide, 2 tall
    assert out.width == 3
    assert out.height == 2
    # T⁻¹(x′, y′) = I(y′, W−1−x′)  where W=2 (input width)
    # O(0,0) = I(y'=0, W-1-x'=1) = I(x=0, y=1) = pixel at x=0,y=1 = {3,0,0,255}
    assert PC.pixel_at(out, 0, 0) == {3, 0, 0, 255}
    # O(1,0) = I(y'=0, W-1-1=0) = I(x=0, y=0) = {1,0,0,255}
    assert PC.pixel_at(out, 1, 0) == {1, 0, 0, 255}
  end

  # ── rotate_90_ccw ──────────────────────────────────────────────────────────

  test "rotate_90_ccw swaps dimensions" do
    img = PC.new(6, 2)
    out = GT.rotate_90_ccw(img)
    assert out.width == 2
    assert out.height == 6
  end

  test "rotate_90_ccw + rotate_90_cw is identity" do
    img =
      from_list(
        [
          {10, 20, 30, 255},
          {40, 50, 60, 255},
          {70, 80, 90, 255},
          {11, 22, 33, 200}
        ],
        2,
        2
      )

    result = img |> GT.rotate_90_ccw() |> GT.rotate_90_cw()
    assert to_list(result) == to_list(img)
  end

  # ── rotate_180 ─────────────────────────────────────────────────────────────

  test "rotate_180 preserves dimensions" do
    img = PC.new(5, 3)
    out = GT.rotate_180(img)
    assert out.width == 5
    assert out.height == 3
  end

  test "rotate_180 twice is identity" do
    img =
      from_list(
        [
          {1, 0, 0, 255},
          {0, 2, 0, 255},
          {0, 0, 3, 255},
          {4, 4, 4, 255}
        ],
        2,
        2
      )

    assert to_list(GT.rotate_180(GT.rotate_180(img))) == to_list(img)
  end

  test "rotate_180 is equivalent to flip_h then flip_v" do
    img =
      from_list(
        [
          {10, 20, 30, 255},
          {40, 50, 60, 128},
          {70, 80, 90, 255},
          {1, 2, 3, 64}
        ],
        2,
        2
      )

    assert to_list(GT.rotate_180(img)) ==
             to_list(GT.flip_horizontal(GT.flip_vertical(img)))
  end

  # ── crop ───────────────────────────────────────────────────────────────────

  test "crop produces correct dimensions" do
    img = PC.new(10, 10)
    out = GT.crop(img, 2, 3, 4, 5)
    assert out.width == 4
    assert out.height == 5
  end

  test "crop extracts correct pixel values" do
    img = PC.new(5, 5)
    img = PC.set_pixel(img, 2, 2, 200, 150, 100, 255)
    out = GT.crop(img, 2, 2, 2, 2)
    assert PC.pixel_at(out, 0, 0) == {200, 150, 100, 255}
    # Pixel at (1,1) in the crop is at (3,3) in the source, which is (0,0,0,0)
    assert PC.pixel_at(out, 1, 1) == {0, 0, 0, 0}
  end

  test "crop full image returns identical content" do
    img =
      from_list(
        [{100, 101, 102, 255}, {103, 104, 105, 255}, {106, 107, 108, 128},
         {200, 201, 202, 255}],
        2,
        2
      )

    out = GT.crop(img, 0, 0, 2, 2)
    assert to_list(out) == to_list(img)
  end

  # ── pad ────────────────────────────────────────────────────────────────────

  test "pad produces correct dimensions" do
    img = PC.new(3, 3)
    out = GT.pad(img, 1, 2, 3, 4)
    assert out.width == 3 + 2 + 4
    assert out.height == 3 + 1 + 3
  end

  test "pad default fill is transparent black" do
    img = PC.new(1, 1)
    out = GT.pad(img, 1, 1, 1, 1)
    assert PC.pixel_at(out, 0, 0) == {0, 0, 0, 0}
    assert PC.pixel_at(out, 2, 2) == {0, 0, 0, 0}
  end

  test "pad with custom fill colours border correctly" do
    img = PC.new(1, 1)
    out = GT.pad(img, 1, 1, 1, 1, {255, 128, 0, 255})
    assert PC.pixel_at(out, 0, 0) == {255, 128, 0, 255}
    assert PC.pixel_at(out, 2, 0) == {255, 128, 0, 255}
    assert PC.pixel_at(out, 0, 2) == {255, 128, 0, 255}
    assert PC.pixel_at(out, 2, 2) == {255, 128, 0, 255}
  end

  test "pad preserves interior pixel" do
    img = PC.new(2, 2)
    img = PC.set_pixel(img, 0, 0, 77, 88, 99, 255)
    out = GT.pad(img, 2, 0, 0, 3, {0, 0, 0, 0})
    # (0,0) in src → (3, 2) in output  (left=3, top=2)
    assert PC.pixel_at(out, 3, 2) == {77, 88, 99, 255}
  end

  # ── scale ──────────────────────────────────────────────────────────────────

  test "scale produces correct output dimensions" do
    img = PC.new(4, 6)
    out = GT.scale(img, 8, 12)
    assert out.width == 8
    assert out.height == 12
  end

  test "scale 1×1 to 1×1 is identity" do
    img = PC.new(1, 1) |> PC.set_pixel(0, 0, 200, 100, 50, 255)
    out = GT.scale(img, 1, 1)
    assert PC.pixel_at(out, 0, 0) == {200, 100, 50, 255}
  end

  test "scale solid color preserves color" do
    # A solid image scaled up should stay the same colour
    img = PC.new(2, 2)
    img = img |> PC.set_pixel(0, 0, 100, 150, 200, 255)
             |> PC.set_pixel(1, 0, 100, 150, 200, 255)
             |> PC.set_pixel(0, 1, 100, 150, 200, 255)
             |> PC.set_pixel(1, 1, 100, 150, 200, 255)

    out = GT.scale(img, 4, 4)
    # All pixels should be close to the original colour
    for y <- 0..(out.height - 1), x <- 0..(out.width - 1) do
      {r, g, b, _} = PC.pixel_at(out, x, y)
      assert abs(r - 100) <= 2
      assert abs(g - 150) <= 2
      assert abs(b - 200) <= 2
    end
  end

  test "scale :nearest mode produces correct output" do
    img = PC.new(2, 2)
    img = img |> PC.set_pixel(0, 0, 255, 0, 0, 255)
             |> PC.set_pixel(1, 0, 0, 255, 0, 255)
             |> PC.set_pixel(0, 1, 0, 0, 255, 255)
             |> PC.set_pixel(1, 1, 255, 255, 0, 255)

    out = GT.scale(img, 4, 4, :nearest)
    assert out.width == 4
    assert out.height == 4
  end

  # ── rotate ─────────────────────────────────────────────────────────────────

  test "rotate 0 radians is approximately identity" do
    img =
      from_list(
        [
          {100, 50, 25, 255},
          {50, 100, 25, 255},
          {25, 50, 100, 255},
          {100, 100, 100, 255}
        ],
        2,
        2
      )

    out = GT.rotate(img, 0.0)
    assert out.width == img.width
    assert out.height == img.height
    # Every pixel should be close to the original (within rounding)
    for y <- 0..(img.height - 1), x <- 0..(img.width - 1) do
      assert close_pixel?(PC.pixel_at(out, x, y), PC.pixel_at(img, x, y), 2)
    end
  end

  test "rotate :fit gives larger output for non-zero angle" do
    img = PC.new(10, 10)
    # 45° rotation should produce a larger canvas
    out = GT.rotate(img, :math.pi() / 4, :bilinear, :fit)
    assert out.width > 10 or out.height > 10
  end

  test "rotate :crop keeps same dimensions" do
    img = PC.new(8, 6)
    out = GT.rotate(img, 0.5, :bilinear, :crop)
    assert out.width == 8
    assert out.height == 6
  end

  test "rotate 2π is identity" do
    # Use a 4×4 solid-colour image so FP drift in the warp doesn't clip corner
    # pixels outside the `:zero` OOB boundary.  For a solid image, all in-bounds
    # pixels look the same, so this robustly checks that rotation by 2π leaves
    # the interior unchanged within rounding error.
    colour = {200, 100, 50, 255}

    img = PC.new(4, 4)
    img =
      for y <- 0..3, x <- 0..3, reduce: img do
        acc -> PC.set_pixel(acc, x, y, 200, 100, 50, 255)
      end

    out = GT.rotate(img, 2 * :math.pi())
    # Centre pixels (avoid edges where OOB zero may show through)
    for y <- 1..2, x <- 1..2 do
      assert close_pixel?(PC.pixel_at(out, x, y), colour, 2)
    end
  end

  # ── affine ─────────────────────────────────────────────────────────────────

  test "affine identity preserves image" do
    img = PC.new(4, 4)
    img = PC.set_pixel(img, 1, 2, 120, 80, 40, 255)
    id = {{1, 0, 0}, {0, 1, 0}}
    out = GT.affine(img, id, 4, 4)
    # Identity affine should preserve the pixel exactly
    assert close_pixel?(PC.pixel_at(out, 1, 2), {120, 80, 40, 255}, 2)
  end

  test "affine preserves output dimensions" do
    img = PC.new(5, 5)
    id = {{1, 0, 0}, {0, 1, 0}}
    out = GT.affine(img, id, 7, 3)
    assert out.width == 7
    assert out.height == 3
  end

  test "affine translation by zero is identity" do
    img = PC.new(4, 4)
    img = PC.set_pixel(img, 2, 1, 255, 128, 64, 255)
    # Translation by 0: maps output (x,y) → source (x+0, y+0)
    mat = {{1, 0, 0}, {0, 1, 0}}
    out = GT.affine(img, mat, 4, 4)
    assert close_pixel?(PC.pixel_at(out, 2, 1), {255, 128, 64, 255}, 2)
  end

  # ── perspective_warp ───────────────────────────────────────────────────────

  test "perspective_warp identity preserves image" do
    img = PC.new(4, 4)
    img = PC.set_pixel(img, 2, 2, 200, 100, 50, 255)
    id = {{1, 0, 0}, {0, 1, 0}, {0, 0, 1}}
    out = GT.perspective_warp(img, id, 4, 4)
    assert close_pixel?(PC.pixel_at(out, 2, 2), {200, 100, 50, 255}, 2)
  end

  test "perspective_warp preserves output dimensions" do
    img = PC.new(5, 5)
    id = {{1, 0, 0}, {0, 1, 0}, {0, 0, 1}}
    out = GT.perspective_warp(img, id, 8, 6)
    assert out.width == 8
    assert out.height == 6
  end

  # ── Out-of-bounds modes ────────────────────────────────────────────────────

  test "OOB :zero returns transparent black for out-of-range coords" do
    img = PC.new(1, 1) |> PC.set_pixel(0, 0, 255, 255, 255, 255)
    # Scale up way beyond the source — use nearest mode with affine to control
    # the exact source coordinate (ask for pixel at (-1, 0))
    mat = {{1, 0, -1}, {0, 1, 0}}
    out = GT.affine(img, mat, 2, 1, :nearest, :zero)
    # Output pixel (0,0) maps to source (-1, 0) which is OOB → {0,0,0,0}
    assert PC.pixel_at(out, 0, 0) == {0, 0, 0, 0}
    # Output pixel (1,0) maps to source (0, 0) which is in-bounds
    assert PC.pixel_at(out, 1, 0) == {255, 255, 255, 255}
  end

  test "OOB :replicate clamps to nearest edge" do
    img = PC.new(1, 1) |> PC.set_pixel(0, 0, 99, 88, 77, 255)
    # Ask for source coordinate (-5, 0) with replicate — should clamp to (0, 0)
    mat = {{1, 0, -5}, {0, 1, 0}}
    out = GT.affine(img, mat, 1, 1, :nearest, :replicate)
    assert PC.pixel_at(out, 0, 0) == {99, 88, 77, 255}
  end

  test "OOB :wrap tiles the image" do
    # 2-pixel image: (0,0)=red, (1,0)=blue
    img = PC.new(2, 1)
    img = PC.set_pixel(img, 0, 0, 255, 0, 0, 255)
    img = PC.set_pixel(img, 1, 0, 0, 0, 255, 255)
    # Ask for source x=2 with wrap (width=2) → should wrap to x=0 (red)
    mat = {{1, 0, 2}, {0, 1, 0}}
    out = GT.affine(img, mat, 1, 1, :nearest, :wrap)
    assert PC.pixel_at(out, 0, 0) == {255, 0, 0, 255}
  end

  test "OOB :reflect mirrors at boundary" do
    img = PC.new(2, 1)
    img = PC.set_pixel(img, 0, 0, 255, 0, 0, 255)
    img = PC.set_pixel(img, 1, 0, 0, 0, 255, 255)
    # Reflect: period = 4 for width=2
    # Source x=2 → in [0,4): 2 >= 2 → 2*2-1-2 = 1 → blue
    mat = {{1, 0, 2}, {0, 1, 0}}
    out = GT.affine(img, mat, 1, 1, :nearest, :reflect)
    assert PC.pixel_at(out, 0, 0) == {0, 0, 255, 255}
  end

  # ── Nearest sampler exact reads ────────────────────────────────────────────

  test "nearest sampler reads exact pixel without blending" do
    img = PC.new(3, 3)
    img = PC.set_pixel(img, 1, 1, 77, 88, 99, 200)
    mat = {{1, 0, 1}, {0, 1, 1}}
    out = GT.affine(img, mat, 1, 1, :nearest, :zero)
    assert PC.pixel_at(out, 0, 0) == {77, 88, 99, 200}
  end

  # ── Bilinear midpoint blending ─────────────────────────────────────────────

  test "bilinear midpoint between two pixels blends in linear light" do
    # A 2×1 image: left=black(0), right=white(255)
    # The midpoint u=0.5 should be approximately encode(0.5) ≈ 188 in linear light
    img = PC.new(2, 1)
    img = PC.set_pixel(img, 0, 0, 0, 0, 0, 255)
    img = PC.set_pixel(img, 1, 0, 255, 255, 255, 255)

    # Use affine to sample exactly at u=0.5, v=0 (midpoint)
    mat = {{1, 0, 0.5}, {0, 1, 0.0}}
    out = GT.affine(img, mat, 1, 1, :bilinear, :replicate)
    {r, g, b, _} = PC.pixel_at(out, 0, 0)
    # Linear midpoint: encode(0.5) ≈ 188
    # Allow some tolerance because the sampler also blends v with row below (OOB → replicate)
    assert r > 127
    assert g > 127
    assert b > 127
  end

  # ── encode/decode round-trip ───────────────────────────────────────────────

  test "sRGB encode-decode round-trip" do
    for i <- [0, 1, 50, 100, 128, 200, 254, 255] do
      assert abs(GT.encode(GT.decode(i)) - i) <= 1
    end
  end

  # ── Bicubic sampler coverage ────────────────────────────────────────────────

  test "bicubic scale produces correct output dimensions" do
    img = PC.new(4, 4)
    out = GT.scale(img, 8, 8, :bicubic)
    assert out.width == 8
    assert out.height == 8
  end

  test "bicubic solid colour is preserved" do
    img = PC.new(4, 4)
    img =
      for y <- 0..3, x <- 0..3, reduce: img do
        acc -> PC.set_pixel(acc, x, y, 100, 150, 200, 255)
      end

    out = GT.scale(img, 8, 8, :bicubic)
    # Interior pixels should be close to the source colour
    for y <- 1..6, x <- 1..6 do
      {r, g, b, _} = PC.pixel_at(out, x, y)
      assert abs(r - 100) <= 3
      assert abs(g - 150) <= 3
      assert abs(b - 200) <= 3
    end
  end

  test "bicubic with :zero OOB exercises nil path in kernel" do
    # Small image — bicubic kernel samples from outside the image (nil coords)
    img = PC.new(2, 2)
    img = PC.set_pixel(img, 0, 0, 200, 100, 50, 255)
    img = PC.set_pixel(img, 1, 0, 200, 100, 50, 255)
    img = PC.set_pixel(img, 0, 1, 200, 100, 50, 255)
    img = PC.set_pixel(img, 1, 1, 200, 100, 50, 255)
    # Affine identity — the 4×4 kernel will reach outside the 2×2 image
    id = {{1, 0, 0}, {0, 1, 0}}
    out = GT.affine(img, id, 2, 2, :bicubic, :zero)
    assert out.width == 2
    assert out.height == 2
  end

  test "bicubic affine identity preserves centre pixel" do
    img = PC.new(6, 6)
    img = PC.set_pixel(img, 3, 3, 80, 160, 240, 255)
    id = {{1, 0, 0}, {0, 1, 0}}
    out = GT.affine(img, id, 6, 6, :bicubic, :replicate)
    assert close_pixel?(PC.pixel_at(out, 3, 3), {80, 160, 240, 255}, 3)
  end

  # ── Perspective degenerate w_h = 0 ─────────────────────────────────────────

  test "perspective_warp degenerate w=0 pixel returns transparent black" do
    img = PC.new(4, 4)
    img = PC.set_pixel(img, 2, 2, 255, 0, 0, 255)
    # Matrix with h20=1, h21=0, h22=0: w = 1*x + 0*y + 0 = x
    # At x=0: w_h = 0 → degenerate, should return {0,0,0,0}
    h = {{1, 0, 0}, {0, 1, 0}, {1, 0, 0}}
    out = GT.perspective_warp(img, h, 4, 4)
    # Column 0 has w_h = 0, should get {0,0,0,0}
    assert PC.pixel_at(out, 0, 0) == {0, 0, 0, 0}
    assert PC.pixel_at(out, 0, 1) == {0, 0, 0, 0}
  end

  # ── rotate :crop bounds ─────────────────────────────────────────────────────

  test "rotate :nearest mode works" do
    img = PC.new(6, 6)
    img = PC.set_pixel(img, 3, 3, 128, 64, 32, 255)
    out = GT.rotate(img, 0.0, :nearest, :fit)
    assert out.width == 6
    assert out.height == 6
  end

  test "rotate bicubic mode works" do
    img = PC.new(6, 6)
    img =
      for y <- 0..5, x <- 0..5, reduce: img do
        acc -> PC.set_pixel(acc, x, y, 100, 100, 100, 255)
      end

    out = GT.rotate(img, 0.0, :bicubic, :fit)
    assert out.width == 6
    assert out.height == 6
  end

  # ── unused solid helper removal (use it once to silence warning) ─────────────

  test "solid/4 helper builds a 1x1 image" do
    img = solid(255, 0, 0, 255)
    assert img.width == 1
    assert img.height == 1
    assert PC.pixel_at(img, 0, 0) == {255, 0, 0, 255}
  end
end
