-- | Pure feature scaling for rectangular matrices.
module FeatureNormalization
    ( Matrix
    , StandardScaler (..)
    , MinMaxScaler (..)
    , fitStandardScaler
    , transformStandard
    , fitMinMaxScaler
    , transformMinMax
    ) where

import Data.List (transpose)

-- | Rows of observations containing feature columns.
type Matrix = [[Double]]

-- | Per-column parameters for standard scaling.
data StandardScaler = StandardScaler
    { means :: [Double]
    , standardDeviations :: [Double]
    }
    deriving (Eq, Show)

-- | Per-column parameters for min-max scaling.
data MinMaxScaler = MinMaxScaler
    { minimums :: [Double]
    , maximums :: [Double]
    }
    deriving (Eq, Show)

-- | Fit column means and population standard deviations.
fitStandardScaler :: Matrix -> Either String StandardScaler
fitStandardScaler rows = do
    _ <- validateMatrix rows
    let columns = transpose rows
        rowCount = fromIntegral (length rows)
        columnMeans = map ((/ rowCount) . sum) columns
        deviations =
            zipWith
                (\mean column -> sqrt (sum (map (squareDistance mean) column) / rowCount))
                columnMeans
                columns
    Right (StandardScaler columnMeans deviations)

-- | Apply a fitted standard scaler to a matrix.
transformStandard :: Matrix -> StandardScaler -> Either String Matrix
transformStandard rows scaler = do
    width <- validateMatrix rows
    validateScalerWidth width [length (means scaler), length (standardDeviations scaler)]
    Right (map transformRow rows)
  where
    transformRow row = zipWith3 scale row (means scaler) (standardDeviations scaler)
    scale value mean deviation
        | deviation == 0.0 = 0.0
        | otherwise = (value - mean) / deviation

-- | Fit per-column minima and maxima.
fitMinMaxScaler :: Matrix -> Either String MinMaxScaler
fitMinMaxScaler rows = do
    _ <- validateMatrix rows
    let columns = transpose rows
    Right (MinMaxScaler (map minimum columns) (map maximum columns))

-- | Apply a fitted min-max scaler to a matrix.
transformMinMax :: Matrix -> MinMaxScaler -> Either String Matrix
transformMinMax rows scaler = do
    width <- validateMatrix rows
    validateScalerWidth width [length (minimums scaler), length (maximums scaler)]
    Right (map transformRow rows)
  where
    transformRow row = zipWith3 scale row (minimums scaler) (maximums scaler)
    scale value minimumValue maximumValue
        | spanValue == 0.0 = 0.0
        | otherwise = (value - minimumValue) / spanValue
      where
        spanValue = maximumValue - minimumValue

validateMatrix :: Matrix -> Either String Int
validateMatrix [] = Left "matrix must have at least one row and one column"
validateMatrix rows@(firstRow : _)
    | null firstRow = Left "matrix must have at least one row and one column"
    | any ((/= width) . length) rows = Left "all rows must have the same number of columns"
    | otherwise = Right width
  where
    width = length firstRow

validateScalerWidth :: Int -> [Int] -> Either String ()
validateScalerWidth width widths
    | all (== width) widths = Right ()
    | otherwise = Left "matrix width must match scaler width"

squareDistance :: Double -> Double -> Double
squareDistance center value = difference * difference
  where
    difference = value - center
