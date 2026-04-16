module WasmValidatorSpec (spec) where

import qualified Data.ByteString as BS
import Test.Hspec
import WasmTypes
import WasmValidator

spec :: Spec
spec = describe "WasmValidator" $ do
    it "accepts a simple well-formed module" $ do
        let moduleValue =
                emptyModule
                    { wasmTypes = [FuncType [I32] [I32]]
                    , wasmFunctions = [0]
                    , wasmExports = [Export "identity" ExternalFunction 0]
                    , wasmCode = [FunctionBody [] (BS.pack [0x20, 0x00, 0x0B])]
                    }
        case validateModule moduleValue of
            Left err -> expectationFailure (show err)
            Right validated -> validatedFuncTypes validated `shouldBe` [FuncType [I32] [I32]]

    it "rejects duplicate export names" $ do
        let moduleValue =
                emptyModule
                    { wasmTypes = [FuncType [] []]
                    , wasmFunctions = [0]
                    , wasmExports = [Export "dup" ExternalFunction 0, Export "dup" ExternalFunction 0]
                    , wasmCode = [FunctionBody [] (BS.pack [0x0B])]
                    }
        case validateModule moduleValue of
            Left err -> validationErrorKind err `shouldBe` DuplicateExportName
            Right _ -> expectationFailure "expected validation failure"

    it "rejects invalid start function signatures" $ do
        let moduleValue =
                emptyModule
                    { wasmTypes = [FuncType [I32] []]
                    , wasmFunctions = [0]
                    , wasmStart = Just 0
                    , wasmCode = [FunctionBody [] (BS.pack [0x0B])]
                    }
        case validateModule moduleValue of
            Left err -> validationErrorKind err `shouldBe` StartFunctionBadType
            Right _ -> expectationFailure "expected validation failure"

    it "rejects over-sized memory declarations" $ do
        let moduleValue = emptyModule {wasmMemories = [MemoryType (Limits 1 (Just 70000))]}
        case validateModule moduleValue of
            Left err -> validationErrorKind err `shouldBe` MemoryLimitExceeded
            Right _ -> expectationFailure "expected validation failure"
