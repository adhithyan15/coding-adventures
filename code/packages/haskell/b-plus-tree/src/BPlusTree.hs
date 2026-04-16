module BPlusTree
    ( BPlusTree
    , new
    , insert
    , search
    , contains
    , delete
    , rangeScan
    , fullScan
    , minKey
    , maxKey
    , size
    , height
    , isValid
    ) where

import Data.List (sortBy)

data BPlusTree k v = BPlusTree
    { bPlusDegree :: Int
    , bPlusEntries :: [(k, v)]
    }
    deriving (Eq, Show)

new :: Int -> BPlusTree k v
new degree = BPlusTree (max 2 degree) []

insert :: Ord k => k -> v -> BPlusTree k v -> BPlusTree k v
insert key value treeValue =
    treeValue{bPlusEntries = upsert key value (bPlusEntries treeValue)}

search :: Ord k => k -> BPlusTree k v -> Maybe v
search key treeValue = lookup key (bPlusEntries treeValue)

contains :: Ord k => k -> BPlusTree k v -> Bool
contains key = maybe False (const True) . search key

delete :: Ord k => k -> BPlusTree k v -> BPlusTree k v
delete key treeValue =
    treeValue{bPlusEntries = filter ((/= key) . fst) (bPlusEntries treeValue)}

rangeScan :: Ord k => k -> k -> BPlusTree k v -> [(k, v)]
rangeScan lower upper =
    filter (\(key, _) -> key >= lower && key <= upper) . bPlusEntries

fullScan :: BPlusTree k v -> [(k, v)]
fullScan = bPlusEntries

minKey :: BPlusTree k v -> Maybe k
minKey treeValue = fmap fst (safeHead (bPlusEntries treeValue))

maxKey :: BPlusTree k v -> Maybe k
maxKey treeValue = fmap fst (safeLast (bPlusEntries treeValue))

size :: BPlusTree k v -> Int
size = length . bPlusEntries

height :: BPlusTree k v -> Int
height treeValue
    | size treeValue == 0 = 0
    | otherwise = ceiling (logBase (fromIntegral (bPlusDegree treeValue)) (fromIntegral (size treeValue + 1) :: Double))

isValid :: Ord k => BPlusTree k v -> Bool
isValid treeValue = strictlyOrdered (map fst (bPlusEntries treeValue))
  where
    strictlyOrdered [] = True
    strictlyOrdered [_] = True
    strictlyOrdered (left : right : rest) = left < right && strictlyOrdered (right : rest)

upsert :: Ord k => k -> v -> [(k, v)] -> [(k, v)]
upsert key value entries =
    sortBy (\(left, _) (right, _) -> compare left right) (replace entries)
  where
    replace [] = [(key, value)]
    replace ((currentKey, currentValue) : rest)
        | key == currentKey = (key, value) : rest
        | otherwise = (currentKey, currentValue) : replace rest

safeHead :: [a] -> Maybe a
safeHead [] = Nothing
safeHead (value : _) = Just value

safeLast :: [a] -> Maybe a
safeLast [] = Nothing
safeLast values = Just (last values)
