module ImmutableList
    ( ImmutableList
    , empty
    , singleton
    , fromList
    , toList
    , push
    , pop
    , get
    , set
    , append
    , len
    ) where

import qualified Data.Sequence as Seq
import Data.Sequence (Seq)
import qualified Data.Foldable as Foldable

newtype ImmutableList a = ImmutableList (Seq a)
    deriving (Eq, Show)

empty :: ImmutableList a
empty = ImmutableList Seq.empty

singleton :: a -> ImmutableList a
singleton value = ImmutableList (Seq.singleton value)

fromList :: [a] -> ImmutableList a
fromList = ImmutableList . Seq.fromList

toList :: ImmutableList a -> [a]
toList (ImmutableList values) = Foldable.toList values

push :: a -> ImmutableList a -> ImmutableList a
push value (ImmutableList values) = ImmutableList (values Seq.|> value)

pop :: ImmutableList a -> Maybe (ImmutableList a, a)
pop (ImmutableList values) =
    case Seq.viewr values of
        Seq.EmptyR -> Nothing
        rest Seq.:> value -> Just (ImmutableList rest, value)

get :: Int -> ImmutableList a -> Maybe a
get index (ImmutableList values)
    | index < 0 || index >= Seq.length values = Nothing
    | otherwise = Just (Seq.index values index)

set :: Int -> a -> ImmutableList a -> Maybe (ImmutableList a)
set index value (ImmutableList values)
    | index < 0 || index >= Seq.length values = Nothing
    | otherwise = Just (ImmutableList (Seq.update index value values))

append :: ImmutableList a -> ImmutableList a -> ImmutableList a
append (ImmutableList left) (ImmutableList right) =
    ImmutableList (left <> right)

len :: ImmutableList a -> Int
len (ImmutableList values) = Seq.length values
