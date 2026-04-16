module JsonParserSpec (spec) where

import Test.Hspec
import JsonParser

spec :: Spec
spec = describe "JsonParser" $ do
    it "exposes a non-empty starter description" $ do
        description `shouldSatisfy` (not . null)
