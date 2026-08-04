module PerceptronSpec (spec) where

import Control.Monad (zipWithM_)
import Data.Either (isLeft)
import Perceptron
import Test.Hspec hiding (fit)

andFeatures :: [[Double]]
andFeatures =
    [ [0.0, 0.0]
    , [0.0, 1.0]
    , [1.0, 0.0]
    , [1.0, 1.0]
    ]

andLabels :: [Double]
andLabels = [0.0, 0.0, 0.0, 1.0]

rightValue :: Show error => Either error value -> IO value
rightValue result = case result of
    Left err -> expectationFailure (show err) >> error "unreachable"
    Right value -> pure value

shouldBeCloseTo :: Double -> Double -> Expectation
actual `shouldBeCloseTo` expected =
    abs (actual - expected) `shouldSatisfy` (<= 1e-12)

shouldBeCloseList :: [Double] -> [Double] -> Expectation
actual `shouldBeCloseList` expected = do
    length actual `shouldBe` length expected
    zipWithM_ shouldBeCloseTo actual expected

spec :: Spec
spec = do
    describe "construction" $ do
        it "provides the shared default hyperparameters" $ do
            learningRate defaultPerceptron `shouldBe` 0.1
            epochs defaultPerceptron `shouldBe` 2000
            weights defaultPerceptron `shouldBe` Nothing
            bias defaultPerceptron `shouldBe` 0.0

        it "accepts finite custom hyperparameters" $ do
            model <- rightValue $ new 0.8 5000
            learningRate model `shouldBe` 0.8
            epochs model `shouldBe` 5000

        it "rejects non-finite learning rates" $ do
            new (0.0 / 0.0) 10 `shouldSatisfy` isLeft
            new (1.0 / 0.0) 10 `shouldSatisfy` isLeft

        it "rejects negative epoch counts" $
            new 0.1 (-1) `shouldBe` Left "Epochs must be non-negative"

    describe "training and prediction" $ do
        it "learns the shared AND-gate example" $ do
            model <- rightValue $ new 0.8 5000
            trained <- rightValue $ fit model andFeatures andLabels
            predictions <- rightValue $ predict trained andFeatures
            take 3 predictions `shouldSatisfy` all (< 0.2)
            last predictions `shouldSatisfy` (> 0.7)
            fmap length (weights trained) `shouldBe` Just 2

        it "accepts one-column labels" $ do
            model <- rightValue $ new 0.8 5000
            trained <-
                rightValue $
                    fitColumnLabels model andFeatures (map (: []) andLabels)
            predictions <- rightValue $ predict trained [[1.0, 1.0]]
            head predictions `shouldSatisfy` (> 0.7)

        it "performs one update when epochs is zero" $ do
            model <- rightValue $ new 0.1 0
            trained <- rightValue $ fit model [[1.0]] [1.0]
            weights trained `shouldBe` Just [0.05]
            bias trained `shouldBeCloseTo` 0.05
            predictions <- rightValue $ predict trained [[1.0]]
            predictions `shouldBeCloseList` [0.52497918747894]

        it "starts every fit from zero parameters" $ do
            model <- rightValue $ new 0.2 4
            first <- rightValue $ fit model andFeatures andLabels
            second <- rightValue $ fit first andFeatures andLabels
            first `shouldBe` second

        it "requires a trained model for prediction" $
            predict defaultPerceptron [[0.0, 1.0]]
                `shouldBe` Left "Perceptron has not been trained yet. Call fit first"

        it "rejects prediction widths that differ from training" $ do
            trained <- rightValue $ fit defaultPerceptron andFeatures andLabels
            predict trained [[1.0]]
                `shouldBe` Left "Feature width 1 does not match trained width 2"

    describe "input validation" $ do
        it "rejects empty, zero-width, and ragged feature data" $ do
            fit defaultPerceptron [] [] `shouldSatisfy` isLeft
            fit defaultPerceptron [[]] [0.0] `shouldSatisfy` isLeft
            fit defaultPerceptron [[0.0], [1.0, 2.0]] [0.0, 1.0]
                `shouldSatisfy` isLeft

        it "rejects label counts that do not match the samples" $
            fit defaultPerceptron [[0.0]] [0.0, 1.0]
                `shouldBe` Left "Labels must match the non-zero sample count"

        it "rejects multi-column label rows" $
            fitColumnLabels defaultPerceptron [[0.0]] [[0.0, 1.0]]
                `shouldBe` Left "Column labels must have exactly one value per row"

        it "rejects non-finite features and labels" $ do
            fit defaultPerceptron [[0.0 / 0.0]] [0.0] `shouldSatisfy` isLeft
            fit defaultPerceptron [[0.0]] [1.0 / 0.0] `shouldSatisfy` isLeft
