defmodule CodingAdventures.ImageGeometricTransforms do
  @moduledoc """
  IMG04 — Geometric transforms on PixelContainer images.

  A **geometric transform** remaps pixel *locations* rather than pixel *values*.
  Every output pixel is computed by mapping its coordinate backwards into the
  source image (the **inverse warp** convention) and sampling there.

  ```
  (u, v) = T⁻¹(x′, y′)          — inverse warp: where to read from input
  O(x′, y′) = sample(I, u, v)   — sampling with chosen interpolation mode
  ```

  The inverse warp convention has two desirable properties:
  - No holes: every output pixel maps to *some* source coordinate.
  - Trivially parallel: each output pixel is independent.

  ## Two families of transforms

  | Family             | Examples                                  | Pixel quality? |
  |--------------------|-------------------------------------------|----------------|
  | Integer (lossless) | flip, rotate 90/180, crop, pad            | Exact byte copy|
  | Continuous (lossy) | scale, rotate, affine, perspective        | Interpolation  |

  Lossless transforms copy raw RGBA8 bytes without arithmetic.  Continuous
  transforms perform the weighted blend **in linear light** — averaging in
  sRGB is physically incorrect because sRGB is a perceptual (gamma-encoded)
  space, not a linear energy space.

  ## sRGB ↔ linear round-trip

  The IEC 61966-2-1 piecewise formula:

  **Decode** u8 → float (sRGB → linear):

      c = byte / 255.0
      if c <= 0.04045  →  c / 12.92
      else             →  ((c + 0.055) / 1.055)^2.4

  **Encode** float → u8 (linear → sRGB):

      if v <= 0.0031308  →  v * 12.92
      else               →  1.055 * v^(1/2.4) - 0.055
      round(clamp(v, 0, 1) * 255)

  The decode LUT is precomputed at compile time into a 256-entry tuple for O(1)
  lookup — the same pattern used in `image_point_ops`.

  ## Out-of-bounds modes

  When an inverse-warped coordinate falls outside the source image:

  | Mode        | Behaviour                                    |
  |-------------|----------------------------------------------|
  | `:zero`     | Transparent black `{0,0,0,0}`                |
  | `:replicate`| Clamp to nearest edge pixel                  |
  | `:reflect`  | Mirror at boundary (period = 2 * dimension)  |
  | `:wrap`     | Tile (modular arithmetic)                    |

  ## Interpolation modes

  | Mode        | Speed  | Quality | Notes                        |
  |-------------|--------|---------|------------------------------|
  | `:nearest`  | Fast   | Low     | No blending; pixel-art look  |
  | `:bilinear` | Medium | Good    | 2×2 linear-light blend       |
  | `:bicubic`  | Slow   | Best    | 4×4 Catmull-Rom kernel       |

  """

  alias CodingAdventures.PixelContainer

  @version "0.1.0"
  def version, do: @version

  # ── sRGB / linear LUT ──────────────────────────────────────────────────────
  #
  # Precomputed at compile time.  Elixir module attributes are evaluated once
  # during compilation; the resulting 256-element tuple lives in the BEAM
  # module's constant pool and is accessed in O(1) via `elem/2`.

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
  def decode(b), do: elem(@srgb_to_linear, b)

  @doc false
  def encode(v) do
    c =
      if v <= 0.0031308 do
        v * 12.92
      else
        1.055 * :math.pow(v, 1.0 / 2.4) - 0.055
      end

    round(min(1.0, max(0.0, c)) * 255)
  end

  # ── Out-of-bounds coordinate resolution ────────────────────────────────────
  #
  # Returns an integer coordinate in [0, max) or `nil` for the `:zero` mode
  # when the coordinate is out of range.

  defp rem_euclid(x, m), do: rem(rem(x, m) + m, m)

  defp resolve_coord(x, max, oob) do
    # Normalise to integer — x may be a float (from u/v computations) or
    # an integer (when x0+1 is passed from bilinear/bicubic neighbours).
    xi = if is_integer(x), do: x, else: trunc(x)

    case oob do
      :zero ->
        if xi < 0 or xi >= max, do: nil, else: xi

      :replicate ->
        max(0, min(max - 1, xi))

      :reflect ->
        # Period is 2*max.  Map xi into [0, 2*max) via Euclidean remainder,
        # then fold the upper half back: if xi >= max, use (2*max - 1 - xi).
        # This gives seamless mirroring with no single-pixel edge duplication.
        period = 2 * max
        xm = rem_euclid(xi, period)
        if xm >= max, do: period - 1 - xm, else: xm

      :wrap ->
        Integer.mod(xi, max)
    end
  end

  # ── Catmull-Rom cubic kernel ────────────────────────────────────────────────
  #
  # Used by the bicubic sampler.  Given distance `d` (always non-negative),
  # the Catmull-Rom weight is the piecewise cubic:
  #
  #   d < 1:  1.5*d³ - 2.5*d² + 1
  #   d < 2:  -0.5*d³ + 2.5*d² - 4*d + 2
  #   else:   0

  defp catmull_rom(d) do
    d = abs(d)

    cond do
      d < 1.0 -> 1.5 * d * d * d - 2.5 * d * d + 1.0
      d < 2.0 -> -0.5 * d * d * d + 2.5 * d * d - 4.0 * d + 2.0
      true -> 0.0
    end
  end

  # ── Sampling helpers ────────────────────────────────────────────────────────
  #
  # Each sampler returns a 4-tuple of integers {r, g, b, a} in [0,255].

  # Nearest-neighbour: round to the nearest integer coordinate.
  # No blending, no colorspace conversion needed — the source RGBA8 bytes
  # are returned as-is.  Perfect for pixel art or when you want no blurring.
  #
  # We use Kernel.round/1 (not Float.round/1) because affine matrices with
  # integer entries can produce integer u/v values; Float.round/1 only accepts
  # floats.  Kernel.round/1 handles both integers and floats uniformly.
  defp sample_nearest(img, u, v, oob) do
    xi = resolve_coord(round(u), img.width, oob)
    yi = resolve_coord(round(v), img.height, oob)

    if is_nil(xi) or is_nil(yi) do
      {0, 0, 0, 0}
    else
      PixelContainer.pixel_at(img, xi, yi)
    end
  end

  # Bilinear: 2×2 weighted blend in linear light.
  #
  # The four corner pixels are decoded to linear floats, blended with
  # bilinear weights (the fractional parts of u/v), and re-encoded.
  #
  # Conceptually: imagine the four pixels as corners of a unit square.
  # The fractional offset (fx, fy) controls how much each corner contributes.
  # This is identical to linear interpolation applied first horizontally then
  # vertically (or vice versa — bilinear is separable).
  defp sample_bilinear(img, u, v, oob) do
    # Use :math.floor/1 (accepts both int and float) rather than Float.floor/1
    # (float-only).  Affine matrices with integer entries can produce integer
    # u/v values at integer output coordinates.
    x0 = trunc(:math.floor(u))
    y0 = trunc(:math.floor(v))
    x1 = x0 + 1
    y1 = y0 + 1
    fx = u - x0
    fy = v - y0

    xi0 = resolve_coord(x0, img.width, oob)
    xi1 = resolve_coord(x1, img.width, oob)
    yi0 = resolve_coord(y0, img.height, oob)
    yi1 = resolve_coord(y1, img.height, oob)

    read = fn xi, yi ->
      if is_nil(xi) or is_nil(yi) do
        {0.0, 0.0, 0.0, 0.0}
      else
        {r, g, b, a} = PixelContainer.pixel_at(img, xi, yi)
        {decode(r), decode(g), decode(b), a / 255.0}
      end
    end

    {r00, g00, b00, a00} = read.(xi0, yi0)
    {r10, g10, b10, a10} = read.(xi1, yi0)
    {r01, g01, b01, a01} = read.(xi0, yi1)
    {r11, g11, b11, a11} = read.(xi1, yi1)

    # Bilinear weights: four corners of the unit cell
    w00 = (1 - fx) * (1 - fy)
    w10 = fx * (1 - fy)
    w01 = (1 - fx) * fy
    w11 = fx * fy

    blend = fn c00, c10, c01, c11 ->
      c00 * w00 + c10 * w10 + c01 * w01 + c11 * w11
    end

    {
      encode(blend.(r00, r10, r01, r11)),
      encode(blend.(g00, g10, g01, g11)),
      encode(blend.(b00, b10, b01, b11)),
      round(blend.(a00, a10, a01, a11) * 255)
    }
  end

  # Bicubic (Catmull-Rom): 4×4 weighted blend in linear light.
  #
  # The 4×4 neighbourhood is sampled, weighted by the Catmull-Rom kernel, and
  # blended in linear light.  We separate the 2D kernel into a horizontal pass
  # followed by a vertical pass (separability of the Catmull-Rom filter):
  #
  #   1. For each of the 4 rows, blend 4 horizontal pixels → 1 value per row.
  #   2. Blend the 4 row values vertically → final pixel.
  defp sample_bicubic(img, u, v, oob) do
    x0 = trunc(:math.floor(u))
    y0 = trunc(:math.floor(v))
    fx = u - x0
    fy = v - y0

    read_linear = fn xi, yi ->
      resolved_x = resolve_coord(xi, img.width, oob)
      resolved_y = resolve_coord(yi, img.height, oob)

      if is_nil(resolved_x) or is_nil(resolved_y) do
        {0.0, 0.0, 0.0, 0.0}
      else
        {r, g, b, a} = PixelContainer.pixel_at(img, resolved_x, resolved_y)
        {decode(r), decode(g), decode(b), a / 255.0}
      end
    end

    # For each of the 4 rows j ∈ {-1, 0, 1, 2}, horizontally blend 4 columns
    # i ∈ {-1, 0, 1, 2} using the Catmull-Rom kernel evaluated at (i - fx).
    row_blends =
      for j <- -1..2 do
        row_y = y0 + j

        {wr, wg, wb, wa} =
          Enum.reduce(-1..2, {0.0, 0.0, 0.0, 0.0}, fn i, {ar, ag, ab, aa} ->
            {lr, lg, lb, la} = read_linear.(x0 + i, row_y)
            w = catmull_rom(i - fx)
            {ar + w * lr, ag + w * lg, ab + w * lb, aa + w * la}
          end)

        {wr, wg, wb, wa}
      end

    # Now blend the 4 row results vertically with weights catmull_rom(j - fy)
    {fr, fg, fb, fa} =
      row_blends
      |> Enum.with_index(-1)
      |> Enum.reduce({0.0, 0.0, 0.0, 0.0}, fn {{lr, lg, lb, la}, j},
                                               {ar, ag, ab, aa} ->
        w = catmull_rom(j - fy)
        {ar + w * lr, ag + w * lg, ab + w * lb, aa + w * la}
      end)

    {
      encode(fr),
      encode(fg),
      encode(fb),
      round(min(1.0, max(0.0, fa)) * 255)
    }
  end

  # Dispatcher: choose sampling mode.
  defp do_sample(img, u, v, mode, oob) do
    case mode do
      :nearest -> sample_nearest(img, u, v, oob)
      :bilinear -> sample_bilinear(img, u, v, oob)
      :bicubic -> sample_bicubic(img, u, v, oob)
    end
  end

  # ── Lossless (integer) transforms ──────────────────────────────────────────
  #
  # These copy raw RGBA8 bytes directly — no arithmetic, no colorspace
  # conversion.  The `for ... into: <<>>` comprehension builds the output
  # binary one pixel at a time using pattern-matched byte extraction.

  @doc """
  Mirror left↔right.

  Each row is reversed pixel-by-pixel:

      T⁻¹(x′, y′) = (W − 1 − x′, y′)

  Applying `flip_horizontal/1` twice returns the original image exactly,
  because `(W-1) - ((W-1) - x) == x` for all valid x.

  ## Example

      iex> alias CodingAdventures.PixelContainer, as: PC
      iex> alias CodingAdventures.ImageGeometricTransforms, as: GT
      iex> img = PC.new(2, 1)
      iex> img = PC.set_pixel(img, 0, 0, 255, 0, 0, 255)  # red at (0,0)
      iex> img = PC.set_pixel(img, 1, 0,   0, 0, 255, 255) # blue at (1,0)
      iex> out = GT.flip_horizontal(img)
      iex> PC.pixel_at(out, 0, 0)
      {0, 0, 255, 255}
      iex> PC.pixel_at(out, 1, 0)
      {255, 0, 0, 255}

  """
  def flip_horizontal(%PixelContainer{} = src) do
    w = src.width
    h = src.height

    new_data =
      for y <- 0..(h - 1), x <- 0..(w - 1), into: <<>> do
        {r, g, b, a} = PixelContainer.pixel_at(src, w - 1 - x, y)
        <<r, g, b, a>>
      end

    %PixelContainer{width: w, height: h, data: new_data}
  end

  @doc """
  Mirror top↔bottom.

  Rows are reordered; bytes within each row are unchanged:

      T⁻¹(x′, y′) = (x′, H − 1 − y′)

  Applying `flip_vertical/1` twice returns the original image exactly.
  """
  def flip_vertical(%PixelContainer{} = src) do
    w = src.width
    h = src.height

    new_data =
      for y <- 0..(h - 1), x <- 0..(w - 1), into: <<>> do
        {r, g, b, a} = PixelContainer.pixel_at(src, x, h - 1 - y)
        <<r, g, b, a>>
      end

    %PixelContainer{width: w, height: h, data: new_data}
  end

  @doc """
  Rotate 90° clockwise.

  Output dimensions are swapped: W′ = H, H′ = W.

      T⁻¹(x′, y′) = (y′, W − 1 − x′)

  Visualisation (A–F are distinct pixels):

      Input:        Output (90° CW):
        A B C         D A
        D E F   →     E B
                      F C

  Four clockwise rotations return the original image.
  """
  def rotate_90_cw(%PixelContainer{} = src) do
    w = src.width
    h = src.height
    # Output dimensions: width = h (old height), height = w (old width)
    out_w = h
    out_h = w

    new_data =
      for y <- 0..(out_h - 1), x <- 0..(out_w - 1), into: <<>> do
        # Inverse map: output (x, y) came from input (y, W-1-x)
        {r, g, b, a} = PixelContainer.pixel_at(src, y, w - 1 - x)
        <<r, g, b, a>>
      end

    %PixelContainer{width: out_w, height: out_h, data: new_data}
  end

  @doc """
  Rotate 90° counter-clockwise.

  Output dimensions are swapped: W′ = H, H′ = W.

      T⁻¹(x′, y′) = (H − 1 − y′, x′)

  Four counter-clockwise rotations return the original image.
  """
  def rotate_90_ccw(%PixelContainer{} = src) do
    w = src.width
    h = src.height
    out_w = h
    out_h = w

    new_data =
      for y <- 0..(out_h - 1), x <- 0..(out_w - 1), into: <<>> do
        # Inverse map: output (x, y) came from input (H-1-y, x)
        {r, g, b, a} = PixelContainer.pixel_at(src, h - 1 - y, x)
        <<r, g, b, a>>
      end

    %PixelContainer{width: out_w, height: out_h, data: new_data}
  end

  @doc """
  Rotate 180°.

  Equivalent to `flip_horizontal(flip_vertical(src))`.

      T⁻¹(x′, y′) = (W − 1 − x′, H − 1 − y′)

  Applying `rotate_180/1` twice returns the original image exactly.
  """
  def rotate_180(%PixelContainer{} = src) do
    w = src.width
    h = src.height

    new_data =
      for y <- 0..(h - 1), x <- 0..(w - 1), into: <<>> do
        {r, g, b, a} = PixelContainer.pixel_at(src, w - 1 - x, h - 1 - y)
        <<r, g, b, a>>
      end

    %PixelContainer{width: w, height: h, data: new_data}
  end

  @doc """
  Extract a rectangular sub-region.

  The output image has dimensions `(w, h)` starting at input pixel `(x0, y0)`:

      O(x′, y′) = I(x0 + x′, y0 + y′)

  Out-of-bounds source pixels (if the crop rectangle extends beyond the image)
  are read as `{0, 0, 0, 0}` (transparent black), consistent with
  `PixelContainer.pixel_at/3`'s own OOB behaviour.

  ## Example

      iex> alias CodingAdventures.PixelContainer, as: PC
      iex> alias CodingAdventures.ImageGeometricTransforms, as: GT
      iex> img = PC.new(4, 4)
      iex> img = PC.set_pixel(img, 2, 2, 100, 150, 200, 255)
      iex> out = GT.crop(img, 2, 2, 2, 2)
      iex> out.width
      2
      iex> out.height
      2
      iex> PC.pixel_at(out, 0, 0)
      {100, 150, 200, 255}

  """
  def crop(%PixelContainer{} = src, x0, y0, w, h) do
    new_data =
      for y <- 0..(h - 1), x <- 0..(w - 1), into: <<>> do
        {r, g, b, a} = PixelContainer.pixel_at(src, x0 + x, y0 + y)
        <<r, g, b, a>>
      end

    %PixelContainer{width: w, height: h, data: new_data}
  end

  @doc """
  Add a border around the image.

  Extends the image by `top`/`right`/`bottom`/`left` pixel rows/columns, filled
  with `fill` (default: transparent black `{0, 0, 0, 0}`).

  Output dimensions: `(W + left + right) × (H + top + bottom)`.

  The original image is placed at offset `(left, top)` in the output:

      O(x′, y′):
        if x′ ∈ [left, left+W) and y′ ∈ [top, top+H):
            = I(x′ - left, y′ - top)
        else:
            = fill

  ## Example

      iex> alias CodingAdventures.PixelContainer, as: PC
      iex> alias CodingAdventures.ImageGeometricTransforms, as: GT
      iex> img = PC.new(2, 2)
      iex> img = PC.set_pixel(img, 0, 0, 255, 0, 0, 255)
      iex> out = GT.pad(img, 1, 1, 1, 1, {0, 255, 0, 255})
      iex> out.width
      4
      iex> out.height
      4
      iex> PC.pixel_at(out, 1, 1)  # was (0,0)
      {255, 0, 0, 255}
      iex> PC.pixel_at(out, 0, 0)  # border
      {0, 255, 0, 255}

  """
  def pad(%PixelContainer{} = src, top, right, bottom, left, fill \\ {0, 0, 0, 0}) do
    {fr, fg, fb, fa} = fill
    out_w = src.width + left + right
    out_h = src.height + top + bottom

    new_data =
      for y <- 0..(out_h - 1), x <- 0..(out_w - 1), into: <<>> do
        src_x = x - left
        src_y = y - top

        if src_x >= 0 and src_y >= 0 and src_x < src.width and src_y < src.height do
          {r, g, b, a} = PixelContainer.pixel_at(src, src_x, src_y)
          <<r, g, b, a>>
        else
          <<fr, fg, fb, fa>>
        end
      end

    %PixelContainer{width: out_w, height: out_h, data: new_data}
  end

  # ── Continuous transforms ───────────────────────────────────────────────────

  @doc """
  Resize the image to `(out_w, out_h)` pixels.

  Uses the **pixel-centre model**: the centre of each output pixel maps to the
  centre of the corresponding fractional input position, preventing the
  off-by-half-pixel shift that plain `x / sx` formulas produce.

  Inverse warp:

      sx = out_w / src.width
      sy = out_h / src.height
      u = (x′ + 0.5) / sx − 0.5
      v = (y′ + 0.5) / sy − 0.5

  Out-of-bounds mode is fixed to `:replicate` (clamp to edge) to prevent
  dark halos at the image boundary.

  `mode` is `:nearest`, `:bilinear` (default), or `:bicubic`.

  ## Example

      iex> alias CodingAdventures.PixelContainer, as: PC
      iex> alias CodingAdventures.ImageGeometricTransforms, as: GT
      iex> img = PC.new(4, 4)
      iex> out = GT.scale(img, 8, 8)
      iex> out.width
      8
      iex> out.height
      8

  """
  def scale(%PixelContainer{} = src, out_w, out_h, mode \\ :bilinear) do
    sx = out_w / src.width
    sy = out_h / src.height

    new_data =
      for y <- 0..(out_h - 1), x <- 0..(out_w - 1), into: <<>> do
        # Pixel-centre model: map output pixel centre to input pixel centre
        u = (x + 0.5) / sx - 0.5
        v = (y + 0.5) / sy - 0.5
        {r, g, b, a} = do_sample(src, u, v, mode, :replicate)
        <<r, g, b, a>>
      end

    %PixelContainer{width: out_w, height: out_h, data: new_data}
  end

  @doc """
  Rotate by an arbitrary angle (radians, counter-clockwise positive).

  Uses the **inverse warp**: for each output pixel, compute where it came from
  in the source via the rotation inverse:

      cx_in  = (src.width  - 1) / 2.0
      cy_in  = (src.height - 1) / 2.0
      cx_out = (out_w - 1) / 2.0
      cy_out = (out_h - 1) / 2.0

      dx = x′ − cx_out
      dy = y′ − cy_out
      u  = cx_in + cos(θ) * dx + sin(θ) * dy
      v  = cy_in − sin(θ) * dx + cos(θ) * dy

  Note: rotating by θ CCW means the inverse warp rotates by θ CW, which has
  the same cos θ on the diagonal but `+sin(θ)` on the off-diagonal (not
  `−sin(θ)` as in the forward rotation matrix).

  ## Bounds

  - `:fit` (default) — output is large enough to contain the entire rotated
    source without clipping:
    ```
    W′ = ceil(|W cos θ| + |H sin θ|)
    H′ = ceil(|W sin θ| + |H cos θ|)
    ```
  - `:crop` — output keeps the same dimensions as the input; corners are clipped.

  ## Out-of-bounds

  Background pixels (where the inverse warp falls outside the source) are
  filled with transparent black `{0, 0, 0, 0}` using the `:zero` mode.
  This lets the background show through naturally.

  `mode` is `:nearest`, `:bilinear` (default), or `:bicubic`.

  ## Example

      iex> alias CodingAdventures.PixelContainer, as: PC
      iex> alias CodingAdventures.ImageGeometricTransforms, as: GT
      iex> img = PC.new(10, 10)
      iex> out = GT.rotate(img, 0.0)
      iex> out.width
      10
      iex> out.height
      10

  """
  def rotate(%PixelContainer{} = src, radians, mode \\ :bilinear, bounds \\ :fit) do
    cos_t = :math.cos(radians)
    sin_t = :math.sin(radians)
    abs_cos = abs(cos_t)
    abs_sin = abs(sin_t)
    w = src.width
    h = src.height

    {out_w, out_h} =
      case bounds do
        :fit ->
          {
            ceil(w * abs_cos + h * abs_sin),
            ceil(w * abs_sin + h * abs_cos)
          }

        :crop ->
          {w, h}
      end

    # Pixel-centre coordinates of the centres of the source and output images
    cx_in = (w - 1) / 2.0
    cy_in = (h - 1) / 2.0
    cx_out = (out_w - 1) / 2.0
    cy_out = (out_h - 1) / 2.0

    new_data =
      for y <- 0..(out_h - 1), x <- 0..(out_w - 1), into: <<>> do
        dx = x - cx_out
        dy = y - cy_out
        u = cx_in + cos_t * dx + sin_t * dy
        v = cy_in - sin_t * dx + cos_t * dy
        {r, g, b, a} = do_sample(src, u, v, mode, :zero)
        <<r, g, b, a>>
      end

    %PixelContainer{width: out_w, height: out_h, data: new_data}
  end

  @doc """
  Apply a 2×3 affine transform (inverse warp).

  `matrix` is a tuple-of-tuples in row-major order:

      {{m00, m01, m02},
       {m10, m11, m12}}

  The matrix is already the **inverse** transform — it maps each output
  coordinate `(x′, y′)` to the corresponding source coordinate `(u, v)`:

      u = m00*x′ + m01*y′ + m02
      v = m10*x′ + m11*y′ + m12

  Common inverse affine matrices (these are already in inverse form — they
  map output → input):

      Identity:       {{1, 0, 0}, {0, 1, 0}}
      Translate −tx:  {{1, 0, tx}, {0, 1, ty}}   (shift source window)
      Scale 1/sx:     {{1/sx, 0, 0}, {0, 1/sy, 0}}

  `out_w` and `out_h` must be provided by the caller.

  `mode` is `:nearest`, `:bilinear` (default), or `:bicubic`.
  `oob` is `:zero`, `:replicate` (default), `:reflect`, or `:wrap`.

  ## Example

      iex> alias CodingAdventures.PixelContainer, as: PC
      iex> alias CodingAdventures.ImageGeometricTransforms, as: GT
      iex> img = PC.new(4, 4)
      iex> img = PC.set_pixel(img, 1, 1, 200, 100, 50, 255)
      iex> id = {{1, 0, 0}, {0, 1, 0}}
      iex> out = GT.affine(img, id, 4, 4)
      iex> PC.pixel_at(out, 1, 1)
      {200, 100, 50, 255}

  """
  def affine(%PixelContainer{} = src, matrix, out_w, out_h, mode \\ :bilinear, oob \\ :replicate) do
    {{m00, m01, m02}, {m10, m11, m12}} = matrix

    new_data =
      for y <- 0..(out_h - 1), x <- 0..(out_w - 1), into: <<>> do
        u = m00 * x + m01 * y + m02
        v = m10 * x + m11 * y + m12
        {r, g, b, a} = do_sample(src, u, v, mode, oob)
        <<r, g, b, a>>
      end

    %PixelContainer{width: out_w, height: out_h, data: new_data}
  end

  @doc """
  Apply a 3×3 perspective (projective) transform.

  `h` is the **inverse** homography matrix — it maps each output pixel `(x′, y′)`
  to a source coordinate `(u, v)` via homogeneous division:

      {{h00, h01, h02},
       {h10, h11, h12},
       {h20, h21, h22}}

      u_h = h00*x′ + h01*y′ + h02
      v_h = h10*x′ + h11*y′ + h12
      w   = h20*x′ + h21*y′ + h22

      u = u_h / w
      v = v_h / w

  When `h20 == h21 == 0` and `h22 == 1`, this reduces to an affine transform.

  **Typical use-case**: document or whiteboard de-warping — provide the
  inverse homography that maps the rectified output back to the distorted input.

  `out_w` and `out_h` must be provided by the caller.

  `mode` is `:nearest`, `:bilinear` (default), or `:bicubic`.
  `oob` is `:zero`, `:replicate` (default), `:reflect`, or `:wrap`.

  ## Example

      iex> alias CodingAdventures.PixelContainer, as: PC
      iex> alias CodingAdventures.ImageGeometricTransforms, as: GT
      iex> img = PC.new(4, 4)
      iex> img = PC.set_pixel(img, 2, 2, 128, 64, 32, 255)
      iex> # Identity perspective
      iex> id = {{1, 0, 0}, {0, 1, 0}, {0, 0, 1}}
      iex> out = GT.perspective_warp(img, id, 4, 4)
      iex> PC.pixel_at(out, 2, 2)
      {128, 64, 32, 255}

  """
  def perspective_warp(
        %PixelContainer{} = src,
        h,
        out_w,
        out_h,
        mode \\ :bilinear,
        oob \\ :replicate
      ) do
    {{h00, h01, h02}, {h10, h11, h12}, {h20, h21, h22}} = h

    new_data =
      for y <- 0..(out_h - 1), x <- 0..(out_w - 1), into: <<>> do
        # Homogeneous projection: divide by the w component
        u_h = h00 * x + h01 * y + h02
        v_h = h10 * x + h11 * y + h12
        w_h = h20 * x + h21 * y + h22

        {r, g, b, a} =
          if abs(w_h) < 1.0e-10 do
            # Degenerate: point at infinity → transparent black
            {0, 0, 0, 0}
          else
            do_sample(src, u_h / w_h, v_h / w_h, mode, oob)
          end

        <<r, g, b, a>>
      end

    %PixelContainer{width: out_w, height: out_h, data: new_data}
  end
end
