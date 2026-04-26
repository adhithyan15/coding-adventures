/// AztecCode.fs — ISO/IEC 24778:2008-compliant Aztec Code encoder for F#
///
/// This module encodes a UTF-8 string (or byte array) as an Aztec Code symbol
/// and returns a ``ModuleGrid`` (from ``CodingAdventures.Barcode2D``) — a plain
/// 2-D boolean grid where ``true`` means a dark module and ``false`` means a
/// light module.
///
/// ## What is Aztec Code?
///
/// Aztec Code was invented by Andrew Longacre Jr. at Welch Allyn in 1995 and
/// published as a royalty-free format.  Where QR Code uses three square
/// "finder patterns" at three corners, Aztec Code places a single
/// **bullseye finder pattern at the centre** of the symbol.  A scanner finds
/// the centre first, then reads outward in a clockwise spiral — and because
/// orientation is determined from the bullseye, no large quiet zone is needed.
///
/// ### Where Aztec Code is used today
///
///   - IATA boarding passes — the barcode on every airline boarding pass
///   - Eurostar and Amtrak rail tickets — printed and on-screen tickets
///   - PostNL, Deutsche Post, La Poste — European postal routing
///   - US military identification cards
///
/// ### Symbol variants
///
///   ```
///   Compact: 1–4 layers,  size = 11 + 4·layers   (15×15 to 27×27)
///   Full:    1–32 layers, size = 15 + 4·layers   (19×19 to 143×143)
///   ```
///
/// ## v0.1.0 encoding pipeline
///
/// ```
/// input string / bytes
///   → Binary-Shift codewords from Upper mode
///   → symbol size selection (smallest compact then full that fits at 23 % ECC)
///   → pad to exact codeword count
///   → GF(256)/0x12D Reed-Solomon ECC (poly 0x12D, b=1 roots α^1..α^n)
///   → bit stuffing (insert complement after 4 consecutive identical bits)
///   → GF(16) mode message (layers + codeword count + 5 or 6 RS nibbles)
///   → ModuleGrid  (bullseye → orientation marks → mode msg → data spiral)
/// ```
///
/// ### v0.1.0 simplifications
///
///   1. **Byte-mode only** — all input is wrapped in a single Binary-Shift
///      block from Upper mode.  Multi-mode optimisation
///      (Digit/Upper/Lower/Mixed/Punct) is reserved for v0.2.0.
///   2. **8-bit codewords → GF(256) RS** — uses the same primitive polynomial
///      as Data Matrix (0x12D, NOT QR Code's 0x11D).  GF(16) and GF(32) RS
///      for 4-bit and 5-bit codewords are reserved for v0.2.0.
///   3. **Default ECC = 23 %** — adjustable via ``AztecOptions.MinEccPercent``.
///   4. **Auto-select compact vs full** — the smallest fitting symbol is
///      chosen; force-compact is reserved for v0.2.0.

module CodingAdventures.AztecCode

open System
open System.Text
open CodingAdventures.Barcode2D

// ============================================================================
// Public types
// ============================================================================

/// Options controlling the encoder behaviour.
///
/// Currently only the minimum error-correction percentage is tunable.
type AztecOptions =
    {
        /// Minimum error-correction percentage (10–90).  Default 23.
        MinEccPercent: int
    }

/// Errors that can be returned by the encoder.
type AztecError =
    /// The input is too long to fit in any 32-layer full Aztec symbol at the
    /// chosen ECC level.
    | InputTooLong of string
    /// The supplied options are out of range (e.g. ECC < 10 or > 90).
    | InvalidOptions of string

    override e.ToString() =
        match e with
        | InputTooLong msg   -> sprintf "InputTooLong: %s" msg
        | InvalidOptions msg -> sprintf "InvalidOptions: %s" msg

/// Default encoder options — 23 % minimum ECC, which is the value recommended
/// by ISO/IEC 24778 for general-purpose use.
let defaultOptions : AztecOptions =
    { MinEccPercent = 23 }

// ============================================================================
// GF(16) arithmetic — for the mode message Reed-Solomon code
// ============================================================================
//
// GF(16) is the finite field with 16 elements, built from the primitive
// polynomial:
//
//   p(x) = x^4 + x + 1   (binary: 10011 = 0x13)
//
// Every non-zero element can be written as a power of the primitive element
// alpha.  alpha is the root of p(x), so alpha^4 = alpha + 1.
//
// The log table maps a field element (1..15) to its discrete log (0..14).
// The antilog table maps a log value to its element.
//
//   alpha^0=1   alpha^1=2   alpha^2=4   alpha^3=8
//   alpha^4=3   alpha^5=6   alpha^6=12  alpha^7=11
//   alpha^8=5   alpha^9=10  alpha^10=7  alpha^11=14
//   alpha^12=15 alpha^13=13 alpha^14=9  alpha^15=1   (period = 15)

/// GF(16) discrete logarithm: ``LOG16[e] = i`` means ``alpha^i = e``.
/// Index 0 (the additive zero) is undefined and stored as ``-1``.
let private LOG16 : int[] =
    [| -1;  0;  1;  4;  2;  8;  5; 10;  3; 14;  9;  7;  6; 13; 11; 12 |]

/// GF(16) antilogarithm: ``ALOG16[i] = alpha^i``.
let private ALOG16 : int[] =
    [| 1; 2; 4; 8; 3; 6; 12; 11; 5; 10; 7; 14; 15; 13; 9; 1 |]

/// Multiply two GF(16) elements.
///
/// Uses log/antilog: ``a * b = ALOG16[(LOG16[a] + LOG16[b]) mod 15]``.
/// Returns 0 if either operand is 0.
let private gf16Mul (a: int) (b: int) : int =
    if a = 0 || b = 0 then 0
    else ALOG16.[(LOG16.[a] + LOG16.[b]) % 15]

/// Build the GF(16) RS generator polynomial with roots ``alpha^1`` through
/// ``alpha^n``.  Returns ``[g_0; g_1; ...; g_n]`` where ``g_n = 1`` (monic).
let private buildGf16Generator (n: int) : int[] =
    let mutable g = [| 1 |]
    for i in 1 .. n do
        let ai = ALOG16.[i % 15]
        let next = Array.zeroCreate (g.Length + 1)
        for j in 0 .. g.Length - 1 do
            next.[j + 1] <- next.[j + 1] ^^^ g.[j]
            next.[j]     <- next.[j]     ^^^ gf16Mul ai g.[j]
        g <- next
    g

/// Compute ``n`` GF(16) RS check nibbles for the given data nibbles using the
/// LFSR polynomial-division algorithm.
let private gf16RsEncode (data: int[]) (n: int) : int[] =
    let g = buildGf16Generator n
    let rem = Array.zeroCreate n
    for nibble in data do
        let fb = nibble ^^^ rem.[0]
        for i in 0 .. n - 2 do
            rem.[i] <- rem.[i + 1] ^^^ gf16Mul g.[i + 1] fb
        rem.[n - 1] <- gf16Mul g.[n] fb
    rem

// ============================================================================
// GF(256)/0x12D arithmetic — for 8-bit data codewords
// ============================================================================
//
// Aztec Code uses GF(256) with primitive polynomial:
//
//   p(x) = x^8 + x^5 + x^4 + x^2 + x + 1   =   0x12D
//
// This is the SAME polynomial as Data Matrix ECC200, but DIFFERENT from
// QR Code (0x11D).  We implement it inline since the repo's shared
// CodingAdventures.Gf256 package uses 0x11D.
//
// Generator convention: b=1, roots alpha^1..alpha^n (MA02 style — the same
// convention used by Data Matrix, Aztec Code, MaxiCode and PDF417).

[<Literal>]
let private GF256_POLY = 0x12d

/// ``EXP_12D[i] = alpha^i`` in GF(256)/0x12D, doubled for fast multiply
/// (so we can index by ``log(a) + log(b)`` without a modulus).
let private EXP_12D : int[] = Array.zeroCreate 512

/// ``LOG_12D[e]`` = discrete log of ``e`` in GF(256)/0x12D.
let private LOG_12D : int[] = Array.zeroCreate 256

// Build the tables once at module load.  The primitive element is alpha = 2.
do
    let mutable x = 1
    for i in 0 .. 254 do
        EXP_12D.[i]       <- x
        EXP_12D.[i + 255] <- x
        LOG_12D.[x]       <- i
        x <- x <<< 1
        if (x &&& 0x100) <> 0 then x <- x ^^^ GF256_POLY
        x <- x &&& 0xff
    EXP_12D.[255] <- 1

/// Multiply two GF(256)/0x12D elements via log/antilog lookup.
let private gf256Mul (a: int) (b: int) : int =
    if a = 0 || b = 0 then 0
    else EXP_12D.[LOG_12D.[a] + LOG_12D.[b]]

/// Build the GF(256)/0x12D RS generator polynomial with roots
/// ``alpha^1`` .. ``alpha^n``.  Big-endian coefficients (highest degree first).
let private buildGf256Generator (n: int) : int[] =
    let mutable g = [| 1 |]
    for i in 1 .. n do
        let ai = EXP_12D.[i]
        let next = Array.zeroCreate (g.Length + 1)
        for j in 0 .. g.Length - 1 do
            next.[j]     <- next.[j]     ^^^ g.[j]
            next.[j + 1] <- next.[j + 1] ^^^ gf256Mul g.[j] ai
        g <- next
    g

/// Compute ``nCheck`` GF(256)/0x12D RS check bytes for the given data bytes.
let private gf256RsEncode (data: int[]) (nCheck: int) : int[] =
    let g = buildGf256Generator nCheck
    let n = g.Length - 1
    let rem = Array.zeroCreate n
    for b in data do
        let fb = b ^^^ rem.[0]
        for i in 0 .. n - 2 do
            rem.[i] <- rem.[i + 1] ^^^ gf256Mul g.[i + 1] fb
        rem.[n - 1] <- gf256Mul g.[n] fb
    rem

// ============================================================================
// Aztec Code capacity tables (ISO/IEC 24778:2008 Table 1)
// ============================================================================

/// Compact-symbol capacity by layer count.  Index 0 is unused padding.
/// Each pair is ``(totalBits, maxBytes8)`` where ``totalBits`` is the total
/// data+ECC bit positions and ``maxBytes8`` is the number of 8-bit codeword
/// slots available.
let private COMPACT_CAPACITY : (int * int)[] =
    [|
        (0,   0)    // padding — index 0 unused
        (72,  9)    // 1 layer  — 15×15
        (200, 25)   // 2 layers — 19×19
        (392, 49)   // 3 layers — 23×23
        (648, 81)   // 4 layers — 27×27
    |]

/// Full-symbol capacity by layer count.  Index 0 is unused padding.
let private FULL_CAPACITY : (int * int)[] =
    [|
        (0,     0)      // padding — index 0 unused
        (88,    11)     //  1 layer
        (216,   27)     //  2 layers
        (360,   45)     //  3 layers
        (520,   65)     //  4 layers
        (696,   87)     //  5 layers
        (888,   111)    //  6 layers
        (1096,  137)    //  7 layers
        (1320,  165)    //  8 layers
        (1560,  195)    //  9 layers
        (1816,  227)    // 10 layers
        (2088,  261)    // 11 layers
        (2376,  297)    // 12 layers
        (2680,  335)    // 13 layers
        (3000,  375)    // 14 layers
        (3336,  417)    // 15 layers
        (3688,  461)    // 16 layers
        (4056,  507)    // 17 layers
        (4440,  555)    // 18 layers
        (4840,  605)    // 19 layers
        (5256,  657)    // 20 layers
        (5688,  711)    // 21 layers
        (6136,  767)    // 22 layers
        (6600,  825)    // 23 layers
        (7080,  885)    // 24 layers
        (7576,  947)    // 25 layers
        (8088,  1011)   // 26 layers
        (8616,  1077)   // 27 layers
        (9160,  1145)   // 28 layers
        (9720,  1215)   // 29 layers
        (10296, 1287)   // 30 layers
        (10888, 1361)   // 31 layers
        (11496, 1437)   // 32 layers
    |]

// ============================================================================
// Data encoding — Binary-Shift from Upper mode (v0.1.0 byte-mode path)
// ============================================================================
//
// All input bytes are wrapped in a single Binary-Shift block from Upper mode:
//
//   1. Emit 5 bits = 0b11111 (Binary-Shift escape in Upper mode)
//   2. If len ≤ 31: 5 bits for length
//      If len > 31: 5 bits = 0b00000 then 11 bits for length
//   3. Each byte as 8 bits, MSB first.

/// Encode input bytes as a flat bit array using the Binary-Shift escape.
/// Returns an array of 0/1 values, MSB first.
let private encodeBytesAsBits (input: byte[]) : int[] =
    let bits = ResizeArray<int>()

    let writeBits (value: int) (count: int) =
        for i in count - 1 .. -1 .. 0 do
            bits.Add((value >>> i) &&& 1)

    let len = input.Length
    writeBits 31 5   // Binary-Shift escape

    if len <= 31 then
        writeBits len 5
    else
        writeBits 0 5
        writeBits len 11

    for b in input do
        writeBits (int b) 8

    bits.ToArray()

// ============================================================================
// Symbol size selection
// ============================================================================

/// A chosen symbol specification — the result of ``selectSymbol``.
type private SymbolSpec =
    {
        /// True for compact (1–4 layers); false for full (1–32 layers).
        Compact: bool
        /// Layer count.
        Layers: int
        /// Number of 8-bit data codewords.
        DataCwCount: int
        /// Number of 8-bit ECC codewords.
        EccCwCount: int
        /// Total data + ECC bit positions in the symbol.
        TotalBits: int
    }

/// Select the smallest symbol that can hold ``dataBitCount`` bits at the
/// requested minimum ECC percentage.  Tries compact 1–4, then full 1–32.
///
/// We pad the bit count by 20 % to give bit-stuffing some breathing room
/// before we commit to a size.  This is conservative — real-world stuffing
/// overhead is closer to a few percent for typical inputs.
///
/// Returns ``Error InputTooLong`` if no 32-layer full symbol fits.
let private selectSymbol (dataBitCount: int) (minEccPct: int) : Result<SymbolSpec, AztecError> =
    let stuffedBitCount = (dataBitCount * 12 + 9) / 10   // ceil(dataBitCount * 1.2)

    let tryFitFromTable (table: (int * int)[]) (compact: bool) (maxLayers: int) =
        let mutable found = None
        let mutable layers = 1
        while found.IsNone && layers <= maxLayers do
            let (totalBits, totalBytes) = table.[layers]
            let eccCwCount  = (minEccPct * totalBytes + 99) / 100   // ceil
            let dataCwCount = totalBytes - eccCwCount
            if dataCwCount > 0 then
                let neededBytes = (stuffedBitCount + 7) / 8
                if neededBytes <= dataCwCount then
                    found <- Some {
                        Compact     = compact
                        Layers      = layers
                        DataCwCount = dataCwCount
                        EccCwCount  = eccCwCount
                        TotalBits   = totalBits
                    }
            layers <- layers + 1
        found

    match tryFitFromTable COMPACT_CAPACITY true 4 with
    | Some spec -> Ok spec
    | None ->
        match tryFitFromTable FULL_CAPACITY false 32 with
        | Some spec -> Ok spec
        | None ->
            Error (InputTooLong (sprintf "Input is too long to fit in any Aztec Code symbol (%d bits needed)" dataBitCount))

// ============================================================================
// Padding
// ============================================================================

/// Pad a bit array up to ``targetBytes * 8`` bits with trailing zero bits.
/// Truncates if longer (caller is responsible for sizing — this is just a
/// safety net).
let private padToBytes (bits: int[]) (targetBytes: int) : int[] =
    let target = targetBytes * 8
    if bits.Length >= target then
        Array.sub bits 0 target
    else
        let out = Array.zeroCreate target
        Array.blit bits 0 out 0 bits.Length
        out

// ============================================================================
// Bit stuffing
// ============================================================================
//
// After every 4 consecutive identical bits (all 0 or all 1), insert one
// complement bit.  Applied only to the data+ECC bit stream (not to the mode
// message).
//
// Example:
//
//   Input:  1 1 1 1 0 0 0 0
//   After 4 ones: insert 0  → [1,1,1,1,0]
//   After 4 zeros: insert 1 → [1,1,1,1,0, 0,0,0,1,0]

/// Apply Aztec bit stuffing.  Inserts a complement bit after every run of 4
/// identical bits.
let private stuffBits (bits: int[]) : int[] =
    let stuffed = ResizeArray<int>()
    let mutable runVal = -1
    let mutable runLen = 0

    for bit in bits do
        if bit = runVal then
            runLen <- runLen + 1
        else
            runVal <- bit
            runLen <- 1

        stuffed.Add(bit)

        if runLen = 4 then
            let stuffBit = 1 - bit
            stuffed.Add(stuffBit)
            runVal <- stuffBit
            runLen <- 1

    stuffed.ToArray()

// ============================================================================
// Mode message encoding
// ============================================================================
//
// The mode message records (layers, dataCwCount), protected by a small
// GF(16) Reed-Solomon code.
//
// Compact (28 bits = 7 nibbles):
//   m = ((layers-1) << 6) | (dataCwCount-1)
//   2 data nibbles + 5 ECC nibbles
//
// Full (40 bits = 10 nibbles):
//   m = ((layers-1) << 11) | (dataCwCount-1)
//   4 data nibbles + 6 ECC nibbles

/// Encode the mode message as a flat bit array.
/// Returns 28 bits for compact symbols, 40 bits for full.
let private encodeModeMessage (compact: bool) (layers: int) (dataCwCount: int) : int[] =
    let dataNibbles, numEcc =
        if compact then
            let m = ((layers - 1) <<< 6) ||| (dataCwCount - 1)
            [| m &&& 0xf; (m >>> 4) &&& 0xf |], 5
        else
            let m = ((layers - 1) <<< 11) ||| (dataCwCount - 1)
            [| m &&& 0xf; (m >>> 4) &&& 0xf; (m >>> 8) &&& 0xf; (m >>> 12) &&& 0xf |], 6

    let eccNibbles = gf16RsEncode dataNibbles numEcc
    let allNibbles = Array.append dataNibbles eccNibbles

    let bits = ResizeArray<int>()
    for nibble in allNibbles do
        for i in 3 .. -1 .. 0 do
            bits.Add((nibble >>> i) &&& 1)
    bits.ToArray()

// ============================================================================
// Grid construction helpers
// ============================================================================

/// Symbol side length: compact = ``11 + 4·layers``, full = ``15 + 4·layers``.
let private symbolSize (compact: bool) (layers: int) =
    if compact then 11 + 4 * layers else 15 + 4 * layers

/// Bullseye Chebyshev radius: compact = 5, full = 7.
let private bullseyeRadius (compact: bool) =
    if compact then 5 else 7

/// Draw the bullseye finder pattern centred at ``(cx, cy)``.
///
/// Colour at Chebyshev distance ``d`` from centre:
///   - ``d ≤ 1``     → DARK  (solid 3×3 inner core)
///   - ``d > 1, even`` → LIGHT
///   - ``d > 1, odd``  → DARK
let private drawBullseye
        (modules: bool[,]) (reserved: bool[,])
        (cx: int) (cy: int) (compact: bool) =
    let br = bullseyeRadius compact
    for row in cy - br .. cy + br do
        for col in cx - br .. cx + br do
            let d = max (abs (col - cx)) (abs (row - cy))
            let dark = if d <= 1 then true else (d % 2 = 1)
            modules.[row, col]  <- dark
            reserved.[row, col] <- true

/// Draw the reference grid for FULL Aztec symbols.
///
/// Reference grid lines run at rows/cols where ``(cy - row) % 16 = 0`` or
/// ``(cx - col) % 16 = 0``.  Modules along those lines alternate dark/light
/// depending on their position relative to the centre.
///
/// Compact symbols don't have a reference grid — only full ones do.
let private drawReferenceGrid
        (modules: bool[,]) (reserved: bool[,])
        (cx: int) (cy: int) (size: int) =
    for row in 0 .. size - 1 do
        for col in 0 .. size - 1 do
            let onH = (cy - row) % 16 = 0
            let onV = (cx - col) % 16 = 0
            if onH || onV then
                let dark =
                    if onH && onV then true
                    elif onH then (cx - col) % 2 = 0
                    else          (cy - row) % 2 = 0
                modules.[row, col]  <- dark
                reserved.[row, col] <- true

/// Place orientation marks (4 dark corners) and mode message bits on the ring
/// at Chebyshev radius ``bullseyeRadius + 1``.
///
/// Returns the remaining non-corner positions in clockwise order, which the
/// caller can use to absorb any "leftover" data bits (this is occasionally
/// useful when the ring is larger than the mode message).
let private drawOrientationAndModeMessage
        (modules: bool[,]) (reserved: bool[,])
        (cx: int) (cy: int) (compact: bool)
        (modeMessageBits: int[]) : (int * int) list =

    let r = bullseyeRadius compact + 1
    let nonCorner = ResizeArray<int * int>()

    // Top edge — left to right, skipping both corners
    for col in cx - r + 1 .. cx + r - 1 do
        nonCorner.Add(col, cy - r)
    // Right edge — top to bottom, skipping both corners
    for row in cy - r + 1 .. cy + r - 1 do
        nonCorner.Add(cx + r, row)
    // Bottom edge — right to left, skipping both corners
    for col in cx + r - 1 .. -1 .. cx - r + 1 do
        nonCorner.Add(col, cy + r)
    // Left edge — bottom to top, skipping both corners
    for row in cy + r - 1 .. -1 .. cy - r + 1 do
        nonCorner.Add(cx - r, row)

    // Place 4 orientation mark corners as DARK.
    let corners = [| (cx - r, cy - r); (cx + r, cy - r); (cx + r, cy + r); (cx - r, cy + r) |]
    for (col, row) in corners do
        modules.[row, col]  <- true
        reserved.[row, col] <- true

    // Place mode message bits in clockwise order from TL+1.
    let n = min modeMessageBits.Length nonCorner.Count
    for i in 0 .. n - 1 do
        let (col, row) = nonCorner.[i]
        modules.[row, col]  <- modeMessageBits.[i] = 1
        reserved.[row, col] <- true

    // Return leftover positions (rare — usually 0) so the data placer can
    // fill them first before spiralling outwards.
    [ for i in modeMessageBits.Length .. nonCorner.Count - 1 -> nonCorner.[i] ]

// ============================================================================
// Data layer spiral placement
// ============================================================================
//
// Bits are placed in a clockwise spiral starting from the innermost data
// layer.  Each layer "band" is 2 modules wide.  For each band we place pairs
// of modules — outer first, then inner — sweeping along all four sides
// clockwise.
//
// The first data layer's inner radius is ``bullseyeRadius + 2``:
//   - compact: br = 5  → first inner = 7
//   - full:    br = 7  → first inner = 9

/// Place all data bits using the clockwise layer spiral, skipping any
/// reserved modules (function patterns, mode message ring, reference grid).
let private placeDataBits
        (modules: bool[,]) (reserved: bool[,])
        (bits: int[])
        (cx: int) (cy: int) (compact: bool) (layers: int)
        (modeRingRemainingPositions: (int * int) list) =

    let size = Array2D.length1 modules
    let mutable bitIndex = 0

    let placeBit (col: int) (row: int) =
        if row >= 0 && row < size && col >= 0 && col < size then
            if not reserved.[row, col] then
                if bitIndex < bits.Length then
                    modules.[row, col] <- bits.[bitIndex] = 1
                bitIndex <- bitIndex + 1

    // Step 1: fill leftover mode-ring positions first (these are unreserved).
    for (col, row) in modeRingRemainingPositions do
        if bitIndex < bits.Length then
            modules.[row, col] <- bits.[bitIndex] = 1
        bitIndex <- bitIndex + 1

    // Step 2: spiral through the data layers.
    let br = bullseyeRadius compact
    let dStart = br + 2   // mode msg ring at br+1, first data layer at br+2

    for L in 0 .. layers - 1 do
        let dI = dStart + 2 * L   // inner radius
        let dO = dI + 1            // outer radius

        // Top edge — left to right
        for col in cx - dI + 1 .. cx + dI do
            placeBit col (cy - dO)
            placeBit col (cy - dI)
        // Right edge — top to bottom
        for row in cy - dI + 1 .. cy + dI do
            placeBit (cx + dO) row
            placeBit (cx + dI) row
        // Bottom edge — right to left
        for col in cx + dI .. -1 .. cx - dI + 1 do
            placeBit col (cy + dO)
            placeBit col (cy + dI)
        // Left edge — bottom to top
        for row in cy + dI .. -1 .. cy - dI + 1 do
            placeBit (cx - dO) row
            placeBit (cx - dI) row

// ============================================================================
// Conversion helpers — Array2D ↔ jagged ``bool[][]``
// ============================================================================

/// Convert an ``Array2D`` to the jagged ``bool[][]`` shape expected by
/// ``ModuleGrid.Modules``.
let private toJaggedArray (modules: bool[,]) : bool[][] =
    let rows = Array2D.length1 modules
    let cols = Array2D.length2 modules
    [| for r in 0 .. rows - 1 ->
        [| for c in 0 .. cols - 1 -> modules.[r, c] |] |]

// ============================================================================
// Public API
// ============================================================================

/// Package version, used by dependency smoke tests.
[<Literal>]
let VERSION = "0.1.0"

/// Encode a UTF-8 string as an Aztec Code symbol.
///
/// Returns a ``ModuleGrid`` where ``modules.[row].[col] = true`` means a dark
/// module.  The grid origin ``(0, 0)`` is the top-left corner.
///
/// Steps:
///   1. Encode input via Binary-Shift from Upper mode.
///   2. Select the smallest symbol at the requested ECC level.
///   3. Pad the data codeword sequence.
///   4. Compute GF(256)/0x12D RS ECC.
///   5. Apply bit stuffing.
///   6. Compute the GF(16) mode message.
///   7. Initialise the grid with structural patterns.
///   8. Place data + ECC bits in the clockwise layer spiral.
///
/// ## Errors
///
/// Returns ``InputTooLong`` if the data exceeds the maximum 32-layer full
/// symbol capacity, or ``InvalidOptions`` if ``MinEccPercent`` is out of
/// range.
///
/// ## Example
///
///     match AztecCode.encodeWith "HELLO" defaultOptions with
///     | Ok grid -> printfn "Symbol size: %d × %d" grid.Rows grid.Cols
///     | Error e -> printfn "Encoding failed: %A" e
/// Hard cap on input byte length.  The largest 32-layer full Aztec symbol
/// holds 1437 8-bit codewords (~11 KB after ECC and stuffing overhead), so
/// any input above ~3 KB is guaranteed to overflow.  We pick 4 KB as a safe
/// rejection threshold — big enough to keep callers honest, small enough to
/// prevent attackers from forcing the encoder to allocate hundreds of MB of
/// bit-array memory before failing inside ``selectSymbol``.
[<Literal>]
let private MAX_INPUT_BYTES = 4096

let encodeBytesWith (input: byte[]) (options: AztecOptions) : Result<ModuleGrid, AztecError> =
    if options.MinEccPercent < 10 || options.MinEccPercent > 90 then
        Error (InvalidOptions (sprintf "MinEccPercent must be between 10 and 90, got %d" options.MinEccPercent))
    elif input.Length > MAX_INPUT_BYTES then
        Error (InputTooLong (sprintf "Input is %d bytes; maximum supported is %d." input.Length MAX_INPUT_BYTES))
    else

    // Step 1: encode data
    let dataBits = encodeBytesAsBits input

    // Step 2: select symbol
    match selectSymbol dataBits.Length options.MinEccPercent with
    | Error e -> Error e
    | Ok spec ->

    let { Compact = compact; Layers = layers; DataCwCount = dataCwCount; EccCwCount = eccCwCount } = spec

    // Step 3: pad to dataCwCount bytes
    let paddedBits = padToBytes dataBits dataCwCount

    let dataBytes = Array.zeroCreate dataCwCount
    for i in 0 .. dataCwCount - 1 do
        let mutable byte = 0
        for b in 0 .. 7 do
            byte <- (byte <<< 1) ||| paddedBits.[i * 8 + b]
        // All-zero codeword avoidance: if the very last data codeword is 0x00,
        // promote it to 0xFF so the symbol never carries an all-zero codeword
        // (this matches the reference implementation's safety rule).
        let final = if byte = 0 && i = dataCwCount - 1 then 0xff else byte
        dataBytes.[i] <- final

    // Step 4: compute RS ECC
    let eccBytes = gf256RsEncode dataBytes eccCwCount

    // Step 5: build the bit stream and stuff
    let allBytes = Array.append dataBytes eccBytes
    let rawBits = Array.zeroCreate (allBytes.Length * 8)
    for i in 0 .. allBytes.Length - 1 do
        let byte = allBytes.[i]
        for b in 0 .. 7 do
            rawBits.[i * 8 + b] <- (byte >>> (7 - b)) &&& 1
    let stuffedBits = stuffBits rawBits

    // Step 6: mode message
    let modeMsg = encodeModeMessage compact layers dataCwCount

    // Step 7: initialise the grid
    let size = symbolSize compact layers
    let cx = size / 2
    let cy = size / 2

    let modules  = Array2D.create size size false
    let reserved = Array2D.create size size false

    // Reference grid first (full only), then bullseye overwrites the centre.
    if not compact then
        drawReferenceGrid modules reserved cx cy size
    drawBullseye modules reserved cx cy compact

    let modeRingRemaining =
        drawOrientationAndModeMessage modules reserved cx cy compact modeMsg

    // Step 8: place data spiral
    placeDataBits modules reserved stuffedBits cx cy compact layers modeRingRemaining

    Ok {
        Rows        = size
        Cols        = size
        Modules     = toJaggedArray modules
        ModuleShape = ModuleShape.Square
    }

/// Encode a UTF-8 string as an Aztec Code symbol using the supplied options.
let encodeWith (input: string) (options: AztecOptions) : Result<ModuleGrid, AztecError> =
    let bytes = Encoding.UTF8.GetBytes(input)
    encodeBytesWith bytes options

/// Encode a UTF-8 string as an Aztec Code symbol with default options
/// (23 % minimum ECC).
let encode (input: string) : Result<ModuleGrid, AztecError> =
    encodeWith input defaultOptions

/// Encode a raw byte array as an Aztec Code symbol with default options.
let encodeBytes (input: byte[]) : Result<ModuleGrid, AztecError> =
    encodeBytesWith input defaultOptions
