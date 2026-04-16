module FsharpLexerSpec (spec) where

import Test.Hspec
import FsharpLexer

spec :: Spec
spec = describe "FsharpLexer" $ do
    it "exposes a non-empty starter description" $ do
        description `shouldSatisfy` (not . null)
