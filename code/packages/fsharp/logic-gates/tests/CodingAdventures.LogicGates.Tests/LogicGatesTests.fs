namespace CodingAdventures.LogicGates.Tests

open System
open Xunit
open CodingAdventures.LogicGates

type LogicGatesTests() =
    let truthTable =
        [|
            0, 0, 1, 0, 0, 0, 1, 1, 1
            0, 1, 1, 0, 1, 1, 1, 0, 0
            1, 0, 0, 0, 1, 1, 1, 0, 0
            1, 1, 0, 1, 1, 0, 0, 0, 1
        |]

    [<Fact>]
    member _.``Fundamental gates match truth tables``() =
        for a, b, notA, andValue, orValue, xorValue, nandValue, norValue, xnorValue in truthTable do
            Assert.Equal(notA, LogicGates.notGate a)
            Assert.Equal(andValue, LogicGates.andGate a b)
            Assert.Equal(orValue, LogicGates.orGate a b)
            Assert.Equal(xorValue, LogicGates.xorGate a b)
            Assert.Equal(nandValue, LogicGates.nand a b)
            Assert.Equal(norValue, LogicGates.nor a b)
            Assert.Equal(xnorValue, LogicGates.xnor a b)

    [<Fact>]
    member _.``NAND derived gates match direct gates``() =
        for a, b, _, _, _, _, _, _, _ in truthTable do
            Assert.Equal(LogicGates.notGate a, LogicGates.nandNot a)
            Assert.Equal(LogicGates.andGate a b, LogicGates.nandAnd a b)
            Assert.Equal(LogicGates.orGate a b, LogicGates.nandOr a b)
            Assert.Equal(LogicGates.xorGate a b, LogicGates.nandXor a b)

    [<Fact>]
    member _.``Multi input gates work``() =
        Assert.Equal(1, LogicGates.andN [ 1; 1; 1; 1 ])
        Assert.Equal(0, LogicGates.andN [ 1; 1; 0; 1 ])
        Assert.Equal(0, LogicGates.orN [ 0; 0; 0 ])
        Assert.Equal(1, LogicGates.orN [ 0; 0; 1; 0 ])
        Assert.Equal(0, LogicGates.xorN [])
        Assert.Equal(1, LogicGates.xorN [ 1 ])
        Assert.Equal(0, LogicGates.xorN [ 1; 1; 1; 1 ])
        Assert.Equal(1, LogicGates.xorN [ 1; 1; 1 ])

    [<Fact>]
    member _.``Invalid inputs are rejected``() =
        Assert.Throws<ArgumentOutOfRangeException>(fun () -> LogicGates.notGate -1 |> ignore) |> ignore
        Assert.Throws<ArgumentOutOfRangeException>(fun () -> LogicGates.andGate 2 1 |> ignore) |> ignore
        Assert.Throws<ArgumentOutOfRangeException>(fun () -> LogicGates.orGate 0 -1 |> ignore) |> ignore
        Assert.Throws<ArgumentOutOfRangeException>(fun () -> LogicGates.xorGate 0 2 |> ignore) |> ignore
        Assert.Throws<ArgumentException>(fun () -> LogicGates.andN [ 1 ] |> ignore) |> ignore
        Assert.Throws<ArgumentException>(fun () -> LogicGates.orN [] |> ignore) |> ignore
        Assert.Throws<ArgumentOutOfRangeException>(fun () -> LogicGates.xorN [ 0; 2 ] |> ignore) |> ignore

    [<Fact>]
    member _.``De Morgan relationships hold``() =
        for a, b, _, _, _, _, _, _, _ in truthTable do
            Assert.Equal(LogicGates.nand a b, LogicGates.orGate (LogicGates.notGate a) (LogicGates.notGate b))
            Assert.Equal(LogicGates.nor a b, LogicGates.andGate (LogicGates.notGate a) (LogicGates.notGate b))
            Assert.Equal(LogicGates.xnor a b, LogicGates.notGate (LogicGates.xorGate a b))
