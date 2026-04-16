module WasmValidator
    ( description
    , ValidationErrorKind(..)
    , ValidationError(..)
    , ValidatedModule(..)
    , validateModule
    ) where

import qualified Data.List as List
import WasmTypes hiding (description)

description :: String
description = "Haskell structural validator for core WebAssembly modules"

data ValidationErrorKind
    = InvalidTypeIndex
    | InvalidFunctionIndex
    | InvalidTableIndex
    | InvalidMemoryIndex
    | InvalidGlobalIndex
    | MultipleMemories
    | MultipleTables
    | MemoryLimitExceeded
    | MemoryLimitOrder
    | TableLimitOrder
    | DuplicateExportName
    | ExportIndexOutOfRange
    | StartFunctionBadType
    deriving (Eq, Show)

data ValidationError = ValidationError
    { validationErrorKind :: ValidationErrorKind
    , validationErrorMessage :: String
    }
    deriving (Eq, Show)

data ValidatedModule = ValidatedModule
    { validatedModule :: WasmModule
    , validatedFuncTypes :: [FuncType]
    , validatedFuncLocals :: [[ValueType]]
    }
    deriving (Eq, Show)

maxMemoryPages :: Integer
maxMemoryPages = 65536

validateModule :: WasmModule -> Either ValidationError ValidatedModule
validateModule moduleValue = do
    let functionImportTypes =
            [ typeIndex
            | Import _ _ ExternalFunction (ImportFunctionType typeIndex) <- wasmImports moduleValue
            ]
        importedTableTypes =
            [ tableType
            | Import _ _ ExternalTable (ImportTableType tableType) <- wasmImports moduleValue
            ]
        importedMemoryTypes =
            [ memoryType
            | Import _ _ ExternalMemory (ImportMemoryType memoryType) <- wasmImports moduleValue
            ]
        importedGlobalTypes =
            [ globalTypeValue
            | Import _ _ ExternalGlobal (ImportGlobalType globalTypeValue) <- wasmImports moduleValue
            ]
    ensure (length (wasmFunctions moduleValue) == length (wasmCode moduleValue)) InvalidFunctionIndex "function and code section lengths must match"
    mapM_ (ensureTypeIndex moduleValue) functionImportTypes
    mapM_ (ensureTypeIndex moduleValue) (wasmFunctions moduleValue)
    let allTableTypes = importedTableTypes ++ wasmTables moduleValue
        allMemoryTypes = importedMemoryTypes ++ wasmMemories moduleValue
        allGlobalTypes = importedGlobalTypes ++ map globalType (wasmGlobals moduleValue)
        localFuncTypes = map (\typeIndex -> wasmTypes moduleValue !! fromIntegral typeIndex) (wasmFunctions moduleValue)
        importedFuncTypes = map (\typeIndex -> wasmTypes moduleValue !! fromIntegral typeIndex) functionImportTypes
        allFuncTypes = importedFuncTypes ++ localFuncTypes
    ensure (length allTableTypes <= 1) MultipleTables "WASM 1.0 allows at most one table"
    ensure (length allMemoryTypes <= 1) MultipleMemories "WASM 1.0 allows at most one memory"
    mapM_ validateMemoryType allMemoryTypes
    mapM_ validateTableType allTableTypes
    validateExports moduleValue allFuncTypes allTableTypes allMemoryTypes allGlobalTypes
    validateStart moduleValue allFuncTypes
    mapM_ (validateElement allFuncTypes) (wasmElements moduleValue)
    let localLocals =
            zipWith
                (\functionType body -> funcTypeParams functionType ++ functionBodyLocals body)
                localFuncTypes
                (wasmCode moduleValue)
    Right
        ValidatedModule
            { validatedModule = moduleValue
            , validatedFuncTypes = allFuncTypes
            , validatedFuncLocals = localLocals
            }

validateMemoryType :: MemoryType -> Either ValidationError ()
validateMemoryType memoryType =
    case memoryTypeLimits memoryType of
        Limits minimumValue maximumValue ->
            case maximumValue of
                Just maxValue
                    | maxValue > maxMemoryPages ->
                        Left (ValidationError MemoryLimitExceeded "memory maximum exceeds WASM 1.0 limits")
                    | minimumValue > maxValue ->
                        Left (ValidationError MemoryLimitOrder "memory minimum exceeds memory maximum")
                _ -> Right ()

validateTableType :: TableType -> Either ValidationError ()
validateTableType tableType =
    case tableTypeLimits tableType of
        Limits minimumValue (Just maximumValue)
            | minimumValue > maximumValue ->
                Left (ValidationError TableLimitOrder "table minimum exceeds table maximum")
        _ -> Right ()

validateExports :: WasmModule -> [FuncType] -> [TableType] -> [MemoryType] -> [GlobalType] -> Either ValidationError ()
validateExports moduleValue functionTypes tableTypes memoryTypes globalTypes = do
    let names = map exportName (wasmExports moduleValue)
    ensure (length names == length (List.nub names)) DuplicateExportName "duplicate export names are not allowed"
    mapM_ validateExport (wasmExports moduleValue)
  where
    validateExport exportValue =
        let upperBound =
                case exportKind exportValue of
                    ExternalFunction -> length functionTypes
                    ExternalTable -> length tableTypes
                    ExternalMemory -> length memoryTypes
                    ExternalGlobal -> length globalTypes
         in ensureIndex (exportIndex exportValue) upperBound ExportIndexOutOfRange "export index is out of range"

validateStart :: WasmModule -> [FuncType] -> Either ValidationError ()
validateStart moduleValue functionTypes =
    case wasmStart moduleValue of
        Nothing -> Right ()
        Just startIndex -> do
            ensureIndex startIndex (length functionTypes) InvalidFunctionIndex "start function index is out of range"
            let startType = functionTypes !! fromIntegral startIndex
            ensure
                (null (funcTypeParams startType) && null (funcTypeResults startType))
                StartFunctionBadType
                "start function must have type () -> ()"

validateElement :: [FuncType] -> Element -> Either ValidationError ()
validateElement functionTypes elementValue =
    mapM_ (\functionIndex -> ensureIndex functionIndex (length functionTypes) InvalidFunctionIndex "element references an invalid function index") (elementFunctionIndices elementValue)

ensureTypeIndex :: WasmModule -> Integer -> Either ValidationError ()
ensureTypeIndex moduleValue typeIndex =
    ensureIndex typeIndex (length (wasmTypes moduleValue)) InvalidTypeIndex "type index is out of range"

ensureIndex :: Integer -> Int -> ValidationErrorKind -> String -> Either ValidationError ()
ensureIndex indexValue lengthValue kind message =
    ensure (indexValue >= 0 && fromIntegral indexValue < lengthValue) kind message

ensure :: Bool -> ValidationErrorKind -> String -> Either ValidationError ()
ensure predicate kind message =
    if predicate
        then Right ()
        else Left (ValidationError kind message)
