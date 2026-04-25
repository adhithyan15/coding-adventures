/// QRCode.fs — ISO/IEC 18004:2015-compliant QR Code encoder for F#
///
/// This module encodes any UTF-8 string into a scannable QR Code and returns
/// a ``ModuleGrid`` (from barcode-2d) — a plain 2-D boolean grid where
/// ``true`` means a dark module and ``false`` means a light module.
///
/// ## The full encoding pipeline
///
/// ```
/// input string
///   → mode selection    (numeric / alphanumeric / byte)
///   → version selection (smallest v1–40 that fits at the ECC level)
///   → bit stream        (mode indicator + char count + data + padding)
///   → blocks + RS ECC   (GF(256) b=0 convention, poly 0x11D)
///   → interleave        (data codewords round-robin, then ECC codewords)
///   → grid init         (finder × 3, separators, timing, alignment, format, dark)
///   → zigzag placement  (two-column snake from bottom-right)
///   → mask evaluation   (8 patterns, 4-rule penalty, pick lowest)
///   → finalize          (format info + version info v7+)
///   → ModuleGrid
/// ```
///
/// ## Literate guide
///
/// A QR Code is a square grid of black and white "modules" (pixels).  The
/// standard specifies exactly:
///
/// 1. **Version** — the size.  Version 1 is 21×21 modules; each higher version
///    adds 4 to both dimensions (version 40 is 177×177).
///
/// 2. **Error Correction Level** — four levels: L (~7% recovery), M (~15%),
///    Q (~25%), H (~30%).  Higher levels use more modules for ECC, reducing
///    data capacity.
///
/// 3. **Encoding modes** — numeric, alphanumeric, byte (and kanji, not
///    implemented here).  The encoder automatically picks the most compact mode
///    that can represent the input.
///
/// 4. **Reed-Solomon error correction** — the data is split into blocks; each
///    block gets ECC codewords computed in GF(2^8).  The RS convention used by
///    QR Code has first root b=0, so the generator is g(x) = ∏(x + α^i) for
///    i in 0..n−1, where α = 2 in GF(256) with primitive poly 0x11D.
///
/// 5. **Masking** — eight XOR patterns are tried; the one producing the least
///    visual clustering (measured by the 4-rule penalty score) is chosen.
///
/// 6. **Format information** — a 15-bit word (5 data bits + 10 BCH parity bits,
///    then XOR'd with mask sequence 101010000010010) placed in two copies.

module CodingAdventures.QRCode

open System
open CodingAdventures.Gf256
open CodingAdventures.Barcode2D

// ============================================================================
// Public types
// ============================================================================

/// Error Correction Level — controls what fraction of codewords may be
/// corrupted and still allow the original data to be recovered.
type EccLevel =
    /// ~7 % of codewords recoverable (highest capacity, lowest resilience).
    | L
    /// ~15 % of codewords recoverable (common default).
    | M
    /// ~25 % of codewords recoverable.
    | Q
    /// ~30 % of codewords recoverable (lowest capacity, highest resilience).
    | H

/// Errors that can be returned by the encoder.
type QRCodeError =
    /// The input is too long to fit in any version at the chosen ECC level.
    | InputTooLong of string

    override e.ToString() =
        match e with
        | InputTooLong msg -> sprintf "InputTooLong: %s" msg

// ============================================================================
// Internal helpers — ECC indexing
// ============================================================================

// ISO 18004 tables are conventionally indexed by (ecc, version).
// We map the four ECC levels to row indices 0–3 to index into the static
// tables below.

[<RequireQualifiedAccess>]
module private EccUtil =

    /// 2-bit indicator stored in the format information word (Table 12).
    let indicator = function
        | L -> 0b01us
        | M -> 0b00us
        | Q -> 0b11us
        | H -> 0b10us

    /// Row index into the ECC tables below.
    let rowIndex = function
        | L -> 0
        | M -> 1
        | Q -> 2
        | H -> 3

// ============================================================================
// ISO 18004:2015 — Table 9: Number of ECC codewords per block
//                  Table 9: Number of error correction blocks
// ============================================================================

// These two arrays are the heart of the QR Code standard.  Every cell has been
// transcribed from the ISO 18004:2015 annex tables.  Row 0 = index padding;
// actual versions start at index 1.

/// ECC codewords per block, [eccRow][version].  Index 0 is padding (−1).
let private eccCodewordsPerBlock : int[,] =
    array2D
        [|
            //  0   1   2   3   4   5   6   7   8   9  10  11  12  13  14  15  16  17  18  19  20  21  22  23  24  25  26  27  28  29  30  31  32  33  34  35  36  37  38  39  40
            [| -1;  7; 10; 15; 20; 26; 18; 20; 24; 30; 18; 20; 24; 26; 30; 22; 24; 28; 30; 28; 28; 28; 28; 30; 30; 26; 28; 30; 30; 30; 30; 30; 30; 30; 30; 30; 30; 30; 30; 30; 30 |] // L
            [| -1; 10; 16; 26; 18; 24; 16; 18; 22; 22; 26; 30; 22; 22; 24; 24; 28; 28; 26; 26; 26; 26; 28; 28; 28; 28; 28; 28; 28; 28; 28; 28; 28; 28; 28; 28; 28; 28; 28; 28; 28 |] // M
            [| -1; 13; 22; 18; 26; 18; 24; 18; 22; 20; 24; 28; 26; 24; 20; 30; 24; 28; 28; 26; 30; 28; 30; 30; 30; 30; 28; 30; 30; 30; 30; 30; 30; 30; 30; 30; 30; 30; 30; 30; 30 |] // Q
            [| -1; 17; 28; 22; 16; 22; 28; 26; 26; 24; 28; 24; 28; 22; 24; 24; 30; 28; 28; 26; 28; 30; 24; 30; 30; 30; 30; 30; 30; 30; 30; 30; 30; 30; 30; 30; 30; 30; 30; 30; 30 |] // H
        |]

/// Number of error correction blocks, [eccRow][version].  Index 0 is padding.
let private numBlocks : int[,] =
    array2D
        [|
            //  0   1   2   3   4   5   6   7   8   9  10  11  12  13  14  15  16  17  18  19  20  21  22  23  24  25  26  27  28  29  30  31  32  33  34  35  36  37  38  39  40
            [| -1;  1;  1;  1;  1;  1;  2;  2;  2;  2;  4;  4;  4;  4;  4;  6;  6;  6;  6;  7;  8;  8;  9;  9; 10; 12; 12; 12; 13; 14; 15; 16; 17; 18; 19; 19; 20; 21; 22; 24; 25 |] // L
            [| -1;  1;  1;  1;  2;  2;  4;  4;  4;  5;  5;  5;  8;  9;  9; 10; 10; 11; 13; 14; 16; 17; 17; 18; 20; 21; 23; 25; 26; 28; 29; 31; 33; 35; 37; 38; 40; 43; 45; 47; 49 |] // M
            [| -1;  1;  1;  2;  2;  4;  4;  6;  6;  8;  8;  8; 10; 12; 16; 12; 17; 16; 18; 21; 20; 23; 23; 25; 27; 29; 34; 34; 35; 38; 40; 43; 45; 48; 51; 53; 56; 59; 62; 65; 68 |] // Q
            [| -1;  1;  1;  2;  4;  4;  4;  5;  6;  8;  8; 11; 11; 16; 16; 18; 16; 19; 21; 25; 25; 25; 34; 30; 32; 35; 37; 40; 42; 45; 48; 51; 54; 57; 60; 63; 66; 70; 74; 77; 80 |] // H
        |]

// ============================================================================
// Alignment pattern center coordinates (ISO 18004, Annex E)
// ============================================================================

// Alignment patterns are small 5×5 patterns (border + center) placed at fixed
// positions to help scanners correct for perspective distortion.  Version 1 has
// no alignment patterns.  Higher versions have one or more rows/columns of
// coordinates; the actual pattern positions are every combination of those
// coordinates (excluding intersections with the three finder patterns).

let private alignmentPositions : byte[][] =
    [|
        [||]                                     // v1  — no alignment patterns
        [| 6uy; 18uy |]                          // v2
        [| 6uy; 22uy |]                          // v3
        [| 6uy; 26uy |]                          // v4
        [| 6uy; 30uy |]                          // v5
        [| 6uy; 34uy |]                          // v6
        [| 6uy; 22uy; 38uy |]                    // v7
        [| 6uy; 24uy; 42uy |]                    // v8
        [| 6uy; 26uy; 46uy |]                    // v9
        [| 6uy; 28uy; 50uy |]                    // v10
        [| 6uy; 30uy; 54uy |]                    // v11
        [| 6uy; 32uy; 58uy |]                    // v12
        [| 6uy; 34uy; 62uy |]                    // v13
        [| 6uy; 26uy; 46uy; 66uy |]              // v14
        [| 6uy; 26uy; 48uy; 70uy |]              // v15
        [| 6uy; 26uy; 50uy; 74uy |]              // v16
        [| 6uy; 30uy; 54uy; 78uy |]              // v17
        [| 6uy; 30uy; 56uy; 82uy |]              // v18
        [| 6uy; 30uy; 58uy; 86uy |]              // v19
        [| 6uy; 34uy; 62uy; 90uy |]              // v20
        [| 6uy; 28uy; 50uy; 72uy; 94uy |]        // v21
        [| 6uy; 26uy; 50uy; 74uy; 98uy |]        // v22
        [| 6uy; 30uy; 54uy; 78uy; 102uy |]       // v23
        [| 6uy; 28uy; 54uy; 80uy; 106uy |]       // v24
        [| 6uy; 32uy; 58uy; 84uy; 110uy |]       // v25
        [| 6uy; 30uy; 58uy; 86uy; 114uy |]       // v26
        [| 6uy; 34uy; 62uy; 90uy; 118uy |]       // v27
        [| 6uy; 26uy; 50uy; 74uy;  98uy; 122uy |] // v28
        [| 6uy; 30uy; 54uy; 78uy; 102uy; 126uy |] // v29
        [| 6uy; 26uy; 52uy; 78uy; 104uy; 130uy |] // v30
        [| 6uy; 30uy; 56uy; 82uy; 108uy; 134uy |] // v31
        [| 6uy; 34uy; 60uy; 86uy; 112uy; 138uy |] // v32
        [| 6uy; 30uy; 58uy; 86uy; 114uy; 142uy |] // v33
        [| 6uy; 34uy; 62uy; 90uy; 118uy; 146uy |] // v34
        [| 6uy; 30uy; 54uy; 78uy; 102uy; 126uy; 150uy |] // v35
        [| 6uy; 24uy; 50uy; 76uy; 102uy; 128uy; 154uy |] // v36
        [| 6uy; 28uy; 54uy; 80uy; 106uy; 132uy; 158uy |] // v37
        [| 6uy; 32uy; 58uy; 84uy; 110uy; 136uy; 162uy |] // v38
        [| 6uy; 26uy; 54uy; 82uy; 110uy; 138uy; 166uy |] // v39
        [| 6uy; 30uy; 58uy; 86uy; 114uy; 142uy; 170uy |] // v40
    |]

// ============================================================================
// Grid geometry helpers
// ============================================================================

// A version-v QR Code is (4v + 17) × (4v + 17) modules.
// The "raw data modules" formula counts all modules minus fixed function
// patterns; the number of data codewords is derived from that.

/// Side length of a version-v QR Code symbol.
let private symbolSize (version: int) = 4 * version + 17

/// Total raw data+ECC bits available in a version-v symbol.
/// (Formula from Nayuki Reference QR Code, which matches ISO 18004.)
let private numRawDataModules (version: int) =
    let v = int64 version
    let mutable result = (16L * v + 128L) * v + 64L
    if version >= 2 then
        let numAlign = v / 7L + 2L
        result <- result - (25L * numAlign - 10L) * numAlign + 55L
        if version >= 7 then
            result <- result - 36L
    int result

/// Number of *data* codewords (raw CW count minus ECC CW count).
let private numDataCodewords (version: int) (ecc: EccLevel) =
    let e = EccUtil.rowIndex ecc
    let rawCw = numRawDataModules version / 8
    let eccCw = numBlocks.[e, version] * eccCodewordsPerBlock.[e, version]
    rawCw - eccCw

/// Remainder bits appended after all codewords (0 for some versions).
let private numRemainderBits (version: int) =
    numRawDataModules version % 8

// ============================================================================
// Reed-Solomon over GF(256), b=0 convention
// ============================================================================

// QR Code uses a specific RS convention: the generator polynomial has its
// first root at α^0 = 1, so g(x) = (x + α^0)(x + α^1)…(x + α^{n-1}).
//
// We use the CodingAdventures.Gf256 package for field arithmetic (poly 0x11D).

/// Build the monic RS generator polynomial of degree ``n`` with roots
/// α^0, α^1, …, α^{n-1}.  The output has ``n+1`` coefficients; index 0 is
/// the leading coefficient (1).
let private buildGenerator (n: int) : byte[] =
    let mutable g = [| 1uy |]
    for i in 0 .. n - 1 do
        // α^i in GF(256)
        let ai = Gf256.power 2uy i
        let next = Array.zeroCreate (g.Length + 1)
        for j in 0 .. g.Length - 1 do
            next.[j] <- next.[j] ^^^ g.[j]
            next.[j + 1] <- next.[j + 1] ^^^ (Gf256.multiply g.[j] ai)
        g <- next
    g

/// Compute n ECC bytes by polynomial long division (LFSR implementation).
///
/// Returns the remainder of D(x)·x^n ÷ G(x) in GF(256).
let private rsEncode (data: byte[]) (generator: byte[]) : byte[] =
    let n = generator.Length - 1
    let rem = Array.zeroCreate n
    for b in data do
        let fb = b ^^^ rem.[0]
        // Shift the register left
        Array.blit rem 1 rem 0 (n - 1)
        rem.[n - 1] <- 0uy
        if fb <> 0uy then
            for i in 0 .. n - 1 do
                rem.[i] <- rem.[i] ^^^ (Gf256.multiply generator.[i + 1] fb)
    rem

// ============================================================================
// Encoding modes
// ============================================================================

// ISO 18004 defines three common encoding modes:
//   Numeric      — digits 0-9 only; packs 3 digits into 10 bits.
//   Alphanumeric — digits + uppercase letters + 9 symbols; pairs → 11 bits.
//   Byte         — arbitrary bytes; 8 bits each (UTF-8 falls here).

/// The 45-character alphanumeric alphabet, in the canonical QR Code order.
let private alphanumChars = "0123456789ABCDEFGHIJKLMNOPQRSTUVWXYZ $%*+-./:"

[<RequireQualifiedAccess>]
type private EncodingMode = Numeric | Alphanumeric | Byte

/// Pick the most compact mode for the given input string.
let private selectMode (input: string) =
    if input |> Seq.forall Char.IsDigit then
        EncodingMode.Numeric
    elif input |> Seq.forall (fun c -> alphanumChars.Contains(c)) then
        EncodingMode.Alphanumeric
    else
        EncodingMode.Byte

/// 4-bit mode indicator (Table 2).
let private modeIndicator = function
    | EncodingMode.Numeric      -> 0b0001u
    | EncodingMode.Alphanumeric -> 0b0010u
    | EncodingMode.Byte         -> 0b0100u

/// Number of bits in the character count field (Table 3).
let private charCountBits (mode: EncodingMode) (version: int) =
    match mode with
    | EncodingMode.Numeric      -> if version <= 9 then 10 elif version <= 26 then 12 else 14
    | EncodingMode.Alphanumeric -> if version <= 9 then  9 elif version <= 26 then 11 else 13
    | EncodingMode.Byte         -> if version <= 9 then  8 else 16

// ============================================================================
// Bit-stream writer
// ============================================================================

// We accumulate individual bits in a bool list (reversed for O(1) cons) then
// pack into bytes at the end.  This is much cleaner than manual bit-shifting
// for a literate implementation.

/// Mutable bit accumulator that can be flushed to a byte array.
type private BitWriter() =
    let mutable bits : bool list = []    // accumulated in reverse order
    let mutable len  = 0

    /// Append ``count`` bits from ``value`` (MSB first).
    member _.Write(value: uint32, count: int) =
        for i in count - 1 .. -1 .. 0 do
            bits <- ((value >>> i) &&& 1u = 1u) :: bits
            len <- len + 1

    member _.BitLength = len

    /// Pack accumulated bits into bytes, MSB first, padding the last byte
    /// with zero bits if necessary.
    member _.ToBytes() : byte[] =
        let arr = Array.ofList (List.rev bits)
        let nBytes = (arr.Length + 7) / 8
        let result = Array.zeroCreate nBytes
        for i in 0 .. arr.Length - 1 do
            if arr.[i] then
                result.[i / 8] <- result.[i / 8] ||| (1uy <<< (7 - i % 8))
        result

/// Encode a numeric string into the bit writer.
let private encodeNumeric (input: string) (w: BitWriter) =
    // Groups of 3 digits → 10-bit value; remainder → 7 bits (2 digits) or 4 bits (1 digit).
    let digits = input |> Seq.map (fun c -> uint32 c - uint32 '0') |> Array.ofSeq
    let mutable i = 0
    while i + 2 < digits.Length do
        w.Write(digits.[i] * 100u + digits.[i + 1] * 10u + digits.[i + 2], 10)
        i <- i + 3
    if i + 1 < digits.Length then
        w.Write(digits.[i] * 10u + digits.[i + 1], 7)
        i <- i + 2
    if i < digits.Length then
        w.Write(digits.[i], 4)

/// Encode an alphanumeric string into the bit writer.
let private encodeAlphanumeric (input: string) (w: BitWriter) =
    // Each character maps to a value 0–44.  Pairs → 11 bits; single → 6 bits.
    let values = input |> Seq.map (fun c -> uint32 (alphanumChars.IndexOf(c))) |> Array.ofSeq
    let mutable i = 0
    while i + 1 < values.Length do
        w.Write(values.[i] * 45u + values.[i + 1], 11)
        i <- i + 2
    if i < values.Length then
        w.Write(values.[i], 6)

/// Encode a byte-mode string (raw UTF-8 bytes) into the bit writer.
let private encodeByteMode (input: string) (w: BitWriter) =
    for b in Text.Encoding.UTF8.GetBytes(input) do
        w.Write(uint32 b, 8)

/// Build the full data codeword sequence for a given version and ECC level.
let private buildDataCodewords (input: string) (version: int) (ecc: EccLevel) : byte[] =
    let mode     = selectMode input
    let capacity = numDataCodewords version ecc
    let w        = BitWriter()

    // Mode indicator (4 bits)
    w.Write(modeIndicator mode, 4)

    // Character count
    let charCount =
        match mode with
        | EncodingMode.Byte -> uint32 (Text.Encoding.UTF8.GetByteCount(input))
        | _                 -> uint32 (Seq.length input)
    w.Write(charCount, charCountBits mode version)

    // Data payload
    match mode with
    | EncodingMode.Numeric      -> encodeNumeric input w
    | EncodingMode.Alphanumeric -> encodeAlphanumeric input w
    | EncodingMode.Byte         -> encodeByteMode input w

    // Terminator: up to 4 zero bits to reach a codeword boundary
    let available = capacity * 8
    let termLen   = min 4 (available - w.BitLength)
    if termLen > 0 then w.Write(0u, termLen)

    // Pad to byte boundary
    let rem = w.BitLength % 8
    if rem <> 0 then w.Write(0u, 8 - rem)

    let mutable bytes = Array.toList (w.ToBytes())

    // Pad with alternating 0xEC/0x11 to fill the data capacity
    let mutable pad = 0xECuy
    while List.length bytes < capacity do
        bytes <- bytes @ [pad]
        pad <- if pad = 0xECuy then 0x11uy else 0xECuy

    List.toArray bytes

// ============================================================================
// Block splitting and interleaving
// ============================================================================

// QR Code splits data into one or two groups of blocks.  Group 1 blocks have
// ``shortLen`` data codewords; Group 2 (the "longer" blocks) have
// ``shortLen + 1``.  Each block gets the same number of ECC codewords.
// After RS encoding, data codewords are interleaved round-robin across all
// blocks, followed by ECC codewords in the same pattern.

type private Block = { Data: byte[]; Ecc: byte[] }

let private computeBlocks (data: byte[]) (version: int) (ecc: EccLevel) : Block[] =
    let e          = EccUtil.rowIndex ecc
    let totalBlocks = numBlocks.[e, version]
    let eccLen     = eccCodewordsPerBlock.[e, version]
    let totalData  = numDataCodewords version ecc
    let shortLen   = totalData / totalBlocks
    let numLong    = totalData % totalBlocks   // how many "long" (shortLen+1) blocks
    let gen        = buildGenerator eccLen

    let blocks     = Array.zeroCreate totalBlocks
    let mutable offset = 0

    // Group 1: (totalBlocks - numLong) blocks of length shortLen
    for i in 0 .. totalBlocks - numLong - 1 do
        let d = data.[offset .. offset + shortLen - 1]
        blocks.[i] <- { Data = d; Ecc = rsEncode d gen }
        offset <- offset + shortLen

    // Group 2: numLong blocks of length shortLen + 1
    for i in totalBlocks - numLong .. totalBlocks - 1 do
        let d = data.[offset .. offset + shortLen]
        blocks.[i] <- { Data = d; Ecc = rsEncode d gen }
        offset <- offset + shortLen + 1

    blocks

let private interleaveBlocks (blocks: Block[]) : byte[] =
    let maxData = blocks |> Array.map (fun b -> b.Data.Length) |> Array.max
    let maxEcc  = blocks |> Array.map (fun b -> b.Ecc.Length)  |> Array.max
    let result  = ResizeArray()

    for i in 0 .. maxData - 1 do
        for b in blocks do
            if i < b.Data.Length then result.Add(b.Data.[i])

    for i in 0 .. maxEcc - 1 do
        for b in blocks do
            if i < b.Ecc.Length then result.Add(b.Ecc.[i])

    result.ToArray()

// ============================================================================
// Work grid — mutable grid used during construction
// ============================================================================

// We need to track two things per module:
//   modules  — the current value (dark = true, light = false)
//   reserved — whether the module is part of a function pattern and must not
//              be overwritten during data placement or masking

type private WorkGrid(size: int) =
    let modules  = Array2D.create size size false
    let reserved = Array2D.create size size false

    member _.Size = size
    member _.Modules  = modules
    member _.Reserved = reserved

    member _.Set(r, c, dark, reserve) =
        modules.[r, c] <- dark
        if reserve then reserved.[r, c] <- true

    member _.Reserve(r, c) = reserved.[r, c] <- true

// ============================================================================
// Function pattern placement
// ============================================================================

// ------ Finder patterns -------------------------------------------------------
//
// Three 7×7 finder patterns sit at the top-left, top-right, and bottom-left
// corners.  Each is a solid border of dark modules, a ring of light, and a 3×3
// dark core:
//
//   ■ ■ ■ ■ ■ ■ ■
//   ■ □ □ □ □ □ ■
//   ■ □ ■ ■ ■ □ ■
//   ■ □ ■ ■ ■ □ ■
//   ■ □ ■ ■ ■ □ ■
//   ■ □ □ □ □ □ ■
//   ■ ■ ■ ■ ■ ■ ■

let private placeFinder (g: WorkGrid) (top: int) (left: int) =
    for dr in 0 .. 6 do
        for dc in 0 .. 6 do
            let onBorder = dr = 0 || dr = 6 || dc = 0 || dc = 6
            let inCore   = dr >= 2 && dr <= 4 && dc >= 2 && dc <= 4
            g.Set(top + dr, left + dc, onBorder || inCore, true)

// ------ Alignment patterns ----------------------------------------------------
//
// A 5×5 pattern with a solid border, light ring, and dark center:
//
//   ■ ■ ■ ■ ■
//   ■ □ □ □ ■
//   ■ □ ■ □ ■
//   ■ □ □ □ ■
//   ■ ■ ■ ■ ■

let private placeAlignment (g: WorkGrid) (row: int) (col: int) =
    for dr in -2 .. 2 do
        for dc in -2 .. 2 do
            let onBorder = abs dr = 2 || abs dc = 2
            let isCenter = dr = 0 && dc = 0
            g.Set(row + dr, col + dc, onBorder || isCenter, true)

let private placeAllAlignments (g: WorkGrid) (version: int) =
    let positions = alignmentPositions.[version - 1]
    for row in positions do
        for col in positions do
            let r = int row
            let c = int col
            // Skip positions that overlap the finder pattern areas
            if not g.Reserved.[r, c] then
                placeAlignment g r c

// ------ Timing patterns -------------------------------------------------------
//
// Alternating dark/light rows/columns at row 6 and column 6, running between
// the finder patterns.

let private placeTiming (g: WorkGrid) =
    let sz = g.Size
    for c in 8 .. sz - 9 do g.Set(6, c, c % 2 = 0, true)
    for r in 8 .. sz - 9 do g.Set(r, 6, r % 2 = 0, true)

// ------ Format information reservation ----------------------------------------
//
// 15 bits of format information appear in two copies:
//   Copy 1: row 8, cols 0-8 (skipping col 6) and col 8, rows 0-8 (skipping row 6)
//   Copy 2: row 8, cols n-8..n-1 and col 8, rows n-7..n-1

let private reserveFormatInfo (g: WorkGrid) =
    let sz = g.Size
    for c in 0 .. 8 do if c <> 6 then g.Reserve(8, c)
    for r in 0 .. 8 do if r <> 6 then g.Reserve(r, 8)
    for r in sz - 7 .. sz - 1 do g.Reserve(r, 8)
    for c in sz - 8 .. sz - 1 do g.Reserve(8, c)

// ------ Dark module -----------------------------------------------------------
//
// A single permanently dark module at (4v+9, 8).

let private placeDarkModule (g: WorkGrid) (version: int) =
    g.Set(4 * version + 9, 8, true, true)

// ============================================================================
// Format information (15-bit word)
// ============================================================================

// The 5-bit data portion of the format word is:
//   bits 4-3: ECC indicator (Table 12)
//   bits 2-0: mask pattern number (0-7)
//
// BCH error correction: divide the data×2^10 by the generator polynomial
// 10100110111 (0x537); the 10-bit remainder is appended.
// Finally XOR with the masking sequence 101010000010010 (0x5412) to ensure
// no all-zero format word can appear.

let private computeFormatBits (ecc: EccLevel) (mask: int) : uint32 =
    let data = (uint32 (EccUtil.indicator ecc) <<< 3) ||| uint32 mask
    let mutable rem = data <<< 10
    for i in 14 .. -1 .. 10 do
        if (rem >>> i) &&& 1u = 1u then rem <- rem ^^^ (0x537u <<< (i - 10))
    ((data <<< 10) ||| (rem &&& 0x3FFu)) ^^^ 0x5412u

/// Write format information into both copies on the grid.
///
/// KEY lesson: bit ordering is MSB-first (f14→f9) left-to-right across
/// row 8 columns 0-5, and LSB-first (f0→f5) top-to-bottom down col 8
/// rows 0-5.  See lessons.md for the full story.
let private writeFormatInfo (g: WorkGrid) (fmt: uint32) =
    let sz = g.Size

    // ── Copy 1 (top-left finder corner) ─────────────────────────────────────
    // Row 8, cols 0-5: f14 down to f9 (MSB first, left-to-right)
    for i in 0 .. 5 do
        g.Modules.[8, i] <- (fmt >>> (14 - i)) &&& 1u = 1u
    g.Modules.[8, 7] <- (fmt >>> 8) &&& 1u = 1u  // f8  (col 6 is timing, skipped)
    g.Modules.[8, 8] <- (fmt >>> 7) &&& 1u = 1u  // f7
    g.Modules.[7, 8] <- (fmt >>> 6) &&& 1u = 1u  // f6  (row 6 is timing, skipped)
    // Col 8, rows 0-5: f0 at row 0 … f5 at row 5 (LSB first, top-to-bottom)
    for i in 0 .. 5 do
        g.Modules.[i, 8] <- (fmt >>> i) &&& 1u = 1u

    // ── Copy 2 (top-right and bottom-left finders) ───────────────────────────
    // Row 8, cols n-1 down to n-8: f0 at col n-1 … f7 at col n-8
    for i in 0 .. 7 do
        g.Modules.[8, sz - 1 - i] <- (fmt >>> i) &&& 1u = 1u
    // Col 8, rows n-7 to n-1: f8 at row n-7 … f14 at row n-1
    for i in 8 .. 14 do
        g.Modules.[sz - 15 + i, 8] <- (fmt >>> i) &&& 1u = 1u

// ============================================================================
// Version information (v7+)
// ============================================================================

// Versions 7 and above embed a 18-bit version information word in two 6×3
// blocks (one near the top-right finder, one near the bottom-left finder).
// BCH polynomial: x^12 + x^11 + x^10 + x^9 + x^8 + x^5 + x^2 + 1 (0x1F25).

let private reserveVersionInfo (g: WorkGrid) (version: int) =
    if version >= 7 then
        let sz = g.Size
        for r in 0 .. 5 do
            for dc in 0 .. 2 do
                g.Reserve(r, sz - 11 + dc)
        for dr in 0 .. 2 do
            for c in 0 .. 5 do
                g.Reserve(sz - 11 + dr, c)

let private computeVersionBits (version: int) : uint32 =
    let v = uint32 version
    let mutable rem = v <<< 12
    for i in 17 .. -1 .. 12 do
        if (rem >>> i) &&& 1u = 1u then rem <- rem ^^^ (0x1F25u <<< (i - 12))
    (v <<< 12) ||| (rem &&& 0xFFFu)

let private writeVersionInfo (g: WorkGrid) (version: int) =
    if version >= 7 then
        let sz   = g.Size
        let bits = computeVersionBits version
        for i in 0u .. 17u do
            let dark = (bits >>> int i) &&& 1u = 1u
            let a    = 5 - int (i / 3u)
            let b    = sz - 9 - int (i % 3u)
            g.Modules.[a, b] <- dark
            g.Modules.[b, a] <- dark

// ============================================================================
// Build the initial work grid (all function patterns, no data yet)
// ============================================================================

let private buildGrid (version: int) : WorkGrid =
    let sz = symbolSize version
    let g  = WorkGrid(sz)

    // Three finder patterns at the three corners
    placeFinder g 0 0           // top-left
    placeFinder g 0 (sz - 7)    // top-right
    placeFinder g (sz - 7) 0    // bottom-left

    // Separator rows/cols (light border around each finder)
    for i in 0 .. 7 do
        g.Set(7, i, false, true);         g.Set(i, 7, false, true)       // TL
        g.Set(7, sz - 1 - i, false, true); g.Set(i, sz - 8, false, true) // TR
        g.Set(sz - 8, i, false, true);    g.Set(sz - 1 - i, 7, false, true) // BL

    placeTiming g
    placeAllAlignments g version
    reserveFormatInfo g
    reserveVersionInfo g version
    placeDarkModule g version
    g

// ============================================================================
// Data placement — zigzag (two-column snake from bottom-right)
// ============================================================================

// Data bits are placed in a two-column-wide snake pattern starting at the
// bottom-right corner, snaking upward until it reaches the top, then moving
// left by two columns and snaking downward, and so on.  Column 6 (the vertical
// timing strip) is treated as if it doesn't exist, so the snake skips it.

let private placeBits (g: WorkGrid) (codewords: byte[]) (version: int) =
    let sz   = g.Size
    let bits =
        [|
            for cw in codewords do
                for b in 7 .. -1 .. 0 do
                    yield (int cw >>> b) &&& 1 = 1
            for _ in 1 .. numRemainderBits version do
                yield false
        |]

    let mutable bitIdx = 0
    let mutable goUp   = true
    let mutable col    = sz - 1

    let mutable running = true
    while running do
        for vi in 0 .. sz - 1 do
            let row = if goUp then sz - 1 - vi else vi
            for dc in 0 .. 1 do
                let c = col - dc
                if c >= 0 && c <> 6 && not g.Reserved.[row, c] then
                    if bitIdx < bits.Length then
                        g.Modules.[row, c] <- bits.[bitIdx]
                    bitIdx <- bitIdx + 1
        goUp <- not goUp
        if col < 2 then running <- false
        else
            col <- col - 2
            if col = 6 then col <- 5

// ============================================================================
// Masking
// ============================================================================

// Eight mask patterns XOR with non-reserved modules.  For each pattern we
// compute a penalty score (four ISO rules) and choose the lowest.

let private maskCondition (mask: int) (r: int) (c: int) =
    match mask with
    | 0 -> (r + c) % 2 = 0
    | 1 -> r % 2 = 0
    | 2 -> c % 3 = 0
    | 3 -> (r + c) % 3 = 0
    | 4 -> (r / 2 + c / 3) % 2 = 0
    | 5 -> (r * c) % 2 + (r * c) % 3 = 0
    | 6 -> ((r * c) % 2 + (r * c) % 3) % 2 = 0
    | 7 -> ((r + c) % 2 + (r * c) % 3) % 2 = 0
    | _ -> false

/// Apply mask pattern to all non-reserved modules; return a new module grid.
let private applyMask (modules: bool[,]) (reserved: bool[,]) (sz: int) (mask: int) =
    let result = Array2D.copy modules
    for r in 0 .. sz - 1 do
        for c in 0 .. sz - 1 do
            if not reserved.[r, c] then
                result.[r, c] <- modules.[r, c] <> maskCondition mask r c
    result

/// Compute the ISO 18004 penalty score for the given module grid.
let private computePenalty (modules: bool[,]) (sz: int) =
    let mutable penalty = 0u

    // ── Rule 1: runs of ≥ 5 same-color modules in any row or column ─────────
    // Penalty = (run_length − 2) per qualifying run.
    for a in 0 .. sz - 1 do
        for horiz in [| true; false |] do
            let mutable run  = 1u
            let mutable prev = if horiz then modules.[a, 0] else modules.[0, a]
            for i in 1 .. sz - 1 do
                let cur = if horiz then modules.[a, i] else modules.[i, a]
                if cur = prev then run <- run + 1u
                else
                    if run >= 5u then penalty <- penalty + run - 2u
                    run  <- 1u
                    prev <- cur
            if run >= 5u then penalty <- penalty + run - 2u

    // ── Rule 2: 2×2 blocks of same color ────────────────────────────────────
    // Penalty = 3 per 2×2 square.
    for r in 0 .. sz - 2 do
        for c in 0 .. sz - 2 do
            let d = modules.[r, c]
            if d = modules.[r, c + 1] && d = modules.[r + 1, c] && d = modules.[r + 1, c + 1] then
                penalty <- penalty + 3u

    // ── Rule 3: finder-like patterns ────────────────────────────────────────
    // Each occurrence of the pattern 1011101 with 4 quiet-zone zeros on either
    // side adds 40 to the penalty.
    let p1 = [| 1; 0; 1; 1; 1; 0; 1; 0; 0; 0; 0 |]
    let p2 = [| 0; 0; 0; 0; 1; 0; 1; 1; 1; 0; 1 |]
    for a in 0 .. sz - 1 do
        for b in 0 .. sz - 12 do
            let mutable mh1 = true
            let mutable mh2 = true
            let mutable mv1 = true
            let mutable mv2 = true
            for k in 0 .. 10 do
                let bh = if modules.[a, b + k] then 1 else 0
                let bv = if modules.[b + k, a] then 1 else 0
                if bh <> p1.[k] then mh1 <- false
                if bh <> p2.[k] then mh2 <- false
                if bv <> p1.[k] then mv1 <- false
                if bv <> p2.[k] then mv2 <- false
            if mh1 then penalty <- penalty + 40u
            if mh2 then penalty <- penalty + 40u
            if mv1 then penalty <- penalty + 40u
            if mv2 then penalty <- penalty + 40u

    // ── Rule 4: dark module ratio deviation from 50 % ───────────────────────
    // Count dark modules; compute nearest 5%-step deviation from 50%.
    let dark =
        let mutable count = 0u
        for r in 0 .. sz - 1 do
            for c in 0 .. sz - 1 do
                if modules.[r, c] then count <- count + 1u
        count
    let total   = float (sz * sz)
    let ratio   = float dark / total * 100.0
    let prev5   = uint32 (floor (ratio / 5.0)) * 5u
    let a       = if prev5 > 50u then prev5 - 50u else 50u - prev5
    let b       = if prev5 + 5u > 50u then prev5 + 5u - 50u else 50u - (prev5 + 5u)
    penalty <- penalty + (min a b / 5u) * 10u

    penalty

// ============================================================================
// Version selection
// ============================================================================

// Walk versions 1–40.  For each version check whether the encoded bit stream
// (mode indicator + char count + data) fits within the version's data capacity.

let private selectVersion (input: string) (ecc: EccLevel) =
    let mode    = selectMode input
    let byteLen = uint32 (Text.Encoding.UTF8.GetByteCount(input))

    let found =
        [1 .. 40] |> List.tryFind (fun v ->
            let capacity = numDataCodewords v ecc
            let dataBits =
                match mode with
                | EncodingMode.Byte ->
                    byteLen * 8u
                | EncodingMode.Numeric ->
                    let n = uint32 (Seq.length input)
                    (n * 10u + 2u) / 3u
                | EncodingMode.Alphanumeric ->
                    let n = uint32 (Seq.length input)
                    (n * 11u + 1u) / 2u
            let bitsNeeded = uint32 (4 + charCountBits mode v) + dataBits
            let cwNeeded   = (bitsNeeded + 7u) / 8u
            int cwNeeded <= capacity)

    match found with
    | Some v -> Ok v
    | None   ->
        Error (InputTooLong (sprintf "Input (%d chars, ECC=%A) exceeds version-40 capacity." input.Length ecc))

// ============================================================================
// Public API
// ============================================================================

/// Version string.
[<Literal>]
let VERSION = "0.1.0"

/// Encode a UTF-8 string into a QR Code ``ModuleGrid``.
///
/// Returns a ``(4V+17) × (4V+17)`` boolean grid where ``true`` = dark module.
/// Automatically selects the smallest version (1–40) that can hold the input
/// at the given ECC level.  Chooses the mask pattern with the lowest ISO 18004
/// penalty score.
///
/// ## Errors
///
/// Returns ``InputTooLong`` if the input exceeds version-40 capacity.
///
/// ## Example
///
///     match QRCode.encode "HELLO WORLD" EccLevel.M with
///     | Ok grid  -> printfn "Size: %d × %d" grid.Rows grid.Cols  // 21 × 21
///     | Error e  -> printfn "Error: %A" e
let encode (input: string) (ecc: EccLevel) : Result<ModuleGrid, QRCodeError> =
    // Guard: version-40 maximum is 7089 numeric chars / 4296 alphanumeric / 2953 bytes.
    if input.Length > 7089 then
        Error (InputTooLong (sprintf "Input byte length %d exceeds 7089." input.Length))
    else

    selectVersion input ecc
    |> Result.map (fun version ->
        let sz          = symbolSize version
        let dataCw      = buildDataCodewords input version ecc
        let blocks      = computeBlocks dataCw version ecc
        let interleaved = interleaveBlocks blocks
        let grid        = buildGrid version
        placeBits grid interleaved version

        // Evaluate 8 masks; pick the one with the lowest penalty score.
        let mutable bestMask    = 0
        let mutable bestPenalty = UInt32.MaxValue

        for m in 0 .. 7 do
            let masked = applyMask grid.Modules grid.Reserved sz m
            let fmt    = computeFormatBits ecc m
            // Build a temporary grid to score format info + mask
            let tmp    = WorkGrid(sz)
            for r in 0 .. sz - 1 do
                for c in 0 .. sz - 1 do
                    tmp.Modules.[r, c]  <- masked.[r, c]
                    tmp.Reserved.[r, c] <- grid.Reserved.[r, c]
            writeFormatInfo tmp fmt
            let p = computePenalty tmp.Modules sz
            if p < bestPenalty then
                bestPenalty <- p
                bestMask    <- m

        // Finalize: apply chosen mask + write format/version info
        let finalMods = applyMask grid.Modules grid.Reserved sz bestMask
        let finalGrid = WorkGrid(sz)
        for r in 0 .. sz - 1 do
            for c in 0 .. sz - 1 do
                finalGrid.Modules.[r, c]  <- finalMods.[r, c]
                finalGrid.Reserved.[r, c] <- grid.Reserved.[r, c]
        writeFormatInfo finalGrid (computeFormatBits ecc bestMask)
        writeVersionInfo finalGrid version

        // Convert Array2D to jagged array (ModuleGrid.Modules is bool[] array)
        let modulesArr =
            [| for r in 0 .. sz - 1 ->
                [| for c in 0 .. sz - 1 -> finalGrid.Modules.[r, c] |] |]

        {
            Rows        = sz
            Cols        = sz
            Modules     = modulesArr
            ModuleShape = ModuleShape.Square
        }
    )
