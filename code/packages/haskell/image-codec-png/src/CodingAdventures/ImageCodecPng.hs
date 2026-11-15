module CodingAdventures.ImageCodecPng
    ( -- * High-level Encoding & Decoding
      encodePng
    , decodePng
    , decodePngBounded
      -- * Adler-32 Checksum
    , adler32
      -- * ImageCodec Type Class Adapter
    , PngCodec(..)
    , defaultPngCodec
    , createPngCodec
      -- * Error Taxonomy (Standing Rule #20)
    , PngError(..)
    , pngErrorCode
    , pngErrorMessage
    , pngErrorCodes
    , allPngErrors
    , pngErrorFromCode
      -- * Portable PNG Constraints & Metadata
    , pngMaxDimension
    , pngDefaultMaxPixels
    , mimeTypePng
    , redactedPlaceholder
    , invalidPlaceholder
    , unknownErrorMessage
    ) where

import CodingAdventures.ImageCodecPng.Adler32 (adler32)
import CodingAdventures.ImageCodecPng.Constants
    ( pngDefaultMaxPixels
    , pngMaxDimension
    )
import CodingAdventures.ImageCodecPng.Decoder
    ( decodePng
    , decodePngBounded
    )
import CodingAdventures.ImageCodecPng.Encoder (encodePng)
import CodingAdventures.ImageCodecPng.Messages
    ( PngError(..)
    , allPngErrors
    , invalidPlaceholder
    , mimeTypePng
    , pngErrorCode
    , pngErrorCodes
    , pngErrorFromCode
    , pngErrorMessage
    , redactedPlaceholder
    , unknownErrorMessage
    )
import CodingAdventures.ImageCodecPng.Types
    ( PngCodec(..)
    , createPngCodec
    , defaultPngCodec
    )
