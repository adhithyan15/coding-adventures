module EcmascriptEs3LexerSpec (spec) where

import Test.Hspec
import EcmascriptEs3Lexer

spec :: Spec
spec = describe "EcmascriptEs3Lexer" $ do
    it "exposes a non-empty starter description" $ do
        description `shouldSatisfy` (not . null)
