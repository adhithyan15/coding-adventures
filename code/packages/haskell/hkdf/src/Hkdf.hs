module Hkdf
    ( description
    , HashAlgorithm(..)
    , hkdfExtract
    , hkdfExpand
    , hkdf
    ) where

import Data.Word (Word8)
import Hmac (hmac)
import Sha256 (sha256)
import Sha512 (sha512)

description :: String
description = "HKDF key derivation for SHA-256 and SHA-512"

data HashAlgorithm
    = HkdfSha256
    | HkdfSha512
    deriving (Eq, Show)

hkdfExtract :: HashAlgorithm -> [Word8] -> [Word8] -> [Word8]
hkdfExtract algorithm salt ikm =
    hmacFor algorithm effectiveSalt ikm
  where
    effectiveSalt =
        if null salt
            then replicate (hashLength algorithm) 0
            else salt

hkdfExpand :: HashAlgorithm -> [Word8] -> [Word8] -> Int -> Either String [Word8]
hkdfExpand algorithm prk info outputLength
    | outputLength <= 0 = Left "HKDF output length must be positive"
    | outputLength > maxLength =
        Left "HKDF output length exceeds 255 * HashLen"
    | otherwise = Right (take outputLength (go [] 1 []))
  where
    hashLen = hashLength algorithm
    maxLength = 255 * hashLen
    blocksNeeded = ceilingDiv outputLength hashLen
    go previous counter blocks
        | counter > blocksNeeded = concat (reverse blocks)
        | otherwise =
            let block = hmacFor algorithm prk (previous ++ info ++ [fromIntegral counter])
             in go block (counter + 1) (block : blocks)

hkdf :: HashAlgorithm -> [Word8] -> [Word8] -> [Word8] -> Int -> Either String [Word8]
hkdf algorithm salt ikm info outputLength =
    hkdfExpand algorithm (hkdfExtract algorithm salt ikm) info outputLength

hashLength :: HashAlgorithm -> Int
hashLength HkdfSha256 = 32
hashLength HkdfSha512 = 64

hmacFor :: HashAlgorithm -> [Word8] -> [Word8] -> [Word8]
hmacFor HkdfSha256 = hmac 64 sha256
hmacFor HkdfSha512 = hmac 128 sha512

ceilingDiv :: Int -> Int -> Int
ceilingDiv numerator denominator =
    (numerator + denominator - 1) `div` denominator
