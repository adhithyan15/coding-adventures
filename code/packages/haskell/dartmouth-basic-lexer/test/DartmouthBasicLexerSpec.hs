module DartmouthBasicLexerSpec (spec) where

import Test.Hspec
import DartmouthBasicLexer

spec :: Spec
spec = describe "DartmouthBasicLexer" $ do
    it "exposes a non-empty starter description" $ do
        description `shouldSatisfy` (not . null)
