defmodule CodingAdventures.ImagePointOps do
  @moduledoc """
  IMG03 — Per-pixel point operations on PixelContainer.

  A point operation transforms each pixel independently using only that
  pixel's own value — no neighbouring pixels, no frequency-domain transform.

  ## Two domains

  **u8-domain** operations work directly on the 8-bit sRGB bytes.  They are
  correct without colour-space conversion because they are monotone remappings
  that never mix or average channel values:

    - `invert/1`, `threshold/2`, `threshold_luminance/2`
    - `posterize/2`, `swap_rgb_bgr/1`, `extract_channel/2`
    - `brightness/2`

  **Linear-light** operations decode each byte to a linear-light float,
  perform the arithmetic, then re-encode.  Averaging in sRGB is incorrect
  (see IMG00 §2):

    - `contrast/2`, `gamma/2`, `exposure/2`
    - `greyscale/2`, `sepia/1`, `colour_matrix/2`
    - `saturate/2`, `hue_rotate/2`

  ## sRGB ↔ linear round-trip

  Decode (u8 → float):

      c = byte / 255.0
      if c <= 0.04045  →  c / 12.92
      else             →  ((c + 0.055) / 1.055) ** 2.4

  Encode (float → u8):

      if c <= 0.0031308  →  c * 12.92
      else               →  1.055 * c ** (1/2.4) - 0.055
      round(clamp(c, 0, 1) * 255)

  """

  alias CodingAdventures.PixelContainer

  @version "0.1.0"
  def version, do: @version

  # ── sRGB / linear LUT ─────────────────────────────────────────────────

  # Module attribute (computed at compile time): 256-entry decode LUT.
  # Tuple lookup is O(1) in Erlang.
  @srgb_to_linear (for i <- 0..255 do
                     c = i / 255.0

                     if c <= 0.04045 do
                       c / 12.92
                     else
                       :math.pow((c + 0.055) / 1.055, 2.4)
                     end
                   end
                   |> List.to_tuple())

  @doc false
  def decode(byte) do
    elem(@srgb_to_linear, byte)
  end

  @doc false
  def encode(linear) do
    c =
      if linear <= 0.0031308 do
        linear * 12.92
      else
        1.055 * :math.pow(linear, 1.0 / 2.4) - 0.055
      end

    round(min(1.0, max(0.0, c)) * 255)
  end

  # ── Iteration helper ───────────────────────────────────────────────────

  defp map_pixels(%PixelContainer{} = src, fun) do
    new_data =
      for y <- 0..(src.height - 1), x <- 0..(src.width - 1), into: <<>> do
        {r, g, b, a} = PixelContainer.pixel_at(src, x, y)
        {nr, ng, nb, na} = fun.(r, g, b, a)
        <<nr::8, ng::8, nb::8, na::8>>
      end

    %PixelContainer{width: src.width, height: src.height, data: new_data}
  end

  # ── u8-domain operations ───────────────────────────────────────────────

  @doc """
  Invert each RGB channel (255 − v).  Alpha is preserved.

  Applying `invert/1` twice returns the original image exactly because
  `255 − (255 − v) == v` for all integers in [0, 255].

  ## Example

      iex> img = CodingAdventures.PixelContainer.new(1, 1)
      iex> img = CodingAdventures.PixelContainer.set_pixel(img, 0, 0, 10, 100, 200, 255)
      iex> out = CodingAdventures.ImagePointOps.invert(img)
      iex> CodingAdventures.PixelContainer.pixel_at(out, 0, 0)
      {245, 155, 55, 255}

  """
  def invert(%PixelContainer{} = src) do
    map_pixels(src, fn r, g, b, a -> {255 - r, 255 - g, 255 - b, a} end)
  end

  @doc """
  Binarise on average luminance.  `(r+g+b)/3 >= value` → white, else black.
  Alpha is preserved.  For perceptual accuracy use `threshold_luminance/2`.
  """
  def threshold(%PixelContainer{} = src, value) do
    map_pixels(src, fn r, g, b, a ->
      luma = div(r + g + b, 3)
      v = if luma >= value, do: 255, else: 0
      {v, v, v, a}
    end)
  end

  @doc """
  Binarise on Rec. 709 luma: `Y = 0.2126 R + 0.7152 G + 0.0722 B`.
  """
  def threshold_luminance(%PixelContainer{} = src, value) do
    map_pixels(src, fn r, g, b, a ->
      luma = 0.2126 * r + 0.7152 * g + 0.0722 * b
      v = if luma >= value, do: 255, else: 0
      {v, v, v, a}
    end)
  end

  @doc """
  Reduce each channel to `levels` equally-spaced steps.

  `levels = 2` gives a high-contrast poster look.
  """
  def posterize(%PixelContainer{} = src, levels) do
    step = 255.0 / (levels - 1)
    q = fn v -> round(round(v / step) * step) end
    map_pixels(src, fn r, g, b, a -> {q.(r), q.(g), q.(b), a} end)
  end

  @doc "Swap R and B channels (RGB ↔ BGR)."
  def swap_rgb_bgr(%PixelContainer{} = src) do
    map_pixels(src, fn r, g, b, a -> {b, g, r, a} end)
  end

  @doc """
  Keep only the nominated channel (0=R, 1=G, 2=B, 3=A), zero the rest.
  Alpha is always preserved.
  """
  def extract_channel(%PixelContainer{} = src, channel) do
    map_pixels(src, fn r, g, b, a ->
      case channel do
        0 -> {r, 0, 0, a}
        1 -> {0, g, 0, a}
        2 -> {0, 0, b, a}
        _ -> {r, g, b, a}
      end
    end)
  end

  @doc """
  Add signed `offset` to each RGB channel, clamped to [0, 255].
  Alpha is preserved.
  """
  def brightness(%PixelContainer{} = src, offset) do
    clamp = fn v -> min(255, max(0, v + offset)) end
    map_pixels(src, fn r, g, b, a -> {clamp.(r), clamp.(g), clamp.(b), a} end)
  end

  # ── Linear-light operations ────────────────────────────────────────────

  @doc """
  Scale each linear channel around mid-grey (0.5 linear).

  `factor = 1.0` → identity; `< 1.0` → lower contrast; `> 1.0` → higher.
  """
  def contrast(%PixelContainer{} = src, factor) do
    map_pixels(src, fn r, g, b, a ->
      {
        encode(0.5 + factor * (decode(r) - 0.5)),
        encode(0.5 + factor * (decode(g) - 0.5)),
        encode(0.5 + factor * (decode(b) - 0.5)),
        a
      }
    end)
  end

  @doc """
  Apply power-law `g` in linear light.
  `g < 1` → brightens; `g > 1` → darkens; `g = 1` → identity.
  """
  def gamma(%PixelContainer{} = src, g) do
    map_pixels(src, fn r, gv, b, a ->
      {
        encode(:math.pow(decode(r), g)),
        encode(:math.pow(decode(gv), g)),
        encode(:math.pow(decode(b), g)),
        a
      }
    end)
  end

  @doc "Multiply linear luminance by 2^stops. +1 stop → double the light."
  def exposure(%PixelContainer{} = src, stops) do
    factor = :math.pow(2, stops)

    map_pixels(src, fn r, g, b, a ->
      {encode(decode(r) * factor), encode(decode(g) * factor), encode(decode(b) * factor), a}
    end)
  end

  @doc """
  Convert to luminance in linear light, re-encode to sRGB.

  `method` is one of `:rec709` (default), `:bt601`, or `:average`.
  """
  def greyscale(%PixelContainer{} = src, method \\ :rec709) do
    {wr, wg, wb} =
      case method do
        :rec709 -> {0.2126, 0.7152, 0.0722}
        :bt601 -> {0.2989, 0.5870, 0.1140}
        :average -> {1.0 / 3, 1.0 / 3, 1.0 / 3}
      end

    map_pixels(src, fn r, g, b, a ->
      y = encode(wr * decode(r) + wg * decode(g) + wb * decode(b))
      {y, y, y, a}
    end)
  end

  @doc "Apply a classic warm sepia tone matrix in linear light."
  def sepia(%PixelContainer{} = src) do
    map_pixels(src, fn r, g, b, a ->
      lr = decode(r)
      lg = decode(g)
      lb = decode(b)

      {
        encode(0.393 * lr + 0.769 * lg + 0.189 * lb),
        encode(0.349 * lr + 0.686 * lg + 0.168 * lb),
        encode(0.272 * lr + 0.534 * lg + 0.131 * lb),
        a
      }
    end)
  end

  @doc """
  Multiply linear [R, G, B] by a 3×3 matrix.

  `matrix` is a 3-tuple of 3-tuples (row-major):

      {{m00,m01,m02},{m10,m11,m12},{m20,m21,m22}}

  Identity: `{{1,0,0},{0,1,0},{0,0,1}}`.
  """
  def colour_matrix(%PixelContainer{} = src, {r_row, g_row, b_row}) do
    {m00, m01, m02} = r_row
    {m10, m11, m12} = g_row
    {m20, m21, m22} = b_row

    map_pixels(src, fn r, g, b, a ->
      lr = decode(r)
      lg = decode(g)
      lb = decode(b)

      {
        encode(m00 * lr + m01 * lg + m02 * lb),
        encode(m10 * lr + m11 * lg + m12 * lb),
        encode(m20 * lr + m21 * lg + m22 * lb),
        a
      }
    end)
  end

  @doc """
  Scale saturation. `0` → greyscale; `1` → identity; `> 1` → vivid.
  Uses Rec. 709 luminance weights.
  """
  def saturate(%PixelContainer{} = src, factor) do
    map_pixels(src, fn r, g, b, a ->
      lr = decode(r)
      lg = decode(g)
      lb = decode(b)
      grey = 0.2126 * lr + 0.7152 * lg + 0.0722 * lb

      {
        encode(grey + factor * (lr - grey)),
        encode(grey + factor * (lg - grey)),
        encode(grey + factor * (lb - grey)),
        a
      }
    end)
  end

  # ── HSV helpers ────────────────────────────────────────────────────────

  defp rgb_to_hsv(r, g, b) do
    mx = max(r, max(g, b))
    mn = min(r, min(g, b))
    delta = mx - mn
    v = mx
    s = if mx == 0.0, do: 0.0, else: delta / mx

    h =
      if delta == 0.0 do
        0.0
      else
        h0 =
          cond do
            mx == r -> :math.fmod((g - b) / delta, 6)
            mx == g -> (b - r) / delta + 2
            true -> (r - g) / delta + 4
          end

        :math.fmod(h0 * 60 + 360, 360)
      end

    {h, s, v}
  end

  defp hsv_to_rgb(h, s, v) do
    c = v * s
    x = c * (1 - abs(:math.fmod(h / 60, 2) - 1))
    m = v - c

    {r, g, b} =
      cond do
        h < 60 -> {c, x, 0.0}
        h < 120 -> {x, c, 0.0}
        h < 180 -> {0.0, c, x}
        h < 240 -> {0.0, x, c}
        h < 300 -> {x, 0.0, c}
        true -> {c, 0.0, x}
      end

    {r + m, g + m, b + m}
  end

  @doc "Rotate hue by `degrees`. 360° is identity."
  def hue_rotate(%PixelContainer{} = src, degrees) do
    map_pixels(src, fn r, g, b, a ->
      {h, s, v} = rgb_to_hsv(decode(r), decode(g), decode(b))
      {nr, ng, nb} = hsv_to_rgb(:math.fmod(h + degrees + 360, 360), s, v)
      {encode(nr), encode(ng), encode(nb), a}
    end)
  end

  # ── Colorspace utilities ───────────────────────────────────────────────

  @doc "Convert sRGB → linear (each byte becomes `round(linear * 255)`)."
  def srgb_to_linear_image(%PixelContainer{} = src) do
    map_pixels(src, fn r, g, b, a ->
      {round(decode(r) * 255), round(decode(g) * 255), round(decode(b) * 255), a}
    end)
  end

  @doc "Convert linear → sRGB (inverse of `srgb_to_linear_image/1`)."
  def linear_to_srgb_image(%PixelContainer{} = src) do
    map_pixels(src, fn r, g, b, a ->
      {encode(r / 255.0), encode(g / 255.0), encode(b / 255.0), a}
    end)
  end

  # ── 1D LUT operations ──────────────────────────────────────────────────

  @doc """
  Apply three 256-entry u8→u8 LUTs (one per channel).  Alpha preserved.

  Each LUT is a tuple of 256 integers, indexed by the input byte value.
  """
  def apply_lut1d_u8(%PixelContainer{} = src, lut_r, lut_g, lut_b) do
    map_pixels(src, fn r, g, b, a ->
      {elem(lut_r, r), elem(lut_g, g), elem(lut_b, b), a}
    end)
  end

  @doc """
  Build a 256-entry LUT from a linear-light mapping function.

  `fun` receives a linear float (0.0–1.0) and returns a linear float.
  Returns a 256-element tuple for O(1) lookup.
  """
  def build_lut1d_u8(fun) do
    for i <- 0..255 do
      encode(fun.(decode(i)))
    end
    |> List.to_tuple()
  end

  @doc "Build a gamma LUT (equivalent to `build_lut1d_u8(fn v -> v ** g end)`)."
  def build_gamma_lut(g) do
    build_lut1d_u8(fn v -> :math.pow(v, g) end)
  end
end
