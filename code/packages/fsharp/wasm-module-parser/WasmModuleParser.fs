namespace CodingAdventures.WasmModuleParser.FSharp

open System
open System.Collections.Generic
open System.Text
open CodingAdventures.WasmLeb128.FSharp
open CodingAdventures.WasmTypes.FSharp

module Version =
    [<Literal>]
    let VERSION = "0.1.0"

module private Helpers =
    let isValidValueType (current: byte) =
        current = byte ValueType.I32
        || current = byte ValueType.I64
        || current = byte ValueType.F32
        || current = byte ValueType.F64

type WasmParseError(message: string, offset: int) =
    inherit Exception(message)

    member _.Offset = offset

type WasmModuleParser() =
    member _.Parse(data: byte array) =
        let reader = BinaryReader(data)
        reader.ParseModule()

and internal BinaryReader(data: byte array) =
    static let wasmMagic = [| 0x00uy; 0x61uy; 0x73uy; 0x6Duy |]
    static let wasmVersion = [| 0x01uy; 0x00uy; 0x00uy; 0x00uy |]

    let sectionCustom = 0uy
    let sectionType = 1uy
    let sectionImport = 2uy
    let sectionFunction = 3uy
    let sectionTable = 4uy
    let sectionMemory = 5uy
    let sectionGlobal = 6uy
    let sectionExport = 7uy
    let sectionStart = 8uy
    let sectionElement = 9uy
    let sectionCode = 10uy
    let sectionData = 11uy
    let funcTypePrefix = 0x60uy
    let endOpcode = 0x0Buy

    let mutable pos = 0

    member _.Offset = pos

    member _.ReadByte() =
        if pos >= data.Length then
            raise (WasmParseError(sprintf "Unexpected end of data: expected 1 byte at offset %d" pos, pos))

        let value = data[pos]
        pos <- pos + 1
        value

    member _.ReadBytes(count: int) =
        if pos + count > data.Length then
            raise (
                WasmParseError(
                    sprintf
                        "Unexpected end of data: expected %d bytes at offset %d, but only %d remain"
                        count
                        pos
                        (data.Length - pos),
                    pos
                )
            )

        let slice = data[pos .. pos + count - 1]
        pos <- pos + count
        slice

    member this.ReadU32() =
        let offset = pos

        try
            let value, consumed = WasmLeb128.decodeUnsignedAt data pos
            pos <- pos + consumed
            value
        with ex ->
            raise (WasmParseError(sprintf "Invalid LEB128 at offset %d: %s" offset ex.Message, offset))

    member this.ReadString() =
        let length = this.ReadU32() |> int
        this.ReadBytes(length) |> Encoding.UTF8.GetString

    member _.AtEnd() = pos >= data.Length

    member this.ReadLimits() =
        let flagsOffset = pos
        let flags = this.ReadByte()
        let minimum = this.ReadU32() |> int

        let maximum =
            if (int flags &&& 1) <> 0 then
                Some(this.ReadU32() |> int)
            else
                if flags <> 0uy then
                    raise (WasmParseError(sprintf "Unknown limits flags byte 0x%02x at offset %d" flags flagsOffset, flagsOffset))

                None

        { Min = minimum; Max = maximum }

    member this.ReadGlobalType() =
        let valueTypeOffset = pos
        let valueTypeByte = this.ReadByte()
        if not (Helpers.isValidValueType valueTypeByte) then
            raise (
                WasmParseError(
                    sprintf "Unknown value type byte 0x%02x at offset %d" valueTypeByte valueTypeOffset,
                    valueTypeOffset
                )
            )

        let mutableByte = this.ReadByte()
        { ValueType = enum<ValueType> (int valueTypeByte); Mutable = mutableByte <> 0uy }

    member _.ReadInitExpr() =
        let start = pos
        let mutable finished = false

        while pos < data.Length && not finished do
            let current = data[pos]
            pos <- pos + 1
            if current = endOpcode then
                finished <- true

        if not finished then
            raise (WasmParseError(sprintf "Init expression at offset %d never terminated with 0x0B (end opcode)" start, start))

        data[start .. pos - 1]

    member this.ReadValueTypeVec() =
        let count = this.ReadU32() |> int
        let values = ResizeArray<ValueType>(count)

        for _ in 0 .. count - 1 do
            let valueTypeOffset = pos
            let valueTypeByte = this.ReadByte()
            if not (Helpers.isValidValueType valueTypeByte) then
                raise (
                    WasmParseError(
                        sprintf "Unknown value type byte 0x%02x at offset %d" valueTypeByte valueTypeOffset,
                        valueTypeOffset
                    )
                )

            values.Add(enum<ValueType> (int valueTypeByte))

        values |> Seq.toList

    member this.ParseTypeSection(moduleValue: WasmModule) =
        let count = this.ReadU32() |> int

        for _ in 0 .. count - 1 do
            let prefixOffset = pos
            let prefix = this.ReadByte()
            if prefix <> funcTypePrefix then
                raise (
                    WasmParseError(
                        sprintf "Expected function type prefix 0x60 at offset %d, got 0x%02x" prefixOffset prefix,
                        prefixOffset
                    )
                )

            moduleValue.Types.Add(WasmTypes.makeFuncType (this.ReadValueTypeVec()) (this.ReadValueTypeVec()))

    member this.ParseImportSection(moduleValue: WasmModule) =
        let count = this.ReadU32() |> int

        for _ in 0 .. count - 1 do
            let moduleName = this.ReadString()
            let name = this.ReadString()
            let kindOffset = pos
            let kind = this.ReadByte()

            let descriptor =
                match kind with
                | k when k = byte ExternalKind.FUNCTION -> FunctionImportDescriptor(this.ReadU32() |> int)
                | k when k = byte ExternalKind.TABLE ->
                    let elementTypeOffset = pos
                    let elementType = this.ReadByte()
                    if elementType <> ReferenceType.FUNCREF then
                        raise (
                            WasmParseError(
                                sprintf "Unknown table element type 0x%02x at offset %d" elementType elementTypeOffset,
                                kindOffset
                            )
                        )

                    TableImportDescriptor { ElementType = elementType; Limits = this.ReadLimits() }
                | k when k = byte ExternalKind.MEMORY -> MemoryImportDescriptor { Limits = this.ReadLimits() }
                | k when k = byte ExternalKind.GLOBAL -> GlobalImportDescriptor(this.ReadGlobalType())
                | _ -> raise (WasmParseError(sprintf "Unknown import kind 0x%02x at offset %d" kind kindOffset, kindOffset))

            moduleValue.Imports.Add(
                {
                    ModuleName = moduleName
                    Name = name
                    Kind = enum<ExternalKind> (int kind)
                    Descriptor = descriptor
                }
            )

    member this.ParseFunctionSection(moduleValue: WasmModule) =
        let count = this.ReadU32() |> int
        for _ in 0 .. count - 1 do
            moduleValue.Functions.Add(this.ReadU32() |> int)

    member this.ParseTableSection(moduleValue: WasmModule) =
        let count = this.ReadU32() |> int
        for _ in 0 .. count - 1 do
            let elementTypeOffset = pos
            let elementType = this.ReadByte()
            if elementType <> ReferenceType.FUNCREF then
                raise (
                    WasmParseError(
                        sprintf "Unknown table element type 0x%02x at offset %d" elementType elementTypeOffset,
                        elementTypeOffset
                    )
                )

            moduleValue.Tables.Add({ ElementType = elementType; Limits = this.ReadLimits() })

    member this.ParseMemorySection(moduleValue: WasmModule) =
        let count = this.ReadU32() |> int
        for _ in 0 .. count - 1 do
            moduleValue.Memories.Add({ Limits = this.ReadLimits() })

    member this.ParseGlobalSection(moduleValue: WasmModule) =
        let count = this.ReadU32() |> int
        for _ in 0 .. count - 1 do
            moduleValue.Globals.Add(Global(this.ReadGlobalType(), this.ReadInitExpr()))

    member this.ParseExportSection(moduleValue: WasmModule) =
        let count = this.ReadU32() |> int
        for _ in 0 .. count - 1 do
            let name = this.ReadString()
            let kindOffset = pos
            let kind = this.ReadByte()

            if kind <> byte ExternalKind.FUNCTION
               && kind <> byte ExternalKind.TABLE
               && kind <> byte ExternalKind.MEMORY
               && kind <> byte ExternalKind.GLOBAL then
                raise (WasmParseError(sprintf "Unknown export kind 0x%02x at offset %d" kind kindOffset, kindOffset))

            moduleValue.Exports.Add(
                {
                    Name = name
                    Kind = enum<ExternalKind> (int kind)
                    Index = this.ReadU32() |> int
                }
            )

    member this.ParseStartSection(moduleValue: WasmModule) =
        moduleValue.Start <- Some(this.ReadU32() |> int)

    member this.ParseElementSection(moduleValue: WasmModule) =
        let count = this.ReadU32() |> int

        for _ in 0 .. count - 1 do
            let tableIndex = this.ReadU32() |> int
            let offsetExpr = this.ReadInitExpr()
            let functionCount = this.ReadU32() |> int
            let functionIndices = ResizeArray<int>()

            for _ in 0 .. functionCount - 1 do
                functionIndices.Add(this.ReadU32() |> int)

            moduleValue.Elements.Add(Element(tableIndex, offsetExpr, functionIndices))

    member this.ParseCodeSection(moduleValue: WasmModule) =
        let count = this.ReadU32() |> int

        for i in 0 .. count - 1 do
            let bodySize = this.ReadU32() |> int
            let bodyStart = pos
            let bodyEnd = bodyStart + bodySize

            if bodyEnd > data.Length then
                raise (
                    WasmParseError(
                        sprintf "Code body %d extends beyond end of data (offset %d, size %d)" i bodyStart bodySize,
                        bodyStart
                    )
                )

            let localDeclCount = this.ReadU32() |> int
            let locals = ResizeArray<ValueType>()

            for _ in 0 .. localDeclCount - 1 do
                let groupCount = this.ReadU32() |> int
                let typeOffset = pos
                let typeByte = this.ReadByte()
                if not (Helpers.isValidValueType typeByte) then
                    raise (WasmParseError(sprintf "Unknown local type byte 0x%02x at offset %d" typeByte typeOffset, typeOffset))

                for _ in 0 .. groupCount - 1 do
                    locals.Add(enum<ValueType> (int typeByte))

            let codeLength = bodyEnd - pos
            if codeLength < 0 then
                raise (WasmParseError(sprintf "Code body %d local declarations exceeded body size at offset %d" i pos, pos))

            moduleValue.Code.Add(FunctionBody(locals, this.ReadBytes(codeLength)))

    member this.ParseDataSection(moduleValue: WasmModule) =
        let count = this.ReadU32() |> int

        for _ in 0 .. count - 1 do
            let memoryIndex = this.ReadU32() |> int
            let offsetExpr = this.ReadInitExpr()
            let byteCount = this.ReadU32() |> int
            moduleValue.Data.Add(DataSegment(memoryIndex, offsetExpr, this.ReadBytes(byteCount)))

    member _.ParseCustomSection(moduleValue: WasmModule, payload: byte array) =
        let subReader = BinaryReader(payload)
        let name = subReader.ReadString()
        moduleValue.Customs.Add(CustomSection(name, subReader.ReadBytes(payload.Length - subReader.Offset)))

    member this.ParseModule() =
        this.ValidateHeader()
        let moduleValue = WasmModule()
        let mutable lastSectionId = 0uy

        while not (this.AtEnd()) do
            let sectionIdOffset = pos
            let sectionId = this.ReadByte()
            let payloadSize = this.ReadU32() |> int
            let payloadStart = pos
            let payloadEnd = payloadStart + payloadSize

            if payloadEnd > data.Length then
                raise (
                    WasmParseError(
                        sprintf "Section %d payload extends beyond end of data (offset %d, size %d)" sectionId payloadStart payloadSize,
                        payloadStart
                    )
                )

            if sectionId <> sectionCustom then
                if sectionId < lastSectionId then
                    raise (
                        WasmParseError(
                            sprintf "Section %d appears out of order: already saw section %d" sectionId lastSectionId,
                            sectionIdOffset
                        )
                    )

                lastSectionId <- sectionId

            let payload = data[payloadStart .. payloadEnd - 1]

            match sectionId with
            | id when id = sectionType -> this.ParseTypeSection(moduleValue)
            | id when id = sectionImport -> this.ParseImportSection(moduleValue)
            | id when id = sectionFunction -> this.ParseFunctionSection(moduleValue)
            | id when id = sectionTable -> this.ParseTableSection(moduleValue)
            | id when id = sectionMemory -> this.ParseMemorySection(moduleValue)
            | id when id = sectionGlobal -> this.ParseGlobalSection(moduleValue)
            | id when id = sectionExport -> this.ParseExportSection(moduleValue)
            | id when id = sectionStart -> this.ParseStartSection(moduleValue)
            | id when id = sectionElement -> this.ParseElementSection(moduleValue)
            | id when id = sectionCode -> this.ParseCodeSection(moduleValue)
            | id when id = sectionData -> this.ParseDataSection(moduleValue)
            | id when id = sectionCustom -> this.ParseCustomSection(moduleValue, payload)
            | _ -> ()

            pos <- payloadEnd

        moduleValue

    member _.ValidateHeader() =
        if data.Length < 8 then
            raise (WasmParseError(sprintf "File too short: %d bytes (need at least 8 for the header)" data.Length, 0))

        for i in 0 .. 3 do
            if data[i] <> wasmMagic[i] then
                raise (
                    WasmParseError(
                        sprintf "Invalid magic bytes at offset %d: expected 0x%02x, got 0x%02x" i wasmMagic[i] data[i],
                        i
                    )
                )

        pos <- 4

        for i in 0 .. 3 do
            if data[4 + i] <> wasmVersion[i] then
                raise (
                    WasmParseError(
                        sprintf
                            "Unsupported WASM version at offset %d: expected 0x%02x, got 0x%02x"
                            (4 + i)
                            wasmVersion[i]
                            data[4 + i],
                        (4 + i)
                    )
                )

        pos <- 8
