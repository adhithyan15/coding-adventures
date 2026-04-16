module CsharpParserSpec (spec) where

import Test.Hspec
import CsharpParser

spec :: Spec
spec = describe "CsharpParser" $ do
    it "exposes a non-empty starter description" $ do
        description `shouldSatisfy` (not . null)
