module EcmascriptEs5LexerSpec (spec) where

import Test.Hspec
import EcmascriptEs5Lexer

spec :: Spec
spec = describe "EcmascriptEs5Lexer" $ do
    it "exposes a non-empty starter description" $ do
        description `shouldSatisfy` (not . null)
