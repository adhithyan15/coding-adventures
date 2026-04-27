module XmlLexerSpec (spec) where

import Test.Hspec
import XmlLexer

spec :: Spec
spec = describe "XmlLexer" $ do
    it "exposes a non-empty starter description" $ do
        description `shouldSatisfy` (not . null)
