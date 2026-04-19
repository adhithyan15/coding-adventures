module IRToWasmCompilerSpec (spec) where

import CompilerIR
import IRToWasmCompiler
import Test.Hspec
import WasmExecution
import WasmModuleEncoder
import WasmRuntime
import WasmTypes
import WasmValidator

spec :: Spec
spec = describe "IRToWasmCompiler" $ do
    it "lowers a constant-returning function" $ do
        let program =
                foldl
                    appendInstruction
                    (emptyProgram "_start")
                    [ instruction Label [LabelRef "_fn_answer"] 0
                    , instruction LoadImm [Register 1, Immediate 7] 1
                    , instruction Ret [] 2
                    ]
            signatures = [FunctionSignature "_fn_answer" 0 (Just "answer")]
        moduleValue <- unwrapEither (compileProgram program signatures)
        validateModule moduleValue `shouldSatisfy` either (const False) (const True)
        bytesValue <- unwrapEither (encodeModule moduleValue)
        result <- loadAndRun (newRuntime Nothing) bytesValue "answer" []
        result `shouldBe` Right [WasmI32 7]

unwrapEither :: Show err => Either err value -> IO value
unwrapEither result =
    case result of
        Left err -> expectationFailure (show err) >> error "unreachable after expectationFailure"
        Right value -> pure value
