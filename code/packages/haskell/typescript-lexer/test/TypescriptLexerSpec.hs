module TypescriptLexerSpec (spec) where

import Test.Hspec
import TypescriptLexer

spec :: Spec
spec = describe "TypescriptLexer" $ do
    it "exposes a non-empty starter description" $ do
        description `shouldSatisfy` (not . null)
