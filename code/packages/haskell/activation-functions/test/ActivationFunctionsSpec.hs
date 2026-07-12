module ActivationFunctionsSpec (spec) where

import ActivationFunctions
import Prelude hiding (tanh)
import Test.Hspec

tolerance :: Double
tolerance = 1e-12

shouldBeCloseTo :: Double -> Double -> Expectation
actual `shouldBeCloseTo` expected =
    abs (actual - expected) `shouldSatisfy` (<= tolerance)

spec :: Spec
spec = do
    describe "linear" $ do
        it "is the identity" $ do
            linear (-3.0) `shouldBeCloseTo` (-3.0)
            linear 0.0 `shouldBeCloseTo` 0.0
            linear 5.0 `shouldBeCloseTo` 5.0
        it "has derivative one everywhere" $
            map linearDerivative [-3.0, 0.0, 5.0] `shouldBe` [1.0, 1.0, 1.0]

    describe "sigmoid" $ do
        it "matches reference values" $ do
            sigmoid 0.0 `shouldBeCloseTo` 0.5
            sigmoid 1.0 `shouldBeCloseTo` 0.7310585786300049
            sigmoid (-1.0) `shouldBeCloseTo` 0.2689414213699951
            sigmoid 10.0 `shouldBeCloseTo` 0.9999546021312976
        it "saturates without overflowing at extremes" $ do
            sigmoid (-710.0) `shouldBeCloseTo` 0.0
            sigmoid 710.0 `shouldBeCloseTo` 1.0
            sigmoid 1e9 `shouldBeCloseTo` 1.0
            sigmoid (-1e9) `shouldBeCloseTo` 0.0
        it "has the expected derivative" $ do
            sigmoidDerivative 0.0 `shouldBeCloseTo` 0.25
            sigmoidDerivative 1.0 `shouldBeCloseTo` 0.19661193324148185

    describe "relu" $ do
        it "is max zero x" $ do
            relu 5.0 `shouldBeCloseTo` 5.0
            relu (-3.0) `shouldBeCloseTo` 0.0
            relu 0.0 `shouldBeCloseTo` 0.0
        it "has derivative zero at and below the origin" $ do
            reluDerivative 5.0 `shouldBeCloseTo` 1.0
            reluDerivative (-3.0) `shouldBeCloseTo` 0.0
            reluDerivative 0.0 `shouldBeCloseTo` 0.0

    describe "leakyRelu" $ do
        it "keeps a small negative slope" $ do
            leakyRelu 5.0 `shouldBeCloseTo` 5.0
            leakyRelu (-3.0) `shouldBeCloseTo` (-0.03)
            leakyRelu 0.0 `shouldBeCloseTo` 0.0
        it "uses the leak slope at and below the origin" $ do
            leakyReluDerivative 5.0 `shouldBeCloseTo` 1.0
            leakyReluDerivative (-3.0) `shouldBeCloseTo` 0.01
            leakyReluDerivative 0.0 `shouldBeCloseTo` 0.01

    describe "tanh" $ do
        it "matches reference values" $ do
            tanh 0.0 `shouldBeCloseTo` 0.0
            tanh 1.0 `shouldBeCloseTo` 0.7615941559557649
            tanh (-1.0) `shouldBeCloseTo` (-0.7615941559557649)
        it "saturates at large magnitudes" $ do
            tanh 50.0 `shouldBeCloseTo` 1.0
            tanh (-50.0) `shouldBeCloseTo` (-1.0)
        it "has the expected derivative" $ do
            tanhDerivative 0.0 `shouldBeCloseTo` 1.0
            tanhDerivative 1.0 `shouldBeCloseTo` 0.41997434161402614
        it "agrees with the exponential definition on a sweep" $
            mapM_ checkTanh [-6.0, -5.5 .. 6.0]

    describe "softplus" $ do
        it "matches reference values" $ do
            softplus 0.0 `shouldBeCloseTo` 0.6931471805599453
            softplus 1.0 `shouldBeCloseTo` 1.3132616875182228
            softplus (-1.0) `shouldBeCloseTo` 0.31326168751822286
        it "does not overflow for large positive input" $ do
            softplus 1000.0 `shouldSatisfy` (> 999.0)
            isInfinite (softplus 1000.0) `shouldBe` False
        it "has derivative equal to sigmoid" $ do
            softplusDerivative 0.0 `shouldBeCloseTo` 0.5
            softplusDerivative 1.0 `shouldBeCloseTo` sigmoid 1.0
            softplusDerivative (-1.0) `shouldBeCloseTo` sigmoid (-1.0)

    describe "leakyReluSlope" $
        it "is 0.01" $
            leakyReluSlope `shouldBeCloseTo` 0.01
  where
    checkTanh x = do
        let ex = exp x
            enx = exp (-x)
            reference = (ex - enx) / (ex + enx)
        tanh x `shouldBeCloseTo` reference
