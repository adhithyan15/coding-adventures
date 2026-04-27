namespace CodingAdventures.PaintVm

open System
open System.Collections.Generic
open CodingAdventures.PaintInstructions
open CodingAdventures.PixelContainer

[<RequireQualifiedAccess>]
module PaintVmPackage =
    [<Literal>]
    let VERSION = "0.1.0"

type UnknownInstructionError(kind: string) =
    inherit Exception(sprintf "No handler registered for instruction kind: '%s'" kind)
    member _.Kind = kind

type DuplicateHandlerError(kind: string) =
    inherit Exception(sprintf "Handler already registered for instruction kind: '%s'" kind)
    member _.Kind = kind

type ExportNotSupportedError(backendName: string) =
    inherit Exception(sprintf "export() is not supported by the %s backend. Use a backend that supports pixel readback." backendName)

type NullContextError() =
    inherit Exception("execute() and patch() require a non-null context")

type ExportOptions =
    {
        Scale: float
        Channels: int
        BitDepth: int
        ColorSpace: string
    }

type PatchCallbacks =
    {
        OnDelete: (PaintInstruction -> unit) option
        OnInsert: (PaintInstruction -> int -> unit) option
        OnUpdate: (PaintInstruction -> PaintInstruction -> unit) option
    }

type PaintHandler<'Context> = PaintInstruction -> 'Context -> PaintVM<'Context> -> unit

and PaintVM<'Context>
    (
        clear: 'Context -> string -> float -> float -> unit,
        ?exporter: PaintScene -> PaintVM<'Context> -> ExportOptions -> PixelContainer
    ) =

    let table = Dictionary<string, PaintHandler<'Context>>()
    let exporter = exporter

    let ensureContext (context: 'Context) =
        if obj.ReferenceEquals(context, null) then
            raise (NullContextError())

    member _.Register(kind: string, handler: PaintHandler<'Context>) =
        if String.IsNullOrWhiteSpace(kind) then
            invalidArg "kind" "kind must not be empty"

        if isNull (box handler) then
            nullArg "handler"

        if table.ContainsKey(kind) then
            raise (DuplicateHandlerError(kind))

        table[kind] <- handler

    member this.Dispatch(instruction: PaintInstruction, context: 'Context) =
        let mutable handler = Unchecked.defaultof<PaintHandler<'Context>>

        let foundSpecific = table.TryGetValue(instruction.Kind, &handler)
        let mutable foundWildcard = false

        if not foundSpecific then
            let mutable wildcardHandler = Unchecked.defaultof<PaintHandler<'Context>>

            if table.TryGetValue("*", &wildcardHandler) then
                handler <- wildcardHandler
                foundWildcard <- true

        if foundSpecific || foundWildcard then
            handler instruction context this
        else
            raise (UnknownInstructionError(instruction.Kind))

    member this.Execute(scene: PaintScene, context: 'Context) =
        ensureContext context
        clear context scene.Background scene.Width scene.Height
        scene.Instructions |> List.iter (fun instruction -> this.Dispatch(instruction, context))

    member this.Patch(oldScene: PaintScene, newScene: PaintScene, context: 'Context, ?callbacks: PatchCallbacks) =
        ensureContext context

        match callbacks with
        | None -> this.Execute(newScene, context)
        | Some callbacks ->
            let oldById =
                oldScene.Instructions
                |> List.choose (fun instruction -> instruction.Id |> Option.map (fun id -> id, instruction))
                |> Map.ofList

            let newById =
                newScene.Instructions
                |> List.choose (fun instruction -> instruction.Id |> Option.map (fun id -> id, instruction))
                |> Map.ofList

            for KeyValue(id, instruction) in oldById do
                if not (newById.ContainsKey(id)) then
                    callbacks.OnDelete |> Option.iter (fun onDelete -> onDelete instruction)

            for index, nextInstruction in newScene.Instructions |> List.indexed do
                match nextInstruction.Id with
                | Some id when oldById.ContainsKey(id) ->
                    let oldInstruction = oldById[id]

                    if not (PaintVM<'Context>.DeepEqual(box nextInstruction, box oldInstruction)) then
                        callbacks.OnUpdate |> Option.iter (fun onUpdate -> onUpdate oldInstruction nextInstruction)
                | _ ->
                    if index < oldScene.Instructions.Length then
                        let positionalOld = oldScene.Instructions[index]

                        if not (PaintVM<'Context>.DeepEqual(box nextInstruction, box positionalOld)) then
                            callbacks.OnUpdate |> Option.iter (fun onUpdate -> onUpdate positionalOld nextInstruction)
                    else
                        callbacks.OnInsert |> Option.iter (fun onInsert -> onInsert nextInstruction index)

    member this.Export(scene: PaintScene, ?options: ExportOptions) =
        match exporter with
        | Some export ->
            export scene
                this
                (defaultArg options
                    {
                        Scale = 1.0
                        Channels = 4
                        BitDepth = 8
                        ColorSpace = "srgb"
                    })
        | None -> raise (ExportNotSupportedError("this"))

    member _.RegisteredKinds() =
        table.Keys |> Seq.sort |> Seq.toArray

    static member DeepEqual(left: obj, right: obj) =
        if obj.ReferenceEquals(left, right) then
            true
        elif isNull left || isNull right then
            false
        elif left.GetType() <> right.GetType() then
            false
        else
            left.Equals(right)
