module HashMap
    ( description
    , HashMap
    , empty
    , singleton
    , isEmpty
    , size
    , has
    , get
    , set
    , delete
    , keys
    , values
    , entries
    , fromList
    , toList
    ) where

import qualified Data.Map.Strict as Map

description :: String
description = "Haskell hash map for DT18 and Mini Redis"

newtype HashMap key value = HashMap
    { unHashMap :: Map.Map key value
    }
    deriving (Eq, Show)

empty :: HashMap key value
empty = HashMap Map.empty

singleton :: Ord key => key -> value -> HashMap key value
singleton key value = HashMap (Map.singleton key value)

isEmpty :: HashMap key value -> Bool
isEmpty (HashMap valuesMap) = Map.null valuesMap

size :: HashMap key value -> Int
size (HashMap valuesMap) = Map.size valuesMap

has :: Ord key => key -> HashMap key value -> Bool
has key (HashMap valuesMap) = Map.member key valuesMap

get :: Ord key => key -> HashMap key value -> Maybe value
get key (HashMap valuesMap) = Map.lookup key valuesMap

set :: Ord key => key -> value -> HashMap key value -> HashMap key value
set key value (HashMap valuesMap) = HashMap (Map.insert key value valuesMap)

delete :: Ord key => key -> HashMap key value -> HashMap key value
delete key (HashMap valuesMap) = HashMap (Map.delete key valuesMap)

keys :: HashMap key value -> [key]
keys (HashMap valuesMap) = Map.keys valuesMap

values :: HashMap key value -> [value]
values (HashMap valuesMap) = Map.elems valuesMap

entries :: HashMap key value -> [(key, value)]
entries (HashMap valuesMap) = Map.toAscList valuesMap

fromList :: Ord key => [(key, value)] -> HashMap key value
fromList = HashMap . Map.fromList

toList :: HashMap key value -> [(key, value)]
toList = entries
