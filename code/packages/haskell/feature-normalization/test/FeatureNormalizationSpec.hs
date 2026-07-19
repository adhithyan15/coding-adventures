module FeatureNormalizationSpec (spec) where

import FeatureNormalization
import Test.Hspec

tolerance :: Double
tolerance = 1e-9

rows :: Matrix
rows =
    [ [1000.0, 3.0, 1.0]
    , [1500.0, 4.0, 0.0]
    , [2000.0, 5.0, 1.0]
    ]

shouldBeCloseTo :: Double -> Double -> Expectation
actual `shouldBeCloseTo` expected =
    abs (actual - expected) `shouldSatisfy` (<= tolerance)

shouldBeMatrixCloseTo :: Matrix -> Matrix -> Expectation
actual `shouldBeMatrixCloseTo` expected = do
    length actual `shouldBe` length expected
    zipWithM_ checkRow actual expected
  where
    checkRow actualRow expectedRow = do
        length actualRow `shouldBe` length expectedRow
        zipWithM_ shouldBeCloseTo actualRow expectedRow

spec :: Spec
spec = do
    describe "fitStandardScaler" $ do
        it "fits the shared column means" $
            means <$> fitStandardScaler rows `shouldBe` Right [1500.0, 4.0, 2.0 / 3.0]
        it "uses population standard deviation" $ do
            let Right scaler = fitStandardScaler rows
            standardDeviations scaler !! 0 `shouldBeCloseTo` sqrt (500000.0 / 3.0)
            standardDeviations scaler !! 1 `shouldBeCloseTo` sqrt (2.0 / 3.0)

    describe "transformStandard" $ do
        it "centers and scales the shared matrix" $ do
            let transformed = fitStandardScaler rows >>= transformStandard rows
            transformed `shouldSatisfy` either (const False) (const True)
            let Right actual = transformed
            actual `shouldBeMatrixCloseTo`
                [ [-1.224744871391589, -1.224744871391589, 0.7071067811865476]
                , [0.0, 0.0, -1.414213562373095]
                , [1.224744871391589, 1.224744871391589, 0.7071067811865476]
                ]
        it "applies a fitted scaler to new rows" $
            transformStandard [[2500.0, 6.0, 2.0]]
                (StandardScaler [1500.0, 4.0, 1.0] [500.0, 1.0, 2.0])
                `shouldBe` Right [[2.0, 2.0, 0.5]]
        it "maps constant columns to zero" $ do
            let constantRows = [[1.0, 7.0], [2.0, 7.0]]
            (fitStandardScaler constantRows >>= transformStandard constantRows)
                `shouldBe` Right [[-1.0, 0.0], [1.0, 0.0]]

    describe "fitMinMaxScaler" $
        it "fits every column's extrema" $
            fitMinMaxScaler rows
                `shouldBe` Right (MinMaxScaler [1000.0, 3.0, 0.0] [2000.0, 5.0, 1.0])

    describe "transformMinMax" $ do
        it "maps the shared matrix to the unit range" $ do
            let transformed = fitMinMaxScaler rows >>= transformMinMax rows
            transformed `shouldBe` Right
                [ [0.0, 0.0, 1.0]
                , [0.5, 0.5, 0.0]
                , [1.0, 1.0, 1.0]
                ]
        it "supports negative feature ranges" $
            transformMinMax [[-5.0], [0.0], [5.0]] (MinMaxScaler [-5.0] [5.0])
                `shouldBe` Right [[0.0], [0.5], [1.0]]
        it "maps constant columns to zero" $ do
            let constantRows = [[1.0, 7.0], [2.0, 7.0]]
            (fitMinMaxScaler constantRows >>= transformMinMax constantRows)
                `shouldBe` Right [[0.0, 0.0], [1.0, 0.0]]

    describe "matrix validation" $ do
        it "rejects an empty matrix" $ do
            fitStandardScaler [] `shouldBe` Left "matrix must have at least one row and one column"
            fitMinMaxScaler [] `shouldBe` Left "matrix must have at least one row and one column"
        it "rejects a zero-width matrix" $
            fitStandardScaler [[]] `shouldBe` Left "matrix must have at least one row and one column"
        it "rejects ragged matrices" $ do
            fitStandardScaler [[1.0], [1.0, 2.0]]
                `shouldBe` Left "all rows must have the same number of columns"
            transformMinMax [[1.0], [1.0, 2.0]] (MinMaxScaler [0.0] [1.0])
                `shouldBe` Left "all rows must have the same number of columns"
        it "rejects standard scaler width mismatches" $ do
            transformStandard [[1.0, 2.0]] (StandardScaler [0.0] [1.0])
                `shouldBe` Left "matrix width must match scaler width"
            transformStandard [[1.0]] (StandardScaler [0.0] [])
                `shouldBe` Left "matrix width must match scaler width"
        it "rejects min-max scaler width mismatches" $ do
            transformMinMax [[1.0, 2.0]] (MinMaxScaler [0.0] [1.0])
                `shouldBe` Left "matrix width must match scaler width"
            transformMinMax [[1.0]] (MinMaxScaler [0.0] [])
                `shouldBe` Left "matrix width must match scaler width"

zipWithM_ :: (a -> b -> Expectation) -> [a] -> [b] -> Expectation
zipWithM_ action left right = sequence_ (zipWith action left right)
