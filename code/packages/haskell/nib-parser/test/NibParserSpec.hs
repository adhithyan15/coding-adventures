module NibParserSpec (spec) where

import Test.Hspec
import NibParser

spec :: Spec
spec = describe "NibParser" $ do
    it "exposes a non-empty starter description" $ do
        description `shouldSatisfy` (not . null)
