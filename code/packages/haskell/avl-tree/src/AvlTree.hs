module AvlTree
    ( AvlTree
    , empty
    , insert
    , delete
    , member
    , fromList
    , toList
    , height
    , isBalanced
    ) where

data AvlTree a
    = Empty
    | Node Int a (AvlTree a) (AvlTree a)
    deriving (Eq, Show)

empty :: AvlTree a
empty = Empty

insert :: Ord a => a -> AvlTree a -> AvlTree a
insert value treeValue =
    balance $
        case treeValue of
            Empty -> Node 1 value Empty Empty
            Node _ current left right
                | value < current -> mkNode current (insert value left) right
                | value > current -> mkNode current left (insert value right)
                | otherwise -> treeValue

delete :: Ord a => a -> AvlTree a -> AvlTree a
delete value =
    fromList . filter (/= value) . toList

member :: Ord a => a -> AvlTree a -> Bool
member value treeValue =
    case treeValue of
        Empty -> False
        Node _ current left right
            | value < current -> member value left
            | value > current -> member value right
            | otherwise -> True

fromList :: Ord a => [a] -> AvlTree a
fromList = foldl (flip insert) empty

toList :: AvlTree a -> [a]
toList Empty = []
toList (Node _ value left right) = toList left ++ [value] ++ toList right

height :: AvlTree a -> Int
height Empty = 0
height (Node nodeHeight _ _ _) = nodeHeight

isBalanced :: AvlTree a -> Bool
isBalanced Empty = True
isBalanced (Node _ _ left right) =
    abs (height left - height right) <= 1
        && isBalanced left
        && isBalanced right

mkNode :: a -> AvlTree a -> AvlTree a -> AvlTree a
mkNode value left right =
    Node (1 + max (height left) (height right)) value left right

balance :: AvlTree a -> AvlTree a
balance Empty = Empty
balance node@(Node _ value left right)
    | balanceFactor node > 1 =
        case left of
            Empty -> node
            leftNode@(Node _ _ leftLeft leftRight)
                | height leftLeft >= height leftRight -> rotateRight node
                | otherwise -> rotateRight (mkNode value (rotateLeft leftNode) right)
    | balanceFactor node < -1 =
        case right of
            Empty -> node
            rightNode@(Node _ _ rightLeft rightRight)
                | height rightRight >= height rightLeft -> rotateLeft node
                | otherwise -> rotateLeft (mkNode value left (rotateRight rightNode))
    | otherwise = mkNode value left right

balanceFactor :: AvlTree a -> Int
balanceFactor Empty = 0
balanceFactor (Node _ _ left right) = height left - height right

rotateLeft :: AvlTree a -> AvlTree a
rotateLeft (Node _ value left (Node _ rightValue rightLeft rightRight)) =
    mkNode rightValue (mkNode value left rightLeft) rightRight
rotateLeft treeValue = treeValue

rotateRight :: AvlTree a -> AvlTree a
rotateRight (Node _ value (Node _ leftValue leftLeft leftRight) right) =
    mkNode leftValue leftLeft (mkNode value leftRight right)
rotateRight treeValue = treeValue
