# frozen_string_literal: true

require "coding_adventures/pixel_container"

# =============================================================================
# CodingAdventures::ImageGeometricTransforms — IMG04 geometric transforms.
# =============================================================================
#
# A geometric transform maps every pixel in an output image back to a
# (possibly fractional) source coordinate in the input image, samples that
# location, and writes the result.  This "backward mapping" approach avoids
# holes and double-sampling artefacts that would occur if we pushed each input
# pixel forward.
#
# ## Coordinate conventions
#
# All pixel coordinates use the "pixel-centre" model:
#
#   pixel (x, y) occupies the unit square centred at (x + 0.5, y + 0.5)
#   within the image's [0, W] × [0, H] continuous domain.
#
# The centre of a W×H image therefore lies at (W/2.0, H/2.0).
#
# ## sRGB ↔ linear light
#
# Whenever pixels are blended (bilinear or bicubic interpolation), we must
# first convert from the non-linear sRGB encoding to linear light, perform
# the weighted average, then convert back.  Blending in sRGB produces
# visually dark artefacts at transitions (the "too-dark midpoint" problem).
#
# The transforms that copy entire pixels without mixing (flip, rotate-90,
# crop, pad) are lossless raw-byte copies that bypass sRGB conversion.
#
# ## Out-of-bounds strategies (oob)
#
#   :zero       — pixels outside the image are transparent black [0,0,0,0]
#   :replicate  — clamp coordinates to the nearest edge pixel
#   :reflect    — mirror at each edge (useful for filter borders)
#   :wrap       — tile the image periodically
#
# ## Interpolation modes
#
#   :nearest    — snap to the nearest integer pixel (fast, blocky on scale-up)
#   :bilinear   — 2×2 weighted average (smooth, some blur)
#   :bicubic    — 4×4 Catmull-Rom spline (sharper edges than bilinear)
#
# ## Operations
#
# Lossless (raw-byte copy, no sRGB conversion):
#   flip_horizontal, flip_vertical, rotate_90_cw, rotate_90_ccw,
#   rotate_180, crop, pad
#
# Continuous (backward mapping with interpolation):
#   scale, rotate, affine, perspective_warp
# =============================================================================

module CodingAdventures
  module ImageGeometricTransforms
    PC = CodingAdventures::PixelContainer

    VERSION = "0.1.0"

    # ── sRGB ↔ linear-light ───────────────────────────────────────────────────
    #
    # We precompute a 256-entry lookup table for sRGB→linear decoding.
    # The piecewise formula from the IEC 61966-2-1 standard:
    #
    #   If c/255 ≤ 0.04045:  linear = c / 255 / 12.92
    #   Otherwise:           linear = ((c/255 + 0.055) / 1.055) ^ 2.4
    #
    # The LUT saves ~256 floating-point exponentiations per pixel compared
    # to calling the formula directly.
    SRGB_TO_LINEAR = (0..255).map { |i|
      c = i / 255.0
      c <= 0.04045 ? c / 12.92 : ((c + 0.055) / 1.055)**2.4
    }.freeze

    # ── Private helpers ───────────────────────────────────────────────────────
    # These are internal utilities not part of the public API.
    # We define them as module_function so they can be called with
    # `decode(b)` inside the module without `self.`.

    class << self
      private

      # -----------------------------------------------------------------------
      # decode(b) → Float
      #
      # Convert a single sRGB byte (0..255) to a linear-light Float (0.0..1.0).
      # Uses the precomputed LUT for speed.
      # -----------------------------------------------------------------------
      def decode(b)
        SRGB_TO_LINEAR[b]
      end

      # -----------------------------------------------------------------------
      # encode(v) → Integer
      #
      # Convert a linear-light Float (0.0..1.0) back to an sRGB byte (0..255).
      # The inverse of decode.  Clamps out-of-range values from interpolation
      # overshoot (Catmull-Rom can produce values slightly outside [0,1]).
      #
      #   If v ≤ 0.0031308:  sRGB = v * 12.92
      #   Otherwise:         sRGB = 1.055 * v^(1/2.4) − 0.055
      # -----------------------------------------------------------------------
      def encode(v)
        v = 0.0 if v < 0.0
        v = 1.0 if v > 1.0
        c = if v <= 0.0031308
              v * 12.92
            else
              1.055 * v**(1.0 / 2.4) - 0.055
            end
        (c * 255.0).round
      end

      # -----------------------------------------------------------------------
      # resolve_coord(x, max, oob) → Integer or nil
      #
      # Map a (possibly out-of-bounds) integer coordinate to a valid source
      # index in [0, max-1], or return nil if the pixel should be transparent.
      #
      # Parameters:
      #   x   — the (integer) coordinate to resolve
      #   max — the dimension length (width or height)
      #   oob — one of :zero, :replicate, :reflect, :wrap
      #
      # Strategy details:
      #
      #   :zero      — outside range → nil (caller returns [0,0,0,0])
      #   :replicate — clamp to the nearest valid index
      #   :reflect   — mirror about each edge; the period is 2*max
      #                so pixel 0 and pixel max-1 are "wall" pixels
      #   :wrap      — modulo arithmetic; Ruby's % always returns
      #                non-negative when the divisor is positive
      # -----------------------------------------------------------------------
      def resolve_coord(x, max, oob)
        case oob
        when :zero
          return nil if x < 0 || x >= max
          x
        when :replicate
          x.clamp(0, max - 1)
        when :reflect
          # Fold x into [0, 2*max) and then map the upper half back.
          period = 2 * max
          x = x % period
          x += period if x < 0   # Ruby % handles negatives, but belt-and-suspenders
          x = period - 1 - x if x >= max
          x
        when :wrap
          x % max  # Ruby's modulo is always non-negative for positive max
        end
      end

      # -----------------------------------------------------------------------
      # catmull_rom(d) → Float
      #
      # Catmull-Rom spline weight for a neighbour at distance |d| from the
      # sample point.  The piecewise cubic is:
      #
      #   |d| < 1:  1.5*|d|^3 − 2.5*|d|^2 + 1
      #   |d| < 2: -0.5*|d|^3 + 2.5*|d|^2 − 4*|d| + 2
      #   else:     0
      #
      # The weights for the 4 neighbours at offsets -1, 0, +1, +2 (relative
      # to the floor of u) sum to exactly 1.0 when no extrapolation occurs.
      # -----------------------------------------------------------------------
      def catmull_rom(d)
        d = d.abs
        if d < 1.0
          1.5 * d**3 - 2.5 * d**2 + 1.0
        elsif d < 2.0
          -0.5 * d**3 + 2.5 * d**2 - 4.0 * d + 2.0
        else
          0.0
        end
      end

      # -----------------------------------------------------------------------
      # sample_nearest(img, u, v, oob) → [r, g, b, a]
      #
      # Round the continuous coordinates (u, v) to the nearest integer pixel
      # and return its raw RGBA bytes.  No sRGB conversion is needed because
      # we copy an exact pixel value unchanged.
      #
      # Parameters:
      #   img — PixelContainer
      #   u   — continuous x coordinate in image space
      #   v   — continuous y coordinate in image space
      #   oob — out-of-bounds mode
      # -----------------------------------------------------------------------
      def sample_nearest(img, u, v, oob)
        xi = u.round
        yi = v.round
        rx = resolve_coord(xi, img.width,  oob)
        ry = resolve_coord(yi, img.height, oob)
        return [0, 0, 0, 0] if rx.nil? || ry.nil?
        PC.pixel_at(img, rx, ry)
      end

      # -----------------------------------------------------------------------
      # sample_bilinear(img, u, v, oob) → [r, g, b, a]
      #
      # Bilinear interpolation: sample a 2×2 grid of neighbours and compute
      # a weighted average in linear light.
      #
      # Let (x0, y0) = floor of (u, v).  The fractional part (fx, fy) gives
      # the weights for the four corners:
      #
      #   (1-fx)(1-fy) · I[x0,  y0  ]
      #   fx   (1-fy) · I[x0+1, y0  ]
      #   (1-fx)  fy  · I[x0,   y0+1]
      #   fx      fy  · I[x0+1, y0+1]
      #
      # RGB channels are blended in linear light; alpha is blended linearly
      # (alpha is never gamma-encoded).
      # -----------------------------------------------------------------------
      def sample_bilinear(img, u, v, oob)
        x0 = u.floor
        y0 = v.floor
        fx = u - x0
        fy = v - y0

        # Gather 2×2 neighbourhood (may hit OOB handling)
        corners = [
          [x0,     y0    ],
          [x0 + 1, y0    ],
          [x0,     y0 + 1],
          [x0 + 1, y0 + 1]
        ]
        weights = [
          (1.0 - fx) * (1.0 - fy),
          fx         * (1.0 - fy),
          (1.0 - fx) * fy,
          fx         * fy
        ]

        r_lin = g_lin = b_lin = a_lin = 0.0
        corners.each_with_index do |(cx, cy), k|
          rx = resolve_coord(cx, img.width,  oob)
          ry = resolve_coord(cy, img.height, oob)
          pix = (rx.nil? || ry.nil?) ? [0, 0, 0, 0] : PC.pixel_at(img, rx, ry)
          w = weights[k]
          r_lin += decode(pix[0]) * w
          g_lin += decode(pix[1]) * w
          b_lin += decode(pix[2]) * w
          a_lin += pix[3] * w    # alpha is linear by convention
        end

        [encode(r_lin), encode(g_lin), encode(b_lin), a_lin.round.clamp(0, 255)]
      end

      # -----------------------------------------------------------------------
      # sample_bicubic(img, u, v, oob) → [r, g, b, a]
      #
      # Bicubic (Catmull-Rom) interpolation: sample a 4×4 grid of neighbours.
      #
      # Let (x0, y0) = floor of (u, v).  The four column offsets relative to
      # x0 are {-1, 0, +1, +2}, same for rows.  The Catmull-Rom weight for
      # column offset dx is catmull_rom(fx - dx) where fx = u - x0.
      #
      # The weights are separable: w(x,y) = wx(dx) · wy(dy), so we can
      # first compute a 4-element row blend, then blend the four rows.
      #
      # Like bilinear, RGB is blended in linear light; alpha is linear.
      # -----------------------------------------------------------------------
      def sample_bicubic(img, u, v, oob)
        x0 = u.floor
        y0 = v.floor
        fx = u - x0
        fy = v - y0

        # Catmull-Rom weights for the 4 neighbours in each axis.
        # Offsets: -1, 0, +1, +2  → distances from sample: fx+1, fx, fx-1, fx-2
        wx = Array.new(4) { |k| catmull_rom(fx - (k - 1)) }
        wy = Array.new(4) { |k| catmull_rom(fy - (k - 1)) }

        r_acc = g_acc = b_acc = a_acc = 0.0
        4.times do |row|
          cy = y0 + row - 1
          ry = resolve_coord(cy, img.height, oob)
          row_r = row_g = row_b = row_a = 0.0
          4.times do |col|
            cx = x0 + col - 1
            rx = resolve_coord(cx, img.width, oob)
            pix = (rx.nil? || ry.nil?) ? [0, 0, 0, 0] : PC.pixel_at(img, rx, ry)
            w = wx[col]
            row_r += decode(pix[0]) * w
            row_g += decode(pix[1]) * w
            row_b += decode(pix[2]) * w
            row_a += pix[3] * w
          end
          wy_row = wy[row]
          r_acc += row_r * wy_row
          g_acc += row_g * wy_row
          b_acc += row_b * wy_row
          a_acc += row_a * wy_row
        end

        [encode(r_acc), encode(g_acc), encode(b_acc), a_acc.round.clamp(0, 255)]
      end

      # -----------------------------------------------------------------------
      # sample(img, u, v, mode, oob) → [r, g, b, a]
      #
      # Dispatch table for the three interpolation modes.
      # -----------------------------------------------------------------------
      def sample(img, u, v, mode, oob)
        case mode
        when :nearest  then sample_nearest(img,  u, v, oob)
        when :bilinear then sample_bilinear(img, u, v, oob)
        when :bicubic  then sample_bicubic(img,  u, v, oob)
        else raise ArgumentError, "unknown interpolation mode: #{mode.inspect}"
        end
      end
    end

    # =========================================================================
    # PUBLIC API — lossless transforms
    #
    # These operations copy pixel bytes without any sRGB conversion.
    # They are exact and reversible.
    # =========================================================================

    # -------------------------------------------------------------------------
    # flip_horizontal(src) → PixelContainer
    #
    # Mirror the image left-to-right.  Row y of the output is the reverse of
    # row y of the input.
    #
    #   I[x, y]  →  O[W-1-x, y]
    #
    # Equivalently, for each row we write column W-1-x of the input into
    # column x of the output.
    # -------------------------------------------------------------------------
    def self.flip_horizontal(src)
      dst = PC.create(src.width, src.height)
      src.height.times do |y|
        src.width.times do |x|
          r, g, b, a = PC.pixel_at(src, x, y)
          PC.set_pixel(dst, src.width - 1 - x, y, r, g, b, a)
        end
      end
      dst
    end

    # -------------------------------------------------------------------------
    # flip_vertical(src) → PixelContainer
    #
    # Mirror the image top-to-bottom.  Column x of the output is the reverse
    # of column x of the input.
    #
    #   I[x, y]  →  O[x, H-1-y]
    # -------------------------------------------------------------------------
    def self.flip_vertical(src)
      dst = PC.create(src.width, src.height)
      src.height.times do |y|
        src.width.times do |x|
          r, g, b, a = PC.pixel_at(src, x, y)
          PC.set_pixel(dst, x, src.height - 1 - y, r, g, b, a)
        end
      end
      dst
    end

    # -------------------------------------------------------------------------
    # rotate_90_cw(src) → PixelContainer
    #
    # Rotate 90 degrees clockwise.  The output dimensions swap: W'=H, H'=W.
    #
    # Derivation — rotating the unit square 90° CW maps (x,y) → (H-1-y, x):
    #
    #   (0,0) → (H-1, 0)   top-left  → top-right
    #   (W-1,0) → (H-1, W-1)
    #
    # Written as a backward-mapping read: output pixel (x', y') comes from
    # input pixel (y', W-1-x').
    #
    #   O[x', y'] = I[y', W-1-x']   where W' = H, H' = W
    # -------------------------------------------------------------------------
    def self.rotate_90_cw(src)
      out_w = src.height
      out_h = src.width
      dst = PC.create(out_w, out_h)
      out_h.times do |yp|
        out_w.times do |xp|
          r, g, b, a = PC.pixel_at(src, yp, src.width - 1 - xp)
          PC.set_pixel(dst, xp, yp, r, g, b, a)
        end
      end
      dst
    end

    # -------------------------------------------------------------------------
    # rotate_90_ccw(src) → PixelContainer
    #
    # Rotate 90 degrees counter-clockwise.  W'=H, H'=W.
    #
    # Backward mapping: O[x', y'] = I[H-1-y', x']
    #   (0,0) → (0, H-1)   top-left → bottom-left
    # -------------------------------------------------------------------------
    def self.rotate_90_ccw(src)
      out_w = src.height
      out_h = src.width
      dst = PC.create(out_w, out_h)
      out_h.times do |yp|
        out_w.times do |xp|
          r, g, b, a = PC.pixel_at(src, src.height - 1 - yp, xp)
          PC.set_pixel(dst, xp, yp, r, g, b, a)
        end
      end
      dst
    end

    # -------------------------------------------------------------------------
    # rotate_180(src) → PixelContainer
    #
    # Rotate 180 degrees.  Same dimensions as input.
    #
    # Equivalent to flip_horizontal ∘ flip_vertical (commutative).
    # Backward mapping: O[x, y] = I[W-1-x, H-1-y]
    # -------------------------------------------------------------------------
    def self.rotate_180(src)
      dst = PC.create(src.width, src.height)
      src.height.times do |y|
        src.width.times do |x|
          r, g, b, a = PC.pixel_at(src, src.width - 1 - x, src.height - 1 - y)
          PC.set_pixel(dst, x, y, r, g, b, a)
        end
      end
      dst
    end

    # -------------------------------------------------------------------------
    # crop(src, x0, y0, w, h) → PixelContainer
    #
    # Extract a w×h rectangle with its top-left corner at (x0, y0).
    # Pixels outside the source image boundary read as [0,0,0,0] (zero OOB).
    # -------------------------------------------------------------------------
    def self.crop(src, x0, y0, w, h)
      raise ArgumentError, "crop width must be positive"  unless w > 0
      raise ArgumentError, "crop height must be positive" unless h > 0
      dst = PC.create(w, h)
      h.times do |dy|
        w.times do |dx|
          # pixel_at already returns [0,0,0,0] for OOB coordinates
          r, g, b, a = PC.pixel_at(src, x0 + dx, y0 + dy)
          PC.set_pixel(dst, dx, dy, r, g, b, a)
        end
      end
      dst
    end

    # -------------------------------------------------------------------------
    # pad(src, top, right, bottom, left, fill: [0,0,0,0]) → PixelContainer
    #
    # Add a border of fill pixels around the image.  Output dimensions:
    #   W' = left + W + right
    #   H' = top  + H + bottom
    #
    # Interior pixels (those that map to a valid source coordinate) are copied
    # verbatim; border pixels receive the fill colour.
    # -------------------------------------------------------------------------
    def self.pad(src, top, right, bottom, left, fill: [0, 0, 0, 0])
      out_w = left + src.width  + right
      out_h = top  + src.height + bottom
      fr, fg, fb, fa = fill
      dst = PC.create(out_w, out_h)
      out_h.times do |y|
        out_w.times do |x|
          sx = x - left
          sy = y - top
          if sx >= 0 && sx < src.width && sy >= 0 && sy < src.height
            r, g, b, a = PC.pixel_at(src, sx, sy)
            PC.set_pixel(dst, x, y, r, g, b, a)
          else
            PC.set_pixel(dst, x, y, fr, fg, fb, fa)
          end
        end
      end
      dst
    end

    # =========================================================================
    # PUBLIC API — continuous transforms
    #
    # All use backward mapping: for each output pixel we compute a floating-
    # point source coordinate and sample the input with the chosen interpolator.
    # =========================================================================

    # -------------------------------------------------------------------------
    # scale(src, out_w, out_h, mode: :bilinear) → PixelContainer
    #
    # Resize the image to out_w × out_h pixels using the pixel-centre model.
    #
    # Pixel-centre mapping:
    #   The output pixel at (x, y) corresponds to the input location
    #   u = (x + 0.5) * (W / out_w) − 0.5
    #   v = (y + 0.5) * (H / out_h) − 0.5
    #
    # This ensures that:
    #   • The top-left output pixel maps to the top-left input pixel.
    #   • The bottom-right output pixel maps to the bottom-right input pixel.
    #   • Upscaling and downscaling are both handled symmetrically.
    #
    # OOB strategy: :replicate (edge pixels are extended, no black borders).
    # -------------------------------------------------------------------------
    def self.scale(src, out_w, out_h, mode: :bilinear)
      raise ArgumentError, "output width must be positive"  unless out_w > 0
      raise ArgumentError, "output height must be positive" unless out_h > 0
      dst = PC.create(out_w, out_h)
      x_ratio = src.width.to_f  / out_w
      y_ratio = src.height.to_f / out_h
      out_h.times do |y|
        out_w.times do |x|
          u = (x + 0.5) * x_ratio - 0.5
          v = (y + 0.5) * y_ratio - 0.5
          r, g, b, a = sample(src, u, v, mode, :replicate)
          PC.set_pixel(dst, x, y, r, g, b, a)
        end
      end
      dst
    end

    # -------------------------------------------------------------------------
    # rotate(src, radians, mode: :bilinear, bounds: :fit) → PixelContainer
    #
    # Rotate the image counter-clockwise by `radians` about its centre.
    #
    # bounds:
    #   :fit  — output canvas is sized to contain the entire rotated image
    #           (no clipping).  New areas outside the original image are zero.
    #   :crop — output canvas matches the input size; corners may be clipped.
    #
    # Backward mapping derivation:
    #   Let cx_in  = (W-1)/2.0,  cy_in  = (H-1)/2.0
    #       cx_out = (W'-1)/2.0, cy_out = (H'-1)/2.0
    #   For output pixel (x', y'):
    #     dx = x' − cx_out,  dy = y' − cy_out
    #     u  = cx_in + cos·dx + sin·dy
    #     v  = cy_in − sin·dx + cos·dy
    #
    #   (Rotating the output offset by −θ to find the input source.)
    #
    # :fit canvas size:
    #   W' = ceil(W·|cos| + H·|sin|)
    #   H' = ceil(W·|sin| + H·|cos|)
    # -------------------------------------------------------------------------
    def self.rotate(src, radians, mode: :bilinear, bounds: :fit)
      cos_r = Math.cos(radians)
      sin_r = Math.sin(radians)
      w     = src.width
      h     = src.height

      if bounds == :fit
        out_w = (w * cos_r.abs + h * sin_r.abs).ceil
        out_h = (w * sin_r.abs + h * cos_r.abs).ceil
      else
        out_w = w
        out_h = h
      end

      cx_in  = (w - 1) / 2.0
      cy_in  = (h - 1) / 2.0
      cx_out = (out_w - 1) / 2.0
      cy_out = (out_h - 1) / 2.0

      dst = PC.create(out_w, out_h)
      out_h.times do |yp|
        out_w.times do |xp|
          dx = xp - cx_out
          dy = yp - cy_out
          u  = cx_in + cos_r * dx + sin_r * dy
          v  = cy_in - sin_r * dx + cos_r * dy
          r, g, b, a = sample(src, u, v, mode, :zero)
          PC.set_pixel(dst, xp, yp, r, g, b, a)
        end
      end
      dst
    end

    # -------------------------------------------------------------------------
    # affine(src, matrix, out_w, out_h, mode: :bilinear, oob: :replicate)
    #   → PixelContainer
    #
    # Apply a 2D affine transform expressed as a 2×3 matrix:
    #
    #   matrix = [[m00, m01, m02],
    #             [m10, m11, m12]]
    #
    # The forward mapping is:
    #   x' = m00·x + m01·y + m02
    #   y' = m10·x + m11·y + m12
    #
    # We need the backward mapping (output → input), so we solve the 2×2 linear
    # system.  Given output pixel (xp, yp):
    #
    #   [m00 m01] [u]   [xp - m02]
    #   [m10 m11] [v] = [yp - m12]
    #
    # det = m00·m11 − m01·m10
    # u   = (m11·(xp−m02) − m01·(yp−m12)) / det
    # v   = (m00·(yp−m12) − m10·(xp−m02)) / det
    #
    # If det ≈ 0 (singular matrix), the result is undefined; we return
    # transparent pixels.
    # -------------------------------------------------------------------------
    def self.affine(src, matrix, out_w, out_h, mode: :bilinear, oob: :replicate)
      m00, m01, m02 = matrix[0]
      m10, m11, m12 = matrix[1]
      det = m00 * m11 - m01 * m10
      dst = PC.create(out_w, out_h)
      out_h.times do |yp|
        out_w.times do |xp|
          if det.abs < 1e-10
            PC.set_pixel(dst, xp, yp, 0, 0, 0, 0)
            next
          end
          dx = xp - m02
          dy = yp - m12
          u = (m11 * dx - m01 * dy) / det
          v = (m00 * dy - m10 * dx) / det
          r, g, b, a = sample(src, u, v, mode, oob)
          PC.set_pixel(dst, xp, yp, r, g, b, a)
        end
      end
      dst
    end

    # -------------------------------------------------------------------------
    # perspective_warp(src, h, out_w, out_h, mode: :bilinear, oob: :replicate)
    #   → PixelContainer
    #
    # Apply a projective (homographic) transform defined by a 3×3 homography
    # matrix H:
    #
    #   h = [[h00, h01, h02],
    #        [h10, h11, h12],
    #        [h20, h21, h22]]
    #
    # Forward mapping of input point (x, y) to output (x', y'):
    #   w̃  = h20·x + h21·y + h22
    #   x' = (h00·x + h01·y + h02) / w̃
    #   y' = (h10·x + h11·y + h12) / w̃
    #
    # For backward mapping we invert H and apply the same formula.
    #
    # 3×3 matrix inverse (Cramer's rule / adjugate / determinant):
    #   We compute the adjugate and divide by the determinant.
    # -------------------------------------------------------------------------
    def self.perspective_warp(src, h_mat, out_w, out_h, mode: :bilinear, oob: :replicate)
      # Unpack the 3×3 homography.
      h00, h01, h02 = h_mat[0]
      h10, h11, h12 = h_mat[1]
      h20, h21, h22 = h_mat[2]

      # Compute the determinant.
      det = h00 * (h11 * h22 - h12 * h21) \
          - h01 * (h10 * h22 - h12 * h20) \
          + h02 * (h10 * h21 - h11 * h20)

      dst = PC.create(out_w, out_h)

      if det.abs < 1e-10
        # Singular matrix — return all transparent.
        return dst
      end

      # Adjugate (transpose of cofactor matrix).
      inv_det = 1.0 / det
      i00 = ( h11 * h22 - h12 * h21) * inv_det
      i01 = (-h01 * h22 + h02 * h21) * inv_det
      i02 = ( h01 * h12 - h02 * h11) * inv_det
      i10 = (-h10 * h22 + h12 * h20) * inv_det
      i11 = ( h00 * h22 - h02 * h20) * inv_det
      i12 = (-h00 * h12 + h02 * h10) * inv_det
      i20 = ( h10 * h21 - h11 * h20) * inv_det
      i21 = (-h00 * h21 + h01 * h20) * inv_det
      i22 = ( h00 * h11 - h01 * h10) * inv_det

      out_h.times do |yp|
        out_w.times do |xp|
          # Apply inverse homography H^{-1} to map output → input.
          w_tilde = i20 * xp + i21 * yp + i22
          if w_tilde.abs < 1e-10
            PC.set_pixel(dst, xp, yp, 0, 0, 0, 0)
            next
          end
          u = (i00 * xp + i01 * yp + i02) / w_tilde
          v = (i10 * xp + i11 * yp + i12) / w_tilde
          r, g, b, a = sample(src, u, v, mode, oob)
          PC.set_pixel(dst, xp, yp, r, g, b, a)
        end
      end
      dst
    end
  end
end
