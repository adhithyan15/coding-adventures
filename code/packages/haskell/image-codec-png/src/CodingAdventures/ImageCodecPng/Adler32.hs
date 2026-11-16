{-# LANGUAGE BangPatterns #-}
module CodingAdventures.ImageCodecPng.Adler32
    ( adler32
    ) where

import Data.Bits ((.|.), shiftL)
import Data.ByteString (ByteString)
import qualified Data.ByteString as BS
import Data.Word (Word8, Word32, Word64)

import CodingAdventures.ImageCodecPng.Constants (adlerChunkSize, adlerModulus, byteShift16)

-- | Compute RFC 1950 Adler-32 checksum over the provided byte sequence.
adler32 :: ByteString -> Word32
adler32 bs = go 1 0 bs
  where
    modulus :: Word64
    modulus = fromIntegral adlerModulus

    go :: Word64 -> Word64 -> ByteString -> Word32
    go !a !b !remBytes
        | BS.null remBytes =
            let !resA = a `mod` modulus
                !resB = b `mod` modulus
            in fromIntegral ((resB `shiftL` byteShift16) .|. resA)
        | otherwise =
            let (chunk, rest) = BS.splitAt adlerChunkSize remBytes
                (newA, newB) = BS.foldl' step (a, b) chunk
                !modA = newA `mod` modulus
                !modB = newB `mod` modulus
            in go modA modB rest

    step :: (Word64, Word64) -> Word8 -> (Word64, Word64)
    step (!a, !b) !byte =
        let !a' = a + fromIntegral byte
            !b' = b + a'
        in (a', b')
