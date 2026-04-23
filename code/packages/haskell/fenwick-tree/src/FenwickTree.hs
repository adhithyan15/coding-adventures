module FenwickTree
    ( FenwickError(..)
    , FenwickTree
    , empty
    , fromList
    , size
    , update
    , prefixSum
    , rangeSum
    , pointQuery
    , findKth
    ) where

data FenwickError
    = InvalidIndex Int
    | InvalidRange Int Int
    | EmptyTreeLookup
    deriving (Eq, Show)

data FenwickTree = FenwickTree
    { fenwickValues :: [Double]
    }
    deriving (Eq, Show)

empty :: FenwickTree
empty = FenwickTree []

fromList :: [Double] -> FenwickTree
fromList = FenwickTree

size :: FenwickTree -> Int
size = length . fenwickValues

update :: Int -> Double -> FenwickTree -> Either FenwickError FenwickTree
update index delta treeState
    | index <= 0 || index > size treeState = Left (InvalidIndex index)
    | otherwise =
        Right $
            treeState
                { fenwickValues =
                    [ if position == index then value + delta else value
                    | (position, value) <- zip [1 ..] (fenwickValues treeState)
                    ]
                }

prefixSum :: Int -> FenwickTree -> Either FenwickError Double
prefixSum index treeState
    | size treeState == 0 = Left EmptyTreeLookup
    | index < 0 || index > size treeState = Left (InvalidIndex index)
    | otherwise = Right (sum (take index (fenwickValues treeState)))

rangeSum :: Int -> Int -> FenwickTree -> Either FenwickError Double
rangeSum left right treeState
    | size treeState == 0 = Left EmptyTreeLookup
    | left <= 0 || right <= 0 || left > right || right > size treeState = Left (InvalidRange left right)
    | otherwise = Right (sum (take (right - left + 1) (drop (left - 1) (fenwickValues treeState))))

pointQuery :: Int -> FenwickTree -> Either FenwickError Double
pointQuery index treeState
    | size treeState == 0 = Left EmptyTreeLookup
    | index <= 0 || index > size treeState = Left (InvalidIndex index)
    | otherwise = Right (fenwickValues treeState !! (index - 1))

findKth :: Double -> FenwickTree -> Either FenwickError Int
findKth target treeState
    | size treeState == 0 = Left EmptyTreeLookup
    | target <= 0 = Left (InvalidIndex 0)
    | otherwise =
        case lookupIndex target (scanl1 (+) (fenwickValues treeState)) 1 of
            Nothing -> Left (InvalidIndex (size treeState + 1))
            Just index -> Right index
  where
    lookupIndex _ [] _ = Nothing
    lookupIndex goal (value : rest) position
        | value >= goal = Just position
        | otherwise = lookupIndex goal rest (position + 1)
