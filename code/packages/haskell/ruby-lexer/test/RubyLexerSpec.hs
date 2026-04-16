module RubyLexerSpec (spec) where

import Test.Hspec
import RubyLexer

spec :: Spec
spec = describe "RubyLexer" $ do
    it "exposes a non-empty starter description" $ do
        description `shouldSatisfy` (not . null)
