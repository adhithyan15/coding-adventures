-- | IMG04 — Geometric transforms on a 'PixelContainer'.
--
-- All transforms share the same core strategy: /inverse warping/.
-- Rather than pushing each input pixel forward to a (possibly
-- non-integer) output location, we iterate over the integer output
-- grid and for each output pixel @(x', y')@ compute the non-integer
-- source location @(u, v)@ that maps to it.  Sampling @(u, v)@ is a
-- well-defined interpolation problem, and every output pixel is
-- written exactly once.
--
-- == Pixel-centre convention
--
-- For continuous transforms we treat integer coordinate @n@ as the
-- /centre/ of pixel @n@, so pixel centres live at @0.5, 1.5, 2.5, ...@
-- in continuous space.  Inverse scale becomes
-- @u = (x' + 0.5) \/ sx - 0.5@.  This avoids a half-pixel shift
-- relative to "integer is top-left corner" conventions.
module ImageGeometricTransforms
    ( Interpolation(..)
    , RotateBounds(..)
    , OutOfBounds(..)
    , flipHorizontal
    , flipVertical
    , rotate90CW
    , rotate90CCW
    , rotate180
    , crop
    , scale
    , rotate
    , translate
    , affine
    , perspectiveWarp
    ) where

import PixelContainer
import Data.Word (Word8)
import qualified Data.ByteString as BS

-- ---------------------------------------------------------------------------
-- Modes
-- ---------------------------------------------------------------------------

-- | Interpolation kernel used by continuous transforms.
--
-- * 'Nearest'  — snap to the closest pixel.  Fastest, blocky.
-- * 'Bilinear' — weighted average of 4 nearest pixels.  Good default.
-- * 'Bicubic'  — Catmull-Rom over 16 pixels.  Sharper, more expensive.
data Interpolation = Nearest | Bilinear | Bicubic deriving (Show, Eq)

-- | Canvas policy for arbitrary 'rotate'.
--
-- * 'Fit'  — expand output canvas to hold the rotated bounding box.
-- * 'Crop' — keep the original canvas size, clipping corners.
data RotateBounds = Fit | Crop deriving (Show, Eq)

-- | Out-of-bounds policy when an inverse-warped coordinate falls
-- outside the source image.  Every continuous transform takes one of
-- these.
--
-- * 'Zero'      — return transparent black.
-- * 'Replicate' — clamp to the nearest edge pixel.
-- * 'Reflect'   — mirror back into bounds.
-- * 'Wrap'      — toroidal modulo.
data OutOfBounds = Zero | Replicate | Reflect | Wrap deriving (Show, Eq)

-- ---------------------------------------------------------------------------
-- sRGB <-> linear (duplicated from image-point-ops so this package
-- stays independent — geometric transforms only need linear-light
-- sampling, not the full point-ops catalogue).
-- ---------------------------------------------------------------------------

-- | Precomputed sRGB decode LUT (see 'decodeSrgb').
srgbToLinear :: [Double]
srgbToLinear =
    [ let c = fromIntegral i / 255.0
      in if c <= 0.04045
         then c / 12.92
         else ((c + 0.055) / 1.055) ** 2.4
    | i <- [0..255 :: Int]
    ]

decodeSrgb :: Word8 -> Double
decodeSrgb b = srgbToLinear !! fromIntegral b

encodeSrgb :: Double -> Word8
encodeSrgb linear =
    let c    = max 0.0 (min 1.0 linear)
        srgb = if c <= 0.0031308
               then c * 12.92
               else 1.055 * (c ** (1.0 / 2.4)) - 0.055
    in round (srgb * 255.0)

-- ---------------------------------------------------------------------------
-- Coordinate resolution
-- ---------------------------------------------------------------------------

-- | Resolve an integer index to an in-bounds source index according to
-- the requested 'OutOfBounds' policy.  'Nothing' means "sample is
-- transparent black" and short-circuits the rest of the work.
resolveCoord :: Int -> Int -> OutOfBounds -> Maybe Int
resolveCoord x maxVal oob
    | x >= 0 && x < maxVal = Just x
    | otherwise = case oob of
        Zero      -> Nothing
        Replicate -> Just (max 0 (min (maxVal - 1) x))
        Reflect   ->
            let period = 2 * maxVal
                xm     = x `mod` period
                xm'    = if xm < 0 then xm + period else xm
            in Just (if xm' >= maxVal then period - xm' - 1 else xm')
        Wrap      ->
            let xm = x `mod` maxVal
            in Just (if xm < 0 then xm + maxVal else xm)

-- ---------------------------------------------------------------------------
-- Image builder
-- ---------------------------------------------------------------------------

-- | Construct a 'PixelContainer' of given dimensions by querying a
-- per-pixel function.  One allocation, @O(w * h)@.
buildImage :: Int -> Int
           -> (Int -> Int -> (Word8, Word8, Word8, Word8))
           -> PixelContainer
buildImage w h f =
    let pixList =
            [ let (r, g, b, a) = f x y in [r, g, b, a]
            | y <- [0 .. h - 1], x <- [0 .. w - 1]
            ]
    in PixelContainer w h (BS.pack (concat pixList))

-- ---------------------------------------------------------------------------
-- Lossless transforms
-- ---------------------------------------------------------------------------

-- | Mirror left-right.  Inverse warp: @src_x = width - 1 - x'@.
flipHorizontal :: PixelContainer -> PixelContainer
flipHorizontal src@(PixelContainer w h _) =
    buildImage w h (\x y -> pixelAt src (w - 1 - x) y)

-- | Mirror top-bottom.  Inverse warp: @src_y = height - 1 - y'@.
flipVertical :: PixelContainer -> PixelContainer
flipVertical src@(PixelContainer w h _) =
    buildImage w h (\x y -> pixelAt src x (h - 1 - y))

-- | Rotate 90 degrees clockwise.  Output dimensions swap: the new
-- width is the old height and vice versa.
--
-- Forward map: @(x, y) -> (srcH - 1 - y, x)@.
-- Inverse map: @src_x = y', src_y = srcH - 1 - x'@.
rotate90CW :: PixelContainer -> PixelContainer
rotate90CW src@(PixelContainer w h _) =
    buildImage h w (\x y -> pixelAt src y (h - 1 - x))

-- | Rotate 90 degrees counter-clockwise.  Output dimensions swap.
--
-- Forward map: @(x, y) -> (y, srcW - 1 - x)@.
-- Inverse map: @src_x = srcW - 1 - y', src_y = x'@.
rotate90CCW :: PixelContainer -> PixelContainer
rotate90CCW src@(PixelContainer w h _) =
    buildImage h w (\x y -> pixelAt src (w - 1 - y) x)

-- | Rotate 180 degrees.  Same dimensions.  Equivalent to
-- @flipHorizontal . flipVertical@ but done in one pass.
rotate180 :: PixelContainer -> PixelContainer
rotate180 src@(PixelContainer w h _) =
    buildImage w h (\x y -> pixelAt src (w - 1 - x) (h - 1 - y))

-- | Extract a rectangular sub-region starting at @(x0, y0)@ with size
-- @w x h@.  Pixels outside the source are transparent black.  No
-- interpolation — strict pixel copy.
crop :: PixelContainer -> Int -> Int -> Int -> Int -> PixelContainer
crop src x0 y0 w h = buildImage w h (\x y -> pixelAt src (x + x0) (y + y0))

-- ---------------------------------------------------------------------------
-- Interpolation kernels
-- ---------------------------------------------------------------------------

-- | Sample the source at the integer coordinate produced by rounding
-- @(u, v)@.  If the resolved coordinate is out of bounds and the OOB
-- policy is 'Zero', return transparent black.
sampleNearest :: PixelContainer -> Double -> Double -> OutOfBounds
              -> (Word8, Word8, Word8, Word8)
sampleNearest src@(PixelContainer w h _) u v oob =
    let x = round u :: Int
        y = round v :: Int
    in case (resolveCoord x w oob, resolveCoord y h oob) of
        (Just rx, Just ry) -> pixelAt src rx ry
        _                  -> (0, 0, 0, 0)

-- | Fetch a resolved source pixel or transparent black.
sampleClamp :: PixelContainer -> Int -> Int -> OutOfBounds
            -> (Word8, Word8, Word8, Word8)
sampleClamp src@(PixelContainer w h _) x y oob =
    case (resolveCoord x w oob, resolveCoord y h oob) of
        (Just rx, Just ry) -> pixelAt src rx ry
        _                  -> (0, 0, 0, 0)

-- | Bilinear interpolation.  Blending happens in linear light so that
-- scaling a dark-on-light image does not darken the result.
sampleBilinear :: PixelContainer -> Double -> Double -> OutOfBounds
               -> (Word8, Word8, Word8, Word8)
sampleBilinear src u v oob =
    let x0 = floor u :: Int
        y0 = floor v :: Int
        fx = u - fromIntegral x0
        fy = v - fromIntegral y0
        p00 = sampleClamp src  x0       y0      oob
        p10 = sampleClamp src (x0 + 1)  y0      oob
        p01 = sampleClamp src  x0      (y0 + 1) oob
        p11 = sampleClamp src (x0 + 1) (y0 + 1) oob
        -- Per-channel blend in linear light.
        blend chan =
            let c00 = channelLinear chan p00
                c10 = channelLinear chan p10
                c01 = channelLinear chan p01
                c11 = channelLinear chan p11
                top = c00 * (1 - fx) + c10 * fx
                bot = c01 * (1 - fx) + c11 * fx
            in top * (1 - fy) + bot * fy
        r = encodeSrgb (blend 0)
        g = encodeSrgb (blend 1)
        b = encodeSrgb (blend 2)
        -- Alpha is blended in linear [0, 1] (treating the byte as
        -- linear opacity, which matches how compositors blend alpha).
        a = let a00 = fromIntegral (alphaOf p00) / 255 :: Double
                a10 = fromIntegral (alphaOf p10) / 255
                a01 = fromIntegral (alphaOf p01) / 255
                a11 = fromIntegral (alphaOf p11) / 255
                top = a00 * (1 - fx) + a10 * fx
                bot = a01 * (1 - fx) + a11 * fx
                aBlend = top * (1 - fy) + bot * fy
            in round (max 0 (min 1 aBlend) * 255)
    in (r, g, b, a)
  where
    channelLinear ch (r, g, b, _) = case ch of
        0 -> decodeSrgb r
        1 -> decodeSrgb g
        _ -> decodeSrgb b
    alphaOf (_, _, _, a) = a

-- | Catmull-Rom cubic weighting kernel.
catmullRom :: Double -> Double
catmullRom dRaw =
    let d = abs dRaw
    in if d < 1.0
       then 1.5 * d**3 - 2.5 * d**2 + 1.0
       else if d < 2.0
            then -0.5 * d**3 + 2.5 * d**2 - 4.0 * d + 2.0
            else 0.0

-- | Bicubic (Catmull-Rom) interpolation over a 4x4 neighbourhood.
-- We apply the kernel separably: a 1D cubic across the row, then a
-- 1D cubic across the resulting four column-intermediates.
sampleBicubic :: PixelContainer -> Double -> Double -> OutOfBounds
              -> (Word8, Word8, Word8, Word8)
sampleBicubic src u v oob =
    let x0 = floor u :: Int
        y0 = floor v :: Int
        fx = u - fromIntegral x0
        fy = v - fromIntegral y0
        wx = [catmullRom (fromIntegral (j - 1) - fx) | j <- [0..3 :: Int]]
        wy = [catmullRom (fromIntegral (k - 1) - fy) | k <- [0..3 :: Int]]
        cell kk jj =
            sampleClamp src (x0 - 1 + jj) (y0 - 1 + kk) oob
        row k = [cell k j | j <- [0..3 :: Int]]
        accumChan chan =
            let rowLin k = [channelLinear chan p | p <- row k]
                hBlends  = [sum (zipWith (*) wx (rowLin k)) | k <- [0..3 :: Int]]
                total    = sum (zipWith (*) wy hBlends)
            in total
        alphaLin p = fromIntegral (alphaOf p) / 255 :: Double
        accumAlpha =
            let rowLin k = [alphaLin p | p <- row k]
                hBlends  = [sum (zipWith (*) wx (rowLin k)) | k <- [0..3 :: Int]]
            in sum (zipWith (*) wy hBlends)
        r = encodeSrgb (accumChan 0)
        g = encodeSrgb (accumChan 1)
        b = encodeSrgb (accumChan 2)
        a = round (max 0 (min 1 accumAlpha) * 255)
    in (r, g, b, a)
  where
    channelLinear ch (r, g, b, _) = case ch of
        0 -> decodeSrgb r
        1 -> decodeSrgb g
        _ -> decodeSrgb b
    alphaOf (_, _, _, a) = a

-- | Dispatch on the interpolation mode.
sample :: Interpolation -> PixelContainer -> Double -> Double -> OutOfBounds
       -> (Word8, Word8, Word8, Word8)
sample Nearest  = sampleNearest
sample Bilinear = sampleBilinear
sample Bicubic  = sampleBicubic

-- ---------------------------------------------------------------------------
-- Continuous transforms
-- ---------------------------------------------------------------------------

-- | Resample to new dimensions.  Pixel-centre convention:
-- @u = (x' + 0.5) \/ sx - 0.5@ where @sx = outW \/ srcW@.
scale :: PixelContainer -> Int -> Int -> Interpolation -> OutOfBounds
      -> PixelContainer
scale src@(PixelContainer sw sh _) outW outH interp oob =
    let sx = fromIntegral outW / fromIntegral sw :: Double
        sy = fromIntegral outH / fromIntegral sh
    in buildImage outW outH $ \x y ->
        let u = (fromIntegral x + 0.5) / sx - 0.5
            v = (fromIntegral y + 0.5) / sy - 0.5
        in sample interp src u v oob

-- | Rotate counter-clockwise by @degrees@.  'Fit' expands the canvas
-- to hold the rotated bounding box; 'Crop' keeps the original size
-- and lets corners fall outside.
--
-- Rotation is centred on the image midpoint.  The forward map is
-- standard 2D rotation; we inverse-warp by rotating by @-degrees@.
rotate :: PixelContainer -> Double -> RotateBounds -> Interpolation
       -> OutOfBounds -> PixelContainer
rotate src@(PixelContainer sw sh _) degrees bounds interp oob =
    let theta = degrees * pi / 180.0
        c     = cos theta
        s     = sin theta
        sw'   = fromIntegral sw :: Double
        sh'   = fromIntegral sh :: Double
        (outW, outH) = case bounds of
            Crop -> (sw, sh)
            Fit  ->
                -- Bounding box of the rotated canvas.  We subtract a
                -- tiny epsilon before 'ceiling' so that numerically
                -- exact results (e.g. a 90-degree rotation that gives
                -- 2.0 + 6e-17) do not accidentally round up to the
                -- next integer.
                let newW = abs (sw' * c) + abs (sh' * s)
                    newH = abs (sw' * s) + abs (sh' * c)
                    eps  = 1e-9
                in ( max 1 (ceiling (newW - eps))
                   , max 1 (ceiling (newH - eps))
                   )
        cx = sw' / 2.0
        cy = sh' / 2.0
        ox = fromIntegral outW / 2.0 :: Double
        oy = fromIntegral outH / 2.0 :: Double
    in buildImage outW outH $ \x y ->
        let dx = fromIntegral x + 0.5 - ox
            dy = fromIntegral y + 0.5 - oy
            -- Inverse rotation (rotate by -theta).
            u  =  c * dx + s * dy + cx - 0.5
            v  = -s * dx + c * dy + cy - 0.5
        in sample interp src u v oob

-- | Translate (shift) by @(tx, ty)@ in pixel units.  Fractional shifts
-- go through the chosen interpolation kernel.
translate :: PixelContainer -> Double -> Double -> Interpolation -> OutOfBounds
          -> PixelContainer
translate src@(PixelContainer sw sh _) tx ty interp oob =
    buildImage sw sh $ \x y ->
        let u = fromIntegral x - tx
            v = fromIntegral y - ty
        in sample interp src u v oob

-- | Invert a 2x3 forward affine matrix @[[a,b,c],[d,e,f]]@ so we can
-- inverse-warp.  The 2x2 part is inverted classically; the
-- translation part is carried through separately when applying.
invertAffine :: [[Double]] -> (Double, Double, Double, Double, Double, Double)
invertAffine m =
    let a = m !! 0 !! 0 ; b = m !! 0 !! 1 ; c = m !! 0 !! 2
        d = m !! 1 !! 0 ; e = m !! 1 !! 1 ; f = m !! 1 !! 2
        det = a * e - b * d
        ia =  e / det
        ib = -b / det
        ic = -d / det
        ie =  a / det
    in (ia, ib, ic, ie, c, f)

-- | Apply a 2x3 forward affine matrix to each output pixel.  The
-- matrix is inverted internally for inverse-warp sampling.  Output
-- has the same dimensions as the input; callers who want a different
-- canvas size should follow up with 'crop' or 'scale'.
affine :: PixelContainer -> [[Double]] -> Interpolation -> OutOfBounds
       -> PixelContainer
affine src@(PixelContainer sw sh _) m interp oob =
    let (ia, ib, ic, ie, c, f) = invertAffine m
    in buildImage sw sh $ \x y ->
        let xp = fromIntegral x - c
            yp = fromIntegral y - f
            u  = ia * xp + ib * yp
            v  = ic * xp + ie * yp
        in sample interp src u v oob

-- | Invert a 3x3 matrix by the cofactor / determinant rule.  Returns
-- a fresh @[[Double]]@.
invert3x3 :: [[Double]] -> [[Double]]
invert3x3 m =
    let a = m !! 0 !! 0 ; b = m !! 0 !! 1 ; c = m !! 0 !! 2
        d = m !! 1 !! 0 ; e = m !! 1 !! 1 ; f = m !! 1 !! 2
        g = m !! 2 !! 0 ; h = m !! 2 !! 1 ; i = m !! 2 !! 2
        det = a * (e * i - f * h)
            - b * (d * i - f * g)
            + c * (d * h - e * g)
    in [ [  (e * i - f * h) / det
         , -(b * i - c * h) / det
         ,  (b * f - c * e) / det ]
       , [ -(d * i - f * g) / det
         ,  (a * i - c * g) / det
         , -(a * f - c * d) / det ]
       , [  (d * h - e * g) / det
         , -(a * h - b * g) / det
         ,  (a * e - b * d) / det ]
       ]

-- | Apply a 3x3 forward perspective (homography) matrix.  Inverted
-- internally for inverse-warp sampling; the homogeneous denominator
-- @w_h@ is divided out per pixel to get the affine-looking
-- @(u, v)@ coordinates.
perspectiveWarp :: PixelContainer -> [[Double]] -> Interpolation -> OutOfBounds
                -> PixelContainer
perspectiveWarp src@(PixelContainer sw sh _) m interp oob =
    let inv = invert3x3 m
    in buildImage sw sh $ \x y ->
        let xp = fromIntegral x :: Double
            yp = fromIntegral y :: Double
            uh = inv !! 0 !! 0 * xp + inv !! 0 !! 1 * yp + inv !! 0 !! 2
            vh = inv !! 1 !! 0 * xp + inv !! 1 !! 1 * yp + inv !! 1 !! 2
            wh = inv !! 2 !! 0 * xp + inv !! 2 !! 1 * yp + inv !! 2 !! 2
        in if abs wh < 1e-12
           then (0, 0, 0, 0)
           else sample interp src (uh / wh) (vh / wh) oob
