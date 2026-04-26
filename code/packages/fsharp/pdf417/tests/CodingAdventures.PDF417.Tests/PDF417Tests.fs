module CodingAdventures.PDF417.Tests

open System
open Xunit
open CodingAdventures.PDF417.FSharp

[<Fact>]
let ``VERSION is 0.1.0`` () =
    Assert.Equal("0.1.0", PDF417.VERSION)

[<Fact>]
let ``default options match PDF417 defaults`` () =
    Assert.Equal(None, PDF417.defaultOptions.EccLevel)
    Assert.Equal(None, PDF417.defaultOptions.Columns)
    Assert.Equal(3, PDF417.defaultOptions.RowHeight)

[<Fact>]
let ``encode text returns a valid module grid`` () =
    let grid = PDF417.encode "HELLO WORLD"

    Assert.True(grid.Rows >= 3)
    Assert.True(grid.Cols >= 86)
    Assert.Equal(0, (grid.Cols - 69) % 17)
    Assert.True(grid.Modules.[0].[0])

[<Fact>]
let ``encode bytes and text agree for ASCII input`` () =
    let bytes = Text.Encoding.UTF8.GetBytes("TEST")
    let fromBytes = PDF417.encodeBytes bytes None
    let fromText = PDF417.encodeText "TEST" None

    Assert.Equal(fromBytes.Rows, fromText.Rows)
    Assert.Equal(fromBytes.Cols, fromText.Cols)
    for row in 0 .. fromBytes.Rows - 1 do
        for col in 0 .. fromBytes.Cols - 1 do
            Assert.Equal(fromBytes.Modules.[row].[col], fromText.Modules.[row].[col])

[<Fact>]
let ``custom options control columns and row height`` () =
    let options =
        { PDF417.defaultOptions with
            Columns = Some 5
            RowHeight = 1 }

    let grid = PDF417.encodeText "TEST" (Some options)

    Assert.Equal(69 + 17 * 5, grid.Cols)
    Assert.True(grid.Rows >= 3)

[<Fact>]
let ``invalid engine options bubble up`` () =
    let options =
        { PDF417.defaultOptions with
            EccLevel = Some 9 }

    Assert.Throws<CodingAdventures.PDF417.InvalidECCLevelException>(
        fun () -> PDF417.encodeText "A" (Some options) |> ignore)
    |> ignore

[<Fact>]
let ``null inputs are rejected by facade`` () =
    Assert.Throws<ArgumentNullException>(fun () -> PDF417.encodeText null None |> ignore) |> ignore
    Assert.Throws<ArgumentNullException>(fun () -> PDF417.encodeBytes null None |> ignore) |> ignore
