module CsharpLexerSpec (spec) where

import Test.Hspec
import CsharpLexer

spec :: Spec
spec = describe "CsharpLexer" $ do
    it "exposes a non-empty starter description" $ do
        description `shouldSatisfy` (not . null)
