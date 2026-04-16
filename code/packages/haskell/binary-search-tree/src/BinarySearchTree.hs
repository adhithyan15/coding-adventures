module BinarySearchTree
    ( BinarySearchTree
    , empty
    , insert
    , member
    , delete
    , fromList
    , toList
    , minValue
    , maxValue
    , rangeQuery
    , predecessor
    , successor
    , height
    , isValid
    ) where

data BinarySearchTree a
    = Empty
    | Node a (BinarySearchTree a) (BinarySearchTree a)
    deriving (Eq, Show)

empty :: BinarySearchTree a
empty = Empty

insert :: Ord a => a -> BinarySearchTree a -> BinarySearchTree a
insert value treeValue =
    case treeValue of
        Empty -> Node value Empty Empty
        Node current left right
            | value < current -> Node current (insert value left) right
            | value > current -> Node current left (insert value right)
            | otherwise -> treeValue

member :: Ord a => a -> BinarySearchTree a -> Bool
member value treeValue =
    case treeValue of
        Empty -> False
        Node current left right
            | value < current -> member value left
            | value > current -> member value right
            | otherwise -> True

delete :: Ord a => a -> BinarySearchTree a -> BinarySearchTree a
delete value treeValue =
    case treeValue of
        Empty -> Empty
        Node current left right
            | value < current -> Node current (delete value left) right
            | value > current -> Node current left (delete value right)
            | otherwise ->
                case (left, right) of
                    (Empty, _) -> right
                    (_, Empty) -> left
                    _ ->
                        case minValue right of
                            Nothing -> left
                            Just successorValue ->
                                Node successorValue left (delete successorValue right)

fromList :: Ord a => [a] -> BinarySearchTree a
fromList = foldl (flip insert) empty

toList :: BinarySearchTree a -> [a]
toList Empty = []
toList (Node value left right) = toList left ++ [value] ++ toList right

minValue :: BinarySearchTree a -> Maybe a
minValue Empty = Nothing
minValue (Node value Empty _) = Just value
minValue (Node _ left _) = minValue left

maxValue :: BinarySearchTree a -> Maybe a
maxValue Empty = Nothing
maxValue (Node value _ Empty) = Just value
maxValue (Node _ _ right) = maxValue right

rangeQuery :: Ord a => a -> a -> BinarySearchTree a -> [a]
rangeQuery lower upper treeValue =
    [ value
    | value <- toList treeValue
    , value >= lower
    , value <= upper
    ]

predecessor :: Ord a => a -> BinarySearchTree a -> Maybe a
predecessor value treeValue =
    go Nothing treeValue
  where
    go candidate Empty = candidate
    go candidate (Node current left right)
        | value <= current = go candidate left
        | otherwise = go (Just current) right

successor :: Ord a => a -> BinarySearchTree a -> Maybe a
successor value treeValue =
    go Nothing treeValue
  where
    go candidate Empty = candidate
    go candidate (Node current left right)
        | value >= current = go candidate right
        | otherwise = go (Just current) left

height :: BinarySearchTree a -> Int
height Empty = 0
height (Node _ left right) = 1 + max (height left) (height right)

isValid :: Ord a => BinarySearchTree a -> Bool
isValid treeValue = ordered (toList treeValue)
  where
    ordered [] = True
    ordered [_] = True
    ordered (left : right : rest) = left < right && ordered (right : rest)
