module BloomFilter
    ( BloomFilter
    , new
    , fromList
    , insert
    , mightContain
    , bitset
    , size
    , hashCount
    ) where

import qualified Data.Char as Char

import qualified Bitset
import Bitset (Bitset)

data BloomFilter = BloomFilter
    { bloomSize :: Int
    , bloomHashCount :: Int
    , bloomBitset :: Bitset
    }
    deriving (Eq, Show)

new :: Int -> Int -> BloomFilter
new requestedSize requestedHashCount =
    BloomFilter
        { bloomSize = max 1 requestedSize
        , bloomHashCount = max 1 requestedHashCount
        , bloomBitset = Bitset.new (max 1 requestedSize)
        }

fromList :: Int -> Int -> [String] -> BloomFilter
fromList requestedSize requestedHashCount =
    foldl (flip insert) (new requestedSize requestedHashCount)

insert :: String -> BloomFilter -> BloomFilter
insert value filterState =
    filterState
        { bloomBitset =
            foldl
                (\current index -> Bitset.setBit index current)
                (bloomBitset filterState)
                (hashIndexes value filterState)
        }

mightContain :: String -> BloomFilter -> Bool
mightContain value filterState =
    all (\index -> Bitset.testBit index (bloomBitset filterState)) (hashIndexes value filterState)

bitset :: BloomFilter -> Bitset
bitset = bloomBitset

size :: BloomFilter -> Int
size = bloomSize

hashCount :: BloomFilter -> Int
hashCount = bloomHashCount

hashIndexes :: String -> BloomFilter -> [Int]
hashIndexes value filterState =
    [ hashWithSalt salt value `mod` bloomSize filterState
    | salt <- [0 .. bloomHashCount filterState - 1]
    ]

hashWithSalt :: Int -> String -> Int
hashWithSalt salt =
    foldl step (146959810 + salt * 16777619)
  where
    step acc charValue =
        (acc * 16777619 + Char.ord charValue + salt * 97) `mod` 2147483647
