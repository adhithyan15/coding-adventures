module Main where

import Test.Hspec
import qualified NeuralGraphVMSpec

main :: IO ()
main = hspec NeuralGraphVMSpec.spec
