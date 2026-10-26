-- | Inert Starlark opcode and operator metadata.
--
-- This module is deliberately smaller than a compiler.  An 'Op' describes one
-- byte, but constructing or decoding it performs no parsing, module loading,
-- output, execution, or host interaction.  Think of the table as the legend on
-- a map: it explains symbols; it does not travel anywhere.
module StarlarkCompiler
    ( Op (..)
    , OpCategory (..)
    , allOps
    , opFromByte
    , opByte
    , opName
    , opCategory
    , binaryOpMap
    , compareOpMap
    , augmentedAssignMap
    , unaryOpMap
    , binaryOpcode
    , compareOpcode
    , augmentedAssignOpcode
    , unaryOpcode
    ) where

import qualified Data.Map.Strict as Map
import Data.Map.Strict (Map)
import Data.Word (Word8)

-- | The 61 assigned instructions in the standalone STP01 profile.
--
-- We intentionally do not derive 'Enum'.  Declaration order is dense, while
-- the wire format contains holes; an automatically derived ordinal would be a
-- convincing but incorrect byte value.
data Op
    = LoadConst | Pop | Dup | LoadNone | LoadTrue | LoadFalse
    | StoreName | LoadName | StoreLocal | LoadLocal | StoreClosure | LoadClosure
    | Add | Sub | Mul | Div | FloorDiv | Mod | Power | Negate
    | BitAnd | BitOr | BitXor | BitNot | LShift | RShift
    | CmpEq | CmpNe | CmpLt | CmpGt | CmpLe | CmpGe | CmpIn | CmpNotIn | Not
    | Jump | JumpIfFalse | JumpIfTrue | JumpIfFalseOrPop | JumpIfTrueOrPop
    | MakeFunction | CallFunction | CallFunctionKw | Return
    | BuildList | BuildDict | BuildTuple | ListAppend | DictSet
    | LoadSubscript | StoreSubscript | LoadAttr | StoreAttr | LoadSlice
    | GetIter | ForIter | UnpackSequence
    | LoadModule | ImportFrom
    | Print
    | Halt
    deriving (Eq, Ord, Show)

-- | Semantic groups encoded by an assigned byte's high nibble.
--
-- Categories are returned only for an 'Op', so a high nibble by itself never
-- turns an unassigned byte into an instruction.
data OpCategory
    = Stack
    | Variable
    | Arithmetic
    | Comparison
    | ControlFlow
    | Function
    | Collection
    | SubscriptAttribute
    | Iteration
    | Module
    | InputOutput
    | VMControl
    deriving (Eq, Ord, Show)

-- | Every assigned opcode in ascending wire-byte order.
allOps :: [Op]
allOps =
    [ LoadConst, Pop, Dup, LoadNone, LoadTrue, LoadFalse
    , StoreName, LoadName, StoreLocal, LoadLocal, StoreClosure, LoadClosure
    , Add, Sub, Mul, Div, FloorDiv, Mod, Power, Negate
    , BitAnd, BitOr, BitXor, BitNot, LShift, RShift
    , CmpEq, CmpNe, CmpLt, CmpGt, CmpLe, CmpGe, CmpIn, CmpNotIn, Not
    , Jump, JumpIfFalse, JumpIfTrue, JumpIfFalseOrPop, JumpIfTrueOrPop
    , MakeFunction, CallFunction, CallFunctionKw, Return
    , BuildList, BuildDict, BuildTuple, ListAppend, DictSet
    , LoadSubscript, StoreSubscript, LoadAttr, StoreAttr, LoadSlice
    , GetIter, ForIter, UnpackSequence
    , LoadModule, ImportFrom, Print, Halt
    ]

-- | Decode one byte.  The 195 holes return 'Nothing'.
opFromByte :: Word8 -> Maybe Op
opFromByte byte = Map.lookup byte byteToOp

byteToOp :: Map Word8 Op
byteToOp = Map.fromList [(opByte opcode, opcode) | opcode <- allOps]

-- | Encode an assigned opcode to its exact STP01 byte.
opByte :: Op -> Word8
opByte opcode = let (byte, _, _) = opcodeRecord opcode in byte

-- | Project the exact case-sensitive neutral symbol.
opName :: Op -> String
opName opcode = let (_, name, _) = opcodeRecord opcode in name

-- | Return the exact neutral category of an assigned opcode.
opCategory :: Op -> OpCategory
opCategory opcode = let (_, _, category) = opcodeRecord opcode in category

-- | A single exhaustive authority keeps byte, name, and category together.
-- Adding a constructor without its record is therefore a compile-time error.
opcodeRecord :: Op -> (Word8, String, OpCategory)
opcodeRecord LoadConst = (0x01, "LoadConst", Stack)
opcodeRecord Pop = (0x02, "Pop", Stack)
opcodeRecord Dup = (0x03, "Dup", Stack)
opcodeRecord LoadNone = (0x04, "LoadNone", Stack)
opcodeRecord LoadTrue = (0x05, "LoadTrue", Stack)
opcodeRecord LoadFalse = (0x06, "LoadFalse", Stack)
opcodeRecord StoreName = (0x10, "StoreName", Variable)
opcodeRecord LoadName = (0x11, "LoadName", Variable)
opcodeRecord StoreLocal = (0x12, "StoreLocal", Variable)
opcodeRecord LoadLocal = (0x13, "LoadLocal", Variable)
opcodeRecord StoreClosure = (0x14, "StoreClosure", Variable)
opcodeRecord LoadClosure = (0x15, "LoadClosure", Variable)
opcodeRecord Add = (0x20, "Add", Arithmetic)
opcodeRecord Sub = (0x21, "Sub", Arithmetic)
opcodeRecord Mul = (0x22, "Mul", Arithmetic)
opcodeRecord Div = (0x23, "Div", Arithmetic)
opcodeRecord FloorDiv = (0x24, "FloorDiv", Arithmetic)
opcodeRecord Mod = (0x25, "Mod", Arithmetic)
opcodeRecord Power = (0x26, "Power", Arithmetic)
opcodeRecord Negate = (0x27, "Negate", Arithmetic)
opcodeRecord BitAnd = (0x28, "BitAnd", Arithmetic)
opcodeRecord BitOr = (0x29, "BitOr", Arithmetic)
opcodeRecord BitXor = (0x2A, "BitXor", Arithmetic)
opcodeRecord BitNot = (0x2B, "BitNot", Arithmetic)
opcodeRecord LShift = (0x2C, "LShift", Arithmetic)
opcodeRecord RShift = (0x2D, "RShift", Arithmetic)
opcodeRecord CmpEq = (0x30, "CmpEq", Comparison)
opcodeRecord CmpNe = (0x31, "CmpNe", Comparison)
opcodeRecord CmpLt = (0x32, "CmpLt", Comparison)
opcodeRecord CmpGt = (0x33, "CmpGt", Comparison)
opcodeRecord CmpLe = (0x34, "CmpLe", Comparison)
opcodeRecord CmpGe = (0x35, "CmpGe", Comparison)
opcodeRecord CmpIn = (0x36, "CmpIn", Comparison)
opcodeRecord CmpNotIn = (0x37, "CmpNotIn", Comparison)
opcodeRecord Not = (0x38, "Not", Comparison)
opcodeRecord Jump = (0x40, "Jump", ControlFlow)
opcodeRecord JumpIfFalse = (0x41, "JumpIfFalse", ControlFlow)
opcodeRecord JumpIfTrue = (0x42, "JumpIfTrue", ControlFlow)
opcodeRecord JumpIfFalseOrPop = (0x43, "JumpIfFalseOrPop", ControlFlow)
opcodeRecord JumpIfTrueOrPop = (0x44, "JumpIfTrueOrPop", ControlFlow)
opcodeRecord MakeFunction = (0x50, "MakeFunction", Function)
opcodeRecord CallFunction = (0x51, "CallFunction", Function)
opcodeRecord CallFunctionKw = (0x52, "CallFunctionKw", Function)
opcodeRecord Return = (0x53, "Return", Function)
opcodeRecord BuildList = (0x60, "BuildList", Collection)
opcodeRecord BuildDict = (0x61, "BuildDict", Collection)
opcodeRecord BuildTuple = (0x62, "BuildTuple", Collection)
opcodeRecord ListAppend = (0x63, "ListAppend", Collection)
opcodeRecord DictSet = (0x64, "DictSet", Collection)
opcodeRecord LoadSubscript = (0x70, "LoadSubscript", SubscriptAttribute)
opcodeRecord StoreSubscript = (0x71, "StoreSubscript", SubscriptAttribute)
opcodeRecord LoadAttr = (0x72, "LoadAttr", SubscriptAttribute)
opcodeRecord StoreAttr = (0x73, "StoreAttr", SubscriptAttribute)
opcodeRecord LoadSlice = (0x74, "LoadSlice", SubscriptAttribute)
opcodeRecord GetIter = (0x80, "GetIter", Iteration)
opcodeRecord ForIter = (0x81, "ForIter", Iteration)
opcodeRecord UnpackSequence = (0x82, "UnpackSequence", Iteration)
opcodeRecord LoadModule = (0x90, "LoadModule", Module)
opcodeRecord ImportFrom = (0x91, "ImportFrom", Module)
opcodeRecord Print = (0xA0, "Print", InputOutput)
opcodeRecord Halt = (0xFF, "Halt", VMControl)

-- | Binary expression spellings.  'Map' is persistent: callers can derive a
-- changed value without mutating this value or a later lookup.
binaryOpMap :: Map String Op
binaryOpMap = Map.fromList
    [ ("+", Add), ("-", Sub), ("*", Mul), ("/", Div)
    , ("//", FloorDiv), ("%", Mod), ("**", Power)
    , ("&", BitAnd), ("|", BitOr), ("^", BitXor)
    , ("<<", LShift), (">>", RShift)
    ]

-- | Comparison expression spellings.
compareOpMap :: Map String Op
compareOpMap = Map.fromList
    [ ("==", CmpEq), ("!=", CmpNe), ("<", CmpLt), (">", CmpGt)
    , ("<=", CmpLe), (">=", CmpGe), ("in", CmpIn), ("not in", CmpNotIn)
    ]

-- | Augmented assignment spellings and their arithmetic instruction.
augmentedAssignMap :: Map String Op
augmentedAssignMap = Map.fromList
    [ ("+=", Add), ("-=", Sub), ("*=", Mul), ("/=", Div)
    , ("//=", FloorDiv), ("%=", Mod), ("**=", Power)
    , ("&=", BitAnd), ("|=", BitOr), ("^=", BitXor)
    , ("<<=", LShift), (">>=", RShift)
    ]

-- | Unary expression spellings.
unaryOpMap :: Map String Op
unaryOpMap = Map.fromList [("-", Negate), ("~", BitNot)]

binaryOpcode :: String -> Maybe Op
binaryOpcode = (`Map.lookup` binaryOpMap)

compareOpcode :: String -> Maybe Op
compareOpcode = (`Map.lookup` compareOpMap)

augmentedAssignOpcode :: String -> Maybe Op
augmentedAssignOpcode = (`Map.lookup` augmentedAssignMap)

unaryOpcode :: String -> Maybe Op
unaryOpcode = (`Map.lookup` unaryOpMap)
