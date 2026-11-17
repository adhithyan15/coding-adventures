module CodingAdventures.ImageCodecPng.Constants
    ( pngMaxDimension
    , pngDefaultMaxPixels
    , pngSignature
    , pngSignatureLength
    , adlerModulus
    , adlerChunkSize
    , ihdrExpectedLength
    , bitDepth8
    , colourTypeGreyscale
    , colourTypeTruecolour
    , colourTypeIndexed
    , colourTypeGreyscaleAlpha
    , colourTypeTruecolourAlpha
    , compressionDeflate
    , filterAdaptive
    , interlaceNone
    , zlibMinHeaderLength
    , zlibCmfMethodDeflate
    , zlibCmfMaxWindowBits
    , zlibHeaderMultiple
    , zlibFlgPresetDictMask
    , zlibHeaderCmfDefault
    , zlibHeaderFlgDefault
    , zlibCmfMethodMask
    , zlibCmfWindowBitsShift
    , zlibHeaderBytes
    , zlibTrailerBytes
    , zlibHeaderMultiplier
    , chunkLengthFieldSize
    , chunkTypeFieldSize
    , chunkCrcFieldSize
    , chunkOverhead
    , channelsGreyscale
    , channelsTruecolour
    , channelsGreyscaleAlpha
    , channelsTruecolourAlpha
    , bytesPerPixelRgba
    , filterNone
    , filterSub
    , filterUp
    , filterAverage
    , filterPaeth
    , filterMaxValid
    , averageFilterDivisor
    , byteMaskInt
    , signedByteThreshold
    , byteModulus
    , maxUint8Sample
    , opaqueAlpha
    , transparentAlpha
    , ihdrWidthOffset
    , ihdrHeightOffset
    , ihdrBitDepthOffset
    , ihdrColourTypeOffset
    , ihdrCompressionOffset
    , ihdrFilterOffset
    , ihdrInterlaceOffset
    , trnsRedOffset
    , trnsGreenOffset
    , trnsBlueOffset
    , plteMinBytes
    , plteMaxBytes
    , plteEntrySize
    , trnsGreyscaleBytes
    , trnsTruecolourBytes
    , byteMask
    , byteShift8
    , byteShift16
    , byteShift24
    , asciiLetterUpperMin
    , asciiLetterUpperMax
    , asciiLetterLowerMin
    , asciiLetterLowerMax
    , chunkReservedBitMask
    , chunkAncillaryBitMask
    ) where

import Data.ByteString (ByteString)
import qualified Data.ByteString as BS
import Data.Word (Word8, Word16, Word32)

-- | Largest width or height accepted by the portable PNG profile.
pngMaxDimension :: Int
pngMaxDimension = 16384

-- | Default and hard pixel-count ceiling (32 megapixels).
pngDefaultMaxPixels :: Int
pngDefaultMaxPixels = 33554432

-- | Canonical 8-byte PNG file signature (RFC 2083 Section 3.1).
pngSignature :: ByteString
pngSignature = BS.pack [0x89, 0x50, 0x4E, 0x47, 0x0D, 0x0A, 0x1A, 0x0A]

-- | Length of the canonical PNG signature in bytes.
pngSignatureLength :: Int
pngSignatureLength = 8

-- | RFC 1950 Adler-32 prime modulus.
adlerModulus :: Word32
adlerModulus = 65521

-- | Largest chunk size before intermediate Adler accumulator can overflow Word64.
adlerChunkSize :: Int
adlerChunkSize = 5552

-- | Length of the IHDR chunk payload.
ihdrExpectedLength :: Int
ihdrExpectedLength = 13

-- | Supported bit depth for IC18 profile (8-bit samples).
bitDepth8 :: Word8
bitDepth8 = 8

-- | PNG colour types.
colourTypeGreyscale :: Word8
colourTypeGreyscale = 0

colourTypeTruecolour :: Word8
colourTypeTruecolour = 2

colourTypeIndexed :: Word8
colourTypeIndexed = 3

colourTypeGreyscaleAlpha :: Word8
colourTypeGreyscaleAlpha = 4

colourTypeTruecolourAlpha :: Word8
colourTypeTruecolourAlpha = 6

-- | RFC 2083 standard compression method (0 = DEFLATE).
compressionDeflate :: Word8
compressionDeflate = 0

-- | RFC 2083 standard filter method (0 = adaptive filtering with 5 basic filter types).
filterAdaptive :: Word8
filterAdaptive = 0

-- | RFC 2083 standard transmission order (0 = non-interlaced).
interlaceNone :: Word8
interlaceNone = 0

-- | Minimum zlib container header length (2-byte header + 4-byte Adler trailer).
zlibMinHeaderLength :: Int
zlibMinHeaderLength = 6

zlibCmfMethodDeflate :: Word8
zlibCmfMethodDeflate = 8

zlibCmfMaxWindowBits :: Word8
zlibCmfMaxWindowBits = 7

zlibHeaderMultiple :: Int
zlibHeaderMultiple = 31

zlibFlgPresetDictMask :: Word8
zlibFlgPresetDictMask = 0x20

zlibHeaderCmfDefault :: Word8
zlibHeaderCmfDefault = 0x78

zlibHeaderFlgDefault :: Word8
zlibHeaderFlgDefault = 0x9C

zlibCmfMethodMask :: Word8
zlibCmfMethodMask = 0x0F

zlibCmfWindowBitsShift :: Int
zlibCmfWindowBitsShift = 4

zlibHeaderBytes :: Int
zlibHeaderBytes = 2

zlibTrailerBytes :: Int
zlibTrailerBytes = 4

zlibHeaderMultiplier :: Int
zlibHeaderMultiplier = 256

chunkLengthFieldSize :: Int
chunkLengthFieldSize = 4

chunkTypeFieldSize :: Int
chunkTypeFieldSize = 4

chunkCrcFieldSize :: Int
chunkCrcFieldSize = 4

-- | Total framing overhead for a chunk: length (4) + type (4) + crc (4).
chunkOverhead :: Int
chunkOverhead = 12

channelsGreyscale :: Int
channelsGreyscale = 1

channelsTruecolour :: Int
channelsTruecolour = 3

channelsGreyscaleAlpha :: Int
channelsGreyscaleAlpha = 2

channelsTruecolourAlpha :: Int
channelsTruecolourAlpha = 4

bytesPerPixelRgba :: Int
bytesPerPixelRgba = 4

-- | 5 normative scanline filter types (RFC 2083 Section 6).
filterNone :: Word8
filterNone = 0

filterSub :: Word8
filterSub = 1

filterUp :: Word8
filterUp = 2

filterAverage :: Word8
filterAverage = 3

filterPaeth :: Word8
filterPaeth = 4

filterMaxValid :: Word8
filterMaxValid = 4

averageFilterDivisor :: Int
averageFilterDivisor = 2

byteMaskInt :: Int
byteMaskInt = 0xFF

signedByteThreshold :: Int
signedByteThreshold = 128

byteModulus :: Int
byteModulus = 256

maxUint8Sample :: Word16
maxUint8Sample = 255

opaqueAlpha :: Word8
opaqueAlpha = 255

transparentAlpha :: Word8
transparentAlpha = 0

ihdrWidthOffset :: Int
ihdrWidthOffset = 0

ihdrHeightOffset :: Int
ihdrHeightOffset = 4

ihdrBitDepthOffset :: Int
ihdrBitDepthOffset = 8

ihdrColourTypeOffset :: Int
ihdrColourTypeOffset = 9

ihdrCompressionOffset :: Int
ihdrCompressionOffset = 10

ihdrFilterOffset :: Int
ihdrFilterOffset = 11

ihdrInterlaceOffset :: Int
ihdrInterlaceOffset = 12

trnsRedOffset :: Int
trnsRedOffset = 0

trnsGreenOffset :: Int
trnsGreenOffset = 2

trnsBlueOffset :: Int
trnsBlueOffset = 4

plteMinBytes :: Int
plteMinBytes = 3

plteMaxBytes :: Int
plteMaxBytes = 768

plteEntrySize :: Int
plteEntrySize = 3

trnsGreyscaleBytes :: Int
trnsGreyscaleBytes = 2

trnsTruecolourBytes :: Int
trnsTruecolourBytes = 6

byteMask :: Word32
byteMask = 0xFF

byteShift8 :: Int
byteShift8 = 8

byteShift16 :: Int
byteShift16 = 16

byteShift24 :: Int
byteShift24 = 24

asciiLetterUpperMin :: Word8
asciiLetterUpperMin = 0x41

asciiLetterUpperMax :: Word8
asciiLetterUpperMax = 0x5A

asciiLetterLowerMin :: Word8
asciiLetterLowerMin = 0x61

asciiLetterLowerMax :: Word8
asciiLetterLowerMax = 0x7A

chunkReservedBitMask :: Word8
chunkReservedBitMask = 0x20

chunkAncillaryBitMask :: Word8
chunkAncillaryBitMask = 0x20
