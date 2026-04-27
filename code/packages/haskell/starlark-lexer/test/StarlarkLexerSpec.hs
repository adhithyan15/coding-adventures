module StarlarkLexerSpec (spec) where

import Test.Hspec
import StarlarkLexer

spec :: Spec
spec = describe "StarlarkLexer" $ do
    it "exposes a non-empty starter description" $ do
        description `shouldSatisfy` (not . null)
