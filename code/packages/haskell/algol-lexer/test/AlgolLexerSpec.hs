module AlgolLexerSpec (spec) where

import Test.Hspec
import AlgolLexer

spec :: Spec
spec = describe "AlgolLexer" $ do
    it "exposes a non-empty starter description" $ do
        description `shouldSatisfy` (not . null)
