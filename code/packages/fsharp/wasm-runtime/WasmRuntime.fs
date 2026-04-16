namespace CodingAdventures.WasmRuntime.FSharp

open System
open System.Collections.Generic
open System.Text
open CodingAdventures.WasmExecution.FSharp
open CodingAdventures.WasmModuleParser.FSharp
open CodingAdventures.WasmTypes.FSharp
open CodingAdventures.WasmValidator.FSharp

module Version =
    [<Literal>]
    let VERSION = "0.1.0"

type ProcExitError(exitCode: int) =
    inherit Exception(sprintf "proc_exit(%d)" exitCode)

    member _.ExitCode = exitCode

type IWasiClock =
    abstract member RealtimeNanoseconds: unit -> int64
    abstract member MonotonicNanoseconds: unit -> int64

type IWasiRandom =
    abstract member FillBytes: byte array -> unit

type SystemClock() =
    interface IWasiClock with
        member _.RealtimeNanoseconds() = DateTimeOffset.UtcNow.ToUnixTimeMilliseconds() * 1_000_000L
        member _.MonotonicNanoseconds() = DateTime.UtcNow.Ticks * 100L

type SystemRandom() =
    interface IWasiRandom with
        member _.FillBytes(buffer: byte array) = Random.Shared.NextBytes(buffer)

type WasiConfig =
    {
        Args: string list
        Env: Map<string, string>
        Stdout: string -> unit
        Stderr: string -> unit
        Clock: IWasiClock
        Random: IWasiRandom
    }

[<RequireQualifiedAccess>]
module WasiConfig =
    let defaultConfig =
        {
            Args = []
            Env = Map.empty
            Stdout = ignore
            Stderr = ignore
            Clock = SystemClock() :> IWasiClock
            Random = SystemRandom() :> IWasiRandom
        }

type WasiStub(?config: WasiConfig) =
    let mutable memory: LinearMemory option = None
    let configValue = defaultArg config WasiConfig.defaultConfig

    member _.SetMemory(newMemory: LinearMemory option) =
        memory <- newMemory

    interface IHostInterface with
        member _.ResolveFunction(moduleName, name) =
            if moduleName <> "wasi_snapshot_preview1" then
                None
            else
                match name with
                | "proc_exit" ->
                    Some(
                        HostFunction(
                            WasmTypes.makeFuncType [ ValueType.I32 ] [],
                            fun args -> raise (ProcExitError(args.Head |> WasmValue.asI32))
                        )
                        :> IHostFunction
                    )
                | "fd_write" ->
                    Some(
                        HostFunction(
                            WasmTypes.makeFuncType [ ValueType.I32; ValueType.I32; ValueType.I32; ValueType.I32 ] [ ValueType.I32 ],
                            fun args ->
                                match memory with
                                | None -> [ I32 52 ]
                                | Some linearMemory ->
                                    let fd = args[0] |> WasmValue.asI32
                                    let iovsPtr = args[1] |> WasmValue.asI32
                                    let iovsLen = args[2] |> WasmValue.asI32
                                    let nwrittenPtr = args[3] |> WasmValue.asI32
                                    let mutable totalBytes = 0

                                    for index in 0 .. iovsLen - 1 do
                                        let bufferPtr = linearMemory.LoadI32(iovsPtr + index * 8)
                                        let bufferLen = linearMemory.LoadI32(iovsPtr + index * 8 + 4)
                                        let text = linearMemory.ReadBytes(bufferPtr, bufferLen) |> Encoding.UTF8.GetString
                                        totalBytes <- totalBytes + bufferLen
                                        if fd = 1 then
                                            configValue.Stdout text
                                        elif fd = 2 then
                                            configValue.Stderr text

                                    linearMemory.StoreI32(nwrittenPtr, totalBytes)
                                    [ I32 0 ]
                        )
                        :> IHostFunction
                    )
                | _ ->
                    Some(
                        HostFunction(
                            WasmTypes.makeFuncType [] [ ValueType.I32 ],
                            fun _ -> [ I32 52 ]
                        )
                        :> IHostFunction
                    )

type WasmInstance =
    {
        ValidatedModule: ValidatedModule
        Engine: WasmExecutionEngine
        Exports: Map<string, Export>
    }

type WasmRuntime(?hostInterface: IHostInterface) =
    let parser = WasmModuleParser()
    let host = hostInterface

    member _.Load(wasmBytes: byte array) = parser.Parse(wasmBytes)
    member _.Validate(moduleValue: WasmModule) = WasmValidator.validate moduleValue

    member this.Instantiate(wasmBytes: byte array) =
        this.Instantiate(this.Load(wasmBytes))

    member _.Instantiate(moduleValue: WasmModule) =
        let validated = WasmValidator.validate moduleValue

        let importedFunctions = ResizeArray<IHostFunction option>()

        for importEntry in moduleValue.Imports do
            match importEntry.Kind with
            | ExternalKind.FUNCTION ->
                let resolved =
                    match host with
                    | Some current -> current.ResolveFunction(importEntry.ModuleName, importEntry.Name)
                    | None -> None

                match resolved with
                | Some hostFunction -> importedFunctions.Add(Some hostFunction)
                | None -> raise (TrapError(sprintf "Missing host function import %s.%s" importEntry.ModuleName importEntry.Name))
            | _ ->
                raise (TrapError(sprintf "Imported %A values are not implemented in the F# runtime yet" importEntry.Kind))

        let memory =
            if moduleValue.Memories.Count > 0 then
                let memoryType = moduleValue.Memories[0]
                Some(LinearMemory(memoryType.Limits.Min, ?maxPages = memoryType.Limits.Max))
            else
                None

        match host with
        | Some (:? WasiStub as wasi) -> wasi.SetMemory(memory)
        | _ -> ()

        let globals = ResizeArray<WasmValue>()
        let globalTypes = ResizeArray<GlobalType>()

        for globalValue in moduleValue.Globals do
            let value = WasmExecution.evaluateConstExpr globalValue.InitExpr (globals |> Seq.toList) |> List.head
            globals.Add(value)
            globalTypes.Add(globalValue.GlobalType)

        match memory with
        | Some linearMemory ->
            for dataSegment in moduleValue.Data do
                let offset = WasmExecution.evaluateConstExpr dataSegment.OffsetExpr [] |> List.head |> WasmValue.asI32
                linearMemory.WriteBytes(offset, dataSegment.Data)
        | None -> ()

        let tables =
            moduleValue.Tables
            |> Seq.map (fun tableType -> Table(tableType.Limits.Min))
            |> Seq.toList

        for element in moduleValue.Elements do
            if not (List.isEmpty tables) then
                let tableOffset = WasmExecution.evaluateConstExpr element.OffsetExpr [] |> List.head |> WasmValue.asI32
                for index in 0 .. element.FunctionIndices.Length - 1 do
                    tables[element.TableIndex][tableOffset + index] <- Some element.FunctionIndices[index]

        let funcBodies =
            [ for _ in importedFunctions -> None ]
            @ (moduleValue.Code |> Seq.map Some |> Seq.toList)

        let hostFunctions =
            (importedFunctions |> Seq.toList)
            @ [ for _ in moduleValue.Code -> None ]

        let engine =
            WasmExecutionEngine(
                {
                    Memory = memory
                    Tables = tables
                    Globals = globals |> Seq.toList
                    GlobalTypes = globalTypes |> Seq.toList
                    FuncTypes = validated.FuncTypes
                    FuncBodies = funcBodies
                    HostFunctions = hostFunctions
                }
            )

        let instance =
            {
                ValidatedModule = validated
                Engine = engine
                Exports = moduleValue.Exports |> Seq.map (fun exportEntry -> exportEntry.Name, exportEntry) |> Map.ofSeq
            }

        match moduleValue.Start with
        | Some startIndex -> engine.CallFunction(startIndex, []) |> ignore
        | None -> ()

        instance

    member _.Call(instance: WasmInstance, exportName: string, [<ParamArray>] args: int array) =
        let exportEntry =
            match instance.Exports.TryFind exportName with
            | Some value -> value
            | None -> raise (TrapError(sprintf "Module does not export '%s'" exportName))

        if exportEntry.Kind <> ExternalKind.FUNCTION then
            raise (TrapError(sprintf "Export '%s' is not a function" exportName))

        let funcType = instance.ValidatedModule.FuncTypes[exportEntry.Index]
        let wasmArgs =
            args
            |> Array.mapi (fun index value ->
                if funcType.Params[index] <> ValueType.I32 then
                    raise (TrapError("The convenience Call overload currently supports only i32 arguments"))
                I32 value)
            |> Array.toList

        instance.Engine.CallFunction(exportEntry.Index, wasmArgs)
        |> List.mapi (fun index result ->
            match funcType.Results[index], result with
            | ValueType.I32, I32 value -> box value
            | ValueType.I64, I64 value -> box value
            | ValueType.F32, F32 value -> box value
            | ValueType.F64, F64 value -> box value
            | _ -> raise (TrapError("Unexpected result type")))

    member this.LoadAndRun(wasmBytes: byte array, exportName: string, [<ParamArray>] args: int array) =
        let instance = this.Instantiate(wasmBytes)
        this.Call(instance, exportName, args)
