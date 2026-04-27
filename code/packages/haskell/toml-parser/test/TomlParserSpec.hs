module TomlParserSpec (spec) where

import Test.Hspec
import TomlParser

spec :: Spec
spec = describe "TomlParser" $ do
    it "exposes a non-empty starter description" $ do
        description `shouldSatisfy` (not . null)
