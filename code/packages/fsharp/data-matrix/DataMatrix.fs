/// DataMatrix.fs — ISO/IEC 16022:2006 Data Matrix ECC200 encoder for F#
///
/// This module encodes a UTF-8 string (or byte array) as a Data Matrix ECC200
/// symbol and returns a ``ModuleGrid`` (from ``CodingAdventures.Barcode2D``) —
/// a plain 2-D boolean grid where ``true`` means a dark module and ``false``
/// means a light module.
///
/// ## What is Data Matrix?
///
/// Data Matrix was invented by RVSI Acuity CiMatrix (formerly Siemens) in 1989
/// under the name "DataCode." The ECC200 variant — introduced in the mid-1990s
/// and standardised as ISO/IEC 16022:2006 — replaced the older ECC000–ECC140
/// lineage with Reed-Solomon error correction over GF(256).
///
/// Data Matrix is used wherever small, high-density, damage-tolerant marks are
/// needed on physical objects:
///
///   - PCB traceability — every board carries a Data Matrix etched into the
///     substrate for tracking through automated assembly lines.
///   - Pharmaceuticals — the US FDA DSCSA mandate requires Data Matrix on
///     unit-dose packaging.
///   - Aerospace — dot-peen or laser-etched marks on aircraft parts (rivets,
///     shims, brackets) that survive decades of abrasion, heat, and cleaning.
///   - US Postal Service — registered mail and customs forms.
///   - Medical devices — GS1 DataMatrix on surgical instruments and implants.
///
/// ## Key design differences from QR Code
///
///   - **No masking** — the diagonal "Utah" placement distributes bits well
///     enough that no XOR masking step is needed.
///   - **L-shaped finder + clock border** — solid-dark L on the left and bottom;
///     alternating clock on the top and right. One border; no three separate
///     corner finder squares.
///   - **GF(256)/0x12D** — the primitive polynomial is 0x12D, not QR's 0x11D.
///   - **b=1 Reed-Solomon convention** — roots are α^1 .. α^n (MA02 style),
///     the same as the CodingAdventures.ReedSolomon package.
///
/// ## Encoding pipeline
///
/// ```
/// input string
///   → ASCII encoding     (chars+1; digit pairs packed into one codeword)
///   → symbol selection   (smallest ECC200 symbol whose capacity ≥ cw count)
///   → pad to capacity    (scrambled-pad codewords fill unused slots)
///   → RS blocks + ECC    (GF(256)/0x12D, b=1, per-block LFSR division)
///   → interleave blocks  (data round-robin then ECC round-robin)
///   → grid init          (L-finder + timing border + alignment borders)
///   → Utah placement     (diagonal codeword placement, no masking)
///   → ModuleGrid         (true = dark module)
/// ```

module CodingAdventures.DataMatrix

open System
open System.Text
open CodingAdventures.Barcode2D

// ============================================================================
// Public version constant
// ============================================================================

/// Package version string.
[<Literal>]
let VERSION = "0.1.0"

// ============================================================================
// Public types
// ============================================================================

/// Discriminated union of all errors the encoder can return.
type DataMatrixError =
    /// The input is too long to fit in any ECC200 symbol (max 144×144).
    | InputTooLong of string

    override e.ToString() =
        match e with
        | InputTooLong msg -> sprintf "InputTooLong: %s" msg

// ============================================================================
// GF(256) / 0x12D arithmetic
// ============================================================================
//
// Data Matrix uses GF(256) with the primitive polynomial:
//
//   p(x) = x^8 + x^5 + x^4 + x^2 + x + 1   =   0x12D   (decimal 301)
//
// This is DIFFERENT from QR Code's 0x11D polynomial.  Both are degree-8
// irreducible polynomials over GF(2), but the fields they define are
// non-isomorphic (they have different multiplication tables).
//
// We pre-compute an exp table and a log table for GF(256)/0x12D:
//
//   EXP_12D.(i) = α^i   (antilog / exponentiation)
//   LOG_12D.(v) = k   where α^k = v   (LOG_12D.(0) is undefined; we store 0)
//
// These tables let us implement multiply in O(1):
//   a × b = EXP_12D.( (LOG_12D.(a) + LOG_12D.(b)) mod 255 )
//
// The tables are built once at module load — the primitive element is α = 2.

/// GF(256)/0x12D primitive polynomial.
[<Literal>]
let private GF256_POLY = 0x12D

/// Antilog table: EXP_12D.(i) = α^i in GF(256)/0x12D.
/// Length 512 so we can index by log(a)+log(b) without a modulus operation.
let private EXP_12D : int[] = Array.zeroCreate 512

/// Log table: LOG_12D.(v) = k where α^k = v.
/// LOG_12D.(0) is undefined; we store 0 there.
let private LOG_12D : int[] = Array.zeroCreate 256

// Build the tables at module-load time.
do
    let mutable x = 1
    for i in 0 .. 254 do
        EXP_12D.[i]       <- x
        EXP_12D.[i + 255] <- x   // double-length avoids modular reduction on lookup
        LOG_12D.[x]       <- i
        x <- x <<< 1
        if (x &&& 0x100) <> 0 then
            x <- x ^^^ GF256_POLY
        x <- x &&& 0xFF
    EXP_12D.[255] <- 1   // α^255 = 1 (the field has multiplicative order 255)

/// Multiply two GF(256)/0x12D elements via log/antilog lookup.
///
/// The identity a × 0 = 0 is handled explicitly — LOG_12D.(0) is not a valid
/// index.  For non-zero a, b:
///
///   a × b = α^{(log(a) + log(b)) mod 255}
///         = EXP_12D.(LOG_12D.(a) + LOG_12D.(b))   (using the doubled table)
let private gf256Mul (a: int) (b: int) : int =
    if a = 0 || b = 0 then 0
    else EXP_12D.[LOG_12D.[a] + LOG_12D.[b]]

// ============================================================================
// Symbol size table
// ============================================================================
//
// Source: ISO/IEC 16022:2006, Table 7.
//
// Each entry describes one ECC200 symbol size.  A "data region" is a
// rectangular sub-area of the symbol interior (between the outer border and
// any inter-region alignment borders).  Small symbols (≤ 26×26) have a single
// data region; larger symbols subdivide into a grid of regions.
//
// Field glossary:
//
//   SymbolRows / SymbolCols — total symbol dimensions, including outer border.
//   RegionRows / RegionCols — number of data regions in the row / col direction.
//   DataRegionHeight / DataRegionWidth — interior size of each data region.
//   DataCW  — total data codeword capacity.
//   EccCW   — total ECC codewords.
//   NumBlocks   — number of interleaved RS blocks.
//   EccPerBlock — ECC codewords per RS block.
//
// The logical data matrix dimensions used by the Utah placement algorithm are:
//
//   logicalRows = RegionRows * DataRegionHeight
//   logicalCols = RegionCols * DataRegionWidth

/// Descriptor for one Data Matrix ECC200 symbol size.
type internal SymbolSizeEntry =
    {
        SymbolRows: int
        SymbolCols: int
        RegionRows: int
        RegionCols: int
        DataRegionHeight: int
        DataRegionWidth: int
        DataCW: int
        EccCW: int
        NumBlocks: int
        EccPerBlock: int
    }

/// All square symbol sizes for Data Matrix ECC200 (ISO/IEC 16022:2006 Table 7).
let private SQUARE_SIZES : SymbolSizeEntry[] =
    [|
        // symbolRows, symbolCols, rr, rc, drH, drW, dataCW, eccCW, blocks, eccPerBlock
        { SymbolRows=  10; SymbolCols=  10; RegionRows=1; RegionCols=1; DataRegionHeight=  8; DataRegionWidth=  8; DataCW=   3; EccCW=   5; NumBlocks=1; EccPerBlock= 5 }
        { SymbolRows=  12; SymbolCols=  12; RegionRows=1; RegionCols=1; DataRegionHeight= 10; DataRegionWidth= 10; DataCW=   5; EccCW=   7; NumBlocks=1; EccPerBlock= 7 }
        { SymbolRows=  14; SymbolCols=  14; RegionRows=1; RegionCols=1; DataRegionHeight= 12; DataRegionWidth= 12; DataCW=   8; EccCW=  10; NumBlocks=1; EccPerBlock=10 }
        { SymbolRows=  16; SymbolCols=  16; RegionRows=1; RegionCols=1; DataRegionHeight= 14; DataRegionWidth= 14; DataCW=  12; EccCW=  12; NumBlocks=1; EccPerBlock=12 }
        { SymbolRows=  18; SymbolCols=  18; RegionRows=1; RegionCols=1; DataRegionHeight= 16; DataRegionWidth= 16; DataCW=  18; EccCW=  14; NumBlocks=1; EccPerBlock=14 }
        { SymbolRows=  20; SymbolCols=  20; RegionRows=1; RegionCols=1; DataRegionHeight= 18; DataRegionWidth= 18; DataCW=  22; EccCW=  18; NumBlocks=1; EccPerBlock=18 }
        { SymbolRows=  22; SymbolCols=  22; RegionRows=1; RegionCols=1; DataRegionHeight= 20; DataRegionWidth= 20; DataCW=  30; EccCW=  20; NumBlocks=1; EccPerBlock=20 }
        { SymbolRows=  24; SymbolCols=  24; RegionRows=1; RegionCols=1; DataRegionHeight= 22; DataRegionWidth= 22; DataCW=  36; EccCW=  24; NumBlocks=1; EccPerBlock=24 }
        { SymbolRows=  26; SymbolCols=  26; RegionRows=1; RegionCols=1; DataRegionHeight= 24; DataRegionWidth= 24; DataCW=  44; EccCW=  28; NumBlocks=1; EccPerBlock=28 }
        { SymbolRows=  32; SymbolCols=  32; RegionRows=2; RegionCols=2; DataRegionHeight= 14; DataRegionWidth= 14; DataCW=  62; EccCW=  36; NumBlocks=2; EccPerBlock=18 }
        { SymbolRows=  36; SymbolCols=  36; RegionRows=2; RegionCols=2; DataRegionHeight= 16; DataRegionWidth= 16; DataCW=  86; EccCW=  42; NumBlocks=2; EccPerBlock=21 }
        { SymbolRows=  40; SymbolCols=  40; RegionRows=2; RegionCols=2; DataRegionHeight= 18; DataRegionWidth= 18; DataCW= 114; EccCW=  48; NumBlocks=2; EccPerBlock=24 }
        { SymbolRows=  44; SymbolCols=  44; RegionRows=2; RegionCols=2; DataRegionHeight= 20; DataRegionWidth= 20; DataCW= 144; EccCW=  56; NumBlocks=4; EccPerBlock=14 }
        { SymbolRows=  48; SymbolCols=  48; RegionRows=2; RegionCols=2; DataRegionHeight= 22; DataRegionWidth= 22; DataCW= 174; EccCW=  68; NumBlocks=4; EccPerBlock=17 }
        { SymbolRows=  52; SymbolCols=  52; RegionRows=2; RegionCols=2; DataRegionHeight= 24; DataRegionWidth= 24; DataCW= 204; EccCW=  84; NumBlocks=4; EccPerBlock=21 }
        { SymbolRows=  64; SymbolCols=  64; RegionRows=4; RegionCols=4; DataRegionHeight= 14; DataRegionWidth= 14; DataCW= 280; EccCW= 112; NumBlocks=4; EccPerBlock=28 }
        { SymbolRows=  72; SymbolCols=  72; RegionRows=4; RegionCols=4; DataRegionHeight= 16; DataRegionWidth= 16; DataCW= 368; EccCW= 144; NumBlocks=4; EccPerBlock=36 }
        { SymbolRows=  80; SymbolCols=  80; RegionRows=4; RegionCols=4; DataRegionHeight= 18; DataRegionWidth= 18; DataCW= 456; EccCW= 192; NumBlocks=4; EccPerBlock=48 }
        { SymbolRows=  88; SymbolCols=  88; RegionRows=4; RegionCols=4; DataRegionHeight= 20; DataRegionWidth= 20; DataCW= 576; EccCW= 224; NumBlocks=4; EccPerBlock=56 }
        { SymbolRows=  96; SymbolCols=  96; RegionRows=4; RegionCols=4; DataRegionHeight= 22; DataRegionWidth= 22; DataCW= 696; EccCW= 272; NumBlocks=4; EccPerBlock=68 }
        { SymbolRows= 104; SymbolCols= 104; RegionRows=4; RegionCols=4; DataRegionHeight= 24; DataRegionWidth= 24; DataCW= 816; EccCW= 336; NumBlocks=6; EccPerBlock=56 }
        { SymbolRows= 120; SymbolCols= 120; RegionRows=6; RegionCols=6; DataRegionHeight= 18; DataRegionWidth= 18; DataCW=1050; EccCW= 408; NumBlocks=6; EccPerBlock=68 }
        { SymbolRows= 132; SymbolCols= 132; RegionRows=6; RegionCols=6; DataRegionHeight= 20; DataRegionWidth= 20; DataCW=1304; EccCW= 496; NumBlocks=8; EccPerBlock=62 }
        { SymbolRows= 144; SymbolCols= 144; RegionRows=6; RegionCols=6; DataRegionHeight= 22; DataRegionWidth= 22; DataCW=1558; EccCW= 620; NumBlocks=10; EccPerBlock=62 }
    |]

/// All rectangular symbol sizes for Data Matrix ECC200 (ISO/IEC 16022:2006 Table 7).
let private RECT_SIZES : SymbolSizeEntry[] =
    [|
        { SymbolRows=  8; SymbolCols= 18; RegionRows=1; RegionCols=1; DataRegionHeight= 6; DataRegionWidth=16; DataCW=  5; EccCW=  7; NumBlocks=1; EccPerBlock= 7 }
        { SymbolRows=  8; SymbolCols= 32; RegionRows=1; RegionCols=2; DataRegionHeight= 6; DataRegionWidth=14; DataCW= 10; EccCW= 11; NumBlocks=1; EccPerBlock=11 }
        { SymbolRows= 12; SymbolCols= 26; RegionRows=1; RegionCols=1; DataRegionHeight=10; DataRegionWidth=24; DataCW= 16; EccCW= 14; NumBlocks=1; EccPerBlock=14 }
        { SymbolRows= 12; SymbolCols= 36; RegionRows=1; RegionCols=2; DataRegionHeight=10; DataRegionWidth=16; DataCW= 22; EccCW= 18; NumBlocks=1; EccPerBlock=18 }
        { SymbolRows= 16; SymbolCols= 36; RegionRows=1; RegionCols=2; DataRegionHeight=14; DataRegionWidth=16; DataCW= 32; EccCW= 24; NumBlocks=1; EccPerBlock=24 }
        { SymbolRows= 16; SymbolCols= 48; RegionRows=1; RegionCols=2; DataRegionHeight=14; DataRegionWidth=22; DataCW= 49; EccCW= 28; NumBlocks=1; EccPerBlock=28 }
    |]

// ============================================================================
// RS generator polynomial builder and cache
// ============================================================================
//
// Reed-Solomon encoding requires a generator polynomial:
//
//   g(x) = (x + α^1)(x + α^2) ··· (x + α^n_ecc)
//
// The b=1 convention means the roots are α^1, α^2, …, α^n — exactly what
// ISO/IEC 16022 specifies.  We build this polynomial by successive
// multiplication and cache the result for each n_ecc value that appears
// in the symbol size table.
//
// The polynomial is stored as ``[g_0; g_1; ...; g_{n_ecc}]`` with g_{n_ecc}=1
// (leading coefficient 1 = monic polynomial).  It has n_ecc+1 coefficients.

/// Build the RS generator polynomial for ``nEcc`` ECC bytes in GF(256)/0x12D.
///
/// Returns an array of length ``nEcc + 1`` where the leading coefficient is 1
/// and the remaining coefficients are the field elements of the polynomial.
let private buildGenerator (nEcc: int) : int[] =
    let mutable g = [| 1 |]
    for i in 1 .. nEcc do
        let ai = EXP_12D.[i]           // α^i
        let next = Array.zeroCreate (g.Length + 1)
        for j in 0 .. g.Length - 1 do
            next.[j]     <- next.[j]     ^^^ g.[j]          // multiply by x
            next.[j + 1] <- next.[j + 1] ^^^ gf256Mul g.[j] ai  // multiply by α^i
        g <- next
    g

/// Cache: nEcc → generator polynomial.
let private genPolyCache = System.Collections.Generic.Dictionary<int, int[]>()

/// Retrieve (or build and cache) the RS generator polynomial for ``nEcc``.
let private getGenerator (nEcc: int) : int[] =
    match genPolyCache.TryGetValue(nEcc) with
    | true,  g -> g
    | false, _ ->
        let g = buildGenerator nEcc
        genPolyCache.[nEcc] <- g
        g

// Pre-build all generators referenced in the size tables.
do
    for e in Array.append SQUARE_SIZES RECT_SIZES do
        getGenerator e.EccPerBlock |> ignore

// ============================================================================
// Reed-Solomon block encoder (GF(256)/0x12D, b=1 convention)
// ============================================================================
//
// Given a data block and the generator polynomial, compute ``nEcc`` ECC bytes
// using LFSR polynomial division:
//
//   Remainder R(x) = D(x) × x^n_ecc  mod  G(x)
//
// LFSR algorithm (operates on a shift register ``rem`` of length n_ecc):
//
//   for each data byte d in sequence:
//     feedback = d XOR rem.[0]
//     shift left: rem.[i] ← rem.[i+1] for i = 0..n_ecc-2
//     rem.[n_ecc-1] ← 0
//     if feedback ≠ 0:
//       for i = 0..n_ecc-1:
//         rem.[i] ^= gen.[i+1] × feedback
//
// The generator array has n_ecc+1 elements; we use elements [1..n_ecc]
// (skipping the leading 1).

/// Compute ``nEcc`` Reed-Solomon ECC bytes for one data block.
///
/// ``generator`` is the output of ``getGenerator nEcc``.
let private rsEncodeBlock (data: int[]) (generator: int[]) : int[] =
    let nEcc = generator.Length - 1
    let rem = Array.zeroCreate nEcc
    for b in data do
        let fb = b ^^^ rem.[0]
        // Shift register left by one position
        for i in 0 .. nEcc - 2 do
            rem.[i] <- rem.[i + 1]
        rem.[nEcc - 1] <- 0
        if fb <> 0 then
            for i in 0 .. nEcc - 1 do
                rem.[i] <- rem.[i] ^^^ gf256Mul generator.[i + 1] fb
    rem

// ============================================================================
// ASCII data encoding (ISO/IEC 16022:2006 §5.2.1)
// ============================================================================
//
// ASCII encoding rules:
//
//   1. Two consecutive ASCII digits → codeword = 130 + (d1 × 10 + d2)
//      This packs a two-digit pair into a single codeword (0 → 130, 99 → 229).
//      Saving a codeword is critical for manufacturing part numbers.
//
//   2. Single ASCII character (0–127) → codeword = ASCII value + 1
//
//   3. Extended ASCII (128–255) → two codewords: 235 (UPPER_SHIFT), byte - 127
//
// Examples:
//   "A"  → [66]        (65 + 1)
//   " "  → [33]        (32 + 1)
//   "12" → [142]       (130 + 12 — digit pair)
//   "1A" → [50; 66]    (49+1, 65+1 — no pair; 'A' is not a digit)
//   "99" → [229]       (130 + 99)

/// Encode a byte array in Data Matrix ASCII mode.
let private encodeAscii (input: byte[]) : int[] =
    let codewords = System.Collections.Generic.List<int>()
    let mutable i = 0
    while i < input.Length do
        let c = int input.[i]
        // Is this a digit pair?  Both this byte and the next must be '0'..'9'.
        if c >= 0x30 && c <= 0x39 &&
           i + 1 < input.Length &&
           int input.[i + 1] >= 0x30 && int input.[i + 1] <= 0x39 then
            let d1 = c - 0x30                       // first digit value  (0–9)
            let d2 = int input.[i + 1] - 0x30       // second digit value (0–9)
            codewords.Add(130 + d1 * 10 + d2)
            i <- i + 2
        elif c <= 127 then
            // Standard ASCII single character
            codewords.Add(c + 1)
            i <- i + 1
        else
            // Extended ASCII: UPPER_SHIFT (235) followed by shifted value
            codewords.Add(235)          // UPPER_SHIFT sentinel
            codewords.Add(c - 127)      // shifted codeword
            i <- i + 1
    codewords.ToArray()

// ============================================================================
// Pad codewords (ISO/IEC 16022:2006 §5.2.3)
// ============================================================================
//
// After encoding the data, unused codeword slots must be filled with pad values.
//
// Padding rules:
//   1. The first pad is always the literal value 129.
//   2. Subsequent pads use a scrambled value:
//        k = 1-based position within the full codeword stream
//        scrambled = 129 + ((149 × k) mod 253) + 1
//        if scrambled > 254 then scrambled ← scrambled - 254
//
// The scrambling prevents a long run of "129 129 129…" from creating a
// degenerate placement pattern that a Utah-scan algorithm might struggle with.
//
// Example: "A" (codeword [66]) padded to 3 codewords (10×10 symbol, dataCW=3):
//   k=2: pad1 = 129   (always literal)
//   k=3: scrambled = 129 + (149×3 mod 253) + 1 = 129 + 194 + 1 = 324 → 324−254 = 70
//   Result: [66; 129; 70]

/// Pad ``codewords`` out to exactly ``dataCW`` codewords.
let private padCodewords (codewords: int[]) (dataCW: int) : int[] =
    let padded = System.Collections.Generic.List<int>(codewords)
    let mutable k = padded.Count + 1  // k is 1-indexed position of the next pad
    while padded.Count < dataCW do
        if padded.Count = codewords.Length then
            // First pad: always literal 129
            padded.Add(129)
        else
            // Subsequent pads: scrambled
            let mutable scrambled = 129 + (149 * k % 253) + 1
            if scrambled > 254 then scrambled <- scrambled - 254
            padded.Add(scrambled)
        k <- k + 1
    padded.ToArray()

// ============================================================================
// Symbol selection
// ============================================================================
//
// Select the smallest ECC200 symbol whose dataCW capacity is ≥ the encoded
// codeword count.  Square symbols are checked first; rectangular symbols are
// only considered when the shape preference is "rectangular" or "any".
//
// The function returns a Result so callers can surface InputTooLong cleanly.

/// Select the smallest symbol whose data capacity holds ``cwCount`` codewords.
///
/// Returns ``Error (InputTooLong …)`` if no symbol can hold the data.
let private selectSymbol (cwCount: int) : Result<SymbolSizeEntry, DataMatrixError> =
    let candidates = SQUARE_SIZES
    match candidates |> Array.tryFind (fun e -> e.DataCW >= cwCount) with
    | Some e -> Ok e
    | None   ->
        Error (InputTooLong (
            sprintf "Encoded data requires %d codewords, exceeds maximum 1558 (144×144 symbol)." cwCount))

/// Select the smallest symbol whose data capacity holds ``cwCount`` codewords,
/// searching rectangular sizes only.
let private selectRectSymbol (cwCount: int) : Result<SymbolSizeEntry, DataMatrixError> =
    match RECT_SIZES |> Array.tryFind (fun e -> e.DataCW >= cwCount) with
    | Some e -> Ok e
    | None   -> selectSymbol cwCount   // fallback to square

// ============================================================================
// Block interleaving
// ============================================================================
//
// For multi-block symbols the data codewords are split into RS blocks,
// RS error correction is computed for each block, and then the blocks are
// interleaved (round-robin) for placement.
//
// Why interleave?
//
// A physical scratch destroying N contiguous modules affects at most
// ceil(N / numBlocks) codewords in each RS block — much more likely to be
// within correction capacity than if the codewords were laid out contiguously.
//
// Interleaving strategy (ISO/IEC 16022:2006 §5.2.4):
//
//   base = dataCW / numBlocks   (floor division)
//   extra = dataCW mod numBlocks
//   The first ``extra`` blocks get (base+1) codewords each.
//   The remaining (numBlocks - extra) blocks get ``base`` codewords each.
//
// Output order:
//   data[0][0], data[1][0], ..., data[B-1][0],
//   data[0][1], data[1][1], ..., data[B-1][1],  ...
//   ecc[0][0],  ecc[1][0],  ..., ecc[B-1][0],
//   ecc[0][1],  ...

/// Compute the full interleaved codeword stream (data + ECC) for all blocks.
let private computeInterleaved (data: int[]) (entry: SymbolSizeEntry) : int[] =
    let { DataCW=dataCW; NumBlocks=numBlocks; EccPerBlock=eccPerBlock } = entry
    let gen = getGenerator eccPerBlock

    // Split data into blocks
    let baseLen    = dataCW / numBlocks
    let extraBlocks= dataCW % numBlocks   // first ``extraBlocks`` blocks get +1

    let dataBlocks : int[][] = Array.init numBlocks (fun b ->
        let len = if b < extraBlocks then baseLen + 1 else baseLen
        let off = b * baseLen + min b extraBlocks
        data.[off .. off + len - 1])

    // Compute ECC for each block
    let eccBlocks : int[][] = dataBlocks |> Array.map (fun d -> rsEncodeBlock d gen)

    // Interleave data round-robin
    let result = System.Collections.Generic.List<int>()
    let maxDataLen = dataBlocks |> Array.map (fun b -> b.Length) |> Array.max
    for pos in 0 .. maxDataLen - 1 do
        for b in 0 .. numBlocks - 1 do
            if pos < dataBlocks.[b].Length then
                result.Add(dataBlocks.[b].[pos])

    // Interleave ECC round-robin
    for pos in 0 .. eccPerBlock - 1 do
        for b in 0 .. numBlocks - 1 do
            result.Add(eccBlocks.[b].[pos])

    result.ToArray()

// ============================================================================
// Grid initialisation — outer border and alignment borders
// ============================================================================
//
// Before placing data modules, we fill the physical grid with all the fixed
// structural elements.
//
// ### Outer border (finder + clock)
//
// The outermost ring of every Data Matrix symbol carries two visual languages:
//
//   Left column  (col 0):          all dark — the vertical leg of the L-finder.
//   Bottom row   (row symbolRows-1): all dark — the horizontal leg of the L-finder.
//   Top row      (row 0):          alternating dark/light starting dark at col 0
//                                   — the top timing clock.
//   Right column (col symbolCols-1): alternating dark/light starting dark at row 0
//                                    — the right timing clock.
//
// The asymmetric L distinguishes orientation: a scanner can tell which corner
// is which even if the symbol is rotated or mirrored.
//
// ### Alignment borders (multi-region symbols)
//
// For symbols with more than one data region (e.g. 32×32 has 2×2 regions), two
// columns/rows of alignment modules separate adjacent regions:
//
//   AB row/col 0: all dark
//   AB row/col 1: alternating dark/light starting dark
//
// This mirrors the outer border language, giving scanners consistent timing
// references inside large symbols.
//
// ### Write order
//
// Alignment borders are written first, then the outer border overrides any
// conflicts at the intersections.

/// Initialise the physical module grid with structural elements.
///
/// Returns a jagged bool array (true = dark, false = light).
let private initGrid (entry: SymbolSizeEntry) : bool[][] =
    let { SymbolRows=R; SymbolCols=C; RegionRows=rr; RegionCols=rc;
          DataRegionHeight=drH; DataRegionWidth=drW } = entry

    let grid = Array.init R (fun _ -> Array.create C false)

    // ── Alignment borders (written first so outer border can override)
    // Between each adjacent pair of region rows there are 2 alignment rows.
    // Physical row of the first alignment row after region row (rIdx+1):
    //   1 + (rIdx+1)*drH + rIdx*2
    for rIdx in 0 .. rr - 2 do
        let abRow0 = 1 + (rIdx + 1) * drH + rIdx * 2
        let abRow1 = abRow0 + 1
        for c in 0 .. C - 1 do
            grid.[abRow0].[c] <- true              // all dark
            grid.[abRow1].[c] <- (c % 2 = 0)      // alternating

    for cIdx in 0 .. rc - 2 do
        let abCol0 = 1 + (cIdx + 1) * drW + cIdx * 2
        let abCol1 = abCol0 + 1
        for r in 0 .. R - 1 do
            grid.[r].[abCol0] <- true              // all dark
            grid.[r].[abCol1] <- (r % 2 = 0)      // alternating

    // ── Top row: alternating dark/light starting dark at col 0
    for c in 0 .. C - 1 do
        grid.[0].[c] <- (c % 2 = 0)

    // ── Right column: alternating dark/light starting dark at row 0
    for r in 0 .. R - 1 do
        grid.[r].[C - 1] <- (r % 2 = 0)

    // ── Left column: all dark (L-finder left leg)
    // Override timing values at col 0 (row 0 is dark in both — safe either way)
    for r in 0 .. R - 1 do
        grid.[r].[0] <- true

    // ── Bottom row: all dark (L-finder bottom leg)
    // Override alignment border alternating values and right-column timing at
    // (R-1, C-1).  The L-finder takes highest visual precedence.
    for c in 0 .. C - 1 do
        grid.[R - 1].[c] <- true

    grid

// ============================================================================
// Utah diagonal placement algorithm
// ============================================================================
//
// The Utah algorithm is the most distinctive part of Data Matrix encoding.
// It was named "Utah" by the inventors because the 8-module shape used to
// place each codeword vaguely resembles the outline of the US state of Utah.
//
// ## The Utah 8-module shape
//
//   (relative to reference position (row, col)):
//
//       col-2  col-1  col
//   row-2:  .    [8]   [7]
//   row-1: [6]   [5]   [4]
//   row  : [3]   [2]   [1]
//
// Numbers 1–8 label the bit positions: bit 1 = LSB of the codeword (bit 0),
// bit 8 = MSB (bit 7).  Bit 1 goes to (row, col-2) and bit 8 goes to (row-2, col).
//
// Wait — let's clarify the bit ordering used by ISO/IEC 16022:
//
//   Placement map (bit 8 = MSB, bit 1 = LSB — the standard numbers from 1):
//     bit 8 → (row,   col)      placed first (MSB)
//     bit 7 → (row,   col-1)
//     bit 6 → (row,   col-2)
//     bit 5 → (row-1, col)
//     bit 4 → (row-1, col-1)
//     bit 3 → (row-1, col-2)
//     bit 2 → (row-2, col)
//     bit 1 → (row-2, col-1)   placed last (LSB)
//
// In code, bit 8 = (codeword >>> 7) &&& 1, bit 1 = (codeword >>> 0) &&& 1.
//
// ## Boundary wrap rules
//
// When the Utah shape extends outside the logical grid, ISO/IEC 16022
// Annex F specifies these wrap rules:
//
//   row < 0 && col = 0:           (row, col) ← (1, 3)
//   row < 0 && col = nCols:       (row, col) ← (0, col-2)
//   row < 0 (otherwise):          row += nRows; col -= 4
//   col < 0 (otherwise):          col += nCols; row -= 4
//
// ## Corner patterns
//
// Four special corner patterns handle references at specific boundary positions.
// They are placed in addition to (or instead of) the standard Utah shape.
//
// ## Diagonal traversal
//
// The reference position starts at (row=4, col=0) and moves diagonally:
//
//   Phase 1 (up-right scan):  row -= 2, col += 2   until out of bounds
//   Step:                      row += 1, col += 3   (brings reference back in)
//   Phase 2 (down-left scan): row += 2, col -= 2   until out of bounds
//   Step:                      row += 3, col += 1   (brings reference back in)
//
// This zigzag covers the entire logical grid systematically.
//
// ## Residual fill
//
// Some symbol sizes leave a few unplaced modules at the bottom-right corner
// after the zigzag completes.  ISO/IEC 16022 §10 specifies that these are
// filled with: module(r, c) = (r + c) mod 2 == 1.

/// Apply the ISO/IEC 16022 Annex F boundary wrap rules to (row, col).
let private applyWrap (row: int) (col: int) (nRows: int) (nCols: int) : struct(int * int) =
    if row < 0 && col = 0 then
        struct(1, 3)
    elif row < 0 && col = nCols then
        struct(0, col - 2)
    elif row < 0 then
        struct(row + nRows, col - 4)
    elif col < 0 then
        struct(row - 4, col + nCols)
    else
        struct(row, col)

/// Place one codeword at the standard Utah 8-module shape centred at (row, col).
///
/// The ``used`` grid tracks which cells have already been written so that later
/// passes do not overwrite earlier placements.
let private placeUtah
    (codeword: int) (row: int) (col: int)
    (nRows: int) (nCols: int)
    (grid: bool[][]) (used: bool[][]) : unit =

    // [rawRow, rawCol, bitIndex] — bitIndex 7=MSB, 0=LSB
    let placements =
        [|
            struct(row,     col,     7)  // bit 8 (MSB)
            struct(row,     col - 1, 6)  // bit 7
            struct(row,     col - 2, 5)  // bit 6
            struct(row - 1, col,     4)  // bit 5
            struct(row - 1, col - 1, 3)  // bit 4
            struct(row - 1, col - 2, 2)  // bit 3
            struct(row - 2, col,     1)  // bit 2
            struct(row - 2, col - 1, 0)  // bit 1 (LSB)
        |]

    for struct(r, c, bit) in placements do
        let struct(wr, wc) = applyWrap r c nRows nCols
        if wr >= 0 && wr < nRows && wc >= 0 && wc < nCols && not used.[wr].[wc] then
            grid.[wr].[wc] <- ((codeword >>> bit) &&& 1) = 1
            used.[wr].[wc] <- true

/// Corner pattern 1 — triggered at the top-left boundary.
///
/// Absolute positions within the logical grid:
///   bit 8 → (0, nCols-2)   bit 7 → (0, nCols-1)
///   bit 6 → (1, 0)         bit 5 → (2, 0)
///   bit 4 → (nRows-2, 0)   bit 3 → (nRows-1, 0)
///   bit 2 → (nRows-1, 1)   bit 1 → (nRows-1, 2)
let private placeCorner1
    (codeword: int) (nRows: int) (nCols: int)
    (grid: bool[][]) (used: bool[][]) : unit =
    let positions =
        [|
            struct(0,        nCols - 2, 7)
            struct(0,        nCols - 1, 6)
            struct(1,        0,         5)
            struct(2,        0,         4)
            struct(nRows - 2,0,         3)
            struct(nRows - 1,0,         2)
            struct(nRows - 1,1,         1)
            struct(nRows - 1,2,         0)
        |]
    for struct(r, c, bit) in positions do
        if r >= 0 && r < nRows && c >= 0 && c < nCols && not used.[r].[c] then
            grid.[r].[c] <- ((codeword >>> bit) &&& 1) = 1
            used.[r].[c] <- true

/// Corner pattern 2 — triggered at the top-right boundary.
///
/// Absolute positions:
///   bit 8 → (0, nCols-2)   bit 7 → (0, nCols-1)
///   bit 6 → (1, nCols-1)   bit 5 → (2, nCols-1)
///   bit 4 → (nRows-1, 0)   bit 3 → (nRows-1, 1)
///   bit 2 → (nRows-1, 2)   bit 1 → (nRows-1, 3)
let private placeCorner2
    (codeword: int) (nRows: int) (nCols: int)
    (grid: bool[][]) (used: bool[][]) : unit =
    let positions =
        [|
            struct(0,        nCols - 2, 7)
            struct(0,        nCols - 1, 6)
            struct(1,        nCols - 1, 5)
            struct(2,        nCols - 1, 4)
            struct(nRows - 1,0,         3)
            struct(nRows - 1,1,         2)
            struct(nRows - 1,2,         1)
            struct(nRows - 1,3,         0)
        |]
    for struct(r, c, bit) in positions do
        if r >= 0 && r < nRows && c >= 0 && c < nCols && not used.[r].[c] then
            grid.[r].[c] <- ((codeword >>> bit) &&& 1) = 1
            used.[r].[c] <- true

/// Corner pattern 3 — triggered at the bottom-left boundary.
///
/// Absolute positions:
///   bit 8 → (0, nCols-1)   bit 7 → (1, 0)
///   bit 6 → (2, 0)         bit 5 → (nRows-2, 0)
///   bit 4 → (nRows-1, 0)   bit 3 → (nRows-1, 1)
///   bit 2 → (nRows-1, 2)   bit 1 → (nRows-1, 3)
let private placeCorner3
    (codeword: int) (nRows: int) (nCols: int)
    (grid: bool[][]) (used: bool[][]) : unit =
    let positions =
        [|
            struct(0,        nCols - 1, 7)
            struct(1,        0,         6)
            struct(2,        0,         5)
            struct(nRows - 2,0,         4)
            struct(nRows - 1,0,         3)
            struct(nRows - 1,1,         2)
            struct(nRows - 1,2,         1)
            struct(nRows - 1,3,         0)
        |]
    for struct(r, c, bit) in positions do
        if r >= 0 && r < nRows && c >= 0 && c < nCols && not used.[r].[c] then
            grid.[r].[c] <- ((codeword >>> bit) &&& 1) = 1
            used.[r].[c] <- true

/// Corner pattern 4 — right-edge wrap for odd-dimension matrices.
///
/// Used when nRows and nCols are both odd (rectangular symbols).
///
/// Absolute positions:
///   bit 8 → (nRows-3, nCols-1)  bit 7 → (nRows-2, nCols-1)
///   bit 6 → (nRows-1, nCols-3)  bit 5 → (nRows-1, nCols-2)
///   bit 4 → (nRows-1, nCols-1)  bit 3 → (0, 0)
///   bit 2 → (1, 0)              bit 1 → (2, 0)
let private placeCorner4
    (codeword: int) (nRows: int) (nCols: int)
    (grid: bool[][]) (used: bool[][]) : unit =
    let positions =
        [|
            struct(nRows - 3, nCols - 1, 7)
            struct(nRows - 2, nCols - 1, 6)
            struct(nRows - 1, nCols - 3, 5)
            struct(nRows - 1, nCols - 2, 4)
            struct(nRows - 1, nCols - 1, 3)
            struct(0,         0,         2)
            struct(1,         0,         1)
            struct(2,         0,         0)
        |]
    for struct(r, c, bit) in positions do
        if r >= 0 && r < nRows && c >= 0 && c < nCols && not used.[r].[c] then
            grid.[r].[c] <- ((codeword >>> bit) &&& 1) = 1
            used.[r].[c] <- true

/// Run the Utah diagonal placement algorithm on the logical data matrix.
///
/// ``codewords`` is the full interleaved stream (data + ECC).
/// ``nRows`` × ``nCols`` is the logical data matrix dimension.
///
/// Returns a nRows × nCols boolean grid (true = dark).
let private utahPlacement (codewords: int[]) (nRows: int) (nCols: int) : bool[][] =
    let grid = Array.init nRows (fun _ -> Array.create nCols false)
    let used = Array.init nRows (fun _ -> Array.create nCols false)
    let mutable cwIdx = 0
    let mutable row   = 4
    let mutable col   = 0

    let tryPlace fn =
        if cwIdx < codewords.Length then
            fn codewords.[cwIdx] nRows nCols grid used
            cwIdx <- cwIdx + 1

    let mutable running = true
    while running do
        // ── Corner special cases (triggered by specific reference positions)
        if row = nRows     && col = 0 && (nRows % 4 = 0 || nCols % 4 = 0) then
            tryPlace placeCorner1
        if row = nRows - 2 && col = 0 && nCols % 4 <> 0 then
            tryPlace placeCorner2
        if row = nRows - 2 && col = 0 && nCols % 8 = 4 then
            tryPlace placeCorner3
        if row = nRows + 4 && col = 2 && nCols % 8 = 0 then
            tryPlace placeCorner4

        // ── Phase 1: diagonal scan upward-right (row -= 2, col += 2)
        let mutable scanning1 = true
        while scanning1 do
            if row >= 0 && row < nRows && col >= 0 && col < nCols && not used.[row].[col] then
                if cwIdx < codewords.Length then
                    placeUtah codewords.[cwIdx] row col nRows nCols grid used
                    cwIdx <- cwIdx + 1
            row <- row - 2
            col <- col + 2
            if not (row >= 0 && col < nCols) then
                scanning1 <- false

        // ── Step to next diagonal start
        row <- row + 1
        col <- col + 3

        // ── Phase 2: diagonal scan downward-left (row += 2, col -= 2)
        let mutable scanning2 = true
        while scanning2 do
            if row >= 0 && row < nRows && col >= 0 && col < nCols && not used.[row].[col] then
                if cwIdx < codewords.Length then
                    placeUtah codewords.[cwIdx] row col nRows nCols grid used
                    cwIdx <- cwIdx + 1
            row <- row + 2
            col <- col - 2
            if not (row < nRows && col >= 0) then
                scanning2 <- false

        // ── Step to next diagonal start
        row <- row + 3
        col <- col + 1

        // ── Termination: reference fully past the grid
        if row >= nRows && col >= nCols then running <- false
        elif cwIdx >= codewords.Length then running <- false

    // ── Residual fill: modules not touched by the walk
    // ISO/IEC 16022 §10 specifies: module(r, c) = dark if (r + c) mod 2 = 1.
    for r in 0 .. nRows - 1 do
        for c in 0 .. nCols - 1 do
            if not used.[r].[c] then
                grid.[r].[c] <- (r + c) % 2 = 1

    grid

// ============================================================================
// Logical → physical coordinate mapping
// ============================================================================
//
// The Utah placement algorithm works entirely in "logical" coordinates — a
// single flat nRows × nCols grid that treats all data regions as one.
//
// After placement, we must map logical coordinates back to the physical symbol
// grid, which includes the outer border and any inter-region alignment borders.
//
// For a symbol with (rr × rc) data regions each of size (rh × rw):
//
//   physRow = (logRow / rh) * (rh + 2) + (logRow mod rh) + 1
//   physCol = (logCol / rw) * (rw + 2) + (logCol mod rw) + 1
//
// The "× (rh + 2)" accounts for the 2-module alignment border between adjacent
// data regions.  The "+ 1" accounts for the 1-module outer border.
//
// For single-region symbols (rr=rc=1) this simplifies to:
//   physRow = logRow + 1,  physCol = logCol + 1

/// Map a logical data matrix coordinate to a physical symbol coordinate.
let private logicalToPhysical (r: int) (c: int) (entry: SymbolSizeEntry) : struct(int * int) =
    let rh = entry.DataRegionHeight
    let rw = entry.DataRegionWidth
    let physRow = (r / rh) * (rh + 2) + (r % rh) + 1
    let physCol = (c / rw) * (rw + 2) + (c % rw) + 1
    struct(physRow, physCol)

// ============================================================================
// Full encoding pipeline
// ============================================================================

/// Encode a byte array as a Data Matrix ECC200 symbol.
///
/// Returns ``Ok (ModuleGrid)`` on success or ``Error (DataMatrixError)`` if
/// the input is too long.
let encodeBytes (input: byte[]) : Result<ModuleGrid, DataMatrixError> =
    // Step 1: ASCII encode the input bytes
    let codewords = encodeAscii input

    // Step 2: Select the smallest symbol that can hold the codewords
    match selectSymbol codewords.Length with
    | Error e -> Error e
    | Ok entry ->

    // Step 3: Pad to capacity
    let padded = padCodewords codewords entry.DataCW

    // Step 4–5: RS ECC computation + interleaving
    let interleaved = computeInterleaved padded entry

    // Step 6: Initialise the physical symbol grid with structural modules
    let physGrid = initGrid entry

    // Step 7: Utah placement on the logical data matrix
    let nRows = entry.RegionRows * entry.DataRegionHeight
    let nCols = entry.RegionCols * entry.DataRegionWidth
    let logicalGrid = utahPlacement interleaved nRows nCols

    // Step 8: Map logical coordinates to physical coordinates
    for r in 0 .. nRows - 1 do
        for c in 0 .. nCols - 1 do
            let struct(pr, pc) = logicalToPhysical r c entry
            physGrid.[pr].[pc] <- logicalGrid.[r].[c]

    // Step 9: Assemble the ModuleGrid (no masking — Data Matrix never masks)
    Ok {
        Rows        = entry.SymbolRows
        Cols        = entry.SymbolCols
        Modules     = physGrid
        ModuleShape = ModuleShape.Square
    }

/// Encode a UTF-8 string as a Data Matrix ECC200 symbol.
///
/// Returns ``Ok (ModuleGrid)`` on success.
/// Returns ``Error (InputTooLong …)`` if the string exceeds 144×144 capacity.
///
/// The ``ModuleGrid`` is the canonical output of this encoder:
///   - ``grid.Modules.[row].[col] = true``  → dark module
///   - ``grid.Modules.[row].[col] = false`` → light module
///
/// Pass the result to ``CodingAdventures.Barcode2D.layout`` to get
/// pixel-level ``PaintScene`` instructions for rendering.
///
/// Examples:
///
///   match DataMatrix.encode "Hello World" with
///   | Ok grid ->
///       // grid.Rows = grid.Cols = 16 for "Hello World" (11 codewords → 16×16)
///       printfn "Symbol: %d × %d modules" grid.Rows grid.Cols
///   | Error e -> printfn "Error: %A" e
let encode (input: string) : Result<ModuleGrid, DataMatrixError> =
    encodeBytes (Encoding.UTF8.GetBytes input)

// ============================================================================
// Exposed internals (for unit testing)
// ============================================================================
//
// These values and functions are deliberately exposed so that unit tests can
// verify each pipeline stage independently.  They are NOT part of the public
// API — they may change between minor versions.

/// GF(256)/0x12D exponentiation table (length 512, doubled for fast multiply).
let internal gfExp = EXP_12D

/// GF(256)/0x12D discrete log table.
let internal gfLog = LOG_12D

/// Multiply two GF(256)/0x12D elements.
let internal gfMul (a: int) (b: int) : int = gf256Mul a b

/// Encode a byte array in ASCII mode and return the codeword sequence.
let internal encodeAsciiInternal (input: byte[]) : int[] = encodeAscii input

/// Pad a codeword sequence to ``dataCW`` codewords.
let internal padCodewordsInternal (codewords: int[]) (dataCW: int) : int[] =
    padCodewords codewords dataCW

/// Select the smallest square symbol for ``cwCount`` codewords.
let internal selectSymbolInternal (cwCount: int) : Result<SymbolSizeEntry, DataMatrixError> =
    selectSymbol cwCount

/// Compute RS ECC for a single data block.
let internal rsEncodeBlockInternal (data: int[]) (nEcc: int) : int[] =
    rsEncodeBlock data (getGenerator nEcc)

/// Run Utah placement on a logical grid and return the result.
let internal utahPlacementInternal (codewords: int[]) (nRows: int) (nCols: int) : bool[][] =
    utahPlacement codewords nRows nCols

/// All square symbol size entries (for inspection in tests).
let internal squareSizes : SymbolSizeEntry[] = SQUARE_SIZES

/// All rectangular symbol size entries.
let internal rectSizes : SymbolSizeEntry[] = RECT_SIZES
