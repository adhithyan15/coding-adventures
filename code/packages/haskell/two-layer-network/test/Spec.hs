module Main where

import Test.Hspec
import qualified TwoLayerNetworkSpec

main :: IO ()
main = hspec TwoLayerNetworkSpec.spec
