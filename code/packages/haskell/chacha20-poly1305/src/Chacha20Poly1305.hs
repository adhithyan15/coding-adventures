module Chacha20Poly1305
    ( description
    , chacha20Encrypt
    , poly1305Mac
    , aeadEncrypt
    , aeadDecrypt
    ) where

import Data.Bits
    ( (.&.)
    , (.|.)
    , rotateL
    , shiftL
    , shiftR
    , xor
    )
import Data.List (foldl')
import Data.Word (Word8, Word32, Word64)

description :: String
description = "ChaCha20-Poly1305 authenticated encryption (RFC 8439)"

constants :: [Word32]
constants = [0x61707865, 0x3320646e, 0x79622d32, 0x6b206574]

chacha20Encrypt :: [Word8] -> [Word8] -> [Word8] -> Word32 -> Either String [Word8]
chacha20Encrypt plaintext keyBytes nonce counter
    | length keyBytes /= 32 =
        Left ("ChaCha20 key must be 32 bytes, got " ++ show (length keyBytes))
    | length nonce /= 12 =
        Left ("ChaCha20 nonce must be 12 bytes, got " ++ show (length nonce))
    | otherwise =
        Right (go plaintext counter)
  where
    go [] _ = []
    go remaining currentCounter =
        let keystream = chacha20Block keyBytes nonce currentCounter
            chunk = take 64 remaining
         in zipWith xor chunk keystream ++ go (drop 64 remaining) (currentCounter + 1)

poly1305Mac :: [Word8] -> [Word8] -> Either String [Word8]
poly1305Mac message keyBytes
    | length keyBytes /= 32 =
        Left ("Poly1305 key must be 32 bytes, got " ++ show (length keyBytes))
    | otherwise =
        Right (integerToBytes16LE ((accumulator + sValue) `mod` twoPower128))
  where
    rValue = clampR (bytesToIntegerLE (take 16 keyBytes))
    sValue = bytesToIntegerLE (drop 16 keyBytes)
    prime = (2 ^ (130 :: Int)) - 5
    accumulator =
        foldl'
            (\current chunk ->
                let chunkValue = bytesToIntegerLE chunk + 2 ^ (8 * length chunk)
                 in ((current + chunkValue) * rValue) `mod` prime
            )
            0
            (chunksOf 16 message)
    twoPower128 = 2 ^ (128 :: Int)

aeadEncrypt :: [Word8] -> [Word8] -> [Word8] -> [Word8] -> Either String ([Word8], [Word8])
aeadEncrypt plaintext keyBytes nonce aad = do
    polyKeyStream <- chacha20Encrypt (replicate 64 0) keyBytes nonce 0
    let polyKey = take 32 polyKeyStream
    ciphertext <- chacha20Encrypt plaintext keyBytes nonce 1
    tag <- poly1305Mac (buildMacData aad ciphertext) polyKey
    Right (ciphertext, tag)

aeadDecrypt :: [Word8] -> [Word8] -> [Word8] -> [Word8] -> [Word8] -> Either String [Word8]
aeadDecrypt ciphertext keyBytes nonce aad tag = do
    polyKeyStream <- chacha20Encrypt (replicate 64 0) keyBytes nonce 0
    let polyKey = take 32 polyKeyStream
    computedTag <- poly1305Mac (buildMacData aad ciphertext) polyKey
    if constantTimeEq computedTag tag
        then chacha20Encrypt ciphertext keyBytes nonce 1
        else Left "ChaCha20-Poly1305 authentication failed"

chacha20Block :: [Word8] -> [Word8] -> Word32 -> [Word8]
chacha20Block keyBytes nonce counter =
    concatMap word32ToBytesLE finalState
  where
    initialState =
        constants
            ++ map bytesToWord32LE (chunksOf 4 keyBytes)
            ++ [counter]
            ++ map bytesToWord32LE (chunksOf 4 nonce)
    workingState = applyRounds 10 initialState
    finalState = zipWith (+) workingState initialState

applyRounds :: Int -> [Word32] -> [Word32]
applyRounds 0 state = state
applyRounds roundsRemaining state =
    applyRounds (roundsRemaining - 1) (diagonalRounds (columnRounds state))

columnRounds :: [Word32] -> [Word32]
columnRounds state =
    quarterRound 3 7 11 15
        (quarterRound 2 6 10 14
            (quarterRound 1 5 9 13
                (quarterRound 0 4 8 12 state)))

diagonalRounds :: [Word32] -> [Word32]
diagonalRounds state =
    quarterRound 3 4 9 14
        (quarterRound 2 7 8 13
            (quarterRound 1 6 11 12
                (quarterRound 0 5 10 15 state)))

quarterRound :: Int -> Int -> Int -> Int -> [Word32] -> [Word32]
quarterRound aIndex bIndex cIndex dIndex state8 =
    replaceMany
        [ (aIndex, a4)
        , (bIndex, b4)
        , (cIndex, c4)
        , (dIndex, d4)
        ]
        state8
  where
    a0 = state8 !! aIndex
    b0 = state8 !! bIndex
    c0 = state8 !! cIndex
    d0 = state8 !! dIndex
    a1 = a0 + b0
    d1 = rotateL (d0 `xor` a1) 16
    c1 = c0 + d1
    b1 = rotateL (b0 `xor` c1) 12
    a2 = a1 + b1
    d2 = rotateL (d1 `xor` a2) 8
    c2 = c1 + d2
    b2 = rotateL (b1 `xor` c2) 7
    a4 = a2
    b4 = b2
    c4 = c2
    d4 = d2

replaceMany :: [(Int, a)] -> [a] -> [a]
replaceMany replacements values =
    [ maybe originalValue id (lookup index replacements)
    | (index, originalValue) <- zip [0 ..] values
    ]

buildMacData :: [Word8] -> [Word8] -> [Word8]
buildMacData aad ciphertext =
    aad
        ++ pad16 aad
        ++ ciphertext
        ++ pad16 ciphertext
        ++ word64ToBytesLE (fromIntegral (length aad))
        ++ word64ToBytesLE (fromIntegral (length ciphertext))

pad16 :: [Word8] -> [Word8]
pad16 values
    | remainder == 0 = []
    | otherwise = replicate (16 - remainder) 0
  where
    remainder = length values `mod` 16

clampR :: Integer -> Integer
clampR value =
    value .&. bytesToIntegerLE [0xff, 0xff, 0xff, 0x0f, 0xfc, 0xff, 0xff, 0x0f, 0xfc, 0xff, 0xff, 0x0f, 0xfc, 0xff, 0xff, 0x0f]

constantTimeEq :: [Word8] -> [Word8] -> Bool
constantTimeEq left right =
    length left == length right
        && foldl' (.|.) 0 (zipWith xor left right) == 0

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
    , fromIntegral (shiftR wordValue 8)
    , fromIntegral (shiftR wordValue 16)
    , fromIntegral (shiftR wordValue 24)
    ]

word64ToBytesLE :: Word64 -> [Word8]
word64ToBytesLE wordValue =
    [ fromIntegral wordValue
    , fromIntegral (shiftR wordValue 8)
    , fromIntegral (shiftR wordValue 16)
    , fromIntegral (shiftR wordValue 24)
    , fromIntegral (shiftR wordValue 32)
    , fromIntegral (shiftR wordValue 40)
    , fromIntegral (shiftR wordValue 48)
    , fromIntegral (shiftR wordValue 56)
    ]

bytesToIntegerLE :: [Word8] -> Integer
bytesToIntegerLE bytes =
    sum
        [ shiftL (fromIntegral byteValue) (8 * index)
        | (index, byteValue) <- zip [0 ..] bytes
        ]

integerToBytes16LE :: Integer -> [Word8]
integerToBytes16LE value =
    [ fromIntegral (shiftR value (8 * index))
    | index <- [0 .. 15]
    ]

chunksOf :: Int -> [a] -> [[a]]
chunksOf _ [] = []
chunksOf size values =
    let (chunk, rest) = splitAt size values
     in chunk : chunksOf size rest
