module CodingAdventures.ImageCodecPng.Types
    ( PngError(..)
    , PngCodec(..)
    , defaultPngCodec
    , createPngCodec
    ) where

import Control.Exception (throw)
import PixelContainer (ImageCodec(..))

import CodingAdventures.ImageCodecPng.Constants (pngDefaultMaxPixels)
import CodingAdventures.ImageCodecPng.Decoder (decodePngBounded)
import CodingAdventures.ImageCodecPng.Encoder (encodePng)
import CodingAdventures.ImageCodecPng.Messages
    ( PngError(..)
    , mimeTypePng
    )

-- | Stateful image codec instance for PNG image data with an eagerly validated pixel ceiling.
data PngCodec = PngCodec
    { pngCodecMaxPixels :: !Int
    } deriving (Eq, Show)

-- | Abstract 'ImageCodec' type class instance for 'PngCodec'.
instance ImageCodec PngCodec where
    mimeType _ = mimeTypePng
    encode _ pc = case encodePng pc of
        Right bs -> bs
        Left err -> throw err
    decode (PngCodec mp) bs = case decodePngBounded mp bs of
        Right pc -> pc
        Left err -> throw err

-- | Create a 'PngCodec' with the standard default pixel limit (32 megapixels).
defaultPngCodec :: PngCodec
defaultPngCodec = PngCodec pngDefaultMaxPixels

-- | Create a 'PngCodec' with a custom positive pixel limit up to 'pngDefaultMaxPixels'.
createPngCodec :: Int -> Either PngError PngCodec
createPngCodec maxPix
    | maxPix <= 0 || maxPix > pngDefaultMaxPixels = Left InvalidMaxPixels
    | otherwise = Right (PngCodec maxPix)
