module FontParserSpec (spec) where

import Test.Hspec
import FontParser

spec :: Spec
spec = describe "FontParser" $ do
    it "exposes a non-empty starter description" $ do
        description `shouldSatisfy` (not . null)
