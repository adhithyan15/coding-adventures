module TwoLayerNetworkSpec (spec) where

import Test.Hspec
import TwoLayerNetwork

xorInputs :: Matrix
xorInputs = [[0.0, 0.0], [0.0, 1.0], [1.0, 0.0], [1.0, 1.0]]

xorTargets :: Matrix
xorTargets = [[0.0], [1.0], [1.0], [0.0]]

sampleParameters :: Int -> Int -> Parameters
sampleParameters inputCount hiddenCount =
    Parameters
        { inputToHiddenWeights =
            [ [0.17 * fromIntegral (feature + 1) - 0.11 * fromIntegral (hidden + 1) | hidden <- [0 .. hiddenCount - 1]]
            | feature <- [0 .. inputCount - 1]
            ]
        , hiddenBiases = [0.05 * fromIntegral (hidden - 1) | hidden <- [0 .. hiddenCount - 1]]
        , hiddenToOutputWeights = [[0.13 * fromIntegral (hidden + 1) - 0.25] | hidden <- [0 .. hiddenCount - 1]]
        , outputBiases = [0.02]
        }

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

    it "runs hidden-layer teaching examples through one training step" $ do
        let cases =
                [ ("XNOR", xorInputs, [[1.0], [0.0], [0.0], [1.0]], 3)
                , ("absolute value", [[-1.0], [-0.5], [0.0], [0.5], [1.0]], [[1.0], [0.5], [0.0], [0.5], [1.0]], 4)
                , ("piecewise pricing", [[0.1], [0.3], [0.5], [0.7], [0.9]], [[0.12], [0.25], [0.55], [0.88], [0.88]], 4)
                , ("circle classifier", [[0.0, 0.0], [0.5, 0.0], [1.0, 1.0], [-0.5, 0.5], [-1.0, 0.0]], [[1.0], [1.0], [0.0], [1.0], [0.0]], 5)
                , ("two moons", [[1.0, 0.0], [0.0, 0.5], [0.5, 0.85], [0.5, -0.35], [-1.0, 0.0], [2.0, 0.5]], [[0.0], [1.0], [0.0], [1.0], [0.0], [1.0]], 5)
                , ("interaction features", [[0.2, 0.25, 0.0], [0.6, 0.5, 1.0], [1.0, 0.75, 1.0], [1.0, 1.0, 0.0]], [[0.08], [0.72], [0.96], [0.76]], 5)
                ]
        mapM_
            ( \(name, exampleInputs, exampleTargets, hiddenCount) -> do
                let step = trainOneEpoch exampleInputs exampleTargets (sampleParameters (length (head exampleInputs)) hiddenCount) 0.4 Sigmoid Sigmoid
                loss step >= 0.0 `shouldBe` True
                length (inputToHiddenWeightGradients step) `shouldBe` length (head exampleInputs)
                length (hiddenToOutputWeightGradients step) `shouldBe` hiddenCount
            )
            cases
