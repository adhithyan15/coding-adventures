module BrainfuckIRCompiler
    ( description
    , BuildConfig(..)
    , CompileResult(..)
    , releaseConfig
    , compileOps
    , compileSource
    ) where

import Brainfuck (BrainfuckError, BrainfuckOp(..), parseSource)
import CompilerIR hiding (description)

description :: String
description = "Haskell Brainfuck to compiler IR frontend"

data CompileResult = CompileResult
    { compileResultProgram :: IrProgram
    , compileResultAst :: [BrainfuckOp]
    }
    deriving (Eq, Show)

data BuildConfig = BuildConfig
    { buildConfigTapeSize :: Int
    , buildConfigMaskByteArithmetic :: Bool
    }
    deriving (Eq, Show)

releaseConfig :: BuildConfig
releaseConfig =
    BuildConfig
        { buildConfigTapeSize = 30000
        , buildConfigMaskByteArithmetic = True
        }

compileSource :: String -> Either BrainfuckError CompileResult
compileSource source = do
    ast <- parseSource source
    Right (CompileResult (compileOps ast) ast)

compileOps :: [BrainfuckOp] -> IrProgram
compileOps = compileOpsWithConfig releaseConfig

compileOpsWithConfig :: BuildConfig -> [BrainfuckOp] -> IrProgram
compileOpsWithConfig config ops =
    let initialProgram =
            appendMany
                ((emptyProgram "_start") {irDataDecls = [IrDataDecl "tape" (buildConfigTapeSize config) 0]})
                [ instruction Label [LabelRef "_start"] (-1)
                , instruction LoadAddr [Register regTapeBase, LabelRef "tape"] 0
                , instruction LoadImm [Register regTapePtr, Immediate 0] 1
                ]
        (nextId, _, bodyProgram) = compileProgram config 2 0 initialProgram ops
     in appendInstruction bodyProgram (instruction Halt [] nextId)

regTapeBase :: Int
regTapeBase = 0

regTapePtr :: Int
regTapePtr = 1

regTemp :: Int
regTemp = 2

regSysArg :: Int
regSysArg = 4

syscallWrite :: Integer
syscallWrite = 1

syscallRead :: Integer
syscallRead = 2

compileProgram :: BuildConfig -> Int -> Int -> IrProgram -> [BrainfuckOp] -> (Int, Int, IrProgram)
compileProgram config nextId loopIndex program ops =
    foldl step (nextId, loopIndex, program) ops
  where
    step (ident, currentLoop, currentProgram) op =
        compileOp config ident currentLoop currentProgram op

compileOp :: BuildConfig -> Int -> Int -> IrProgram -> BrainfuckOp -> (Int, Int, IrProgram)
compileOp config ident loopIndex program op =
    case op of
        MoveRight ->
            emitOne ident loopIndex program (instruction AddImm [Register regTapePtr, Register regTapePtr, Immediate 1] ident)
        MoveLeft ->
            emitOne ident loopIndex program (instruction AddImm [Register regTapePtr, Register regTapePtr, Immediate (-1)] ident)
        Increment ->
            emitCellMutation config ident loopIndex program 1
        Decrement ->
            emitCellMutation config ident loopIndex program (-1)
        Output ->
            emitMany
                ident
                loopIndex
                program
                [ instruction LoadByte [Register regTemp, Register regTapeBase, Register regTapePtr] ident
                , instruction AddImm [Register regSysArg, Register regTemp, Immediate 0] (ident + 1)
                , instruction Syscall [Immediate syscallWrite] (ident + 2)
                ]
        Input ->
            emitMany
                ident
                loopIndex
                program
                [ instruction Syscall [Immediate syscallRead] ident
                , instruction StoreByte [Register regSysArg, Register regTapeBase, Register regTapePtr] (ident + 1)
                ]
        Loop body ->
            let startLabel = "loop_" ++ show loopIndex ++ "_start"
                endLabel = "loop_" ++ show loopIndex ++ "_end"
                withHeader =
                    appendMany
                        program
                        [ instruction Label [LabelRef startLabel] (-1)
                        , instruction LoadByte [Register regTemp, Register regTapeBase, Register regTapePtr] ident
                        , instruction BranchZ [Register regTemp, LabelRef endLabel] (ident + 1)
                        ]
                (afterBodyId, afterBodyLoop, afterBodyProgram) = compileProgram config (ident + 2) (loopIndex + 1) withHeader body
                withBackedge =
                    appendMany
                        afterBodyProgram
                        [ instruction Jump [LabelRef startLabel] afterBodyId
                        , instruction Label [LabelRef endLabel] (-1)
                        ]
             in (afterBodyId + 1, afterBodyLoop, withBackedge)

emitCellMutation :: BuildConfig -> Int -> Int -> IrProgram -> Integer -> (Int, Int, IrProgram)
emitCellMutation config ident loopIndex program delta =
    emitMany ident loopIndex program instructions
  where
    instructions =
        [ instruction LoadByte [Register regTemp, Register regTapeBase, Register regTapePtr] ident
        , instruction AddImm [Register regTemp, Register regTemp, Immediate delta] (ident + 1)
        ]
            ++ [instruction AndImm [Register regTemp, Register regTemp, Immediate 255] (ident + 2) | buildConfigMaskByteArithmetic config]
            ++ [instruction StoreByte [Register regTemp, Register regTapeBase, Register regTapePtr] (ident + if buildConfigMaskByteArithmetic config then 3 else 2)]

emitOne :: Int -> Int -> IrProgram -> IrInstruction -> (Int, Int, IrProgram)
emitOne ident loopIndex program inst =
    (ident + 1, loopIndex, appendInstruction program inst)

emitMany :: Int -> Int -> IrProgram -> [IrInstruction] -> (Int, Int, IrProgram)
emitMany ident loopIndex program instructions =
    (ident + length instructions, loopIndex, appendMany program instructions)

appendMany :: IrProgram -> [IrInstruction] -> IrProgram
appendMany = foldl appendInstruction
