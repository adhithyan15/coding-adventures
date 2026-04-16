module MosaicLexerSpec (spec) where

import Test.Hspec
import MosaicLexer

spec :: Spec
spec = describe "MosaicLexer" $ do
    it "exposes a non-empty starter description" $ do
        description `shouldSatisfy` (not . null)
