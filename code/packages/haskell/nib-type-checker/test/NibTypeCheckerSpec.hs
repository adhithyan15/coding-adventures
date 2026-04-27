module NibTypeCheckerSpec (spec) where

import NibTypeChecker
import Test.Hspec

spec :: Spec
spec = describe "NibTypeChecker" $ do
    it "accepts a simple u4 return" $ do
        typeCheckOk (checkSource "fn answer() -> u4 { return 7; }") `shouldBe` True

    it "rejects mismatched let initializers" $ do
        let result = checkSource "fn bad() -> u4 { let x: u4 = true; return 1; }"
        typeCheckOk result `shouldBe` False
        typeCheckErrors result `shouldSatisfy` (not . null)

    it "counts function parameters" $ do
        let typed = checkSource "fn add(a: u4, b: u4) -> u4 { return a +% b; }"
        case functionNodes (typedAstRoot (typeCheckTypedAst typed)) of
            fn : _ -> countParams fn `shouldBe` 2
            [] -> expectationFailure "expected a function"
