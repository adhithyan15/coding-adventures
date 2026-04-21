-- | IMG03 — Per-pixel point operations.
--
-- A /point operation/ is any image transform where the output pixel at
-- @(x, y)@ depends only on the /input/ pixel at @(x, y)@.  No
-- neighbourhood, no history, no context — just a pure function of one
-- RGBA sample.  This makes the operations trivially parallel (we just
-- map over pixels) and easy to reason about.
--
-- == sRGB vs. linear light
--
-- JPEG/PNG pixels are /gamma-encoded/: the 8-bit value is a power-law
-- compressed version of the physical light intensity, roughly
-- @byte = 255 * linear ** (1/2.4)@ with a small linear toe near black.
-- Arithmetic (scaling, blending, averaging) should happen in /linear
-- light/ to be physically correct; otherwise dark colours dominate.
--
-- We therefore:
--
-- * Pre-compute a 256-entry lookup table 'srgbToLinear' for the decode
--   direction (the hot path).
-- * Use 'encodeSrgb' for the encode direction (rarer, so we compute on
--   the fly).
--
-- Operations that only re-index pixels (invert, threshold, swap, LUTs)
-- stay in sRGB because they do not perform any arithmetic on intensity.
-- Operations that scale or blend (gamma, exposure, greyscale, sepia,
-- saturation, hue-rotate, colour matrix) round-trip through linear.
module ImagePointOps
    ( GreyscaleMethod(..)
    , invert
    , threshold
    , thresholdLuminance
    , posterize
    , swapRgbBgr
    , extractChannel
    , brightness
    , contrast
    , gamma
    , exposure
    , greyscale
    , sepia
    , colourMatrix
    , saturate
    , hueRotate
    , srgbToLinearImage
    , linearToSrgbImage
    , applyLut1dU8
    , buildLut1dU8
    , buildGammaLut
    ) where

import PixelContainer
import Data.Word (Word8)
import qualified Data.ByteString as BS

-- ---------------------------------------------------------------------------
-- sRGB <-> linear helpers
-- ---------------------------------------------------------------------------

-- | 256-entry LUT mapping a byte in sRGB space to a 'Double' in
-- [0, 1] linear-light space.
--
-- The formal sRGB EOTF is:
--
-- @
-- c = byte / 255
-- linear = if c <= 0.04045
--          then c / 12.92
--          else ((c + 0.055) / 1.055) ** 2.4
-- @
srgbToLinear :: [Double]
srgbToLinear =
    [ let c = fromIntegral i / 255.0
      in if c <= 0.04045
         then c / 12.92
         else ((c + 0.055) / 1.055) ** 2.4
    | i <- [0..255 :: Int]
    ]

-- | Decode a single sRGB byte to linear-light [0, 1].
decodeSrgb :: Word8 -> Double
decodeSrgb b = srgbToLinear !! fromIntegral b

-- | Encode a linear-light value in [0, 1] back to an sRGB byte.
--
-- Input is clamped before encoding so that downstream clipping never
-- produces NaNs from @(negative)**2.4@.
encodeSrgb :: Double -> Word8
encodeSrgb linear =
    let c    = max 0.0 (min 1.0 linear)
        srgb = if c <= 0.0031308
               then c * 12.92
               else 1.055 * (c ** (1.0 / 2.4)) - 0.055
    in round (srgb * 255.0)

-- ---------------------------------------------------------------------------
-- Generic per-pixel map
-- ---------------------------------------------------------------------------

-- | Apply a pure function to every pixel.  This is the single workhorse
-- every point operation goes through.  Because 'BS.ByteString' is
-- immutable, we compute the new contents as a list of lists of bytes
-- and pack them in one go — one allocation, linear in the image size.
mapPixels :: PixelContainer
          -> (Word8 -> Word8 -> Word8 -> Word8 -> (Word8, Word8, Word8, Word8))
          -> PixelContainer
mapPixels src@(PixelContainer w h _) f =
    let pixList =
            [ let (r, g, b, a)     = pixelAt src x y
                  (r', g', b', a') = f r g b a
              in [r', g', b', a']
            | y <- [0 .. h - 1], x <- [0 .. w - 1]
            ]
    in PixelContainer w h (BS.pack (concat pixList))

-- | Clamp an 'Int' to the @[0, 255]@ range and cast to 'Word8'.
clampByte :: Int -> Word8
clampByte v = fromIntegral (max 0 (min 255 v))

-- | Clamp a 'Double' to @[0, 255]@ and round to a 'Word8'.
clampByteD :: Double -> Word8
clampByteD v = fromIntegral (max 0 (min 255 (round v :: Int)))

-- ---------------------------------------------------------------------------
-- Simple index-remapping operations (stay in sRGB)
-- ---------------------------------------------------------------------------

-- | Invert RGB (photographic negative).  Alpha is preserved because it
-- is not a colour — inverting transparency would be a bug, not a
-- feature.
invert :: PixelContainer -> PixelContainer
invert pc = mapPixels pc (\r g b a -> (255 - r, 255 - g, 255 - b, a))

-- | Binarise using the average of the three channels as the measure.
-- Any pixel whose average meets or exceeds the threshold becomes
-- white; anything darker becomes black.  Alpha is preserved.
threshold :: PixelContainer -> Word8 -> PixelContainer
threshold pc t = mapPixels pc $ \r g b a ->
    let avg = (fromIntegral r + fromIntegral g + fromIntegral b) `div` (3 :: Int)
    in if avg >= fromIntegral t
       then (255, 255, 255, a)
       else (0, 0, 0, a)

-- | Binarise using Rec. 709 luminance
-- (@Y = 0.2126 R + 0.7152 G + 0.0722 B@).  This matches how humans
-- perceive brightness and is what you want for most "convert to B&W"
-- use-cases — a bright yellow and a dark blue do not average to the
-- same visual grey.
thresholdLuminance :: PixelContainer -> Word8 -> PixelContainer
thresholdLuminance pc t = mapPixels pc $ \r g b a ->
    let y = 0.2126 * fromIntegral r
          + 0.7152 * fromIntegral g
          + 0.0722 * fromIntegral b
    in if y >= fromIntegral t
       then (255, 255, 255, a)
       else (0, 0, 0, a)

-- | Posterise to @levels@ discrete tones per channel.  Each input value
-- is snapped to the nearest multiple of @255 \/ (levels - 1)@, giving a
-- stepped "paint-by-numbers" look.  @levels < 2@ is clamped to 2.
posterize :: PixelContainer -> Int -> PixelContainer
posterize pc levelsRaw =
    let levels = max 2 levelsRaw
        step   = 255.0 / fromIntegral (levels - 1) :: Double
        q v    = let vd = fromIntegral v / step
                     snapped = fromIntegral (round vd :: Int) * step
                 in clampByteD snapped
    in mapPixels pc (\r g b a -> (q r, q g, q b, a))

-- | Swap red and blue channels — RGBA -> BGRA.  Useful for debugging
-- or interfacing with libraries that use BGR order.
swapRgbBgr :: PixelContainer -> PixelContainer
swapRgbBgr pc = mapPixels pc (\r g b a -> (b, g, r, a))

-- | Extract a single channel as an opaque greyscale image.  @ch = 0,
-- 1, 2, 3@ selects R, G, B, A respectively.  The chosen channel is
-- replicated into all three colour channels and alpha is forced to
-- fully opaque so the result is immediately viewable.  Any other value
-- of @ch@ returns the image unchanged.
extractChannel :: PixelContainer -> Int -> PixelContainer
extractChannel pc ch = mapPixels pc $ \r g b a ->
    let v = case ch of
                0 -> r
                1 -> g
                2 -> b
                3 -> a
                _ -> 0
    in case ch of
        0 -> (v, v, v, 255)
        1 -> (v, v, v, 255)
        2 -> (v, v, v, 255)
        3 -> (v, v, v, 255)
        _ -> (r, g, b, a)

-- ---------------------------------------------------------------------------
-- Tone operations
-- ---------------------------------------------------------------------------

-- | Additive brightness in sRGB space.  Each RGB channel is shifted by
-- @delta@ and clamped to @[0, 255]@.  Working in sRGB here matches the
-- "brightness slider" in a naive image editor; use 'exposure' for a
-- physically correct scale.
brightness :: PixelContainer -> Int -> PixelContainer
brightness pc delta = mapPixels pc $ \r g b a ->
    ( clampByte (fromIntegral r + delta)
    , clampByte (fromIntegral g + delta)
    , clampByte (fromIntegral b + delta)
    , a
    )

-- | GIMP-style contrast in sRGB.  @factor@ is in @[-1, 1]@: zero is
-- identity, positive pushes away from mid-grey, negative compresses
-- toward mid-grey.  The classic formula is:
--
-- @
-- f   = (259 * (factor * 255 + 255)) / (255 * (259 - factor * 255))
-- out = f * (in - 128) + 128
-- @
contrast :: PixelContainer -> Double -> PixelContainer
contrast pc factor =
    let f       = (259.0 * (factor * 255.0 + 255.0))
                / (255.0 * (259.0 - factor * 255.0))
        adjust v = clampByteD (f * (fromIntegral v - 128.0) + 128.0)
    in mapPixels pc (\r g b a -> (adjust r, adjust g, adjust b, a))

-- | Apply a gamma curve @y = x^g@ in linear light.  @g < 1@ brightens
-- mid-tones, @g > 1@ darkens them.
gamma :: PixelContainer -> Double -> PixelContainer
gamma pc g = mapPixels pc $ \r gr b a ->
    let rl = decodeSrgb r  ** g
        gl = decodeSrgb gr ** g
        bl = decodeSrgb b  ** g
    in (encodeSrgb rl, encodeSrgb gl, encodeSrgb bl, a)

-- | Physically correct exposure adjustment: multiply linear light by
-- @2 ^ stops@.  @+1 stop@ doubles brightness, @-1 stop@ halves it.
exposure :: PixelContainer -> Double -> PixelContainer
exposure pc stops =
    let scale = 2.0 ** stops
    in mapPixels pc $ \r g b a ->
        ( encodeSrgb (decodeSrgb r * scale)
        , encodeSrgb (decodeSrgb g * scale)
        , encodeSrgb (decodeSrgb b * scale)
        , a
        )

-- ---------------------------------------------------------------------------
-- Colour-space style operations
-- ---------------------------------------------------------------------------

-- | Algorithm family for converting colour to greyscale.
--
-- * 'Rec709' — modern HDTV weights, what most digital tools use today.
-- * 'Bt601'  — older NTSC/PAL weights, what most legacy tools used.
-- * 'Average' — plain @(R + G + B) \/ 3@, no perceptual correction.
data GreyscaleMethod = Rec709 | Bt601 | Average deriving (Show, Eq)

-- | Convert to greyscale using the requested weighting scheme.  Work
-- in linear light so that two equally bright colours produce the same
-- grey.  The alpha channel is preserved.
greyscale :: PixelContainer -> GreyscaleMethod -> PixelContainer
greyscale pc method =
    let (wr, wg, wb) = case method of
            Rec709  -> (0.2126, 0.7152, 0.0722)
            Bt601   -> (0.299 , 0.587 , 0.114 )
            Average -> (1/3   , 1/3   , 1/3   )
    in mapPixels pc $ \r g b a ->
        let yLin = wr * decodeSrgb r + wg * decodeSrgb g + wb * decodeSrgb b
            y    = encodeSrgb yLin
        in (y, y, y, a)

-- | Sepia tone via the classic 3x3 linear-light matrix.  Warm yellows
-- map to themselves, cool colours get pulled toward a brown-orange
-- anchor.  The matrix is applied in linear light — applying it in sRGB
-- produces a muddier result.
sepia :: PixelContainer -> PixelContainer
sepia pc = mapPixels pc $ \r g b a ->
    let rl = decodeSrgb r
        gl = decodeSrgb g
        bl = decodeSrgb b
        rOut = 0.393 * rl + 0.769 * gl + 0.189 * bl
        gOut = 0.349 * rl + 0.686 * gl + 0.168 * bl
        bOut = 0.272 * rl + 0.534 * gl + 0.131 * bl
    in (encodeSrgb rOut, encodeSrgb gOut, encodeSrgb bOut, a)

-- | Apply a generic 3x3 colour matrix in linear light.  The matrix is
-- a list of rows @[[m00, m01, m02], [m10, m11, m12], [m20, m21, m22]]@;
-- each output channel is a weighted sum of the three input channels.
-- This subsumes sepia, hue-rotation via known matrices, and custom
-- channel mixing.
colourMatrix :: PixelContainer -> [[Double]] -> PixelContainer
colourMatrix pc m
    | length m /= 3 || any ((/= 3) . length) m =
        error "colourMatrix: matrix must be exactly 3×3"
    | otherwise = mapPixels pc $ \r g b a ->
    let rl = decodeSrgb r
        gl = decodeSrgb g
        bl = decodeSrgb b
        row i = m !! i
        rOut = row 0 !! 0 * rl + row 0 !! 1 * gl + row 0 !! 2 * bl
        gOut = row 1 !! 0 * rl + row 1 !! 1 * gl + row 1 !! 2 * bl
        bOut = row 2 !! 0 * rl + row 2 !! 1 * gl + row 2 !! 2 * bl
    in (encodeSrgb rOut, encodeSrgb gOut, encodeSrgb bOut, a)

-- | Saturation.  @factor = 0@ produces pure greyscale, @1@ is identity,
-- values above 1 push colours further from grey.  The operation is a
-- linear interpolation between each pixel's linear luminance and its
-- linear colour.
saturate :: PixelContainer -> Double -> PixelContainer
saturate pc factor = mapPixels pc $ \r g b a ->
    let rl = decodeSrgb r
        gl = decodeSrgb g
        bl = decodeSrgb b
        y  = 0.2126 * rl + 0.7152 * gl + 0.0722 * bl
        lerp v = y + factor * (v - y)
    in (encodeSrgb (lerp rl), encodeSrgb (lerp gl), encodeSrgb (lerp bl), a)

-- | Hue rotation (degrees, counter-clockwise on the colour wheel).
-- Converts RGB -> HSV, shifts hue, converts back.  Work is done in
-- linear light so that the rotation is perceptually balanced.
hueRotate :: PixelContainer -> Double -> PixelContainer
hueRotate pc degrees = mapPixels pc $ \r g b a ->
    let rl = decodeSrgb r
        gl = decodeSrgb g
        bl = decodeSrgb b
        (h, s, v) = rgbToHsv rl gl bl
        h'        = wrap360 (h + degrees)
        (r', g', b') = hsvToRgb h' s v
    in (encodeSrgb r', encodeSrgb g', encodeSrgb b', a)

-- | Fold a hue in degrees into @[0, 360)@.
wrap360 :: Double -> Double
wrap360 h =
    let x = h - 360.0 * fromIntegral (floor (h / 360.0) :: Int)
    in if x < 0 then x + 360.0 else x

-- | RGB (in [0, 1]) to HSV (H in degrees [0, 360), S and V in [0, 1]).
rgbToHsv :: Double -> Double -> Double -> (Double, Double, Double)
rgbToHsv r g b =
    let cmax  = max r (max g b)
        cmin  = min r (min g b)
        delta = cmax - cmin
        s     = if cmax < 1e-6 then 0 else delta / cmax
        v     = cmax
        h | delta < 1e-6 = 0
          | cmax == r    = 60 * fmodD ((g - b) / delta) 6
          | cmax == g    = 60 * (((b - r) / delta) + 2)
          | otherwise    = 60 * (((r - g) / delta) + 4)
    in (if h < 0 then h + 360 else h, s, v)

-- | HSV back to RGB.  Inverse of 'rgbToHsv'.  The sector index picks
-- one of six (R,G,B) permutations; each sector is a linear blend of
-- two primaries.
hsvToRgb :: Double -> Double -> Double -> (Double, Double, Double)
hsvToRgb h s v =
    let c      = v * s
        hp     = h / 60.0
        x      = c * (1.0 - abs (fmodD hp 2 - 1))
        m      = v - c
        sector = floor hp :: Int
        (r1, g1, b1) = case sector `mod` 6 of
            0 -> (c, x, 0)
            1 -> (x, c, 0)
            2 -> (0, c, x)
            3 -> (0, x, c)
            4 -> (x, 0, c)
            _ -> (c, 0, x)
    in (r1 + m, g1 + m, b1 + m)

-- | Floating-point modulus that always returns a non-negative result,
-- mirroring the behaviour of Python's @%@ operator.
fmodD :: Double -> Double -> Double
fmodD a b =
    let r = a - b * fromIntegral (floor (a / b) :: Int)
    in if r < 0 then r + b else r

-- ---------------------------------------------------------------------------
-- sRGB <-> linear image transforms
-- ---------------------------------------------------------------------------

-- | Treat each pixel byte as sRGB-encoded, decode to linear [0, 1],
-- and re-pack as an 8-bit byte (@round(linear * 255)@).  Useful when
-- downstream code assumes linear-light bytes — e.g. for blending.
srgbToLinearImage :: PixelContainer -> PixelContainer
srgbToLinearImage pc = mapPixels pc $ \r g b a ->
    let toLin v = clampByteD (decodeSrgb v * 255.0)
    in (toLin r, toLin g, toLin b, a)

-- | Inverse of 'srgbToLinearImage': treat each byte as linear [0, 1]
-- (@byte \/ 255@), encode with the sRGB curve, round back to a byte.
linearToSrgbImage :: PixelContainer -> PixelContainer
linearToSrgbImage pc = mapPixels pc $ \r g b a ->
    let toSrgb v = encodeSrgb (fromIntegral v / 255.0)
    in (toSrgb r, toSrgb g, toSrgb b, a)

-- ---------------------------------------------------------------------------
-- 1D LUTs
-- ---------------------------------------------------------------------------

-- | Apply three independent 1D LUTs to the R, G, B channels.  Each
-- LUT must be 256 entries.  Alpha is left untouched.  Because the
-- table is a pure value, LUT-based grading is embarrassingly simple
-- to compose: chain two LUTs off-line and you get a single LUT.
applyLut1dU8 :: PixelContainer -> [Word8] -> [Word8] -> [Word8] -> PixelContainer
applyLut1dU8 pc lutR lutG lutB
    | length lutR /= 256 || length lutG /= 256 || length lutB /= 256 =
        error "applyLut1dU8: each LUT must have exactly 256 entries"
    | otherwise = mapPixels pc $ \r g b a ->
        ( lutR !! fromIntegral r
        , lutG !! fromIntegral g
        , lutB !! fromIntegral b
        , a
        )

-- | Build a 256-entry LUT from a linear-light function
-- @f :: Double -> Double@ by decoding each byte to linear, applying
-- @f@, and encoding back to sRGB.
buildLut1dU8 :: (Double -> Double) -> [Word8]
buildLut1dU8 f =
    [ encodeSrgb (f (decodeSrgb (fromIntegral i)))
    | i <- [0..255 :: Int]
    ]

-- | Convenience wrapper: build a gamma LUT (@y = x^g@ in linear
-- light).  Equivalent to @buildLut1dU8 (** g)@.
buildGammaLut :: Double -> [Word8]
buildGammaLut g = buildLut1dU8 (\x -> x ** g)
