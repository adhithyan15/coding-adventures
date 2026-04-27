module WasmModuleParserSpec (spec) where

import qualified Data.ByteString as BS
import Data.ByteString (ByteString)
import Data.Word (Word8)
import Test.Hspec
import WasmModuleParser
import WasmTypes

spec :: Spec
spec = describe "WasmModuleParser" $ do
    it "parses the minimal header-only module" $ do
        parseModule (wasmMagic <> wasmVersion) `shouldBe` Right emptyModule

    it "parses types, functions, exports, and code" $ do
        let moduleBytes =
                makeWasm
                    [ (1, BS.pack [0x01, 0x60, 0x01, 0x7F, 0x01, 0x7F])
                    , (3, BS.pack [0x01, 0x00])
                    , (7, BS.pack [0x01, 0x03, 0x61, 0x64, 0x64, 0x00, 0x00])
                    , (10, BS.pack [0x01, 0x04, 0x00, 0x20, 0x00, 0x0B])
                    ]
        case parseModule moduleBytes of
            Left err -> expectationFailure (show err)
            Right moduleValue -> do
                wasmTypes moduleValue `shouldBe` [FuncType [I32] [I32]]
                wasmFunctions moduleValue `shouldBe` [0]
                wasmExports moduleValue `shouldBe` [Export "add" ExternalFunction 0]
                wasmCode moduleValue `shouldBe` [FunctionBody [] (BS.pack [0x20, 0x00, 0x0B])]

    it "parses imports, memory, globals, data, and custom sections" $ do
        let importPayload = BS.pack [0x01, 0x03, 0x65, 0x6E, 0x76, 0x06, 0x6D, 0x65, 0x6D, 0x6F, 0x72, 0x79, 0x02, 0x00, 0x01]
            memoryPayload = BS.pack [0x01, 0x01, 0x01, 0x02]
            globalPayload = BS.pack [0x01, 0x7F, 0x00, 0x41, 0x2A, 0x0B]
            dataPayload = BS.pack [0x01, 0x00, 0x41, 0x00, 0x0B, 0x03, 0x4E, 0x69, 0x62]
            customPayload = BS.pack [0x04, 0x6E, 0x61, 0x6D, 0x65, 0x01, 0x02]
            moduleBytes = makeWasm [(2, importPayload), (5, memoryPayload), (6, globalPayload), (11, dataPayload), (0, customPayload)]
        case parseModule moduleBytes of
            Left err -> expectationFailure (show err)
            Right moduleValue -> do
                length (wasmImports moduleValue) `shouldBe` 1
                wasmMemories moduleValue `shouldBe` [MemoryType (Limits 1 (Just 2))]
                length (wasmGlobals moduleValue) `shouldBe` 1
                case wasmDataSegments moduleValue of
                    firstSegment : _ -> dataSegmentBytes firstSegment `shouldBe` BS.pack [0x4E, 0x69, 0x62]
                    [] -> expectationFailure "expected a data segment"
                wasmCustomSections moduleValue `shouldBe` [CustomSection "name" (BS.pack [0x01, 0x02])]

    it "rejects bad magic bytes" $ do
        parseModule (BS.pack [0x57, 0x41, 0x53, 0x4D, 0x01, 0x00, 0x00, 0x00]) `shouldSatisfy` isLeft

    it "rejects truncated section payloads" $ do
        parseModule (wasmMagic <> wasmVersion <> BS.pack [0x01, 0x0A, 0x01, 0x60]) `shouldSatisfy` isLeft

makeWasm :: [(Word8, ByteString)] -> ByteString
makeWasm sections = wasmMagic <> wasmVersion <> BS.concat (map encodeSection sections)
  where
    encodeSection (sectionId, payload) = BS.pack [sectionId] <> encodeUnsigned (toInteger (BS.length payload)) <> payload

encodeUnsigned :: Integer -> ByteString
encodeUnsigned value =
    if value < 128
        then BS.singleton (fromIntegral value)
        else BS.pack [fromIntegral (value `mod` 128) + 0x80, fromIntegral (value `div` 128)]

isLeft :: Either a b -> Bool
isLeft eitherValue =
    case eitherValue of
        Left _ -> True
        Right _ -> False
