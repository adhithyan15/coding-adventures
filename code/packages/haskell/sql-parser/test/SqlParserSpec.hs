module SqlParserSpec (spec) where

import Test.Hspec
import SqlParser

spec :: Spec
spec = describe "SqlParser" $ do
    it "exposes a non-empty starter description" $ do
        description `shouldSatisfy` (not . null)
