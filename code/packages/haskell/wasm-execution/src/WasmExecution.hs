module WasmExecution
    ( description
    , TrapError(..)
    , WasmValue(..)
    , defaultValue
    , LinearMemory
    , newLinearMemory
    , memorySize
    , readBytes
    , writeBytes
    , loadI32
    , loadI32_8u
    , storeI32
    , storeI32_8
    , Table
    , newTable
    , tableSize
    , tableGet
    , tableSet
    , HostFunction
    , WasmExecutionEngine(..)
    , evaluateConstExpr
    , callFunction
    ) where

import qualified Data.Bits as Bits
import qualified Data.ByteString as BS
import Data.ByteString (ByteString)
import Data.IORef
import Data.Int (Int32, Int64)
import qualified Data.List as List
import Data.Word (Word8, Word32)
import WasmLeb128 (decodeSigned, decodeUnsigned, encodeUnsigned)
import WasmTypes hiding (description)

description :: String
description = "Haskell execution layer for a focused subset of WebAssembly"

data TrapError = TrapError
    { trapErrorMessage :: String
    }
    deriving (Eq, Show)

data WasmValue
    = WasmI32 Int32
    | WasmI64 Int64
    | WasmF32 Float
    | WasmF64 Double
    deriving (Eq, Show)

defaultValue :: ValueType -> WasmValue
defaultValue valueType =
    case valueType of
        I32 -> WasmI32 0
        I64 -> WasmI64 0
        F32 -> WasmF32 0
        F64 -> WasmF64 0

data LinearMemory = LinearMemory
    { linearMemoryBytes :: IORef ByteString
    , linearMemoryMaxPages :: Maybe Integer
    }

data Table = Table
    { tableEntries :: IORef [Maybe Integer]
    , tableMaxEntries :: Maybe Integer
    }

type HostFunction = [WasmValue] -> IO [WasmValue]

data WasmExecutionEngine = WasmExecutionEngine
    { engineMemory :: Maybe LinearMemory
    , engineTables :: [Table]
    , engineGlobals :: IORef [WasmValue]
    , engineGlobalTypes :: [GlobalType]
    , engineFuncTypes :: [FuncType]
    , engineFuncBodies :: [Maybe FunctionBody]
    , engineHostFunctions :: [Maybe HostFunction]
    }

pageSize :: Int
pageSize = 65536

newLinearMemory :: Integer -> Maybe Integer -> IO LinearMemory
newLinearMemory minimumPages maximumPages = do
    bytesRef <- newIORef (BS.replicate (fromIntegral minimumPages * pageSize) 0)
    pure LinearMemory {linearMemoryBytes = bytesRef, linearMemoryMaxPages = maximumPages}

memorySize :: LinearMemory -> IO Integer
memorySize memory = do
    bytesValue <- readIORef (linearMemoryBytes memory)
    pure (toInteger (BS.length bytesValue `div` pageSize))

readBytes :: LinearMemory -> Int -> Int -> IO ByteString
readBytes memory offset lengthValue = do
    bytesValue <- readIORef (linearMemoryBytes memory)
    pure (BS.take lengthValue (BS.drop offset bytesValue))

writeBytes :: LinearMemory -> Int -> ByteString -> IO ()
writeBytes memory offset chunk = do
    bytesValue <- readIORef (linearMemoryBytes memory)
    let prefix = BS.take offset bytesValue
        suffix = BS.drop (offset + BS.length chunk) bytesValue
    writeIORef (linearMemoryBytes memory) (prefix <> chunk <> suffix)

loadI32_8u :: LinearMemory -> Int -> IO Int
loadI32_8u memory offset = do
    bytesValue <- readBytes memory offset 1
    pure
        (if BS.null bytesValue
            then 0
            else fromIntegral (BS.head bytesValue))

loadI32 :: LinearMemory -> Int -> IO Int32
loadI32 memory offset = do
    bytesValue <- readBytes memory offset 4
    let padded = bytesValue <> BS.replicate (max 0 (4 - BS.length bytesValue)) 0
        b0 = fromIntegral (BS.index padded 0) :: Word32
        b1 = fromIntegral (BS.index padded 1) :: Word32
        b2 = fromIntegral (BS.index padded 2) :: Word32
        b3 = fromIntegral (BS.index padded 3) :: Word32
        wordValue = b0 + Bits.shiftL b1 8 + Bits.shiftL b2 16 + Bits.shiftL b3 24
    pure (fromIntegral wordValue)

storeI32 :: LinearMemory -> Int -> Int32 -> IO ()
storeI32 memory offset value =
    writeBytes memory offset (int32ToBytes value)

storeI32_8 :: LinearMemory -> Int -> Int32 -> IO ()
storeI32_8 memory offset value =
    writeBytes memory offset (BS.singleton (fromIntegral value))

newTable :: Integer -> Maybe Integer -> IO Table
newTable minimumEntries maximumEntries = do
    entriesRef <- newIORef (replicate (fromIntegral minimumEntries) Nothing)
    pure Table {tableEntries = entriesRef, tableMaxEntries = maximumEntries}

tableSize :: Table -> IO Int
tableSize table = length <$> readIORef (tableEntries table)

tableGet :: Table -> Int -> IO (Maybe Integer)
tableGet table indexValue = do
    entriesValue <- readIORef (tableEntries table)
    pure
        ( if indexValue >= 0 && indexValue < length entriesValue
            then entriesValue !! indexValue
            else Nothing
        )

tableSet :: Table -> Int -> Integer -> IO ()
tableSet table indexValue functionIndex = do
    entriesValue <- readIORef (tableEntries table)
    let needed = indexValue + 1 - length entriesValue
        extendedEntries =
            if needed > 0
                then entriesValue ++ replicate needed Nothing
                else entriesValue
        updatedEntries =
            take indexValue extendedEntries
                ++ [Just functionIndex]
                ++ drop (indexValue + 1) extendedEntries
    writeIORef (tableEntries table) updatedEntries

evaluateConstExpr :: ByteString -> [WasmValue] -> Either TrapError WasmValue
evaluateConstExpr expr globalsValue =
    case BS.uncons expr of
        Nothing -> Left (TrapError "empty const expression")
        Just (opcode, rest) ->
            case opcode of
                0x41 -> parseSignedConst (WasmI32 . fromIntegral) rest
                0x42 -> parseSignedConst (WasmI64 . fromIntegral) rest
                0x23 ->
                    case decodeUnsigned expr 1 of
                        Left _ -> Left (TrapError "invalid global.get immediate")
                        Right (indexValue, consumed)
                            | 1 + consumed >= BS.length expr || BS.index expr (1 + consumed) /= 0x0B ->
                                Left (TrapError "unterminated global.get const expression")
                            | fromIntegral indexValue < length globalsValue ->
                                Right (globalsValue !! fromIntegral indexValue)
                            | otherwise ->
                                Left (TrapError "global.get index out of range")
                _ -> Left (TrapError "unsupported const expression opcode")
  where
    parseSignedConst constructor rest =
        case decodeSigned expr 1 of
            Left _ -> Left (TrapError "invalid signed const immediate")
            Right (value, consumed)
                | 1 + consumed >= BS.length expr || BS.index expr (1 + consumed) /= 0x0B ->
                    Left (TrapError "unterminated const expression")
                | otherwise -> Right (constructor value)

callFunction :: WasmExecutionEngine -> Int -> [WasmValue] -> IO (Either TrapError [WasmValue])
callFunction engine functionIndex args
    | functionIndex < 0 || functionIndex >= length (engineFuncTypes engine) =
        pure (Left (TrapError "function index out of range"))
    | otherwise =
        case engineHostFunctions engine !! functionIndex of
            Just hostFunction -> Right <$> hostFunction args
            Nothing ->
                case engineFuncBodies engine !! functionIndex of
                    Nothing -> pure (Left (TrapError "missing function body"))
                    Just body ->
                        executeFunctionBody
                            engine
                            (engineFuncTypes engine !! functionIndex)
                            body
                            args

executeFunctionBody :: WasmExecutionEngine -> FuncType -> FunctionBody -> [WasmValue] -> IO (Either TrapError [WasmValue])
executeFunctionBody engine functionType body args = go 0 0 [] initialLocals
  where
    codeBytes = functionBodyCode body
    resultCount = length (funcTypeResults functionType)
    initialLocals = args ++ map defaultValue (functionBodyLocals body)

    go pc blockDepth stack locals
        | pc >= BS.length codeBytes = pure (Right (reverse (take resultCount stack)))
        | otherwise =
            case BS.index codeBytes pc of
                0x02 -> continueWithBlock pc blockDepth stack locals
                0x03 -> continueWithBlock pc blockDepth stack locals
                0x04 ->
                    case popValue stack of
                        Left err -> pure (Left err)
                        Right (_, restStack) -> continueWithBlock pc blockDepth restStack locals
                0x0B ->
                    if blockDepth > 0
                        then go (pc + 1) (blockDepth - 1) stack locals
                        else pure (Right (reverse (take resultCount stack)))
                0x0F -> pure (Right (reverse (take resultCount stack)))
                0x10 -> do
                    let immediateBytes = BS.drop (pc + 1) codeBytes
                    case decodeUnsigned immediateBytes 0 of
                        Left _ -> pure (Left (TrapError "invalid call immediate"))
                        Right (targetIndex, consumed) ->
                            if fromIntegral targetIndex >= length (engineFuncTypes engine)
                                then pure (Left (TrapError "call target out of range"))
                                else do
                                    let targetType = engineFuncTypes engine !! fromIntegral targetIndex
                                    case popArguments (length (funcTypeParams targetType)) stack of
                                        Left err -> pure (Left err)
                                        Right (callArgs, restStack) -> do
                                            result <- callFunction engine (fromIntegral targetIndex) callArgs
                                            case result of
                                                Left err -> pure (Left err)
                                                Right values -> go (pc + 1 + consumed) blockDepth (List.foldl' (flip (:)) restStack values) locals
                0x20 -> decodeIndexAndContinue pc $ \indexValue nextPc ->
                    if indexValue < length locals
                        then go nextPc blockDepth (locals !! indexValue : stack) locals
                        else pure (Left (TrapError "local.get index out of range"))
                0x21 -> decodeIndexAndContinue pc $ \indexValue nextPc ->
                    case popValue stack of
                        Left err -> pure (Left err)
                        Right (value, restStack) ->
                            if indexValue < length locals
                                then go nextPc blockDepth restStack (replaceAt indexValue value locals)
                                else pure (Left (TrapError "local.set index out of range"))
                0x22 -> decodeIndexAndContinue pc $ \indexValue nextPc ->
                    case popValue stack of
                        Left err -> pure (Left err)
                        Right (value, restStack) ->
                            if indexValue < length locals
                                then go nextPc blockDepth (value : restStack) (replaceAt indexValue value locals)
                                else pure (Left (TrapError "local.tee index out of range"))
                0x23 -> decodeIndexAndContinue pc $ \indexValue nextPc -> do
                    globalsValue <- readIORef (engineGlobals engine)
                    if indexValue < length globalsValue
                        then go nextPc blockDepth (globalsValue !! indexValue : stack) locals
                        else pure (Left (TrapError "global.get index out of range"))
                0x24 -> decodeIndexAndContinue pc $ \indexValue nextPc ->
                    case popValue stack of
                        Left err -> pure (Left err)
                        Right (value, restStack) ->
                            if indexValue < length (engineGlobalTypes engine)
                                then
                                    if globalMutable (engineGlobalTypes engine !! indexValue)
                                        then do
                                            globalsValue <- readIORef (engineGlobals engine)
                                            writeIORef (engineGlobals engine) (replaceAt indexValue value globalsValue)
                                            go nextPc blockDepth restStack locals
                                        else pure (Left (TrapError "global.set on immutable global"))
                                else pure (Left (TrapError "global.set index out of range"))
                0x28 -> doMemoryLoad pc stack locals blockDepth loadI32
                0x2D -> doMemoryLoad pc stack locals blockDepth (\memory offset -> fromIntegral <$> loadI32_8u memory offset)
                0x36 -> doMemoryStore pc stack locals blockDepth storeI32Value
                0x3A -> doMemoryStore pc stack locals blockDepth storeI32_8Value
                0x3F -> skipReservedImmediate pc $ \nextPc -> do
                    case engineMemory engine of
                        Nothing -> pure (Left (TrapError "memory.size requires linear memory"))
                        Just memory -> do
                            currentPages <- memorySize memory
                            go nextPc blockDepth (WasmI32 (fromIntegral currentPages) : stack) locals
                0x40 -> skipReservedImmediate pc $ \nextPc ->
                    case popValue stack of
                        Left err -> pure (Left err)
                        Right (WasmI32 pagesValue, restStack) ->
                            case engineMemory engine of
                                Nothing -> pure (Left (TrapError "memory.grow requires linear memory"))
                                Just memory -> do
                                    previousPages <- memorySize memory
                                    bytesValue <- readIORef (linearMemoryBytes memory)
                                    let growth = max 0 (fromIntegral pagesValue)
                                        newPages = previousPages + toInteger growth
                                    case linearMemoryMaxPages memory of
                                        Just maximumPages | newPages > maximumPages ->
                                            go nextPc blockDepth (WasmI32 (-1) : restStack) locals
                                        _ -> do
                                            writeIORef (linearMemoryBytes memory) (bytesValue <> BS.replicate (growth * pageSize) 0)
                                            go nextPc blockDepth (WasmI32 (fromIntegral previousPages) : restStack) locals
                        Right _ -> pure (Left (TrapError "memory.grow expects an i32 page count"))
                0x41 -> doSignedConst pc stack locals blockDepth (WasmI32 . fromIntegral)
                0x42 -> doSignedConst pc stack locals blockDepth (WasmI64 . fromIntegral)
                0x45 -> unaryI32Op pc blockDepth stack locals (\value -> if value == 0 then 1 else 0)
                0x6A -> binaryI32Op pc blockDepth stack locals (+)
                0x6B -> binaryI32Op pc blockDepth stack locals (-)
                0x6C -> binaryI32Op pc blockDepth stack locals (*)
                0x71 -> binaryI32Op pc blockDepth stack locals (Bits..&.)
                _ -> pure (Left (TrapError ("unsupported opcode: " ++ show (BS.index codeBytes pc))))

    continueWithBlock pc blockDepth stack locals =
        case readBlockImmediate (BS.drop (pc + 1) codeBytes) of
            Left err -> pure (Left err)
            Right consumed -> go (pc + 1 + consumed) (blockDepth + 1) stack locals

    decodeIndexAndContinue pc continuation =
        case decodeUnsigned (BS.drop (pc + 1) codeBytes) 0 of
            Left _ -> pure (Left (TrapError "invalid index immediate"))
            Right (indexValue, consumed) -> continuation (fromIntegral indexValue) (pc + 1 + consumed)

    skipReservedImmediate pc continuation =
        case decodeUnsigned (BS.drop (pc + 1) codeBytes) 0 of
            Left _ -> pure (Left (TrapError "invalid reserved memory immediate"))
            Right (_, consumed) -> continuation (pc + 1 + consumed)

    doSignedConst pc stack locals blockDepth constructor =
        case decodeSigned (BS.drop (pc + 1) codeBytes) 0 of
            Left _ -> pure (Left (TrapError "invalid signed immediate"))
            Right (value, consumed) -> go (pc + 1 + consumed) blockDepth (constructor value : stack) locals

    doMemoryLoad pc stack locals blockDepth loader =
        case readMemArg (BS.drop (pc + 1) codeBytes) of
            Left err -> pure (Left err)
            Right ((_, memOffset), consumed) ->
                case popValue stack of
                    Left err -> pure (Left err)
                    Right (WasmI32 baseAddress, restStack) ->
                        case engineMemory engine of
                            Nothing -> pure (Left (TrapError "memory load requires linear memory"))
                            Just memory -> do
                                loadedValue <- loader memory (fromIntegral baseAddress + memOffset)
                                go (pc + 1 + consumed) blockDepth (WasmI32 loadedValue : restStack) locals
                    Right _ -> pure (Left (TrapError "memory load expects an i32 address"))

    doMemoryStore pc stack locals blockDepth storeAction =
        case readMemArg (BS.drop (pc + 1) codeBytes) of
            Left err -> pure (Left err)
            Right ((_, memOffset), consumed) ->
                case popValue stack of
                    Left err -> pure (Left err)
                    Right (value, restAfterValue) ->
                        case popValue restAfterValue of
                            Left err -> pure (Left err)
                            Right (WasmI32 baseAddress, restStack) ->
                                case engineMemory engine of
                                    Nothing -> pure (Left (TrapError "memory store requires linear memory"))
                                    Just memory -> do
                                        storeAction memory (fromIntegral baseAddress + memOffset) value
                                        go (pc + 1 + consumed) blockDepth restStack locals
                            Right _ -> pure (Left (TrapError "memory store expects an i32 address"))

    unaryI32Op pc blockDepth stack locals operation =
        case popValue stack of
            Left err -> pure (Left err)
            Right (WasmI32 value, restStack) ->
                go (pc + 1) blockDepth (WasmI32 (operation value) : restStack) locals
            Right _ -> pure (Left (TrapError "numeric instruction expects an i32 operand"))

    binaryI32Op pc blockDepth stack locals operation =
        case popValue stack of
            Left err -> pure (Left err)
            Right (WasmI32 rhs, restAfterRhs) ->
                case popValue restAfterRhs of
                    Left err -> pure (Left err)
                    Right (WasmI32 lhs, restStack) ->
                        go (pc + 1) blockDepth (WasmI32 (lhs `operation` rhs) : restStack) locals
                    Right _ -> pure (Left (TrapError "numeric instruction expects i32 operands"))
            Right _ -> pure (Left (TrapError "numeric instruction expects i32 operands"))

storeI32Value :: LinearMemory -> Int -> WasmValue -> IO ()
storeI32Value memory offset wasmValue =
    case wasmValue of
        WasmI32 value -> storeI32 memory offset value
        WasmI64 value -> storeI32 memory offset (fromIntegral value)
        WasmF32 value -> storeI32 memory offset (truncate value)
        WasmF64 value -> storeI32 memory offset (truncate value)

storeI32_8Value :: LinearMemory -> Int -> WasmValue -> IO ()
storeI32_8Value memory offset wasmValue =
    case wasmValue of
        WasmI32 value -> storeI32_8 memory offset value
        WasmI64 value -> storeI32_8 memory offset (fromIntegral value)
        WasmF32 value -> storeI32_8 memory offset (truncate value)
        WasmF64 value -> storeI32_8 memory offset (truncate value)

readBlockImmediate :: ByteString -> Either TrapError Int
readBlockImmediate bytesValue =
    case BS.uncons bytesValue of
        Nothing -> Left (TrapError "missing blocktype immediate")
        Just (0x40, _) -> Right 1
        Just (byte, _)
            | byte `elem` map valueTypeByte [I32, I64, F32, F64] -> Right 1
        _ ->
            case decodeSigned bytesValue 0 of
                Left _ -> Left (TrapError "invalid blocktype immediate")
                Right (_, consumed) -> Right consumed

readMemArg :: ByteString -> Either TrapError ((Int, Int), Int)
readMemArg bytesValue =
    case decodeUnsigned bytesValue 0 of
        Left _ -> Left (TrapError "invalid memarg alignment")
        Right (alignment, consumedA) ->
            case decodeUnsigned bytesValue consumedA of
                Left _ -> Left (TrapError "invalid memarg offset")
                Right (offsetValue, consumedB) ->
                    Right ((fromIntegral alignment, fromIntegral offsetValue), consumedA + consumedB)

popArguments :: Int -> [WasmValue] -> Either TrapError ([WasmValue], [WasmValue])
popArguments count stack = go count stack []
  where
    go remaining currentStack acc
        | remaining <= 0 = Right (acc, currentStack)
        | otherwise =
            case popValue currentStack of
                Left err -> Left err
                Right (value, restStack) -> go (remaining - 1) restStack (value : acc)

popValue :: [WasmValue] -> Either TrapError (WasmValue, [WasmValue])
popValue stack =
    case stack of
        [] -> Left (TrapError "stack underflow")
        value : rest -> Right (value, rest)

replaceAt :: Int -> a -> [a] -> [a]
replaceAt indexValue newValue values =
    take indexValue values ++ [newValue] ++ drop (indexValue + 1) values

int32ToBytes :: Int32 -> ByteString
int32ToBytes value =
    let wordValue = fromIntegral value :: Word32
     in BS.pack
            [ fromIntegral wordValue
            , fromIntegral (Bits.shiftR wordValue 8)
            , fromIntegral (Bits.shiftR wordValue 16)
            , fromIntegral (Bits.shiftR wordValue 24)
            ]
