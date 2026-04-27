namespace CodingAdventures.HuffmanTree.FSharp.Tests

open System
open CodingAdventures.HuffmanTree.FSharp
open Xunit

type HuffmanTreeTests() =
    [<Fact>]
    member _.``build rejects empty weights``() =
        let error = Assert.Throws<ArgumentException>(fun () -> HuffmanTree.Build [] |> ignore)
        Assert.Contains("weights must not be empty", error.Message)

    [<Fact>]
    member _.``build rejects non-positive frequencies``() =
        let error = Assert.Throws<ArgumentException>(fun () -> HuffmanTree.Build [ 42, 0 ] |> ignore)
        Assert.Contains("symbol=42, freq=0", error.Message)

    [<Fact>]
    member _.``single symbol tree uses zero code by convention``() =
        let tree = HuffmanTree.Build [ 65, 5 ]

        Assert.Equal(1, tree.SymbolCount())
        Assert.Equal(5, tree.Weight())
        Assert.Equal(0, tree.Depth())
        Assert.Equal("0", tree.CodeTable().[65])
        Assert.Equal("0", tree.CanonicalCodeTable().[65])
        Assert.Equal<int list>([ 65; 65; 65 ], tree.DecodeAll("000", 3))
        Assert.Equal<int list>([ 65 ], tree.DecodeAll("", 1))
        Assert.True(tree.IsValid())

    [<Fact>]
    member _.``classic three symbol example has deterministic codes``() =
        let tree = HuffmanTree.Build [ 65, 3; 66, 2; 67, 1 ]
        let codes = tree.CodeTable()

        Assert.Equal("0", codes.[65])
        Assert.Equal("10", codes.[67])
        Assert.Equal("11", codes.[66])
        Assert.Equal(Some "10", tree.CodeFor 67)
        Assert.Equal(None, tree.CodeFor 99)
        Assert.Equal<(int * string) list>([ 65, "0"; 67, "10"; 66, "11" ], tree.Leaves())
        Assert.True(tree.IsValid())

    [<Fact>]
    member _.``canonical code table sorts by length then symbol``() =
        let tree = HuffmanTree.Build [ 65, 3; 66, 2; 67, 1 ]
        let canonical = tree.CanonicalCodeTable()

        Assert.Equal("0", canonical.[65])
        Assert.Equal("10", canonical.[66])
        Assert.Equal("11", canonical.[67])

    [<Fact>]
    member _.``decodeAll throws when bits run out mid symbol``() =
        let tree = HuffmanTree.Build [ 65, 3; 66, 2; 67, 1 ]
        let error = Assert.Throws<InvalidOperationException>(fun () -> tree.DecodeAll("1", 1) |> ignore)
        Assert.Contains("exhausted", error.Message)

    [<Fact>]
    member _.``decodeAll rejects non-binary characters``() =
        let tree = HuffmanTree.Build [ 65, 3; 66, 2 ]
        let error = Assert.Throws<InvalidOperationException>(fun () -> tree.DecodeAll("2", 1) |> ignore)
        Assert.Contains("only '0' and '1'", error.Message)

    [<Fact>]
    member _.``two symbol tree weight depth and decode match tree shape``() =
        let tree = HuffmanTree.Build [ 65, 3; 66, 1 ]
        let codes = tree.CodeTable()
        let bits = codes.[66] + codes.[65] + codes.[66]

        Assert.Equal(2, tree.SymbolCount())
        Assert.Equal(4, tree.Weight())
        Assert.Equal(1, tree.Depth())
        Assert.Equal<int list>([ 66; 65; 66 ], tree.DecodeAll(bits, 3))
