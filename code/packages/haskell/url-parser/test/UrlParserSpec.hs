module UrlParserSpec (spec) where

import Test.Hspec
import UrlParser

spec :: Spec
spec = describe "UrlParser" $ do
    it "exposes a non-empty starter description" $ do
        description `shouldSatisfy` (not . null)
