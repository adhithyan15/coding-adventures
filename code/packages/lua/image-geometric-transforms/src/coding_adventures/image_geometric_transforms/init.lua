-- image_geometric_transforms — IMG04: Geometric transforms on PixelContainer
--
-- A geometric transform repositions pixels in 2-D space.  Unlike point
-- operations (which examine one pixel in isolation), geometric transforms
-- need to know *where* a pixel came from, compute its new location, and
-- decide what colour to assign when the transformed coordinates fall between
-- grid positions (interpolation).
--
-- ## Taxonomy of transforms
--
-- We split operations into two families:
--
--   1. LOSSLESS (exact, integer-domain)
--      Flip horizontal/vertical, rotate by multiples of 90°, crop, pad.
--      No interpolation, no sRGB conversion — raw bytes are copied directly.
--      These are pixel-exact: applying the inverse recovers the original image.
--
--   2. CONTINUOUS (real-valued, require sampling)
--      Scale, arbitrary-angle rotate, affine, perspective warp.
--      Each output pixel is mapped backward into the source coordinate system
--      (inverse mapping) and the source colour is reconstructed by a chosen
--      interpolation filter.
--
-- ## Inverse mapping
--
--   Forward mapping asks "where does source pixel P end up?" — this creates
--   holes and overlaps in the output.  Inverse mapping asks "for each output
--   pixel Q, where in the source does it come from?" — guaranteed coverage.
--   All continuous transforms in this module use inverse mapping.
--
-- ## Pixel-centre model
--
--   We use the pixel-centre convention: pixel (i, j) covers the unit square
--   centred at (i + 0.5, j + 0.5) when expressed in continuous coordinates.
--   This makes scale, rotate, and affine behave consistently with most
--   industry tools (PIL/Pillow, stb_image_resize, …).
--
-- ## sRGB and linear light
--
--   Bilinear and bicubic interpolation blend colours.  Blending sRGB bytes
--   directly produces the "dark edge" artefact: the arithmetic mean of two
--   sRGB values is brighter than the perceptually correct average because
--   sRGB encodes dark values with more precision than a linear scale.
--
--   We decode to linear light before blending and re-encode after:
--
--     c = byte / 255
--     linear = c / 12.92                     if c <= 0.04045
--             = ((c + 0.055) / 1.055)^2.4    otherwise
--
--     encoded = 12.92 * linear               if linear <= 0.0031308
--              = 1.055 * linear^(1/2.4) − 0.055  otherwise
--     byte = clamp(round(encoded * 255))
--
--   Nearest-neighbour copies an exact existing byte so no conversion is
--   needed.  Lossless ops never touch the colour values at all.

local M = {}
M.VERSION = "0.1.0"

local pc = require("coding_adventures.pixel_container")

-- ---------------------------------------------------------------------------
-- sRGB / linear-light LUT
-- ---------------------------------------------------------------------------
--
-- Building a 256-entry table at load time avoids repeated floating-point
-- exponentiation during sampling.  The encode function is not tabulated
-- because its input is a floating-point value, not an integer index.

-- SRGB_TO_LINEAR[i] = linear value for sRGB byte i (0 ≤ i ≤ 255).
-- 0-indexed so that SRGB_TO_LINEAR[byte] works without +1 gymnastics.
local SRGB_TO_LINEAR = {}
for i = 0, 255 do
    local c = i / 255.0
    SRGB_TO_LINEAR[i] = c <= 0.04045 and c / 12.92
                                      or ((c + 0.055) / 1.055) ^ 2.4
end

-- Decode one sRGB byte to a linear-light [0, 1] float.
local function decode(b) return SRGB_TO_LINEAR[b] end

-- Encode a linear-light float back to a clamped, rounded sRGB byte.
-- Input v outside [0, 1] is clamped before conversion.
local function encode(v)
    -- Clamp to [0, 1] so that slight overflows from Catmull-Rom don't wrap.
    v = math.max(0.0, math.min(1.0, v))
    local c = v <= 0.0031308 and 12.92 * v
                              or 1.055 * v ^ (1.0 / 2.4) - 0.055
    c = math.max(0.0, math.min(1.0, c))
    return math.floor(c * 255.0 + 0.5)
end

-- ---------------------------------------------------------------------------
-- Out-of-bounds coordinate resolution
-- ---------------------------------------------------------------------------
--
-- When inverse-mapping, the source coordinate can fall outside [0, W) × [0, H).
-- Different applications call for different boundary behaviours:
--
--   "zero"      — treat OOB pixels as transparent black (return nil → caller
--                 uses 0,0,0,0)
--   "replicate" — clamp to the nearest edge pixel (good for scale/affine)
--   "reflect"   — mirror the image at boundaries (good for convolution)
--   "wrap"      — tile the image (good for repeating textures)

-- resolve_coord maps a possibly-OOB integer coordinate to a valid in-bounds
-- index, or returns nil for "zero" mode when OOB.
--
-- @param x    integer   0-indexed coordinate (may be negative or >= max)
-- @param max  integer   size of axis (width or height)
-- @param oob  string    "zero", "replicate", "reflect", "wrap"
-- @return integer|nil   resolved 0-indexed coordinate, or nil for zero-fill
local function resolve_coord(x, max, oob)
    if oob == "zero" then
        return (x >= 0 and x < max) and x or nil
    elseif oob == "replicate" then
        return math.max(0, math.min(max - 1, x))
    elseif oob == "reflect" then
        -- Reflect with period = 2*max.  Works for all integer x.
        -- e.g. for max=4: sequence is 0 1 2 3 3 2 1 0 0 1 2 3 …
        local period = 2 * max
        x = x % period
        if x < 0 then x = x + period end
        if x >= max then x = period - 1 - x end
        return x
    else  -- "wrap"
        return x % max
    end
end

-- ---------------------------------------------------------------------------
-- Catmull-Rom cubic kernel
-- ---------------------------------------------------------------------------
--
-- Catmull-Rom is a locally-scoped cubic spline that passes through its
-- sample points (interpolating, not approximating).  It is the most popular
-- choice for image bicubic resampling.
--
-- The kernel w(d) for |d| ∈ [0, 2):
--
--   |d| < 1:  w = 1.5|d|^3 − 2.5|d|^2 + 1
--   |d| < 2:  w = −0.5|d|^3 + 2.5|d|^2 − 4|d| + 2
--   |d| >= 2: w = 0
--
-- These piecewise formulas come from setting a=−0.5 in the family of cubic
-- kernels studied by Keys (1989).

local function catmull_rom(d)
    d = math.abs(d)
    if d < 1.0 then
        return 1.5 * d * d * d - 2.5 * d * d + 1.0
    elseif d < 2.0 then
        return -0.5 * d * d * d + 2.5 * d * d - 4.0 * d + 2.0
    else
        return 0.0
    end
end

-- ---------------------------------------------------------------------------
-- Sampling helpers
-- ---------------------------------------------------------------------------
--
-- All sampling functions take continuous source coordinates (u, v) and return
-- r, g, b, a as integers in [0, 255].

-- Nearest-neighbour: snap to the integer pixel whose centre is closest.
-- This is the fastest filter and produces aliasing/pixelation but is
-- pixel-exact (no arithmetic on colour values).
local function sample_nearest(img, u, v, oob)
    -- floor(u) gives the 0-indexed pixel whose left edge u is inside.
    local ix = math.floor(u)
    local iy = math.floor(v)
    local rx = resolve_coord(ix, img.width,  oob)
    local ry = resolve_coord(iy, img.height, oob)
    if rx == nil or ry == nil then
        return 0, 0, 0, 0
    end
    return pc.pixel_at(img, rx, ry)
end

-- Bilinear: weighted average of the 2×2 grid surrounding (u, v).
--
--   Let (x0, y0) = (floor(u), floor(v)).
--   tx = u - x0 (fractional part in x)
--   ty = v - y0 (fractional part in y)
--
--   The four corners and their weights:
--     P00 (x0,   y0  ) weight (1-tx)*(1-ty)
--     P10 (x0+1, y0  ) weight    tx *(1-ty)
--     P01 (x0,   y0+1) weight (1-tx)*   ty
--     P11 (x0+1, y0+1) weight    tx *   ty
--
-- Blending is performed in linear light; the result is re-encoded to sRGB.
-- Alpha is blended linearly (it is already linear by convention).
local function sample_bilinear(img, u, v, oob)
    local x0 = math.floor(u)
    local y0 = math.floor(v)
    local tx = u - x0
    local ty = v - y0

    -- Gather the four neighbours, resolving OOB via the chosen strategy.
    local function get(ix, iy)
        local rx = resolve_coord(ix, img.width,  oob)
        local ry = resolve_coord(iy, img.height, oob)
        if rx == nil or ry == nil then return 0, 0, 0, 0 end
        return pc.pixel_at(img, rx, ry)
    end

    local r00, g00, b00, a00 = get(x0,   y0  )
    local r10, g10, b10, a10 = get(x0+1, y0  )
    local r01, g01, b01, a01 = get(x0,   y0+1)
    local r11, g11, b11, a11 = get(x0+1, y0+1)

    -- Weights for bilinear interpolation.
    local w00 = (1 - tx) * (1 - ty)
    local w10 =      tx  * (1 - ty)
    local w01 = (1 - tx) *      ty
    local w11 =      tx  *      ty

    -- Blend in linear light for RGB.
    local lr = w00*decode(r00) + w10*decode(r10) + w01*decode(r01) + w11*decode(r11)
    local lg = w00*decode(g00) + w10*decode(g10) + w01*decode(g01) + w11*decode(g11)
    local lb = w00*decode(b00) + w10*decode(b10) + w01*decode(b01) + w11*decode(b11)
    -- Alpha is conventional linear data — blend directly.
    local la = w00*a00 + w10*a10 + w01*a01 + w11*a11

    return encode(lr), encode(lg), encode(lb), math.floor(la + 0.5)
end

-- Bicubic (Catmull-Rom): weighted average of the 4×4 grid surrounding (u, v).
--
-- The 4×4 grid spans columns [x0-1 .. x0+2] and rows [y0-1 .. y0+2], where
-- x0 = floor(u), y0 = floor(v).  The kernel weight for column j is
-- catmull_rom(tx - (j - x0)) and similarly for rows.
-- The separable 2-D weight is the product of the two 1-D weights.
local function sample_bicubic(img, u, v, oob)
    local x0 = math.floor(u)
    local y0 = math.floor(v)
    local tx = u - x0   -- fractional position within [x0, x0+1)
    local ty = v - y0

    local function get(ix, iy)
        local rx = resolve_coord(ix, img.width,  oob)
        local ry = resolve_coord(iy, img.height, oob)
        if rx == nil or ry == nil then return 0, 0, 0, 0 end
        return pc.pixel_at(img, rx, ry)
    end

    -- Precompute x-axis kernel weights for columns x0-1 .. x0+2.
    -- The distances from the sample point to each column centre are:
    --   col x0-1: distance = tx + 1
    --   col x0  : distance = tx
    --   col x0+1: distance = 1 - tx
    --   col x0+2: distance = 2 - tx
    local wx = {
        catmull_rom(tx + 1),
        catmull_rom(tx),
        catmull_rom(1.0 - tx),
        catmull_rom(2.0 - tx),
    }
    -- Similarly for y.
    local wy = {
        catmull_rom(ty + 1),
        catmull_rom(ty),
        catmull_rom(1.0 - ty),
        catmull_rom(2.0 - ty),
    }

    local sum_lr, sum_lg, sum_lb, sum_la = 0, 0, 0, 0

    for dy = 0, 3 do
        local iy = y0 - 1 + dy
        for dx = 0, 3 do
            local ix = x0 - 1 + dx
            local r, g, b, a = get(ix, iy)
            local w = wx[dx + 1] * wy[dy + 1]
            sum_lr = sum_lr + w * decode(r)
            sum_lg = sum_lg + w * decode(g)
            sum_lb = sum_lb + w * decode(b)
            sum_la = sum_la + w * a
        end
    end

    return encode(sum_lr), encode(sum_lg), encode(sum_lb),
           math.max(0, math.min(255, math.floor(sum_la + 0.5)))
end

-- Dispatcher: choose the right sampling function.
-- mode can be "nearest", "bilinear", or "bicubic".
local function do_sample(img, u, v, mode, oob)
    if mode == "nearest" then
        return sample_nearest(img, u, v, oob)
    elseif mode == "bicubic" then
        return sample_bicubic(img, u, v, oob)
    else  -- default: bilinear
        return sample_bilinear(img, u, v, oob)
    end
end

-- ---------------------------------------------------------------------------
-- LOSSLESS OPERATIONS
-- ---------------------------------------------------------------------------
--
-- These copy raw bytes without any arithmetic on the colour values.
-- They are all O(W * H) time and O(W * H) space.

--- Flip horizontal: mirror each row left-to-right.
--
-- Pixel (x, y) in the source maps to (W-1-x, y) in the output.
-- Applied twice, produces the original image exactly.
--
-- @param src  table  pixel container
-- @return table  new pixel container
function M.flip_horizontal(src)
    local W, H = src.width, src.height
    local out = pc.new(W, H)
    for y = 0, H - 1 do
        for x = 0, W - 1 do
            local r, g, b, a = pc.pixel_at(src, x, y)
            pc.set_pixel(out, W - 1 - x, y, r, g, b, a)
        end
    end
    return out
end

--- Flip vertical: mirror each column top-to-bottom.
--
-- Pixel (x, y) in the source maps to (x, H-1-y) in the output.
-- Applied twice, produces the original image exactly.
--
-- @param src  table  pixel container
-- @return table  new pixel container
function M.flip_vertical(src)
    local W, H = src.width, src.height
    local out = pc.new(W, H)
    for y = 0, H - 1 do
        for x = 0, W - 1 do
            local r, g, b, a = pc.pixel_at(src, x, y)
            pc.set_pixel(out, x, H - 1 - y, r, g, b, a)
        end
    end
    return out
end

--- Rotate 90° clockwise.
--
-- Output dimensions: W' = H_src, H' = W_src.
-- The source pixel (x, y) maps to output (H_src-1-y, x):
--
--   Imagine a clock.  The top-left corner of the source becomes the
--   top-right corner of the output.  So the source column x becomes
--   the output row x, and the source row y becomes the output column
--   (H_src - 1 - y) counting from the left.
--
-- Equivalently (inverse view): output pixel (x', y') comes from source
--   source_x = y',  source_y = W' - 1 - x'   (where W' = H_src).
-- We iterate over the source for clarity.
--
-- @param src  table  pixel container
-- @return table  new W'=src.height, H'=src.width pixel container
function M.rotate_90_cw(src)
    local W, H = src.width, src.height
    -- Output dimensions are swapped.
    local out = pc.new(H, W)  -- W' = H_src, H' = W_src
    for y = 0, H - 1 do
        for x = 0, W - 1 do
            local r, g, b, a = pc.pixel_at(src, x, y)
            -- src(x, y) → out(H-1-y, x)
            pc.set_pixel(out, H - 1 - y, x, r, g, b, a)
        end
    end
    return out
end

--- Rotate 90° counter-clockwise.
--
-- Output dimensions: W' = H_src, H' = W_src.
-- The source pixel (x, y) maps to output (y, W-1-x):
--
--   The top-left corner of the source becomes the bottom-left corner of
--   the output.  Column x becomes row (W-1-x) from the bottom, which is
--   column y from the left.
--
-- @param src  table  pixel container
-- @return table  new W'=src.height, H'=src.width pixel container
function M.rotate_90_ccw(src)
    local W, H = src.width, src.height
    local out = pc.new(H, W)
    for y = 0, H - 1 do
        for x = 0, W - 1 do
            local r, g, b, a = pc.pixel_at(src, x, y)
            -- src(x, y) → out(y, W-1-x)
            pc.set_pixel(out, y, W - 1 - x, r, g, b, a)
        end
    end
    return out
end

--- Rotate 180°.
--
-- Equivalent to flip_horizontal followed by flip_vertical.
-- Pixel (x, y) maps to (W-1-x, H-1-y).  Applied twice → identity.
--
-- @param src  table  pixel container
-- @return table  new pixel container (same dimensions)
function M.rotate_180(src)
    local W, H = src.width, src.height
    local out = pc.new(W, H)
    for y = 0, H - 1 do
        for x = 0, W - 1 do
            local r, g, b, a = pc.pixel_at(src, x, y)
            pc.set_pixel(out, W - 1 - x, H - 1 - y, r, g, b, a)
        end
    end
    return out
end

--- Crop: extract a rectangular sub-image.
--
-- All coordinates are 0-indexed.  The crop rectangle is [x0, x0+w) × [y0, y0+h).
-- Coordinates are clamped against the source boundary; if the rectangle
-- extends outside the source, the corresponding output pixels are left as
-- transparent black (the default from pc.new).
--
-- @param src  table   pixel container
-- @param x0   number  left column (0-indexed, inclusive)
-- @param y0   number  top row (0-indexed, inclusive)
-- @param w    number  output width in pixels
-- @param h    number  output height in pixels
-- @return table  new pixel container of dimensions w × h
function M.crop(src, x0, y0, w, h)
    local out = pc.new(w, h)
    for dy = 0, h - 1 do
        for dx = 0, w - 1 do
            -- pc.pixel_at returns 0,0,0,0 for OOB coordinates, so no guard needed.
            local r, g, b, a = pc.pixel_at(src, x0 + dx, y0 + dy)
            pc.set_pixel(out, dx, dy, r, g, b, a)
        end
    end
    return out
end

--- Pad: add a border of constant colour around the source image.
--
-- The fill colour defaults to {0, 0, 0, 0} (transparent black).
--
-- Output dimensions: W + left + right, H + top + bottom.
-- The source image is placed at offset (left, top) in the output.
--
--   ┌──────────────────────────┐
--   │     top padding          │
--   │  ┌────────────────────┐  │
--   │  │                    │  │
--   │ L│     source         │R │
--   │  │                    │  │
--   │  └────────────────────┘  │
--   │     bottom padding       │
--   └──────────────────────────┘
--
-- @param src     table    pixel container
-- @param top     number   rows of padding above
-- @param right   number   columns of padding to the right
-- @param bottom  number   rows of padding below
-- @param left    number   columns of padding to the left
-- @param fill    table    {r, g, b, a} fill colour (default {0,0,0,0})
-- @return table  new padded pixel container
function M.pad(src, top, right, bottom, left, fill)
    fill = fill or {0, 0, 0, 0}
    local fr, fg, fb, fa = fill[1], fill[2], fill[3], fill[4]
    local W, H = src.width, src.height
    local OW = W + left + right
    local OH = H + top  + bottom
    local out = pc.new(OW, OH)

    -- Fill the entire output with the pad colour first (faster than
    -- checking per-pixel whether we're in the border zone).
    for y = 0, OH - 1 do
        for x = 0, OW - 1 do
            pc.set_pixel(out, x, y, fr, fg, fb, fa)
        end
    end

    -- Stamp the source image into the interior.
    for y = 0, H - 1 do
        for x = 0, W - 1 do
            local r, g, b, a = pc.pixel_at(src, x, y)
            pc.set_pixel(out, x + left, y + top, r, g, b, a)
        end
    end
    return out
end

-- ---------------------------------------------------------------------------
-- CONTINUOUS OPERATIONS
-- ---------------------------------------------------------------------------
--
-- All continuous operations use inverse mapping: for each output pixel (ox, oy),
-- compute the corresponding source coordinate (u, v), then sample the source.
--
-- The pixel-centre convention places the centre of pixel (i, j) at (i + 0.5,
-- j + 0.5) in continuous space.  Inverse mapping proceeds as:
--
--   1. Map output pixel centre to continuous output coords: (ox + 0.5, oy + 0.5)
--   2. Apply the inverse transform to get continuous source coords: (u, v)
--   3. Convert back to 0-indexed pixel space: u' = u - 0.5, v' = v - 0.5
--   4. Sample src at (u', v') using the chosen interpolation filter

--- Scale (resize) an image to new_w × new_h using the specified filter.
--
-- The pixel-centre inverse mapping for scaling by factors sx = new_w/W and
-- sy = new_h/H is:
--
--   u = (ox + 0.5) * (W  / new_w) - 0.5
--   v = (oy + 0.5) * (H  / new_h) - 0.5
--
-- Out-of-bounds samples use "replicate" (edge-extend) to avoid dark halos
-- at image borders when using bilinear or bicubic filters.
--
-- @param src    table   source pixel container
-- @param out_w  number  output width in pixels
-- @param out_h  number  output height in pixels
-- @param mode   string  "nearest", "bilinear" (default), or "bicubic"
-- @return table  new pixel container of dimensions out_w × out_h
function M.scale(src, out_w, out_h, mode)
    mode = mode or "bilinear"
    local W, H = src.width, src.height
    local out = pc.new(out_w, out_h)
    for oy = 0, out_h - 1 do
        for ox = 0, out_w - 1 do
            -- Map output pixel centre (ox+0.5, oy+0.5) back to source space.
            local u = (ox + 0.5) * W  / out_w - 0.5
            local v = (oy + 0.5) * H  / out_h - 0.5
            local r, g, b, a = do_sample(src, u, v, mode, "replicate")
            pc.set_pixel(out, ox, oy, r, g, b, a)
        end
    end
    return out
end

--- Rotate an image by an arbitrary angle (in radians).
--
-- Uses inverse mapping: for each output pixel, rotate backward by -radians
-- around the respective image centres, then sample the source.
--
-- Centre of source: (W/2, H/2) in continuous pixel-centre space.
-- Centre of output: (out_w/2, out_h/2).
--
-- Inverse rotation (by -θ) of output pixel centre (ox+0.5, oy+0.5):
--   dx = (ox + 0.5) - cx_out
--   dy = (oy + 0.5) - cy_out
--   u = cx_in +  cos(θ)*dx + sin(θ)*dy  − 0.5
--   v = cy_in + -sin(θ)*dx + cos(θ)*dy  − 0.5
--
-- bounds:
--   "fit"  — output canvas is sized to contain the entire rotated source
--            (no clipping, corners are zero-filled)
--   "crop" — output canvas is the same size as the source
--            (corners of the source may be clipped)
--
-- Out-of-bounds samples use "zero" fill so corners appear transparent.
--
-- @param src      table   source pixel container
-- @param radians  number  rotation angle in radians (positive = CCW in math,
--                         but image Y-axis points down so CCW visually = CW math)
-- @param mode     string  interpolation filter (default "bilinear")
-- @param bounds   string  "fit" (default) or "crop"
-- @return table  new pixel container
function M.rotate(src, radians, mode, bounds)
    mode   = mode   or "bilinear"
    bounds = bounds or "fit"
    local W, H = src.width, src.height

    local cos_a = math.cos(radians)
    local sin_a = math.sin(radians)
    local abs_cos = math.abs(cos_a)
    local abs_sin = math.abs(sin_a)

    -- Compute output canvas size.
    local out_w, out_h
    if bounds == "fit" then
        -- The bounding box of the rotated rectangle.
        out_w = math.ceil(W * abs_cos + H * abs_sin)
        out_h = math.ceil(W * abs_sin + H * abs_cos)
    else  -- "crop"
        out_w = W
        out_h = H
    end

    -- Continuous centres (pixel-centre convention: centre of pixel i is i+0.5).
    local cx_in  = W  / 2.0
    local cy_in  = H  / 2.0
    local cx_out = out_w / 2.0
    local cy_out = out_h / 2.0

    local out = pc.new(out_w, out_h)
    for oy = 0, out_h - 1 do
        for ox = 0, out_w - 1 do
            -- Displacement from output centre.
            local dx = (ox + 0.5) - cx_out
            local dy = (oy + 0.5) - cy_out
            -- Apply inverse rotation (rotate by -radians).
            -- For image coordinates (Y down): CW visual rotation by θ means
            --   u = cx_in + cos(θ)*dx + sin(θ)*dy
            --   v = cy_in - sin(θ)*dx + cos(θ)*dy
            local u = cx_in + cos_a * dx + sin_a * dy - 0.5
            local v = cy_in - sin_a * dx + cos_a * dy - 0.5
            local r, g, b, a = do_sample(src, u, v, mode, "zero")
            pc.set_pixel(out, ox, oy, r, g, b, a)
        end
    end
    return out
end

--- Affine transform.
--
-- An affine transform is any combination of scale, rotation, shear, and
-- translation that preserves parallelism (parallel lines stay parallel).
-- It is described by a 2×3 matrix applied to homogeneous 2-D coordinates:
--
--   | x' |   | m00  m01  m02 |   | x |
--   | y' | = | m10  m11  m12 | × | y |
--                                  | 1 |
--
-- This function uses the INVERSE of the given matrix to map each output
-- pixel back to source space.  The caller supplies the FORWARD matrix
-- (source → output).
--
-- To compute the inverse of a 2×3 affine matrix:
--   Given M = [[a, b, tx], [c, d, ty]], let det = a*d - b*c, then
--   M_inv = [[ d/det, -b/det, (b*ty - d*tx)/det ],
--            [-c/det,  a/det, (c*tx - a*ty)/det ]]
--
-- @param src    table   source pixel container
-- @param matrix table   {{m00,m01,m02},{m10,m11,m12}} forward affine matrix
-- @param out_w  number  output width in pixels
-- @param out_h  number  output height in pixels
-- @param mode   string  interpolation filter (default "bilinear")
-- @param oob    string  OOB mode (default "zero")
-- @return table  new pixel container
function M.affine(src, matrix, out_w, out_h, mode, oob)
    mode = mode or "bilinear"
    oob  = oob  or "zero"
    local m = matrix  -- shorthand

    -- Extract the 2×2 linear part and translation.
    local a, b, tx = m[1][1], m[1][2], m[1][3]
    local c, d, ty = m[2][1], m[2][2], m[2][3]

    -- Compute determinant of the linear part.
    local det = a * d - b * c
    if math.abs(det) < 1e-10 then
        error("image_geometric_transforms.affine: matrix is singular (det ≈ 0)")
    end

    -- Inverse matrix coefficients (2×3).
    local inv_a  =  d / det
    local inv_b  = -b / det
    local inv_tx = (b * ty - d * tx) / det
    local inv_c  = -c / det
    local inv_d  =  a / det
    local inv_ty = (c * tx - a * ty) / det

    local out = pc.new(out_w, out_h)
    for oy = 0, out_h - 1 do
        for ox = 0, out_w - 1 do
            -- Output pixel centre in continuous space.
            local px = ox + 0.5
            local py = oy + 0.5
            -- Apply inverse transform (output → source).
            local u = inv_a * px + inv_b * py + inv_tx - 0.5
            local v = inv_c * px + inv_d * py + inv_ty - 0.5
            local r, g, b, a = do_sample(src, u, v, mode, oob)
            pc.set_pixel(out, ox, oy, r, g, b, a)
        end
    end
    return out
end

--- Perspective warp (homography).
--
-- A homography (perspective transform) maps four source quadrilateral corners
-- to four output corners.  Unlike affine, parallel lines are NOT preserved.
-- The transform is encoded as a 3×3 homogeneous matrix H:
--
--   | wx' |   | h00  h01  h02 |   | x |
--   | wy' | = | h10  h11  h12 | × | y |
--   |  w  |   | h20  h21  h22 |   | 1 |
--
--   Normalised: x' = wx'/w,  y' = wy'/w
--
-- As with affine, we invert H to map output pixels back to source space.
-- The 3×3 inverse is computed via cofactor expansion.
--
-- @param src    table   source pixel container
-- @param h      table   3×3 nested table {{h00,h01,h02},{h10,h11,h12},{h20,h21,h22}}
-- @param out_w  number  output width
-- @param out_h  number  output height
-- @param mode   string  interpolation filter (default "bilinear")
-- @param oob    string  OOB mode (default "zero")
-- @return table  new pixel container
function M.perspective_warp(src, h, out_w, out_h, mode, oob)
    mode = mode or "bilinear"
    oob  = oob  or "zero"

    -- Invert the 3×3 matrix using cofactors / adjugate method.
    -- det(H) = h00*(h11*h22 - h12*h21) - h01*(h10*h22 - h12*h20)
    --         + h02*(h10*h21 - h11*h20)
    local h00, h01, h02 = h[1][1], h[1][2], h[1][3]
    local h10, h11, h12 = h[2][1], h[2][2], h[2][3]
    local h20, h21, h22 = h[3][1], h[3][2], h[3][3]

    local det = h00*(h11*h22 - h12*h21)
              - h01*(h10*h22 - h12*h20)
              + h02*(h10*h21 - h11*h20)

    if math.abs(det) < 1e-10 then
        error("image_geometric_transforms.perspective_warp: homography matrix is singular")
    end

    local inv_det = 1.0 / det

    -- Adjugate matrix (transpose of cofactor matrix), divided by det.
    local i00 = (h11*h22 - h12*h21) * inv_det
    local i01 = (h02*h21 - h01*h22) * inv_det
    local i02 = (h01*h12 - h02*h11) * inv_det
    local i10 = (h12*h20 - h10*h22) * inv_det
    local i11 = (h00*h22 - h02*h20) * inv_det
    local i12 = (h02*h10 - h00*h12) * inv_det
    local i20 = (h10*h21 - h11*h20) * inv_det
    local i21 = (h01*h20 - h00*h21) * inv_det
    local i22 = (h00*h11 - h01*h10) * inv_det

    local out = pc.new(out_w, out_h)
    for oy = 0, out_h - 1 do
        for ox = 0, out_w - 1 do
            -- Pixel centre in continuous output space.
            local px = ox + 0.5
            local py = oy + 0.5

            -- Apply inverse homography.
            local wx = i00 * px + i01 * py + i02
            local wy = i10 * px + i11 * py + i12
            local w  = i20 * px + i21 * py + i22

            -- Normalise (divide by homogeneous weight).
            local u = wx / w - 0.5
            local v = wy / w - 0.5

            local r, g, b, a = do_sample(src, u, v, mode, oob)
            pc.set_pixel(out, ox, oy, r, g, b, a)
        end
    end
    return out
end

return M
