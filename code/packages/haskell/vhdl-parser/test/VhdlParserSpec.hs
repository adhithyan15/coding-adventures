module VhdlParserSpec (spec) where

import Test.Hspec
import VhdlParser

spec :: Spec
spec = describe "VhdlParser" $ do
    it "exposes a non-empty starter description" $ do
        description `shouldSatisfy` (not . null)
