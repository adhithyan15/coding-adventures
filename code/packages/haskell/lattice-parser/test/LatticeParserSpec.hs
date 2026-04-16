module LatticeParserSpec (spec) where

import Test.Hspec
import LatticeParser

spec :: Spec
spec = describe "LatticeParser" $ do
    it "exposes a non-empty starter description" $ do
        description `shouldSatisfy` (not . null)
