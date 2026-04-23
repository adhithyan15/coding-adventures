module Rope
    ( Rope
    , empty
    , fromString
    , toString
    , len
    , append
    , concatRopes
    , splitRope
    , takeRope
    , dropRope
    , indexRope
    , insert
    , deleteRange
    ) where

data Rope
    = Empty
    | Leaf Int String
    | Concat Int Rope Rope
    deriving (Eq, Show)

empty :: Rope
empty = Empty

fromString :: String -> Rope
fromString "" = Empty
fromString value = Leaf (length value) value

toString :: Rope -> String
toString ropeValue =
    case ropeValue of
        Empty -> ""
        Leaf _ value -> value
        Concat _ left right -> toString left ++ toString right

len :: Rope -> Int
len ropeValue =
    case ropeValue of
        Empty -> 0
        Leaf size' _ -> size'
        Concat size' _ _ -> size'

append :: Rope -> Rope -> Rope
append Empty right = right
append left Empty = left
append left right = Concat (len left + len right) left right

concatRopes :: [Rope] -> Rope
concatRopes = foldl append empty

splitRope :: Int -> Rope -> (Rope, Rope)
splitRope index ropeValue =
    let (left, right) = splitAt index (toString ropeValue)
     in (fromString left, fromString right)

takeRope :: Int -> Rope -> Rope
takeRope index = fst . splitRope index

dropRope :: Int -> Rope -> Rope
dropRope index = snd . splitRope index

indexRope :: Int -> Rope -> Maybe Char
indexRope index ropeValue
    | index < 0 || index >= len ropeValue = Nothing
    | otherwise = Just (toString ropeValue !! index)

insert :: Int -> String -> Rope -> Rope
insert index chunk ropeValue =
    let (left, right) = splitRope index ropeValue
     in append left (append (fromString chunk) right)

deleteRange :: Int -> Int -> Rope -> Rope
deleteRange start count ropeValue =
    let prefix = takeRope start ropeValue
        suffix = dropRope (start + count) ropeValue
     in append prefix suffix
