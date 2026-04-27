module WasmOpcodes
    ( description
    , OpcodeInfo(..)
    , opcodes
    , opcodeByByte
    , opcodeByName
    ) where

import qualified Data.Map.Strict as Map
import Data.Map.Strict (Map)
import Data.Word (Word8)

description :: String
description = "Haskell WebAssembly opcode metadata for the core execution subset"

data OpcodeInfo = OpcodeInfo
    { opcodeName :: String
    , opcodeByte :: Word8
    , opcodeCategory :: String
    , opcodeImmediates :: [String]
    , opcodeStackPop :: Int
    , opcodeStackPush :: Int
    }
    deriving (Eq, Ord, Show)

opcodes :: [OpcodeInfo]
opcodes =
    [ control "unreachable" 0x00 [] 0 0
    , control "nop" 0x01 [] 0 0
    , control "block" 0x02 ["blocktype"] 0 0
    , control "loop" 0x03 ["blocktype"] 0 0
    , control "if" 0x04 ["blocktype"] 1 0
    , control "else" 0x05 [] 0 0
    , control "end" 0x0B [] 0 0
    , control "br" 0x0C ["labelidx"] 0 0
    , control "br_if" 0x0D ["labelidx"] 1 0
    , control "return" 0x0F [] 0 0
    , control "call" 0x10 ["funcidx"] 0 0
    , control "call_indirect" 0x11 ["typeidx", "tableidx"] 1 0
    , category "parametric" "drop" 0x1A [] 1 0
    , category "parametric" "select" 0x1B [] 3 1
    , variable "local.get" 0x20 ["localidx"] 0 1
    , variable "local.set" 0x21 ["localidx"] 1 0
    , variable "local.tee" 0x22 ["localidx"] 1 1
    , variable "global.get" 0x23 ["globalidx"] 0 1
    , variable "global.set" 0x24 ["globalidx"] 1 0
    , memory "i32.load" 0x28 ["memarg"] 1 1
    , memory "i64.load" 0x29 ["memarg"] 1 1
    , memory "f32.load" 0x2A ["memarg"] 1 1
    , memory "f64.load" 0x2B ["memarg"] 1 1
    , memory "i32.load8_s" 0x2C ["memarg"] 1 1
    , memory "i32.load8_u" 0x2D ["memarg"] 1 1
    , memory "i32.load16_s" 0x2E ["memarg"] 1 1
    , memory "i32.load16_u" 0x2F ["memarg"] 1 1
    , memory "i64.load8_s" 0x30 ["memarg"] 1 1
    , memory "i64.load8_u" 0x31 ["memarg"] 1 1
    , memory "i64.load16_s" 0x32 ["memarg"] 1 1
    , memory "i64.load16_u" 0x33 ["memarg"] 1 1
    , memory "i64.load32_s" 0x34 ["memarg"] 1 1
    , memory "i64.load32_u" 0x35 ["memarg"] 1 1
    , memory "i32.store" 0x36 ["memarg"] 2 0
    , memory "i64.store" 0x37 ["memarg"] 2 0
    , memory "f32.store" 0x38 ["memarg"] 2 0
    , memory "f64.store" 0x39 ["memarg"] 2 0
    , memory "i32.store8" 0x3A ["memarg"] 2 0
    , memory "i32.store16" 0x3B ["memarg"] 2 0
    , memory "i64.store8" 0x3C ["memarg"] 2 0
    , memory "i64.store16" 0x3D ["memarg"] 2 0
    , memory "i64.store32" 0x3E ["memarg"] 2 0
    , memory "memory.size" 0x3F ["memidx"] 0 1
    , memory "memory.grow" 0x40 ["memidx"] 1 1
    , numeric "numeric_i32" "i32.const" 0x41 ["i32"] 0 1
    , numeric "numeric_i64" "i64.const" 0x42 ["i64"] 0 1
    , numeric "numeric_f32" "f32.const" 0x43 ["f32"] 0 1
    , numeric "numeric_f64" "f64.const" 0x44 ["f64"] 0 1
    , numeric "numeric_i32" "i32.eqz" 0x45 [] 1 1
    , numeric "numeric_i32" "i32.eq" 0x46 [] 2 1
    , numeric "numeric_i32" "i32.ne" 0x47 [] 2 1
    , numeric "numeric_i32" "i32.lt_s" 0x48 [] 2 1
    , numeric "numeric_i32" "i32.gt_s" 0x4A [] 2 1
    , numeric "numeric_i32" "i32.le_s" 0x4C [] 2 1
    , numeric "numeric_i32" "i32.ge_s" 0x4E [] 2 1
    , numeric "numeric_i64" "i64.eqz" 0x50 [] 1 1
    , numeric "numeric_f32" "f32.eq" 0x5B [] 2 1
    , numeric "numeric_f32" "f32.ne" 0x5C [] 2 1
    , numeric "numeric_f32" "f32.lt" 0x5D [] 2 1
    , numeric "numeric_f32" "f32.gt" 0x5E [] 2 1
    , numeric "numeric_f32" "f32.le" 0x5F [] 2 1
    , numeric "numeric_f32" "f32.ge" 0x60 [] 2 1
    , numeric "numeric_f64" "f64.eq" 0x61 [] 2 1
    , numeric "numeric_f64" "f64.ne" 0x62 [] 2 1
    , numeric "numeric_f64" "f64.lt" 0x63 [] 2 1
    , numeric "numeric_f64" "f64.gt" 0x64 [] 2 1
    , numeric "numeric_f64" "f64.le" 0x65 [] 2 1
    , numeric "numeric_f64" "f64.ge" 0x66 [] 2 1
    , numeric "numeric_i32" "i32.add" 0x6A [] 2 1
    , numeric "numeric_i32" "i32.sub" 0x6B [] 2 1
    , numeric "numeric_i32" "i32.mul" 0x6C [] 2 1
    , numeric "numeric_i32" "i32.div_s" 0x6D [] 2 1
    , numeric "numeric_i32" "i32.rem_s" 0x6F [] 2 1
    , numeric "numeric_i32" "i32.and" 0x71 [] 2 1
    , numeric "numeric_i32" "i32.or" 0x72 [] 2 1
    , numeric "numeric_i32" "i32.xor" 0x73 [] 2 1
    , numeric "numeric_i32" "i32.shl" 0x74 [] 2 1
    , numeric "numeric_i32" "i32.shr_s" 0x75 [] 2 1
    , numeric "numeric_i32" "i32.rotl" 0x77 [] 2 1
    , numeric "numeric_i32" "i32.rotr" 0x78 [] 2 1
    , numeric "numeric_i64" "i64.add" 0x7C [] 2 1
    , numeric "numeric_i64" "i64.sub" 0x7D [] 2 1
    , numeric "numeric_i64" "i64.mul" 0x7E [] 2 1
    , numeric "numeric_f32" "f32.add" 0x92 [] 2 1
    , numeric "numeric_f32" "f32.sub" 0x93 [] 2 1
    , numeric "numeric_f32" "f32.mul" 0x94 [] 2 1
    , numeric "numeric_f32" "f32.div" 0x95 [] 2 1
    , numeric "numeric_f64" "f64.add" 0xA0 [] 2 1
    , numeric "numeric_f64" "f64.sub" 0xA1 [] 2 1
    , numeric "numeric_f64" "f64.mul" 0xA2 [] 2 1
    , numeric "numeric_f64" "f64.div" 0xA3 [] 2 1
    , category "conversion" "i32.wrap_i64" 0xA7 [] 1 1
    , category "conversion" "i64.extend_i32_s" 0xAC [] 1 1
    , category "conversion" "i64.extend_i32_u" 0xAD [] 1 1
    , category "conversion" "i32.reinterpret_f32" 0xBC [] 1 1
    , category "conversion" "i64.reinterpret_f64" 0xBD [] 1 1
    , category "conversion" "f32.reinterpret_i32" 0xBE [] 1 1
    , category "conversion" "f64.reinterpret_i64" 0xBF [] 1 1
    ]

opcodeByByte :: Word8 -> Maybe OpcodeInfo
opcodeByByte byte = Map.lookup byte opcodeMapByByte

opcodeByName :: String -> Maybe OpcodeInfo
opcodeByName name = Map.lookup name opcodeMapByName

opcodeMapByByte :: Map Word8 OpcodeInfo
opcodeMapByByte = Map.fromList [(opcodeByte opcodeInfo, opcodeInfo) | opcodeInfo <- opcodes]

opcodeMapByName :: Map String OpcodeInfo
opcodeMapByName = Map.fromList [(opcodeName opcodeInfo, opcodeInfo) | opcodeInfo <- opcodes]

category :: String -> String -> Word8 -> [String] -> Int -> Int -> OpcodeInfo
category categoryName name byte immediates pops pushes =
    OpcodeInfo
        { opcodeName = name
        , opcodeByte = byte
        , opcodeCategory = categoryName
        , opcodeImmediates = immediates
        , opcodeStackPop = pops
        , opcodeStackPush = pushes
        }

control :: String -> Word8 -> [String] -> Int -> Int -> OpcodeInfo
control = category "control"

variable :: String -> Word8 -> [String] -> Int -> Int -> OpcodeInfo
variable = category "variable"

memory :: String -> Word8 -> [String] -> Int -> Int -> OpcodeInfo
memory = category "memory"

numeric :: String -> String -> Word8 -> [String] -> Int -> Int -> OpcodeInfo
numeric = category
