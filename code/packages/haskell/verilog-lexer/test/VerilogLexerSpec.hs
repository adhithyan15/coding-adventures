module VerilogLexerSpec (spec) where

import Test.Hspec
import VerilogLexer

spec :: Spec
spec = describe "VerilogLexer" $ do
    it "exposes a non-empty starter description" $ do
        description `shouldSatisfy` (not . null)
