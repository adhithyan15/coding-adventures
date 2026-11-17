module CodingAdventures.ImageCodecPng.Messages
    ( PngError(..)
    , pngErrorCode
    , pngErrorMessage
    , pngErrorCodes
    , allPngErrors
    , pngErrorFromCode
    , mimeTypePng
    , redactedPlaceholder
    , invalidPlaceholder
    , unknownErrorMessage
    , tagIhdr
    , tagIdat
    , tagIend
    , tagPlte
    , tagTrns
    , tagAcTl
    , tagFcTl
    , tagFdAt
    , nameIhdr
    , nameIdat
    , nameIend
    , namePlte
    , nameTrns
    ) where

import Control.Exception (Exception)
import Data.ByteString (ByteString)
import qualified Data.ByteString as BS

-- | Closed normative error taxonomy for IC18 PNG operations.
--
-- Standing Rule #20: 100% of error messages, diagnostic strings, and placeholders
-- are strictly isolated in this module and guaranteed payload-blind.
data PngError
    = InvalidMaxPixels
    | InvalidImageDimensions
    | InvalidPixelDataLength
    | FileTooShort
    | InvalidSignature
    | TruncatedChunk
    | InvalidChunkType
    | ChunkCrcMismatch
    | ChunkBeforeIhdr
    | DuplicateIhdr
    | InvalidIhdrLength
    | InvalidDimensions
    | DimensionLimit
    | PixelLimit
    | UnsupportedFeature
    | InvalidPlte
    | InvalidTrns
    | NonconsecutiveIdat
    | InvalidIend
    | TrailingData
    | UnknownCriticalChunk
    | MissingRequiredChunk
    | InvalidZlibHeader
    | PresetDictionary
    | InflateFailed
    | InflatedLengthMismatch
    | IdatCavity
    | AdlerMismatch
    | InvalidFilter
    deriving (Eq, Enum, Bounded)

instance Show PngError where
    show = pngErrorMessage

instance Exception PngError

-- | Stable hyphenated error identifier matching IC18 fixtures.
pngErrorCode :: PngError -> String
pngErrorCode err = case err of
    InvalidMaxPixels       -> "invalid-max-pixels"
    InvalidImageDimensions -> "invalid-image-dimensions"
    InvalidPixelDataLength -> "invalid-pixel-data-length"
    FileTooShort           -> "file-too-short"
    InvalidSignature       -> "invalid-signature"
    TruncatedChunk         -> "truncated-chunk"
    InvalidChunkType       -> "invalid-chunk-type"
    ChunkCrcMismatch       -> "chunk-crc-mismatch"
    ChunkBeforeIhdr        -> "chunk-before-ihdr"
    DuplicateIhdr          -> "duplicate-ihdr"
    InvalidIhdrLength      -> "invalid-ihdr-length"
    InvalidDimensions      -> "invalid-dimensions"
    DimensionLimit         -> "dimension-limit"
    PixelLimit             -> "pixel-limit"
    UnsupportedFeature     -> "unsupported-feature"
    InvalidPlte            -> "invalid-plte"
    InvalidTrns            -> "invalid-trns"
    NonconsecutiveIdat     -> "nonconsecutive-idat"
    InvalidIend            -> "invalid-iend"
    TrailingData           -> "trailing-data"
    UnknownCriticalChunk   -> "unknown-critical-chunk"
    MissingRequiredChunk   -> "missing-required-chunk"
    InvalidZlibHeader      -> "invalid-zlib-header"
    PresetDictionary       -> "preset-dictionary"
    InflateFailed          -> "inflate-failed"
    InflatedLengthMismatch -> "inflated-length-mismatch"
    IdatCavity             -> "idat-cavity"
    AdlerMismatch          -> "adler-mismatch"
    InvalidFilter          -> "invalid-filter"

-- | Localized, payload-blind human diagnostic message.
pngErrorMessage :: PngError -> String
pngErrorMessage err = case err of
    InvalidMaxPixels       -> "image-codec-png: max_pixels must be a positive integer <= 33554432"
    InvalidImageDimensions -> "image-codec-png: image dimensions must be positive integers within limit"
    InvalidPixelDataLength -> "image-codec-png: pixel buffer length does not match width * height * 4"
    FileTooShort           -> "image-codec-png: input too short to contain PNG signature"
    InvalidSignature       -> "image-codec-png: invalid PNG signature"
    TruncatedChunk         -> "image-codec-png: chunk header or payload truncated"
    InvalidChunkType       -> "image-codec-png: invalid chunk type name"
    ChunkCrcMismatch       -> "image-codec-png: chunk CRC mismatch"
    ChunkBeforeIhdr        -> "image-codec-png: chunk found before IHDR"
    DuplicateIhdr          -> "image-codec-png: duplicate IHDR chunk"
    InvalidIhdrLength      -> "image-codec-png: IHDR length must be exactly 13 bytes"
    InvalidDimensions      -> "image-codec-png: image dimensions must be non-zero"
    DimensionLimit         -> "image-codec-png: image dimensions exceed limit"
    PixelLimit             -> "image-codec-png: image pixel count exceeds limit"
    UnsupportedFeature     -> "image-codec-png: unsupported PNG feature or APNG chunk"
    InvalidPlte            -> "image-codec-png: invalid, duplicate, or misplaced PLTE chunk"
    InvalidTrns            -> "image-codec-png: invalid, duplicate, or misplaced tRNS chunk"
    NonconsecutiveIdat     -> "image-codec-png: non-consecutive IDAT chunks"
    InvalidIend            -> "image-codec-png: IEND chunk must be empty"
    TrailingData           -> "image-codec-png: trailing data found after IEND chunk"
    UnknownCriticalChunk   -> "image-codec-png: unknown critical chunk"
    MissingRequiredChunk   -> "image-codec-png: missing required chunk (IHDR, IDAT, or IEND)"
    InvalidZlibHeader      -> "image-codec-png: invalid zlib header in IDAT"
    PresetDictionary       -> "image-codec-png: preset dictionary is unsupported"
    InflateFailed          -> "image-codec-png: DEFLATE decompression failed"
    InflatedLengthMismatch -> "image-codec-png: decompressed length does not match expected scanline size"
    IdatCavity             -> "image-codec-png: unconsumed bytes between compressed data and Adler-32"
    AdlerMismatch          -> "image-codec-png: Adler-32 checksum mismatch"
    InvalidFilter          -> "image-codec-png: invalid scanline filter type"

-- | List of all 29 error constructors in normative order.
allPngErrors :: [PngError]
allPngErrors = [minBound .. maxBound]

-- | List of all 29 hyphenated error identifiers.
pngErrorCodes :: [String]
pngErrorCodes = map pngErrorCode allPngErrors

-- | Lookup an error constructor by its hyphenated error code.
pngErrorFromCode :: String -> Maybe PngError
pngErrorFromCode code = lookup code (zip pngErrorCodes allPngErrors)

-- | IANA MIME type for PNG.
mimeTypePng :: String
mimeTypePng = "image/png"

-- | Standing Rule #20 placeholder strings.
redactedPlaceholder :: String
redactedPlaceholder = "<redacted>"

invalidPlaceholder :: String
invalidPlaceholder = "<invalid>"

unknownErrorMessage :: String
unknownErrorMessage = "image-codec-png: unknown error"

-- | 4-byte chunk type tags as raw ByteStrings.
tagIhdr :: ByteString
tagIhdr = BS.pack [0x49, 0x48, 0x44, 0x52] -- "IHDR"

tagIdat :: ByteString
tagIdat = BS.pack [0x49, 0x44, 0x41, 0x54] -- "IDAT"

tagIend :: ByteString
tagIend = BS.pack [0x49, 0x45, 0x4E, 0x44] -- "IEND"

tagPlte :: ByteString
tagPlte = BS.pack [0x50, 0x4C, 0x54, 0x45] -- "PLTE"

tagTrns :: ByteString
tagTrns = BS.pack [0x74, 0x52, 0x4E, 0x53] -- "tRNS"

tagAcTl :: ByteString
tagAcTl = BS.pack [0x61, 0x63, 0x54, 0x4C] -- "acTL"

tagFcTl :: ByteString
tagFcTl = BS.pack [0x66, 0x63, 0x54, 0x4C] -- "fcTL"

tagFdAt :: ByteString
tagFdAt = BS.pack [0x66, 0x64, 0x41, 0x54] -- "fdAT"

-- | Chunk type names as human-readable strings.
nameIhdr :: String
nameIhdr = "IHDR"

nameIdat :: String
nameIdat = "IDAT"

nameIend :: String
nameIend = "IEND"

namePlte :: String
namePlte = "PLTE"

nameTrns :: String
nameTrns = "tRNS"
