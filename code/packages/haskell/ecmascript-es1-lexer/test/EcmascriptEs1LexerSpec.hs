module EcmascriptEs1LexerSpec (spec) where

import Test.Hspec
import EcmascriptEs1Lexer

spec :: Spec
spec = describe "EcmascriptEs1Lexer" $ do
    it "exposes a non-empty starter description" $ do
        description `shouldSatisfy` (not . null)
