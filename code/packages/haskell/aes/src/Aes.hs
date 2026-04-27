module Aes
    ( description
    , expandKey
    , encryptBlock
    , decryptBlock
    ) where

import Data.Array (Array, (!), array)
import Data.Bits
    ( (.&.)
    , rotateL
    , shiftL
    , shiftR
    , testBit
    , xor
    )
import Data.List (foldl')
import Data.Word (Word8)

description :: String
description = "AES block cipher (FIPS 197) for 128-bit blocks and 128/192/256-bit keys"

type Word4 = [Word8]
type State = [[Word8]]

expandKey :: [Word8] -> Either String [[Word8]]
expandKey keyBytes
    | length keyBytes `notElem` [16, 24, 32] =
        Left ("AES key must be 16, 24, or 32 bytes; got " ++ show (length keyBytes))
    | otherwise =
        Right (map roundKeyToBytes (roundKeys keyBytes))

encryptBlock :: [Word8] -> [Word8] -> Either String [Word8]
encryptBlock block keyBytes
    | length block /= 16 =
        Left ("AES block must be 16 bytes; got " ++ show (length block))
    | otherwise =
        do
            let keys = roundKeys keyBytes
            if null keys
                then Left "AES key expansion failed"
                else Right (stateToBytes (encryptWithKeys (bytesToState block) keys))

decryptBlock :: [Word8] -> [Word8] -> Either String [Word8]
decryptBlock block keyBytes
    | length block /= 16 =
        Left ("AES block must be 16 bytes; got " ++ show (length block))
    | otherwise =
        do
            let keys = roundKeys keyBytes
            if null keys
                then Left "AES key expansion failed"
                else Right (stateToBytes (decryptWithKeys (bytesToState block) keys))

roundKeys :: [Word8] -> [State]
roundKeys keyBytes
    | length keyBytes `notElem` [16, 24, 32] = []
    | otherwise = map wordsToState (chunksOf 4 expandedWords)
  where
    nk = length keyBytes `div` 4
    nr
        | nk == 4 = 10
        | nk == 6 = 12
        | otherwise = 14
    totalWords = 4 * (nr + 1)
    initialWords = chunksOf 4 keyBytes
    expandedWords = expandWords nk totalWords initialWords nk

expandWords :: Int -> Int -> [Word4] -> Int -> [Word4]
expandWords nk totalWords wordsSoFar index
    | index >= totalWords = wordsSoFar
    | otherwise = expandWords nk totalWords (wordsSoFar ++ [nextWord]) (index + 1)
  where
    previousWord = last wordsSoFar
    adjustedWord
        | index `mod` nk == 0 =
            xorWord (subWord (rotWord previousWord)) [rcon !! (index `div` nk), 0, 0, 0]
        | nk == 8 && index `mod` nk == 4 =
            subWord previousWord
        | otherwise = previousWord
    nextWord = xorWord (wordsSoFar !! (index - nk)) adjustedWord

rcon :: [Word8]
rcon = [0x00, 0x01, 0x02, 0x04, 0x08, 0x10, 0x20, 0x40, 0x80, 0x1b, 0x36, 0x6c, 0xd8, 0xab, 0x4d]

rotWord :: Word4 -> Word4
rotWord [a, b, c, d] = [b, c, d, a]
rotWord values = values

subWord :: Word4 -> Word4
subWord = map subByte

xorWord :: Word4 -> Word4 -> Word4
xorWord = zipWith xor

encryptWithKeys :: State -> [State] -> State
encryptWithKeys state [] = state
encryptWithKeys state [onlyKey] = addRoundKey state onlyKey
encryptWithKeys state (firstKey : remainingKeys) =
    case reverse remainingKeys of
        [] -> addRoundKey state firstKey
        finalKey : reversedMiddleKeys ->
            addRoundKey finalState finalKey
          where
            middleKeys = reverse reversedMiddleKeys
            firstState = addRoundKey state firstKey
            preFinalState =
                foldl'
                    (\currentState roundKeyState -> addRoundKey (mixColumns (shiftRows (subBytes currentState))) roundKeyState)
                    firstState
                    middleKeys
            finalState = shiftRows (subBytes preFinalState)

decryptWithKeys :: State -> [State] -> State
decryptWithKeys state [] = state
decryptWithKeys state [onlyKey] = addRoundKey state onlyKey
decryptWithKeys state keys =
    case reverse keys of
        [] -> state
        firstKey : remainingKeys ->
            case reverse remainingKeys of
                [] -> addRoundKey state firstKey
                finalKey : reversedMiddleKeys ->
                    addRoundKey finalState finalKey
                  where
                    middleKeys = reverse reversedMiddleKeys
                    firstState = addRoundKey state firstKey
                    preFinalState =
                        foldl'
                            (\currentState roundKeyState -> invMixColumns (addRoundKey (invSubBytes (invShiftRows currentState)) roundKeyState))
                            firstState
                            middleKeys
                    finalState = invSubBytes (invShiftRows preFinalState)

bytesToState :: [Word8] -> State
bytesToState block =
    [ [ block !! (row + 4 * column)
      | column <- [0 .. 3]
      ]
    | row <- [0 .. 3]
    ]

stateToBytes :: State -> [Word8]
stateToBytes state =
    [ state !! row !! column
    | column <- [0 .. 3]
    , row <- [0 .. 3]
    ]

roundKeyToBytes :: State -> [Word8]
roundKeyToBytes = stateToBytes

wordsToState :: [Word4] -> State
wordsToState wordsForRound =
    [ [ wordsForRound !! column !! row
      | column <- [0 .. 3]
      ]
    | row <- [0 .. 3]
    ]

addRoundKey :: State -> State -> State
addRoundKey =
    zipWith (zipWith xor)

subBytes :: State -> State
subBytes = map (map subByte)

invSubBytes :: State -> State
invSubBytes = map (map invSubByte)

shiftRows :: State -> State
shiftRows state =
    [ shiftLeft rowIndex rowValues
    | (rowIndex, rowValues) <- zip [0 ..] state
    ]

invShiftRows :: State -> State
invShiftRows state =
    [ shiftRight rowIndex rowValues
    | (rowIndex, rowValues) <- zip [0 ..] state
    ]

shiftLeft :: Int -> [a] -> [a]
shiftLeft count values =
    drop offset values ++ take offset values
  where
    offset = count `mod` length values

shiftRight :: Int -> [a] -> [a]
shiftRight count values =
    drop offset values ++ take offset values
  where
    offset = (length values - (count `mod` length values)) `mod` length values

mixColumns :: State -> State
mixColumns state =
    transposeColumns (map mixColumn (columns state))

invMixColumns :: State -> State
invMixColumns state =
    transposeColumns (map invMixColumn (columns state))

columns :: State -> [[Word8]]
columns state =
    [ [ state !! row !! column
      | row <- [0 .. 3]
      ]
    | column <- [0 .. 3]
    ]

transposeColumns :: [[Word8]] -> State
transposeColumns cols =
    [ [ cols !! column !! row
      | column <- [0 .. 3]
      ]
    | row <- [0 .. 3]
    ]

mixColumn :: [Word8] -> [Word8]
mixColumn [s0, s1, s2, s3] =
    [ gfMul 0x02 s0 `xor` gfMul 0x03 s1 `xor` s2 `xor` s3
    , s0 `xor` gfMul 0x02 s1 `xor` gfMul 0x03 s2 `xor` s3
    , s0 `xor` s1 `xor` gfMul 0x02 s2 `xor` gfMul 0x03 s3
    , gfMul 0x03 s0 `xor` s1 `xor` s2 `xor` gfMul 0x02 s3
    ]
mixColumn values = values

invMixColumn :: [Word8] -> [Word8]
invMixColumn [s0, s1, s2, s3] =
    [ gfMul 0x0e s0 `xor` gfMul 0x0b s1 `xor` gfMul 0x0d s2 `xor` gfMul 0x09 s3
    , gfMul 0x09 s0 `xor` gfMul 0x0e s1 `xor` gfMul 0x0b s2 `xor` gfMul 0x0d s3
    , gfMul 0x0d s0 `xor` gfMul 0x09 s1 `xor` gfMul 0x0e s2 `xor` gfMul 0x0b s3
    , gfMul 0x0b s0 `xor` gfMul 0x0d s1 `xor` gfMul 0x09 s2 `xor` gfMul 0x0e s3
    ]
invMixColumn values = values

gfMul :: Word8 -> Word8 -> Word8
gfMul aValue bValue =
    go aValue bValue 0 0
  where
    go multiplicand multiplier result 8 = result
    go multiplicand multiplier result step =
        let result' =
                if multiplier .&. 1 /= 0
                    then result `xor` multiplicand
                    else result
            highBit = testBit multiplicand 7
            multiplicand' =
                if highBit
                    then shiftL multiplicand 1 `xor` 0x1b
                    else shiftL multiplicand 1
            multiplier' = shiftR multiplier 1
         in go multiplicand' multiplier' result' (step + 1)

subByte :: Word8 -> Word8
subByte value = sbox ! fromIntegral value

invSubByte :: Word8 -> Word8
invSubByte value = invSbox ! fromIntegral value

sbox :: Array Int Word8
sbox =
    array (0, 255) [(index, affineTransform (multiplicativeInverse (fromIntegral index))) | index <- [0 .. 255]]

invSbox :: Array Int Word8
invSbox =
    array (0, 255) [(fromIntegral substituted, fromIntegral original) | original <- [0 .. 255], let substituted = subByte (fromIntegral original)]

multiplicativeInverse :: Word8 -> Word8
multiplicativeInverse 0 = 0
multiplicativeInverse value =
    case [candidate | candidate <- [1 .. 255], gfMul value candidate == 1] of
        candidate : _ -> candidate
        [] -> 0

affineTransform :: Word8 -> Word8
affineTransform byteValue =
    foldl' xor 0x63 [rotateL byteValue rotation | rotation <- [0 .. 4]]

chunksOf :: Int -> [a] -> [[a]]
chunksOf _ [] = []
chunksOf size values =
    let (chunk, rest) = splitAt size values
     in chunk : chunksOf size rest
