module JavascriptLexerSpec (spec) where

import Test.Hspec
import JavascriptLexer

spec :: Spec
spec = describe "JavascriptLexer" $ do
    it "exposes a non-empty starter description" $ do
        description `shouldSatisfy` (not . null)
