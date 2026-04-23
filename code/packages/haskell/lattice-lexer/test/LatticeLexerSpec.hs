module LatticeLexerSpec (spec) where

import Test.Hspec
import LatticeLexer

spec :: Spec
spec = describe "LatticeLexer" $ do
    it "exposes a non-empty starter description" $ do
        description `shouldSatisfy` (not . null)
