module TwoLayerNetworkSpec (spec) where

import Test.Hspec
import TwoLayerNetwork

xorInputs :: Matrix
xorInputs = [[0.0, 0.0], [0.0, 1.0], [1.0, 0.0], [1.0, 1.0]]

xorTargets :: Matrix
xorTargets = [[0.0], [1.0], [1.0], [0.0]]

spec :: Spec
spec = do
    it "forward pass exposes hidden activations" $ do
        let passed = forward xorInputs xorWarmStartParameters Sigmoid Sigmoid
        length (hiddenActivations passed) `shouldBe` 4
        length (head (hiddenActivations passed)) `shouldBe` 2
        head (predictions passed !! 1) > 0.7 `shouldBe` True
        head (head (predictions passed)) < 0.3 `shouldBe` True

    it "training step exposes both layer gradients" $ do
        let step = trainOneEpoch xorInputs xorTargets xorWarmStartParameters 0.5 Sigmoid Sigmoid
        length (inputToHiddenWeightGradients step) `shouldBe` 2
        length (head (inputToHiddenWeightGradients step)) `shouldBe` 2
        length (hiddenToOutputWeightGradients step) `shouldBe` 2
        length (head (hiddenToOutputWeightGradients step)) `shouldBe` 1
