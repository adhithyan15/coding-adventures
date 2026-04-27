module AlgolParserSpec (spec) where

import Test.Hspec
import AlgolParser

spec :: Spec
spec = describe "AlgolParser" $ do
    it "exposes a non-empty starter description" $ do
        description `shouldSatisfy` (not . null)
