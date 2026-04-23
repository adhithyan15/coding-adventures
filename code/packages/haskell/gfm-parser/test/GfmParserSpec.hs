module GfmParserSpec (spec) where

import Test.Hspec
import GfmParser

spec :: Spec
spec = describe "GfmParser" $ do
    it "exposes a non-empty starter description" $ do
        description `shouldSatisfy` (not . null)
