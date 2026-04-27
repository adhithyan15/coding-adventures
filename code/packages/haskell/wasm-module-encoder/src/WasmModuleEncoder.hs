module WasmModuleEncoder
    ( description
    , wasmMagic
    , wasmVersion
    , WasmEncodeError(..)
    , encodeModule
    ) where

import qualified Data.ByteString as BS
import Data.ByteString (ByteString)
import qualified Data.List as List
import Data.Word (Word8)
import WasmLeb128 (encodeUnsigned)
import WasmTypes hiding (description)

description :: String
description = "Haskell WebAssembly binary encoder for the core module format"

wasmMagic :: ByteString
wasmMagic = BS.pack [0x00, 0x61, 0x73, 0x6D]

wasmVersion :: ByteString
wasmVersion = BS.pack [0x01, 0x00, 0x00, 0x00]

data WasmEncodeError = WasmEncodeError
    { wasmEncodeErrorMessage :: String
    }
    deriving (Eq, Show)

encodeModule :: WasmModule -> Either WasmEncodeError ByteString
encodeModule moduleValue = do
    customSections <- mapM (encodeSection 0 . encodeCustomSection) (wasmCustomSections moduleValue)
    typeSection <- encodeOptionalSection 1 (wasmTypes moduleValue) encodeFuncType
    importSection <- encodeOptionalSection 2 (wasmImports moduleValue) encodeImport
    functionSection <- encodeOptionalSection 3 (wasmFunctions moduleValue) (Right . encodeUnsigned)
    tableSection <- encodeOptionalSection 4 (wasmTables moduleValue) (Right . encodeTableType)
    memorySection <- encodeOptionalSection 5 (wasmMemories moduleValue) (Right . encodeMemoryType)
    globalSection <- encodeOptionalSection 6 (wasmGlobals moduleValue) (Right . encodeGlobal)
    exportSection <- encodeOptionalSection 7 (wasmExports moduleValue) (Right . encodeExport)
    startSection <-
        case wasmStart moduleValue of
            Nothing -> Right BS.empty
            Just startIndex -> encodeSection 8 (Right (encodeUnsigned startIndex))
    elementSection <- encodeOptionalSection 9 (wasmElements moduleValue) (Right . encodeElement)
    codeSection <- encodeOptionalSection 10 (wasmCode moduleValue) (Right . encodeFunctionBody)
    dataSection <- encodeOptionalSection 11 (wasmDataSegments moduleValue) (Right . encodeDataSegment)
    Right
        ( BS.concat
            [ wasmMagic
            , wasmVersion
            , BS.concat customSections
            , typeSection
            , importSection
            , functionSection
            , tableSection
            , memorySection
            , globalSection
            , exportSection
            , startSection
            , elementSection
            , codeSection
            , dataSection
            ]
        )

encodeOptionalSection :: Word8 -> [a] -> (a -> Either WasmEncodeError ByteString) -> Either WasmEncodeError ByteString
encodeOptionalSection _ [] _ = Right BS.empty
encodeOptionalSection sectionId values encoder =
    encodeSection sectionId (encodeVector values encoder)

encodeSection :: Word8 -> Either WasmEncodeError ByteString -> Either WasmEncodeError ByteString
encodeSection sectionId payloadResult = do
    payload <- payloadResult
    Right (BS.cons sectionId (encodeUnsigned (toInteger (BS.length payload)) <> payload))

encodeVector :: [a] -> (a -> Either WasmEncodeError ByteString) -> Either WasmEncodeError ByteString
encodeVector values encoder = do
    encodedValues <- mapM encoder values
    Right (encodeUnsigned (toInteger (length values)) <> BS.concat encodedValues)

encodeName :: String -> ByteString
encodeName text =
    let bytesValue = BS.pack (map (fromIntegral . fromEnum) text)
     in encodeUnsigned (toInteger (BS.length bytesValue)) <> bytesValue

encodeValueTypes :: [ValueType] -> ByteString
encodeValueTypes values = encodeUnsigned (toInteger (length values)) <> BS.pack (map valueTypeByte values)

encodeFuncType :: FuncType -> Either WasmEncodeError ByteString
encodeFuncType functionType =
    Right (BS.pack [0x60] <> encodeValueTypes (funcTypeParams functionType) <> encodeValueTypes (funcTypeResults functionType))

encodeLimits :: Limits -> ByteString
encodeLimits limitsValue =
    case limitsMax limitsValue of
        Nothing -> BS.pack [0x00] <> encodeUnsigned (limitsMin limitsValue)
        Just maximumValue -> BS.pack [0x01] <> encodeUnsigned (limitsMin limitsValue) <> encodeUnsigned maximumValue

encodeMemoryType :: MemoryType -> ByteString
encodeMemoryType = encodeLimits . memoryTypeLimits

encodeTableType :: TableType -> ByteString
encodeTableType tableType =
    BS.pack [tableTypeElementType tableType] <> encodeLimits (tableTypeLimits tableType)

encodeGlobalType :: GlobalType -> ByteString
encodeGlobalType globalTypeValue =
    BS.pack [valueTypeByte (globalValueType globalTypeValue), if globalMutable globalTypeValue then 0x01 else 0x00]

encodeImport :: Import -> Either WasmEncodeError ByteString
encodeImport importValue =
    case (importKind importValue, importTypeInfo importValue) of
        (ExternalFunction, ImportFunctionType typeIndex) ->
            Right
                ( encodeName (importModuleName importValue)
                    <> encodeName (importName importValue)
                    <> BS.pack [externalKindByte ExternalFunction]
                    <> encodeUnsigned typeIndex
                )
        (ExternalTable, ImportTableType tableType) ->
            Right
                ( encodeName (importModuleName importValue)
                    <> encodeName (importName importValue)
                    <> BS.pack [externalKindByte ExternalTable]
                    <> encodeTableType tableType
                )
        (ExternalMemory, ImportMemoryType memoryType) ->
            Right
                ( encodeName (importModuleName importValue)
                    <> encodeName (importName importValue)
                    <> BS.pack [externalKindByte ExternalMemory]
                    <> encodeMemoryType memoryType
                )
        (ExternalGlobal, ImportGlobalType globalTypeValue) ->
            Right
                ( encodeName (importModuleName importValue)
                    <> encodeName (importName importValue)
                    <> BS.pack [externalKindByte ExternalGlobal]
                    <> encodeGlobalType globalTypeValue
                )
        _ ->
            Left
                WasmEncodeError
                    { wasmEncodeErrorMessage = "import kind does not match import metadata"
                    }

encodeExport :: Export -> ByteString
encodeExport exportValue =
    encodeName (exportName exportValue)
        <> BS.pack [externalKindByte (exportKind exportValue)]
        <> encodeUnsigned (exportIndex exportValue)

encodeGlobal :: Global -> ByteString
encodeGlobal globalValue = encodeGlobalType (globalType globalValue) <> globalInitExpr globalValue

encodeElement :: Element -> ByteString
encodeElement elementValue =
    encodeUnsigned (elementTableIndex elementValue)
        <> elementOffsetExpr elementValue
        <> encodeUnsigned (toInteger (length (elementFunctionIndices elementValue)))
        <> BS.concat (map encodeUnsigned (elementFunctionIndices elementValue))

encodeDataSegment :: DataSegment -> ByteString
encodeDataSegment segment =
    encodeUnsigned (dataSegmentMemoryIndex segment)
        <> dataSegmentOffsetExpr segment
        <> encodeUnsigned (toInteger (BS.length (dataSegmentBytes segment)))
        <> dataSegmentBytes segment

encodeFunctionBody :: FunctionBody -> ByteString
encodeFunctionBody body =
    let localGroups = groupLocals (functionBodyLocals body)
        localPayload =
            encodeUnsigned (toInteger (length localGroups))
                <> BS.concat
                    [ encodeUnsigned (toInteger count) <> BS.pack [valueTypeByte valueType]
                    | (count, valueType) <- localGroups
                    ]
        payload = localPayload <> functionBodyCode body
     in encodeUnsigned (toInteger (BS.length payload)) <> payload

groupLocals :: [ValueType] -> [(Int, ValueType)]
groupLocals [] = []
groupLocals values =
    map toPair (List.group values)
  where
    toPair groupValues =
        case groupValues of
            [] -> error "groupLocals received an empty group"
            valueType : _ -> (length groupValues, valueType)

encodeCustomSection :: CustomSection -> Either WasmEncodeError ByteString
encodeCustomSection customSection =
    Right (encodeName (customSectionName customSection) <> customSectionData customSection)
