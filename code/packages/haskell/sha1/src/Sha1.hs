module Sha1
    ( description
    , sha1
    , sha1Hex
    ) where

import Data.Bits
    ( (.&.)
    , complement
    , rotateL
    , shiftR
    , shiftL
    , xor
    )
import Data.List (foldl')
import Data.Word (Word8, Word32, Word64)
import Numeric (showHex)

description :: String
description = "SHA-1 cryptographic hash function (FIPS 180-4) implemented from scratch"

initialState :: [Word32]
initialState =
    [ 0x67452301
    , 0xEFCDAB89
    , 0x98BADCFE
    , 0x10325476
    , 0xC3D2E1F0
    ]

roundConstants :: [Word32]
roundConstants =
    concat
        [ replicate 20 0x5A827999
        , replicate 20 0x6ED9EBA1
        , replicate 20 0x8F1BBCDC
        , replicate 20 0xCA62C1D6
        ]

sha1 :: [Word8] -> [Word8]
sha1 inputBytes =
    concatMap word32ToBytes (foldl' compress initialState (chunksOf 64 (pad inputBytes)))

sha1Hex :: [Word8] -> String
sha1Hex =
    concatMap renderByte . sha1
  where
    renderByte byteValue =
        let rendered = showHex byteValue ""
         in if length rendered == 1 then '0' : rendered else rendered

pad :: [Word8] -> [Word8]
pad inputBytes =
    inputBytes ++ [0x80] ++ replicate paddingLength 0 ++ lengthBytes
  where
    bitLength :: Word64
    bitLength = fromIntegral (length inputBytes) * 8
    lengthBytes = word64ToBytes bitLength
    paddingLength =
        let remainder = (length inputBytes + 1 + 8) `mod` 64
         in if remainder == 0 then 0 else 64 - remainder

compress :: [Word32] -> [Word8] -> [Word32]
compress state block =
    zipWith (+) state [a', b', c', d', e']
  where
    scheduleWords = messageSchedule block
    [a0, b0, c0, d0, e0] = state
    (a', b', c', d', e') =
        foldl'
            roundStep
            (a0, b0, c0, d0, e0)
            (zip3 [0 .. 79] roundConstants scheduleWords)

roundStep :: (Word32, Word32, Word32, Word32, Word32) -> (Int, Word32, Word32) -> (Word32, Word32, Word32, Word32, Word32)
roundStep (a, b, c, d, e) (roundIndex, constantValue, wordValue) =
    ( temp
    , a
    , rotateLeft32 30 b
    , c
    , d
    )
  where
    temp =
        rotateLeft32 5 a
            + roundFunction roundIndex b c d
            + e
            + constantValue
            + wordValue

roundFunction :: Int -> Word32 -> Word32 -> Word32 -> Word32
roundFunction roundIndex b c d
    | roundIndex < 20 = (b .&. c) `xor` (complement b .&. d)
    | roundIndex < 40 = b `xor` c `xor` d
    | roundIndex < 60 = (b .&. c) `xor` (b .&. d) `xor` (c .&. d)
    | otherwise = b `xor` c `xor` d

messageSchedule :: [Word8] -> [Word32]
messageSchedule block =
    initialWords ++ expand initialWords
  where
    initialWords = map bytesToWord32 (chunksOf 4 block)
    expand wordsSoFar
        | length wordsSoFar == 80 = []
        | otherwise =
            let nextValue =
                    rotateLeft32 1 $
                        (wordsSoFar !! 13)
                            `xor` (wordsSoFar !! 8)
                            `xor` (wordsSoFar !! 2)
                            `xor` (wordsSoFar !! 0)
                rotated = drop 1 wordsSoFar ++ [nextValue]
             in nextValue : expand rotated

rotateLeft32 :: Int -> Word32 -> Word32
rotateLeft32 count value =
    rotateL value count

word32ToBytes :: Word32 -> [Word8]
word32ToBytes wordValue =
    [ fromIntegral (wordValue `shiftR` 24)
    , fromIntegral (wordValue `shiftR` 16)
    , fromIntegral (wordValue `shiftR` 8)
    , fromIntegral wordValue
    ]

word64ToBytes :: Word64 -> [Word8]
word64ToBytes wordValue =
    [ fromIntegral (wordValue `shiftR` 56)
    , fromIntegral (wordValue `shiftR` 48)
    , fromIntegral (wordValue `shiftR` 40)
    , fromIntegral (wordValue `shiftR` 32)
    , fromIntegral (wordValue `shiftR` 24)
    , fromIntegral (wordValue `shiftR` 16)
    , fromIntegral (wordValue `shiftR` 8)
    , fromIntegral wordValue
    ]

bytesToWord32 :: [Word8] -> Word32
bytesToWord32 [a, b, c, d] =
    shiftL (fromIntegral a) 24
        + shiftL (fromIntegral b) 16
        + shiftL (fromIntegral c) 8
        + fromIntegral d
bytesToWord32 _ = 0

chunksOf :: Int -> [a] -> [[a]]
chunksOf _ [] = []
chunksOf size values =
    let (chunk, rest) = splitAt size values
     in chunk : chunksOf size rest
