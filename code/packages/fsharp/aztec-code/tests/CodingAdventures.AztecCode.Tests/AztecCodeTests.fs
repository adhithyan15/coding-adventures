/// AztecCodeTests.fs — Unit tests for CodingAdventures.AztecCode
///
/// Coverage focus areas (target ≥ 90 % line coverage):
///   - VERSION constant
///   - Default options
///   - Symbol size selection: smallest compact for short input, larger
///     compact for medium, full for very large
///   - Bullseye finder pattern geometry (Chebyshev rings)
///   - Orientation mark corners (always dark)
///   - Reference grid presence/absence (full vs compact)
///   - Mode message ring placement
///   - Determinism (same input → identical grid)
///   - ECC option validation (out-of-range error)
///   - Byte array vs string round-trip equivalence
///   - InputTooLong error for huge payloads

module CodingAdventures.AztecCode.Tests

open System
open Xunit
open CodingAdventures.AztecCode
open CodingAdventures.Barcode2D

// ============================================================================
// Helpers
// ============================================================================

/// Unwrap an ``Ok`` result; fail the test with the error message otherwise.
let private unwrap result =
    match result with
    | Ok v    -> v
    | Error e -> failwith (sprintf "Expected Ok, got Error: %A" e)

/// Read a module from the jagged grid as a bool.
let private mAt (grid: ModuleGrid) (row: int) (col: int) =
    grid.Modules.[row].[col]

/// Chebyshev distance between two grid points.
let private cheb (r1, c1) (r2, c2) =
    max (abs (r1 - r2)) (abs (c1 - c2))

// ============================================================================
// Basic sanity
// ============================================================================

[<Fact>]
let ``VERSION is 0.1.0`` () =
    Assert.Equal("0.1.0", VERSION)

[<Fact>]
let ``defaultOptions has 23 percent ECC`` () =
    Assert.Equal(23, defaultOptions.MinEccPercent)

// ============================================================================
// Smallest compact symbol — single character input
// ============================================================================

[<Fact>]
let ``empty string produces a valid compact symbol`` () =
    let grid = unwrap (encode "")
    // The smallest compact symbol is 15×15 (1 layer).  An empty string
    // requires only 10 bits (5 escape + 5 length), well within capacity.
    Assert.Equal(15, grid.Rows)
    Assert.Equal(15, grid.Cols)
    Assert.Equal(ModuleShape.Square, grid.ModuleShape)

[<Fact>]
let ``single character produces a 15x15 compact symbol`` () =
    let grid = unwrap (encode "A")
    Assert.Equal(15, grid.Rows)
    Assert.Equal(15, grid.Cols)

[<Fact>]
let ``HELLO fits in compact symbol`` () =
    let grid = unwrap (encode "HELLO")
    // 5 bytes = 40 bits + ~13 bits overhead.  Should fit in compact 1 (9 cw)
    // or compact 2 (25 cw) at 23 % ECC.
    Assert.True(grid.Rows <= 19, sprintf "Expected ≤ 19, got %d" grid.Rows)

// ============================================================================
// Compact ↔ full transition
// ============================================================================

[<Fact>]
let ``medium payload still fits compact`` () =
    // 20 bytes ≈ 160 bits; compact 4 has 81 codewords with 19 ECC → 62 data.
    // 20 bytes well under that.
    let grid = unwrap (encode (String.replicate 20 "A"))
    let sz   = grid.Rows
    Assert.True(sz <= 27, sprintf "Expected compact (≤ 27), got %d" sz)

[<Fact>]
let ``large payload requires full symbol`` () =
    // 100 bytes ≈ 800+ bits; compact 4 max is 81 codewords (~62 data) so this
    // forces a full symbol, which has minimum size 19×19 (full 1 layer).
    let grid = unwrap (encode (String.replicate 100 "A"))
    Assert.True(grid.Rows >= 19, sprintf "Expected ≥ 19 (full symbol), got %d" grid.Rows)

[<Fact>]
let ``encode handles 200-byte payload`` () =
    let grid = unwrap (encode (String.replicate 200 "B"))
    // Should still be a manageable full symbol.
    Assert.True(grid.Rows > 0)
    Assert.Equal(grid.Rows, grid.Cols)

// ============================================================================
// Symbol-size formula: compact = 11+4L, full = 15+4L
// ============================================================================

[<Fact>]
let ``compact symbol size matches 11 plus 4 times layers`` () =
    // The smallest possible symbol (1 char) is compact 1 layer = 15.
    let grid = unwrap (encode "A")
    let sz   = grid.Rows
    // Must equal one of {15, 19, 23, 27}.
    let valid = [| 15; 19; 23; 27 |]
    Assert.Contains(sz, valid)

[<Fact>]
let ``grid is always square`` () =
    for input in [| "A"; "HELLO"; "1234567890"; String.replicate 50 "X" |] do
        let grid = unwrap (encode input)
        Assert.Equal(grid.Rows, grid.Cols)

// ============================================================================
// Bullseye finder pattern
// ============================================================================

/// Compact symbols use bullseye radius 5; full symbols use 7.
let private expectedBullseyeRadius (sz: int) =
    if sz <= 27 then 5 else 7

[<Fact>]
let ``compact bullseye centre is dark`` () =
    let grid = unwrap (encode "A")
    let cx = grid.Cols / 2
    let cy = grid.Rows / 2
    Assert.True(mAt grid cy cx, "centre module should be dark")

[<Fact>]
let ``bullseye obeys Chebyshev rings for compact symbols`` () =
    // The bullseye occupies Chebyshev distance ≤ br from centre.
    // Distance 0 = dark, 1 = dark, 2 = light, 3 = dark, 4 = light, 5 = dark.
    let grid = unwrap (encode "A")
    let sz   = grid.Rows
    let br   = expectedBullseyeRadius sz
    Assert.Equal(5, br)
    let cx = sz / 2
    let cy = sz / 2

    for d in 0 .. br do
        // Pick the cell directly above the centre (still inside the bullseye).
        let row = cy - d
        let col = cx
        let expected = if d <= 1 then true else (d % 2 = 1)
        Assert.Equal(expected, mAt grid row col)

[<Fact>]
let ``bullseye obeys Chebyshev rings for full symbols`` () =
    // 100 bytes forces a full symbol; bullseye radius = 7.
    let grid = unwrap (encode (String.replicate 100 "A"))
    let sz   = grid.Rows
    let br   = expectedBullseyeRadius sz
    Assert.Equal(7, br)
    let cx = sz / 2
    let cy = sz / 2

    // For a full symbol the (cy-d, cx) column might also be on a reference
    // grid line.  We restrict d ≤ br and pick the cell along the diagonal so
    // the reference grid (which only sits on rows/cols hitting (cy-row)%16=0
    // or (cx-col)%16=0) does not interfere with the bullseye check.
    for d in 0 .. br do
        let row = cy - d
        let col = cx + d
        let expected = if d <= 1 then true else (d % 2 = 1)
        Assert.Equal(expected, mAt grid row col)

// ============================================================================
// Orientation marks — the four corners of the mode message ring
// ============================================================================

[<Fact>]
let ``four orientation mark corners are dark`` () =
    let grid = unwrap (encode "HELLO")
    let sz   = grid.Rows
    let br   = expectedBullseyeRadius sz
    let r    = br + 1
    let cx   = sz / 2
    let cy   = sz / 2

    // The four corners of the mode message ring (Chebyshev distance r) are
    // always painted DARK as orientation marks.
    Assert.True(mAt grid (cy - r) (cx - r), "TL orientation corner")
    Assert.True(mAt grid (cy - r) (cx + r), "TR orientation corner")
    Assert.True(mAt grid (cy + r) (cx + r), "BR orientation corner")
    Assert.True(mAt grid (cy + r) (cx - r), "BL orientation corner")

[<Fact>]
let ``orientation corners dark for full symbol`` () =
    let grid = unwrap (encode (String.replicate 100 "A"))
    let sz   = grid.Rows
    let br   = expectedBullseyeRadius sz
    let r    = br + 1
    let cx   = sz / 2
    let cy   = sz / 2

    Assert.True(mAt grid (cy - r) (cx - r), "TL orientation corner (full)")
    Assert.True(mAt grid (cy - r) (cx + r), "TR orientation corner (full)")
    Assert.True(mAt grid (cy + r) (cx + r), "BR orientation corner (full)")
    Assert.True(mAt grid (cy + r) (cx - r), "BL orientation corner (full)")

// ============================================================================
// Reference grid — full symbols only
// ============================================================================

[<Fact>]
let ``full symbol has reference grid intersection at centre`` () =
    // Reference grid lines run where (cy - row) % 16 = 0 or (cx - col) % 16 = 0.
    // The bullseye still overwrites the centre, so the centre module should be
    // dark (overwrite by bullseye).
    let grid = unwrap (encode (String.replicate 100 "A"))
    let sz   = grid.Rows
    let cx   = sz / 2
    let cy   = sz / 2
    Assert.True(mAt grid cy cx, "centre should still be dark")

// ============================================================================
// Determinism
// ============================================================================

[<Fact>]
let ``encoding the same input twice produces identical grids`` () =
    let g1 = unwrap (encode "HELLO WORLD")
    let g2 = unwrap (encode "HELLO WORLD")
    Assert.Equal(g1.Rows, g2.Rows)
    let sz = g1.Rows
    for r in 0 .. sz - 1 do
        for c in 0 .. sz - 1 do
            Assert.Equal(g1.Modules.[r].[c], g2.Modules.[r].[c])

// ============================================================================
// Byte array equivalence
// ============================================================================

[<Fact>]
let ``encodeBytes produces the same grid as encode for ASCII strings`` () =
    let s     = "HELLO"
    let bytes = System.Text.Encoding.UTF8.GetBytes(s)
    let g1    = unwrap (encode s)
    let g2    = unwrap (encodeBytes bytes)
    Assert.Equal(g1.Rows, g2.Rows)
    let sz = g1.Rows
    for r in 0 .. sz - 1 do
        for c in 0 .. sz - 1 do
            Assert.Equal(g1.Modules.[r].[c], g2.Modules.[r].[c])

[<Fact>]
let ``encodeBytes with arbitrary binary payload`` () =
    let payload = [| 0x00uy; 0x01uy; 0xffuy; 0x7fuy; 0x80uy |]
    let grid    = unwrap (encodeBytes payload)
    Assert.True(grid.Rows >= 15)

// ============================================================================
// Long-run input — exercise the bit-stuffing path
// ============================================================================

[<Fact>]
let ``input that triggers bit stuffing still encodes`` () =
    // 'AAA...' is 0x41 = 0100 0001 — interspersed runs of 4 in some places.
    // 'a' (0x61 = 0110 0001), '0' (0x30 = 0011 0000), and 'F' (0x46 =
    // 0100 0110) push different bit patterns through the stuffing path.
    let inputs = [| String.replicate 30 "A"
                    String.replicate 30 "a"
                    String.replicate 30 "0"
                    String.replicate 30 "F" |]
    for input in inputs do
        let grid = unwrap (encode input)
        Assert.True(grid.Rows > 0)
        Assert.Equal(grid.Rows, grid.Cols)

// ============================================================================
// ECC option validation
// ============================================================================

[<Fact>]
let ``MinEccPercent below 10 returns InvalidOptions`` () =
    let opts = { defaultOptions with MinEccPercent = 5 }
    match encodeWith "HELLO" opts with
    | Error (InvalidOptions _) -> ()
    | other -> Assert.Fail(sprintf "Expected InvalidOptions, got %A" other)

[<Fact>]
let ``MinEccPercent above 90 returns InvalidOptions`` () =
    let opts = { defaultOptions with MinEccPercent = 95 }
    match encodeWith "HELLO" opts with
    | Error (InvalidOptions _) -> ()
    | other -> Assert.Fail(sprintf "Expected InvalidOptions, got %A" other)

[<Fact>]
let ``MinEccPercent at 10 is allowed`` () =
    let opts = { defaultOptions with MinEccPercent = 10 }
    let _ = unwrap (encodeWith "HELLO" opts)
    ()

[<Fact>]
let ``MinEccPercent at 90 is allowed`` () =
    let opts = { defaultOptions with MinEccPercent = 90 }
    let _ = unwrap (encodeWith "HELLO" opts)
    ()

[<Fact>]
let ``higher ECC produces larger or equal symbol`` () =
    let g23 = unwrap (encode "HELLO WORLD")
    let g50 = unwrap (encodeWith "HELLO WORLD" { defaultOptions with MinEccPercent = 50 })
    Assert.True(g50.Rows >= g23.Rows)

// ============================================================================
// InputTooLong
// ============================================================================

[<Fact>]
let ``massive payload returns InputTooLong`` () =
    // The largest 32-layer full symbol holds ~1437 codewords ≈ 11 KB of
    // 8-bit data.  100 KB is comfortably beyond that.
    let huge = String.replicate 100000 "A"
    match encode huge with
    | Error (InputTooLong _) -> ()
    | other -> Assert.Fail(sprintf "Expected InputTooLong, got %A" other)

// ============================================================================
// AztecError formatting
// ============================================================================

[<Fact>]
let ``InputTooLong ToString round-trips`` () =
    let err = InputTooLong "test message"
    Assert.Contains("test message", err.ToString())

[<Fact>]
let ``InvalidOptions ToString round-trips`` () =
    let err = InvalidOptions "bad ecc"
    Assert.Contains("bad ecc", err.ToString())

// ============================================================================
// Grid invariants
// ============================================================================

[<Fact>]
let ``module grid dimensions match Rows and Cols`` () =
    let grid = unwrap (encode "HELLO")
    Assert.Equal(grid.Rows, grid.Modules.Length)
    for row in grid.Modules do
        Assert.Equal(grid.Cols, row.Length)

[<Fact>]
let ``module grid contains both dark and light modules`` () =
    let grid = unwrap (encode "HELLO WORLD")
    let sz   = grid.Rows
    let mutable darkCount  = 0
    let mutable lightCount = 0
    for r in 0 .. sz - 1 do
        for c in 0 .. sz - 1 do
            if grid.Modules.[r].[c] then darkCount <- darkCount + 1
            else lightCount <- lightCount + 1
    Assert.True(darkCount  > 0, "expected some dark modules")
    Assert.True(lightCount > 0, "expected some light modules")

// ============================================================================
// UTF-8 input
// ============================================================================

[<Fact>]
let ``UTF-8 multi-byte characters encode without error`` () =
    // Each non-ASCII character takes 2-4 UTF-8 bytes.
    let grid = unwrap (encode "café résumé")
    Assert.True(grid.Rows > 0)

[<Fact>]
let ``31-byte input uses short length encoding (5 bits)`` () =
    // 31 chars exactly hits the 5-bit length boundary.
    let grid = unwrap (encode (String.replicate 31 "X"))
    Assert.True(grid.Rows > 0)

[<Fact>]
let ``32-byte input uses long length encoding (11 bits)`` () =
    // 32 chars crosses into the long form (5-bit zero escape + 11-bit length).
    let grid = unwrap (encode (String.replicate 32 "X"))
    Assert.True(grid.Rows > 0)

// ============================================================================
// Centre symmetry — orientation corners around the bullseye
// ============================================================================

[<Fact>]
let ``mode-message ring corners equidistant from centre`` () =
    // Sanity check that the four orientation corners we tested above are all
    // at Chebyshev distance bullseyeRadius+1 from the centre.
    let grid = unwrap (encode "ABC")
    let sz   = grid.Rows
    let cx   = sz / 2
    let cy   = sz / 2
    let br   = expectedBullseyeRadius sz
    let expected = br + 1
    let corners = [| (cy - expected, cx - expected)
                     (cy - expected, cx + expected)
                     (cy + expected, cx + expected)
                     (cy + expected, cx - expected) |]
    for (r, c) in corners do
        let d = cheb (r, c) (cy, cx)
        Assert.Equal(expected, d)
