module SegmentTree
    ( SegmentTree
    , fromList
    , update
    , rangeQuery
    , pointQuery
    , size
    , toList
    ) where

data SegmentTree a = SegmentTree [a]
    deriving (Eq, Show)

fromList :: [a] -> SegmentTree a
fromList = SegmentTree

update :: Int -> a -> SegmentTree a -> Maybe (SegmentTree a)
update index value (SegmentTree values)
    | index < 0 || index >= length values = Nothing
    | otherwise =
        Just
            ( SegmentTree
                [ if position == index then value else current
                | (position, current) <- zip [0 ..] values
                ]
            )

rangeQuery :: Monoid a => Int -> Int -> SegmentTree a -> Maybe a
rangeQuery left right (SegmentTree values)
    | left < 0 || right < left || right >= length values = Nothing
    | otherwise = Just (mconcat (take (right - left + 1) (drop left values)))

pointQuery :: Int -> SegmentTree a -> Maybe a
pointQuery index (SegmentTree values)
    | index < 0 || index >= length values = Nothing
    | otherwise = Just (values !! index)

size :: SegmentTree a -> Int
size (SegmentTree values) = length values

toList :: SegmentTree a -> [a]
toList (SegmentTree values) = values
