module LispLexerSpec (spec) where

import Test.Hspec
import LispLexer

spec :: Spec
spec = describe "LispLexer" $ do
    it "exposes a non-empty starter description" $ do
        description `shouldSatisfy` (not . null)
