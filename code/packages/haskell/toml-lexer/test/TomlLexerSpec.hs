module TomlLexerSpec (spec) where

import Test.Hspec
import TomlLexer

spec :: Spec
spec = describe "TomlLexer" $ do
    it "exposes a non-empty starter description" $ do
        description `shouldSatisfy` (not . null)
