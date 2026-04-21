-- | IC00 — Universal RGBA8 Pixel Buffer
--
-- 'PixelContainer' is the zero-dependency foundation for the
-- coding-adventures image processing stack.  Every image codec and
-- processing stage depends only on this module, which keeps the stack
-- modular: codecs do not know about point operations, point operations
-- do not know about codecs, and neither knows about the other.
--
-- == Layout
--
-- Pixels are stored row-major, top-left origin, RGBA interleaved:
--
-- @
-- offset = (y * width + x) * 4
-- data[offset + 0] = R
-- data[offset + 1] = G
-- data[offset + 2] = B
-- data[offset + 3] = A
-- @
--
-- Channel count and bit depth are fixed: 4 channels, 8 bits each (RGBA8).
-- This is the same layout used by web canvases, most GPU textures, and
-- the Paint-VM internal framebuffer.  By fixing the layout at the lowest
-- layer we avoid a combinatorial explosion of pixel-format conversions
-- further up the stack.
module PixelContainer
    ( PixelContainer(..)
    , ImageCodec(..)
    , createPixelContainer
    , pixelAt
    , setPixel
    , fillPixels
    ) where

import qualified Data.ByteString as BS
import Data.Word (Word8)

-- | A fixed-format RGBA8 pixel buffer.
--
-- 'pcPixels' is a flat 'BS.ByteString' of @width × height × 4@ bytes.
-- We use a strict 'BS.ByteString' rather than a list of tuples because
-- the underlying memory is a single contiguous chunk: random access is
-- O(1), memory footprint is predictable, and we can hand the buffer
-- straight to a codec or the GPU without a conversion pass.
data PixelContainer = PixelContainer
    { pcWidth  :: !Int           -- ^ Image width in pixels.
    , pcHeight :: !Int           -- ^ Image height in pixels.
    , pcPixels :: !BS.ByteString -- ^ Raw RGBA bytes, row-major.
    } deriving (Show, Eq)

-- | Abstract codec interface.  Concrete codecs (PNG, BMP, PPM, …) live
-- in their own packages and implement this class.  Keeping the type
-- class in this module means there is a single, canonical contract:
-- if you hold any @a@ with 'ImageCodec' you can encode or decode.
class ImageCodec a where
    -- | The IANA MIME type, e.g. @"image\/png"@.
    mimeType :: a -> String
    -- | Encode a 'PixelContainer' into raw codec bytes (file contents).
    encode   :: a -> PixelContainer -> BS.ByteString
    -- | Decode raw codec bytes back into a 'PixelContainer'.
    decode   :: a -> BS.ByteString  -> PixelContainer

-- | Create a new 'PixelContainer' filled with transparent black (all
-- zeros).  This is the canonical empty canvas used by higher layers.
--
-- A zero-sized container (@0×0@, @0×h@, @w×0@) is legal and stores an
-- empty 'BS.ByteString'.  This keeps edge cases (e.g. cropping to an
-- empty region) uniform.
createPixelContainer :: Int -> Int -> PixelContainer
createPixelContainer w h =
    PixelContainer w h (BS.replicate (w * h * 4) 0)

-- | Read the RGBA components at pixel column @x@, row @y@.
--
-- Out-of-bounds coordinates return @(0, 0, 0, 0)@ — i.e. transparent
-- black.  This sentinel is used by geometric transforms as a cheap OOB
-- fallback; callers who need a different behaviour (clamp, reflect,
-- wrap) should resolve coordinates themselves before calling.
pixelAt :: PixelContainer -> Int -> Int -> (Word8, Word8, Word8, Word8)
pixelAt (PixelContainer w h pixels) x y
    | x < 0 || x >= w || y < 0 || y >= h = (0, 0, 0, 0)
    | otherwise =
        let i = (y * w + x) * 4
        in ( BS.index pixels  i
           , BS.index pixels (i + 1)
           , BS.index pixels (i + 2)
           , BS.index pixels (i + 3)
           )

-- | Write the RGBA components at pixel column @x@, row @y@.
--
-- No-op for out-of-bounds coordinates.  Returns a new 'PixelContainer'
-- (the old one is unmodified) because 'BS.ByteString' is immutable.
-- This is O(n) in the buffer size; callers doing bulk work should use
-- 'mapPixels'-style helpers in higher-level packages, which build the
-- new buffer once.
setPixel :: PixelContainer
         -> Int  -- ^ Column (x).
         -> Int  -- ^ Row (y).
         -> Word8 -> Word8 -> Word8 -> Word8
         -> PixelContainer
setPixel pc@(PixelContainer w h pixels) x y r g b a
    | x < 0 || x >= w || y < 0 || y >= h = pc
    | otherwise =
        let i      = (y * w + x) * 4
            before = BS.take i pixels
            after  = BS.drop (i + 4) pixels
        in PixelContainer w h (before <> BS.pack [r, g, b, a] <> after)

-- | Set every pixel in the container to the given RGBA colour.  This
-- is the standard "clear the canvas" primitive; we build the new
-- 'BS.ByteString' by packing the 4-byte colour once and concatenating
-- @w × h@ copies of it, which avoids re-encoding per pixel.
fillPixels :: PixelContainer -> Word8 -> Word8 -> Word8 -> Word8 -> PixelContainer
fillPixels (PixelContainer w h _) r g b a =
    let pixel = BS.pack [r, g, b, a]
    in PixelContainer w h (BS.concat (replicate (w * h) pixel))
