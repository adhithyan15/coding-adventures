module JavaParserSpec (spec) where

import Test.Hspec
import JavaParser

spec :: Spec
spec = describe "JavaParser" $ do
    it "exposes a non-empty starter description" $ do
        description `shouldSatisfy` (not . null)
