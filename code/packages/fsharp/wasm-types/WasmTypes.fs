namespace CodingAdventures.WasmTypes.FSharp

open System.Collections.Generic

module Version =
    [<Literal>]
    let VERSION = "0.1.0"

type ValueType =
    | I32 = 0x7F
    | I64 = 0x7E
    | F32 = 0x7D
    | F64 = 0x7C

[<RequireQualifiedAccess>]
module BlockType =
    let EMPTY = 0x40uy

type ExternalKind =
    | FUNCTION = 0x00
    | TABLE = 0x01
    | MEMORY = 0x02
    | GLOBAL = 0x03

[<RequireQualifiedAccess>]
module ReferenceType =
    let FUNCREF = 0x70uy

type FuncType =
    {
        Params: ValueType list
        Results: ValueType list
    }

[<Struct>]
type Limits =
    {
        Min: int
        Max: int option
    }

[<Struct>]
type MemoryType =
    {
        Limits: Limits
    }

[<Struct>]
type TableType =
    {
        ElementType: byte
        Limits: Limits
    }

[<Struct>]
type GlobalType =
    {
        ValueType: ValueType
        Mutable: bool
    }

type ImportDescriptor =
    | FunctionImportDescriptor of int
    | TableImportDescriptor of TableType
    | MemoryImportDescriptor of MemoryType
    | GlobalImportDescriptor of GlobalType

type Import =
    {
        ModuleName: string
        Name: string
        Kind: ExternalKind
        Descriptor: ImportDescriptor
    }

[<Struct>]
type Export =
    {
        Name: string
        Kind: ExternalKind
        Index: int
    }

type Global(globalType: GlobalType, initExpr: byte array) =
    let copiedInitExpr = Array.copy initExpr

    member _.GlobalType = globalType
    member _.InitExpr = copiedInitExpr

type Element(tableIndex: int, offsetExpr: byte array, functionIndices: seq<int>) =
    let copiedOffsetExpr = Array.copy offsetExpr
    let copiedFunctionIndices = functionIndices |> Seq.toList

    member _.TableIndex = tableIndex
    member _.OffsetExpr = copiedOffsetExpr
    member _.FunctionIndices = copiedFunctionIndices

type DataSegment(memoryIndex: int, offsetExpr: byte array, data: byte array) =
    let copiedOffsetExpr = Array.copy offsetExpr
    let copiedData = Array.copy data

    member _.MemoryIndex = memoryIndex
    member _.OffsetExpr = copiedOffsetExpr
    member _.Data = copiedData

type FunctionBody(locals: seq<ValueType>, code: byte array) =
    let copiedLocals = locals |> Seq.toList
    let copiedCode = Array.copy code

    member _.Locals = copiedLocals
    member _.Code = copiedCode

type CustomSection(name: string, data: byte array) =
    let copiedData = Array.copy data

    member _.Name = name
    member _.Data = copiedData

type WasmModule() =
    member val Types = ResizeArray<FuncType>() with get
    member val Imports = ResizeArray<Import>() with get
    member val Functions = ResizeArray<int>() with get
    member val Tables = ResizeArray<TableType>() with get
    member val Memories = ResizeArray<MemoryType>() with get
    member val Globals = ResizeArray<Global>() with get
    member val Exports = ResizeArray<Export>() with get
    member val Start: int option = None with get, set
    member val Elements = ResizeArray<Element>() with get
    member val Code = ResizeArray<FunctionBody>() with get
    member val Data = ResizeArray<DataSegment>() with get
    member val Customs = ResizeArray<CustomSection>() with get

[<RequireQualifiedAccess>]
module WasmTypes =
    let makeFuncType (parameters: seq<ValueType>) (results: seq<ValueType>) =
        {
            Params = parameters |> Seq.toList
            Results = results |> Seq.toList
        }
