module ExcelLexerSpec (spec) where

import Test.Hspec
import ExcelLexer

spec :: Spec
spec = describe "ExcelLexer" $ do
    it "exposes a non-empty starter description" $ do
        description `shouldSatisfy` (not . null)
