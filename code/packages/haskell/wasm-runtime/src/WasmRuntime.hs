module WasmRuntime
    ( description
    , WasmRuntime(..)
    , WasmInstance(..)
    , WasiConfig(..)
    , WasiHost
    , newRuntime
    , newWasiHost
    , instantiateModule
    , callExportedFunction
    , loadAndRun
    ) where

import qualified Data.ByteString as BS
import Data.ByteString (ByteString)
import Data.IORef
import qualified Data.List as List
import WasmExecution hiding (description)
import WasmModuleParser (parseModule)
import WasmTypes hiding (description)
import WasmValidator (ValidatedModule(..), validateModule)

description :: String
description = "Haskell WebAssembly runtime built from parser, validator, and execution layers"

data WasmRuntime = WasmRuntime
    { runtimeHost :: Maybe WasiHost
    }

data WasmInstance = WasmInstance
    { instanceModule :: WasmModule
    , instanceMemory :: Maybe LinearMemory
    , instanceTables :: [Table]
    , instanceGlobals :: IORef [WasmValue]
    , instanceGlobalTypes :: [GlobalType]
    , instanceFuncTypes :: [FuncType]
    , instanceFuncBodies :: [Maybe FunctionBody]
    , instanceHostFunctions :: [Maybe HostFunction]
    , instanceExports :: [Export]
    }

data WasiConfig = WasiConfig
    { wasiStdout :: String -> IO ()
    }

data WasiHost = WasiHost
    { wasiHostConfig :: WasiConfig
    , wasiHostMemory :: IORef (Maybe LinearMemory)
    }

newRuntime :: Maybe WasiHost -> WasmRuntime
newRuntime host = WasmRuntime {runtimeHost = host}

newWasiHost :: WasiConfig -> IO WasiHost
newWasiHost config = do
    memoryRef <- newIORef Nothing
    pure WasiHost {wasiHostConfig = config, wasiHostMemory = memoryRef}

instantiateModule :: WasmRuntime -> WasmModule -> IO (Either String WasmInstance)
instantiateModule runtime moduleValue =
    case validateModule moduleValue of
        Left err -> pure (Left (show err))
        Right validated -> do
            let host = runtimeHost runtime
                importsValue = wasmImports moduleValue
            importedFunctions <- mapM (resolveImportedFunction host) importsValue
            let hostErrors = [message | Left message <- importedFunctions]
            if not (null hostErrors)
                then
                    case hostErrors of
                        firstError : _ -> pure (Left firstError)
                        [] -> pure (Left "unknown host resolution failure")
                else do
                    importedMemory <- resolveImportedMemory host importsValue
                    importedTables <- resolveImportedTables host importsValue
                    importedGlobals <- resolveImportedGlobals host importsValue
                    memoryValue <-
                        case (importedMemory, wasmMemories moduleValue) of
                            (Just memory, _) -> pure (Just memory)
                            (Nothing, memoryType : _) -> Just <$> newLinearMemory (limitsMin (memoryTypeLimits memoryType)) (limitsMax (memoryTypeLimits memoryType))
                            _ -> pure Nothing
                    localTables <- mapM (\tableType -> newTable (limitsMin (tableTypeLimits tableType)) (limitsMax (tableTypeLimits tableType))) (wasmTables moduleValue)
                    globalsList <- initializeGlobals importedGlobals (wasmGlobals moduleValue)
                    globalsRef <- newIORef globalsList
                    applyDataSegments memoryValue globalsList (wasmDataSegments moduleValue)
                    applyElementSegments (importedTables ++ localTables) globalsList (wasmElements moduleValue)
                    case (host, memoryValue) of
                        (Just wasiHost, Just memory) -> writeIORef (wasiHostMemory wasiHost) (Just memory)
                        _ -> pure ()
                    let instanceValue =
                            WasmInstance
                                { instanceModule = moduleValue
                                , instanceMemory = memoryValue
                                , instanceTables = importedTables ++ localTables
                                , instanceGlobals = globalsRef
                                , instanceGlobalTypes = map globalType (wasmGlobals moduleValue)
                                , instanceFuncTypes = validatedFuncTypes validated
                                , instanceFuncBodies = importedFunctionBodies importedFunctions ++ map Just (wasmCode moduleValue)
                                , instanceHostFunctions = importedHostFunctions importedFunctions ++ replicate (length (wasmCode moduleValue)) Nothing
                                , instanceExports = wasmExports moduleValue
                                }
                    case wasmStart moduleValue of
                        Nothing -> pure (Right instanceValue)
                        Just startIndex -> do
                            startResult <- callFunction (toEngine instanceValue) (fromIntegral startIndex) []
                            pure
                                (case startResult of
                                    Left err -> Left (show err)
                                    Right _ -> Right instanceValue
                                )

callExportedFunction :: WasmRuntime -> WasmInstance -> String -> [WasmValue] -> IO (Either TrapError [WasmValue])
callExportedFunction _ instanceValue exportLabel args =
    case List.find (\exportValue -> exportName exportValue == exportLabel) (instanceExports instanceValue) of
        Nothing -> pure (Left (TrapError ("export \"" ++ exportLabel ++ "\" not found")))
        Just exportValue ->
            case exportKind exportValue of
                ExternalFunction -> callFunction (toEngine instanceValue) (fromIntegral (exportIndex exportValue)) args
                _ -> pure (Left (TrapError ("export \"" ++ exportLabel ++ "\" is not a function")))

loadAndRun :: WasmRuntime -> ByteString -> String -> [WasmValue] -> IO (Either String [WasmValue])
loadAndRun runtime wasmBytes exportLabel args =
    case parseModule wasmBytes of
        Left err -> pure (Left (show err))
        Right moduleValue -> do
            instanceResult <- instantiateModule runtime moduleValue
            case instanceResult of
                Left message -> pure (Left message)
                Right instanceValue -> do
                    result <- callExportedFunction runtime instanceValue exportLabel args
                    pure (either (Left . show) Right result)

resolveImportedFunction :: Maybe WasiHost -> Import -> IO (Either String (Maybe FunctionBody, Maybe HostFunction))
resolveImportedFunction host importValue =
    case importKind importValue of
        ExternalFunction ->
            case resolveHostFunction host (importModuleName importValue) (importName importValue) of
                Nothing -> pure (Left ("unresolved host function import: " ++ importModuleName importValue ++ "." ++ importName importValue))
                Just hostFunction -> pure (Right (Nothing, Just hostFunction))
        _ -> pure (Right (Nothing, Nothing))

resolveImportedMemory :: Maybe WasiHost -> [Import] -> IO (Maybe LinearMemory)
resolveImportedMemory _ _ = pure Nothing

resolveImportedTables :: Maybe WasiHost -> [Import] -> IO [Table]
resolveImportedTables _ importsValue =
    mapM
        (\importValue ->
            case importTypeInfo importValue of
                ImportTableType tableType -> newTable (limitsMin (tableTypeLimits tableType)) (limitsMax (tableTypeLimits tableType))
                _ -> newTable 0 Nothing
        )
        [importValue | importValue <- importsValue, importKind importValue == ExternalTable]

resolveImportedGlobals :: Maybe WasiHost -> [Import] -> IO [WasmValue]
resolveImportedGlobals _ importsValue =
    pure
        [ defaultValue (globalValueType globalTypeValue)
        | Import _ _ ExternalGlobal (ImportGlobalType globalTypeValue) <- importsValue
        ]

importedFunctionBodies :: [Either String (Maybe FunctionBody, Maybe HostFunction)] -> [Maybe FunctionBody]
importedFunctionBodies importsValue = [body | Right (body, _) <- importsValue]

importedHostFunctions :: [Either String (Maybe FunctionBody, Maybe HostFunction)] -> [Maybe HostFunction]
importedHostFunctions importsValue = [hostFunction | Right (_, hostFunction) <- importsValue]

initializeGlobals :: [WasmValue] -> [Global] -> IO [WasmValue]
initializeGlobals importedGlobalsValue globalsValue = pure (go importedGlobalsValue globalsValue)
  where
    go current [] = current
    go current (globalValue : rest) =
        case evaluateConstExpr (globalInitExpr globalValue) current of
            Left _ -> current
            Right value -> go (current ++ [value]) rest

applyDataSegments :: Maybe LinearMemory -> [WasmValue] -> [DataSegment] -> IO ()
applyDataSegments memoryValue globalsValue segments =
    case memoryValue of
        Nothing -> pure ()
        Just memory ->
            mapM_
                (\segment ->
                    case evaluateConstExpr (dataSegmentOffsetExpr segment) globalsValue of
                        Right (WasmI32 offsetValue) -> writeBytes memory (fromIntegral offsetValue) (dataSegmentBytes segment)
                        _ -> pure ()
                )
                segments

applyElementSegments :: [Table] -> [WasmValue] -> [Element] -> IO ()
applyElementSegments tablesValue globalsValue elementsValue =
    mapM_
        (\elementValue ->
            case evaluateConstExpr (elementOffsetExpr elementValue) globalsValue of
                Right (WasmI32 offsetValue)
                    | fromIntegral (elementTableIndex elementValue) < length tablesValue ->
                        mapM_
                            (\(entryOffset, functionIndex) ->
                                tableSet
                                    (tablesValue !! fromIntegral (elementTableIndex elementValue))
                                    (fromIntegral offsetValue + entryOffset)
                                    functionIndex
                            )
                            (zip [0 ..] (elementFunctionIndices elementValue))
                _ -> pure ()
        )
        elementsValue

resolveHostFunction :: Maybe WasiHost -> String -> String -> Maybe HostFunction
resolveHostFunction Nothing _ _ = Nothing
resolveHostFunction (Just host) moduleName name
    | moduleName == "wasi_snapshot_preview1" && name == "fd_write" =
        Just (fdWrite host)
    | otherwise = Nothing

fdWrite :: WasiHost -> HostFunction
fdWrite host args =
    case args of
        [WasmI32 _, WasmI32 iovsPtr, WasmI32 iovsLen, WasmI32 nwrittenPtr] -> do
            memoryValue <- readIORef (wasiHostMemory host)
            case memoryValue of
                Nothing -> pure [WasmI32 1]
                Just memory -> do
                    chunks <- mapM (readIovec memory (fromIntegral iovsPtr)) [0 .. fromIntegral iovsLen - 1]
                    let bytesValue = BS.concat chunks
                    wasiStdout (wasiHostConfig host) (map (toEnum . fromIntegral) (BS.unpack bytesValue))
                    storeI32 memory (fromIntegral nwrittenPtr) (fromIntegral (BS.length bytesValue))
                    pure [WasmI32 0]
        _ -> pure [WasmI32 1]

readIovec :: LinearMemory -> Int -> Int -> IO ByteString
readIovec memory basePtr indexValue = do
    let entryOffset = basePtr + indexValue * 8
    ptrValue <- loadI32 memory entryOffset
    lenValue <- loadI32 memory (entryOffset + 4)
    readBytes memory (fromIntegral ptrValue) (fromIntegral lenValue)

toEngine :: WasmInstance -> WasmExecutionEngine
toEngine instanceValue =
    WasmExecutionEngine
        { engineMemory = instanceMemory instanceValue
        , engineTables = instanceTables instanceValue
        , engineGlobals = instanceGlobals instanceValue
        , engineGlobalTypes = instanceGlobalTypes instanceValue
        , engineFuncTypes = instanceFuncTypes instanceValue
        , engineFuncBodies = instanceFuncBodies instanceValue
        , engineHostFunctions = instanceHostFunctions instanceValue
        }
