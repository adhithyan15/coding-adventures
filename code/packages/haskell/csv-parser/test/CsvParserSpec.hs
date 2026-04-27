module CsvParserSpec (spec) where

import Test.Hspec
import CsvParser

spec :: Spec
spec = describe "CsvParser" $ do
    it "exposes a non-empty starter description" $ do
        description `shouldSatisfy` (not . null)
