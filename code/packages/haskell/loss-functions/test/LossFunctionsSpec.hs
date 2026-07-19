module LossFunctionsSpec (spec) where

import Control.Monad (zipWithM_)
import Data.Either (isLeft)
import LossFunctions
import Test.Hspec

tolerance :: Double
tolerance = 1e-6

shouldBeCloseTo :: Double -> Double -> Expectation
actual `shouldBeCloseTo` expected =
    abs (actual - expected) `shouldSatisfy` (<= tolerance)

shouldBeCloseList :: [Double] -> [Double] -> Expectation
actual `shouldBeCloseList` expected = do
    length actual `shouldBe` length expected
    zipWithM_ shouldBeCloseTo actual expected

rightValue :: Show error => Either error value -> IO value
rightValue result = case result of
    Left err -> expectationFailure (show err) >> error "unreachable"
    Right value -> pure value

spec :: Spec
spec = do
    describe "mse" $ do
        it "matches the shared reference vector" $ do
            result <- rightValue $ mse [1.0, 0.0] [0.9, 0.1]
            result `shouldBeCloseTo` 0.01
        it "is zero for identical vectors" $ do
            result <- rightValue $ mse [1.0, 0.0, 0.5] [1.0, 0.0, 0.5]
            result `shouldBeCloseTo` 0.0

    describe "mae" $ do
        it "matches the shared reference vector" $ do
            result <- rightValue $ mae [1.0, 0.0] [0.9, 0.1]
            result `shouldBeCloseTo` 0.1
        it "is zero for identical vectors" $ do
            result <- rightValue $ mae [1.0, 0.0, 0.5] [1.0, 0.0, 0.5]
            result `shouldBeCloseTo` 0.0

    describe "bce" $ do
        it "matches the shared reference vector" $ do
            result <- rightValue $ bce [1.0, 0.0] [0.9, 0.1]
            result `shouldBeCloseTo` 0.1053605
        it "clamps zero and one predictions" $ do
            result <- rightValue $ bce [1.0, 0.0] [0.0, 1.0]
            result `shouldBeCloseTo` (negate (log epsilon))

    describe "cce" $ do
        it "matches the shared reference vector" $ do
            result <- rightValue $ cce [1.0, 0.0] [0.9, 0.1]
            result `shouldBeCloseTo` 0.0526802
        it "clamps a zero prediction" $ do
            result <- rightValue $ cce [1.0, 0.0] [0.0, 1.0]
            result `shouldBeCloseTo` (negate (log epsilon) / 2.0)

    describe "derivatives" $ do
        it "computes the MSE gradient" $ do
            result <- rightValue $ mseDerivative [1.0, 0.0] [0.8, 0.2]
            result `shouldBeCloseList` [-0.2, 0.2]
        it "computes all three MAE gradient branches" $ do
            result <- rightValue $ maeDerivative [1.0, 0.0, 0.5] [0.8, 0.2, 0.5]
            result `shouldBeCloseList` [-1.0 / 3.0, 1.0 / 3.0, 0.0]
        it "computes the BCE gradient" $ do
            result <- rightValue $ bceDerivative [1.0, 0.0] [0.8, 0.2]
            result `shouldBeCloseList` [-0.625, 0.625]
        it "computes the CCE gradient" $ do
            result <- rightValue $ cceDerivative [1.0, 0.0] [0.8, 0.2]
            result `shouldBeCloseList` [-0.625, 0.0]
        it "clamps derivative probabilities" $ do
            binary <- rightValue $ bceDerivative [1.0, 0.0] [0.0, 1.0]
            categorical <- rightValue $ cceDerivative [1.0, 0.0] [0.0, 1.0]
            all (not . isInfinite) binary `shouldBe` True
            categorical `shouldBeCloseList` [(-0.5) / epsilon, 0.0]

    describe "input validation" $ do
        it "rejects empty vectors for every loss and derivative" $ do
            all isLeft [mse [] [], mae [] [], bce [] [], cce [] []] `shouldBe` True
            all isLeft
                [ mseDerivative [] []
                , maeDerivative [] []
                , bceDerivative [] []
                , cceDerivative [] []
                ] `shouldBe` True
        it "rejects length mismatches for every loss and derivative" $ do
            all isLeft
                [ mse [1.0] [0.9, 0.1]
                , mae [1.0] [0.9, 0.1]
                , bce [1.0] [0.9, 0.1]
                , cce [1.0] [0.9, 0.1]
                ] `shouldBe` True
            all isLeft
                [ mseDerivative [1.0] [0.9, 0.1]
                , maeDerivative [1.0] [0.9, 0.1]
                , bceDerivative [1.0] [0.9, 0.1]
                , cceDerivative [1.0] [0.9, 0.1]
                ] `shouldBe` True
