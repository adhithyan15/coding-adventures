module MatrixSpec (spec) where

import Matrix
import Prelude hiding (subtract)
import Test.Hspec

matrix2x2 :: Matrix
matrix2x2 = rightMatrix [[1.0, 2.0], [3.0, 4.0]]

other2x2 :: Matrix
other2x2 = rightMatrix [[5.0, 6.0], [7.0, 8.0]]

rightMatrix :: [[Double]] -> Matrix
rightMatrix values =
    case fromRows values of
        Right matrix -> matrix
        Left message -> error message

spec :: Spec
spec = do
    describe "construction" $ do
        it "constructs scalars, row vectors, and the empty matrix" $ do
            toRows (scalar 5.0) `shouldBe` [[5.0]]
            toRows (rowVector [1.0, 2.0, 3.0]) `shouldBe` [[1.0, 2.0, 3.0]]
            (rows empty, cols empty, toRows empty) `shouldBe` (0, 0, [])
        it "constructs rectangular matrices" $ do
            fromRows [[1.0, 2.0], [3.0, 4.0]] `shouldBe` Right matrix2x2
            (rows matrix2x2, cols matrix2x2) `shouldBe` (2, 2)
        it "rejects ragged rows" $
            fromRows [[1.0], [2.0, 3.0]]
                `shouldBe` Left "all rows must have the same number of columns"
        it "constructs zero matrices and rejects negative dimensions" $ do
            fmap toRows (zeros 2 3) `shouldBe` Right [[0.0, 0.0, 0.0], [0.0, 0.0, 0.0]]
            zeros (-1) 2 `shouldBe` Left "matrix dimensions must be non-negative"
            zeros 2 (-1) `shouldBe` Left "matrix dimensions must be non-negative"
        it "constructs identity matrices and rejects negative sizes" $ do
            fmap toRows (identity 3)
                `shouldBe` Right [[1.0, 0.0, 0.0], [0.0, 1.0, 0.0], [0.0, 0.0, 1.0]]
            identity (-1) `shouldBe` Left "matrix dimensions must be non-negative"
        it "constructs diagonal matrices" $ do
            toRows (fromDiagonal [2.0, 3.0]) `shouldBe` [[2.0, 0.0], [0.0, 3.0]]
            fromDiagonal [] `shouldBe` empty

    describe "arithmetic" $ do
        it "adds and subtracts element by element" $ do
            fmap toRows (add matrix2x2 other2x2) `shouldBe` Right [[6.0, 8.0], [10.0, 12.0]]
            fmap toRows (subtract other2x2 matrix2x2) `shouldBe` Right [[4.0, 4.0], [4.0, 4.0]]
        it "rejects mismatched element-wise shapes" $ do
            let one = scalar 1.0
            add matrix2x2 one `shouldBe` Left "addition dimension mismatch: 2x2 vs 1x1"
            subtract matrix2x2 one `shouldBe` Left "subtraction dimension mismatch: 2x2 vs 1x1"
        it "broadcasts and scales scalars" $ do
            toRows (addScalar matrix2x2 2.0) `shouldBe` [[3.0, 4.0], [5.0, 6.0]]
            toRows (subtractScalar matrix2x2 1.0) `shouldBe` [[0.0, 1.0], [2.0, 3.0]]
            toRows (scale matrix2x2 2.0) `shouldBe` [[2.0, 4.0], [6.0, 8.0]]
        it "transposes regular and zero-row matrices" $ do
            toRows (transpose (rightMatrix [[1.0, 2.0, 3.0], [4.0, 5.0, 6.0]]))
                `shouldBe` [[1.0, 4.0], [2.0, 5.0], [3.0, 6.0]]
            fmap (\matrix -> (rows matrix, cols matrix, toRows matrix)) (transpose <$> zeros 0 3)
                `shouldBe` Right (3, 0, [[], [], []])
        it "multiplies square matrices" $
            fmap toRows (dot matrix2x2 other2x2)
                `shouldBe` Right [[19.0, 22.0], [43.0, 50.0]]
        it "multiplies vectors and rejects mismatched shapes" $ do
            let columnVector = rightMatrix [[4.0], [5.0], [6.0]]
            fmap toRows (dot (rowVector [1.0, 2.0, 3.0]) columnVector) `shouldBe` Right [[32.0]]
            dot matrix2x2 columnVector
                `shouldBe` Left "dot product dimension mismatch: 2 columns vs 3 rows"
        it "leaves a matrix unchanged when multiplied by identity" $ do
            let tall = rightMatrix [[1.0, 2.0], [3.0, 4.0], [5.0, 6.0]]
            (identity 3 >>= (`dot` tall)) `shouldBe` Right tall

    describe "element access" $ do
        it "reads each corner" $ do
            get 0 0 matrix2x2 `shouldBe` Right 1.0
            get 1 1 matrix2x2 `shouldBe` Right 4.0
        it "rejects out-of-bounds reads" $ do
            get (-1) 0 matrix2x2 `shouldBe` Left "index (-1, 0) out of bounds for 2x2 matrix"
            get 0 2 matrix2x2 `shouldBe` Left "index (0, 2) out of bounds for 2x2 matrix"
        it "updates immutably" $ do
            let updated = set 0 0 99.0 matrix2x2
            get 0 0 matrix2x2 `shouldBe` Right 1.0
            (updated >>= get 0 0) `shouldBe` Right 99.0
            fmap toRows updated `shouldBe` Right [[99.0, 2.0], [3.0, 4.0]]
        it "rejects out-of-bounds updates" $
            set 2 0 5.0 matrix2x2
                `shouldBe` Left "index (2, 0) out of bounds for 2x2 matrix"

    describe "reductions" $ do
        it "sums all elements, rows, and columns" $ do
            sumElements matrix2x2 `shouldBe` 10.0
            toRows (sumRows matrix2x2) `shouldBe` [[3.0], [7.0]]
            toRows (sumColumns matrix2x2) `shouldBe` [[4.0, 6.0]]
        it "computes a mean and rejects an empty mean" $ do
            mean matrix2x2 `shouldBe` Right 2.5
            mean empty `shouldBe` Left "cannot compute mean of an empty matrix"
        it "finds extrema across negative values" $ do
            let values = rightMatrix [[-5.0, 3.0], [0.0, -1.0]]
            minimumElement values `shouldBe` Right (-5.0)
            maximumElement values `shouldBe` Right 3.0
        it "finds first extrema positions on ties" $ do
            argmin (rightMatrix [[3.0, 1.0], [4.0, 2.0]]) `shouldBe` Right (0, 1)
            argmax (rightMatrix [[4.0, 2.0], [3.0, 4.0]]) `shouldBe` Right (0, 0)
        it "rejects extrema on an empty matrix" $ do
            minimumElement empty `shouldBe` Left "cannot compute minimum of an empty matrix"
            maximumElement empty `shouldBe` Left "cannot compute maximum of an empty matrix"
            argmin empty `shouldBe` Left "cannot compute argmin of an empty matrix"
            argmax empty `shouldBe` Left "cannot compute argmax of an empty matrix"

    describe "element-wise math" $ do
        it "maps functions and square roots" $ do
            let squares = rightMatrix [[1.0, 4.0], [9.0, 16.0]]
            toRows (mapElements sqrt squares) `shouldBe` [[1.0, 2.0], [3.0, 4.0]]
            fmap toRows (squareRoot squares) `shouldBe` Right [[1.0, 2.0], [3.0, 4.0]]
        it "rejects square roots of negative elements" $
            squareRoot (rowVector [1.0, -1.0])
                `shouldBe` Left "cannot take square root of a negative matrix element"
        it "computes absolute values and powers" $ do
            toRows (absolute (rightMatrix [[-1.0, 2.0], [-3.0, 4.0]]))
                `shouldBe` [[1.0, 2.0], [3.0, 4.0]]
            toRows (power matrix2x2 2.0) `shouldBe` [[1.0, 4.0], [9.0, 16.0]]
            (squareRoot matrix2x2 >>= Right . (`power` 2.0))
                `shouldSatisfy` either (const False) (`close` matrix2x2)

    describe "shape operations" $ do
        it "flattens in row-major order and reshapes round-trip" $ do
            toRows (flatten matrix2x2) `shouldBe` [[1.0, 2.0, 3.0, 4.0]]
            reshape 2 2 (flatten matrix2x2) `shouldBe` Right matrix2x2
        it "reshapes row vectors and zero-width matrices" $ do
            fmap toRows (reshape 2 3 (rowVector [1.0, 2.0, 3.0, 4.0, 5.0, 6.0]))
                `shouldBe` Right [[1.0, 2.0, 3.0], [4.0, 5.0, 6.0]]
            fmap (\matrix -> (rows matrix, cols matrix, toRows matrix)) (reshape 2 0 empty)
                `shouldBe` Right (2, 0, [[], []])
        it "rejects invalid reshapes" $ do
            reshape 3 3 matrix2x2
                `shouldBe` Left "cannot reshape 2x2 matrix into 3x3"
            reshape (-1) 4 matrix2x2
                `shouldBe` Left "matrix dimensions must be non-negative"
        it "extracts rows and columns" $ do
            fmap toRows (row 1 matrix2x2) `shouldBe` Right [[3.0, 4.0]]
            fmap toRows (column 0 matrix2x2) `shouldBe` Right [[1.0], [3.0]]
        it "rejects invalid row and column indices" $ do
            row 2 matrix2x2 `shouldBe` Left "row 2 out of bounds for 2-row matrix"
            column (-1) matrix2x2 `shouldBe` Left "column -1 out of bounds for 2-column matrix"
        it "extracts half-open slices" $
            fmap toRows (slice 0 2 1 3 (rightMatrix [[1.0, 2.0, 3.0], [4.0, 5.0, 6.0], [7.0, 8.0, 9.0]]))
                `shouldBe` Right [[2.0, 3.0], [5.0, 6.0]]
        it "rejects out-of-bounds and empty slices" $ do
            slice 0 3 0 1 matrix2x2 `shouldBe` Left "slice bounds are outside the matrix"
            slice 1 1 0 1 matrix2x2 `shouldBe` Left "slice dimensions must be positive"

    describe "comparison" $ do
        it "compares exact values and shapes" $ do
            exactEquals matrix2x2 (rightMatrix [[1.0, 2.0], [3.0, 4.0]]) `shouldBe` True
            exactEquals matrix2x2 (rightMatrix [[1.0, 2.0], [3.0, 5.0]]) `shouldBe` False
            exactEquals (rowVector [1.0, 2.0]) (rightMatrix [[1.0], [2.0]]) `shouldBe` False
        it "compares floating-point values with tolerances" $ do
            let near = rightMatrix [[1.0 + 1e-10, 2.0 - 1e-10], [3.0, 4.0]]
            close matrix2x2 near `shouldBe` True
            closeWithin 1e-12 matrix2x2 near `shouldBe` False
            close matrix2x2 (scalar 1.0) `shouldBe` False
