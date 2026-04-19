module CompilerIRSpec (spec) where

import CompilerIR
import Test.Hspec

spec :: Spec
spec = describe "CompilerIR" $ do
    it "stores linear instructions" $ do
        let program =
                appendInstruction
                    (emptyProgram "_start")
                    (instruction LoadImm [Register 1, Immediate 7] 0)
        irEntryLabel program `shouldBe` "_start"
        maxRegister program `shouldBe` 1

    it "describes the package" $ do
        description `shouldSatisfy` (not . null)
