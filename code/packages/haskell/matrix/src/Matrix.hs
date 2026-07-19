-- | Immutable rectangular matrices from first principles.
module Matrix
    ( Matrix
    , fromRows
    , scalar
    , rowVector
    , empty
    , toRows
    , rows
    , cols
    , zeros
    , identity
    , fromDiagonal
    , add
    , addScalar
    , subtract
    , subtractScalar
    , scale
    , transpose
    , dot
    , get
    , set
    , sumElements
    , sumRows
    , sumColumns
    , mean
    , minimumElement
    , maximumElement
    , argmin
    , argmax
    , mapElements
    , squareRoot
    , absolute
    , power
    , flatten
    , reshape
    , row
    , column
    , slice
    , exactEquals
    , close
    , closeWithin
    ) where

import Prelude hiding (subtract)

-- | A rectangular matrix stored in row-major order.
data Matrix = Matrix
    { matrixValues :: [[Double]]
    , matrixRowCount :: Int
    , matrixColumnCount :: Int
    }
    deriving (Eq, Show)

-- | Construct a matrix from rows, rejecting ragged input.
fromRows :: [[Double]] -> Either String Matrix
fromRows [] = Right empty
fromRows values@(firstRow : remainingRows)
    | any ((/= width) . length) remainingRows =
        Left "all rows must have the same number of columns"
    | otherwise = Right (Matrix values (length values) width)
  where
    width = length firstRow

-- | Construct a one-by-one matrix.
scalar :: Double -> Matrix
scalar value = Matrix [[value]] 1 1

-- | Construct a one-row matrix.
rowVector :: [Double] -> Matrix
rowVector values = Matrix [values] 1 (length values)

-- | The empty zero-by-zero matrix.
empty :: Matrix
empty = Matrix [] 0 0

-- | Return the matrix's row-major values.
toRows :: Matrix -> [[Double]]
toRows = matrixValues

-- | Number of rows.
rows :: Matrix -> Int
rows = matrixRowCount

-- | Number of columns.
cols :: Matrix -> Int
cols = matrixColumnCount

-- | Construct a matrix filled with zeros.
zeros :: Int -> Int -> Either String Matrix
zeros rowCount columnCount = do
    validateDimensions rowCount columnCount
    Right (Matrix (replicate rowCount (replicate columnCount 0.0)) rowCount columnCount)

-- | Construct a square identity matrix.
identity :: Int -> Either String Matrix
identity size
    | size < 0 = Left "matrix dimensions must be non-negative"
    | otherwise =
        Right
            ( Matrix
                [ [if rowIndex == columnIndex then 1.0 else 0.0 | columnIndex <- indices size]
                | rowIndex <- indices size
                ]
                size
                size
            )

-- | Construct a square matrix whose only non-zero values are diagonal.
fromDiagonal :: [Double] -> Matrix
fromDiagonal values =
    Matrix
        [ [if rowIndex == columnIndex then values !! rowIndex else 0.0 | columnIndex <- indices size]
        | rowIndex <- indices size
        ]
        size
        size
  where
    size = length values

-- | Add matrices element by element.
add :: Matrix -> Matrix -> Either String Matrix
add = combine "addition" (+)

-- | Add a scalar to every element.
addScalar :: Matrix -> Double -> Matrix
addScalar matrix value = mapElements (+ value) matrix

-- | Subtract matrices element by element.
subtract :: Matrix -> Matrix -> Either String Matrix
subtract = combine "subtraction" (-)

-- | Subtract a scalar from every element.
subtractScalar :: Matrix -> Double -> Matrix
subtractScalar matrix value = mapElements (\element -> element - value) matrix

-- | Multiply every element by a scalar.
scale :: Matrix -> Double -> Matrix
scale matrix value = mapElements (* value) matrix

-- | Swap rows and columns.
transpose :: Matrix -> Matrix
transpose (Matrix values rowCount columnCount) =
    Matrix
        [ [values !! rowIndex !! columnIndex | rowIndex <- indices rowCount]
        | columnIndex <- indices columnCount
        ]
        columnCount
        rowCount

-- | Multiply two matrices.
dot :: Matrix -> Matrix -> Either String Matrix
dot left right
    | cols left /= rows right =
        Left
            ( "dot product dimension mismatch: "
                ++ show (cols left)
                ++ " columns vs "
                ++ show (rows right)
                ++ " rows"
            )
    | otherwise =
        Right
            ( Matrix
                [ [ cell rowIndex columnIndex
                  | columnIndex <- indices (cols right)
                  ]
                | rowIndex <- indices (rows left)
                ]
                (rows left)
                (cols right)
            )
  where
    cell rowIndex columnIndex =
        sum
            [ matrixValues left !! rowIndex !! innerIndex
                * matrixValues right !! innerIndex !! columnIndex
            | innerIndex <- indices (cols left)
            ]

-- | Read an element at a zero-based row and column.
get :: Int -> Int -> Matrix -> Either String Double
get rowIndex columnIndex matrix
    | not (validIndex rowIndex columnIndex matrix) = Left (indexError rowIndex columnIndex matrix)
    | otherwise = Right (matrixValues matrix !! rowIndex !! columnIndex)

-- | Return a new matrix with one element replaced.
set :: Int -> Int -> Double -> Matrix -> Either String Matrix
set rowIndex columnIndex value matrix
    | not (validIndex rowIndex columnIndex matrix) = Left (indexError rowIndex columnIndex matrix)
    | otherwise =
        Right
            matrix
                { matrixValues =
                    replaceAt
                        rowIndex
                        (replaceAt columnIndex value (matrixValues matrix !! rowIndex))
                        (matrixValues matrix)
                }

-- | Sum every element.
sumElements :: Matrix -> Double
sumElements = sum . map sum . matrixValues

-- | Sum each row into a column vector.
sumRows :: Matrix -> Matrix
sumRows matrix = Matrix (map (\values -> [sum values]) (matrixValues matrix)) (rows matrix) 1

-- | Sum each column into a row vector.
sumColumns :: Matrix -> Matrix
sumColumns matrix =
    Matrix
        [ [ sum [matrixValues matrix !! rowIndex !! columnIndex | rowIndex <- indices (rows matrix)]
          | columnIndex <- indices (cols matrix)
          ]
        ]
        1
        (cols matrix)

-- | Arithmetic mean of every element.
mean :: Matrix -> Either String Double
mean matrix
    | elementCount == 0 = Left "cannot compute mean of an empty matrix"
    | otherwise = Right (sumElements matrix / fromIntegral elementCount)
  where
    elementCount = rows matrix * cols matrix

-- | Smallest element.
minimumElement :: Matrix -> Either String Double
minimumElement matrix = do
    (_, value) <- extreme (<) "minimum" matrix
    Right value

-- | Largest element.
maximumElement :: Matrix -> Either String Double
maximumElement matrix = do
    (_, value) <- extreme (>) "maximum" matrix
    Right value

-- | Position of the first smallest element in row-major order.
argmin :: Matrix -> Either String (Int, Int)
argmin matrix = fst <$> extreme (<) "argmin" matrix

-- | Position of the first largest element in row-major order.
argmax :: Matrix -> Either String (Int, Int)
argmax matrix = fst <$> extreme (>) "argmax" matrix

-- | Apply a function to every element.
mapElements :: (Double -> Double) -> Matrix -> Matrix
mapElements transform matrix = matrix {matrixValues = map (map transform) (matrixValues matrix)}

-- | Take the square root of every element, rejecting negative inputs.
squareRoot :: Matrix -> Either String Matrix
squareRoot matrix
    | any (< 0.0) (concat (matrixValues matrix)) = Left "cannot take square root of a negative matrix element"
    | otherwise = Right (mapElements sqrt matrix)

-- | Take the absolute value of every element.
absolute :: Matrix -> Matrix
absolute = mapElements abs

-- | Raise every element to a power.
power :: Matrix -> Double -> Matrix
power matrix exponent = mapElements (** exponent) matrix

-- | Flatten a matrix into one row in row-major order.
flatten :: Matrix -> Matrix
flatten matrix = Matrix [concat (matrixValues matrix)] 1 (rows matrix * cols matrix)

-- | Rearrange elements into a new rectangular shape.
reshape :: Int -> Int -> Matrix -> Either String Matrix
reshape rowCount columnCount matrix = do
    validateDimensions rowCount columnCount
    if rowCount * columnCount /= elementCount
        then
            Left
                ( "cannot reshape "
                    ++ show (rows matrix)
                    ++ "x"
                    ++ show (cols matrix)
                    ++ " matrix into "
                    ++ show rowCount
                    ++ "x"
                    ++ show columnCount
                )
        else Right (Matrix reshaped rowCount columnCount)
  where
    elementCount = rows matrix * cols matrix
    flattened = concat (matrixValues matrix)
    reshaped
        | columnCount == 0 = replicate rowCount []
        | otherwise = chunksOf columnCount flattened

-- | Extract one row as a one-row matrix.
row :: Int -> Matrix -> Either String Matrix
row rowIndex matrix
    | rowIndex < 0 || rowIndex >= rows matrix =
        Left ("row " ++ show rowIndex ++ " out of bounds for " ++ show (rows matrix) ++ "-row matrix")
    | otherwise = Right (Matrix [matrixValues matrix !! rowIndex] 1 (cols matrix))

-- | Extract one column as a column vector.
column :: Int -> Matrix -> Either String Matrix
column columnIndex matrix
    | columnIndex < 0 || columnIndex >= cols matrix =
        Left ("column " ++ show columnIndex ++ " out of bounds for " ++ show (cols matrix) ++ "-column matrix")
    | otherwise =
        Right
            ( Matrix
                [[values !! columnIndex] | values <- matrixValues matrix]
                (rows matrix)
                1
            )

-- | Extract rows @[r0,r1)@ and columns @[c0,c1)@.
slice :: Int -> Int -> Int -> Int -> Matrix -> Either String Matrix
slice r0 r1 c0 c1 matrix
    | r0 < 0 || r1 > rows matrix || c0 < 0 || c1 > cols matrix =
        Left "slice bounds are outside the matrix"
    | r0 >= r1 || c0 >= c1 =
        Left "slice dimensions must be positive"
    | otherwise =
        Right
            ( Matrix
                (map (take (c1 - c0) . drop c0) (take (r1 - r0) (drop r0 (matrixValues matrix))))
                (r1 - r0)
                (c1 - c0)
            )

-- | Exact shape and element equality.
exactEquals :: Matrix -> Matrix -> Bool
exactEquals = (==)

-- | Element-wise comparison with the default tolerance of @1e-9@.
close :: Matrix -> Matrix -> Bool
close = closeWithin 1e-9

-- | Element-wise comparison with an explicit absolute tolerance.
closeWithin :: Double -> Matrix -> Matrix -> Bool
closeWithin tolerance left right =
    rows left == rows right
        && cols left == cols right
        && and
            ( zipWith
                (\leftRow rightRow -> and (zipWith (\a b -> abs (a - b) <= tolerance) leftRow rightRow))
                (matrixValues left)
                (matrixValues right)
            )

combine :: String -> (Double -> Double -> Double) -> Matrix -> Matrix -> Either String Matrix
combine operation transform left right
    | rows left /= rows right || cols left /= cols right =
        Left
            ( operation
                ++ " dimension mismatch: "
                ++ show (rows left)
                ++ "x"
                ++ show (cols left)
                ++ " vs "
                ++ show (rows right)
                ++ "x"
                ++ show (cols right)
            )
    | otherwise =
        Right
            left
                { matrixValues =
                    zipWith (zipWith transform) (matrixValues left) (matrixValues right)
                }

extreme :: (Double -> Double -> Bool) -> String -> Matrix -> Either String ((Int, Int), Double)
extreme better operation matrix =
    case indexedValues of
        [] -> Left ("cannot compute " ++ operation ++ " of an empty matrix")
        first : remaining -> Right (foldl choose first remaining)
  where
    indexedValues =
        [ ((rowIndex, columnIndex), matrixValues matrix !! rowIndex !! columnIndex)
        | rowIndex <- indices (rows matrix)
        , columnIndex <- indices (cols matrix)
        ]
    choose best@(_, bestValue) candidate@(_, candidateValue)
        | candidateValue `better` bestValue = candidate
        | otherwise = best

validateDimensions :: Int -> Int -> Either String ()
validateDimensions rowCount columnCount
    | rowCount < 0 || columnCount < 0 = Left "matrix dimensions must be non-negative"
    | otherwise = Right ()

validIndex :: Int -> Int -> Matrix -> Bool
validIndex rowIndex columnIndex matrix =
    rowIndex >= 0
        && rowIndex < rows matrix
        && columnIndex >= 0
        && columnIndex < cols matrix

indexError :: Int -> Int -> Matrix -> String
indexError rowIndex columnIndex matrix =
    "index ("
        ++ show rowIndex
        ++ ", "
        ++ show columnIndex
        ++ ") out of bounds for "
        ++ show (rows matrix)
        ++ "x"
        ++ show (cols matrix)
        ++ " matrix"

replaceAt :: Int -> value -> [value] -> [value]
replaceAt index value values = take index values ++ [value] ++ drop (index + 1) values

chunksOf :: Int -> [value] -> [[value]]
chunksOf _ [] = []
chunksOf size values = take size values : chunksOf size (drop size values)

indices :: Int -> [Int]
indices size = [0 .. size - 1]
