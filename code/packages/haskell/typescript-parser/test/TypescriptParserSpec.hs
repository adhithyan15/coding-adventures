module TypescriptParserSpec (spec) where

import Test.Hspec
import TypescriptParser

spec :: Spec
spec = describe "TypescriptParser" $ do
    it "exposes a non-empty starter description" $ do
        description `shouldSatisfy` (not . null)
