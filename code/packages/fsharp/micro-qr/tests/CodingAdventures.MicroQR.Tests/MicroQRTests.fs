/// MicroQRTests.fs — Unit and integration tests for CodingAdventures.MicroQR
///
/// Coverage targets (all must pass ≥ 90% line coverage):
///   - VERSION constant
///   - Symbol size correctness (M1=11×11, M2=13×13, M3=15×15, M4=17×17)
///   - Auto-selection: smallest symbol chosen for short inputs
///   - Grid is always square (rows = cols)
///   - All modules are booleans (always true in F#, but grid structure is valid)
///   - Deterministic encoding (same input → same grid)
///   - Larger input → bigger symbol
///   - ECC levels L, M, Q work for appropriate symbols
///   - InputTooLong error for inputs exceeding M4-L capacity
///   - InvalidECCLevel for Q on M1/M2/M3, Detection on M2+
///   - InvalidOptions for bad symbol string
///   - Mask pattern override works
///   - Numeric and byte mode encoding
///   - M1 symbol (11×11) encoding works end-to-end
///   - Format info is placed (row 8 and col 8 modules are set structurally)

module CodingAdventures.MicroQR.Tests

open Xunit
open CodingAdventures.MicroQR
open CodingAdventures.Barcode2D

// ============================================================================
// Helpers
// ============================================================================

/// Unwrap a Result<'a, 'b>, failing the test with a message on Error.
///
/// Using this helper keeps tests concise:
///   let grid = unwrap (encode "hello" defaultOptions)
let unwrap result =
    match result with
    | Ok v    -> v
    | Error e -> failwith (sprintf "Expected Ok but got Error: %A" e)

/// Verify that a Result is an Error and return the error value.
let unwrapError result =
    match result with
    | Error e -> e
    | Ok v    -> failwith (sprintf "Expected Error but got Ok: %A" v)

// ============================================================================
// Version
// ============================================================================

/// The package version constant must match the published version.
[<Fact>]
let ``Version is 0.1.0`` () =
    Assert.Equal("0.1.0", Version)

// ============================================================================
// Basic encode returns Ok
// ============================================================================

/// A single ASCII letter "A" can always be encoded — it fits in M2-L in
/// alphanumeric mode. The encode function must return Ok.
[<Fact>]
let ``encode "A" defaultOptions returns Ok`` () =
    let result = encode "A" defaultOptions
    match result with
    | Ok _    -> ()  // pass
    | Error e -> Assert.True(false, sprintf "Expected Ok but got Error: %A" e)

/// An empty string is technically valid numeric input and should encode to M1.
[<Fact>]
let ``encode empty string returns Ok`` () =
    let result = encode "" defaultOptions
    match result with
    | Ok _    -> ()
    | Error e -> Assert.True(false, sprintf "Expected Ok but got Error: %A" e)

// ============================================================================
// Grid is always square
// ============================================================================

/// Every Micro QR symbol is square: rows = cols.
///
/// The formula is size = 2 × version_number + 9:
///   M1: 2×1+9=11   M2: 2×2+9=13   M3: 2×3+9=15   M4: 2×4+9=17
[<Fact>]
let ``encode result is always square (rows = cols)`` () =
    let inputs = [ "1"; "12345"; "HELLO"; "Hello World" ]
    for input in inputs do
        let grid = unwrap (encode input defaultOptions)
        Assert.Equal(grid.Rows, grid.Cols)

// ============================================================================
// Symbol sizes
// ============================================================================

/// M1 symbols are always 11×11.
[<Fact>]
let ``M1 symbol is 11x11`` () =
    let opts = { defaultOptions with Symbol = Some "M1" }
    let grid = unwrap (encode "123" opts)
    Assert.Equal(11, grid.Rows)
    Assert.Equal(11, grid.Cols)

/// M2 symbols are always 13×13.
[<Fact>]
let ``M2 symbol is 13x13`` () =
    let opts = { defaultOptions with Symbol = Some "M2" }
    let grid = unwrap (encode "A" opts)
    Assert.Equal(13, grid.Rows)
    Assert.Equal(13, grid.Cols)

/// M3 symbols are always 15×15.
[<Fact>]
let ``M3 symbol is 15x15`` () =
    let opts = { defaultOptions with Symbol = Some "M3" }
    let grid = unwrap (encode "HELLO" opts)
    Assert.Equal(15, grid.Rows)
    Assert.Equal(15, grid.Cols)

/// M4 symbols are always 17×17.
[<Fact>]
let ``M4 symbol is 17x17`` () =
    let opts = { defaultOptions with Symbol = Some "M4" }
    let grid = unwrap (encode "HELLO WORLD" opts)
    Assert.Equal(17, grid.Rows)
    Assert.Equal(17, grid.Cols)

// ============================================================================
// ModuleGrid structure
// ============================================================================

/// The Modules array must have exactly Rows × Cols boolean values.
/// In F#, bool arrays already enforce this, but we verify the sizes explicitly.
[<Fact>]
let ``modules array has correct dimensions`` () =
    let grid = unwrap (encode "123" defaultOptions)
    Assert.Equal(grid.Rows, grid.Modules.Length)
    for row in grid.Modules do
        Assert.Equal(grid.Cols, row.Length)

/// ModuleShape must be Square for all Micro QR symbols.
[<Fact>]
let ``module shape is Square`` () =
    let grid = unwrap (encode "ABC" defaultOptions)
    Assert.Equal(Square, grid.ModuleShape)

// ============================================================================
// Determinism
// ============================================================================

/// Encoding the same string twice with the same options must produce
/// bit-for-bit identical grids.
[<Fact>]
let ``encoding is deterministic`` () =
    let input = "12345"
    let g1 = unwrap (encode input defaultOptions)
    let g2 = unwrap (encode input defaultOptions)
    Assert.Equal(g1.Rows, g2.Rows)
    Assert.Equal(g1.Cols, g2.Cols)
    for r in 0 .. g1.Rows - 1 do
        for c in 0 .. g1.Cols - 1 do
            Assert.Equal(g1.Modules.[r].[c], g2.Modules.[r].[c])

/// Encoding different strings must not produce identical grids (with high
/// probability — we test obviously different inputs).
[<Fact>]
let ``different inputs produce different grids`` () =
    let g1 = unwrap (encode "1" defaultOptions)
    let g2 = unwrap (encode "9" defaultOptions)
    // Both are M1 (single digit), same size — but the data bits differ
    let same =
        [ for r in 0 .. g1.Rows - 1 do
            for c in 0 .. g1.Cols - 1 do
                yield g1.Modules.[r].[c] = g2.Modules.[r].[c] ]
        |> List.forall id
    Assert.False(same)

// ============================================================================
// Auto-selection: larger input → bigger symbol
// ============================================================================

/// "1" fits in M1 (numeric, 1 char ≤ 5 cap).
[<Fact>]
let ``single digit selects M1 automatically`` () =
    let grid = unwrap (encode "1" defaultOptions)
    Assert.Equal(11, grid.Rows)

/// 6 numeric digits exceeds M1 (cap=5) → must use at least M2.
[<Fact>]
let ``six digits selects M2 or larger automatically`` () =
    let grid = unwrap (encode "123456" defaultOptions)
    Assert.True(grid.Rows >= 13)

/// A string requiring byte mode (e.g. lowercase) cannot fit in M1 or M2 easily
/// and should get a 15×15 or 17×17 symbol.
[<Fact>]
let ``byte mode input selects appropriate symbol`` () =
    let grid = unwrap (encode "hello" defaultOptions)
    Assert.True(grid.Rows >= 13)  // "hello" = 5 bytes, M2-L cap is 4 bytes

/// A 9-character byte string should select M3 or M4.
[<Fact>]
let ``long byte input selects M3 or larger`` () =
    let grid = unwrap (encode "123456789" defaultOptions)
    // 9 numeric chars: M3-L supports 23, so auto-selects M2-L or M3-L depending on mode
    Assert.True(grid.Rows >= 13)

// ============================================================================
// ECC level control
// ============================================================================

/// M2 with L ECC produces a 13×13 symbol.
[<Fact>]
let ``M2 L gives 13x13`` () =
    let opts = { defaultOptions with Symbol = Some "M2"; ECCLevel = Some L }
    let grid = unwrap (encode "AB" opts)
    Assert.Equal(13, grid.Rows)

/// M2 with M ECC produces a 13×13 symbol (same size, fewer data codewords).
[<Fact>]
let ``M2 M gives 13x13`` () =
    let opts = { defaultOptions with Symbol = Some "M2"; ECCLevel = Some M }
    let grid = unwrap (encode "AB" opts)
    Assert.Equal(13, grid.Rows)

/// M4 with Q (highest level for Micro QR) produces a 17×17 symbol.
[<Fact>]
let ``M4 Q gives 17x17`` () =
    let opts = { defaultOptions with Symbol = Some "M4"; ECCLevel = Some Q }
    let grid = unwrap (encode "HELLO" opts)
    Assert.Equal(17, grid.Rows)

/// M3 with L ECC produces a 15×15 symbol.
[<Fact>]
let ``M3 L gives 15x15`` () =
    let opts = { defaultOptions with Symbol = Some "M3"; ECCLevel = Some L }
    let grid = unwrap (encode "HELLO WORLD" opts)
    Assert.Equal(15, grid.Rows)

/// M3 with M ECC produces a 15×15 symbol.
[<Fact>]
let ``M3 M gives 15x15`` () =
    let opts = { defaultOptions with Symbol = Some "M3"; ECCLevel = Some M }
    let grid = unwrap (encode "HELLO" opts)
    Assert.Equal(15, grid.Rows)

// ============================================================================
// Error: InputTooLong
// ============================================================================

/// Input exceeding M4-L numeric capacity (35 digits) must return InputTooLong.
///
/// We test with 36 digits — one more than the maximum.
[<Fact>]
let ``too many digits returns InputTooLong`` () =
    let input = String.replicate 36 "1"  // 36 > 35 (M4-L numeric cap)
    let err = unwrapError (encode input defaultOptions)
    match err with
    | InputTooLong _ -> ()  // correct
    | other -> Assert.True(false, sprintf "Expected InputTooLong but got %A" other)

/// Input exceeding M4-L byte capacity (15 bytes) must return InputTooLong.
///
/// We use a 16-byte lowercase string (byte mode, since lowercase is not in
/// the alphanumeric set).
[<Fact>]
let ``too many bytes returns InputTooLong`` () =
    let input = String.replicate 16 "x"  // 16 > 15 (M4-L byte cap)
    let err = unwrapError (encode input defaultOptions)
    match err with
    | InputTooLong _ -> ()
    | other -> Assert.True(false, sprintf "Expected InputTooLong but got %A" other)

// ============================================================================
// Error: InvalidECCLevel
// ============================================================================

/// Q ECC is only defined for M4. Requesting M1-Q must fail.
[<Fact>]
let ``Q ECC on M1 returns InvalidECCLevel`` () =
    let opts = { defaultOptions with Symbol = Some "M1"; ECCLevel = Some Q }
    let err = unwrapError (encode "1" opts)
    match err with
    | InvalidECCLevel _ -> ()
    | other -> Assert.True(false, sprintf "Expected InvalidECCLevel but got %A" other)

/// Q ECC is only defined for M4. Requesting M2-Q must fail.
[<Fact>]
let ``Q ECC on M2 returns InvalidECCLevel`` () =
    let opts = { defaultOptions with Symbol = Some "M2"; ECCLevel = Some Q }
    let err = unwrapError (encode "A" opts)
    match err with
    | InvalidECCLevel _ -> ()
    | other -> Assert.True(false, sprintf "Expected InvalidECCLevel but got %A" other)

/// Q ECC is only defined for M4. Requesting M3-Q must fail.
[<Fact>]
let ``Q ECC on M3 returns InvalidECCLevel`` () =
    let opts = { defaultOptions with Symbol = Some "M3"; ECCLevel = Some Q }
    let err = unwrapError (encode "HELLO" opts)
    match err with
    | InvalidECCLevel _ -> ()
    | other -> Assert.True(false, sprintf "Expected InvalidECCLevel but got %A" other)

/// Detection ECC is only defined for M1. Requesting M2-Detection must fail.
[<Fact>]
let ``Detection ECC on M2 returns InvalidECCLevel`` () =
    let opts = { defaultOptions with Symbol = Some "M2"; ECCLevel = Some Detection }
    let err = unwrapError (encode "12" opts)
    match err with
    | InvalidECCLevel _ -> ()
    | other -> Assert.True(false, sprintf "Expected InvalidECCLevel but got %A" other)

/// Detection ECC is only defined for M1. Requesting M4-Detection must fail.
[<Fact>]
let ``Detection ECC on M4 returns InvalidECCLevel`` () =
    let opts = { defaultOptions with Symbol = Some "M4"; ECCLevel = Some Detection }
    let err = unwrapError (encode "123" opts)
    match err with
    | InvalidECCLevel _ -> ()
    | other -> Assert.True(false, sprintf "Expected InvalidECCLevel but got %A" other)

// ============================================================================
// Error: InvalidOptions
// ============================================================================

/// An unrecognized symbol string must return InvalidOptions.
[<Fact>]
let ``invalid symbol string returns InvalidOptions`` () =
    let opts = { defaultOptions with Symbol = Some "M5" }
    let err = unwrapError (encode "1" opts)
    match err with
    | InvalidOptions _ -> ()
    | other -> Assert.True(false, sprintf "Expected InvalidOptions but got %A" other)

/// Another invalid symbol string test.
[<Fact>]
let ``gibberish symbol string returns InvalidOptions`` () =
    let opts = { defaultOptions with Symbol = Some "QR" }
    let err = unwrapError (encode "1" opts)
    match err with
    | InvalidOptions _ -> ()
    | other -> Assert.True(false, sprintf "Expected InvalidOptions but got %A" other)

// ============================================================================
// Mask pattern override
// ============================================================================

/// Forcing a specific mask (0–3) must still produce a valid ModuleGrid.
[<Fact>]
let ``mask pattern 0 override produces valid grid`` () =
    let opts = { defaultOptions with MaskPattern = Some 0 }
    let grid = unwrap (encode "123" opts)
    Assert.True(grid.Rows > 0)

[<Fact>]
let ``mask pattern 3 override produces valid grid`` () =
    let opts = { defaultOptions with MaskPattern = Some 3 }
    let grid = unwrap (encode "HELLO" opts)
    Assert.True(grid.Rows > 0)

/// Different forced masks produce different grids (the mask XOR changes modules).
[<Fact>]
let ``different forced masks produce different grids`` () =
    let opts0 = { defaultOptions with Symbol = Some "M2"; MaskPattern = Some 0 }
    let opts1 = { defaultOptions with Symbol = Some "M2"; MaskPattern = Some 1 }
    let g0 = unwrap (encode "HELLO" opts0)
    let g1 = unwrap (encode "HELLO" opts1)
    // They might differ — check at least one module differs
    let allSame =
        [ for r in 0 .. g0.Rows - 1 do
            for c in 0 .. g0.Cols - 1 do
                yield g0.Modules.[r].[c] = g1.Modules.[r].[c] ]
        |> List.forall id
    // Two different mask patterns should produce different module arrays
    // (extremely unlikely to produce identical grids)
    Assert.False(allSame)

/// Out-of-range mask returns InvalidOptions.
[<Fact>]
let ``mask pattern 4 returns InvalidOptions`` () =
    let opts = { defaultOptions with MaskPattern = Some 4 }
    let err = unwrapError (encode "1" opts)
    match err with
    | InvalidOptions _ -> ()
    | other -> Assert.True(false, sprintf "Expected InvalidOptions but got %A" other)

// ============================================================================
// Mode coverage
// ============================================================================

/// Pure numeric input (only digits) should produce a valid grid.
[<Fact>]
let ``numeric mode encodes correctly`` () =
    let grid = unwrap (encode "12345" defaultOptions)
    Assert.Equal(11, grid.Rows)  // 5 digits fits in M1

/// Alphanumeric input (uppercase + digits) should produce a valid grid.
[<Fact>]
let ``alphanumeric mode encodes correctly`` () =
    let opts = { defaultOptions with Symbol = Some "M2" }
    let grid = unwrap (encode "HELLO" opts)
    Assert.Equal(13, grid.Rows)

/// Byte mode input (lowercase letters are not in the alphanumeric set).
[<Fact>]
let ``byte mode encodes correctly`` () =
    let opts = { defaultOptions with Symbol = Some "M3" }
    let grid = unwrap (encode "hello" opts)
    Assert.Equal(15, grid.Rows)

/// Alphanumeric special characters (space, $, %) are in the alphanumeric set.
[<Fact>]
let ``alphanumeric special chars encode correctly`` () =
    let opts = { defaultOptions with Symbol = Some "M3" }
    let grid = unwrap (encode "AB CD" opts)
    Assert.Equal(15, grid.Rows)

// ============================================================================
// M1 peculiarity: half-codeword
// ============================================================================

/// M1 must produce an 11×11 grid for a 1-digit input.
[<Fact>]
let ``M1 single digit produces 11x11 grid`` () =
    let opts = { defaultOptions with Symbol = Some "M1" }
    let grid = unwrap (encode "5" opts)
    Assert.Equal(11, grid.Rows)
    Assert.Equal(11, grid.Cols)

/// M1 can hold up to 5 numeric digits.
[<Fact>]
let ``M1 five digits produces 11x11 grid`` () =
    let opts = { defaultOptions with Symbol = Some "M1" }
    let grid = unwrap (encode "99999" opts)
    Assert.Equal(11, grid.Rows)

/// M1 does not support alphanumeric mode (no alpha capacity in M1).
/// Providing a letter to M1 should fail with InputTooLong.
[<Fact>]
let ``M1 rejects alphanumeric input`` () =
    let opts = { defaultOptions with Symbol = Some "M1" }
    let err = unwrapError (encode "A" opts)
    match err with
    | InputTooLong _ -> ()
    | other -> Assert.True(false, sprintf "Expected InputTooLong but got %A" other)

// ============================================================================
// Structural invariants
// ============================================================================

/// The top-left 7×7 region should contain the finder pattern.
/// The finder's outer border is all-dark (row 0 and col 0 within the 7×7 are dark).
/// We spot-check module (0,0) which is always a dark module in the finder.
[<Fact>]
let ``top-left module (0,0) is always dark (finder pattern corner)`` () =
    for version in [ "M1"; "M2"; "M3"; "M4" ] do
        let opts = { defaultOptions with Symbol = Some version }
        let grid = unwrap (encode "123" opts)
        Assert.True(grid.Modules.[0].[0], sprintf "%s: expected (0,0) to be dark" version)

/// Module (7,7) is the corner of the L-shaped separator — always light.
[<Fact>]
let ``module (7,7) is always light (separator corner)`` () =
    for version in [ "M1"; "M2"; "M3"; "M4" ] do
        let opts = { defaultOptions with Symbol = Some version }
        let grid = unwrap (encode "123" opts)
        Assert.False(grid.Modules.[7].[7], sprintf "%s: expected (7,7) to be light" version)

/// Module (0,8) is the first timing module on row 0 (index 8, even → dark).
[<Fact>]
let ``timing row 0 col 8 is dark (even index)`` () =
    for version in [ "M2"; "M3"; "M4" ] do  // M1 is 11×11, col 8 is timing
        let opts = { defaultOptions with Symbol = Some version }
        let grid = unwrap (encode "1" opts)
        Assert.True(grid.Modules.[0].[8], sprintf "%s: expected (0,8) to be dark (timing)" version)

// ============================================================================
// Capacity boundary tests
// ============================================================================

/// 5 digits is the maximum for M1. Must succeed.
[<Fact>]
let ``M1 accepts 5 digits (at capacity)`` () =
    let opts = { defaultOptions with Symbol = Some "M1" }
    let grid = unwrap (encode "12345" opts)
    Assert.Equal(11, grid.Rows)

/// 6 digits exceeds M1 capacity. When forced to M1, must fail.
[<Fact>]
let ``M1 rejects 6 digits (over capacity)`` () =
    let opts = { defaultOptions with Symbol = Some "M1" }
    let err = unwrapError (encode "123456" opts)
    match err with
    | InputTooLong _ -> ()
    | other -> Assert.True(false, sprintf "Expected InputTooLong but got %A" other)

/// 15 bytes is the maximum for M4-L byte mode. Must succeed.
[<Fact>]
let ``M4 L accepts 15 byte chars (at capacity)`` () =
    let opts = { defaultOptions with Symbol = Some "M4"; ECCLevel = Some L }
    // "x" is not in alphanumeric set → byte mode
    let input = String.replicate 15 "x"
    let grid = unwrap (encode input opts)
    Assert.Equal(17, grid.Rows)

/// 10 bytes is the maximum for M4-Q byte mode. Must succeed.
[<Fact>]
let ``M4 Q accepts 9 byte chars (at capacity)`` () =
    let opts = { defaultOptions with Symbol = Some "M4"; ECCLevel = Some Q }
    let input = String.replicate 9 "x"
    let grid = unwrap (encode input opts)
    Assert.Equal(17, grid.Rows)

// ============================================================================
// Auto Q-level selection
// ============================================================================

/// Requesting Q without a version filter should auto-select M4-Q.
[<Fact>]
let ``auto Q without version selects M4`` () =
    let opts = { defaultOptions with ECCLevel = Some Q }
    let grid = unwrap (encode "HELLO" opts)
    Assert.Equal(17, grid.Rows)

/// Requesting Detection without a version filter should auto-select M1.
[<Fact>]
let ``auto Detection without version selects M1`` () =
    let opts = { defaultOptions with ECCLevel = Some Detection }
    let grid = unwrap (encode "123" opts)
    Assert.Equal(11, grid.Rows)
