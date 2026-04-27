module AsciidocParserSpec (spec) where

import Test.Hspec
import AsciidocParser

spec :: Spec
spec = describe "AsciidocParser" $ do
    it "exposes a non-empty starter description" $ do
        description `shouldSatisfy` (not . null)
