namespace CodingAdventures.JitCompiler.FSharp

open System
open System.Collections.Generic

type TargetIsa =
    | RiscV
    | Arm
    | X86

type JitCompilerConfig =
    { Target: TargetIsa
      HotThreshold: uint64 }

[<RequireQualifiedAccess>]
module JitCompilerConfig =
    let create target hotThreshold =
        if hotThreshold = 0UL then
            invalidArg (nameof hotThreshold) "hotThreshold must be greater than zero."

        { Target = target
          HotThreshold = hotThreshold }

type HotPathProfile =
    { BytecodeOffset: int
      ExecutionCount: uint64
      IsHot: bool }

type NativeBlock =
    { BytecodeOffset: int
      Target: TargetIsa
      MachineCode: byte list
      Assumptions: string list }

type JitCompiler(config: JitCompilerConfig) =
    do
        if isNull (box config) then
            nullArg (nameof config)

    let executionCounts = SortedDictionary<int, uint64>()
    let nativeBlocks = SortedDictionary<int, NativeBlock>()

    let validateOffset bytecodeOffset =
        if bytecodeOffset < 0 then
            invalidArg (nameof bytecodeOffset) "bytecodeOffset must be zero or greater."

    static member VERSION = "0.1.0"

    member _.Config = config

    member _.ObserveExecution(bytecodeOffset: int) =
        validateOffset bytecodeOffset
        let mutable count = 0UL
        executionCounts.TryGetValue(bytecodeOffset, &count) |> ignore
        count <- count + 1UL
        executionCounts[bytecodeOffset] <- count
        count = config.HotThreshold

    member _.Profile(bytecodeOffset: int) =
        validateOffset bytecodeOffset
        let mutable executionCount = 0UL
        if executionCounts.TryGetValue(bytecodeOffset, &executionCount) then
            Some
                { BytecodeOffset = bytecodeOffset
                  ExecutionCount = executionCount
                  IsHot = executionCount >= config.HotThreshold }
        else
            None

    member _.InstallShellBlock(bytecodeOffset: int, assumptions: string list) =
        validateOffset bytecodeOffset
        if isNull (box assumptions) then
            nullArg (nameof assumptions)

        let block =
            { BytecodeOffset = bytecodeOffset
              Target = config.Target
              MachineCode = []
              Assumptions = assumptions }

        nativeBlocks[bytecodeOffset] <- block
        block

    member _.HasNativeBlock(bytecodeOffset: int) =
        validateOffset bytecodeOffset
        nativeBlocks.ContainsKey bytecodeOffset

    member _.GetNativeBlock(bytecodeOffset: int) =
        validateOffset bytecodeOffset
        let mutable block = Unchecked.defaultof<NativeBlock>
        if nativeBlocks.TryGetValue(bytecodeOffset, &block) then Some block else None

    member _.Deoptimize(bytecodeOffset: int) =
        validateOffset bytecodeOffset
        let mutable block = Unchecked.defaultof<NativeBlock>
        if nativeBlocks.TryGetValue(bytecodeOffset, &block) then
            nativeBlocks.Remove bytecodeOffset |> ignore
            Some block
        else
            None
