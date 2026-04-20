# frozen_string_literal: true

require "coding_adventures/pixel_container"

# =============================================================================
# CodingAdventures::ImagePointOps — IMG03 per-pixel point operations.
# =============================================================================
#
# A point operation transforms each pixel independently using only that
# pixel's own value.  No neighbouring pixels are consulted, no frequency-
# domain transform is needed.
#
# ## Two domains
#
# u8-domain operations work directly on the 8-bit sRGB bytes.  They are
# correct without colour-space conversion because they are monotone remappings
# that never mix or average channel values.
#
# Linear-light operations decode each byte to a linear-light Float, perform
# the arithmetic, then re-encode the result.  Averaging in sRGB is incorrect
# (see IMG00 §2).
#
# ## sRGB ↔ linear
#
#   Decode (u8 → Float):
#     c = byte / 255.0
#     c <= 0.04045  →  c / 12.92
#     else          →  ((c + 0.055) / 1.055) ** 2.4
#
#   Encode (Float → u8):
#     c <= 0.0031308  →  c * 12.92
#     else            →  1.055 * c ** (1/2.4) − 0.055
#     (clamp to [0,1] then multiply by 255 and round)
# =============================================================================

module CodingAdventures
  module ImagePointOps
    include CodingAdventures::PixelContainer

    VERSION = "0.1.0"

    # ── sRGB / linear LUT ─────────────────────────────────────────────────
    # 256-entry decode LUT built once at module load.
    SRGB_TO_LINEAR = Array.new(256) do |i|
      c = i / 255.0
      c <= 0.04045 ? c / 12.92 : ((c + 0.055) / 1.055)**2.4
    end.freeze

    # @param byte [Integer] sRGB byte 0–255
    # @return [Float] linear 0.0–1.0
    def self.decode(byte)
      SRGB_TO_LINEAR[byte]
    end

    # @param linear [Float] linear 0.0–1.0 (clamped)
    # @return [Integer] sRGB byte 0–255
    def self.encode(linear)
      c = if linear <= 0.0031308
            linear * 12.92
          else
            1.055 * linear**(1.0 / 2.4) - 0.055
          end
      (c.clamp(0.0, 1.0) * 255).round
    end

    # ── Iteration helper ───────────────────────────────────────────────────

    # Applies a block to every pixel, returning a new Container.
    # Yields [r, g, b, a] (all Integers 0–255) and expects the block to
    # return [r', g', b', a'] of the same shape.
    def self.map_pixels(src, &block)
      out = CodingAdventures::PixelContainer.create(src.width, src.height)
      src.height.times do |y|
        src.width.times do |x|
          pixel = CodingAdventures::PixelContainer.pixel_at(src, x, y)
          nr, ng, nb, na = block.call(*pixel)
          CodingAdventures::PixelContainer.set_pixel(out, x, y, nr, ng, nb, na)
        end
      end
      out
    end

    # ── u8-domain operations ───────────────────────────────────────────────

    # Invert: flip each RGB channel (255 − v).  Alpha preserved.
    def self.invert(src)
      map_pixels(src) { |r, g, b, a| [255 - r, 255 - g, 255 - b, a] }
    end

    # Threshold: (r+g+b)/3 >= value → white, else black.
    def self.threshold(src, value)
      map_pixels(src) do |r, g, b, a|
        v = (r + g + b) / 3 >= value ? 255 : 0
        [v, v, v, a]
      end
    end

    # Threshold on Rec. 709 luma: Y = 0.2126 R + 0.7152 G + 0.0722 B.
    def self.threshold_luminance(src, value)
      map_pixels(src) do |r, g, b, a|
        luma = 0.2126 * r + 0.7152 * g + 0.0722 * b
        v = luma >= value ? 255 : 0
        [v, v, v, a]
      end
    end

    # Posterize: reduce each channel to `levels` equally-spaced steps.
    def self.posterize(src, levels)
      step = 255.0 / (levels - 1)
      q = ->(v) { ((v / step).round * step).round }
      map_pixels(src) { |r, g, b, a| [q.call(r), q.call(g), q.call(b), a] }
    end

    # Swap R and B channels (RGB ↔ BGR).
    def self.swap_rgb_bgr(src)
      map_pixels(src) { |r, g, b, a| [b, g, r, a] }
    end

    # Extract one channel (0=R, 1=G, 2=B, 3=A), zero the rest.
    def self.extract_channel(src, channel)
      map_pixels(src) do |r, g, b, a|
        case channel
        when 0 then [r, 0, 0, a]
        when 1 then [0, g, 0, a]
        when 2 then [0, 0, b, a]
        else [r, g, b, a]
        end
      end
    end

    # Additive brightness: add signed offset to each channel, clamp [0, 255].
    def self.brightness(src, offset)
      clamp = ->(v) { (v + offset).clamp(0, 255) }
      map_pixels(src) { |r, g, b, a| [clamp.call(r), clamp.call(g), clamp.call(b), a] }
    end

    # ── Linear-light operations ────────────────────────────────────────────

    # Contrast: scale around linear mid-grey 0.5.
    def self.contrast(src, factor)
      map_pixels(src) do |r, g, b, a|
        [
          encode(0.5 + factor * (decode(r) - 0.5)),
          encode(0.5 + factor * (decode(g) - 0.5)),
          encode(0.5 + factor * (decode(b) - 0.5)),
          a
        ]
      end
    end

    # Gamma: apply power-law γ in linear light.
    def self.gamma(src, g)
      map_pixels(src) do |r, gv, b, a|
        [encode(decode(r)**g), encode(decode(gv)**g), encode(decode(b)**g), a]
      end
    end

    # Exposure: multiply linear by 2^stops.
    def self.exposure(src, stops)
      factor = 2.0**stops
      map_pixels(src) do |r, g, b, a|
        [encode(decode(r) * factor), encode(decode(g) * factor), encode(decode(b) * factor), a]
      end
    end

    # Greyscale method symbols: :rec709, :bt601, :average.
    GREYSCALE_WEIGHTS = {
      rec709:  [0.2126, 0.7152, 0.0722],
      bt601:   [0.2989, 0.5870, 0.1140],
      average: [1.0 / 3, 1.0 / 3, 1.0 / 3]
    }.freeze

    # Greyscale: convert to luminance in linear light.
    def self.greyscale(src, method = :rec709)
      wr, wg, wb = GREYSCALE_WEIGHTS.fetch(method)
      map_pixels(src) do |r, g, b, a|
        y = encode(wr * decode(r) + wg * decode(g) + wb * decode(b))
        [y, y, y, a]
      end
    end

    # Sepia: classic warm sepia tone matrix.
    def self.sepia(src)
      map_pixels(src) do |r, g, b, a|
        lr, lg, lb = decode(r), decode(g), decode(b)
        [
          encode(0.393 * lr + 0.769 * lg + 0.189 * lb),
          encode(0.349 * lr + 0.686 * lg + 0.168 * lb),
          encode(0.272 * lr + 0.534 * lg + 0.131 * lb),
          a
        ]
      end
    end

    # Colour matrix: multiply linear [R, G, B] by 3×3 matrix (row-major array).
    # matrix = [[m00,m01,m02],[m10,m11,m12],[m20,m21,m22]]
    def self.colour_matrix(src, matrix)
      m = matrix
      map_pixels(src) do |r, g, b, a|
        lr, lg, lb = decode(r), decode(g), decode(b)
        [
          encode(m[0][0] * lr + m[0][1] * lg + m[0][2] * lb),
          encode(m[1][0] * lr + m[1][1] * lg + m[1][2] * lb),
          encode(m[2][0] * lr + m[2][1] * lg + m[2][2] * lb),
          a
        ]
      end
    end

    # Saturate: 0 → greyscale; 1 → identity; >1 → vivid.
    def self.saturate(src, factor)
      map_pixels(src) do |r, g, b, a|
        lr, lg, lb = decode(r), decode(g), decode(b)
        grey = 0.2126 * lr + 0.7152 * lg + 0.0722 * lb
        [
          encode(grey + factor * (lr - grey)),
          encode(grey + factor * (lg - grey)),
          encode(grey + factor * (lb - grey)),
          a
        ]
      end
    end

    # ── HSV helpers ────────────────────────────────────────────────────────

    def self.rgb_to_hsv(r, g, b)
      mx = [r, g, b].max
      mn = [r, g, b].min
      delta = mx - mn
      v = mx
      s = mx.zero? ? 0.0 : delta / mx
      h = 0.0
      unless delta.zero?
        h = if mx == r
              ((g - b) / delta) % 6
            elsif mx == g
              (b - r) / delta + 2
            else
              (r - g) / delta + 4
            end
        h = (h * 60 + 360) % 360
      end
      [h, s, v]
    end

    def self.hsv_to_rgb(h, s, v)
      c = v * s
      x = c * (1 - ((h / 60.0) % 2 - 1).abs)
      m = v - c
      r, g, b = case h.to_i / 60
                when 0 then [c, x, 0.0]
                when 1 then [x, c, 0.0]
                when 2 then [0.0, c, x]
                when 3 then [0.0, x, c]
                when 4 then [x, 0.0, c]
                else [c, 0.0, x]
                end
      [r + m, g + m, b + m]
    end

    # Hue rotate: rotate hue by degrees in linear-light HSV space.
    def self.hue_rotate(src, degrees)
      map_pixels(src) do |r, g, b, a|
        h, s, v = rgb_to_hsv(decode(r), decode(g), decode(b))
        nr, ng, nb = hsv_to_rgb((h + degrees + 360) % 360, s, v)
        [encode(nr), encode(ng), encode(nb), a]
      end
    end

    # ── Colorspace utilities ───────────────────────────────────────────────

    # Convert sRGB → linear (each byte becomes linear * 255 rounded).
    def self.srgb_to_linear_image(src)
      map_pixels(src) do |r, g, b, a|
        [(decode(r) * 255).round, (decode(g) * 255).round, (decode(b) * 255).round, a]
      end
    end

    # Convert linear → sRGB (inverse of srgb_to_linear_image).
    def self.linear_to_srgb_image(src)
      map_pixels(src) { |r, g, b, a| [encode(r / 255.0), encode(g / 255.0), encode(b / 255.0), a] }
    end

    # ── 1D LUT operations ──────────────────────────────────────────────────

    # Apply three 256-entry u8→u8 LUTs (one per channel).  Alpha preserved.
    def self.apply_lut1d_u8(src, lut_r, lut_g, lut_b)
      map_pixels(src) { |r, g, b, a| [lut_r[r], lut_g[g], lut_b[b], a] }
    end

    # Build a 256-entry LUT from a linear-light mapping function.
    # `fn` receives a linear Float and returns a linear Float.
    def self.build_lut1d_u8(&fn)
      Array.new(256) { |i| encode(fn.call(decode(i))) }
    end

    # Build a gamma LUT (equivalent to build_lut1d_u8 { |v| v ** g }).
    def self.build_gamma_lut(g)
      build_lut1d_u8 { |v| v**g }
    end
  end
end
