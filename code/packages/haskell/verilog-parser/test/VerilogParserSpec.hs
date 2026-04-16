module VerilogParserSpec (spec) where

import Test.Hspec
import VerilogParser

spec :: Spec
spec = describe "VerilogParser" $ do
    it "exposes a non-empty starter description" $ do
        description `shouldSatisfy` (not . null)
