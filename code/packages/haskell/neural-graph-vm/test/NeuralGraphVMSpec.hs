module NeuralGraphVMSpec (spec) where

import qualified Data.Map.Strict as Map
import Test.Hspec

import NeuralNetwork
import NeuralGraphVM

tinyGraph :: NeuralGraph
tinyGraph =
    addOutput "out" "relu" "prediction" emptyBag (Just "relu_to_out")
    $ addActivation "relu" "sum" "relu" emptyBag (Just "sum_to_relu")
    $ addWeightedSum "sum" [wi "x0" 0.25 "x0_to_sum", wi "x1" 0.75 "x1_to_sum", wi "bias" (-1.0) "bias_to_sum"] emptyBag
    $ addConstant "bias" 1.0 emptyBag
    $ addInput "x1" "x1" emptyBag
    $ addInput "x0" "x0" emptyBag
    $ createNeuralGraph (Just "tiny")

spec :: Spec
spec = do
    it "runs the tiny weighted sum" $ do
        let result = compileNeuralGraphToBytecode tinyGraph >>= (`runNeuralBytecodeForward` Map.fromList [("x0", 4.0), ("x1", 8.0)])
        fmap (Map.lookup "prediction") result `shouldBe` Right (Just 6.0)

    it "runs xor" $ do
        let Right bytecode = compileNeuralNetworkToBytecode (createXorNetwork "xor")
            cases = [(0.0, 0.0, 0.0), (0.0, 1.0, 1.0), (1.0, 0.0, 1.0), (1.0, 1.0, 0.0)]
            check (x0, x1, expected) =
                let Right outputs = runNeuralBytecodeForward bytecode (Map.fromList [("x0", x0), ("x1", x1)])
                    Just prediction = Map.lookup "prediction" outputs
                in if expected == 1.0 then prediction > 0.99 else prediction < 0.01
        map check cases `shouldBe` [True, True, True, True]
