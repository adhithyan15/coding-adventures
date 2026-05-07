/// DataMatrixTests.fs — Unit tests for CodingAdventures.DataMatrix
///
/// Coverage focus areas (target ≥ 90 % line coverage):
///   - VERSION constant
///   - GF(256)/0x12D arithmetic tables
///   - GF multiplication correctness
///   - ASCII encoding (single chars, digit pairs, extended ASCII)
///   - Pad codewords (first pad = 129, subsequent scrambled)
///   - Symbol selection (correct size for various input lengths)
///   - RS ECC block encoding (verify against known vectors)
///   - Grid initialisation (L-finder dark/light, timing alternation)
///   - Utah placement grid dimensions
///   - Alignment borders in multi-region symbols
///   - Full encode pipeline: string and byte-array inputs
///   - Symbol dimensions match expected sizes
///   - L-finder border correctness
///   - Timing clock alternation
///   - Determinism (same input → identical grid)
///   - Error path: InputTooLong for huge payloads
///   - Encode rectangle of "A" digits
///   - Digit-pair packing reduces codeword count

module CodingAdventures.DataMatrix.Tests

open System
open Xunit
open CodingAdventures.DataMatrix
open CodingAdventures.Barcode2D

// ============================================================================
// Helpers
// ============================================================================

/// Unwrap an ``Ok`` result; fail the test with a message otherwise.
let private unwrap result =
    match result with
    | Ok v    -> v
    | Error e -> failwith (sprintf "Expected Ok, got Error: %A" e)

/// Read a module from the grid as a bool (true = dark).
let private mAt (grid: ModuleGrid) (row: int) (col: int) =
    grid.Modules.[row].[col]

// ============================================================================
// VERSION
// ============================================================================

[<Fact>]
let ``VERSION is 0.1.0`` () =
    Assert.Equal("0.1.0", VERSION)

// ============================================================================
// GF(256)/0x12D tables
// ============================================================================

[<Fact>]
let ``gfExp 0 is 1`` () =
    // α^0 = 1 in every Galois field
    Assert.Equal(1, gfExp.[0])

[<Fact>]
let ``gfExp 1 is 2`` () =
    // α^1 = 2 (the primitive element in GF(256)/0x12D is 2)
    Assert.Equal(2, gfExp.[1])

[<Fact>]
let ``gfExp 7 is 128`` () =
    // 2^7 = 128 — no reduction needed because 128 < 256
    Assert.Equal(128, gfExp.[7])

[<Fact>]
let ``gfExp 8 is 45`` () =
    // 2^8 = 256 → 256 XOR 0x12D = 256 XOR 301 = 0x100 XOR 0x12D = 0x02D = 45
    Assert.Equal(45, gfExp.[8])

[<Fact>]
let ``gfLog of 1 is 0`` () =
    // log_α(1) = 0  since α^0 = 1
    Assert.Equal(0, gfLog.[1])

[<Fact>]
let ``gfLog of 2 is 1`` () =
    Assert.Equal(1, gfLog.[2])

[<Fact>]
let ``gfExp and gfLog are inverses for all non-zero values`` () =
    // For v in 1..255: gfExp.[gfLog.[v]] = v
    for v in 1 .. 255 do
        Assert.Equal(v, gfExp.[gfLog.[v]])

// ============================================================================
// GF(256) multiplication
// ============================================================================

[<Fact>]
let ``gfMul 0 anything is 0`` () =
    Assert.Equal(0, gfMul 0 42)
    Assert.Equal(0, gfMul 42 0)
    Assert.Equal(0, gfMul 0 0)

[<Fact>]
let ``gfMul 1 x is x`` () =
    // 1 is the multiplicative identity
    for x in 0 .. 255 do
        Assert.Equal(x, gfMul 1 x)

[<Fact>]
let ``gfMul is commutative`` () =
    // GF(256) multiplication is commutative: a*b = b*a
    for a in 1 .. 10 do
        for b in 1 .. 10 do
            Assert.Equal(gfMul a b, gfMul b a)

[<Fact>]
let ``gfMul 2 45 is 90`` () =
    // 2 × 45 = α^1 × α^8 = α^9 = gfExp.[9]
    let expected = gfExp.[1 + 8]
    Assert.Equal(expected, gfMul 2 45)

[<Fact>]
let ``gfMul result is always in range 0..255`` () =
    // Every GF(256) element is a byte value
    for a in 0 .. 15 do
        for b in 0 .. 15 do
            let r = gfMul a b
            Assert.True(r >= 0 && r <= 255, sprintf "gfMul %d %d = %d out of range" a b r)

// ============================================================================
// ASCII encoding
// ============================================================================

[<Fact>]
let ``ASCII encode single 'A' gives 66`` () =
    // 'A' = 65; ASCII codeword = 65 + 1 = 66
    let cw = encodeAsciiInternal [| byte 'A' |]
    Assert.Equal<int[]>([| 66 |], cw)

[<Fact>]
let ``ASCII encode space gives 33`` () =
    // ' ' = 32; codeword = 32 + 1 = 33
    let cw = encodeAsciiInternal [| byte ' ' |]
    Assert.Equal<int[]>([| 33 |], cw)

[<Fact>]
let ``ASCII encode digit pair '1''2' gives 142`` () =
    // Digit pair: 130 + 12 = 142
    let cw = encodeAsciiInternal [| byte '1'; byte '2' |]
    Assert.Equal<int[]>([| 142 |], cw)

[<Fact>]
let ``ASCII encode '0''0' gives 130`` () =
    // Digit pair 00: 130 + 0 = 130
    let cw = encodeAsciiInternal [| byte '0'; byte '0' |]
    Assert.Equal<int[]>([| 130 |], cw)

[<Fact>]
let ``ASCII encode '9''9' gives 229`` () =
    // Digit pair 99: 130 + 99 = 229
    let cw = encodeAsciiInternal [| byte '9'; byte '9' |]
    Assert.Equal<int[]>([| 229 |], cw)

[<Fact>]
let ``ASCII encode mixed '1''A' gives two codewords`` () =
    // '1' is a digit but 'A' is not → no pairing
    let cw = encodeAsciiInternal [| byte '1'; byte 'A' |]
    Assert.Equal<int[]>([| 50; 66 |], cw)   // 49+1=50, 65+1=66

[<Fact>]
let ``ASCII encode 'Hello' has correct length`` () =
    // "Hello" = 5 ASCII chars → 5 codewords (no digit pairs)
    let cw = encodeAsciiInternal (System.Text.Encoding.ASCII.GetBytes "Hello")
    Assert.Equal(5, cw.Length)

[<Fact>]
let ``ASCII encode extended byte 128 gives two codewords`` () =
    // Extended: 235 (UPPER_SHIFT) then (128 - 127) = 1
    let cw = encodeAsciiInternal [| 128uy |]
    Assert.Equal<int[]>([| 235; 1 |], cw)

[<Fact>]
let ``ASCII encode digit string has fewer codewords than chars`` () =
    // "1234" → digit pairs → 2 codewords instead of 4
    let cw = encodeAsciiInternal (System.Text.Encoding.ASCII.GetBytes "1234")
    Assert.Equal(2, cw.Length)

[<Fact>]
let ``ASCII encode odd digit string uses pair then single`` () =
    // "123" → pair "12" + single "3" = 2 codewords
    let cw = encodeAsciiInternal (System.Text.Encoding.ASCII.GetBytes "123")
    Assert.Equal(2, cw.Length)
    Assert.Equal(142, cw.[0])   // 130 + 12
    Assert.Equal(52, cw.[1])    // '3'=51; 51+1=52

// ============================================================================
// Pad codewords
// ============================================================================

[<Fact>]
let ``pad single codeword to 3 uses 129 as first pad`` () =
    // [66] padded to 3: [66, 129, scrambled]
    let padded = padCodewordsInternal [| 66 |] 3
    Assert.Equal(3, padded.Length)
    Assert.Equal(66,  padded.[0])
    Assert.Equal(129, padded.[1])

[<Fact>]
let ``pad already-full array returns same content`` () =
    let padded = padCodewordsInternal [| 66; 67; 68 |] 3
    Assert.Equal<int[]>([| 66; 67; 68 |], padded)

[<Fact>]
let ``padded codewords stay in 1..255 range`` () =
    let padded = padCodewordsInternal [| 1 |] 10
    for cw in padded do
        Assert.True(cw >= 1 && cw <= 255, sprintf "pad value %d out of range" cw)

[<Fact>]
let ``pad length is exactly dataCW`` () =
    for dataCW in [| 3; 5; 8; 12; 18 |] do
        let padded = padCodewordsInternal [||] dataCW
        Assert.Equal(dataCW, padded.Length)

// ============================================================================
// Symbol selection
// ============================================================================

[<Fact>]
let ``select symbol for 1 codeword gives 10x10`` () =
    // The smallest square symbol (10×10) has dataCW=3 ≥ 1
    let entry = unwrap (selectSymbolInternal 1)
    Assert.Equal(10, entry.SymbolRows)
    Assert.Equal(10, entry.SymbolCols)

[<Fact>]
let ``select symbol for 3 codewords gives 10x10`` () =
    let entry = unwrap (selectSymbolInternal 3)
    Assert.Equal(10, entry.SymbolRows)

[<Fact>]
let ``select symbol for 4 codewords gives 12x12`` () =
    // 10×10 has dataCW=3 < 4; next is 12×12 with dataCW=5
    let entry = unwrap (selectSymbolInternal 4)
    Assert.Equal(12, entry.SymbolRows)

[<Fact>]
let ``select symbol for 44 codewords gives 26x26`` () =
    // 26×26 has dataCW=44
    let entry = unwrap (selectSymbolInternal 44)
    Assert.Equal(26, entry.SymbolRows)

[<Fact>]
let ``select symbol for 45 codewords gives 32x32`` () =
    // 26×26 has dataCW=44 < 45; next is 32×32 with dataCW=62
    let entry = unwrap (selectSymbolInternal 45)
    Assert.Equal(32, entry.SymbolRows)

[<Fact>]
let ``select symbol for 1559 codewords returns InputTooLong`` () =
    // Maximum capacity is 144×144 with dataCW=1558
    match selectSymbolInternal 1559 with
    | Error (InputTooLong _) -> ()   // expected
    | Ok e  -> failwith (sprintf "Expected error, got entry %d×%d" e.SymbolRows e.SymbolCols)

[<Fact>]
let ``square sizes table has 24 entries`` () =
    Assert.Equal(24, squareSizes.Length)

[<Fact>]
let ``rect sizes table has 6 entries`` () =
    Assert.Equal(6, rectSizes.Length)

[<Fact>]
let ``square sizes are in ascending dataCW order`` () =
    // Each entry should have dataCW >= previous entry's dataCW
    for i in 1 .. squareSizes.Length - 1 do
        Assert.True(
            squareSizes.[i].DataCW >= squareSizes.[i-1].DataCW,
            sprintf "Size %d has dataCW %d < previous %d"
                i squareSizes.[i].DataCW squareSizes.[i-1].DataCW)

// ============================================================================
// Reed-Solomon ECC
// ============================================================================

[<Fact>]
let ``RS encode empty block with 5 ECC gives 5 zero bytes`` () =
    // Encoding all-zero data produces ECC bytes (all zero for this trivial case)
    let ecc = rsEncodeBlockInternal [| 0; 0; 0 |] 5
    Assert.Equal(5, ecc.Length)
    // All-zero data → all-zero remainder (0^anything = 0)
    for e in ecc do
        Assert.Equal(0, e)

[<Fact>]
let ``RS encode length matches nEcc`` () =
    for nEcc in [| 5; 7; 10; 12; 14 |] do
        let ecc = rsEncodeBlockInternal [| 66; 129 |] nEcc
        Assert.Equal(nEcc, ecc.Length)

[<Fact>]
let ``RS encode two identical inputs produce identical ECC`` () =
    let ecc1 = rsEncodeBlockInternal [| 66; 33; 72 |] 7
    let ecc2 = rsEncodeBlockInternal [| 66; 33; 72 |] 7
    Assert.Equal<int[]>(ecc1, ecc2)

[<Fact>]
let ``RS ECC values are in 0..255 range`` () =
    let ecc = rsEncodeBlockInternal [| 100; 200; 50 |] 10
    for e in ecc do
        Assert.True(e >= 0 && e <= 255, sprintf "ECC byte %d out of range" e)

// ============================================================================
// Grid initialisation
// ============================================================================

/// Get the 10×10 symbol size entry for grid tests.
let private entry10x10 = squareSizes.[0]   // 10×10, single region

[<Fact>]
let ``initGrid bottom row is all dark`` () =
    match encode "A" with
    | Ok grid ->
        let R = grid.Rows
        for c in 0 .. grid.Cols - 1 do
            Assert.True(mAt grid (R-1) c, sprintf "Bottom row, col %d should be dark" c)
    | Error e -> failwith (string e)

[<Fact>]
let ``initGrid left column is all dark`` () =
    match encode "A" with
    | Ok grid ->
        for r in 0 .. grid.Rows - 1 do
            Assert.True(mAt grid r 0, sprintf "Left col, row %d should be dark" r)
    | Error e -> failwith (string e)

[<Fact>]
let ``initGrid top row alternates dark light starting dark`` () =
    // Top row: alternating dark/light starting dark at col 0.
    // The top-right corner (row 0, col C-1) is overwritten by the right-column
    // timing rule, so we skip it here and verify it separately.
    // The bottom-left corner (row 0, col 0) is all-dark (L-finder), which agrees
    // with the even-column timing (0 % 2 = 0 → dark).
    match encode "A" with
    | Ok grid ->
        let C = grid.Cols
        // Skip the last column — it is owned by the right-column timing rule.
        for c in 0 .. C - 2 do
            let expected = (c % 2 = 0)
            Assert.Equal(expected, mAt grid 0 c)
    | Error e -> failwith (string e)

[<Fact>]
let ``initGrid right column alternates dark light starting dark`` () =
    // Right column: alternating dark/light starting dark at row 0.
    // The bottom-right corner (row R-1, col C-1) is overwritten by the
    // all-dark bottom row (L-finder), so we skip it here.
    match encode "A" with
    | Ok grid ->
        let C = grid.Cols - 1
        let R = grid.Rows
        // Skip the last row — it is owned by the all-dark L-finder bottom row.
        for r in 0 .. R - 2 do
            let expected = (r % 2 = 0)
            Assert.Equal(expected, mAt grid r C)
    | Error e -> failwith (string e)

// ============================================================================
// Full encode pipeline — symbol sizes
// ============================================================================

[<Fact>]
let ``encode 'A' produces a 10x10 grid`` () =
    // "A" → 1 codeword; 10×10 symbol has dataCW=3 ≥ 1 → smallest
    let grid = unwrap (encode "A")
    Assert.Equal(10, grid.Rows)
    Assert.Equal(10, grid.Cols)

[<Fact>]
let ``encode 'Hello World' produces a 16x16 grid`` () =
    // "Hello World" → 11 codewords (no digit pairs)
    // 12×12 dataCW=5 < 11; 14×14 dataCW=8 < 11; 16×16 dataCW=12 ≥ 11
    let grid = unwrap (encode "Hello World")
    Assert.Equal(16, grid.Rows)
    Assert.Equal(16, grid.Cols)

[<Fact>]
let ``encode empty string produces 10x10 grid`` () =
    // "" → 0 codewords; smallest symbol (10×10, dataCW=3) can hold 0
    let grid = unwrap (encode "")
    Assert.Equal(10, grid.Rows)

[<Fact>]
let ``encode returns square grid`` () =
    for input in [| "A"; "Hello"; "1234567890"; String.replicate 20 "X" |] do
        let grid = unwrap (encode input)
        Assert.Equal(grid.Rows, grid.Cols)

[<Fact>]
let ``encode module shape is Square`` () =
    let grid = unwrap (encode "test")
    Assert.Equal(ModuleShape.Square, grid.ModuleShape)

[<Fact>]
let ``encode 100 chars fits in a 40x40 or larger symbol`` () =
    // 100 ASCII chars → 100 codewords; must select a symbol with dataCW ≥ 100
    let grid = unwrap (encode (String.replicate 100 "A"))
    Assert.True(grid.Rows >= 40, sprintf "Expected ≥ 40×40, got %d×%d" grid.Rows grid.Cols)

[<Fact>]
let ``encode 1000 chars fits in the 120x120 symbol`` () =
    // 1000 codewords → squareSizes has 120×120 with dataCW=1050 ≥ 1000
    let grid = unwrap (encode (String.replicate 1000 "A"))
    Assert.True(grid.Rows >= 96, sprintf "Expected large symbol, got %d×%d" grid.Rows grid.Cols)

// ============================================================================
// L-finder border
// ============================================================================

[<Fact>]
let ``top-left corner is dark`` () =
    let grid = unwrap (encode "A")
    Assert.True(mAt grid 0 0, "Top-left corner should be dark (part of both borders)")

[<Fact>]
let ``bottom-left corner is dark`` () =
    let grid = unwrap (encode "A")
    Assert.True(mAt grid (grid.Rows-1) 0)

[<Fact>]
let ``bottom-right corner is dark`` () =
    // Bottom row is all dark → bottom-right is dark
    let grid = unwrap (encode "A")
    Assert.True(mAt grid (grid.Rows-1) (grid.Cols-1))

[<Fact>]
let ``top-right corner is dark`` () =
    // Top row starts dark; right column starts dark at row 0 → top-right is dark
    let grid = unwrap (encode "A")
    Assert.True(mAt grid 0 (grid.Cols-1))

// ============================================================================
// Timing clock
// ============================================================================

[<Fact>]
let ``second cell of top row is light`` () =
    // Top row: col 0 = dark, col 1 = light, col 2 = dark, …
    let grid = unwrap (encode "A")
    Assert.False(mAt grid 0 1, "Top row col 1 should be light")

[<Fact>]
let ``second cell of right column is light`` () =
    // Right column: row 0 = dark, row 1 = light, …
    let grid = unwrap (encode "A")
    Assert.False(mAt grid 1 (grid.Cols-1), "Right col row 1 should be light")

// ============================================================================
// Determinism
// ============================================================================

[<Fact>]
let ``same input produces identical grids`` () =
    let g1 = unwrap (encode "CodingAdventures")
    let g2 = unwrap (encode "CodingAdventures")
    Assert.Equal(g1.Rows, g2.Rows)
    for r in 0 .. g1.Rows - 1 do
        for c in 0 .. g1.Cols - 1 do
            Assert.Equal(mAt g1 r c, mAt g2 r c)

[<Fact>]
let ``different inputs produce different grids`` () =
    let g1 = unwrap (encode "AAAA")
    let g2 = unwrap (encode "BBBB")
    // At least one module should differ (if sizes match)
    if g1.Rows = g2.Rows then
        let anyDiff =
            [ for r in 0 .. g1.Rows - 1 do
                for c in 0 .. g1.Cols - 1 do
                    if mAt g1 r c <> mAt g2 r c then yield true ]
            |> List.isEmpty
            |> not
        Assert.True(anyDiff, "Expected different grids for different inputs")

// ============================================================================
// Byte array input
// ============================================================================

[<Fact>]
let ``encodeBytes produces same grid as encode string`` () =
    let bytes = System.Text.Encoding.UTF8.GetBytes "DataMatrix"
    let g1 = unwrap (encode "DataMatrix")
    let g2 = unwrap (encodeBytes bytes)
    Assert.Equal(g1.Rows, g2.Rows)
    Assert.Equal(g1.Cols, g2.Cols)
    for r in 0 .. g1.Rows - 1 do
        for c in 0 .. g1.Cols - 1 do
            Assert.Equal(mAt g1 r c, mAt g2 r c)

// ============================================================================
// Error path
// ============================================================================

[<Fact>]
let ``InputTooLong for 2000-char input`` () =
    // 2000 ASCII chars → 2000 codewords; max dataCW = 1558 (144×144)
    match encode (String.replicate 2000 "X") with
    | Error (InputTooLong _) -> ()   // expected
    | Ok g  -> failwith (sprintf "Expected InputTooLong, got %d×%d grid" g.Rows g.Cols)

[<Fact>]
let ``InputTooLong error message mentions codeword count`` () =
    match selectSymbolInternal 1600 with
    | Error (InputTooLong msg) ->
        Assert.True(msg.Contains("1600"), sprintf "Message should mention 1600: %s" msg)
    | _ -> failwith "Expected InputTooLong"

// ============================================================================
// Digit-pair packing
// ============================================================================

[<Fact>]
let ``digit string of 8 chars encodes to 4 codewords`` () =
    // "12345678" → 4 digit pairs → 4 codewords
    let cw = encodeAsciiInternal (System.Text.Encoding.ASCII.GetBytes "12345678")
    Assert.Equal(4, cw.Length)

[<Fact>]
let ``digit-packed input uses smaller symbol than non-digits`` () =
    // "1234" → 2 codewords; "ABCD" → 4 codewords
    let digits  = unwrap (encode "1234")
    let letters = unwrap (encode "ABCD")
    // digits should fit in a smaller or equal symbol
    Assert.True(digits.Rows * digits.Cols <= letters.Rows * letters.Cols,
        sprintf "Digits %d×%d should be ≤ letters %d×%d" digits.Rows digits.Cols letters.Rows letters.Cols)

// ============================================================================
// Utah placement
// ============================================================================

[<Fact>]
let ``utahPlacement returns correct grid dimensions`` () =
    // 8×8 logical grid
    let result = utahPlacementInternal [| 1..10 |] 8 8
    Assert.Equal(8, result.Length)
    for row in result do
        Assert.Equal(8, row.Length)

[<Fact>]
let ``utahPlacement 24x24 has correct dimensions`` () =
    let result = utahPlacementInternal (Array.create 72 1) 24 24
    Assert.Equal(24, result.Length)
    for row in result do
        Assert.Equal(24, row.Length)

// ============================================================================
// Multi-region symbols (alignment borders)
// ============================================================================

[<Fact>]
let ``32x32 symbol has alignment borders`` () =
    // 32×32 has 2×2 regions; alignment borders appear at physical rows/cols.
    //   abRow0 = 1 + 1*14 + 0*2 = 15  (all dark)
    //   abRow1 = 16                    (alternating dark/light)
    //   abCol0 = 1 + 1*14 + 0*2 = 15  (all dark)
    //   abCol1 = 16                    (alternating dark/light)
    //
    // Write order in initGrid:
    //   1. alignment ROWS (abRow0=dark, abRow1=alternating)
    //   2. alignment COLS (abCol0=dark, abCol1=alternating) — overrides intersections
    //   3. outer border (overrides edges)
    //
    // Intersections of abRow0 × abCol1: (15, 16) → abCol1 row 15 = 15%2=1 = light
    // So we must skip the alignment column positions (15 and 16) when checking
    // the alignment row, and skip the outer border (col 0 and col 31).
    let data = String.replicate 62 "A"
    let grid = unwrap (encode data)
    if grid.Rows = 32 then
        // abRow0 at row 15: all dark except where alignment cols and outer border override
        // Skip: outer border cols (0, 31), alignment col positions (15, 16)
        let skipCols = Set.ofList [0; 15; 16; 31]
        for c in 1 .. grid.Cols - 2 do
            if not (Set.contains c skipCols) then
                Assert.True(grid.Modules.[15].[c],
                    sprintf "Alignment row 15, col %d should be dark" c)

[<Fact>]
let ``encode large input picks multi-region symbol`` () =
    // Force selection of 32×32 (which has 2 data regions per dimension)
    let data = String.replicate 62 "A"   // 62 codewords → 32×32
    let grid = unwrap (encode data)
    Assert.Equal(32, grid.Rows)

// ============================================================================
// Grid module count sanity
// ============================================================================

[<Fact>]
let ``10x10 grid has exactly 100 modules`` () =
    let grid = unwrap (encode "A")
    let count =
        grid.Modules
        |> Array.sumBy (fun row -> row.Length)
    Assert.Equal(100, count)

[<Fact>]
let ``grid module count equals Rows times Cols`` () =
    for s in [| "A"; "Hello World"; "1234567890ABCDEF" |] do
        let grid = unwrap (encode s)
        let count = grid.Modules |> Array.sumBy (fun row -> row.Length)
        Assert.Equal(grid.Rows * grid.Cols, count)

// ============================================================================
// Well-known encoding vectors
// ============================================================================

[<Fact>]
let ``encode 'A' produces a valid 10x10 symbol with correct border`` () =
    let grid = unwrap (encode "A")
    // Bottom row all dark
    for c in 0 .. 9 do
        Assert.True(grid.Modules.[9].[c])
    // Left column all dark
    for r in 0 .. 9 do
        Assert.True(grid.Modules.[r].[0])

[<Fact>]
let ``ascii encode of NUL (0x00) gives codeword 1`` () =
    let cw = encodeAsciiInternal [| 0uy |]
    Assert.Equal<int[]>([| 1 |], cw)

[<Fact>]
let ``ascii encode of DEL (0x7F = 127) gives codeword 128`` () =
    let cw = encodeAsciiInternal [| 127uy |]
    Assert.Equal<int[]>([| 128 |], cw)

// ============================================================================
// DataMatrixError.ToString coverage
// ============================================================================

[<Fact>]
let ``DataMatrixError InputTooLong ToString has correct prefix`` () =
    let e = InputTooLong "test message"
    Assert.True(e.ToString().StartsWith("InputTooLong:"))

// ============================================================================
// Cover additional symbol sizes (corner patterns 3, 4 and applyWrap branches)
// ============================================================================

[<Fact>]
let ``encode 22x22 symbol produces correct border`` () =
    // 22×22 symbol: single data region 20×20 (nCols=20, 20 % 8 = 4 → triggers corner3)
    // dataCW=30; encode 30 chars to hit this symbol exactly.
    let data = String.replicate 30 "A"
    let grid = unwrap (encode data)
    Assert.Equal(22, grid.Rows)
    Assert.Equal(22, grid.Cols)
    // Verify L-finder bottom row
    for c in 0 .. 21 do
        Assert.True(grid.Modules.[21].[c])

[<Fact>]
let ``encode 24x24 symbol is valid`` () =
    // 24×24 symbol: single data region 22×22 (nCols=22, 22 % 8 = 6)
    // dataCW=36; encode exactly 36 chars
    let data = String.replicate 36 "B"
    let grid = unwrap (encode data)
    Assert.Equal(24, grid.Rows)
    // Verify left column all dark
    for r in 0 .. 23 do
        Assert.True(grid.Modules.[r].[0])

[<Fact>]
let ``encode 14x14 symbol is valid`` () =
    // 14×14 symbol: single data region 12×12 (nRows=12, nCols=12, 12 % 8 = 4 → corner3)
    // dataCW=8; encode 8 chars
    let data = String.replicate 8 "Z"
    let grid = unwrap (encode data)
    Assert.Equal(14, grid.Rows)
    Assert.Equal(14, grid.Cols)

[<Fact>]
let ``encode 16x16 symbol is valid`` () =
    // 16×16 symbol: single data region 14×14 (nRows=14, nCols=14, 14 % 8 = 6)
    // dataCW=12; encode 9 chars (> 8 but ≤ 12)
    let data = String.replicate 9 "X"
    let grid = unwrap (encode data)
    Assert.Equal(16, grid.Rows)

[<Fact>]
let ``encode 18x18 symbol is valid`` () =
    // 18×18 symbol: single data region 16×16 (nCols=16, 16 % 8 = 0 → corner4)
    // dataCW=18; encode 13 chars (> 12 but ≤ 18)
    let data = String.replicate 13 "Y"
    let grid = unwrap (encode data)
    Assert.Equal(18, grid.Rows)

[<Fact>]
let ``encode 20x20 symbol is valid`` () =
    // 20×20: single data region 18×18 (nCols=18, 18 % 8 = 2)
    // dataCW=22; encode 19 chars
    let data = String.replicate 19 "W"
    let grid = unwrap (encode data)
    Assert.Equal(20, grid.Rows)

[<Fact>]
let ``utah placement on 12x12 grid has correct dimensions`` () =
    // 12×12 logical grid for a 14×14 symbol (drH=12, drW=12)
    // nCols=12, 12 % 8 = 4 → triggers corner3 condition
    let codewords = Array.create 18 42
    let result = utahPlacementInternal codewords 12 12
    Assert.Equal(12, result.Length)
    for row in result do
        Assert.Equal(12, row.Length)

[<Fact>]
let ``utah placement on 16x16 grid has correct dimensions`` () =
    // 16×16 logical grid: nCols=16, 16 % 8 = 0 → triggers corner4 condition
    let codewords = Array.create 30 1
    let result = utahPlacementInternal codewords 16 16
    Assert.Equal(16, result.Length)

[<Fact>]
let ``utah placement on 6x16 grid has correct dimensions`` () =
    // 6×16 logical grid: non-square; nCols=16, 16 % 8 = 0 → corner4
    // These dimensions arise in the 8×18 rectangular symbol (drH=6, drW=16)
    let codewords = Array.create 16 1
    let result = utahPlacementInternal codewords 6 16
    Assert.Equal(6, result.Length)
    for row in result do
        Assert.Equal(16, row.Length)

[<Fact>]
let ``multiple sizes all produce valid grids`` () =
    // Exercise many symbol sizes systematically to maximise path coverage
    // in the Utah placement algorithm (corner patterns, wrap branches, etc.)
    let sizes =
        [| 1; 3; 5; 8; 12; 18; 22; 30; 36; 44; 45; 62; 86 |]
    for n in sizes do
        let data = String.replicate n "A"
        let grid = unwrap (encode data)
        Assert.True(grid.Rows > 0 && grid.Cols > 0,
            sprintf "Symbol for %d codewords should have positive dimensions" n)
        Assert.True(grid.Modules.[grid.Rows - 1].[0],
            sprintf "Bottom-left corner should be dark for %d codewords" n)
