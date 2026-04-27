module ExcelParserSpec (spec) where

import Test.Hspec
import ExcelParser

spec :: Spec
spec = describe "ExcelParser" $ do
    it "exposes a non-empty starter description" $ do
        description `shouldSatisfy` (not . null)
