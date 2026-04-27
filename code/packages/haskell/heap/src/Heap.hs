module Heap
    ( description
    , MinHeap
    , new
    , empty
    , singleton
    , isEmpty
    , size
    , push
    , peek
    , pop
    , minView
    , fromList
    , toAscList
    ) where

import Data.List (foldl', insert)

description :: String
description = "Haskell heap package for TTL scheduling"

newtype MinHeap value = MinHeap
    { unMinHeap :: [value]
    }
    deriving (Eq, Show)

new :: MinHeap value
new = MinHeap []

empty :: MinHeap value
empty = new

singleton :: value -> MinHeap value
singleton value = MinHeap [value]

isEmpty :: MinHeap value -> Bool
isEmpty (MinHeap valuesList) = null valuesList

size :: MinHeap value -> Int
size (MinHeap valuesList) = length valuesList

push :: Ord value => value -> MinHeap value -> MinHeap value
push value (MinHeap valuesList) = MinHeap (insert value valuesList)

peek :: MinHeap value -> Maybe value
peek (MinHeap valuesList) =
    case valuesList of
        [] -> Nothing
        value : _ -> Just value

pop :: MinHeap value -> MinHeap value
pop (MinHeap valuesList) =
    case valuesList of
        [] -> MinHeap []
        _ : rest -> MinHeap rest

minView :: MinHeap value -> Maybe (value, MinHeap value)
minView valuesHeap =
    case peek valuesHeap of
        Nothing -> Nothing
        Just value -> Just (value, pop valuesHeap)

fromList :: Ord value => [value] -> MinHeap value
fromList = foldl' (flip push) new

toAscList :: MinHeap value -> [value]
toAscList (MinHeap valuesList) = valuesList
