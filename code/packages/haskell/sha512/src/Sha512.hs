module Sha512
    ( description
    , sha512
    , sha512Hex
    ) where

import Data.Bits
    ( (.&.)
    , rotateR
    , shiftR
    , shiftL
    , xor
    )
import Data.List (foldl')
import Data.Word (Word8, Word64)
import Numeric (showHex)

description :: String
description = "SHA-512 cryptographic hash function (FIPS 180-4) implemented from scratch"

initialState :: [Word64]
initialState =
    [ 0x6A09E667F3BCC908
    , 0xBB67AE8584CAA73B
    , 0x3C6EF372FE94F82B
    , 0xA54FF53A5F1D36F1
    , 0x510E527FADE682D1
    , 0x9B05688C2B3E6C1F
    , 0x1F83D9ABFB41BD6B
    , 0x5BE0CD19137E2179
    ]

roundConstants :: [Word64]
roundConstants =
    [ 0x428A2F98D728AE22, 0x7137449123EF65CD, 0xB5C0FBCFEC4D3B2F, 0xE9B5DBA58189DBBC
    , 0x3956C25BF348B538, 0x59F111F1B605D019, 0x923F82A4AF194F9B, 0xAB1C5ED5DA6D8118
    , 0xD807AA98A3030242, 0x12835B0145706FBE, 0x243185BE4EE4B28C, 0x550C7DC3D5FFB4E2
    , 0x72BE5D74F27B896F, 0x80DEB1FE3B1696B1, 0x9BDC06A725C71235, 0xC19BF174CF692694
    , 0xE49B69C19EF14AD2, 0xEFBE4786384F25E3, 0x0FC19DC68B8CD5B5, 0x240CA1CC77AC9C65
    , 0x2DE92C6F592B0275, 0x4A7484AA6EA6E483, 0x5CB0A9DCBD41FBD4, 0x76F988DA831153B5
    , 0x983E5152EE66DFAB, 0xA831C66D2DB43210, 0xB00327C898FB213F, 0xBF597FC7BEEF0EE4
    , 0xC6E00BF33DA88FC2, 0xD5A79147930AA725, 0x06CA6351E003826F, 0x142929670A0E6E70
    , 0x27B70A8546D22FFC, 0x2E1B21385C26C926, 0x4D2C6DFC5AC42AED, 0x53380D139D95B3DF
    , 0x650A73548BAF63DE, 0x766A0ABB3C77B2A8, 0x81C2C92E47EDAEE6, 0x92722C851482353B
    , 0xA2BFE8A14CF10364, 0xA81A664BBC423001, 0xC24B8B70D0F89791, 0xC76C51A30654BE30
    , 0xD192E819D6EF5218, 0xD69906245565A910, 0xF40E35855771202A, 0x106AA07032BBD1B8
    , 0x19A4C116B8D2D0C8, 0x1E376C085141AB53, 0x2748774CDF8EEB99, 0x34B0BCB5E19B48A8
    , 0x391C0CB3C5C95A63, 0x4ED8AA4AE3418ACB, 0x5B9CCA4F7763E373, 0x682E6FF3D6B2B8A3
    , 0x748F82EE5DEFB2FC, 0x78A5636F43172F60, 0x84C87814A1F0AB72, 0x8CC702081A6439EC
    , 0x90BEFFFA23631E28, 0xA4506CEBDE82BDE9, 0xBEF9A3F7B2C67915, 0xC67178F2E372532B
    , 0xCA273ECEEA26619C, 0xD186B8C721C0C207, 0xEADA7DD6CDE0EB1E, 0xF57D4F7FEE6ED178
    , 0x06F067AA72176FBA, 0x0A637DC5A2C898A6, 0x113F9804BEF90DAE, 0x1B710B35131C471B
    , 0x28DB77F523047D84, 0x32CAAB7B40C72493, 0x3C9EBE0A15C9BEBC, 0x431D67C49C100D4C
    , 0x4CC5D4BECB3E42B6, 0x597F299CFC657E2A, 0x5FCB6FAB3AD6FAEC, 0x6C44198C4A475817
    ]

sha512 :: [Word8] -> [Word8]
sha512 inputBytes =
    concatMap word64ToBytes (foldl' compress initialState (chunksOf 128 (pad inputBytes)))

sha512Hex :: [Word8] -> String
sha512Hex =
    concatMap renderByte . sha512
  where
    renderByte byteValue =
        let rendered = showHex byteValue ""
         in if length rendered == 1 then '0' : rendered else rendered

pad :: [Word8] -> [Word8]
pad inputBytes =
    inputBytes ++ [0x80] ++ replicate paddingLength 0 ++ integerToBytes16 bitLength
  where
    bitLength :: Integer
    bitLength = fromIntegral (length inputBytes) * 8
    paddingLength =
        let remainder = (length inputBytes + 1 + 16) `mod` 128
         in if remainder == 0 then 0 else 128 - remainder

compress :: [Word64] -> [Word8] -> [Word64]
compress state block =
    zipWith (+) state [a', b', c', d', e', f', g', h']
  where
    scheduleWords = messageSchedule block
    [a0, b0, c0, d0, e0, f0, g0, h0] = state
    (a', b', c', d', e', f', g', h') =
        foldl'
            roundStep
            (a0, b0, c0, d0, e0, f0, g0, h0)
            (zip roundConstants scheduleWords)

roundStep :: (Word64, Word64, Word64, Word64, Word64, Word64, Word64, Word64) -> (Word64, Word64) -> (Word64, Word64, Word64, Word64, Word64, Word64, Word64, Word64)
roundStep (a, b, c, d, e, f, g, h) (constantValue, wordValue) =
    ( t1 + t2
    , a
    , b
    , c
    , d + t1
    , e
    , f
    , g
    )
  where
    t1 = h + bigSigma1 e + choose e f g + constantValue + wordValue
    t2 = bigSigma0 a + majority a b c

messageSchedule :: [Word8] -> [Word64]
messageSchedule block =
    initialWords ++ expand initialWords
  where
    initialWords = map bytesToWord64 (chunksOf 8 block)
    expand wordsSoFar
        | length wordsSoFar == 80 = []
        | otherwise =
            let nextValue =
                    smallSigma1 (wordsSoFar !! 14)
                        + (wordsSoFar !! 9)
                        + smallSigma0 (wordsSoFar !! 1)
                        + (wordsSoFar !! 0)
                rotated = drop 1 wordsSoFar ++ [nextValue]
             in nextValue : expand rotated

choose :: Word64 -> Word64 -> Word64 -> Word64
choose x y z = (x .&. y) `xor` (complement64 x .&. z)

majority :: Word64 -> Word64 -> Word64 -> Word64
majority x y z = (x .&. y) `xor` (x .&. z) `xor` (y .&. z)

bigSigma0 :: Word64 -> Word64
bigSigma0 x = rotateRight64 28 x `xor` rotateRight64 34 x `xor` rotateRight64 39 x

bigSigma1 :: Word64 -> Word64
bigSigma1 x = rotateRight64 14 x `xor` rotateRight64 18 x `xor` rotateRight64 41 x

smallSigma0 :: Word64 -> Word64
smallSigma0 x = rotateRight64 1 x `xor` rotateRight64 8 x `xor` (x `shiftR` 7)

smallSigma1 :: Word64 -> Word64
smallSigma1 x = rotateRight64 19 x `xor` rotateRight64 61 x `xor` (x `shiftR` 6)

rotateRight64 :: Int -> Word64 -> Word64
rotateRight64 count value =
    rotateR value count

complement64 :: Word64 -> Word64
complement64 value = maxBound `xor` value

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

bytesToWord64 :: [Word8] -> Word64
bytesToWord64 [a, b, c, d, e, f, g, h] =
    shiftL (fromIntegral a) 56
        + shiftL (fromIntegral b) 48
        + shiftL (fromIntegral c) 40
        + shiftL (fromIntegral d) 32
        + shiftL (fromIntegral e) 24
        + shiftL (fromIntegral f) 16
        + shiftL (fromIntegral g) 8
        + fromIntegral h
bytesToWord64 _ = 0

integerToBytes16 :: Integer -> [Word8]
integerToBytes16 value =
    map byteAt [15, 14 .. 0]
  where
    byteAt index =
        fromIntegral ((value `shiftRInteger` (index * 8)) `mod` 256)

shiftRInteger :: Integer -> Int -> Integer
shiftRInteger value amount =
    value `div` (2 ^ amount)

chunksOf :: Int -> [a] -> [[a]]
chunksOf _ [] = []
chunksOf size values =
    let (chunk, rest) = splitAt size values
     in chunk : chunksOf size rest
