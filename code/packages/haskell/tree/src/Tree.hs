module Tree
    ( Tree
    , new
    , addChild
    , removeSubtree
    , root
    , parent
    , children
    , siblings
    , hasNode
    , isLeaf
    , isRoot
    , depth
    , height
    , size
    , nodes
    , leaves
    , preorder
    , postorder
    , levelOrder
    , pathTo
    , lca
    , subtree
    , toAscii
    ) where

import Data.List (delete, intercalate, nub, sort)
import qualified Data.Map.Strict as Map
import Data.Map.Strict (Map)

data Tree a = Tree
    { treeRoot :: a
    , treeChildren :: Map a [a]
    , treeParents :: Map a a
    }
    deriving (Eq, Show)

new :: Ord a => a -> Tree a
new rootNode =
    Tree
        { treeRoot = rootNode
        , treeChildren = Map.singleton rootNode []
        , treeParents = Map.empty
        }

addChild :: Ord a => a -> a -> Tree a -> Either String (Tree a)
addChild parentNode childNode treeValue
    | not (hasNode parentNode treeValue) = Left "parent node not found"
    | hasNode childNode treeValue = Left "child already exists"
    | otherwise =
        Right
            treeValue
                { treeChildren =
                    Map.insert childNode []
                        (Map.adjust (++ [childNode]) parentNode (treeChildren treeValue))
                , treeParents = Map.insert childNode parentNode (treeParents treeValue)
                }

removeSubtree :: Ord a => a -> Tree a -> Either String (Tree a)
removeSubtree node treeValue
    | node == treeRoot treeValue = Left "cannot remove the root subtree"
    | not (hasNode node treeValue) = Left "node not found"
    | otherwise =
        let doomed = subtreeNodes node treeValue
            parentNode = treeParents treeValue Map.! node
         in Right
                treeValue
                    { treeChildren =
                        foldr Map.delete
                            (Map.adjust (filter (`notElem` doomed)) parentNode (treeChildren treeValue))
                            doomed
                    , treeParents = foldr Map.delete (treeParents treeValue) doomed
                    }

root :: Tree a -> a
root = treeRoot

parent :: Ord a => a -> Tree a -> Maybe a
parent node treeValue = Map.lookup node (treeParents treeValue)

children :: Ord a => a -> Tree a -> [a]
children node treeValue = Map.findWithDefault [] node (treeChildren treeValue)

siblings :: (Ord a, Eq a) => a -> Tree a -> [a]
siblings node treeValue =
    case parent node treeValue of
        Nothing -> []
        Just parentNode -> filter (/= node) (children parentNode treeValue)

hasNode :: Ord a => a -> Tree a -> Bool
hasNode node treeValue = Map.member node (treeChildren treeValue)

isLeaf :: Ord a => a -> Tree a -> Bool
isLeaf node treeValue = hasNode node treeValue && null (children node treeValue)

isRoot :: Eq a => a -> Tree a -> Bool
isRoot node treeValue = node == treeRoot treeValue

depth :: (Ord a, Eq a) => a -> Tree a -> Maybe Int
depth node treeValue
    | not (hasNode node treeValue) = Nothing
    | otherwise = Just (length (pathToRoot node treeValue) - 1)

height :: Ord a => Tree a -> Int
height treeValue = go (treeRoot treeValue)
  where
    go node =
        case children node treeValue of
            [] -> 1
            next -> 1 + maximum (map go next)

size :: Tree a -> Int
size = Map.size . treeChildren

nodes :: Ord a => Tree a -> [a]
nodes treeValue = preorder treeValue

leaves :: Ord a => Tree a -> [a]
leaves treeValue =
    filter (\node -> isLeaf node treeValue) (preorder treeValue)

preorder :: Ord a => Tree a -> [a]
preorder treeValue = walk (treeRoot treeValue)
  where
    walk node = node : concatMap walk (children node treeValue)

postorder :: Ord a => Tree a -> [a]
postorder treeValue = walk (treeRoot treeValue)
  where
    walk node = concatMap walk (children node treeValue) ++ [node]

levelOrder :: Ord a => Tree a -> [a]
levelOrder treeValue = walk [treeRoot treeValue]
  where
    walk [] = []
    walk currentLayer =
        currentLayer ++ walk (concatMap (`children` treeValue) currentLayer)

pathTo :: (Ord a, Eq a) => a -> Tree a -> Maybe [a]
pathTo node treeValue
    | not (hasNode node treeValue) = Nothing
    | otherwise = Just (reverse (pathToRoot node treeValue))

lca :: (Ord a, Eq a) => a -> a -> Tree a -> Maybe a
lca left right treeValue = do
    leftPath <- pathTo left treeValue
    rightPath <- pathTo right treeValue
    pure (last (commonPrefix leftPath rightPath))

subtree :: Ord a => a -> Tree a -> Maybe (Tree a)
subtree node treeValue
    | not (hasNode node treeValue) = Nothing
    | otherwise =
        let keptNodes = subtreeNodes node treeValue
         in Just
                Tree
                    { treeRoot = node
                    , treeChildren =
                        Map.filterWithKey
                            (\key _ -> key `elem` keptNodes)
                            (Map.map (filter (`elem` keptNodes)) (treeChildren treeValue))
                    , treeParents =
                        Map.filterWithKey
                            (\key parentNode -> key `elem` keptNodes && parentNode `elem` keptNodes)
                            (treeParents treeValue)
                    }

toAscii :: (Ord a, Show a) => Tree a -> String
toAscii treeValue =
    intercalate "\n" (render True "" (treeRoot treeValue))
  where
    render isRootNode prefix node =
        let label = if isRootNode then show node else prefix ++ "+-- " ++ show node
            nextChildren = children node treeValue
            childPrefix = if isRootNode then "" else prefix ++ "|   "
         in label : concatMap (render False childPrefix) nextChildren

subtreeNodes :: Ord a => a -> Tree a -> [a]
subtreeNodes node treeValue = walk [node]
  where
    walk [] = []
    walk (current : rest) =
        current : walk (children current treeValue ++ rest)

pathToRoot :: Ord a => a -> Tree a -> [a]
pathToRoot node treeValue =
    case Map.lookup node (treeParents treeValue) of
        Nothing -> [node]
        Just parentNode -> node : pathToRoot parentNode treeValue

commonPrefix :: Eq a => [a] -> [a] -> [a]
commonPrefix (left : leftRest) (right : rightRest)
    | left == right = left : commonPrefix leftRest rightRest
commonPrefix _ _ = []
