module Main where

import Test.Hspec
import qualified NeuralNetworkSpec

main :: IO ()
main = hspec NeuralNetworkSpec.spec
