module RedBlackTree
    ( Color(..)
    , RedBlackTree
    , empty
    , insert
    , delete
    , member
    , fromList
    , toList
    , rootColor
    , isValid
    ) where

data Color = Red | Black
    deriving (Eq, Show)

data RedBlackTree a
    = Empty
    | Node Color (RedBlackTree a) a (RedBlackTree a)
    deriving (Eq, Show)

empty :: RedBlackTree a
empty = Empty

insert :: Ord a => a -> RedBlackTree a -> RedBlackTree a
insert value treeValue =
    makeBlack (go treeValue)
  where
    go Empty = Node Red Empty value Empty
    go (Node color left current right)
        | value < current = balance color (go left) current right
        | value > current = balance color left current (go right)
        | otherwise = Node color left current right

delete :: Ord a => a -> RedBlackTree a -> RedBlackTree a
delete value =
    fromList . filter (/= value) . toList

member :: Ord a => a -> RedBlackTree a -> Bool
member value treeValue =
    case treeValue of
        Empty -> False
        Node _ left current right
            | value < current -> member value left
            | value > current -> member value right
            | otherwise -> True

fromList :: Ord a => [a] -> RedBlackTree a
fromList = foldl (flip insert) empty

toList :: RedBlackTree a -> [a]
toList Empty = []
toList (Node _ left value right) = toList left ++ [value] ++ toList right

rootColor :: RedBlackTree a -> Maybe Color
rootColor Empty = Nothing
rootColor (Node color _ _ _) = Just color

isValid :: Ord a => RedBlackTree a -> Bool
isValid treeValue =
    ordered (toList treeValue)
        && rootColor treeValue /= Just Red
        && noRedRed treeValue
        && consistentBlackHeight treeValue
  where
    ordered [] = True
    ordered [_] = True
    ordered (left : right : rest) = left < right && ordered (right : rest)

noRedRed :: RedBlackTree a -> Bool
noRedRed Empty = True
noRedRed (Node color left _ right) =
    noRedRed left
        && noRedRed right
        && case color of
            Black -> True
            Red -> childColor left /= Just Red && childColor right /= Just Red

consistentBlackHeight :: RedBlackTree a -> Bool
consistentBlackHeight treeValue =
    case blackHeight treeValue of
        Nothing -> False
        Just _ -> True

blackHeight :: RedBlackTree a -> Maybe Int
blackHeight Empty = Just 1
blackHeight (Node color left _ right) = do
    leftHeight <- blackHeight left
    rightHeight <- blackHeight right
    if leftHeight /= rightHeight
        then Nothing
        else Just (leftHeight + if color == Black then 1 else 0)

childColor :: RedBlackTree a -> Maybe Color
childColor Empty = Nothing
childColor (Node color _ _ _) = Just color

makeBlack :: RedBlackTree a -> RedBlackTree a
makeBlack Empty = Empty
makeBlack (Node _ left value right) = Node Black left value right

balance :: Color -> RedBlackTree a -> a -> RedBlackTree a -> RedBlackTree a
balance Black (Node Red (Node Red a x b) y c) z d =
    Node Red (Node Black a x b) y (Node Black c z d)
balance Black (Node Red a x (Node Red b y c)) z d =
    Node Red (Node Black a x b) y (Node Black c z d)
balance Black a x (Node Red (Node Red b y c) z d) =
    Node Red (Node Black a x b) y (Node Black c z d)
balance Black a x (Node Red b y (Node Red c z d)) =
    Node Red (Node Black a x b) y (Node Black c z d)
balance color left value right =
    Node color left value right
