module WasmTypesSpec (spec) where

import qualified Data.ByteString as BS
import Test.Hspec
import WasmTypes

spec :: Spec
spec = describe "WasmTypes" $ do
    it "maps value types to their WASM bytes" $ do
        map valueTypeByte [I32, I64, F32, F64] `shouldBe` [0x7F, 0x7E, 0x7D, 0x7C]

    it "round-trips external kinds through bytes" $ do
        map externalKindFromByte [0x00, 0x01, 0x02, 0x03]
            `shouldBe` map Just [ExternalFunction, ExternalTable, ExternalMemory, ExternalGlobal]

    it "starts an empty module with no sections" $ do
        wasmTypes emptyModule `shouldBe` []
        wasmImports emptyModule `shouldBe` []
        wasmFunctions emptyModule `shouldBe` []
        wasmStart emptyModule `shouldBe` Nothing

    it "supports realistic module values" $ do
        let moduleValue =
                emptyModule
                    { wasmTypes = [FuncType [I32] [I32]]
                    , wasmMemories = [MemoryType (Limits 1 (Just 2))]
                    , wasmDataSegments =
                        [ DataSegment
                            { dataSegmentMemoryIndex = 0
                            , dataSegmentOffsetExpr = BS.pack [0x41, 0x00, 0x0B]
                            , dataSegmentBytes = BS.pack [0x4E, 0x69, 0x62]
                            }
                        ]
                    }
        length (wasmTypes moduleValue) `shouldBe` 1
        case wasmDataSegments moduleValue of
            firstSegment : _ -> dataSegmentBytes firstSegment `shouldBe` BS.pack [0x4E, 0x69, 0x62]
            [] -> expectationFailure "expected a data segment"
