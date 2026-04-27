module BinaryTree
    ( BinaryTree(..)
    , leaf
    , size
    , height
    , inorder
    , preorder
    , postorder
    , levelOrder
    , leaves
    , mapTree
    , foldTree
    ) where

data BinaryTree a
    = EmptyTree
    | Node a (BinaryTree a) (BinaryTree a)
    deriving (Eq, Show)

leaf :: a -> BinaryTree a
leaf value = Node value EmptyTree EmptyTree

size :: BinaryTree a -> Int
size EmptyTree = 0
size (Node _ left right) = 1 + size left + size right

height :: BinaryTree a -> Int
height EmptyTree = 0
height (Node _ left right) = 1 + max (height left) (height right)

inorder :: BinaryTree a -> [a]
inorder EmptyTree = []
inorder (Node value left right) = inorder left ++ [value] ++ inorder right

preorder :: BinaryTree a -> [a]
preorder EmptyTree = []
preorder (Node value left right) = value : preorder left ++ preorder right

postorder :: BinaryTree a -> [a]
postorder EmptyTree = []
postorder (Node value left right) = postorder left ++ postorder right ++ [value]

levelOrder :: BinaryTree a -> [a]
levelOrder treeValue = walk [treeValue]
  where
    walk [] = []
    walk (EmptyTree : rest) = walk rest
    walk (Node value left right : rest) = value : walk (rest ++ [left, right])

leaves :: BinaryTree a -> [a]
leaves EmptyTree = []
leaves (Node value EmptyTree EmptyTree) = [value]
leaves (Node _ left right) = leaves left ++ leaves right

mapTree :: (a -> b) -> BinaryTree a -> BinaryTree b
mapTree _ EmptyTree = EmptyTree
mapTree fn (Node value left right) =
    Node (fn value) (mapTree fn left) (mapTree fn right)

foldTree :: (a -> b -> b -> b) -> b -> BinaryTree a -> b
foldTree _ zeroValue EmptyTree = zeroValue
foldTree fn zeroValue (Node value left right) =
    fn value (foldTree fn zeroValue left) (foldTree fn zeroValue right)
