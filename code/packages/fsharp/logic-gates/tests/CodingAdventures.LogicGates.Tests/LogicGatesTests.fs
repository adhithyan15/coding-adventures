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
        Assert.ThrowsAny<ArgumentOutOfRangeException>(fun () -> LogicGates.notGate -1 |> ignore) |> ignore
        Assert.ThrowsAny<ArgumentOutOfRangeException>(fun () -> LogicGates.andGate 2 1 |> ignore) |> ignore
        Assert.ThrowsAny<ArgumentOutOfRangeException>(fun () -> LogicGates.orGate 0 -1 |> ignore) |> ignore
        Assert.ThrowsAny<ArgumentOutOfRangeException>(fun () -> LogicGates.xorGate 0 2 |> ignore) |> ignore
        Assert.Throws<ArgumentException>(fun () -> LogicGates.andN [ 1 ] |> ignore) |> ignore
        Assert.Throws<ArgumentException>(fun () -> LogicGates.orN [] |> ignore) |> ignore
        Assert.ThrowsAny<ArgumentOutOfRangeException>(fun () -> LogicGates.xorN [ 0; 2 ] |> ignore) |> ignore

    [<Fact>]
    member _.``De Morgan relationships hold``() =
        for a, b, _, _, _, _, _, _, _ in truthTable do
            Assert.Equal(LogicGates.nand a b, LogicGates.orGate (LogicGates.notGate a) (LogicGates.notGate b))
            Assert.Equal(LogicGates.nor a b, LogicGates.andGate (LogicGates.notGate a) (LogicGates.notGate b))
            Assert.Equal(LogicGates.xnor a b, LogicGates.notGate (LogicGates.xorGate a b))

type NaryTests() =
    let gates : (string * int * (int list -> int)) list =
        [ "andN", 2, LogicGates.andN; "orN", 2, LogicGates.orN; "xorN", 0, LogicGates.xorN ]

    let expected name (l: int list) =
        match name with
        | "andN" -> if List.forall ((=) 1) l then 1 else 0
        | "orN" -> if List.exists ((=) 1) l then 1 else 0
        | _ -> (List.sum l) % 2

    [<Fact>]
    member _.NullRejected() =
        for (_, _, f) in gates do
            let ex = Assert.Throws<ArgumentNullException>(fun () -> f Unchecked.defaultof<int list> |> ignore)
            Assert.Equal("inputs", ex.ParamName)

    [<Fact>]
    member _.MaxInputsIs65536() = Assert.Equal(65536, LogicGates.maxInputs)

    [<Fact>]
    member _.MinLengthRules() =
        for (_, m, f) in gates do
            if m > 0 then
                for n in 0 .. m - 1 do
                    let ex = Assert.Throws<ArgumentException>(fun () -> f (List.replicate n 1) |> ignore)
                    Assert.Equal("inputs", ex.ParamName)
                f (List.replicate m 1) |> ignore
        Assert.Equal(0, LogicGates.xorN [])

    [<Fact>]
    member _.CapAcceptedAndRejected() =
        for (_, _, f) in gates do
            f (List.replicate 65536 1) |> ignore
            let ex = Assert.Throws<ArgumentException>(fun () -> f (List.replicate 65537 1) |> ignore)
            Assert.Equal("inputs", ex.ParamName)
            Assert.IsNotType<ArgumentOutOfRangeException>(ex)

    [<Fact>]
    member _.ElementIndexNamed() =
        for (_, m, f) in gates do
            let l = List.replicate (max m 3) 1 @ [ 7; 9 ]
            let idx = List.findIndex (fun v -> v = 7) l
            let ex = Assert.Throws<InvalidBitException>(fun () -> f l |> ignore)
            Assert.Equal(sprintf "inputs[%d]" idx, ex.ParamName)

    [<Fact>]
    member _.ShapeWinsOverBitError() =
        for (_, _, f) in gates do
            let ex = Assert.Throws<ArgumentException>(fun () -> f (2 :: List.replicate 65537 1) |> ignore)
            Assert.IsNotType<ArgumentOutOfRangeException>(ex)
        let ex = Assert.Throws<ArgumentException>(fun () -> LogicGates.andN [ 5 ] |> ignore)
        Assert.IsNotType<ArgumentOutOfRangeException>(ex)

    [<Fact>]
    member _.ExhaustiveSmallLists() =
        let vals = [ -1; 0; 1; 2 ]
        let rec lists n =
            if n = 0 then [ [] ] else [ for v in vals do for t in lists (n - 1) -> v :: t ]
        for len in 0 .. 8 do
            for l in lists len do
                for (name, m, f) in gates do
                    if len < m then
                        Assert.Throws<ArgumentException>(fun () -> f l |> ignore) |> ignore
                    else
                        match List.tryFindIndex (fun v -> v <> 0 && v <> 1) l with
                        | Some i ->
                            let ex = Assert.Throws<InvalidBitException>(fun () -> f l |> ignore)
                            Assert.Equal(sprintf "inputs[%d]" i, ex.ParamName)
                        | None -> Assert.Equal(expected name l, f l)

type NaryLateIndexTests() =
    [<Fact>]
    member _.LateBadElementNamedByIndex() =
        let l = List.replicate 65535 1 @ [ 2 ]
        for f in [ LogicGates.andN; LogicGates.orN; LogicGates.xorN ] do
            let ex = Assert.Throws<InvalidBitException>(fun () -> f l |> ignore)
            Assert.Equal("inputs[65535]", ex.ParamName)
