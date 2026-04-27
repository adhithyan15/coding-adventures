module CssLexerSpec (spec) where

import Test.Hspec
import CssLexer

spec :: Spec
spec = describe "CssLexer" $ do
    it "exposes a non-empty starter description" $ do
        description `shouldSatisfy` (not . null)
