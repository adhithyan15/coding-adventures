module Pbkdf2
    ( description
    , PrfAlgorithm(..)
    , pbkdf2
    , pbkdf2Hex
    ) where

import Data.Bits (xor)
import Data.List (foldl')
import Data.Word (Word8, Word32)
import Hmac (hmacSha1, hmacSha256, hmacSha512)
import Numeric (showHex)

description :: String
description = "PBKDF2 key derivation using HMAC-SHA1, HMAC-SHA256, and HMAC-SHA512"

data PrfAlgorithm
    = Pbkdf2Sha1
    | Pbkdf2Sha256
    | Pbkdf2Sha512
    deriving (Eq, Show)

pbkdf2 :: PrfAlgorithm -> [Word8] -> [Word8] -> Int -> Int -> Either String [Word8]
pbkdf2 algorithm password salt iterations keyLength
    | iterations <= 0 = Left "PBKDF2 iterations must be positive"
    | keyLength <= 0 = Left "PBKDF2 key length must be positive"
    | otherwise = Right (take keyLength (concatMap deriveBlock [1 .. blocksNeeded]))
  where
    hashLen = hashLength algorithm
    blocksNeeded = ceilingDiv keyLength hashLen
    deriveBlock blockIndex =
        foldl1 xorBytes (take iterations (iterate nextBlock firstBlock))
      where
        firstBlock = prf algorithm password (salt ++ word32ToBytes (fromIntegral blockIndex))
        nextBlock previous = prf algorithm password previous

pbkdf2Hex :: PrfAlgorithm -> [Word8] -> [Word8] -> Int -> Int -> Either String String
pbkdf2Hex algorithm password salt iterations keyLength =
    fmap bytesToHex (pbkdf2 algorithm password salt iterations keyLength)

hashLength :: PrfAlgorithm -> Int
hashLength Pbkdf2Sha1 = 20
hashLength Pbkdf2Sha256 = 32
hashLength Pbkdf2Sha512 = 64

prf :: PrfAlgorithm -> [Word8] -> [Word8] -> [Word8]
prf Pbkdf2Sha1 = hmacSha1
prf Pbkdf2Sha256 = hmacSha256
prf Pbkdf2Sha512 = hmacSha512

xorBytes :: [Word8] -> [Word8] -> [Word8]
xorBytes = zipWith xor

word32ToBytes :: Word32 -> [Word8]
word32ToBytes wordValue =
    [ fromIntegral (wordValue `shiftRightWord32` 24)
    , fromIntegral (wordValue `shiftRightWord32` 16)
    , fromIntegral (wordValue `shiftRightWord32` 8)
    , fromIntegral wordValue
    ]

shiftRightWord32 :: Word32 -> Int -> Word32
shiftRightWord32 value amount =
    value `div` (2 ^ amount)

ceilingDiv :: Int -> Int -> Int
ceilingDiv numerator denominator =
    (numerator + denominator - 1) `div` denominator

bytesToHex :: [Word8] -> String
bytesToHex =
    concatMap renderByte
  where
    renderByte byteValue =
        let rendered = showHex byteValue ""
         in if length rendered == 1 then '0' : rendered else rendered
