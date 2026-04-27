module Treap
    ( Treap
    , empty
    , insert
    , delete
    , member
    , fromList
    , toList
    , rootPriority
    , isValid
    ) where

data Treap a
    = Empty
    | Node Int a (Treap a) (Treap a)
    deriving (Eq, Show)

empty :: Treap a
empty = Empty

insert :: Ord a => a -> Int -> Treap a -> Treap a
insert value priority treeValue =
    case treeValue of
        Empty -> Node priority value Empty Empty
        Node nodePriority current left right
            | value < current ->
                bubble (Node nodePriority current (insert value priority left) right)
            | value > current ->
                bubble (Node nodePriority current left (insert value priority right))
            | otherwise -> treeValue

delete :: Ord a => a -> Treap a -> Treap a
delete value treeValue =
    case treeValue of
        Empty -> Empty
        Node priority current left right
            | value < current -> Node priority current (delete value left) right
            | value > current -> Node priority current left (delete value right)
            | otherwise -> merge left right

member :: Ord a => a -> Treap a -> Bool
member value treeValue =
    case treeValue of
        Empty -> False
        Node _ current left right
            | value < current -> member value left
            | value > current -> member value right
            | otherwise -> True

fromList :: Ord a => [(a, Int)] -> Treap a
fromList = foldl (\treeValue (value, priority) -> insert value priority treeValue) empty

toList :: Treap a -> [a]
toList Empty = []
toList (Node _ value left right) = toList left ++ [value] ++ toList right

rootPriority :: Treap a -> Maybe Int
rootPriority Empty = Nothing
rootPriority (Node priority _ _ _) = Just priority

isValid :: Ord a => Treap a -> Bool
isValid treeValue =
    ordered (toList treeValue) && heapOrdered treeValue
  where
    ordered [] = True
    ordered [_] = True
    ordered (left : right : rest) = left < right && ordered (right : rest)

heapOrdered Empty = True
heapOrdered (Node priority _ left right) =
    maybe True (>= priority) (rootPriority left)
        && maybe True (>= priority) (rootPriority right)
        && heapOrdered left
        && heapOrdered right

bubble :: Treap a -> Treap a
bubble (Node priority value left right) =
    case (left, right) of
        (Node leftPriority leftValue leftLeft leftRight, _)
            | leftPriority < priority ->
                Node leftPriority leftValue leftLeft (Node priority value leftRight right)
        (_, Node rightPriority rightValue rightLeft rightRight)
            | rightPriority < priority ->
                Node rightPriority rightValue (Node priority value left rightLeft) rightRight
        _ -> Node priority value left right
bubble treeValue = treeValue

merge :: Treap a -> Treap a -> Treap a
merge Empty right = right
merge left Empty = left
merge left@(Node leftPriority leftValue leftLeft leftRight) right@(Node rightPriority rightValue rightLeft rightRight)
    | leftPriority <= rightPriority =
        Node leftPriority leftValue leftLeft (merge leftRight right)
    | otherwise =
        Node rightPriority rightValue (merge left rightLeft) rightRight
