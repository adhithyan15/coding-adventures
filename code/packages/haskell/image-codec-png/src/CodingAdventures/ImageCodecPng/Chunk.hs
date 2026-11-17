{-# LANGUAGE BangPatterns #-}
module CodingAdventures.ImageCodecPng.Chunk
    ( readWord32BE
    , readWord16BE
    , word32ToBytesBE
    , validChunkType
    , isCriticalChunk
    , makeChunk
    , calculateCrc
    ) where

import Data.Bits ((.&.), (.|.), shiftL, shiftR)
import Data.ByteString (ByteString)
import qualified Data.ByteString as BS
import Data.Word (Word8, Word16, Word32)
import qualified Zip

import CodingAdventures.ImageCodecPng.Constants
    ( asciiLetterLowerMax
    , asciiLetterLowerMin
    , asciiLetterUpperMax
    , asciiLetterUpperMin
    , byteMask
    , byteShift16
    , byteShift24
    , byteShift8
    , chunkAncillaryBitMask
    , chunkReservedBitMask
    , chunkTypeFieldSize
    )

-- | Read a 32-bit unsigned big-endian integer from a ByteString at the specified offset.
readWord32BE :: ByteString -> Int -> Word32
readWord32BE bs offset =
    let !b0 = fromIntegral (BS.index bs offset) :: Word32
        !b1 = fromIntegral (BS.index bs (offset + 1)) :: Word32
        !b2 = fromIntegral (BS.index bs (offset + 2)) :: Word32
        !b3 = fromIntegral (BS.index bs (offset + 3)) :: Word32
    in (b0 `shiftL` byteShift24) .|. (b1 `shiftL` byteShift16) .|. (b2 `shiftL` byteShift8) .|. b3

-- | Read a 16-bit unsigned big-endian integer from a ByteString at the specified offset.
readWord16BE :: ByteString -> Int -> Word16
readWord16BE bs offset =
    let !b0 = fromIntegral (BS.index bs offset) :: Word16
        !b1 = fromIntegral (BS.index bs (offset + 1)) :: Word16
    in (b0 `shiftL` byteShift8) .|. b1

-- | Serialize a 32-bit unsigned integer to a 4-byte big-endian ByteString.
word32ToBytesBE :: Word32 -> ByteString
word32ToBytesBE w = BS.pack
    [ fromIntegral ((w `shiftR` byteShift24) .&. byteMask)
    , fromIntegral ((w `shiftR` byteShift16) .&. byteMask)
    , fromIntegral ((w `shiftR` byteShift8) .&. byteMask)
    , fromIntegral (w .&. byteMask)
    ]

-- | Check if a 4-byte chunk type is valid per RFC 2083 Section 3.2.
-- All characters must be ASCII letters, and the reserved bit (byte 2, bit 5) must be 0.
validChunkType :: ByteString -> Bool
validChunkType ctype
    | BS.length ctype /= chunkTypeFieldSize = False
    | (BS.index ctype 2 .&. chunkReservedBitMask) /= 0 = False
    | otherwise = BS.all isAsciiLetter ctype
  where
    isAsciiLetter :: Word8 -> Bool
    isAsciiLetter b =
        (b >= asciiLetterUpperMin && b <= asciiLetterUpperMax) ||
        (b >= asciiLetterLowerMin && b <= asciiLetterLowerMax)

-- | Determine whether a chunk is critical based on bit 5 of the first byte.
isCriticalChunk :: ByteString -> Bool
isCriticalChunk ctype =
    BS.length ctype >= 1 && (BS.index ctype 0 .&. chunkAncillaryBitMask) == 0

-- | Calculate CRC-32 for a chunk given its 4-byte type and payload.
calculateCrc :: ByteString -> ByteString -> Word32
calculateCrc ctype cdata =
    let typeCrc = Zip.crc32 ctype 0
    in Zip.crc32 cdata typeCrc

-- | Frame a chunk with length, type, payload, and CRC-32.
makeChunk :: ByteString -> ByteString -> ByteString
makeChunk ctype cdata =
    let len = fromIntegral (BS.length cdata) :: Word32
        crc = calculateCrc ctype cdata
    in word32ToBytesBE len <> ctype <> cdata <> word32ToBytesBE crc
