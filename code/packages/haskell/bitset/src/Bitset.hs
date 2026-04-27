module Bitset
    ( Bitset
    , empty
    , new
    , fromIntegerValue
    , fromBinaryString
    , setBit
    , clearBit
    , toggleBit
    , testBit
    , andBitset
    , orBitset
    , xorBitset
    , andNotBitset
    , notBitset
    , popCount
    , len
    , capacity
    , anyBits
    , allBits
    , noneBits
    , iterSetBits
    , toIntegerValue
    , toBinaryString
    ) where

import qualified Data.Bits as Bits

data Bitset = Bitset
    { bitsetLength :: Int
    , bitsetValue :: Integer
    }
    deriving (Eq, Show)

empty :: Bitset
empty = Bitset 0 0

new :: Int -> Bitset
new requestedLength = Bitset (max 0 requestedLength) 0

fromIntegerValue :: Integer -> Bitset
fromIntegerValue value
    | value <= 0 = empty
    | otherwise =
        Bitset
            { bitsetLength = highestBitIndex value + 1
            , bitsetValue = value
            }

fromBinaryString :: String -> Either String Bitset
fromBinaryString input
    | null input = Right empty
    | any (`notElem` "01") input = Left "binary string may contain only 0 and 1"
    | otherwise =
        Right
            ( Bitset
                { bitsetLength = length input
                , bitsetValue = foldl step 0 input
                }
            )
  where
    step acc charValue =
        acc * 2 + if charValue == '1' then 1 else 0

setBit :: Int -> Bitset -> Bitset
setBit index bitset =
    normalize $
        ensureLength (index + 1) bitset
            { bitsetValue = Bits.setBit (bitsetValue (ensureLength (index + 1) bitset)) index
            }

clearBit :: Int -> Bitset -> Bitset
clearBit index bitset
    | index < 0 = bitset
    | otherwise =
        normalize $
            bitset
                { bitsetValue = Bits.clearBit (bitsetValue bitset) index
                }

toggleBit :: Int -> Bitset -> Bitset
toggleBit index bitset =
    normalize $
        ensureLength (index + 1) bitset
            { bitsetValue = Bits.xor (bitsetValue (ensureLength (index + 1) bitset)) (Bits.bit index)
            }

testBit :: Int -> Bitset -> Bool
testBit index bitset =
    index >= 0
        && index < bitsetLength bitset
        && Bits.testBit (bitsetValue bitset) index

andBitset :: Bitset -> Bitset -> Bitset
andBitset left right =
    normalize $
        Bitset
            { bitsetLength = max (bitsetLength left) (bitsetLength right)
            , bitsetValue = (Bits..&.) (bitsetValue left) (bitsetValue right)
            }

orBitset :: Bitset -> Bitset -> Bitset
orBitset left right =
    normalize $
        Bitset
            { bitsetLength = max (bitsetLength left) (bitsetLength right)
            , bitsetValue = (Bits..|.) (bitsetValue left) (bitsetValue right)
            }

xorBitset :: Bitset -> Bitset -> Bitset
xorBitset left right =
    normalize $
        Bitset
            { bitsetLength = max (bitsetLength left) (bitsetLength right)
            , bitsetValue = Bits.xor (bitsetValue left) (bitsetValue right)
            }

andNotBitset :: Bitset -> Bitset -> Bitset
andNotBitset left right =
    andBitset left (notBitset (ensureLength (bitsetLength left) right))

notBitset :: Bitset -> Bitset
notBitset bitset =
    normalize $
        bitset
            { bitsetValue = Bits.xor (bitMask (bitsetLength bitset)) (bitsetValue bitset)
            }

popCount :: Bitset -> Int
popCount = Bits.popCount . bitsetValue

len :: Bitset -> Int
len = bitsetLength

capacity :: Bitset -> Int
capacity bitset =
    if bitsetLength bitset == 0
        then 0
        else ((bitsetLength bitset - 1) `div` 64 + 1) * 64

anyBits :: Bitset -> Bool
anyBits bitset = bitsetValue bitset /= 0

allBits :: Bitset -> Bool
allBits bitset =
    bitsetLength bitset > 0
        && bitsetValue bitset == bitMask (bitsetLength bitset)

noneBits :: Bitset -> Bool
noneBits = Prelude.not . anyBits

iterSetBits :: Bitset -> [Int]
iterSetBits bitset =
    [ index
    | index <- [0 .. bitsetLength bitset - 1]
    , Bits.testBit (bitsetValue bitset) index
    ]

toIntegerValue :: Bitset -> Integer
toIntegerValue = bitsetValue

toBinaryString :: Bitset -> String
toBinaryString bitset
    | bitsetLength bitset == 0 = ""
    | otherwise =
        [ if Bits.testBit (bitsetValue bitset) index then '1' else '0'
        | index <- reverse [0 .. bitsetLength bitset - 1]
        ]

ensureLength :: Int -> Bitset -> Bitset
ensureLength requestedLength bitset =
    bitset{bitsetLength = max (bitsetLength bitset) (max 0 requestedLength)}

normalize :: Bitset -> Bitset
normalize bitset =
    bitset{bitsetValue = (Bits..&.) (bitsetValue bitset) (bitMask (bitsetLength bitset))}

bitMask :: Int -> Integer
bitMask requestedLength
    | requestedLength <= 0 = 0
    | otherwise = Bits.bit requestedLength - 1

highestBitIndex :: Integer -> Int
highestBitIndex value =
    go 0 value
  where
    go index current
        | current <= 1 = index
        | otherwise = go (index + 1) (current `div` 2)
