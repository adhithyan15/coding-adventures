module X25519
    ( description
    , x25519
    , x25519Base
    , generateKeypair
    ) where

import Data.Bits ((.&.), (.|.), shiftL, shiftR)
import Data.Word (Word8)

description :: String
description = "X25519 key agreement (RFC 7748) implemented from scratch in pure Haskell"

x25519 :: [Word8] -> [Word8] -> Either String [Word8]
x25519 privateKey publicKey
    | length privateKey /= 32 =
        Left ("X25519 private key must be 32 bytes, got " ++ show (length privateKey))
    | length publicKey /= 32 =
        Left ("X25519 public key must be 32 bytes, got " ++ show (length publicKey))
    | otherwise =
        let result = montgomeryLadder privateKey publicKey
         in if all (== 0) result
                then Left "X25519 produced the all-zeros output (low-order point)"
                else Right result

x25519Base :: [Word8] -> Either String [Word8]
x25519Base privateKey = x25519 privateKey basePoint

generateKeypair :: [Word8] -> Either String [Word8]
generateKeypair = x25519Base

basePoint :: [Word8]
basePoint = 9 : replicate 31 0

prime :: Integer
prime = (2 ^ (255 :: Integer)) - 19

modP :: Integer -> Integer
modP value =
    let reduced = value `mod` prime
     in if reduced < 0 then reduced + prime else reduced

addP :: Integer -> Integer -> Integer
addP left right = modP (left + right)

subP :: Integer -> Integer -> Integer
subP left right = modP (left - right)

mulP :: Integer -> Integer -> Integer
mulP left right = modP (left * right)

squareP :: Integer -> Integer
squareP value = mulP value value

invP :: Integer -> Integer
invP value = powMod (modP value) (prime - 2) prime

powMod :: Integer -> Integer -> Integer -> Integer
powMod _ 0 modulus = 1 `mod` modulus
powMod baseValue exponent modulus = go baseValue exponent 1
  where
    go _ 0 acc = acc
    go current power acc
        | odd power =
            go next (power `div` 2) ((acc * current) `mod` modulus)
        | otherwise =
            go next (power `div` 2) acc
      where
        next = (current * current) `mod` modulus

decodeLittleEndian :: [Word8] -> Integer
decodeLittleEndian =
    foldr step 0 . zip [0 ..]
  where
    step (indexValue, byteValue) acc =
        acc + (fromIntegral byteValue `shiftL` (8 * indexValue))

encodeLittleEndian32 :: Integer -> [Word8]
encodeLittleEndian32 value =
    [ fromIntegral ((normalized `shiftR` (8 * indexValue)) .&. 0xff)
    | indexValue <- [0 .. 31]
    ]
  where
    normalized = modP value

clampScalar :: [Word8] -> [Word8]
clampScalar bytesValue =
    case bytesValue of
        [] -> []
        firstByte : _ ->
            let middleBytes = take 30 (drop 1 bytesValue)
                lastByte = bytesValue !! 31
             in (firstByte .&. 248)
                    : middleBytes
                    ++ [ (lastByte .&. 127) .|. 64 ]

montgomeryLadder :: [Word8] -> [Word8] -> [Word8]
montgomeryLadder scalarBytes uBytes =
    encodeLittleEndian32 result
  where
    clampedScalar = clampScalar scalarBytes
    maskedUBytes = take 31 uBytes ++ [last uBytes .&. 0x7f]
    x1 = decodeLittleEndian maskedUBytes
    (x2End, z2End, x3End, z3End, lastBit) =
        foldl ladderStep (1, 0, x1, 1, 0 :: Int) [254, 253 .. 0]
    (x2Final, z2Final) =
        if lastBit == 1
            then (x3End, z3End)
            else (x2End, z2End)
    result = mulP x2Final (invP z2Final)

    ladderStep (x2, z2, x3, z3, previousBit) bitIndex =
        let bitValue = scalarBit clampedScalar bitIndex
            (x2Swapped, z2Swapped, x3Swapped, z3Swapped) =
                if bitValue /= previousBit
                    then (x3, z3, x2, z2)
                    else (x2, z2, x3, z3)
            a = addP x2Swapped z2Swapped
            aa = squareP a
            b = subP x2Swapped z2Swapped
            bb = squareP b
            e = subP aa bb
            c = addP x3Swapped z3Swapped
            d = subP x3Swapped z3Swapped
            da = mulP d a
            cb = mulP c b
            x3Next = squareP (addP da cb)
            z3Next = mulP x1 (squareP (subP da cb))
            x2Next = mulP aa bb
            z2Next = mulP e (addP bb (mulP 121666 e))
         in (x2Next, z2Next, x3Next, z3Next, bitValue)

scalarBit :: [Word8] -> Int -> Int
scalarBit scalarBytes bitIndex =
    fromIntegral ((scalarBytes !! byteIndex `shiftR` bitOffset) .&. 1)
  where
    byteIndex = bitIndex `div` 8
    bitOffset = bitIndex `mod` 8
