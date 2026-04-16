module JsonLexerSpec (spec) where

import Test.Hspec
import JsonLexer

spec :: Spec
spec = describe "JsonLexer" $ do
    it "exposes a non-empty starter description" $ do
        description `shouldSatisfy` (not . null)
