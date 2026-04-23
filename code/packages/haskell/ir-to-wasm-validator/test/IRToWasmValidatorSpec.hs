module IRToWasmValidatorSpec (spec) where

import CompilerIR
import IRToWasmValidator
import Test.Hspec

spec :: Spec
spec = describe "IRToWasmValidator" $ do
    it "accepts the lowering subset" $ do
        let program =
                appendInstruction
                    (emptyProgram "_start")
                    (instruction LoadImm [Register 1, Immediate 7] 0)
        validateProgram program `shouldBe` []

    it "rejects malformed operands" $ do
        let program =
                appendInstruction
                    (emptyProgram "_start")
                    (instruction LoadImm [LabelRef "bad"] 9)
        validateProgram program `shouldSatisfy` (not . null)
