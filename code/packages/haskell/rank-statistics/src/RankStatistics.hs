-- | Reusable medians and exact order-statistic confidence intervals.
module RankStatistics
    ( Interval(..)
    , median
    , medianInterval
    , exactMedianRank
    ) where

import Data.List (sort)

-- | Closed interval selected from the sorted observations.
data Interval = Interval
    { intervalLow :: Double
    , intervalHigh :: Double
    }
    deriving (Eq, Show)

finite :: Double -> Bool
finite value = not (isNaN value || isInfinite value)

orderedFinite :: [Double] -> Either String [Double]
orderedFinite [] = Left "sample must not be empty"
orderedFinite values
    | all finite values = Right (sort values)
    | otherwise = Left "sample values must be finite"

-- | Return the sample median without mutating the supplied list.  Rational
-- arithmetic avoids both overflow and double-rounding at subnormal midpoints.
median :: [Double] -> Either String Double
median values = do
    ordered <- orderedFinite values
    let count = length ordered
        middle = count `div` 2
    if odd count
        then Right (ordered !! middle)
        else
            let low = ordered !! (middle - 1)
                high = ordered !! middle
             in Right (fromRational ((toRational low + toRational high) / 2))

-- | Largest one-based rank k satisfying the exact two-sided binomial cutoff.
exactMedianRank :: Int -> Int -> Either String Int
exactMedianRank count confidence
    | count <= 0 = Left "sample must not be empty"
    | confidence < 1 || confidence > 99 = Left "confidence must be from 1 to 99"
    | otherwise = Right (go 0 1 0 0)
  where
    cutoff = toInteger (100 - confidence) * (2 ^ count)
    go j coefficient tailMass best
        | j > count `div` 2 = best
        | 200 * (tailMass + coefficient) > cutoff = best
        | otherwise =
            let nextCoefficient = coefficient * toInteger (count - j) `div` toInteger (j + 1)
             in go (j + 1) nextCoefficient (tailMass + coefficient) (j + 1)

-- | Exact two-sided distribution-free interval; 'Nothing' means no rank
-- satisfies the requested confidence for this sample size.
medianInterval :: [Double] -> Int -> Either String (Maybe Interval)
medianInterval values confidence = do
    ordered <- orderedFinite values
    rank <- exactMedianRank (length ordered) confidence
    if rank == 0
        then Right Nothing
        else Right (Just (Interval (ordered !! (rank - 1)) (ordered !! (length ordered - rank))))
