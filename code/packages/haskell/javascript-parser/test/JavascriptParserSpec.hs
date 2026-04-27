module JavascriptParserSpec (spec) where

import Test.Hspec
import JavascriptParser

spec :: Spec
spec = describe "JavascriptParser" $ do
    it "exposes a non-empty starter description" $ do
        description `shouldSatisfy` (not . null)
