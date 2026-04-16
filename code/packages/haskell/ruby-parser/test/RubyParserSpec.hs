module RubyParserSpec (spec) where

import Test.Hspec
import RubyParser

spec :: Spec
spec = describe "RubyParser" $ do
    it "exposes a non-empty starter description" $ do
        description `shouldSatisfy` (not . null)
