module CompilerIR
    ( description
    , IrOp(..)
    , IrOperand(..)
    , IrInstruction(..)
    , IrDataDecl(..)
    , IrProgram(..)
    , emptyProgram
    , appendInstruction
    , instruction
    , maxRegister
    ) where

description :: String
description = "Haskell compiler IR for source-to-Wasm pipeline packages"

data IrOp
    = Comment
    | Label
    | LoadImm
    | LoadAddr
    | LoadByte
    | StoreByte
    | Add
    | AddImm
    | Sub
    | And
    | AndImm
    | Jump
    | BranchZ
    | BranchNz
    | Call
    | Ret
    | Syscall
    | Halt
    | Nop
    deriving (Eq, Ord, Show)

data IrOperand
    = Register Int
    | Immediate Integer
    | LabelRef String
    deriving (Eq, Ord, Show)

data IrInstruction = IrInstruction
    { irOpcode :: IrOp
    , irOperands :: [IrOperand]
    , irInstructionId :: Int
    }
    deriving (Eq, Ord, Show)

data IrDataDecl = IrDataDecl
    { irDataLabel :: String
    , irDataSize :: Int
    , irDataInit :: Int
    }
    deriving (Eq, Ord, Show)

data IrProgram = IrProgram
    { irEntryLabel :: String
    , irInstructions :: [IrInstruction]
    , irDataDecls :: [IrDataDecl]
    }
    deriving (Eq, Ord, Show)

emptyProgram :: String -> IrProgram
emptyProgram entryLabel =
    IrProgram
        { irEntryLabel = entryLabel
        , irInstructions = []
        , irDataDecls = []
        }

appendInstruction :: IrProgram -> IrInstruction -> IrProgram
appendInstruction program inst =
    program {irInstructions = irInstructions program ++ [inst]}

instruction :: IrOp -> [IrOperand] -> Int -> IrInstruction
instruction opcode operands ident =
    IrInstruction
        { irOpcode = opcode
        , irOperands = operands
        , irInstructionId = ident
        }

maxRegister :: IrProgram -> Int
maxRegister program =
    maximum (0 : concatMap instructionRegisters (irInstructions program))
  where
    instructionRegisters inst =
        [register | Register register <- irOperands inst]
