module DartmouthBasicParserSpec (spec) where

import Test.Hspec
import DartmouthBasicParser

spec :: Spec
spec = describe "DartmouthBasicParser" $ do
    it "exposes a non-empty starter description" $ do
        description `shouldSatisfy` (not . null)
