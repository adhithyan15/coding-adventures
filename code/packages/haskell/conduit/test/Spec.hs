module Main (main) where

import Test.Hspec (hspec)
import qualified ConduitSpec
import qualified ServerE2ESpec

main :: IO ()
main = hspec $ do
  ConduitSpec.spec
  ServerE2ESpec.spec
