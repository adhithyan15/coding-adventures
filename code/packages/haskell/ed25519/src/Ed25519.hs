module Ed25519
    ( description
    , generateKeypair
    , sign
    , verify
    ) where

import Data.Bits ((.&.), (.|.), shiftL, shiftR)
import Data.Word (Word8)
import Sha512 (sha512)

description :: String
description = "Ed25519 digital signatures (RFC 8032) implemented from scratch in pure Haskell"

generateKeypair :: [Word8] -> Either String ([Word8], [Word8])
generateKeypair seed
    | length seed /= 32 =
        Left ("Ed25519 seed must be 32 bytes, got " ++ show (length seed))
    | otherwise =
        let digest = sha512 seed
            scalarA = clampScalarInteger (take 32 digest)
            publicKey = encodePoint (scalarMul scalarA basePoint)
         in Right (publicKey, seed ++ publicKey)

sign :: [Word8] -> [Word8] -> Either String [Word8]
sign message secretKey
    | length secretKey /= 64 =
        Left ("Ed25519 secret key must be 64 bytes, got " ++ show (length secretKey))
    | otherwise =
        let seed = take 32 secretKey
            suppliedPublicKey = drop 32 secretKey
         in case generateKeypair seed of
                Left err -> Left err
                Right (_, reconstructedSecretKey)
                    | reconstructedSecretKey /= secretKey ->
                        Left "Ed25519 secret key must be seed || public_key"
                    | otherwise ->
                        let digest = sha512 seed
                            scalarA = clampScalarInteger (take 32 digest)
                            prefix = drop 32 digest
                            nonceR = reduceScalar (sha512 (prefix ++ message))
                            encodedR = encodePoint (scalarMul nonceR basePoint)
                            challengeK = reduceScalar (sha512 (encodedR ++ suppliedPublicKey ++ message))
                            scalarS = (nonceR + challengeK * scalarA) `mod` groupOrder
                         in Right (encodedR ++ encodeScalar scalarS)

verify :: [Word8] -> [Word8] -> [Word8] -> Bool
verify message signature publicKey
    | length signature /= 64 = False
    | length publicKey /= 32 = False
    | scalarS >= groupOrder = False
    | otherwise =
        case (decodePoint encodedR, decodePoint publicKey) of
            (Just pointR, Just pointA) ->
                let challengeK = reduceScalar (sha512 (encodedR ++ publicKey ++ message))
                    lhs = scalarMul scalarS basePoint
                    rhs = pointAdd pointR (scalarMul challengeK pointA)
                 in encodePoint lhs == encodePoint rhs
            _ -> False
  where
    encodedR = take 32 signature
    scalarS = decodeLittleEndian (drop 32 signature)

data Point = Point
    { pointX :: Integer
    , pointY :: Integer
    }

prime :: Integer
prime = (2 ^ (255 :: Integer)) - 19

groupOrder :: Integer
groupOrder = (2 ^ (252 :: Integer)) + 27742317777372353535851937790883648493

dConstant :: Integer
dConstant = decodeLittleEndian dBytes

sqrtM1 :: Integer
sqrtM1 = decodeLittleEndian sqrtM1Bytes

basePoint :: Point
basePoint =
    Point
        { pointX = decodeLittleEndian baseXBytes
        , pointY = decodeLittleEndian baseYBytes
        }

identityPoint :: Point
identityPoint = Point 0 1

baseXBytes :: [Word8]
baseXBytes =
    [ 0x1a, 0xd5, 0x25, 0x8f, 0x60, 0x2d, 0x56, 0xc9
    , 0xb2, 0xa7, 0x25, 0x95, 0x60, 0xc7, 0x2c, 0x69
    , 0x5c, 0xdc, 0xd6, 0xfd, 0x31, 0xe2, 0xa4, 0xc0
    , 0xfe, 0x53, 0x6e, 0xcd, 0xd3, 0x36, 0x69, 0x21
    ]

baseYBytes :: [Word8]
baseYBytes =
    [ 0x58, 0x66, 0x66, 0x66, 0x66, 0x66, 0x66, 0x66
    , 0x66, 0x66, 0x66, 0x66, 0x66, 0x66, 0x66, 0x66
    , 0x66, 0x66, 0x66, 0x66, 0x66, 0x66, 0x66, 0x66
    , 0x66, 0x66, 0x66, 0x66, 0x66, 0x66, 0x66, 0x66
    ]

dBytes :: [Word8]
dBytes =
    [ 0xa3, 0x78, 0x59, 0x13, 0xca, 0x4d, 0xeb, 0x75
    , 0xab, 0xd8, 0x41, 0x41, 0x4d, 0x0a, 0x70, 0x00
    , 0x98, 0xe8, 0x79, 0x77, 0x79, 0x40, 0xc7, 0x8c
    , 0x73, 0xfe, 0x6f, 0x2b, 0xee, 0x6c, 0x03, 0x52
    ]

sqrtM1Bytes :: [Word8]
sqrtM1Bytes =
    [ 0xb0, 0xa0, 0x0e, 0x4a, 0x27, 0x1b, 0xee, 0xc4
    , 0x78, 0xe4, 0x2f, 0xad, 0x06, 0x18, 0x43, 0x2f
    , 0xa7, 0xd7, 0xfb, 0x3d, 0x99, 0x00, 0x4d, 0x2b
    , 0x0b, 0xdf, 0xc1, 0x4f, 0x80, 0x24, 0x83, 0x2b
    ]

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
powMod baseValue exponent modulus = go (baseValue `mod` modulus) exponent 1
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
    normalized = value `mod` (2 ^ (256 :: Integer))

clampScalarBytes :: [Word8] -> [Word8]
clampScalarBytes bytesValue =
    case bytesValue of
        firstByte : rest ->
            let middleBytes = take 30 rest
                lastByte = bytesValue !! 31
             in (firstByte .&. 248)
                    : middleBytes
                    ++ [ (lastByte .&. 127) .|. 64 ]
        [] -> []

clampScalarInteger :: [Word8] -> Integer
clampScalarInteger = decodeLittleEndian . clampScalarBytes

reduceScalar :: [Word8] -> Integer
reduceScalar bytesValue = decodeLittleEndian bytesValue `mod` groupOrder

encodeScalar :: Integer -> [Word8]
encodeScalar = encodeLittleEndian32

pointAdd :: Point -> Point -> Point
pointAdd (Point x1 y1) (Point x2 y2) =
    Point x3 y3
  where
    x1x2 = mulP x1 x2
    y1y2 = mulP y1 y2
    xyxy = mulP x1x2 y1y2
    denominatorX = invP (addP 1 (mulP dConstant xyxy))
    denominatorY = invP (subP 1 (mulP dConstant xyxy))
    x3 = mulP (addP (mulP x1 y2) (mulP y1 x2)) denominatorX
    y3 = mulP (addP y1y2 x1x2) denominatorY

pointDouble :: Point -> Point
pointDouble pointValue = pointAdd pointValue pointValue

scalarMul :: Integer -> Point -> Point
scalarMul scalarValue pointValue = go scalarValue pointValue identityPoint
  where
    go 0 _ accumulator = accumulator
    go n addend accumulator =
        let nextAccumulator =
                if odd n
                    then pointAdd accumulator addend
                    else accumulator
         in go (n `div` 2) (pointDouble addend) nextAccumulator

sqrtModP :: Integer -> Maybe Integer
sqrtModP value
    | normalized == 0 = Just 0
    | checkRoot candidate = Just candidate
    | checkRoot adjusted = Just adjusted
    | otherwise = Nothing
  where
    normalized = modP value
    candidate = powMod normalized ((prime + 3) `div` 8) prime
    adjusted = mulP candidate sqrtM1
    checkRoot rootValue = modP (squareP rootValue - normalized) == 0

encodePoint :: Point -> [Word8]
encodePoint (Point xValue yValue) =
    take 31 yBytes ++ [last yBytes .|. signBit]
  where
    yBytes = encodeLittleEndian32 (modP yValue)
    signBit =
        if odd (modP xValue)
            then 0x80
            else 0

decodePoint :: [Word8] -> Maybe Point
decodePoint encoded
    | length encoded /= 32 = Nothing
    | yValue >= prime = Nothing
    | otherwise =
        case sqrtModP xSquared of
            Nothing -> Nothing
            Just root
                | root == 0 && signBit == 1 -> Nothing
                | otherwise ->
                    let xValue =
                            if fromIntegral (root .&. 1) == signBit
                                then root
                                else modP (-root)
                     in Just (Point xValue yValue)
  where
    signBit = fromIntegral ((last encoded `shiftR` 7) .&. 1)
    yBytes = take 31 encoded ++ [last encoded .&. 0x7f]
    yValue = decodeLittleEndian yBytes
    ySquared = squareP yValue
    xSquared = mulP (subP ySquared 1) (invP (addP 1 (mulP dConstant ySquared)))
