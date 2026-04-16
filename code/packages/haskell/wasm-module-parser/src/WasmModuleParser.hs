module WasmModuleParser
    ( description
    , wasmMagic
    , wasmVersion
    , WasmParseError(..)
    , parseModule
    ) where

import qualified Data.Bits as Bits
import qualified Data.ByteString as BS
import Data.ByteString (ByteString)
import Data.Word (Word8)
import WasmLeb128 (LEB128Error(..), decodeUnsigned)
import WasmTypes hiding (description)

description :: String
description = "Haskell WebAssembly binary parser for the core module format"

wasmMagic :: ByteString
wasmMagic = BS.pack [0x00, 0x61, 0x73, 0x6D]

wasmVersion :: ByteString
wasmVersion = BS.pack [0x01, 0x00, 0x00, 0x00]

data WasmParseError = WasmParseError
    { wasmParseErrorMessage :: String
    , wasmParseErrorOffset :: Int
    }
    deriving (Eq, Show)

parseModule :: ByteString -> Either WasmParseError WasmModule
parseModule bytes
    | BS.length bytes < 8 =
        Left
            WasmParseError
                { wasmParseErrorMessage = "truncated WASM header"
                , wasmParseErrorOffset = 0
                }
    | BS.take 4 bytes /= wasmMagic =
        Left
            WasmParseError
                { wasmParseErrorMessage = "bad WASM magic bytes"
                , wasmParseErrorOffset = 0
                }
    | BS.take 4 (BS.drop 4 bytes) /= wasmVersion =
        Left
            WasmParseError
                { wasmParseErrorMessage = "unsupported WASM version"
                , wasmParseErrorOffset = 4
                }
    | otherwise = parseSections bytes 8 0 emptyModule

parseSections :: ByteString -> Int -> Word8 -> WasmModule -> Either WasmParseError WasmModule
parseSections bytes offset lastNonCustomId moduleValue
    | offset >= BS.length bytes = Right moduleValue
    | otherwise = do
        sectionId <- readByte bytes offset
        (sectionSize, payloadStart) <- readUnsigned bytes (offset + 1)
        let payloadEnd = payloadStart + fromIntegral sectionSize
        ensure bytes payloadStart (fromIntegral sectionSize)
        let payload = BS.take (fromIntegral sectionSize) (BS.drop payloadStart bytes)
            nextLastNonCustomId =
                if sectionId == 0
                    then lastNonCustomId
                    else sectionId
        if sectionId /= 0 && sectionId < lastNonCustomId
            then
                Left
                    WasmParseError
                        { wasmParseErrorMessage = "section ordering violation"
                        , wasmParseErrorOffset = payloadStart
                        }
            else do
                nextModule <- parseSection sectionId payload moduleValue
                parseSections bytes payloadEnd nextLastNonCustomId nextModule

parseSection :: Word8 -> ByteString -> WasmModule -> Either WasmParseError WasmModule
parseSection sectionId payload moduleValue =
    case sectionId of
        0 -> do
            customSection <- parseCustomSection payload
            Right moduleValue {wasmCustomSections = wasmCustomSections moduleValue ++ [customSection]}
        1 -> do
            sectionValues <- parseTypeSection payload
            Right moduleValue {wasmTypes = sectionValues}
        2 -> do
            sectionValues <- parseImportSection payload
            Right moduleValue {wasmImports = sectionValues}
        3 -> do
            sectionValues <- parseFunctionSection payload
            Right moduleValue {wasmFunctions = sectionValues}
        4 -> do
            sectionValues <- parseTableSection payload
            Right moduleValue {wasmTables = sectionValues}
        5 -> do
            sectionValues <- parseMemorySection payload
            Right moduleValue {wasmMemories = sectionValues}
        6 -> do
            sectionValues <- parseGlobalSection payload
            Right moduleValue {wasmGlobals = sectionValues}
        7 -> do
            sectionValues <- parseExportSection payload
            Right moduleValue {wasmExports = sectionValues}
        8 -> do
            startIndex <- parseStartSection payload
            Right moduleValue {wasmStart = Just startIndex}
        9 -> do
            sectionValues <- parseElementSection payload
            Right moduleValue {wasmElements = sectionValues}
        10 -> do
            sectionValues <- parseCodeSection payload
            Right moduleValue {wasmCode = sectionValues}
        11 -> do
            sectionValues <- parseDataSection payload
            Right moduleValue {wasmDataSegments = sectionValues}
        _ ->
            Left
                WasmParseError
                    { wasmParseErrorMessage = "unknown section id"
                    , wasmParseErrorOffset = 0
                    }

parseTypeSection :: ByteString -> Either WasmParseError [FuncType]
parseTypeSection payload = do
    (count, offset) <- readUnsigned payload 0
    parseManyPayload payload offset count parseFuncType

parseFuncType :: ByteString -> Int -> Either WasmParseError (FuncType, Int)
parseFuncType payload offset = do
    marker <- readByte payload offset
    if marker /= 0x60
        then
            Left
                WasmParseError
                    { wasmParseErrorMessage = "expected functype marker"
                    , wasmParseErrorOffset = offset
                    }
        else do
            (paramCount, afterParamCount) <- readUnsigned payload (offset + 1)
            (params, afterParams) <- parseValueTypes payload afterParamCount paramCount
            (resultCount, afterResultCount) <- readUnsigned payload afterParams
            (results, afterResults) <- parseValueTypes payload afterResultCount resultCount
            Right (FuncType params results, afterResults)

parseImportSection :: ByteString -> Either WasmParseError [Import]
parseImportSection payload = do
    (count, offset) <- readUnsigned payload 0
    parseManyPayload payload offset count parseImport

parseImport :: ByteString -> Int -> Either WasmParseError (Import, Int)
parseImport payload offset = do
    (moduleName, afterModuleName) <- readName payload offset
    (entityName, afterEntityName) <- readName payload afterModuleName
    kindByte <- readByte payload afterEntityName
    case externalKindFromByte kindByte of
        Nothing ->
            Left
                WasmParseError
                    { wasmParseErrorMessage = "unknown import kind"
                    , wasmParseErrorOffset = afterEntityName
                    }
        Just kind ->
            case kind of
                ExternalFunction -> do
                    (typeIndex, nextOffset) <- readUnsigned payload (afterEntityName + 1)
                    Right
                        ( Import moduleName entityName kind (ImportFunctionType typeIndex)
                        , nextOffset
                        )
                ExternalTable -> do
                    elementType <- readByte payload (afterEntityName + 1)
                    (limits, nextOffset) <- readLimits payload (afterEntityName + 2)
                    Right
                        ( Import moduleName entityName kind (ImportTableType (TableType elementType limits))
                        , nextOffset
                        )
                ExternalMemory -> do
                    (limits, nextOffset) <- readLimits payload (afterEntityName + 1)
                    Right
                        ( Import moduleName entityName kind (ImportMemoryType (MemoryType limits))
                        , nextOffset
                        )
                ExternalGlobal -> do
                    valueTypeByte <- readByte payload (afterEntityName + 1)
                    mutableByte <- readByte payload (afterEntityName + 2)
                    case valueTypeFromByte valueTypeByte of
                        Nothing ->
                            Left
                                WasmParseError
                                    { wasmParseErrorMessage = "unknown global value type"
                                    , wasmParseErrorOffset = afterEntityName + 1
                                    }
                        Just valueType ->
                            Right
                                ( Import moduleName entityName kind (ImportGlobalType (GlobalType valueType (mutableByte /= 0)))
                                , afterEntityName + 3
                                )

parseFunctionSection :: ByteString -> Either WasmParseError [Integer]
parseFunctionSection payload = parseVectorU32 payload

parseTableSection :: ByteString -> Either WasmParseError [TableType]
parseTableSection payload = do
    (count, offset) <- readUnsigned payload 0
    parseManyPayload payload offset count parseTableType

parseTableType :: ByteString -> Int -> Either WasmParseError (TableType, Int)
parseTableType payload offset = do
    elementType <- readByte payload offset
    (limits, nextOffset) <- readLimits payload (offset + 1)
    Right (TableType elementType limits, nextOffset)

parseMemorySection :: ByteString -> Either WasmParseError [MemoryType]
parseMemorySection payload = do
    (count, offset) <- readUnsigned payload 0
    parseManyPayload payload offset count parseMemoryType

parseMemoryType :: ByteString -> Int -> Either WasmParseError (MemoryType, Int)
parseMemoryType payload offset = do
    (limits, nextOffset) <- readLimits payload offset
    Right (MemoryType limits, nextOffset)

parseGlobalSection :: ByteString -> Either WasmParseError [Global]
parseGlobalSection payload = do
    (count, offset) <- readUnsigned payload 0
    parseManyPayload payload offset count parseGlobal

parseGlobal :: ByteString -> Int -> Either WasmParseError (Global, Int)
parseGlobal payload offset = do
    valueTypeByte <- readByte payload offset
    mutableByte <- readByte payload (offset + 1)
    case valueTypeFromByte valueTypeByte of
        Nothing ->
            Left
                WasmParseError
                    { wasmParseErrorMessage = "unknown global type"
                    , wasmParseErrorOffset = offset
                    }
        Just valueType -> do
            (exprBytes, nextOffset) <- readExpr payload (offset + 2)
            Right (Global (GlobalType valueType (mutableByte /= 0)) exprBytes, nextOffset)

parseExportSection :: ByteString -> Either WasmParseError [Export]
parseExportSection payload = do
    (count, offset) <- readUnsigned payload 0
    parseManyPayload payload offset count parseExport

parseExport :: ByteString -> Int -> Either WasmParseError (Export, Int)
parseExport payload offset = do
    (name, afterName) <- readName payload offset
    kindByte <- readByte payload afterName
    (indexValue, nextOffset) <- readUnsigned payload (afterName + 1)
    case externalKindFromByte kindByte of
        Nothing ->
            Left
                WasmParseError
                    { wasmParseErrorMessage = "unknown export kind"
                    , wasmParseErrorOffset = afterName
                    }
        Just kind -> Right (Export name kind indexValue, nextOffset)

parseStartSection :: ByteString -> Either WasmParseError Integer
parseStartSection payload = fmap fst (readUnsigned payload 0)

parseElementSection :: ByteString -> Either WasmParseError [Element]
parseElementSection payload = do
    (count, offset) <- readUnsigned payload 0
    parseManyPayload payload offset count parseElement

parseElement :: ByteString -> Int -> Either WasmParseError (Element, Int)
parseElement payload offset = do
    (tableIndex, afterIndex) <- readUnsigned payload offset
    (offsetExpr, afterExpr) <- readExpr payload afterIndex
    (functionCount, afterCount) <- readUnsigned payload afterExpr
    (functionIndices, nextOffset) <- parseManyPayloadWithOffset payload afterCount functionCount readUnsignedAt
    Right
        ( Element tableIndex offsetExpr functionIndices
        , nextOffset
        )
  where
    readUnsignedAt currentPayload currentOffset = readUnsigned currentPayload currentOffset

parseCodeSection :: ByteString -> Either WasmParseError [FunctionBody]
parseCodeSection payload = do
    (count, offset) <- readUnsigned payload 0
    parseManyPayload payload offset count parseFunctionBody

parseFunctionBody :: ByteString -> Int -> Either WasmParseError (FunctionBody, Int)
parseFunctionBody payload offset = do
    (bodySize, afterSize) <- readUnsigned payload offset
    let bodyStart = afterSize
        bodyEnd = bodyStart + fromIntegral bodySize
    ensure payload bodyStart (fromIntegral bodySize)
    (localGroupCount, afterLocalCount) <- readUnsigned payload bodyStart
    (localsList, codeStart) <- parseLocalGroups payload afterLocalCount localGroupCount
    let codeBytes = BS.take (bodyEnd - codeStart) (BS.drop codeStart payload)
    Right (FunctionBody localsList codeBytes, bodyEnd)

parseDataSection :: ByteString -> Either WasmParseError [DataSegment]
parseDataSection payload = do
    (count, offset) <- readUnsigned payload 0
    parseManyPayload payload offset count parseDataSegment

parseDataSegment :: ByteString -> Int -> Either WasmParseError (DataSegment, Int)
parseDataSegment payload offset = do
    (memoryIndex, afterMemoryIndex) <- readUnsigned payload offset
    (offsetExpr, afterExpr) <- readExpr payload afterMemoryIndex
    (byteCount, afterCount) <- readUnsigned payload afterExpr
    ensure payload afterCount (fromIntegral byteCount)
    let bytesValue = BS.take (fromIntegral byteCount) (BS.drop afterCount payload)
    Right
        ( DataSegment memoryIndex offsetExpr bytesValue
        , afterCount + fromIntegral byteCount
        )

parseCustomSection :: ByteString -> Either WasmParseError CustomSection
parseCustomSection payload = do
    (name, offset) <- readName payload 0
    Right (CustomSection name (BS.drop offset payload))

parseLocalGroups :: ByteString -> Int -> Integer -> Either WasmParseError ([ValueType], Int)
parseLocalGroups payload offset count
    | count <= 0 = Right ([], offset)
    | otherwise = do
        (runCount, afterRunCount) <- readUnsigned payload offset
        valueTypeByte <- readByte payload afterRunCount
        case valueTypeFromByte valueTypeByte of
            Nothing ->
                Left
                    WasmParseError
                        { wasmParseErrorMessage = "unknown local value type"
                        , wasmParseErrorOffset = afterRunCount
                        }
            Just valueType -> do
                (rest, finalOffset) <- parseLocalGroups payload (afterRunCount + 1) (count - 1)
                Right (replicate (fromIntegral runCount) valueType ++ rest, finalOffset)

parseValueTypes :: ByteString -> Int -> Integer -> Either WasmParseError ([ValueType], Int)
parseValueTypes payload offset count
    | count <= 0 = Right ([], offset)
    | otherwise = do
        byte <- readByte payload offset
        case valueTypeFromByte byte of
            Nothing ->
                Left
                    WasmParseError
                        { wasmParseErrorMessage = "unknown value type"
                        , wasmParseErrorOffset = offset
                        }
            Just valueType -> do
                (rest, finalOffset) <- parseValueTypes payload (offset + 1) (count - 1)
                Right (valueType : rest, finalOffset)

parseVectorU32 :: ByteString -> Either WasmParseError [Integer]
parseVectorU32 payload = do
    (count, offset) <- readUnsigned payload 0
    parseManyPayload payload offset count readUnsignedAt
  where
    readUnsignedAt currentPayload currentOffset = readUnsigned currentPayload currentOffset

parseManyPayload :: ByteString -> Int -> Integer -> (ByteString -> Int -> Either WasmParseError (a, Int)) -> Either WasmParseError [a]
parseManyPayload payload offset count parser =
    fmap fst (parseManyPayloadWithOffset payload offset count parser)

parseManyPayloadWithOffset :: ByteString -> Int -> Integer -> (ByteString -> Int -> Either WasmParseError (a, Int)) -> Either WasmParseError ([a], Int)
parseManyPayloadWithOffset payload offset count parser
    | count <= 0 = Right ([], offset)
    | otherwise = do
        (item, nextOffset) <- parser payload offset
        (rest, finalOffset) <- parseManyPayloadWithOffset payload nextOffset (count - 1) parser
        Right (item : rest, finalOffset)

readUnsigned :: ByteString -> Int -> Either WasmParseError (Integer, Int)
readUnsigned payload offset =
    case decodeUnsigned payload offset of
        Left err ->
            Left
                WasmParseError
                    { wasmParseErrorMessage = leb128ErrorMessage err
                    , wasmParseErrorOffset = leb128ErrorOffset err
                    }
        Right (value, consumed) -> Right (value, offset + consumed)

readByte :: ByteString -> Int -> Either WasmParseError Word8
readByte payload offset = do
    ensure payload offset 1
    Right (BS.index payload offset)

readName :: ByteString -> Int -> Either WasmParseError (String, Int)
readName payload offset = do
    (nameLength, afterLength) <- readUnsigned payload offset
    ensure payload afterLength (fromIntegral nameLength)
    let nameBytes = BS.take (fromIntegral nameLength) (BS.drop afterLength payload)
    Right (map (toEnum . fromIntegral) (BS.unpack nameBytes), afterLength + fromIntegral nameLength)

readLimits :: ByteString -> Int -> Either WasmParseError (Limits, Int)
readLimits payload offset = do
    flags <- readByte payload offset
    (minimumValue, afterMinimum) <- readUnsigned payload (offset + 1)
    if flags Bits..&. 0x01 == 0
        then Right (Limits minimumValue Nothing, afterMinimum)
        else do
            (maximumValue, nextOffset) <- readUnsigned payload afterMinimum
            Right (Limits minimumValue (Just maximumValue), nextOffset)

readExpr :: ByteString -> Int -> Either WasmParseError (ByteString, Int)
readExpr payload offset = go offset
  where
    go current
        | current >= BS.length payload =
            Left
                WasmParseError
                    { wasmParseErrorMessage = "unterminated init expression"
                    , wasmParseErrorOffset = offset
                    }
        | BS.index payload current == 0x0B =
            let nextOffset = current + 1
             in Right (BS.take (nextOffset - offset) (BS.drop offset payload), nextOffset)
        | otherwise = go (current + 1)

ensure :: ByteString -> Int -> Int -> Either WasmParseError ()
ensure payload offset needed
    | offset + needed <= BS.length payload = Right ()
    | otherwise =
        Left
            WasmParseError
                { wasmParseErrorMessage = "unexpected end of input"
                , wasmParseErrorOffset = offset
                }
