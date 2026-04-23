module MosaicParserSpec (spec) where

import Test.Hspec
import MosaicParser

spec :: Spec
spec = describe "MosaicParser" $ do
    it "exposes a non-empty starter description" $ do
        description `shouldSatisfy` (not . null)
