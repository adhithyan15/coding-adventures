module WasmTypes
    ( description
    , ValueType(..)
    , BlockType(..)
    , ExternalKind(..)
    , FuncType(..)
    , Limits(..)
    , MemoryType(..)
    , TableType(..)
    , GlobalType(..)
    , ImportTypeInfo(..)
    , Import(..)
    , Export(..)
    , Global(..)
    , Element(..)
    , DataSegment(..)
    , FunctionBody(..)
    , CustomSection(..)
    , WasmModule(..)
    , emptyModule
    , valueTypeByte
    , valueTypeFromByte
    , blockTypeByte
    , externalKindByte
    , externalKindFromByte
    ) where

import Data.ByteString (ByteString)
import Data.Word (Word8)

description :: String
description = "Haskell WASM 1.0 type system and module data structures"

data ValueType
    = I32
    | I64
    | F32
    | F64
    deriving (Eq, Ord, Enum, Bounded, Show)

data BlockType
    = EmptyBlockType
    | BlockValueType ValueType
    | BlockTypeIndex Integer
    deriving (Eq, Ord, Show)

data ExternalKind
    = ExternalFunction
    | ExternalTable
    | ExternalMemory
    | ExternalGlobal
    deriving (Eq, Ord, Enum, Bounded, Show)

data FuncType = FuncType
    { funcTypeParams :: [ValueType]
    , funcTypeResults :: [ValueType]
    }
    deriving (Eq, Ord, Show)

data Limits = Limits
    { limitsMin :: Integer
    , limitsMax :: Maybe Integer
    }
    deriving (Eq, Ord, Show)

data MemoryType = MemoryType
    { memoryTypeLimits :: Limits
    }
    deriving (Eq, Ord, Show)

data TableType = TableType
    { tableTypeElementType :: Word8
    , tableTypeLimits :: Limits
    }
    deriving (Eq, Ord, Show)

data GlobalType = GlobalType
    { globalValueType :: ValueType
    , globalMutable :: Bool
    }
    deriving (Eq, Ord, Show)

data ImportTypeInfo
    = ImportFunctionType Integer
    | ImportTableType TableType
    | ImportMemoryType MemoryType
    | ImportGlobalType GlobalType
    deriving (Eq, Ord, Show)

data Import = Import
    { importModuleName :: String
    , importName :: String
    , importKind :: ExternalKind
    , importTypeInfo :: ImportTypeInfo
    }
    deriving (Eq, Ord, Show)

data Export = Export
    { exportName :: String
    , exportKind :: ExternalKind
    , exportIndex :: Integer
    }
    deriving (Eq, Ord, Show)

data Global = Global
    { globalType :: GlobalType
    , globalInitExpr :: ByteString
    }
    deriving (Eq, Ord, Show)

data Element = Element
    { elementTableIndex :: Integer
    , elementOffsetExpr :: ByteString
    , elementFunctionIndices :: [Integer]
    }
    deriving (Eq, Ord, Show)

data DataSegment = DataSegment
    { dataSegmentMemoryIndex :: Integer
    , dataSegmentOffsetExpr :: ByteString
    , dataSegmentBytes :: ByteString
    }
    deriving (Eq, Ord, Show)

data FunctionBody = FunctionBody
    { functionBodyLocals :: [ValueType]
    , functionBodyCode :: ByteString
    }
    deriving (Eq, Ord, Show)

data CustomSection = CustomSection
    { customSectionName :: String
    , customSectionData :: ByteString
    }
    deriving (Eq, Ord, Show)

data WasmModule = WasmModule
    { wasmTypes :: [FuncType]
    , wasmImports :: [Import]
    , wasmFunctions :: [Integer]
    , wasmTables :: [TableType]
    , wasmMemories :: [MemoryType]
    , wasmGlobals :: [Global]
    , wasmExports :: [Export]
    , wasmStart :: Maybe Integer
    , wasmElements :: [Element]
    , wasmCode :: [FunctionBody]
    , wasmDataSegments :: [DataSegment]
    , wasmCustomSections :: [CustomSection]
    }
    deriving (Eq, Ord, Show)

emptyModule :: WasmModule
emptyModule =
    WasmModule
        { wasmTypes = []
        , wasmImports = []
        , wasmFunctions = []
        , wasmTables = []
        , wasmMemories = []
        , wasmGlobals = []
        , wasmExports = []
        , wasmStart = Nothing
        , wasmElements = []
        , wasmCode = []
        , wasmDataSegments = []
        , wasmCustomSections = []
        }

valueTypeByte :: ValueType -> Word8
valueTypeByte valueType =
    case valueType of
        I32 -> 0x7F
        I64 -> 0x7E
        F32 -> 0x7D
        F64 -> 0x7C

valueTypeFromByte :: Word8 -> Maybe ValueType
valueTypeFromByte byte =
    case byte of
        0x7F -> Just I32
        0x7E -> Just I64
        0x7D -> Just F32
        0x7C -> Just F64
        _ -> Nothing

blockTypeByte :: BlockType -> Maybe Word8
blockTypeByte blockType =
    case blockType of
        EmptyBlockType -> Just 0x40
        BlockValueType valueType -> Just (valueTypeByte valueType)
        BlockTypeIndex _ -> Nothing

externalKindByte :: ExternalKind -> Word8
externalKindByte kind =
    case kind of
        ExternalFunction -> 0x00
        ExternalTable -> 0x01
        ExternalMemory -> 0x02
        ExternalGlobal -> 0x03

externalKindFromByte :: Word8 -> Maybe ExternalKind
externalKindFromByte byte =
    case byte of
        0x00 -> Just ExternalFunction
        0x01 -> Just ExternalTable
        0x02 -> Just ExternalMemory
        0x03 -> Just ExternalGlobal
        _ -> Nothing
