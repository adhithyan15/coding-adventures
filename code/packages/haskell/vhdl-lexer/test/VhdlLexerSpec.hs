module VhdlLexerSpec (spec) where

import Test.Hspec
import VhdlLexer

spec :: Spec
spec = describe "VhdlLexer" $ do
    it "exposes a non-empty starter description" $ do
        description `shouldSatisfy` (not . null)
