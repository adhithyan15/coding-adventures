module PythonParserSpec (spec) where

import Test.Hspec
import PythonParser

spec :: Spec
spec = describe "PythonParser" $ do
    it "exposes a non-empty starter description" $ do
        description `shouldSatisfy` (not . null)
