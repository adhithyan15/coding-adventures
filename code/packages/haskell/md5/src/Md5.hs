module Md5
    ( description
    , sumMd5
    , hexString
    ) where

import Data.Bits
    ( (.&.)
    , (.|.)
    , complement
    , rotateL
    , shiftL
    , shiftR
    , xor
    )
import Data.List (foldl')
import Data.Word (Word8, Word32, Word64)
import Numeric (showHex)

description :: String
description = "MD5 message digest algorithm (RFC 1321) implemented from scratch"

initialState :: [Word32]
initialState =
    [ 0x67452301
    , 0xefcdab89
    , 0x98badcfe
    , 0x10325476
    ]

tTable :: [Word32]
tTable =
    [ 0xd76aa478, 0xe8c7b756, 0x242070db, 0xc1bdceee
    , 0xf57c0faf, 0x4787c62a, 0xa8304613, 0xfd469501
    , 0x698098d8, 0x8b44f7af, 0xffff5bb1, 0x895cd7be
    , 0x6b901122, 0xfd987193, 0xa679438e, 0x49b40821
    , 0xf61e2562, 0xc040b340, 0x265e5a51, 0xe9b6c7aa
    , 0xd62f105d, 0x02441453, 0xd8a1e681, 0xe7d3fbc8
    , 0x21e1cde6, 0xc33707d6, 0xf4d50d87, 0x455a14ed
    , 0xa9e3e905, 0xfcefa3f8, 0x676f02d9, 0x8d2a4c8a
    , 0xfffa3942, 0x8771f681, 0x6d9d6122, 0xfde5380c
    , 0xa4beea44, 0x4bdecfa9, 0xf6bb4b60, 0xbebfbc70
    , 0x289b7ec6, 0xeaa127fa, 0xd4ef3085, 0x04881d05
    , 0xd9d4d039, 0xe6db99e5, 0x1fa27cf8, 0xc4ac5665
    , 0xf4292244, 0x432aff97, 0xab9423a7, 0xfc93a039
    , 0x655b59c3, 0x8f0ccc92, 0xffeff47d, 0x85845dd1
    , 0x6fa87e4f, 0xfe2ce6e0, 0xa3014314, 0x4e0811a1
    , 0xf7537e82, 0xbd3af235, 0x2ad7d2bb, 0xeb86d391
    ]

shiftTable :: [Int]
shiftTable =
    [ 7, 12, 17, 22, 7, 12, 17, 22, 7, 12, 17, 22, 7, 12, 17, 22
    , 5, 9, 14, 20, 5, 9, 14, 20, 5, 9, 14, 20, 5, 9, 14, 20
    , 4, 11, 16, 23, 4, 11, 16, 23, 4, 11, 16, 23, 4, 11, 16, 23
    , 6, 10, 15, 21, 6, 10, 15, 21, 6, 10, 15, 21, 6, 10, 15, 21
    ]

sumMd5 :: [Word8] -> [Word8]
sumMd5 inputBytes =
    concatMap word32ToBytesLE (foldl' compress initialState (chunksOf 64 (pad inputBytes)))

hexString :: [Word8] -> String
hexString =
    concatMap renderByte . sumMd5
  where
    renderByte byteValue =
        let rendered = showHex byteValue ""
         in if length rendered == 1 then '0' : rendered else rendered

pad :: [Word8] -> [Word8]
pad inputBytes =
    inputBytes ++ [0x80] ++ replicate paddingLength 0 ++ word64ToBytesLE bitLength
  where
    bitLength :: Word64
    bitLength = fromIntegral (length inputBytes) * 8
    paddingLength =
        let remainder = (length inputBytes + 1 + 8) `mod` 64
         in if remainder == 0 then 0 else 64 - remainder

compress :: [Word32] -> [Word8] -> [Word32]
compress state block =
    zipWith (+) state [a', b', c', d']
  where
    messageWords = map bytesToWord32LE (chunksOf 4 block)
    [a0, b0, c0, d0] = state
    (a', b', c', d') =
        foldl'
            roundStep
            (a0, b0, c0, d0)
            [0 .. 63]

    roundStep :: (Word32, Word32, Word32, Word32) -> Int -> (Word32, Word32, Word32, Word32)
    roundStep (a, b, c, d) roundIndex =
        ( d
        , b + rotateL (a + functionValue + selectedWord + tValue) shiftCount
        , b
        , c
        )
      where
        functionValue = roundFunction roundIndex b c d
        tValue = tTable !! roundIndex
        shiftCount = shiftTable !! roundIndex
        selectedWord = scheduleWord roundIndex
        scheduleWord index =
            let wordIndex
                    | index < 16 = index
                    | index < 32 = (5 * index + 1) `mod` 16
                    | index < 48 = (3 * index + 5) `mod` 16
                    | otherwise = (7 * index) `mod` 16
             in messageWords !! wordIndex

roundFunction :: Int -> Word32 -> Word32 -> Word32 -> Word32
roundFunction roundIndex b c d
    | roundIndex < 16 = (b .&. c) `xor` (complement b .&. d)
    | roundIndex < 32 = (d .&. b) `xor` (complement d .&. c)
    | roundIndex < 48 = b `xor` c `xor` d
    | otherwise = c `xor` (b .|. complement d)

bytesToWord32LE :: [Word8] -> Word32
bytesToWord32LE [a, b, c, d] =
    fromIntegral a
        + shiftL (fromIntegral b) 8
        + shiftL (fromIntegral c) 16
        + shiftL (fromIntegral d) 24
bytesToWord32LE _ = 0

word32ToBytesLE :: Word32 -> [Word8]
word32ToBytesLE wordValue =
    [ fromIntegral wordValue
    , fromIntegral (wordValue `shiftR` 8)
    , fromIntegral (wordValue `shiftR` 16)
    , fromIntegral (wordValue `shiftR` 24)
    ]

word64ToBytesLE :: Word64 -> [Word8]
word64ToBytesLE wordValue =
    [ fromIntegral wordValue
    , fromIntegral (wordValue `shiftR` 8)
    , fromIntegral (wordValue `shiftR` 16)
    , fromIntegral (wordValue `shiftR` 24)
    , fromIntegral (wordValue `shiftR` 32)
    , fromIntegral (wordValue `shiftR` 40)
    , fromIntegral (wordValue `shiftR` 48)
    , fromIntegral (wordValue `shiftR` 56)
    ]

chunksOf :: Int -> [a] -> [[a]]
chunksOf _ [] = []
chunksOf size values =
    let (chunk, rest) = splitAt size values
     in chunk : chunksOf size rest
