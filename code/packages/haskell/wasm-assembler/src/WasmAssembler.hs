module WasmAssembler
    ( description
    , WasmAssemblerError(..)
    , parseAssembly
    , assembleToBytes
    ) where

import qualified Data.ByteString as BS
import Data.ByteString (ByteString)
import qualified Data.List as List
import qualified Data.Map.Strict as Map
import Data.Map.Strict (Map)
import WasmLeb128 (encodeSigned, encodeUnsigned)
import WasmModuleEncoder (encodeModule)
import WasmOpcodes hiding (description)
import WasmTypes hiding (description)

description :: String
description = "Haskell text assembler for a focused WebAssembly subset"

data WasmAssemblerError = WasmAssemblerError
    { wasmAssemblerErrorMessage :: String
    }
    deriving (Eq, Show)

data AssemblyState = AssemblyState
    { assemblyTypes :: Map Int FuncType
    , assemblyMemories :: Map Int MemoryType
    , assemblyFunctions :: Map Int Integer
    , assemblyBodies :: Map Int FunctionBody
    , assemblyImports :: [Import]
    , assemblyExports :: [Export]
    , assemblyDataSegments :: [DataSegment]
    , currentFunction :: Maybe CurrentFunction
    }

data CurrentFunction = CurrentFunction
    { currentFunctionIndex :: Int
    , currentFunctionTypeIndex :: Integer
    , currentFunctionLocals :: [ValueType]
    , currentFunctionBytes :: ByteString
    }

parseAssembly :: String -> Either WasmAssemblerError WasmModule
parseAssembly text = finalize =<< foldM handleLine emptyState (lines text)
  where
    emptyState =
        AssemblyState
            { assemblyTypes = Map.empty
            , assemblyMemories = Map.empty
            , assemblyFunctions = Map.empty
            , assemblyBodies = Map.empty
            , assemblyImports = []
            , assemblyExports = []
            , assemblyDataSegments = []
            , currentFunction = Nothing
            }

    handleLine state rawLine =
        let line = trim rawLine
         in if null line || "#" `List.isPrefixOf` line || ";" `List.isPrefixOf` line
                then Right state
                else
                    case currentFunction state of
                        Just current
                            | not ("." `List.isPrefixOf` line) -> do
                                instructionBytes <- assembleInstruction line
                                Right state {currentFunction = Just current {currentFunctionBytes = currentFunctionBytes current <> instructionBytes}}
                        _ -> parseDirective state line

    finalize state =
        case currentFunction state of
            Just _ -> Left (WasmAssemblerError "unterminated .func block")
            Nothing ->
                Right
                    emptyModule
                        { wasmTypes = map snd (Map.toAscList (assemblyTypes state))
                        , wasmImports = assemblyImports state
                        , wasmMemories = map snd (Map.toAscList (assemblyMemories state))
                        , wasmFunctions = map snd (Map.toAscList (assemblyFunctions state))
                        , wasmCode = map snd (Map.toAscList (assemblyBodies state))
                        , wasmExports = assemblyExports state
                        , wasmDataSegments = reverse (assemblyDataSegments state)
                        }

parseDirective :: AssemblyState -> String -> Either WasmAssemblerError AssemblyState
parseDirective state line =
    case words line of
        [".type", indexText, paramsPart, resultsPart] ->
            Right
                state
                    { assemblyTypes =
                        Map.insert
                            (read indexText)
                            (FuncType (parseTypes (valuePart paramsPart)) (parseTypes (valuePart resultsPart)))
                            (assemblyTypes state)
                    }
        [".memory", indexText, minPart, maxPart] ->
            Right
                state
                    { assemblyMemories =
                        Map.insert
                            (read indexText)
                            (MemoryType (Limits (read (valuePart minPart)) (parseOptionalInteger (valuePart maxPart))))
                            (assemblyMemories state)
                    }
        ".import" : kindText : moduleName : entityName : metadata ->
            Right
                state
                    { assemblyImports =
                        assemblyImports state
                            ++ [ Import
                                    { importModuleName = moduleName
                                    , importName = entityName
                                    , importKind = parseExternalKind kindText
                                    , importTypeInfo = parseImportTypeInfo kindText (metadataMap metadata)
                                    }
                               ]
                    }
        [".export", kindText, label, indexText] ->
            Right
                state
                    { assemblyExports =
                        assemblyExports state
                            ++ [Export label (parseExternalKind kindText) (read indexText)]
                    }
        [".func", indexText, typePart, localsPart] ->
            Right
                state
                    { currentFunction =
                        Just
                            CurrentFunction
                                { currentFunctionIndex = read indexText
                                , currentFunctionTypeIndex = read (valuePart typePart)
                                , currentFunctionLocals = parseTypes (valuePart localsPart)
                                , currentFunctionBytes = BS.empty
                                }
                    }
        [".endfunc"] ->
            case currentFunction state of
                Nothing -> Left (WasmAssemblerError ".endfunc without active function")
                Just current ->
                    Right
                        state
                            { assemblyFunctions = Map.insert (currentFunctionIndex current) (currentFunctionTypeIndex current) (assemblyFunctions state)
                            , assemblyBodies =
                                Map.insert
                                    (currentFunctionIndex current)
                                    (FunctionBody (currentFunctionLocals current) (currentFunctionBytes current))
                                    (assemblyBodies state)
                            , currentFunction = Nothing
                            }
        [".data", memoryText, offsetPart, bytesPart] ->
            Right
                state
                    { assemblyDataSegments =
                        DataSegment
                            { dataSegmentMemoryIndex = read memoryText
                            , dataSegmentOffsetExpr = constExpr (read (valuePart offsetPart))
                            , dataSegmentBytes = parseByteList (valuePart bytesPart)
                            }
                            : assemblyDataSegments state
                    }
        _ -> Left (WasmAssemblerError ("unrecognized assembly line: " ++ line))

assembleInstruction :: String -> Either WasmAssemblerError ByteString
assembleInstruction line =
    case words line of
        [] -> Right BS.empty
        mnemonic : arguments ->
            case opcodeByName mnemonic of
                Nothing -> Left (WasmAssemblerError ("unknown instruction: " ++ mnemonic))
                Just opcodeInfo ->
                    BS.cons (opcodeByte opcodeInfo) <$> encodeImmediates opcodeInfo arguments

encodeImmediates :: OpcodeInfo -> [String] -> Either WasmAssemblerError ByteString
encodeImmediates opcodeInfo arguments =
    case opcodeImmediates opcodeInfo of
        [] -> Right BS.empty
        ["i32"] ->
            case arguments of
                [value] -> Right (encodeSigned (read value))
                _ -> Left (WasmAssemblerError "i32.const expects one argument")
        ["i64"] ->
            case arguments of
                [value] -> Right (encodeSigned (read value))
                _ -> Left (WasmAssemblerError "i64.const expects one argument")
        ["blocktype"] ->
            case arguments of
                [value] -> Right (encodeBlockType value)
                _ -> Left (WasmAssemblerError "block/loop/if expects one blocktype")
        ["memarg"] ->
            let kv = metadataMap arguments
             in Right (encodeUnsigned (read (lookupRequired "align" kv)) <> encodeUnsigned (read (lookupRequired "offset" kv)))
        ["labelidx"] ->
            case arguments of
                [value] -> Right (encodeUnsigned (read value))
                _ -> Left (WasmAssemblerError "branch instruction expects one label index")
        ["funcidx"] ->
            case arguments of
                [value] -> Right (encodeUnsigned (read value))
                _ -> Left (WasmAssemblerError "call expects one function index")
        ["localidx"] ->
            case arguments of
                [value] -> Right (encodeUnsigned (read value))
                _ -> Left (WasmAssemblerError "local instruction expects one local index")
        ["globalidx"] ->
            case arguments of
                [value] -> Right (encodeUnsigned (read value))
                _ -> Left (WasmAssemblerError "global instruction expects one global index")
        ["memidx"] -> Right (encodeUnsigned 0)
        ["typeidx", "tableidx"] ->
            case arguments of
                [typeIndex, tableIndex] -> Right (encodeUnsigned (read typeIndex) <> encodeUnsigned (read tableIndex))
                [typeIndex] -> Right (encodeUnsigned (read typeIndex) <> encodeUnsigned 0)
                _ -> Left (WasmAssemblerError "call_indirect expects typeidx and optional tableidx")
        _ -> Left (WasmAssemblerError "unsupported immediate form")

assembleToBytes :: String -> Either WasmAssemblerError ByteString
assembleToBytes text = do
    moduleValue <- parseAssembly text
    case encodeModule moduleValue of
        Left err -> Left (WasmAssemblerError (show err))
        Right bytesValue -> Right bytesValue

parseTypes :: String -> [ValueType]
parseTypes "none" = []
parseTypes text = map parseValueType (splitOn ',' text)

parseValueType :: String -> ValueType
parseValueType text =
    case text of
        "i32" -> I32
        "i64" -> I64
        "f32" -> F32
        "f64" -> F64
        _ -> error ("unknown value type: " ++ text)

parseExternalKind :: String -> ExternalKind
parseExternalKind text =
    case text of
        "function" -> ExternalFunction
        "table" -> ExternalTable
        "memory" -> ExternalMemory
        "global" -> ExternalGlobal
        _ -> error ("unknown external kind: " ++ text)

parseImportTypeInfo :: String -> Map String String -> ImportTypeInfo
parseImportTypeInfo kindText kv =
    case kindText of
        "function" -> ImportFunctionType (read (lookupRequired "type" kv))
        "memory" -> ImportMemoryType (MemoryType (Limits (read (lookupRequired "min" kv)) (parseOptionalInteger (lookupRequired "max" kv))))
        "table" ->
            ImportTableType
                (TableType
                    { tableTypeElementType = 0x70
                    , tableTypeLimits = Limits (read (lookupRequired "min" kv)) (parseOptionalInteger (lookupRequired "max" kv))
                    }
                )
        "global" ->
            ImportGlobalType
                (GlobalType
                    { globalValueType = parseValueType (lookupRequired "type" kv)
                    , globalMutable = lookupRequired "mutable" kv == "true"
                    }
                )
        _ -> error "unsupported import kind"

metadataMap :: [String] -> Map String String
metadataMap =
    Map.fromList . map splitEntry
  where
    splitEntry entry =
        let (key, rest) = break (== '=') entry
         in (key, drop 1 rest)

lookupRequired :: String -> Map String String -> String
lookupRequired key kv =
    case Map.lookup key kv of
        Just value -> value
        Nothing -> error ("missing required metadata key: " ++ key)

parseOptionalInteger :: String -> Maybe Integer
parseOptionalInteger "none" = Nothing
parseOptionalInteger text = Just (read text)

parseByteList :: String -> ByteString
parseByteList "none" = BS.empty
parseByteList text = BS.pack (map (fromIntegral . readHexByte) (splitOn ',' text))

readHexByte :: String -> Int
readHexByte text =
    case reads ("0x" ++ text) of
        [(value, "")] -> value
        _ -> error ("invalid hex byte: " ++ text)

encodeBlockType :: String -> ByteString
encodeBlockType text =
    case text of
        "void" -> BS.pack [0x40]
        "i32" -> BS.pack [valueTypeByte I32]
        "i64" -> BS.pack [valueTypeByte I64]
        "f32" -> BS.pack [valueTypeByte F32]
        "f64" -> BS.pack [valueTypeByte F64]
        _ -> encodeSigned (read text)

constExpr :: Integer -> ByteString
constExpr value = BS.pack [0x41] <> encodeSigned value <> BS.pack [0x0B]

valuePart :: String -> String
valuePart entry = drop 1 (dropWhile (/= '=') entry)

splitOn :: Char -> String -> [String]
splitOn separator text =
    case break (== separator) text of
        (prefix, []) -> [prefix]
        (prefix, _ : rest) -> prefix : splitOn separator rest

trim :: String -> String
trim = dropWhile (== ' ') . reverse . dropWhile (== ' ') . reverse

foldM :: (a -> b -> Either e a) -> a -> [b] -> Either e a
foldM _ acc [] = Right acc
foldM step acc (item : rest) = do
    next <- step acc item
    foldM step next rest
