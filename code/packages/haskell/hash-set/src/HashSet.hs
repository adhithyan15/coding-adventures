module HashSet
    ( description
    , HashSet
    , empty
    , singleton
    , isEmpty
    , size
    , contains
    , add
    , remove
    , fromList
    , toList
    , union
    , intersection
    , difference
    ) where

import qualified Data.Set as Set

description :: String
description = "Haskell hash set for DT19 and Mini Redis"

newtype HashSet value = HashSet
    { unHashSet :: Set.Set value
    }
    deriving (Eq, Show)

empty :: HashSet value
empty = HashSet Set.empty

singleton :: Ord value => value -> HashSet value
singleton value = HashSet (Set.singleton value)

isEmpty :: HashSet value -> Bool
isEmpty (HashSet valuesSet) = Set.null valuesSet

size :: HashSet value -> Int
size (HashSet valuesSet) = Set.size valuesSet

contains :: Ord value => value -> HashSet value -> Bool
contains value (HashSet valuesSet) = Set.member value valuesSet

add :: Ord value => value -> HashSet value -> HashSet value
add value (HashSet valuesSet) = HashSet (Set.insert value valuesSet)

remove :: Ord value => value -> HashSet value -> HashSet value
remove value (HashSet valuesSet) = HashSet (Set.delete value valuesSet)

fromList :: Ord value => [value] -> HashSet value
fromList = HashSet . Set.fromList

toList :: HashSet value -> [value]
toList (HashSet valuesSet) = Set.toAscList valuesSet

union :: Ord value => HashSet value -> HashSet value -> HashSet value
union (HashSet leftSet) (HashSet rightSet) = HashSet (Set.union leftSet rightSet)

intersection :: Ord value => HashSet value -> HashSet value -> HashSet value
intersection (HashSet leftSet) (HashSet rightSet) =
    HashSet (Set.intersection leftSet rightSet)

difference :: Ord value => HashSet value -> HashSet value -> HashSet value
difference (HashSet leftSet) (HashSet rightSet) =
    HashSet (Set.difference leftSet rightSet)
