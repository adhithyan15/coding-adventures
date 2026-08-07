module GradientDescentSpec (spec) where

import Control.Monad (zipWithM_)
import Data.Either (isLeft)
import GradientDescent (sgd)
import Test.Hspec

tolerance :: Double
tolerance = 1e-12

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
    describe "sgd" $ do
        it "matches the shared ML02 parity vector" $ do
            result <- rightValue $ sgd [1.0, -0.5, 2.0] [0.1, -0.2, 0.0] 0.1
            result `shouldBeCloseList` [0.99, -0.48, 2.0]

        it "updates every element independently" $ do
            result <- rightValue $ sgd [4.0, 3.0, 2.0, 1.0] [1.0, 2.0, 3.0, 4.0] 0.25
            result `shouldBeCloseList` [3.75, 2.5, 1.25, 0.0]

        it "supports a singleton vector" $ do
            result <- rightValue $ sgd [2.0] [0.75] 0.4
            result `shouldBeCloseList` [1.7]

        it "leaves weights unchanged at a zero learning rate" $ do
            result <- rightValue $ sgd [1.0, -2.0, 3.5] [100.0, -50.0, 7.0] 0.0
            result `shouldBeCloseList` [1.0, -2.0, 3.5]

        it "moves in the gradient direction for a negative learning rate" $ do
            result <- rightValue $ sgd [1.0, -1.0] [0.5, -0.25] (-2.0)
            result `shouldBeCloseList` [2.0, -1.5]

        it "handles positive, negative, and zero gradients" $ do
            result <- rightValue $ sgd [0.0, 0.0, 0.0] [2.0, -3.0, 0.0] 0.5
            result `shouldBeCloseList` [-1.0, 1.5, 0.0]

        it "does not mutate the input values" $ do
            let weights = [3.0, 5.0]
                gradients = [1.0, 2.0]
            _ <- rightValue $ sgd weights gradients 0.5
            weights `shouldBe` [3.0, 5.0]
            gradients `shouldBe` [1.0, 2.0]

    describe "input validation" $ do
        it "rejects two empty vectors" $
            sgd [] [] 0.1 `shouldSatisfy` isLeft

        it "rejects a missing gradient" $
            sgd [1.0] [] 0.1
                `shouldBe` Left "Weights and gradients must have the same non-zero length"

        it "rejects extra gradients" $
            sgd [1.0] [0.1, 0.2] 0.1 `shouldSatisfy` isLeft

        it "uses the shared validation message" $ do
            sgd [] [] 0.1
                `shouldBe` Left "Weights and gradients must have the same non-zero length"
