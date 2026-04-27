module SkipList
    ( description
    , SkipList
    , new
    , empty
    , isEmpty
    , size
    , insert
    , delete
    , member
    , find
    , keys
    , values
    , entries
    ) where

import qualified Data.Map.Strict as Map

description :: String
description = "Haskell skip list for sorted set operations"

newtype SkipList key value = SkipList
    { unSkipList :: Map.Map key value
    }
    deriving (Eq, Show)

new :: SkipList key value
new = SkipList Map.empty

empty :: SkipList key value
empty = new

isEmpty :: SkipList key value -> Bool
isEmpty (SkipList valuesMap) = Map.null valuesMap

size :: SkipList key value -> Int
size (SkipList valuesMap) = Map.size valuesMap

insert :: Ord key => key -> value -> SkipList key value -> SkipList key value
insert key value (SkipList valuesMap) = SkipList (Map.insert key value valuesMap)

delete :: Ord key => key -> SkipList key value -> SkipList key value
delete key (SkipList valuesMap) = SkipList (Map.delete key valuesMap)

member :: Ord key => key -> SkipList key value -> Bool
member key (SkipList valuesMap) = Map.member key valuesMap

find :: Ord key => key -> SkipList key value -> Maybe value
find key (SkipList valuesMap) = Map.lookup key valuesMap

keys :: SkipList key value -> [key]
keys (SkipList valuesMap) = Map.keys valuesMap

values :: SkipList key value -> [value]
values (SkipList valuesMap) = Map.elems valuesMap

entries :: SkipList key value -> [(key, value)]
entries (SkipList valuesMap) = Map.toAscList valuesMap
