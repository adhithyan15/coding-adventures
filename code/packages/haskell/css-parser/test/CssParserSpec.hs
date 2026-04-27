module CssParserSpec (spec) where

import Test.Hspec
import CssParser

spec :: Spec
spec = describe "CssParser" $ do
    it "exposes a non-empty starter description" $ do
        description `shouldSatisfy` (not . null)
