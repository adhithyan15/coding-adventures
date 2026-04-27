namespace CodingAdventures.CompilerSourceMap.FSharp

open System
open System.Collections.Generic

type SourcePosition =
    { File: string
      Line: int
      Column: int
      Length: int }

    override this.ToString() =
        $"{this.File}:{this.Line}:{this.Column} (len={this.Length})"

type SourceToAstEntry =
    { Position: SourcePosition
      AstNodeId: int }

type SourceToAst() =
    let entries = ResizeArray<SourceToAstEntry>()

    member _.Entries = entries |> Seq.toList

    member _.Add(position: SourcePosition, astNodeId: int) =
        entries.Add({ Position = position; AstNodeId = astNodeId })

    member _.LookupByNodeId(astNodeId: int) =
        entries
        |> Seq.tryFind (fun entry -> entry.AstNodeId = astNodeId)
        |> Option.map _.Position

type AstToIrEntry =
    { AstNodeId: int
      IrIds: int64 list }

type AstToIr() =
    let entries = ResizeArray<AstToIrEntry>()

    member _.Entries = entries |> Seq.toList

    member _.Add(astNodeId: int, irIds: seq<int64>) =
        if isNull (box irIds) then
            nullArg "irIds"

        entries.Add({ AstNodeId = astNodeId; IrIds = irIds |> Seq.toList })

    member _.LookupByAstNodeId(astNodeId: int) =
        entries
        |> Seq.tryFind (fun entry -> entry.AstNodeId = astNodeId)
        |> Option.map _.IrIds

    member _.LookupByIrId(irId: int64) =
        match entries |> Seq.tryFind (fun entry -> entry.IrIds |> List.contains irId) with
        | Some entry -> entry.AstNodeId
        | None -> -1

type IrToIrEntry =
    { OriginalId: int64
      NewIds: int64 list }

type IrToIr(?passName: string) =
    let entries = ResizeArray<IrToIrEntry>()
    let deleted = HashSet<int64>()
    let passName = defaultArg passName ""

    member _.Entries = entries |> Seq.toList
    member _.Deleted = deleted
    member _.PassName = passName

    member _.AddMapping(originalId: int64, newIds: seq<int64>) =
        if isNull (box newIds) then
            nullArg "newIds"

        entries.Add({ OriginalId = originalId; NewIds = newIds |> Seq.toList })

    member _.AddDeletion(originalId: int64) =
        deleted.Add originalId |> ignore
        entries.Add({ OriginalId = originalId; NewIds = [] })

    member _.LookupByOriginalId(originalId: int64) =
        if deleted.Contains originalId then
            None
        else
            entries
            |> Seq.tryFind (fun entry -> entry.OriginalId = originalId)
            |> Option.map _.NewIds

    member _.LookupByNewId(newId: int64) =
        match entries |> Seq.tryFind (fun entry -> entry.NewIds |> List.contains newId) with
        | Some entry -> entry.OriginalId
        | None -> -1L

type IrToMachineCodeEntry =
    { IrId: int64
      MachineCodeOffset: int64
      MachineCodeLength: int64 }

type IrToMachineCode() =
    let entries = ResizeArray<IrToMachineCodeEntry>()

    member _.Entries = entries |> Seq.toList

    member _.Add(irId: int64, machineCodeOffset: int64, machineCodeLength: int64) =
        entries.Add(
            { IrId = irId
              MachineCodeOffset = machineCodeOffset
              MachineCodeLength = machineCodeLength }
        )

    member _.LookupByIrId(irId: int64) =
        match entries |> Seq.tryFind (fun entry -> entry.IrId = irId) with
        | Some entry -> entry.MachineCodeOffset, entry.MachineCodeLength
        | None -> -1L, 0L

    member _.LookupByMachineCodeOffset(offset: int64) =
        match
            entries
            |> Seq.tryFind (fun entry ->
                entry.MachineCodeOffset <= offset
                && offset < entry.MachineCodeOffset + entry.MachineCodeLength)
        with
        | Some entry -> entry.IrId
        | None -> -1L

type SourceMapChain() =
    let sourceToAst = SourceToAst()
    let astToIr = AstToIr()
    let irToIr = ResizeArray<IrToIr>()
    let mutable irToMachineCode: IrToMachineCode option = None

    member _.SourceToAst = sourceToAst
    member _.AstToIr = astToIr
    member _.IrToIr = irToIr |> Seq.toList

    member _.IrToMachineCode
        with get () = irToMachineCode
        and set value = irToMachineCode <- value

    static member New() = SourceMapChain()

    member _.AddOptimizerPass(segment: IrToIr) =
        if isNull (box segment) then
            nullArg "segment"

        irToIr.Add segment

    member _.SourceToMc(position: SourcePosition) =
        match irToMachineCode with
        | None -> None
        | Some machineCode ->
            let astNodeId =
                sourceToAst.Entries
                |> List.tryFind (fun entry ->
                    entry.Position.File = position.File
                    && entry.Position.Line = position.Line
                    && entry.Position.Column = position.Column)
                |> Option.map _.AstNodeId

            match astNodeId with
            | None -> None
            | Some nodeId ->
                match astToIr.LookupByAstNodeId nodeId with
                | None -> None
                | Some irIds ->
                    let mutable currentIds = irIds

                    for passSegment in irToIr do
                        currentIds <-
                            currentIds
                            |> List.collect (fun irId ->
                                if passSegment.Deleted.Contains irId then
                                    []
                                else
                                    match passSegment.LookupByOriginalId irId with
                                    | Some newIds -> newIds
                                    | None -> [])

                    if List.isEmpty currentIds then
                        None
                    else
                        let results =
                            currentIds
                            |> List.choose (fun irId ->
                                let offset, length = machineCode.LookupByIrId irId

                                if offset >= 0L then
                                    Some
                                        { IrId = irId
                                          MachineCodeOffset = offset
                                          MachineCodeLength = length }
                                else
                                    None)

                        if List.isEmpty results then None else Some results

    member _.McToSource(machineCodeOffset: int64) =
        match irToMachineCode with
        | None -> None
        | Some machineCode ->
            let mutable currentId = machineCode.LookupByMachineCodeOffset machineCodeOffset

            if currentId = -1L then
                None
            else
                let mutable traced = true

                for passSegment in irToIr |> Seq.rev do
                    if traced then
                        let originalId = passSegment.LookupByNewId currentId

                        if originalId = -1L then
                            traced <- false
                        else
                            currentId <- originalId

                if not traced then
                    None
                else
                    let astNodeId = astToIr.LookupByIrId currentId

                    if astNodeId = -1 then
                        None
                    else
                        sourceToAst.LookupByNodeId astNodeId

[<RequireQualifiedAccess>]
module CompilerSourceMapPackage =
    [<Literal>]
    let Version = "0.1.0"
