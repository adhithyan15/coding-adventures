namespace CodingAdventures.JitCompiler.Tests

open System
open CodingAdventures.JitCompiler.FSharp
open Xunit

module JitCompilerTests =
    [<Fact>]
    let ``version exists`` () =
        Assert.Equal("0.1.0", JitCompiler.VERSION)

    [<Fact>]
    let ``config validates hot threshold`` () =
        let config = JitCompilerConfig.create RiscV 3UL

        Assert.Equal(RiscV, config.Target)
        Assert.Equal(3UL, config.HotThreshold)
        Assert.Throws<ArgumentException>(fun () -> JitCompilerConfig.create Arm 0UL |> ignore) |> ignore

    [<Fact>]
    let ``path becomes hot exactly at threshold`` () =
        let jit = JitCompiler(JitCompilerConfig.create RiscV 3UL)

        Assert.False(jit.ObserveExecution 24)
        Assert.False(jit.ObserveExecution 24)
        Assert.True(jit.ObserveExecution 24)
        Assert.False(jit.ObserveExecution 24)

    [<Fact>]
    let ``profile reports execution count and hotness`` () =
        let jit = JitCompiler(JitCompilerConfig.create Arm 2UL)

        Assert.True((jit.Profile 8).IsNone)
        jit.ObserveExecution 8 |> ignore
        let profile = (jit.Profile 8).Value
        Assert.Equal(1UL, profile.ExecutionCount)
        Assert.False(profile.IsHot)

        jit.ObserveExecution 8 |> ignore
        let hotProfile = (jit.Profile 8).Value
        Assert.Equal(2UL, hotProfile.ExecutionCount)
        Assert.True(hotProfile.IsHot)

    [<Fact>]
    let ``shell block installation uses configured target`` () =
        let jit = JitCompiler(JitCompilerConfig.create X86 5UL)

        let block = jit.InstallShellBlock(32, [ "locals stay integers" ])

        Assert.Equal(32, block.BytecodeOffset)
        Assert.Equal(X86, block.Target)
        Assert.Empty(block.MachineCode)
        Assert.Equal<string list>([ "locals stay integers" ], block.Assumptions)
        Assert.True(jit.HasNativeBlock 32)
        Assert.Equal(Some block, jit.GetNativeBlock 32)

    [<Fact>]
    let ``deoptimize removes native block`` () =
        let jit = JitCompiler(JitCompilerConfig.create RiscV 10UL)
        jit.InstallShellBlock(99, [ "shape stays stable" ]) |> ignore

        let block = (jit.Deoptimize 99).Value

        Assert.Equal(99, block.BytecodeOffset)
        Assert.False(jit.HasNativeBlock 99)
        Assert.True((jit.GetNativeBlock 99).IsNone)
        Assert.True((jit.Deoptimize 99).IsNone)

    [<Fact>]
    let ``invalid arguments are rejected`` () =
        Assert.Throws<ArgumentNullException>(fun () -> JitCompiler(Unchecked.defaultof<JitCompilerConfig>) |> ignore) |> ignore
        let jit = JitCompiler(JitCompilerConfig.create Arm 1UL)
        Assert.Throws<ArgumentException>(fun () -> jit.ObserveExecution -1 |> ignore) |> ignore
        Assert.Throws<ArgumentException>(fun () -> jit.Profile -1 |> ignore) |> ignore
        Assert.Throws<ArgumentException>(fun () -> jit.HasNativeBlock -1 |> ignore) |> ignore
        Assert.Throws<ArgumentException>(fun () -> jit.GetNativeBlock -1 |> ignore) |> ignore
        Assert.Throws<ArgumentException>(fun () -> jit.Deoptimize -1 |> ignore) |> ignore
        Assert.Throws<ArgumentNullException>(fun () -> jit.InstallShellBlock(1, Unchecked.defaultof<string list>) |> ignore) |> ignore
