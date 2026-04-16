module WasmModuleParserSpec (spec) where

import Test.Hspec
import WasmModuleParser

spec :: Spec
spec = describe "WasmModuleParser" $ do
    it "exposes a non-empty starter description" $ do
        description `shouldSatisfy` (not . null)
