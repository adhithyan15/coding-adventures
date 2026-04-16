module WasmModuleEncoderSpec (spec) where

import qualified Data.ByteString as BS
import Test.Hspec
import WasmModuleEncoder
import WasmModuleParser (parseModule)
import WasmTypes

spec :: Spec
spec = describe "WasmModuleEncoder" $ do
    it "round-trips a minimal exported function through the parser" $ do
        let moduleValue =
                emptyModule
                    { wasmTypes = [FuncType [I32] [I32]]
                    , wasmFunctions = [0]
                    , wasmExports = [Export "identity" ExternalFunction 0]
                    , wasmCode = [FunctionBody [] (BS.pack [0x20, 0x00, 0x0B])]
                    }
        case encodeModule moduleValue of
            Left err -> expectationFailure (show err)
            Right encoded ->
                case parseModule encoded of
                    Left err -> expectationFailure (show err)
                    Right parsed -> parsed `shouldBe` moduleValue

    it "round-trips memory, globals, start, and data segments" $ do
        let moduleValue =
                emptyModule
                    { wasmTypes = [FuncType [] [I32]]
                    , wasmFunctions = [0]
                    , wasmMemories = [MemoryType (Limits 1 (Just 2))]
                    , wasmGlobals = [Global (GlobalType I32 False) (BS.pack [0x41, 0x2A, 0x0B])]
                    , wasmExports = [Export "main" ExternalFunction 0, Export "memory" ExternalMemory 0]
                    , wasmStart = Just 0
                    , wasmCode = [FunctionBody [I32] (BS.pack [0x41, 0x07, 0x0B])]
                    , wasmDataSegments =
                        [ DataSegment
                            { dataSegmentMemoryIndex = 0
                            , dataSegmentOffsetExpr = BS.pack [0x41, 0x00, 0x0B]
                            , dataSegmentBytes = BS.pack [0x4E, 0x69, 0x62]
                            }
                        ]
                    }
        case encodeModule moduleValue of
            Left err -> expectationFailure (show err)
            Right encoded ->
                case parseModule encoded of
                    Left err -> expectationFailure (show err)
                    Right parsed -> do
                        wasmMemories parsed `shouldBe` wasmMemories moduleValue
                        wasmGlobals parsed `shouldBe` wasmGlobals moduleValue
                        wasmStart parsed `shouldBe` wasmStart moduleValue
                        wasmDataSegments parsed `shouldBe` wasmDataSegments moduleValue

    it "rejects mismatched import metadata" $ do
        let brokenModule =
                emptyModule
                    { wasmImports =
                        [ Import
                            { importModuleName = "env"
                            , importName = "f"
                            , importKind = ExternalFunction
                            , importTypeInfo = ImportMemoryType (MemoryType (Limits 1 Nothing))
                            }
                        ]
                    }
        encodeModule brokenModule `shouldSatisfy` isLeft

isLeft :: Either a b -> Bool
isLeft eitherValue =
    case eitherValue of
        Left _ -> True
        Right _ -> False
