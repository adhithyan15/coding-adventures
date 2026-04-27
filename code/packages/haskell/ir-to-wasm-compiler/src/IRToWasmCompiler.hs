module IRToWasmCompiler
    ( description
    , FunctionSignature(..)
    , LoweringError(..)
    , compileProgram
    ) where

import qualified Data.ByteString as BS
import Data.ByteString (ByteString)
import qualified Data.List as List
import qualified Data.Map.Strict as Map
import Data.Map.Strict (Map)
import CompilerIR hiding (description)
import WasmLeb128 (encodeSigned, encodeUnsigned)
import WasmTypes hiding (description)

description :: String
description = "Haskell compiler IR to WebAssembly lowerer"

data FunctionSignature = FunctionSignature
    { signatureLabel :: String
    , signatureParamCount :: Int
    , signatureExportName :: Maybe String
    }
    deriving (Eq, Ord, Show)

data LoweringError = LoweringError
    { loweringErrorMessage :: String
    }
    deriving (Eq, Show)

data FunctionIR = FunctionIR
    { functionLabel :: String
    , functionInstructions :: [IrInstruction]
    , functionSignature :: FunctionSignature
    }
    deriving (Eq, Show)

data WasiImport = WasiImport
    { wasiSyscallNumber :: Integer
    , wasiImportName :: String
    , wasiImportType :: FuncType
    }
    deriving (Eq, Show)

data LoweringContext = LoweringContext
    { contextFunctionIndices :: Map String Integer
    , contextAllFunctions :: [FunctionIR]
    , contextDataOffsets :: Map String Integer
    , contextWasiImports :: Map Integer Integer
    , contextWasiScratchBase :: Maybe Integer
    }
    deriving (Eq, Show)

regSysArg :: Int
regSysArg = 4

regSysScratch :: Int
regSysScratch = 7

syscallWrite :: Integer
syscallWrite = 1

syscallRead :: Integer
syscallRead = 2

syscallExit :: Integer
syscallExit = 10

wasiScratchSize :: Integer
wasiScratchSize = 16

compileProgram :: IrProgram -> [FunctionSignature] -> Either LoweringError WasmModule
compileProgram program extraSignatures = do
    functions <- splitFunctions program signatures
    importsValue <- collectWasiImports program
    let importedTypes = map wasiImportType importsValue
        localTypes = map functionType functions
        functionTypeStart = toInteger (length importedTypes)
        functionIndices =
            Map.fromList
                [ (functionLabel functionValue, toInteger indexValue + toInteger (length importsValue))
                | (functionValue, indexValue) <- zip functions [0 :: Int ..]
                ]
        wasiImportIndices =
            Map.fromList
                [ (wasiSyscallNumber importValue, toInteger indexValue)
                | (importValue, indexValue) <- zip importsValue [0 :: Int ..]
                ]
        dataOffsets = layoutData (irDataDecls program)
        scratchBase =
            if needsWasiScratch program
                then Just (alignUp (totalDataSize (irDataDecls program)) 4)
                else Nothing
        memoryByteCount =
            maybe (totalDataSize (irDataDecls program)) (+ wasiScratchSize) scratchBase
        memoryExports =
            [Export "memory" ExternalMemory 0 | needsMemory program || scratchBase /= Nothing]
        functionExports =
            [ Export exportLabel ExternalFunction indexValue
            | functionValue <- functions
            , Just exportLabel <- [signatureExportName (functionSignature functionValue)]
            , Just indexValue <- [Map.lookup (functionLabel functionValue) functionIndices]
            ]
        context =
            LoweringContext
                { contextFunctionIndices = functionIndices
                , contextAllFunctions = functions
                , contextDataOffsets = dataOffsets
                , contextWasiImports = wasiImportIndices
                , contextWasiScratchBase = scratchBase
                }
    bodies <- mapM (lowerFunction context) functions
    Right
        emptyModule
            { wasmTypes = importedTypes ++ localTypes
            , wasmImports =
                [ Import "wasi_snapshot_preview1" (wasiImportName importValue) ExternalFunction (ImportFunctionType (toInteger indexValue))
                | (importValue, indexValue) <- zip importsValue [0 :: Int ..]
                ]
            , wasmFunctions = [functionTypeStart .. functionTypeStart + toInteger (length functions) - 1]
            , wasmMemories = [MemoryType (Limits (pagesFor memoryByteCount) Nothing) | needsMemory program || scratchBase /= Nothing]
            , wasmExports = memoryExports ++ functionExports
            , wasmCode = bodies
            , wasmDataSegments =
                [ DataSegment 0 (constExpr offset) (BS.replicate sizeValue (fromIntegral initValue))
                | decl <- irDataDecls program
                , let offset = Map.findWithDefault 0 (irDataLabel decl) dataOffsets
                , let sizeValue = irDataSize decl
                , let initValue = irDataInit decl
                ]
            }
  where
    signatures = Map.union (Map.fromList [(signatureLabel sig, sig) | sig <- extraSignatures]) (inferSignatures program)

inferSignatures :: IrProgram -> Map String FunctionSignature
inferSignatures program =
    Map.fromList
        [ (labelName, FunctionSignature labelName 0 (Just labelName))
        | IrInstruction Label [LabelRef labelName] _ <- irInstructions program
        , labelName == irEntryLabel program
        ]

splitFunctions :: IrProgram -> Map String FunctionSignature -> Either LoweringError [FunctionIR]
splitFunctions program signatures =
    Right (finish (foldl step ([], Nothing) (irInstructions program)))
  where
    finish (done, Nothing) = done
    finish (done, Just current) = done ++ [current]
    isFunctionLabel labelName = Map.member labelName signatures || labelName == irEntryLabel program || "_fn_" `List.isPrefixOf` labelName
    step (done, current) inst =
        case inst of
            IrInstruction Label [LabelRef labelName] _
                | isFunctionLabel labelName ->
                    let signatureValue =
                            Map.findWithDefault
                                (FunctionSignature labelName 0 (Just labelName))
                                labelName
                                signatures
                        nextFunction = FunctionIR labelName [] signatureValue
                     in (maybe done (\functionValue -> done ++ [functionValue]) current, Just nextFunction)
            _ ->
                case current of
                    Nothing -> (done, current)
                    Just functionValue ->
                        (done, Just functionValue {functionInstructions = functionInstructions functionValue ++ [inst]})

functionType :: FunctionIR -> FuncType
functionType functionValue =
    let params = replicate (signatureParamCount (functionSignature functionValue)) I32
        results =
            if functionLabel functionValue == "_start"
                then []
                else [I32]
     in FuncType params results

lowerFunction :: LoweringContext -> FunctionIR -> Either LoweringError FunctionBody
lowerFunction context functionValue = do
    body <- lowerRegion context functionValue labels 0 (length instructions)
    let localCount = maximum (0 : map instructionMaxRegister instructions ++ syscallRegisters) + 1
    Right (FunctionBody (replicate localCount I32) (body <> BS.pack [0x0B]))
  where
    instructions = functionInstructions functionValue
    labels =
        Map.fromList
            [ (labelName, indexValue)
            | (inst, indexValue) <- zip instructions [0 :: Int ..]
            , IrInstruction Label [LabelRef labelName] _ <- [inst]
            ]
    syscallRegisters =
        [regSysScratch | any ((== Syscall) . irOpcode) instructions]

lowerRegion :: LoweringContext -> FunctionIR -> Map String Int -> Int -> Int -> Either LoweringError ByteString
lowerRegion context functionValue labels startIndex endIndex =
    go startIndex BS.empty
  where
    instructions = functionInstructions functionValue
    go indexValue bytesValue
        | indexValue >= endIndex = Right bytesValue
        | otherwise =
            case instructions !! indexValue of
                IrInstruction Comment _ _ -> go (indexValue + 1) bytesValue
                IrInstruction Label [LabelRef labelName] _
                    | isLoopStart labelName -> do
                        (loopBytes, nextIndex) <- lowerLoop context functionValue labels indexValue endIndex labelName
                        go nextIndex (bytesValue <> loopBytes)
                    | otherwise -> go (indexValue + 1) bytesValue
                inst
                    | irOpcode inst == Jump || irOpcode inst == BranchZ || irOpcode inst == BranchNz ->
                        Left (LoweringError ("unexpected unstructured control flow in " ++ functionLabel functionValue))
                    | otherwise -> do
                        chunk <- lowerInstruction context functionValue inst
                        go (indexValue + 1) (bytesValue <> chunk)

lowerLoop :: LoweringContext -> FunctionIR -> Map String Int -> Int -> Int -> String -> Either LoweringError (ByteString, Int)
lowerLoop context functionValue labels labelIndex regionEnd startLabel = do
    endIndex <- requireLabel labels endLabel
    branchIndex <- findFirstBranchToLabel instructions (labelIndex + 1) endIndex endLabel
    backedgeIndex <- findLastJumpToLabel instructions (branchIndex + 1) endIndex startLabel
    condReg <- branchRegister (instructions !! branchIndex)
    preludeBytes <- lowerRegion context functionValue labels (labelIndex + 1) branchIndex
    bodyBytes <- lowerRegion context functionValue labels (branchIndex + 1) backedgeIndex
    let branchOp = irOpcode (instructions !! branchIndex)
        conditionBytes =
            localGet (localIndex functionValue condReg)
                <> if branchOp == BranchZ then BS.pack [0x45] else BS.empty
        loopBytes =
            BS.pack [0x02, 0x40, 0x03, 0x40]
                <> preludeBytes
                <> conditionBytes
                <> BS.pack [0x0D]
                <> encodeUnsigned 1
                <> bodyBytes
                <> BS.pack [0x0C]
                <> encodeUnsigned 0
                <> BS.pack [0x0B, 0x0B]
    if endIndex >= regionEnd
        then Left (LoweringError ("loop end label " ++ endLabel ++ " escapes the current region"))
        else Right (loopBytes, endIndex + 1)
  where
    instructions = functionInstructions functionValue
    endLabel = matchingLoopEnd startLabel

lowerInstruction :: LoweringContext -> FunctionIR -> IrInstruction -> Either LoweringError ByteString
lowerInstruction context functionValue inst =
    case (irOpcode inst, irOperands inst) of
        (Comment, _) -> Right BS.empty
        (Nop, []) -> Right (BS.pack [0x01])
        (LoadImm, [Register dst, Immediate value]) ->
            Right (i32Const value <> localSet (localIndex functionValue dst))
        (LoadAddr, [Register dst, LabelRef labelName]) ->
            case Map.lookup labelName (contextDataOffsets context) of
                Nothing -> Left (LoweringError ("unknown data label " ++ labelName))
                Just offset -> Right (i32Const offset <> localSet (localIndex functionValue dst))
        (LoadByte, [Register dst, Register base, Register offset]) ->
            Right (address functionValue base offset <> BS.pack [0x2D] <> memarg 0 0 <> localSet (localIndex functionValue dst))
        (StoreByte, [Register src, Register base, Register offset]) ->
            Right (address functionValue base offset <> localGet (localIndex functionValue src) <> BS.pack [0x3A] <> memarg 0 0)
        (Add, [Register dst, Register lhs, Register rhs]) ->
            Right (localGet (localIndex functionValue lhs) <> localGet (localIndex functionValue rhs) <> BS.pack [0x6A] <> localSet (localIndex functionValue dst))
        (AddImm, [Register dst, Register src, Immediate value]) ->
            Right (localGet (localIndex functionValue src) <> i32Const value <> BS.pack [0x6A] <> localSet (localIndex functionValue dst))
        (Sub, [Register dst, Register lhs, Register rhs]) ->
            Right (localGet (localIndex functionValue lhs) <> localGet (localIndex functionValue rhs) <> BS.pack [0x6B] <> localSet (localIndex functionValue dst))
        (And, [Register dst, Register lhs, Register rhs]) ->
            Right (localGet (localIndex functionValue lhs) <> localGet (localIndex functionValue rhs) <> BS.pack [0x71] <> localSet (localIndex functionValue dst))
        (AndImm, [Register dst, Register src, Immediate value]) ->
            Right (localGet (localIndex functionValue src) <> i32Const value <> BS.pack [0x71] <> localSet (localIndex functionValue dst))
        (Call, [LabelRef labelName]) ->
            case Map.lookup labelName (contextFunctionIndices context) of
                Nothing -> Left (LoweringError ("unknown call target " ++ labelName))
                Just indexValue ->
                    let targetParamCount = maybe 0 (signatureParamCount . functionSignature) (findFunction labelName)
                        args = BS.concat [localGet (localIndex functionValue register) | register <- take targetParamCount [2 ..]]
                        storeResult =
                            if labelName == "_start"
                                then BS.empty
                                else localSet (localIndex functionValue 1)
                     in Right (args <> BS.pack [0x10] <> encodeUnsigned indexValue <> storeResult)
        (Ret, []) ->
            Right
                ( if functionLabel functionValue == "_start"
                    then BS.pack [0x0F]
                    else localGet (localIndex functionValue 1) <> BS.pack [0x0F]
                )
        (Halt, []) -> Right (BS.pack [0x0F])
        (Syscall, [Immediate number]) -> lowerSyscall context functionValue number
        _ -> Left (LoweringError ("unsupported IR instruction " ++ show inst))
  where
    findFunction labelName =
        List.find ((== labelName) . functionLabel) (contextAllFunctions context)

lowerSyscall :: LoweringContext -> FunctionIR -> Integer -> Either LoweringError ByteString
lowerSyscall context functionValue number =
    case number of
        1 -> do
            scratch <- requireScratch context
            indexValue <- requireWasiImport context syscallWrite
            let iovecPtr = scratch
                nwrittenPtr = scratch + 8
                bytePtr = scratch + 12
            Right
                ( i32Const bytePtr
                    <> localGet (localIndex functionValue regSysArg)
                    <> BS.pack [0x3A]
                    <> memarg 0 0
                    <> storeConstI32 iovecPtr bytePtr
                    <> storeConstI32 (iovecPtr + 4) 1
                    <> i32Const 1
                    <> i32Const iovecPtr
                    <> i32Const 1
                    <> i32Const nwrittenPtr
                    <> BS.pack [0x10]
                    <> encodeUnsigned indexValue
                    <> localSet (localIndex functionValue regSysScratch)
                )
        2 -> do
            scratch <- requireScratch context
            indexValue <- requireWasiImport context syscallRead
            let iovecPtr = scratch
                nreadPtr = scratch + 8
                bytePtr = scratch + 12
            Right
                ( i32Const bytePtr
                    <> i32Const 0
                    <> BS.pack [0x3A]
                    <> memarg 0 0
                    <> storeConstI32 iovecPtr bytePtr
                    <> storeConstI32 (iovecPtr + 4) 1
                    <> i32Const 0
                    <> i32Const iovecPtr
                    <> i32Const 1
                    <> i32Const nreadPtr
                    <> BS.pack [0x10]
                    <> encodeUnsigned indexValue
                    <> localSet (localIndex functionValue regSysScratch)
                    <> i32Const bytePtr
                    <> BS.pack [0x2D]
                    <> memarg 0 0
                    <> localSet (localIndex functionValue regSysArg)
                )
        10 -> do
            indexValue <- requireWasiImport context syscallExit
            Right (localGet (localIndex functionValue regSysArg) <> BS.pack [0x10] <> encodeUnsigned indexValue <> BS.pack [0x0F])
        _ -> Left (LoweringError ("unsupported SYSCALL number " ++ show number))

collectWasiImports :: IrProgram -> Either LoweringError [WasiImport]
collectWasiImports program =
    let required =
            List.nub
                [ number
                | IrInstruction Syscall [Immediate number] _ <- irInstructions program
                ]
        unsupported = filter (`notElem` [syscallWrite, syscallRead, syscallExit]) required
     in if null unsupported
            then Right [entry | entry <- orderedWasiImports, wasiSyscallNumber entry `elem` required]
            else Left (LoweringError ("unsupported SYSCALL number(s) " ++ show unsupported))

orderedWasiImports :: [WasiImport]
orderedWasiImports =
    [ WasiImport syscallWrite "fd_write" (FuncType [I32, I32, I32, I32] [I32])
    , WasiImport syscallRead "fd_read" (FuncType [I32, I32, I32, I32] [I32])
    , WasiImport syscallExit "proc_exit" (FuncType [I32] [])
    ]

requireWasiImport :: LoweringContext -> Integer -> Either LoweringError Integer
requireWasiImport context number =
    case Map.lookup number (contextWasiImports context) of
        Just indexValue -> Right indexValue
        Nothing -> Left (LoweringError ("missing WASI import for SYSCALL " ++ show number))

requireScratch :: LoweringContext -> Either LoweringError Integer
requireScratch context =
    case contextWasiScratchBase context of
        Just scratch -> Right scratch
        Nothing -> Left (LoweringError "SYSCALL lowering requires WASM scratch memory")

needsMemory :: IrProgram -> Bool
needsMemory program =
    not (null (irDataDecls program))
        || any
            ((`elem` [LoadAddr, LoadByte, StoreByte]) . irOpcode)
            (irInstructions program)

needsWasiScratch :: IrProgram -> Bool
needsWasiScratch program =
    any isScratchSyscall (irInstructions program)
  where
    isScratchSyscall (IrInstruction Syscall [Immediate number] _) = number == syscallWrite || number == syscallRead
    isScratchSyscall _ = False

layoutData :: [IrDataDecl] -> Map String Integer
layoutData decls =
    snd (foldl step (0, Map.empty) decls)
  where
    step (offset, mapping) decl =
        (offset + toInteger (irDataSize decl), Map.insert (irDataLabel decl) offset mapping)

totalDataSize :: [IrDataDecl] -> Integer
totalDataSize = sum . map (toInteger . irDataSize)

alignUp :: Integer -> Integer -> Integer
alignUp value alignment =
    ((value + alignment - 1) `div` alignment) * alignment

pagesFor :: Integer -> Integer
pagesFor bytesValue =
    max 1 ((bytesValue + 65535) `div` 65536)

instructionMaxRegister :: IrInstruction -> Int
instructionMaxRegister inst =
    maximum (0 : [register | Register register <- irOperands inst])

localIndex :: FunctionIR -> Int -> Integer
localIndex functionValue register
    | register >= 2 && register < 2 + paramCount = toInteger (register - 2)
    | otherwise = toInteger (paramCount + register)
  where
    paramCount = signatureParamCount (functionSignature functionValue)

address :: FunctionIR -> Int -> Int -> ByteString
address functionValue base offset =
    localGet (localIndex functionValue base) <> localGet (localIndex functionValue offset) <> BS.pack [0x6A]

i32Const :: Integer -> ByteString
i32Const value = BS.pack [0x41] <> encodeSigned value

constExpr :: Integer -> ByteString
constExpr value = i32Const value <> BS.pack [0x0B]

localGet :: Integer -> ByteString
localGet indexValue = BS.pack [0x20] <> encodeUnsigned indexValue

localSet :: Integer -> ByteString
localSet indexValue = BS.pack [0x21] <> encodeUnsigned indexValue

memarg :: Integer -> Integer -> ByteString
memarg alignment offset =
    encodeUnsigned alignment <> encodeUnsigned offset

storeConstI32 :: Integer -> Integer -> ByteString
storeConstI32 addressValue value =
    i32Const addressValue <> i32Const value <> BS.pack [0x36] <> memarg 2 0

isLoopStart :: String -> Bool
isLoopStart labelName =
    "loop_" `List.isPrefixOf` labelName && "_start" `List.isSuffixOf` labelName

matchingLoopEnd :: String -> String
matchingLoopEnd labelName =
    take (length labelName - length ("_start" :: String)) labelName ++ "_end"

requireLabel :: Map String Int -> String -> Either LoweringError Int
requireLabel labels labelName =
    case Map.lookup labelName labels of
        Just indexValue -> Right indexValue
        Nothing -> Left (LoweringError ("missing label " ++ labelName))

findFirstBranchToLabel :: [IrInstruction] -> Int -> Int -> String -> Either LoweringError Int
findFirstBranchToLabel instructions startIndex endIndex labelName =
    case [indexValue | indexValue <- [startIndex .. endIndex - 1], branchesTo labelName (instructions !! indexValue)] of
        indexValue : _ -> Right indexValue
        [] -> Left (LoweringError ("expected branch to " ++ labelName))

findLastJumpToLabel :: [IrInstruction] -> Int -> Int -> String -> Either LoweringError Int
findLastJumpToLabel instructions startIndex endIndex labelName =
    case [indexValue | indexValue <- reverse [startIndex .. endIndex - 1], jumpsTo labelName (instructions !! indexValue)] of
        indexValue : _ -> Right indexValue
        [] -> Left (LoweringError ("expected jump to " ++ labelName))

branchesTo :: String -> IrInstruction -> Bool
branchesTo labelName (IrInstruction opcode [Register _, LabelRef target] _) =
    (opcode == BranchZ || opcode == BranchNz) && target == labelName
branchesTo _ _ = False

jumpsTo :: String -> IrInstruction -> Bool
jumpsTo labelName (IrInstruction Jump [LabelRef target] _) = target == labelName
jumpsTo _ _ = False

branchRegister :: IrInstruction -> Either LoweringError Int
branchRegister (IrInstruction opcode [Register register, LabelRef _] _)
    | opcode == BranchZ || opcode == BranchNz = Right register
branchRegister inst = Left (LoweringError ("expected branch instruction, got " ++ show inst))
