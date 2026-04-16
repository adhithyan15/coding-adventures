module TreeSet
    ( TreeSet
    , empty
    , fromList
    , toSortedList
    , contains
    , insert
    , delete
    , union
    , intersection
    , difference
    , range
    , rank
    , predecessor
    , successor
    ) where

import Prelude hiding (union)
import qualified Data.Set as Set
import Data.Set (Set)

newtype TreeSet a = TreeSet (Set a)
    deriving (Eq, Show)

empty :: TreeSet a
empty = TreeSet Set.empty

fromList :: Ord a => [a] -> TreeSet a
fromList = TreeSet . Set.fromList

toSortedList :: TreeSet a -> [a]
toSortedList (TreeSet values) = Set.toAscList values

contains :: Ord a => a -> TreeSet a -> Bool
contains value (TreeSet values) = Set.member value values

insert :: Ord a => a -> TreeSet a -> TreeSet a
insert value (TreeSet values) = TreeSet (Set.insert value values)

delete :: Ord a => a -> TreeSet a -> TreeSet a
delete value (TreeSet values) = TreeSet (Set.delete value values)

union :: Ord a => TreeSet a -> TreeSet a -> TreeSet a
union (TreeSet left) (TreeSet right) = TreeSet (Set.union left right)

intersection :: Ord a => TreeSet a -> TreeSet a -> TreeSet a
intersection (TreeSet left) (TreeSet right) = TreeSet (Set.intersection left right)

difference :: Ord a => TreeSet a -> TreeSet a -> TreeSet a
difference (TreeSet left) (TreeSet right) = TreeSet (Set.difference left right)

range :: Ord a => a -> a -> TreeSet a -> [a]
range lower upper =
    filter (\value -> value >= lower && value <= upper) . toSortedList

rank :: Ord a => a -> TreeSet a -> Int
rank value =
    length . filter (< value) . toSortedList

predecessor :: Ord a => a -> TreeSet a -> Maybe a
predecessor value =
    lastMaybe . filter (< value) . toSortedList

successor :: Ord a => a -> TreeSet a -> Maybe a
successor value =
    firstMaybe . filter (> value) . toSortedList

firstMaybe :: [a] -> Maybe a
firstMaybe [] = Nothing
firstMaybe (value : _) = Just value

lastMaybe :: [a] -> Maybe a
lastMaybe [] = Nothing
lastMaybe values = Just (last values)
