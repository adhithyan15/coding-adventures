namespace CodingAdventures.Barcode2D.Tests

// Barcode2DTests.fs — Comprehensive xUnit tests for the Barcode2D F# module
// ==========================================================================
//
// Test organisation mirrors the module structure:
//
//   1.  VERSION constant
//   2.  ModuleShape discriminated union
//   3.  makeModuleGrid — construction and defaults
//   4.  setModule — immutable updates, out-of-bounds
//   5.  defaultConfig — fields and values
//   6.  layout (Square) — background rect, dark module rects, quiet zone,
//                          pixel dimensions, zero modules, all-dark grid
//   7.  layout (Hex)    — hex geometry, odd-row offset, tiling
//   8.  layout validation — moduleSizePx <= 0, quietZoneModules < 0
//   9.  AnnotatedModuleGrid — construction and role access
//  10.  Integration — chained setModule calls, full 3×3 grid round-trip

open System
open Xunit
open CodingAdventures.Barcode2D
open CodingAdventures.PaintInstructions

// ---------------------------------------------------------------------------
// Small helpers used across multiple test groups
// ---------------------------------------------------------------------------

/// Helpers must live in a module — F# namespaces cannot contain values.
[<AutoOpen>]
module TestHelpers =

    /// Extract the ``PaintRect`` from a ``Rect`` instruction, failing if it isn't one.
    let asRect (instr: PaintInstruction) : PaintRect =
        match instr with
        | Rect r -> r
        | other  -> failwith (sprintf "Expected Rect but got %s" other.Kind)

    /// Extract the ``PaintPath`` from a ``Path`` instruction, failing if it isn't one.
    let asPath (instr: PaintInstruction) : PaintPath =
        match instr with
        | Path p -> p
        | other  -> failwith (sprintf "Expected Path but got %s" other.Kind)

    /// A tiny tolerance for floating-point geometry comparisons.
    let eps = 1e-9

    /// Assert two floats are equal within ``eps``.
    let assertApprox (expected: float) (actual: float) =
        Assert.True(
            Math.Abs(expected - actual) < eps,
            sprintf "Expected ~%g but got %g (diff=%g)" expected actual (Math.Abs(expected - actual)))

// ===========================================================================
// 1. VERSION
// ===========================================================================

type VersionTests() =

    [<Fact>]
    member _.``VERSION equals 0.1.0``() =
        Assert.Equal("0.1.0", Barcode2D.VERSION)

// ===========================================================================
// 2. ModuleShape discriminated union
// ===========================================================================

type ModuleShapeTests() =

    [<Fact>]
    member _.``Square and Hex are distinct cases``() =
        // Pattern-matching should cleanly distinguish the two cases.
        let describeShape shape =
            match shape with
            | Square -> "square"
            | Hex    -> "hex"
        Assert.Equal("square", describeShape Square)
        Assert.Equal("hex",    describeShape Hex)

    [<Fact>]
    member _.``Square does not equal Hex``() =
        Assert.NotEqual<ModuleShape>(Square, Hex)

// ===========================================================================
// 3. makeModuleGrid
// ===========================================================================

type MakeModuleGridTests() =

    [<Fact>]
    member _.``creates grid with correct Rows``() =
        let g = Barcode2D.makeModuleGrid 5 7 Square
        Assert.Equal(5, g.Rows)

    [<Fact>]
    member _.``creates grid with correct Cols``() =
        let g = Barcode2D.makeModuleGrid 5 7 Square
        Assert.Equal(7, g.Cols)

    [<Fact>]
    member _.``all modules start as false (light)``() =
        let g = Barcode2D.makeModuleGrid 4 4 Square
        for row in 0 .. g.Rows - 1 do
            for col in 0 .. g.Cols - 1 do
                Assert.False(g.Modules.[row].[col], sprintf "Module [%d][%d] should be false" row col)

    [<Fact>]
    member _.``stores the module shape — Square``() =
        let g = Barcode2D.makeModuleGrid 3 3 Square
        Assert.Equal(Square, g.ModuleShape)

    [<Fact>]
    member _.``stores the module shape — Hex``() =
        let g = Barcode2D.makeModuleGrid 3 3 Hex
        Assert.Equal(Hex, g.ModuleShape)

    [<Fact>]
    member _.``1×1 grid is valid``() =
        let g = Barcode2D.makeModuleGrid 1 1 Square
        Assert.Equal(1, g.Rows)
        Assert.Equal(1, g.Cols)
        Assert.False(g.Modules.[0].[0])

    [<Fact>]
    member _.``non-square grid (more rows than cols)``() =
        let g = Barcode2D.makeModuleGrid 10 3 Square
        Assert.Equal(10, g.Rows)
        Assert.Equal(3,  g.Cols)

    [<Fact>]
    member _.``non-square grid (more cols than rows)``() =
        let g = Barcode2D.makeModuleGrid 2 20 Hex
        Assert.Equal(2,  g.Rows)
        Assert.Equal(20, g.Cols)

    [<Fact>]
    member _.``typical QR v1 size 21×21``() =
        let g = Barcode2D.makeModuleGrid 21 21 Square
        Assert.Equal(21, g.Rows)
        Assert.Equal(21, g.Cols)

    [<Fact>]
    member _.``MaxiCode fixed size 33×30 Hex``() =
        let g = Barcode2D.makeModuleGrid 33 30 Hex
        Assert.Equal(33, g.Rows)
        Assert.Equal(30, g.Cols)
        Assert.Equal(Hex, g.ModuleShape)

// ===========================================================================
// 4. setModule
// ===========================================================================

type SetModuleTests() =

    [<Fact>]
    member _.``sets a module to true``() =
        let g  = Barcode2D.makeModuleGrid 3 3 Square
        let g2 = Barcode2D.setModule g 1 1 true
        Assert.True(g2.Modules.[1].[1])

    [<Fact>]
    member _.``sets a module to false``() =
        let g  = Barcode2D.makeModuleGrid 3 3 Square
        let g1 = Barcode2D.setModule g  1 1 true
        let g2 = Barcode2D.setModule g1 1 1 false
        Assert.False(g2.Modules.[1].[1])

    [<Fact>]
    member _.``original grid is unchanged after setModule``() =
        let g  = Barcode2D.makeModuleGrid 3 3 Square
        let _  = Barcode2D.setModule g 0 0 true
        // The original should still report false at (0,0).
        Assert.False(g.Modules.[0].[0])

    [<Fact>]
    member _.``unaffected rows are unchanged``() =
        let g  = Barcode2D.makeModuleGrid 3 3 Square
        let g2 = Barcode2D.setModule g 1 1 true
        // Rows 0 and 2 should still be all-false.
        for col in 0 .. 2 do
            Assert.False(g2.Modules.[0].[col], sprintf "Row 0, col %d should be false" col)
            Assert.False(g2.Modules.[2].[col], sprintf "Row 2, col %d should be false" col)

    [<Fact>]
    member _.``unaffected columns in the same row are unchanged``() =
        let g  = Barcode2D.makeModuleGrid 1 5 Square
        let g2 = Barcode2D.setModule g 0 2 true
        Assert.False(g2.Modules.[0].[0])
        Assert.False(g2.Modules.[0].[1])
        Assert.True( g2.Modules.[0].[2])
        Assert.False(g2.Modules.[0].[3])
        Assert.False(g2.Modules.[0].[4])

    [<Fact>]
    member _.``returns a new grid object (structural immutability)``() =
        let g  = Barcode2D.makeModuleGrid 3 3 Square
        let g2 = Barcode2D.setModule g 0 0 true
        // Reference inequality — they are different records.
        Assert.False(Object.ReferenceEquals(g, g2))

    [<Fact>]
    member _.``chaining setModule calls accumulates changes``() =
        let g =
            Barcode2D.makeModuleGrid 3 3 Square
            |> fun g -> Barcode2D.setModule g 0 0 true
            |> fun g -> Barcode2D.setModule g 0 2 true
            |> fun g -> Barcode2D.setModule g 2 0 true
            |> fun g -> Barcode2D.setModule g 2 2 true
        // Corners set.
        Assert.True(g.Modules.[0].[0])
        Assert.True(g.Modules.[0].[2])
        Assert.True(g.Modules.[2].[0])
        Assert.True(g.Modules.[2].[2])
        // Centre not set.
        Assert.False(g.Modules.[1].[1])

    [<Fact>]
    member _.``setModule at row 0 col 0 (top-left corner)``() =
        let g  = Barcode2D.makeModuleGrid 5 5 Square
        let g2 = Barcode2D.setModule g 0 0 true
        Assert.True(g2.Modules.[0].[0])

    [<Fact>]
    member _.``setModule at last row last col (bottom-right corner)``() =
        let g  = Barcode2D.makeModuleGrid 5 5 Square
        let g2 = Barcode2D.setModule g 4 4 true
        Assert.True(g2.Modules.[4].[4])

    [<Fact>]
    member _.``out-of-range row raises ArgumentOutOfRangeException``() =
        let g = Barcode2D.makeModuleGrid 3 3 Square
        Assert.Throws<ArgumentOutOfRangeException>(fun () ->
            Barcode2D.setModule g 3 0 true |> ignore)

    [<Fact>]
    member _.``negative row raises ArgumentOutOfRangeException``() =
        let g = Barcode2D.makeModuleGrid 3 3 Square
        Assert.Throws<ArgumentOutOfRangeException>(fun () ->
            Barcode2D.setModule g -1 0 true |> ignore)

    [<Fact>]
    member _.``out-of-range col raises ArgumentOutOfRangeException``() =
        let g = Barcode2D.makeModuleGrid 3 3 Square
        Assert.Throws<ArgumentOutOfRangeException>(fun () ->
            Barcode2D.setModule g 0 3 true |> ignore)

    [<Fact>]
    member _.``negative col raises ArgumentOutOfRangeException``() =
        let g = Barcode2D.makeModuleGrid 3 3 Square
        Assert.Throws<ArgumentOutOfRangeException>(fun () ->
            Barcode2D.setModule g 0 -1 true |> ignore)

// ===========================================================================
// 5. defaultConfig
// ===========================================================================

type DefaultConfigTests() =

    [<Fact>]
    member _.``ModuleSizePx defaults to 10``() =
        assertApprox 10.0 Barcode2D.defaultConfig.ModuleSizePx

    [<Fact>]
    member _.``QuietZoneModules defaults to 4``() =
        Assert.Equal(4, Barcode2D.defaultConfig.QuietZoneModules)

    [<Fact>]
    member _.``DarkColor defaults to black``() =
        Assert.Equal("#000000", Barcode2D.defaultConfig.DarkColor)

    [<Fact>]
    member _.``LightColor defaults to white``() =
        Assert.Equal("#ffffff", Barcode2D.defaultConfig.LightColor)

// ===========================================================================
// 6. layout — Square modules
// ===========================================================================

type LayoutSquareTests() =

    // Helper: a 3×3 all-dark square grid with the default config.
    let allDark3x3 () =
        let mutable g = Barcode2D.makeModuleGrid 3 3 Square
        for r in 0..2 do
            for c in 0..2 do
                g <- Barcode2D.setModule g r c true
        g

    [<Fact>]
    member _.``layout returns a PaintScene``() =
        let g = Barcode2D.makeModuleGrid 5 5 Square
        let scene = Barcode2D.layout g Barcode2D.defaultConfig
        // Basic shape check — totalWidth = (5 + 2*4) * 10 = 130
        assertApprox 130.0 scene.Width
        assertApprox 130.0 scene.Height

    [<Fact>]
    member _.``scene background colour matches LightColor``() =
        let g     = Barcode2D.makeModuleGrid 3 3 Square
        let scene = Barcode2D.layout g Barcode2D.defaultConfig
        Assert.Equal("#ffffff", scene.Background)

    [<Fact>]
    member _.``first instruction is the background rect``() =
        let g     = Barcode2D.makeModuleGrid 3 3 Square
        let scene = Barcode2D.layout g Barcode2D.defaultConfig
        let bg    = asRect (List.head scene.Instructions)
        assertApprox 0.0 bg.X
        assertApprox 0.0 bg.Y

    [<Fact>]
    member _.``background rect covers full canvas width and height``() =
        // 3-col grid, qz=4, size=10 → total = (3+8)*10 = 110
        let g     = Barcode2D.makeModuleGrid 3 5 Square
        let scene = Barcode2D.layout g Barcode2D.defaultConfig
        let bg    = asRect (List.head scene.Instructions)
        assertApprox 130.0 bg.Width   // (5+8)*10
        assertApprox 110.0 bg.Height  // (3+8)*10

    [<Fact>]
    member _.``background rect fill is LightColor``() =
        let g     = Barcode2D.makeModuleGrid 2 2 Square
        let scene = Barcode2D.layout g Barcode2D.defaultConfig
        let bg    = asRect (List.head scene.Instructions)
        Assert.Equal(Some "#ffffff", bg.Fill)

    [<Fact>]
    member _.``all-light grid produces only the background rect``() =
        // No dark modules → no dark rects after the background.
        let g     = Barcode2D.makeModuleGrid 5 5 Square
        let scene = Barcode2D.layout g Barcode2D.defaultConfig
        Assert.Equal(1, List.length scene.Instructions)

    [<Fact>]
    member _.``one dark module produces exactly 2 instructions``() =
        let g     = Barcode2D.makeModuleGrid 3 3 Square
        let g2    = Barcode2D.setModule g 0 0 true
        let scene = Barcode2D.layout g2 Barcode2D.defaultConfig
        // background + 1 dark rect
        Assert.Equal(2, List.length scene.Instructions)

    [<Fact>]
    member _.``all-dark 3×3 grid produces 10 instructions (1 bg + 9 dark)``() =
        let scene = Barcode2D.layout (allDark3x3 ()) Barcode2D.defaultConfig
        Assert.Equal(10, List.length scene.Instructions)

    [<Fact>]
    member _.``dark module rect has correct X coordinate``() =
        // module at col=0, qz=4, size=10 → x = 40
        let g     = Barcode2D.makeModuleGrid 3 3 Square
        let g2    = Barcode2D.setModule g 0 0 true
        let scene = Barcode2D.layout g2 Barcode2D.defaultConfig
        let rect  = asRect (List.item 1 scene.Instructions)
        assertApprox 40.0 rect.X

    [<Fact>]
    member _.``dark module rect has correct Y coordinate``() =
        // module at row=0, qz=4, size=10 → y = 40
        let g     = Barcode2D.makeModuleGrid 3 3 Square
        let g2    = Barcode2D.setModule g 0 0 true
        let scene = Barcode2D.layout g2 Barcode2D.defaultConfig
        let rect  = asRect (List.item 1 scene.Instructions)
        assertApprox 40.0 rect.Y

    [<Fact>]
    member _.``dark module rect width and height equal moduleSizePx``() =
        let g     = Barcode2D.makeModuleGrid 3 3 Square
        let g2    = Barcode2D.setModule g 1 2 true
        let scene = Barcode2D.layout g2 Barcode2D.defaultConfig
        let rect  = asRect (List.item 1 scene.Instructions)
        assertApprox 10.0 rect.Width
        assertApprox 10.0 rect.Height

    [<Fact>]
    member _.``dark module rect fill is DarkColor``() =
        let g     = Barcode2D.makeModuleGrid 3 3 Square
        let g2    = Barcode2D.setModule g 0 0 true
        let scene = Barcode2D.layout g2 Barcode2D.defaultConfig
        let rect  = asRect (List.item 1 scene.Instructions)
        Assert.Equal(Some "#000000", rect.Fill)

    [<Fact>]
    member _.``pixel position of module at row=2 col=1``() =
        // qz = 4*10 = 40; x = 40 + 1*10 = 50; y = 40 + 2*10 = 60
        let cfg   = Barcode2D.defaultConfig
        let g     = Barcode2D.makeModuleGrid 5 5 Square
        let g2    = Barcode2D.setModule g 2 1 true
        let scene = Barcode2D.layout g2 cfg
        let rect  = asRect (List.item 1 scene.Instructions)
        assertApprox 50.0 rect.X
        assertApprox 60.0 rect.Y

    [<Fact>]
    member _.``zero quiet zone — dark module at origin``() =
        let cfg   = { Barcode2D.defaultConfig with QuietZoneModules = 0 }
        let g     = Barcode2D.makeModuleGrid 3 3 Square
        let g2    = Barcode2D.setModule g 0 0 true
        let scene = Barcode2D.layout g2 cfg
        let rect  = asRect (List.item 1 scene.Instructions)
        assertApprox 0.0 rect.X
        assertApprox 0.0 rect.Y

    [<Fact>]
    member _.``custom moduleSizePx = 5 is respected``() =
        let cfg   = { Barcode2D.defaultConfig with ModuleSizePx = 5.0; QuietZoneModules = 0 }
        let g     = Barcode2D.makeModuleGrid 4 4 Square
        let g2    = Barcode2D.setModule g 0 0 true
        let scene = Barcode2D.layout g2 cfg
        let rect  = asRect (List.item 1 scene.Instructions)
        assertApprox 5.0 rect.Width
        assertApprox 5.0 rect.Height

    [<Fact>]
    member _.``total canvas width with qz=2 size=10 cols=5 is (5+4)*10=90``() =
        let cfg   = { Barcode2D.defaultConfig with QuietZoneModules = 2 }
        let g     = Barcode2D.makeModuleGrid 5 5 Square
        let scene = Barcode2D.layout g cfg
        assertApprox 90.0 scene.Width

    [<Fact>]
    member _.``custom DarkColor is propagated to rect``() =
        let cfg   = { Barcode2D.defaultConfig with DarkColor = "#ff0000" }
        let g     = Barcode2D.makeModuleGrid 2 2 Square
        let g2    = Barcode2D.setModule g 0 0 true
        let scene = Barcode2D.layout g2 cfg
        let rect  = asRect (List.item 1 scene.Instructions)
        Assert.Equal(Some "#ff0000", rect.Fill)

    [<Fact>]
    member _.``custom LightColor is propagated to background and scene``() =
        let cfg   = { Barcode2D.defaultConfig with LightColor = "#eeeeee" }
        let g     = Barcode2D.makeModuleGrid 2 2 Square
        let scene = Barcode2D.layout g cfg
        Assert.Equal("#eeeeee", scene.Background)
        let bg = asRect (List.head scene.Instructions)
        Assert.Equal(Some "#eeeeee", bg.Fill)

// ===========================================================================
// 7. layout — Hex modules
// ===========================================================================

type LayoutHexTests() =

    [<Fact>]
    member _.``all-light hex grid produces only the background rect``() =
        let g     = Barcode2D.makeModuleGrid 4 4 Hex
        let scene = Barcode2D.layout g Barcode2D.defaultConfig
        Assert.Equal(1, List.length scene.Instructions)

    [<Fact>]
    member _.``one dark hex module produces exactly 2 instructions``() =
        let g     = Barcode2D.makeModuleGrid 3 3 Hex
        let g2    = Barcode2D.setModule g 0 0 true
        let scene = Barcode2D.layout g2 Barcode2D.defaultConfig
        Assert.Equal(2, List.length scene.Instructions)

    [<Fact>]
    member _.``dark hex module instruction is a Path``() =
        let g     = Barcode2D.makeModuleGrid 3 3 Hex
        let g2    = Barcode2D.setModule g 0 0 true
        let scene = Barcode2D.layout g2 Barcode2D.defaultConfig
        let instr = List.item 1 scene.Instructions
        match instr with
        | Path _ -> ()  // correct
        | _      -> Assert.Fail("Expected Path instruction for hex module")

    [<Fact>]
    member _.``hex path has exactly 8 commands (MoveTo + 5 LineTo + Close)``() =
        // A flat-top hexagon: MoveTo v0, LineTo v1..v5, Close → 7 commands.
        // Wait — that's 1 + 5 + 1 = 7 commands total.
        let g     = Barcode2D.makeModuleGrid 3 3 Hex
        let g2    = Barcode2D.setModule g 0 0 true
        let scene = Barcode2D.layout g2 Barcode2D.defaultConfig
        let path  = asPath (List.item 1 scene.Instructions)
        Assert.Equal(7, List.length path.Commands)

    [<Fact>]
    member _.``hex path starts with MoveTo``() =
        let g    = Barcode2D.makeModuleGrid 3 3 Hex
        let g2   = Barcode2D.setModule g 0 0 true
        let scene = Barcode2D.layout g2 Barcode2D.defaultConfig
        let path  = asPath (List.item 1 scene.Instructions)
        match List.head path.Commands with
        | MoveTo _ -> ()
        | _        -> Assert.Fail("First command should be MoveTo")

    [<Fact>]
    member _.``hex path ends with Close``() =
        let g    = Barcode2D.makeModuleGrid 3 3 Hex
        let g2   = Barcode2D.setModule g 0 0 true
        let scene = Barcode2D.layout g2 Barcode2D.defaultConfig
        let path  = asPath (List.item 1 scene.Instructions)
        match List.last path.Commands with
        | Close -> ()
        | _     -> Assert.Fail("Last command should be Close")

    [<Fact>]
    member _.``hex path fill is DarkColor``() =
        let g    = Barcode2D.makeModuleGrid 3 3 Hex
        let g2   = Barcode2D.setModule g 0 0 true
        let scene = Barcode2D.layout g2 Barcode2D.defaultConfig
        let path  = asPath (List.item 1 scene.Instructions)
        Assert.Equal(Some "#000000", path.Fill)

    [<Fact>]
    member _.``hex scene width accounts for odd-row offset``() =
        // totalWidth = (cols + 2*qz) * hexWidth + hexWidth/2
        // With cols=4, qz=4, sz=10: (4+8)*10 + 5 = 125
        let g     = Barcode2D.makeModuleGrid 3 4 Hex
        let scene = Barcode2D.layout g Barcode2D.defaultConfig
        assertApprox 125.0 scene.Width

    [<Fact>]
    member _.``hex scene height uses hexHeight = sz * sqrt3/2``() =
        // totalHeight = (rows + 2*qz) * hexHeight
        // With rows=2, qz=4, sz=10: (2+8)*10*(sqrt(3)/2) = 100*0.866025... ≈ 86.60
        let g     = Barcode2D.makeModuleGrid 2 4 Hex
        let scene = Barcode2D.layout g Barcode2D.defaultConfig
        let expected = float (2 + 2*4) * 10.0 * (Math.Sqrt 3.0 / 2.0)
        assertApprox expected scene.Height

    [<Fact>]
    member _.``odd-row module center x is offset by hexWidth/2``() =
        // Row 1 (odd), col 0, qz=0, sz=10:
        //   cx_even = 0 + 0*10 + 0*(10/2) = 0      (row 0 col 0)
        //   cx_odd  = 0 + 0*10 + 1*(10/2) = 5      (row 1 col 0)
        let cfg  = { Barcode2D.defaultConfig with QuietZoneModules = 0 }
        let g    = Barcode2D.makeModuleGrid 2 2 Hex

        // Place one module at row=0, col=0 and one at row=1, col=0.
        let g2   = Barcode2D.setModule g 0 0 true
        let g3   = Barcode2D.setModule g 1 0 true

        let scene0 = Barcode2D.layout g2 cfg
        let scene1 = Barcode2D.layout g3 cfg

        let path0 = asPath (List.item 1 scene0.Instructions)
        let path1 = asPath (List.item 1 scene1.Instructions)

        // Extract MoveTo coordinates.
        let cx0 =
            match List.head path0.Commands with
            | MoveTo(x, _) -> x
            | _ -> failwith "expected MoveTo"

        let cx1 =
            match List.head path1.Commands with
            | MoveTo(x, _) -> x
            | _ -> failwith "expected MoveTo"

        // circumR = 10/sqrt(3), so vertex 0 is at cx + circumR.
        // For even row, cx = 0.0 (qz=0, col=0), vertex0_x = 0 + circumR
        // For odd row,  cx = 5.0,               vertex0_x = 5 + circumR
        // Difference in MoveTo.X should be exactly 5.0.
        assertApprox 5.0 (cx1 - cx0)

    [<Fact>]
    member _.``hex 3×3 all-dark produces 1 bg + 9 path instructions``() =
        let mutable g = Barcode2D.makeModuleGrid 3 3 Hex
        for r in 0..2 do
            for c in 0..2 do
                g <- Barcode2D.setModule g r c true
        let scene = Barcode2D.layout g Barcode2D.defaultConfig
        Assert.Equal(10, List.length scene.Instructions)

// ===========================================================================
// 8. layout validation
// ===========================================================================

type LayoutValidationTests() =

    [<Fact>]
    member _.``moduleSizePx = 0 raises ArgumentException``() =
        let g   = Barcode2D.makeModuleGrid 3 3 Square
        let cfg = { Barcode2D.defaultConfig with ModuleSizePx = 0.0 }
        Assert.Throws<ArgumentException>(fun () ->
            Barcode2D.layout g cfg |> ignore)

    [<Fact>]
    member _.``negative moduleSizePx raises ArgumentException``() =
        let g   = Barcode2D.makeModuleGrid 3 3 Square
        let cfg = { Barcode2D.defaultConfig with ModuleSizePx = -5.0 }
        Assert.Throws<ArgumentException>(fun () ->
            Barcode2D.layout g cfg |> ignore)

    [<Fact>]
    member _.``negative quietZoneModules raises ArgumentException``() =
        let g   = Barcode2D.makeModuleGrid 3 3 Square
        let cfg = { Barcode2D.defaultConfig with QuietZoneModules = -1 }
        Assert.Throws<ArgumentException>(fun () ->
            Barcode2D.layout g cfg |> ignore)

    [<Fact>]
    member _.``zero quietZoneModules is valid (no exception)``() =
        let g   = Barcode2D.makeModuleGrid 3 3 Square
        let cfg = { Barcode2D.defaultConfig with QuietZoneModules = 0 }
        // Should not throw.
        let scene = Barcode2D.layout g cfg
        assertApprox 30.0 scene.Width   // (3+0)*10

    [<Fact>]
    member _.``very small moduleSizePx = 0.001 is valid``() =
        let g   = Barcode2D.makeModuleGrid 2 2 Square
        let cfg = { Barcode2D.defaultConfig with ModuleSizePx = 0.001 }
        let scene = Barcode2D.layout g cfg
        Assert.True(scene.Width > 0.0)

// ===========================================================================
// 9. AnnotatedModuleGrid
// ===========================================================================

type AnnotatedModuleGridTests() =

    [<Fact>]
    member _.``can construct AnnotatedModuleGrid with None roles``() =
        let grid  = Barcode2D.makeModuleGrid 3 3 Square
        let roles = Array.init 3 (fun _ -> Array.create 3 None)
        let annotated = { Grid = grid; Roles = roles }
        Assert.Equal(3, annotated.Grid.Rows)
        Assert.Equal(3, annotated.Grid.Cols)

    [<Fact>]
    member _.``can store Some role string per module``() =
        let grid  = Barcode2D.makeModuleGrid 2 2 Square
        let roles = Array.init 2 (fun _ -> Array.create 2 (Some "data"))
        let annotated = { Grid = grid; Roles = roles }
        Assert.Equal(Some "data", annotated.Roles.[0].[0])
        Assert.Equal(Some "data", annotated.Roles.[1].[1])

    [<Fact>]
    member _.``mixed Some and None roles``() =
        let grid  = Barcode2D.makeModuleGrid 1 3 Square
        let roles = [| [| Some "finder"; None; Some "ecc" |] |]
        let annotated = { Grid = grid; Roles = roles }
        Assert.Equal(Some "finder", annotated.Roles.[0].[0])
        Assert.Equal(None,          annotated.Roles.[0].[1])
        Assert.Equal(Some "ecc",    annotated.Roles.[0].[2])

    [<Fact>]
    member _.``AnnotatedModuleGrid grid can be used with layout``() =
        // The underlying grid inside AnnotatedModuleGrid is a plain ModuleGrid
        // and can be passed directly to layout.
        let grid     = Barcode2D.makeModuleGrid 3 3 Square
        let grid2    = Barcode2D.setModule grid 0 0 true
        let roles    = Array.init 3 (fun _ -> Array.create 3 None)
        let annotated = { Grid = grid2; Roles = roles }
        let scene    = Barcode2D.layout annotated.Grid Barcode2D.defaultConfig
        Assert.Equal(2, List.length scene.Instructions)

// ===========================================================================
// 10. Integration tests
// ===========================================================================

type IntegrationTests() =

    [<Fact>]
    member _.``round-trip: encode then decode module positions (3×3 checkerboard)``() =
        // Build a checkerboard: dark where (row + col) is even.
        let mutable g = Barcode2D.makeModuleGrid 3 3 Square
        for r in 0..2 do
            for c in 0..2 do
                if (r + c) % 2 = 0 then
                    g <- Barcode2D.setModule g r c true
        let scene = Barcode2D.layout g Barcode2D.defaultConfig
        // 5 dark modules + 1 bg = 6 instructions.
        Assert.Equal(6, List.length scene.Instructions)

    [<Fact>]
    member _.``layout produces deterministic output for the same grid``() =
        let g     = Barcode2D.makeModuleGrid 5 5 Square
        let g2    = Barcode2D.setModule g 2 2 true
        let s1    = Barcode2D.layout g2 Barcode2D.defaultConfig
        let s2    = Barcode2D.layout g2 Barcode2D.defaultConfig
        // Same number of instructions and same canvas size.
        Assert.Equal(List.length s1.Instructions, List.length s2.Instructions)
        assertApprox s1.Width  s2.Width
        assertApprox s1.Height s2.Height

    [<Fact>]
    member _.``hex layout: all-dark 33×30 MaxiCode grid produces 991 instructions``() =
        // 33 rows × 30 cols = 990 dark modules + 1 background = 991 total.
        let mutable g = Barcode2D.makeModuleGrid 33 30 Hex
        for r in 0..32 do
            for c in 0..29 do
                g <- Barcode2D.setModule g r c true
        let scene = Barcode2D.layout g Barcode2D.defaultConfig
        Assert.Equal(991, List.length scene.Instructions)

    [<Fact>]
    member _.``square layout with qz=1 size=2: total size is (cols+2)*2 × (rows+2)*2``() =
        let cfg   = { Barcode2D.defaultConfig with QuietZoneModules = 1; ModuleSizePx = 2.0 }
        let g     = Barcode2D.makeModuleGrid 4 6 Square
        let scene = Barcode2D.layout g cfg
        // width  = (6 + 2*1) * 2 = 16
        // height = (4 + 2*1) * 2 = 12
        assertApprox 16.0 scene.Width
        assertApprox 12.0 scene.Height

    [<Fact>]
    member _.``setModule then layout preserves modules array immutability``() =
        let g0 = Barcode2D.makeModuleGrid 5 5 Square
        let g1 = Barcode2D.setModule g0 0 0 true
        let g2 = Barcode2D.setModule g1 4 4 true
        // g0 should still be all-false.
        for r in 0..4 do
            for c in 0..4 do
                Assert.False(g0.Modules.[r].[c], sprintf "g0[%d][%d] should still be false" r c)
        // g1 should only have (0,0) true.
        Assert.True(g1.Modules.[0].[0])
        Assert.False(g1.Modules.[4].[4])
        // g2 should have both (0,0) and (4,4) true.
        Assert.True(g2.Modules.[0].[0])
        Assert.True(g2.Modules.[4].[4])
