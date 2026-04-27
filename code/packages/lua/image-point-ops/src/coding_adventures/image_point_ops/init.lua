-- image_point_ops — IMG03: Per-pixel point operations on PixelContainer
--
-- A point operation transforms each pixel independently using only that
-- pixel's own value.  No neighbouring pixels, no frequency domain, no
-- geometry.
--
-- ## Two domains
--
-- u8-domain operations (invert, threshold, posterize, channel ops, brightness)
-- work directly on the 8-bit sRGB bytes.  Correct without colour-space
-- conversion because they are monotone remappings that never mix values.
--
-- Linear-light operations (contrast, gamma, exposure, greyscale, sepia,
-- colour_matrix, saturate, hue_rotate) decode each byte to linear f32 first:
--
--   c = byte / 255
--   linear = c / 12.92               if c <= 0.04045
--           = ((c + 0.055)/1.055)^2.4  otherwise
--
-- Then re-encode after the operation:
--
--   encoded = linear * 12.92                        if linear <= 0.0031308
--           = 1.055 * linear^(1/2.4) − 0.055        otherwise
--   byte = math.max(0, math.min(255, math.floor(encoded * 255 + 0.5)))
--
-- ## Usage
--
--   local ipo = require("coding_adventures.image_point_ops")
--   local pc  = require("coding_adventures.pixel_container")
--
--   local img = pc.new(640, 480)
--   local inv = ipo.invert(img)
--   local bw  = ipo.greyscale(img, "rec709")

local M = {}
M.VERSION = "0.1.0"

local pc = require("coding_adventures.pixel_container")

-- ---------------------------------------------------------------------------
-- sRGB / linear LUT
-- ---------------------------------------------------------------------------

-- 256-entry decode LUT: index is byte value (1-indexed in Lua tables).
-- Built once at module load.
local SRGB_TO_LINEAR = {}
for i = 0, 255 do
    local c = i / 255.0
    if c <= 0.04045 then
        SRGB_TO_LINEAR[i] = c / 12.92
    else
        SRGB_TO_LINEAR[i] = ((c + 0.055) / 1.055) ^ 2.4
    end
end

local function decode(byte)
    return SRGB_TO_LINEAR[byte]
end

local function encode(linear)
    local c
    if linear <= 0.0031308 then
        c = linear * 12.92
    else
        c = 1.055 * linear ^ (1.0 / 2.4) - 0.055
    end
    c = math.max(0.0, math.min(1.0, c))
    return math.floor(c * 255 + 0.5)
end

-- ---------------------------------------------------------------------------
-- Iteration helper
-- ---------------------------------------------------------------------------

local function map_pixels(src, fn)
    local out = pc.new(src.width, src.height)
    for y = 0, src.height - 1 do
        for x = 0, src.width - 1 do
            local r, g, b, a = pc.pixel_at(src, x, y)
            local nr, ng, nb, na = fn(r, g, b, a)
            pc.set_pixel(out, x, y, nr, ng, nb, na)
        end
    end
    return out
end

-- ---------------------------------------------------------------------------
-- u8-domain operations
-- ---------------------------------------------------------------------------

--- Invert: flip each RGB channel (255 − v).  Alpha is preserved.
-- Applying invert twice returns the original image exactly.
function M.invert(src)
    return map_pixels(src, function(r, g, b, a)
        return 255 - r, 255 - g, 255 - b, a
    end)
end

--- Threshold: (r+g+b)/3 >= value → white, else black.  Alpha preserved.
function M.threshold(src, value)
    return map_pixels(src, function(r, g, b, a)
        local luma = math.floor((r + g + b) / 3)
        local v = luma >= value and 255 or 0
        return v, v, v, a
    end)
end

--- Threshold on Rec. 709 luma: Y = 0.2126 R + 0.7152 G + 0.0722 B.
function M.threshold_luminance(src, value)
    return map_pixels(src, function(r, g, b, a)
        local luma = 0.2126 * r + 0.7152 * g + 0.0722 * b
        local v = luma >= value and 255 or 0
        return v, v, v, a
    end)
end

--- Posterize: reduce each channel to `levels` equally-spaced steps.
function M.posterize(src, levels)
    local step = 255.0 / (levels - 1)
    local function q(v)
        return math.floor(math.floor(v / step + 0.5) * step + 0.5)
    end
    return map_pixels(src, function(r, g, b, a)
        return q(r), q(g), q(b), a
    end)
end

--- Swap R and B channels (RGB ↔ BGR).
function M.swap_rgb_bgr(src)
    return map_pixels(src, function(r, g, b, a)
        return b, g, r, a
    end)
end

--- Extract one channel (0=R, 1=G, 2=B, 3=A), zero the rest.
-- Alpha is always preserved.
function M.extract_channel(src, channel)
    return map_pixels(src, function(r, g, b, a)
        if channel == 0 then return r, 0, 0, a end
        if channel == 1 then return 0, g, 0, a end
        if channel == 2 then return 0, 0, b, a end
        return r, g, b, a
    end)
end

--- Additive brightness: add signed offset, clamped to [0, 255].
function M.brightness(src, offset)
    return map_pixels(src, function(r, g, b, a)
        local function clamp(v)
            return math.max(0, math.min(255, v + offset))
        end
        return clamp(r), clamp(g), clamp(b), a
    end)
end

-- ---------------------------------------------------------------------------
-- Linear-light operations
-- ---------------------------------------------------------------------------

--- Contrast: scale around linear mid-grey (0.5).
-- factor = 1 → identity; < 1 → lower; > 1 → higher contrast.
function M.contrast(src, factor)
    return map_pixels(src, function(r, g, b, a)
        return encode(0.5 + factor * (decode(r) - 0.5)),
               encode(0.5 + factor * (decode(g) - 0.5)),
               encode(0.5 + factor * (decode(b) - 0.5)),
               a
    end)
end

--- Gamma: apply power-law g in linear light.
-- g < 1 → brightens; g > 1 → darkens; g = 1 → identity.
function M.gamma(src, g)
    return map_pixels(src, function(r, gv, b, a)
        return encode(decode(r) ^ g),
               encode(decode(gv) ^ g),
               encode(decode(b) ^ g),
               a
    end)
end

--- Exposure: multiply linear by 2^stops.
function M.exposure(src, stops)
    local factor = 2 ^ stops
    return map_pixels(src, function(r, g, b, a)
        return encode(decode(r) * factor),
               encode(decode(g) * factor),
               encode(decode(b) * factor),
               a
    end)
end

--- Greyscale: convert to luminance in linear light.
-- method: "rec709" (default), "bt601", or "average".
function M.greyscale(src, method)
    method = method or "rec709"
    local wr, wg, wb
    if method == "rec709" then
        wr, wg, wb = 0.2126, 0.7152, 0.0722
    elseif method == "bt601" then
        wr, wg, wb = 0.2989, 0.5870, 0.1140
    else
        wr, wg, wb = 1/3, 1/3, 1/3
    end
    return map_pixels(src, function(r, g, b, a)
        local y = encode(wr * decode(r) + wg * decode(g) + wb * decode(b))
        return y, y, y, a
    end)
end

--- Sepia: classic warm sepia tone matrix in linear light.
function M.sepia(src)
    return map_pixels(src, function(r, g, b, a)
        local lr, lg, lb = decode(r), decode(g), decode(b)
        return encode(0.393 * lr + 0.769 * lg + 0.189 * lb),
               encode(0.349 * lr + 0.686 * lg + 0.168 * lb),
               encode(0.272 * lr + 0.534 * lg + 0.131 * lb),
               a
    end)
end

--- Colour matrix: multiply linear [R, G, B] by a 3×3 matrix.
-- matrix is a table {{m00,m01,m02},{m10,m11,m12},{m20,m21,m22}}.
function M.colour_matrix(src, matrix)
    local m = matrix
    return map_pixels(src, function(r, g, b, a)
        local lr, lg, lb = decode(r), decode(g), decode(b)
        return encode(m[1][1] * lr + m[1][2] * lg + m[1][3] * lb),
               encode(m[2][1] * lr + m[2][2] * lg + m[2][3] * lb),
               encode(m[3][1] * lr + m[3][2] * lg + m[3][3] * lb),
               a
    end)
end

--- Saturate: 0 → greyscale; 1 → identity; > 1 → vivid.
function M.saturate(src, factor)
    return map_pixels(src, function(r, g, b, a)
        local lr, lg, lb = decode(r), decode(g), decode(b)
        local grey = 0.2126 * lr + 0.7152 * lg + 0.0722 * lb
        return encode(grey + factor * (lr - grey)),
               encode(grey + factor * (lg - grey)),
               encode(grey + factor * (lb - grey)),
               a
    end)
end

-- ---------------------------------------------------------------------------
-- HSV helpers
-- ---------------------------------------------------------------------------

local function rgb_to_hsv(r, g, b)
    local mx = math.max(r, g, b)
    local mn = math.min(r, g, b)
    local delta = mx - mn
    local v = mx
    local s = mx == 0 and 0 or delta / mx
    local h = 0
    if delta ~= 0 then
        if mx == r then
            h = ((g - b) / delta) % 6
        elseif mx == g then
            h = (b - r) / delta + 2
        else
            h = (r - g) / delta + 4
        end
        h = (h * 60 + 360) % 360
    end
    return h, s, v
end

local function hsv_to_rgb(h, s, v)
    local c = v * s
    local x = c * (1 - math.abs((h / 60) % 2 - 1))
    local m = v - c
    local r, g, b
    local sector = math.floor(h / 60)
    if     sector == 0 then r, g, b = c, x, 0
    elseif sector == 1 then r, g, b = x, c, 0
    elseif sector == 2 then r, g, b = 0, c, x
    elseif sector == 3 then r, g, b = 0, x, c
    elseif sector == 4 then r, g, b = x, 0, c
    else                    r, g, b = c, 0, x
    end
    return r + m, g + m, b + m
end

--- Hue rotate: rotate hue by degrees.  360° is identity.
function M.hue_rotate(src, degrees)
    return map_pixels(src, function(r, g, b, a)
        local h, s, v = rgb_to_hsv(decode(r), decode(g), decode(b))
        local nr, ng, nb = hsv_to_rgb((h + degrees + 360) % 360, s, v)
        return encode(nr), encode(ng), encode(nb), a
    end)
end

-- ---------------------------------------------------------------------------
-- Colorspace utilities
-- ---------------------------------------------------------------------------

--- Convert sRGB → linear (each byte becomes round(linear * 255)).
function M.srgb_to_linear_image(src)
    return map_pixels(src, function(r, g, b, a)
        return math.floor(decode(r) * 255 + 0.5),
               math.floor(decode(g) * 255 + 0.5),
               math.floor(decode(b) * 255 + 0.5),
               a
    end)
end

--- Convert linear → sRGB (inverse of srgb_to_linear_image).
function M.linear_to_srgb_image(src)
    return map_pixels(src, function(r, g, b, a)
        return encode(r / 255), encode(g / 255), encode(b / 255), a
    end)
end

-- ---------------------------------------------------------------------------
-- 1D LUT operations
-- ---------------------------------------------------------------------------

--- Apply three 256-entry u8→u8 LUTs (one per channel).  Alpha preserved.
-- Each LUT is a table indexed 0–255.
function M.apply_lut1d_u8(src, lut_r, lut_g, lut_b)
    return map_pixels(src, function(r, g, b, a)
        return lut_r[r], lut_g[g], lut_b[b], a
    end)
end

--- Build a 256-entry LUT from a linear-light mapping function.
-- fn receives a linear float [0,1] and returns a linear float [0,1].
-- Returns a table indexed 0–255.
function M.build_lut1d_u8(fn)
    local lut = {}
    for i = 0, 255 do
        lut[i] = encode(fn(decode(i)))
    end
    return lut
end

--- Build a gamma LUT (equivalent to build_lut1d_u8 with v^g).
function M.build_gamma_lut(g)
    return M.build_lut1d_u8(function(v) return v ^ g end)
end

return M
