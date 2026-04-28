module Main where

import Test.Hspec
import qualified SingleLayerNetworkSpec

main :: IO ()
main = hspec SingleLayerNetworkSpec.spec
