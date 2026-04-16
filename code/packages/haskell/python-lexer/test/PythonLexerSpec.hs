module PythonLexerSpec (spec) where

import Test.Hspec
import PythonLexer

spec :: Spec
spec = describe "PythonLexer" $ do
    it "exposes a non-empty starter description" $ do
        description `shouldSatisfy` (not . null)
