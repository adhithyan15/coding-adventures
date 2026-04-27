module SqlLexerSpec (spec) where

import Test.Hspec
import SqlLexer

spec :: Spec
spec = describe "SqlLexer" $ do
    it "exposes a non-empty starter description" $ do
        description `shouldSatisfy` (not . null)
