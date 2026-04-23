module FsharpParserSpec (spec) where

import Test.Hspec
import FsharpParser

spec :: Spec
spec = describe "FsharpParser" $ do
    it "exposes a non-empty starter description" $ do
        description `shouldSatisfy` (not . null)
