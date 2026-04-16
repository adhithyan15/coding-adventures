module WasmRuntimeSpec (spec) where

import qualified Data.ByteString as BS
import Test.Hspec
import WasmExecution
import WasmRuntime
import WasmTypes

spec :: Spec
spec = describe "WasmRuntime" $ do
    it "instantiates and calls a square function" $ do
        let moduleValue =
                emptyModule
                    { wasmTypes = [FuncType [I32] [I32]]
                    , wasmFunctions = [0]
                    , wasmExports = [Export "square" ExternalFunction 0]
                    , wasmCode = [FunctionBody [] (BS.pack [0x20, 0x00, 0x20, 0x00, 0x6C, 0x0B])]
                    }
            runtime = newRuntime Nothing
        instanceResult <- instantiateModule runtime moduleValue
        case instanceResult of
            Left err -> expectationFailure err
            Right instanceValue -> do
                result <- callExportedFunction runtime instanceValue "square" [WasmI32 5]
                result `shouldBe` Right [WasmI32 25]

    it "applies data segments to memory during instantiation" $ do
        let moduleValue =
                emptyModule
                    { wasmTypes = [FuncType [] []]
                    , wasmFunctions = [0]
                    , wasmMemories = [MemoryType (Limits 1 Nothing)]
                    , wasmCode = [FunctionBody [] (BS.pack [0x0B])]
                    , wasmDataSegments =
                        [ DataSegment
                            { dataSegmentMemoryIndex = 0
                            , dataSegmentOffsetExpr = BS.pack [0x41, 0x00, 0x0B]
                            , dataSegmentBytes = BS.pack [0x57, 0x41, 0x53, 0x4D]
                            }
                        ]
                    }
            runtime = newRuntime Nothing
        instanceResult <- instantiateModule runtime moduleValue
        case instanceResult of
            Left err -> expectationFailure err
            Right instanceValue ->
                case instanceMemory instanceValue of
                    Nothing -> expectationFailure "expected memory to be allocated"
                    Just memory -> do
                        loadI32_8u memory 0 `shouldReturn` 0x57
                        loadI32_8u memory 1 `shouldReturn` 0x41

    it "rejects missing exports" $ do
        let runtime = newRuntime Nothing
            moduleValue = emptyModule
            globalsRefAction = instantiateModule runtime moduleValue
        instanceResult <- globalsRefAction
        case instanceResult of
            Left err -> expectationFailure err
            Right instanceValue -> do
                result <- callExportedFunction runtime instanceValue "missing" []
                result `shouldBe` Left (TrapError "export \"missing\" not found")
