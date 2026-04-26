/// QRCodeTests.fs — Unit tests for CodingAdventures.QRCode
///
/// Coverage targets (all must pass ≥ 90 % line coverage):
///   - Version constant
///   - ECC level handling (L, M, Q, H)
///   - Mode selection (numeric, alphanumeric, byte)
///   - Version selection (v1 through a few higher versions)
///   - Structural validity of the output grid
///       – correct size for each version
///       – finder pattern presence and shape
///       – format information BCH validity
///       – dark module presence
///   - Capacity: small inputs fit in v1, medium in mid-range, large near v40
///   - Error cases: input too long
///   - Numeric encoding: "0000000" etc.
///   - Alphanumeric encoding: canonical QR alphanum string
///   - Byte encoding: arbitrary UTF-8

module CodingAdventures.QRCode.Tests

open System
open Xunit
open CodingAdventures.QRCode
open CodingAdventures.Barcode2D

// ============================================================================
// Helpers
// ============================================================================

/// Unwrap a Result, failing the test with the error message if Error.
let unwrap result =
    match result with
    | Ok v    -> v
    | Error e -> failwith (sprintf "Expected Ok but got Error: %A" e)

/// Verify that a 7×7 finder pattern is correctly placed at (top, left).
///
/// A finder pattern looks like this:
///
///   ■ ■ ■ ■ ■ ■ ■
///   ■ □ □ □ □ □ ■
///   ■ □ ■ ■ ■ □ ■
///   ■ □ ■ ■ ■ □ ■
///   ■ □ ■ ■ ■ □ ■
///   ■ □ □ □ □ □ ■
///   ■ ■ ■ ■ ■ ■ ■
let hasFinder (modules: bool[][] ) (top: int) (left: int) =
    let mutable ok = true
    for dr in 0 .. 6 do
        for dc in 0 .. 6 do
            let onBorder = dr = 0 || dr = 6 || dc = 0 || dc = 6
            let inCore   = dr >= 2 && dr <= 4 && dc >= 2 && dc <= 4
            let expected = onBorder || inCore
            if modules.[top + dr].[left + dc] <> expected then ok <- false
    ok

/// Read back the format information from copy 1 on the finished grid and
/// verify its BCH parity.  Returns Some (eccIndicator, maskPattern) if valid.
///
/// Copy 1 layout (f14 = MSB):
///   (8,0)→f14  (8,1)→f13  (8,2)→f12  (8,3)→f11  (8,4)→f10  (8,5)→f9
///   (8,7)→f8   (8,8)→f7
///   (7,8)→f6
///   (5,8)→f5   (4,8)→f4   (3,8)→f3   (2,8)→f2   (1,8)→f1   (0,8)→f0
let formatInfoValid (modules: bool[][]) : (uint32 * uint32) option =
    // Read raw bits f14..f0 in that order
    let positions =
        [| (8,0); (8,1); (8,2); (8,3); (8,4); (8,5); (8,7); (8,8)
           (7,8); (5,8); (4,8); (3,8); (2,8); (1,8); (0,8) |]
    let mutable raw = 0u
    for i in 0 .. 14 do
        let (r, c) = positions.[i]
        if modules.[r].[c] then raw <- raw ||| (1u <<< (14 - i))
    // Remove the ISO masking sequence to get the plain format word
    let fmt = raw ^^^ 0x5412u
    // BCH verify: recompute the 10-bit remainder from the 5-bit data portion
    let mutable rem = (fmt >>> 10) <<< 10
    for i in 14 .. -1 .. 10 do
        if (rem >>> i) &&& 1u = 1u then rem <- rem ^^^ (0x537u <<< (i - 10))
    if (rem &&& 0x3FFu) <> (fmt &&& 0x3FFu) then None
    else Some ((fmt >>> 13) &&& 0x3u, (fmt >>> 10) &&& 0x7u)

// ============================================================================
// Basic sanity
// ============================================================================

[<Fact>]
let ``VERSION is 0.1.0`` () =
    Assert.Equal("0.1.0", VERSION)

// ============================================================================
// Grid size
// ============================================================================

// Grid sizes: version V → (4V+17) × (4V+17).
// We pick inputs that are exactly long enough to require each version at ECC L.
// Numeric-mode capacities at ECC L (from ISO 18004 Table 9):
//   v1-L: 41 digits   v2-L: 77 digits   v4-L: 187 digits  v9-L: 468 digits
// Using N+1 digits forces the next version up.

[<Fact>]
let ``version 1 grid is 21x21`` () =
    // Any short input produces version 1.
    let grid = unwrap (encode "1" EccLevel.L)
    Assert.Equal(21, grid.Rows)
    Assert.Equal(21, grid.Cols)

[<Fact>]
let ``version 2 grid is 25x25`` () =
    // 42 numeric digits exceed v1-L capacity (41), forcing version 2.
    let grid = unwrap (encode (String.replicate 42 "1") EccLevel.L)
    Assert.Equal(25, grid.Rows)
    Assert.Equal(25, grid.Cols)

[<Fact>]
let ``version 5 grid is 37x37`` () =
    // 188 numeric digits exceed v4-L numeric capacity (187), forcing version 5.
    let grid = unwrap (encode (String.replicate 188 "1") EccLevel.L)
    Assert.Equal(37, grid.Rows)
    Assert.Equal(37, grid.Cols)

[<Fact>]
let ``version 10 or higher grid from large input`` () =
    // 600 numeric digits exceed v9-L numeric capacity, forcing version 10+.
    // Version 9 symbol is 53×53 (4·9+17), so ≥57 rows confirms v10+.
    let grid = unwrap (encode (String.replicate 600 "1") EccLevel.L)
    Assert.True(grid.Rows >= 57, sprintf "Expected ≥57 rows for large input, got %d" grid.Rows)

// ============================================================================
// Finder patterns
// ============================================================================

[<Fact>]
let ``version-1 grid has three valid finder patterns`` () =
    let grid = unwrap (encode "HELLO WORLD" EccLevel.M)
    let m    = grid.Modules
    let sz   = grid.Rows

    Assert.True(hasFinder m 0 0,          "top-left finder")
    Assert.True(hasFinder m 0 (sz - 7),   "top-right finder")
    Assert.True(hasFinder m (sz - 7) 0,   "bottom-left finder")

[<Fact>]
let ``version-5 grid has three valid finder patterns`` () =
    // 188 numeric digits force version 5 at ECC L → 37×37 grid.
    let input = String.replicate 188 "1"
    let grid  = unwrap (encode input EccLevel.L)
    let m     = grid.Modules
    let sz    = grid.Rows

    Assert.Equal(37, sz)
    Assert.True(hasFinder m 0 0,          "top-left finder")
    Assert.True(hasFinder m 0 (sz - 7),   "top-right finder")
    Assert.True(hasFinder m (sz - 7) 0,   "bottom-left finder")

// ============================================================================
// Format information
// ============================================================================

[<Theory>]
[<InlineData("HELLO WORLD", "M")>]
[<InlineData("12345", "L")>]
[<InlineData("TEST", "Q")>]
[<InlineData("ABC", "H")>]
let ``format info BCH is valid and ECC matches`` (input: string) (eccStr: string) =
    let ecc =
        match eccStr with
        | "L" -> EccLevel.L
        | "M" -> EccLevel.M
        | "Q" -> EccLevel.Q
        | "H" -> EccLevel.H
        | _   -> EccLevel.M

    let expectedIndicator =
        match ecc with
        | EccLevel.L -> 0b01u
        | EccLevel.M -> 0b00u
        | EccLevel.Q -> 0b11u
        | EccLevel.H -> 0b10u

    let grid = unwrap (encode input ecc)
    match formatInfoValid grid.Modules with
    | None                -> Assert.Fail("Format info BCH check failed")
    | Some (indicator, _) -> Assert.Equal(expectedIndicator, indicator)

// ============================================================================
// Dark module
// ============================================================================

[<Fact>]
let ``dark module is present at (4V+9, 8) for version 1`` () =
    let grid = unwrap (encode "HELLO WORLD" EccLevel.M)
    // Version 1 → dark module at (4·1+9, 8) = (13, 8)
    Assert.True(grid.Modules.[13].[8], "dark module at (13,8)")

[<Fact>]
let ``dark module is present at (4V+9, 8) for version 5`` () =
    // Force version 5: 188 numeric digits exceed v4-L capacity (187).
    let input = String.replicate 188 "1"
    let grid  = unwrap (encode input EccLevel.L)
    // Version 5 → grid is 37×37; dark module at (4·5+9, 8) = (29, 8)
    Assert.Equal(37, grid.Rows)
    Assert.True(grid.Modules.[29].[8], "dark module at (29,8)")

// ============================================================================
// Mode selection and capacity
// ============================================================================

[<Fact>]
let ``numeric input stays in version 1 at ECC M`` () =
    // "01234567" — 8 numeric chars; version 1-M numeric capacity is 25 chars.
    let grid = unwrap (encode "01234567" EccLevel.M)
    Assert.Equal(21, grid.Rows)

[<Fact>]
let ``alphanumeric input version 1 at ECC M`` () =
    // "HELLO WORLD" — 11 chars including space; version 1-M alphanum cap is 14.
    let grid = unwrap (encode "HELLO WORLD" EccLevel.M)
    Assert.Equal(21, grid.Rows)

[<Fact>]
let ``byte input for UTF-8 string`` () =
    // Lowercase letters trigger byte mode.
    let grid = unwrap (encode "hello" EccLevel.M)
    Assert.Equal(21, grid.Rows)

[<Fact>]
let ``pure numeric string uses smaller version than byte mode`` () =
    // A 20-digit number should fit in v1 at ECC L (numeric cap = 41 digits).
    let gridNum  = unwrap (encode "12345678901234567890" EccLevel.L)
    // The same content as bytes would be 20 bytes; v1-L byte cap is 19 bytes,
    // so this is exactly at the boundary (v2-L byte cap is 34 bytes).
    Assert.Equal(21, gridNum.Rows)

[<Fact>]
let ``medium input selects higher version`` () =
    // 300 numeric digits — version 1 cannot hold them; ensure version > 1.
    let input = String.replicate 300 "3"
    let grid  = unwrap (encode input EccLevel.L)
    Assert.True(grid.Rows > 21, sprintf "Expected version > 1, got %d rows" grid.Rows)

[<Fact>]
let ``all four ECC levels encode without error`` () =
    let input = "HELLO"
    for ecc in [| EccLevel.L; EccLevel.M; EccLevel.Q; EccLevel.H |] do
        let grid = unwrap (encode input ecc)
        Assert.True(grid.Rows >= 21)

// ============================================================================
// Error cases
// ============================================================================

[<Fact>]
let ``empty string encodes without error`` () =
    // Empty string: byte mode, 0 data bits, fits trivially in v1.
    let grid = unwrap (encode "" EccLevel.M)
    Assert.Equal(21, grid.Rows)

[<Fact>]
let ``overly long input returns InputTooLong`` () =
    // 8000 bytes is beyond version-40 capacity in any mode.
    let input = String.replicate 8000 "A"
    match encode input EccLevel.H with
    | Error (InputTooLong _) -> ()
    | Ok _                   -> Assert.Fail("Expected InputTooLong error")

[<Fact>]
let ``input at exact 7089 byte limit returns InputTooLong because byte mode can't hold 7089`` () =
    // 7089 lowercase 'a' characters: byte mode at ECC L version 40 capacity is
    // 2953 bytes, so this will fail with InputTooLong.
    let input = String.replicate 7089 "a"
    match encode input EccLevel.L with
    | Error (InputTooLong _) -> ()
    | Ok _                   -> Assert.Fail("Expected InputTooLong error (byte mode overflow)")

// ============================================================================
// Structural properties
// ============================================================================

[<Fact>]
let ``module grid dimensions match Rows and Cols`` () =
    let grid = unwrap (encode "HELLO WORLD" EccLevel.M)
    Assert.Equal(grid.Rows, grid.Modules.Length)
    for row in grid.Modules do
        Assert.Equal(grid.Cols, row.Length)

[<Fact>]
let ``timing row 6 alternates dark light starting dark`` () =
    let grid = unwrap (encode "HELLO WORLD" EccLevel.M)
    let sz   = grid.Rows
    // Timing row is row 6, cols 8 to sz-9
    for c in 8 .. sz - 9 do
        let expected = (c % 2 = 0)
        Assert.Equal(expected, grid.Modules.[6].[c])

[<Fact>]
let ``timing col 6 alternates dark light starting dark`` () =
    let grid = unwrap (encode "HELLO WORLD" EccLevel.M)
    let sz   = grid.Rows
    for r in 8 .. sz - 9 do
        let expected = (r % 2 = 0)
        Assert.Equal(expected, grid.Modules.[r].[6])

// ============================================================================
// Version info (v7+)
// ============================================================================

[<Fact>]
let ``version 7 grid has correct size`` () =
    // Version 7 symbol is 45×45.  Use enough data to force version 7 at ECC L.
    // Version 7-L byte capacity is 154 bytes.
    let input = String.replicate 155 "a"   // forces v8+; just check size ≥ 45
    let grid  = unwrap (encode input EccLevel.L)
    Assert.True(grid.Rows >= 45)

[<Fact>]
let ``version-10 format info is valid`` () =
    let input = String.replicate 280 "1"   // forces version 10+ at ECC L
    let grid  = unwrap (encode input EccLevel.L)
    match formatInfoValid grid.Modules with
    | None   -> Assert.Fail("Format info invalid for high-version symbol")
    | Some _ -> ()

// ============================================================================
// Alphanumeric character set
// ============================================================================

[<Fact>]
let ``full alphanumeric charset encodes successfully`` () =
    let input = "0123456789ABCDEFGHIJKLMNOPQRSTUVWXYZ $%*+-./:"
    let grid  = unwrap (encode input EccLevel.L)
    Assert.True(grid.Rows >= 21)

[<Fact>]
let ``alphanumeric with space encodes correctly`` () =
    // Space is character index 36 in the alphanumeric table.
    let grid = unwrap (encode "HELLO WORLD" EccLevel.Q)
    Assert.Equal(21, grid.Rows)

// ============================================================================
// Numeric encoding edge cases
// ============================================================================

[<Fact>]
let ``single digit numeric encodes`` () =
    let grid = unwrap (encode "0" EccLevel.L)
    Assert.Equal(21, grid.Rows)

[<Fact>]
let ``two digit numeric encodes`` () =
    let grid = unwrap (encode "42" EccLevel.L)
    Assert.Equal(21, grid.Rows)

[<Fact>]
let ``three digit numeric encodes`` () =
    let grid = unwrap (encode "123" EccLevel.L)
    Assert.Equal(21, grid.Rows)

[<Fact>]
let ``all zeros numeric encodes`` () =
    let grid = unwrap (encode "000" EccLevel.M)
    Assert.Equal(21, grid.Rows)

// ============================================================================
// Byte mode
// ============================================================================

[<Fact>]
let ``URL encodes in byte mode`` () =
    let grid = unwrap (encode "https://example.com" EccLevel.M)
    Assert.True(grid.Rows >= 21)

[<Fact>]
let ``mixed case forces byte mode`` () =
    // Lowercase 'l' is not in the alphanumeric charset.
    let grid = unwrap (encode "Hello World" EccLevel.M)
    Assert.True(grid.Rows >= 21)

// ============================================================================
// Grid invariants
// ============================================================================

[<Fact>]
let ``separator rows are all light around top-left finder`` () =
    let grid = unwrap (encode "HELLO WORLD" EccLevel.M)
    let m    = grid.Modules
    // Row 7, cols 0-7 should be all light (separator)
    for c in 0 .. 7 do
        Assert.False(m.[7].[c], sprintf "Expected light at (7,%d)" c)
    // Col 7, rows 0-7 should be all light (separator)
    for r in 0 .. 7 do
        Assert.False(m.[r].[7], sprintf "Expected light at (%d,7)" r)

[<Fact>]
let ``module grid has no negative or out-of-range indexing`` () =
    // Just verify the jagged array structure is fully populated.
    let grid = unwrap (encode "TEST" EccLevel.H)
    let sz   = grid.Rows
    let mutable count = 0
    for r in 0 .. sz - 1 do
        for c in 0 .. sz - 1 do
            let _ = grid.Modules.[r].[c]
            count <- count + 1
    Assert.Equal(sz * sz, count)

// ============================================================================
// ECC level Q and H produce valid grids
// ============================================================================

[<Fact>]
let ``ECC Q produces valid format info`` () =
    let grid = unwrap (encode "HELLO WORLD" EccLevel.Q)
    match formatInfoValid grid.Modules with
    | None              -> Assert.Fail("Format info invalid for ECC Q")
    | Some (indicator, _) -> Assert.Equal(0b11u, indicator)

[<Fact>]
let ``ECC H produces valid format info`` () =
    let grid = unwrap (encode "HELLO WORLD" EccLevel.H)
    match formatInfoValid grid.Modules with
    | None              -> Assert.Fail("Format info invalid for ECC H")
    | Some (indicator, _) -> Assert.Equal(0b10u, indicator)

[<Fact>]
let ``ECC L produces valid format info`` () =
    let grid = unwrap (encode "HELLO WORLD" EccLevel.L)
    match formatInfoValid grid.Modules with
    | None              -> Assert.Fail("Format info invalid for ECC L")
    | Some (indicator, _) -> Assert.Equal(0b01u, indicator)

// ============================================================================
// Determinism
// ============================================================================

[<Fact>]
let ``encoding the same input twice produces identical grids`` () =
    let input = "HELLO WORLD"
    let g1    = unwrap (encode input EccLevel.M)
    let g2    = unwrap (encode input EccLevel.M)
    Assert.Equal(g1.Rows, g2.Rows)
    let sz = g1.Rows
    for r in 0 .. sz - 1 do
        for c in 0 .. sz - 1 do
            Assert.Equal(g1.Modules.[r].[c], g2.Modules.[r].[c])
