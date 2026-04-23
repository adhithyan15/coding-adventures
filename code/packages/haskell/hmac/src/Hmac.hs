module Hmac
    ( description
    , hmac
    , hmacSha1
    , hmacSha1Hex
    , hmacSha256
    , hmacSha256Hex
    , hmacSha512
    , hmacSha512Hex
    , verifyTag
    ) where

import Data.Bits
    ( (.|.)
    , xor
    )
import Data.List (foldl')
import Data.Word (Word8)
import Numeric (showHex)
import Sha1 (sha1)
import Sha256 (sha256)
import Sha512 (sha512)

description :: String
description = "HMAC authentication code helpers for SHA-1, SHA-256, and SHA-512"

ipad :: Word8
ipad = 0x36

opad :: Word8
opad = 0x5C

hmac :: Int -> ([Word8] -> [Word8]) -> [Word8] -> [Word8] -> [Word8]
hmac blockSize hashFunction key message =
    hashFunction (outerKey ++ innerDigest)
  where
    normalizedKey = normalizeKey blockSize hashFunction key
    innerKey = map (`xor` ipad) normalizedKey
    outerKey = map (`xor` opad) normalizedKey
    innerDigest = hashFunction (innerKey ++ message)

hmacSha1 :: [Word8] -> [Word8] -> [Word8]
hmacSha1 = hmac 64 sha1

hmacSha1Hex :: [Word8] -> [Word8] -> String
hmacSha1Hex key message = bytesToHex (hmacSha1 key message)

hmacSha256 :: [Word8] -> [Word8] -> [Word8]
hmacSha256 = hmac 64 sha256

hmacSha256Hex :: [Word8] -> [Word8] -> String
hmacSha256Hex key message = bytesToHex (hmacSha256 key message)

hmacSha512 :: [Word8] -> [Word8] -> [Word8]
hmacSha512 = hmac 128 sha512

hmacSha512Hex :: [Word8] -> [Word8] -> String
hmacSha512Hex key message = bytesToHex (hmacSha512 key message)

verifyTag :: [Word8] -> [Word8] -> Bool
verifyTag expected actual =
    diff == 0
  where
    maxLength = max (length expected) (length actual)
    paddedExpected = take maxLength (expected ++ repeat 0)
    paddedActual = take maxLength (actual ++ repeat 0)
    lengthDiff = length expected `xor` length actual
    diff =
        foldl'
            (.|.)
            lengthDiff
            (map fromIntegral (zipWith xor paddedExpected paddedActual))

normalizeKey :: Int -> ([Word8] -> [Word8]) -> [Word8] -> [Word8]
normalizeKey blockSize hashFunction key =
    take blockSize (baseKey ++ repeat 0)
  where
    baseKey =
        if length key > blockSize
            then hashFunction key
            else key

bytesToHex :: [Word8] -> String
bytesToHex =
    concatMap renderByte
  where
    renderByte byteValue =
        let rendered = showHex byteValue ""
         in if length rendered == 1 then '0' : rendered else rendered
