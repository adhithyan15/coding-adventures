module LispParserSpec (spec) where

import Test.Hspec
import LispParser

spec :: Spec
spec = describe "LispParser" $ do
    it "exposes a non-empty starter description" $ do
        description `shouldSatisfy` (not . null)
