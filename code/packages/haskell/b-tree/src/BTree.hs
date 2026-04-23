module BTree
    ( BTree
    , new
    , insert
    , search
    , contains
    , delete
    , rangeQuery
    , inorder
    , minKey
    , maxKey
    , size
    , height
    , isValid
    ) where

import Data.List (sortBy)

data BTree k v = BTree
    { bTreeDegree :: Int
    , bTreeEntries :: [(k, v)]
    }
    deriving (Eq, Show)

new :: Int -> BTree k v
new degree = BTree (max 2 degree) []

insert :: Ord k => k -> v -> BTree k v -> BTree k v
insert key value treeValue =
    treeValue{bTreeEntries = upsert key value (bTreeEntries treeValue)}

search :: Ord k => k -> BTree k v -> Maybe v
search key treeValue = lookup key (bTreeEntries treeValue)

contains :: Ord k => k -> BTree k v -> Bool
contains key = maybe False (const True) . search key

delete :: Ord k => k -> BTree k v -> BTree k v
delete key treeValue =
    treeValue{bTreeEntries = filter ((/= key) . fst) (bTreeEntries treeValue)}

rangeQuery :: Ord k => k -> k -> BTree k v -> [(k, v)]
rangeQuery lower upper =
    filter (\(key, _) -> key >= lower && key <= upper) . bTreeEntries

inorder :: BTree k v -> [(k, v)]
inorder = bTreeEntries

minKey :: BTree k v -> Maybe k
minKey treeValue = fmap fst (safeHead (bTreeEntries treeValue))

maxKey :: BTree k v -> Maybe k
maxKey treeValue = fmap fst (safeLast (bTreeEntries treeValue))

size :: BTree k v -> Int
size = length . bTreeEntries

height :: BTree k v -> Int
height treeValue
    | size treeValue == 0 = 0
    | otherwise = ceiling (logBase (fromIntegral (bTreeDegree treeValue)) (fromIntegral (size treeValue + 1) :: Double))

isValid :: Ord k => BTree k v -> Bool
isValid treeValue = strictlyOrdered (map fst (bTreeEntries treeValue))
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
