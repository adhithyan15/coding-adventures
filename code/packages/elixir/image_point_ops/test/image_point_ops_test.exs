defmodule CodingAdventures.ImagePointOpsTest do
  use ExUnit.Case, async: true

  alias CodingAdventures.PixelContainer, as: PC
  alias CodingAdventures.ImagePointOps, as: Ops

  defp solid(r, g, b, a) do
    img = PC.new(1, 1)
    PC.set_pixel(img, 0, 0, r, g, b, a)
  end

  defp px(img), do: PC.pixel_at(img, 0, 0)

  # ── version ───────────────────────────────────────────────────────────

  test "version returns a string" do
    assert is_binary(Ops.version())
  end

  # ── dimensions ────────────────────────────────────────────────────────

  test "dimensions are preserved" do
    img = PC.new(3, 5)
    out = Ops.invert(img)
    assert out.width == 3
    assert out.height == 5
  end

  # ── invert ────────────────────────────────────────────────────────────

  test "invert flips RGB" do
    out = Ops.invert(solid(10, 100, 200, 255))
    assert px(out) == {245, 155, 55, 255}
  end

  test "invert preserves alpha" do
    out = Ops.invert(solid(10, 100, 200, 128))
    {_, _, _, a} = px(out)
    assert a == 128
  end

  test "double invert is identity" do
    img = solid(30, 80, 180, 255)
    assert px(Ops.invert(Ops.invert(img))) == px(img)
  end

  # ── threshold ─────────────────────────────────────────────────────────

  test "threshold above gives white" do
    out = Ops.threshold(solid(200, 200, 200, 255), 128)
    assert px(out) == {255, 255, 255, 255}
  end

  test "threshold below gives black" do
    out = Ops.threshold(solid(50, 50, 50, 255), 128)
    assert px(out) == {0, 0, 0, 255}
  end

  test "threshold_luminance white stays white" do
    out = Ops.threshold_luminance(solid(255, 255, 255, 255), 128)
    assert px(out) == {255, 255, 255, 255}
  end

  # ── posterize ─────────────────────────────────────────────────────────

  test "posterize 2 levels binarises" do
    out = Ops.posterize(solid(50, 50, 50, 255), 2)
    {r, _, _, _} = px(out)
    assert r in [0, 255]
  end

  # ── swap_rgb_bgr ──────────────────────────────────────────────────────

  test "swap_rgb_bgr swaps R and B" do
    out = Ops.swap_rgb_bgr(solid(255, 0, 0, 255))
    assert px(out) == {0, 0, 255, 255}
  end

  # ── extract_channel ───────────────────────────────────────────────────

  test "extract channel 0 zeroes G and B" do
    out = Ops.extract_channel(solid(100, 150, 200, 255), 0)
    assert px(out) == {100, 0, 0, 255}
  end

  test "extract channel 1 zeroes R and B" do
    out = Ops.extract_channel(solid(100, 150, 200, 255), 1)
    assert px(out) == {0, 150, 0, 255}
  end

  test "extract channel 2 zeroes R and G" do
    out = Ops.extract_channel(solid(100, 150, 200, 255), 2)
    assert px(out) == {0, 0, 200, 255}
  end

  test "extract channel default preserves all" do
    out = Ops.extract_channel(solid(100, 150, 200, 255), 99)
    assert px(out) == {100, 150, 200, 255}
  end

  # ── brightness ────────────────────────────────────────────────────────

  test "brightness clamps high" do
    out = Ops.brightness(solid(250, 10, 10, 255), 20)
    {r, g, _, _} = px(out)
    assert r == 255
    assert g == 30
  end

  test "brightness clamps low" do
    out = Ops.brightness(solid(5, 10, 10, 255), -20)
    {r, _, _, _} = px(out)
    assert r == 0
  end

  # ── contrast ──────────────────────────────────────────────────────────

  test "contrast factor=1 is identity" do
    img = solid(100, 150, 200, 255)
    out = Ops.contrast(img, 1.0)
    {r, g, b, _} = px(out)
    {ir, ig, ib, _} = px(img)
    assert abs(r - ir) <= 1
    assert abs(g - ig) <= 1
    assert abs(b - ib) <= 1
  end

  # ── gamma ─────────────────────────────────────────────────────────────

  test "gamma=1 is identity" do
    img = solid(100, 150, 200, 255)
    out = Ops.gamma(img, 1.0)
    {r, _, _, _} = px(out)
    {ir, _, _, _} = px(img)
    assert abs(r - ir) <= 1
  end

  test "gamma<1 brightens midtones" do
    out = Ops.gamma(solid(128, 128, 128, 255), 0.5)
    {r, _, _, _} = px(out)
    assert r > 128
  end

  # ── exposure ──────────────────────────────────────────────────────────

  test "exposure +1 stop brightens" do
    img = solid(100, 100, 100, 255)
    out = Ops.exposure(img, 1.0)
    {r, _, _, _} = px(out)
    {ir, _, _, _} = px(img)
    assert r > ir
  end

  # ── greyscale ─────────────────────────────────────────────────────────

  test "greyscale white stays white" do
    for method <- [:rec709, :bt601, :average] do
      out = Ops.greyscale(solid(255, 255, 255, 255), method)
      assert px(out) == {255, 255, 255, 255}
    end
  end

  test "greyscale black stays black" do
    out = Ops.greyscale(solid(0, 0, 0, 255))
    assert px(out) == {0, 0, 0, 255}
  end

  test "greyscale gives equal channels" do
    out = Ops.greyscale(solid(100, 100, 100, 255))
    {r, g, b, _} = px(out)
    assert r == g
    assert g == b
  end

  # ── sepia ─────────────────────────────────────────────────────────────

  test "sepia preserves alpha" do
    out = Ops.sepia(solid(128, 128, 128, 200))
    {_, _, _, a} = px(out)
    assert a == 200
  end

  # ── colour_matrix ─────────────────────────────────────────────────────

  test "colour_matrix identity is identity" do
    img = solid(80, 120, 200, 255)
    id = {{1, 0, 0}, {0, 1, 0}, {0, 0, 1}}
    out = Ops.colour_matrix(img, id)
    {r, g, b, _} = px(out)
    {ir, ig, ib, _} = px(img)
    assert abs(r - ir) <= 1
    assert abs(g - ig) <= 1
    assert abs(b - ib) <= 1
  end

  # ── saturate ──────────────────────────────────────────────────────────

  test "saturate 0 gives grey" do
    out = Ops.saturate(solid(200, 100, 50, 255), 0.0)
    {r, g, b, _} = px(out)
    assert r == g
    assert g == b
  end

  # ── hue_rotate ────────────────────────────────────────────────────────

  test "hue_rotate shifts hue" do
    # Red pixel (hue ~0°) rotated 180° should become cyan-ish
    out = Ops.hue_rotate(solid(255, 0, 0, 255), 180.0)
    {r, _, b, _} = px(out)
    assert b > r
  end

  test "hue_rotate 360 is identity" do
    img = solid(200, 80, 40, 255)
    out = Ops.hue_rotate(img, 360.0)
    {r, g, b, _} = px(out)
    {ir, ig, ib, _} = px(img)
    assert abs(r - ir) <= 2
    assert abs(g - ig) <= 2
    assert abs(b - ib) <= 2
  end

  # ── colorspace ────────────────────────────────────────────────────────

  test "sRGB linear roundtrip" do
    img = solid(100, 150, 200, 255)
    out = Ops.linear_to_srgb_image(Ops.srgb_to_linear_image(img))
    {r, g, b, _} = px(out)
    {ir, ig, ib, _} = px(img)
    assert abs(r - ir) <= 2
    assert abs(g - ig) <= 2
    assert abs(b - ib) <= 2
  end

  # ── LUTs ──────────────────────────────────────────────────────────────

  test "apply_lut1d invert LUT inverts image" do
    lut = for i <- 0..255, do: 255 - i
    lut_t = List.to_tuple(lut)
    out = Ops.apply_lut1d_u8(solid(100, 0, 200, 255), lut_t, lut_t, lut_t)
    assert px(out) == {155, 255, 55, 255}
  end

  test "build_lut1d_u8 identity produces identity LUT" do
    lut = Ops.build_lut1d_u8(fn v -> v end)

    for i <- 0..255 do
      assert abs(elem(lut, i) - i) <= 1, "index #{i}: #{elem(lut, i)}"
    end
  end

  test "build_gamma_lut gamma=1 produces identity LUT" do
    lut = Ops.build_gamma_lut(1.0)

    for i <- 0..255 do
      assert abs(elem(lut, i) - i) <= 1, "index #{i}: #{elem(lut, i)}"
    end
  end
end
