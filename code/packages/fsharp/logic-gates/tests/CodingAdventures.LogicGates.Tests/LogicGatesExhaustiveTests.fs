namespace CodingAdventures.LogicGates.Tests

open System
open Xunit
open CodingAdventures.LogicGates

module LogicGatesExhaustiveTests =

    // Literal truth tables (written out by hand, not derived from the gates).
    // Columns: a, b, not a, and, or, xor, nand, nor, xnor
    let table =
        [
            0, 0, 1, 0, 0, 0, 1, 1, 1
            0, 1, 1, 0, 1, 1, 1, 0, 0
            1, 0, 0, 0, 1, 1, 1, 0, 0
            1, 1, 0, 1, 1, 0, 0, 0, 1
        ]

    /// Every gate as (name, arity, function over (a, b); unary gates ignore b).
    let gates : (string * int * (int -> int -> int)) list =
        [
            "notGate", 1, (fun a _ -> LogicGates.notGate a)
            "andGate", 2, LogicGates.andGate
            "orGate", 2, LogicGates.orGate
            "xorGate", 2, LogicGates.xorGate
            "nand", 2, LogicGates.nand
            "nor", 2, LogicGates.nor
            "xnor", 2, LogicGates.xnor
            "nandNot", 1, (fun a _ -> LogicGates.nandNot a)
            "nandAnd", 2, LogicGates.nandAnd
            "nandOr", 2, LogicGates.nandOr
            "nandXor", 2, LogicGates.nandXor
        ]

    /// Every int in [-1024, 1024] other than 0 and 1, plus int32 extremes.
    let invalidValues =
        [ for v in -1024 .. 1024 do
            if v <> 0 && v <> 1 then
                yield v
          yield Int32.MinValue
          yield Int32.MinValue + 1
          yield Int32.MaxValue - 1
          yield Int32.MaxValue ]

    [<Fact>]
    let ``literal truth tables for all eleven gates`` () =
        for a, b, n, an, o, x, na, no, xn in table do
            Assert.Equal(n, LogicGates.notGate a)
            Assert.Equal(an, LogicGates.andGate a b)
            Assert.Equal(o, LogicGates.orGate a b)
            Assert.Equal(x, LogicGates.xorGate a b)
            Assert.Equal(na, LogicGates.nand a b)
            Assert.Equal(no, LogicGates.nor a b)
            Assert.Equal(xn, LogicGates.xnor a b)
            Assert.Equal(n, LogicGates.nandNot a)
            Assert.Equal(an, LogicGates.nandAnd a b)
            Assert.Equal(o, LogicGates.nandOr a b)
            Assert.Equal(x, LogicGates.nandXor a b)

    [<Fact>]
    let ``NAND-built gates equal direct gates`` () =
        for a, b, _, _, _, _, _, _, _ in table do
            Assert.Equal(LogicGates.notGate a, LogicGates.nandNot a)
            Assert.Equal(LogicGates.andGate a b, LogicGates.nandAnd a b)
            Assert.Equal(LogicGates.orGate a b, LogicGates.nandOr a b)
            Assert.Equal(LogicGates.xorGate a b, LogicGates.nandXor a b)

    [<Fact>]
    let ``valid inputs never raise and results are exactly 0 or 1`` () =
        for _, arity, f in gates do
            for a in [ 0; 1 ] do
                for b in [ 0; 1 ] do
                    let r = f a b
                    Assert.True(r = 0 || r = 1)
            ignore arity

    [<Fact>]
    let ``every invalid int in every position raises InvalidBitException`` () =
        let mutable rejected = 0
        for name, arity, f in gates do
            for v in invalidValues do
                // First position invalid, second valid (0 and 1).
                for other in [ 0; 1 ] do
                    let ex = Assert.Throws<InvalidBitException>(fun () -> f v other |> ignore)
                    Assert.Equal("a", ex.Parameter)
                    Assert.Equal(v, ex.Value)
                    Assert.Equal("a", ex.ParamName)
                    Assert.IsAssignableFrom<ArgumentOutOfRangeException>(ex) |> ignore
                    rejected <- rejected + 1
                if arity = 2 then
                    // Second position invalid, first valid.
                    for other in [ 0; 1 ] do
                        let ex = Assert.Throws<InvalidBitException>(fun () -> f other v |> ignore)
                        Assert.Equal("b", ex.Parameter)
                        Assert.Equal(v, ex.Value)
                        rejected <- rejected + 1
                    // Both invalid: the first parameter is reported.
                    let ex = Assert.Throws<InvalidBitException>(fun () -> f v v |> ignore)
                    Assert.Equal("a", ex.Parameter)
                    rejected <- rejected + 1
            ignore name
        Assert.True(rejected > 0)

    [<Fact>]
    let ``NAND-built gates report the public parameter not an inner one`` () =
        let ex = Assert.Throws<InvalidBitException>(fun () -> LogicGates.nandOr 1 2 |> ignore)
        Assert.Equal("b", ex.Parameter)
        Assert.Equal(2, ex.Value)
        let ex = Assert.Throws<InvalidBitException>(fun () -> LogicGates.nandXor 2 1 |> ignore)
        Assert.Equal("a", ex.Parameter)
        let ex = Assert.Throws<InvalidBitException>(fun () -> LogicGates.nandAnd 1 -1 |> ignore)
        Assert.Equal("b", ex.Parameter)
        let ex = Assert.Throws<InvalidBitException>(fun () -> LogicGates.nandNot 5 |> ignore)
        Assert.Equal("a", ex.Parameter)

    [<Fact>]
    let ``multi-input gates reject null lists and bad arity`` () =
        let nullList = Unchecked.defaultof<int list>
        Assert.Throws<ArgumentNullException>(fun () -> LogicGates.andN nullList |> ignore) |> ignore
        Assert.Throws<ArgumentNullException>(fun () -> LogicGates.orN nullList |> ignore) |> ignore
        Assert.Throws<ArgumentNullException>(fun () -> LogicGates.xorN nullList |> ignore) |> ignore
        Assert.ThrowsAny<ArgumentException>(fun () -> LogicGates.andN [ 1 ] |> ignore) |> ignore
        Assert.ThrowsAny<ArgumentException>(fun () -> LogicGates.orN [] |> ignore) |> ignore
        Assert.Throws<InvalidBitException>(fun () -> LogicGates.andN [ 1; 1; 7 ] |> ignore) |> ignore
