module JavaLexerSpec (spec) where

import Test.Hspec
import JavaLexer

spec :: Spec
spec = describe "JavaLexer" $ do
    it "exposes a non-empty starter description" $ do
        description `shouldSatisfy` (not . null)
