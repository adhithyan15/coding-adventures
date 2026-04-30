module NeuralNetworkSpec (spec) where

import Test.Hspec
import NeuralNetwork

spec :: Spec
spec = do
    it "builds a tiny weighted graph" $ do
        let graph = addOutput "out" "relu" "prediction" emptyBag (Just "relu_to_out")
                  $ addActivation "relu" "sum" "relu" emptyBag (Just "sum_to_relu")
                  $ addWeightedSum "sum" [wi "x0" 0.25 "x0_to_sum", wi "x1" 0.75 "x1_to_sum", wi "bias" (-1.0) "bias_to_sum"] emptyBag
                  $ addConstant "bias" 1.0 emptyBag
                  $ addInput "x1" "x1" emptyBag
                  $ addInput "x0" "x0" emptyBag
                  $ createNeuralGraph (Just "tiny")
        length (incomingEdges "sum" graph) `shouldBe` 3
        fmap last (topologicalSort graph) `shouldBe` Right "out"

    it "builds xor with hidden output edge" $ do
        let graph = networkGraph (createXorNetwork "xor")
        any ((== "h_or_to_out") . edgeId) (graphEdges graph) `shouldBe` True
