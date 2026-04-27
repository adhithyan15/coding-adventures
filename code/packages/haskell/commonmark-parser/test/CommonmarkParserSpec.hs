module CommonmarkParserSpec (spec) where

import Test.Hspec
import CommonmarkParser

spec :: Spec
spec = describe "CommonmarkParser" $ do
    it "exposes a non-empty starter description" $ do
        description `shouldSatisfy` (not . null)
