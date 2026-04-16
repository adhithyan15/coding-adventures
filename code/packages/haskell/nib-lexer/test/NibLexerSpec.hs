module NibLexerSpec (spec) where

import Test.Hspec
import NibLexer

spec :: Spec
spec = describe "NibLexer" $ do
    it "exposes a non-empty starter description" $ do
        description `shouldSatisfy` (not . null)
