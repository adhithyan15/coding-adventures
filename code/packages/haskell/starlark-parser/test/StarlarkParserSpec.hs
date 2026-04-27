module StarlarkParserSpec (spec) where

import Test.Hspec
import StarlarkParser

spec :: Spec
spec = describe "StarlarkParser" $ do
    it "exposes a non-empty starter description" $ do
        description `shouldSatisfy` (not . null)
