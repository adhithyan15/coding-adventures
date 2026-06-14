/// MicroQR.fs — ISO/IEC 18004:2015 Annex E-compliant Micro QR Code encoder for F#
///
/// Micro QR Code is the compact sibling of standard QR Code, designed for
/// surface-mount component labels, circuit board markings, and any application
/// where even the smallest standard QR symbol (21×21) is too large.
///
/// ## Symbol sizes
///
/// ```
/// M1: 11×11   M2: 13×13   M3: 15×15   M4: 17×17
/// formula: size = 2 × version_number + 9
/// ```
///
/// ## Key differences from regular QR Code
///
/// | Feature              | Regular QR            | Micro QR              |
/// |---------------------|-----------------------|-----------------------|
/// | Finder patterns      | Three 7×7 corners     | One 7×7 top-left only |
/// | Timing row/col       | Row 6 / col 6         | Row 0 / col 0         |
/// | Mask patterns        | 8 (patterns 0–7)      | 4 (patterns 0–3)      |
/// | Format XOR mask      | 0x5412                | 0x4445                |
/// | Format info copies   | Two                   | One (L-shaped strip)  |
/// | Quiet zone           | 4 modules             | 2 modules             |
/// | Mode indicator bits  | Always 4              | 0–3 (symbol-dependent)|
/// | RS interleaving      | Multi-block possible  | Always single block   |
///
/// ## Encoding pipeline
///
/// ```
/// input string
///   → auto-select smallest symbol (M1..M4) and encoding mode
///   → build bit stream (mode indicator + char count + data + terminator + padding)
///   → Reed-Solomon ECC  (GF(256)/0x11D, b=0, single block)
///   → initialize grid   (finder, L-shaped separator, timing at row0/col0, format reserved)
///   → zigzag placement  (two-column snake from bottom-right, skipping reserved)
///   → evaluate 4 masks, pick lowest penalty score
///   → write format information (15-bit BCH word XOR 0x4445, L-shaped strip)
///   → ModuleGrid
/// ```
///
/// ## M1 peculiarity: the half-codeword
///
/// M1 has 20 data-capacity bits — not a multiple of 8. The standard treats
/// this as 2.5 codewords: two full bytes plus one 4-bit nibble. The RS encoder
/// receives 3 bytes where byte[2] carries data in its upper 4 bits and zeroes
/// in the lower 4. This means the bit-stream extraction for M1 only takes 4
/// bits from the last data codeword.
///
/// ## Reed-Solomon over GF(256)
///
/// Generator polynomial: g(x) = (x + α⁰)(x + α¹)···(x + α^{n-1}) where
/// α = 2 and the field is GF(256) reduced by the primitive polynomial 0x11D.
/// The b=0 convention means the first root is α^0 = 1.
///
/// All generator polynomials for the codeword counts used by Micro QR
/// (2, 5, 6, 8, 10, 14) are pre-computed constants matching the Java port.

module CodingAdventures.MicroQR

open System
open CodingAdventures.Barcode2D

// ============================================================================
// Version
// ============================================================================

/// Package version string.
let [<Literal>] Version = "0.1.0"

// ============================================================================
// Public types
// ============================================================================

/// Error correction level for Micro QR Code.
///
/// Not all levels are available for every symbol:
///
/// ```
/// Level     | Available in    | Recovery
/// ----------|-----------------|---------------------
/// Detection | M1 only         | detects errors only
/// L         | M2, M3, M4      | ~7% of codewords
/// M         | M2, M3, M4      | ~15% of codewords
/// Q         | M4 only         | ~25% of codewords
/// ```
///
/// Level H (high) is not defined for any Micro QR symbol.
type ECCLevel = Detection | L | M | Q

/// Options for the Micro QR encoder.
///
/// All fields are optional:
///   - Symbol:      fix the symbol version ("M1"/"M2"/"M3"/"M4") or None for auto
///   - ECCLevel:    fix the ECC level or None for auto (smallest symbol's default)
///   - MaskPattern: fix the mask (0–3) or None for auto (lowest-penalty)
type MicroQROptions =
    {
        /// Force a specific symbol version. None = auto-select smallest that fits.
        Symbol: string option
        /// Force a specific ECC level. None = auto-select.
        ECCLevel: ECCLevel option
        /// Force a specific mask pattern (0–3). None = auto-select lowest penalty.
        MaskPattern: int option
    }

/// Default options — fully automatic selection.
let defaultOptions = { Symbol = None; ECCLevel = None; MaskPattern = None }

/// Errors that can occur during Micro QR encoding.
///
/// The ``Result<ModuleGrid, MicroQRError>`` return type forces callers to
/// handle these explicitly. Use ``match result with Ok g -> ... | Error e -> ...``.
type MicroQRError =
    /// Input string is too long to fit in any M1–M4 symbol at the requested
    /// version/ECC combination.
    | InputTooLong of string
    /// The requested ECC level is not defined for the chosen symbol.
    /// Example: "Q" is only valid for M4; using it with M1/M2/M3 fails here.
    | InvalidECCLevel of string
    /// The options combination is impossible (e.g. version string not M1–M4).
    | InvalidOptions of string

// ============================================================================
// Internal types
// ============================================================================

/// Encoding mode — determines how characters are packed into bits.
///
/// Selection priority (most compact first): Numeric > Alphanumeric > Byte.
///
/// ```
/// Mode          | Characters allowed
/// --------------|-------------------------------------------
/// Numeric       | 0–9 only
/// Alphanumeric  | 0–9, A–Z, space, $%*+-./:
/// Byte          | any UTF-8 byte sequence
/// ```
type private EncodingMode = Numeric | Alphanumeric | Byte

/// Compile-time constants for one (version, ECC) combination.
///
/// There are exactly 8 valid Micro QR configurations.  All values come
/// directly from ISO 18004:2015 Annex E and are embedded as constants to
/// guarantee identical behaviour across all language ports.
type private SymbolConfig =
    {
        /// Symbol version label: M1, M2, M3, or M4.
        Version: string
        /// ECC level for this configuration.
        Ecc: ECCLevel
        /// 3-bit symbol indicator placed in format information (0–7).
        SymbolIndicator: int
        /// Symbol side length in modules (11, 13, 15, or 17).
        Size: int
        /// Number of data codewords. M1: 3 (but last is only 4 bits).
        DataCw: int
        /// Number of ECC codewords.
        EccCw: int
        /// Maximum numeric-mode characters (0 = mode not supported).
        NumericCap: int
        /// Maximum alphanumeric-mode characters (0 = mode not supported).
        AlphaCap: int
        /// Maximum byte-mode characters (0 = mode not supported).
        ByteCap: int
        /// Number of terminator zero-bits (3, 5, 7, or 9).
        TerminatorBits: int
        /// Mode indicator field width (0 for M1, 1 for M2, 2 for M3, 3 for M4).
        ModeIndicatorBits: int
        /// Character count field width for numeric mode.
        CcBitsNumeric: int
        /// Character count field width for alphanumeric mode.
        CcBitsAlpha: int
        /// Character count field width for byte mode.
        CcBitsByte: int
        /// True only for M1: last data codeword is 4 bits, total = 20 bits.
        M1HalfCw: bool
    }

// ============================================================================
// Symbol configuration table
// ============================================================================

/// All 8 valid Micro QR symbol configurations from ISO 18004:2015 Annex E.
///
/// Ordered from smallest (M1/Detection) to largest (M4/Q) so the auto-selection
/// loop can stop at the first configuration that fits the input.
///
/// ```
/// Symbol | ECC | Numeric | Alpha | Byte | Data CWs | ECC CWs
/// -------|-----|---------|-------|------|----------|---------
/// M1     | Det |       5 |     — |    — |        3 |       2
/// M2     | L   |      10 |     6 |    4 |        5 |       5
/// M2     | M   |       8 |     5 |    3 |        4 |       6
/// M3     | L   |      23 |    14 |    9 |       11 |       6
/// M3     | M   |      18 |    11 |    7 |        9 |       8
/// M4     | L   |      35 |    21 |   15 |       16 |       8
/// M4     | M   |      30 |    18 |   13 |       14 |      10
/// M4     | Q   |      21 |    13 |    9 |       10 |      14
/// ```
let private symbolConfigs : SymbolConfig[] =
    [|
        // M1 / Detection — numeric only, no mode indicator
        { Version="M1"; Ecc=Detection; SymbolIndicator=0; Size=11; DataCw=3; EccCw=2
          NumericCap=5; AlphaCap=0; ByteCap=0; TerminatorBits=3; ModeIndicatorBits=0
          CcBitsNumeric=3; CcBitsAlpha=0; CcBitsByte=0; M1HalfCw=true }
        // M2 / L
        { Version="M2"; Ecc=L; SymbolIndicator=1; Size=13; DataCw=5; EccCw=5
          NumericCap=10; AlphaCap=6; ByteCap=4; TerminatorBits=5; ModeIndicatorBits=1
          CcBitsNumeric=4; CcBitsAlpha=3; CcBitsByte=4; M1HalfCw=false }
        // M2 / M
        { Version="M2"; Ecc=M; SymbolIndicator=2; Size=13; DataCw=4; EccCw=6
          NumericCap=8; AlphaCap=5; ByteCap=3; TerminatorBits=5; ModeIndicatorBits=1
          CcBitsNumeric=4; CcBitsAlpha=3; CcBitsByte=4; M1HalfCw=false }
        // M3 / L
        { Version="M3"; Ecc=L; SymbolIndicator=3; Size=15; DataCw=11; EccCw=6
          NumericCap=23; AlphaCap=14; ByteCap=9; TerminatorBits=7; ModeIndicatorBits=2
          CcBitsNumeric=5; CcBitsAlpha=4; CcBitsByte=4; M1HalfCw=false }
        // M3 / M
        { Version="M3"; Ecc=M; SymbolIndicator=4; Size=15; DataCw=9; EccCw=8
          NumericCap=18; AlphaCap=11; ByteCap=7; TerminatorBits=7; ModeIndicatorBits=2
          CcBitsNumeric=5; CcBitsAlpha=4; CcBitsByte=4; M1HalfCw=false }
        // M4 / L
        { Version="M4"; Ecc=L; SymbolIndicator=5; Size=17; DataCw=16; EccCw=8
          NumericCap=35; AlphaCap=21; ByteCap=15; TerminatorBits=9; ModeIndicatorBits=3
          CcBitsNumeric=6; CcBitsAlpha=5; CcBitsByte=5; M1HalfCw=false }
        // M4 / M
        { Version="M4"; Ecc=M; SymbolIndicator=6; Size=17; DataCw=14; EccCw=10
          NumericCap=30; AlphaCap=18; ByteCap=13; TerminatorBits=9; ModeIndicatorBits=3
          CcBitsNumeric=6; CcBitsAlpha=5; CcBitsByte=5; M1HalfCw=false }
        // M4 / Q
        { Version="M4"; Ecc=Q; SymbolIndicator=7; Size=17; DataCw=10; EccCw=14
          NumericCap=21; AlphaCap=13; ByteCap=9; TerminatorBits=9; ModeIndicatorBits=3
          CcBitsNumeric=6; CcBitsAlpha=5; CcBitsByte=5; M1HalfCw=false }
    |]

// ============================================================================
// Format information table
// ============================================================================

/// All 32 pre-computed format words (after XOR with 0x4445).
///
/// Indexed as ``formatTable.[symbolIndicator].[maskPattern]``.
///
/// The 15-bit format word structure:
/// ```
///   [symbol_indicator (3b)] [mask_pattern (2b)] [BCH-10 remainder]
/// ```
/// XOR-masked with 0x4445 (Micro QR specific, not 0x5412 like regular QR).
/// This prevents a Micro QR symbol from being misread as a regular QR symbol.
///
/// ```
/// Symbol+ECC  | Mask 0 | Mask 1 | Mask 2 | Mask 3
/// ------------|--------|--------|--------|--------
/// M1  (000)   | 0x4445 | 0x4172 | 0x4E2B | 0x4B1C
/// M2-L (001)  | 0x5528 | 0x501F | 0x5F46 | 0x5A71
/// M2-M (010)  | 0x6649 | 0x637E | 0x6C27 | 0x6910
/// M3-L (011)  | 0x7764 | 0x7253 | 0x7D0A | 0x783D
/// M3-M (100)  | 0x06DE | 0x03E9 | 0x0CB0 | 0x0987
/// M4-L (101)  | 0x17F3 | 0x12C4 | 0x1D9D | 0x18AA
/// M4-M (110)  | 0x24B2 | 0x2185 | 0x2EDC | 0x2BEB
/// M4-Q (111)  | 0x359F | 0x30A8 | 0x3FF1 | 0x3AC6
/// ```
let private formatTable : int[][] =
    [|
        [| 0x4445; 0x4172; 0x4E2B; 0x4B1C |]  // M1
        [| 0x5528; 0x501F; 0x5F46; 0x5A71 |]  // M2-L
        [| 0x6649; 0x637E; 0x6C27; 0x6910 |]  // M2-M
        [| 0x7764; 0x7253; 0x7D0A; 0x783D |]  // M3-L
        [| 0x06DE; 0x03E9; 0x0CB0; 0x0987 |]  // M3-M
        [| 0x17F3; 0x12C4; 0x1D9D; 0x18AA |]  // M4-L
        [| 0x24B2; 0x2185; 0x2EDC; 0x2BEB |]  // M4-M
        [| 0x359F; 0x30A8; 0x3FF1; 0x3AC6 |]  // M4-Q
    |]

// ============================================================================
// Reed-Solomon generator polynomials
// ============================================================================

/// Monic RS generator polynomials for GF(256)/0x11D with b=0 convention.
///
/// The generator polynomial of degree n is:
/// ```
///   g(x) = (x + α⁰)(x + α¹)···(x + α^{n-1})
/// ```
/// where α = 2 (the primitive element of GF(256)/0x11D).
///
/// Array length is n+1 (leading monic coefficient 0x01 included).
/// Only the counts {2, 5, 6, 8, 10, 14} are needed for Micro QR.
///
/// These pre-computed constants match the Java port exactly.
let private getGenerator (eccCount: int) : int[] =
    match eccCount with
    | 2  -> [| 0x01; 0x03; 0x02 |]
    | 5  -> [| 0x01; 0x1f; 0xf6; 0x44; 0xd9; 0x68 |]
    | 6  -> [| 0x01; 0x3f; 0x4e; 0x17; 0x9b; 0x05; 0x37 |]
    | 8  -> [| 0x01; 0x63; 0x0d; 0x60; 0x6d; 0x5b; 0x10; 0xa2; 0xa3 |]
    | 10 -> [| 0x01; 0xf6; 0x75; 0xa8; 0xd0; 0xc3; 0xe3; 0x36; 0xe1; 0x3c; 0x45 |]
    | 14 -> [| 0x01; 0xf6; 0x9a; 0x60; 0x97; 0x8a; 0xf1; 0xa4; 0xa1; 0x8e; 0xfc; 0x7a; 0x52; 0xad; 0xac |]
    | n  -> failwithf "No generator for ecc_count=%d" n

// ============================================================================
// GF(256) arithmetic
// ============================================================================

/// GF(256) log table: LOG.[x] = i such that 2^i = x in GF(256)/0x11D.
/// LOG.[0] is left at 0 (log of zero is undefined; gf256Mul short-circuits).
let private gf256Log : int[] =
    let tbl = Array.zeroCreate 256
    let mutable v = 1
    for i in 0 .. 254 do
        tbl.[v] <- i
        v <- v <<< 1
        if v >= 256 then v <- v ^^^ 0x11D
    tbl

/// GF(256) antilog table: ALOG.[i] = 2^i in GF(256)/0x11D, with wrap at 255.
let private gf256ALog : int[] =
    let tbl = Array.zeroCreate 256
    let mutable v = 1
    for i in 0 .. 254 do
        tbl.[i] <- v
        v <- v <<< 1
        if v >= 256 then v <- v ^^^ 0x11D
    tbl.[255] <- 1
    tbl

/// GF(256) multiplication using log/antilog tables.
///
/// ```
/// mul(a, b) = α^(log(a) + log(b))  with α = 2
/// ```
///
/// Short-circuits to 0 if either operand is 0 (since log(0) is undefined).
let private gf256Mul (a: int) (b: int) : int =
    if a = 0 || b = 0 then 0
    else gf256ALog.[(gf256Log.[a] + gf256Log.[b]) % 255]

// ============================================================================
// Reed-Solomon encoder
// ============================================================================

/// Compute ECC bytes using the LFSR polynomial division algorithm over GF(256).
///
/// Returns the remainder of D(x)·x^n mod G(x) — this is the RS "remainder"
/// method, equivalent to the syndrome-based approach but much simpler to
/// implement.
///
/// Algorithm (LFSR / polynomial long division):
/// ```
///   ecc = [0] × n
///   for each data byte b:
///       feedback = b XOR ecc[0]
///       shift ecc left by one position (discard ecc[0], append 0)
///       for i in 0 .. n-1:
///           ecc[i] ^= GF256.mul(generator[i+1], feedback)
///   result = ecc
/// ```
///
/// Parameters:
///   data      — data codewords as byte values (0–255)
///   generator — monic generator polynomial (length = eccCount + 1)
///
/// Returns: eccCount ECC codeword values.
let private rsEncode (data: byte[]) (generator: int[]) : byte[] =
    let n = generator.Length - 1  // number of ECC codewords
    let rem = Array.zeroCreate n  // shift register starts all-zero

    for b in data do
        let fb = (int b) ^^^ rem.[0]  // feedback = data byte XOR leading register
        // Shift register left: rem[0] drops off, rem[n-1] fills with 0
        Array.blit rem 1 rem 0 (n - 1)
        rem.[n - 1] <- 0
        if fb <> 0 then
            for i in 0 .. n - 1 do
                rem.[i] <- rem.[i] ^^^ (gf256Mul generator.[i + 1] fb)

    rem |> Array.map byte

// ============================================================================
// Encoding mode helpers
// ============================================================================

/// The 45-character alphanumeric set shared with regular QR Code.
///
/// Characters are assigned indices 0–44. Pairs are packed into 11 bits as:
///   first_index × 45 + second_index
/// A trailing single character uses 6 bits.
let private alphanumChars = "0123456789ABCDEFGHIJKLMNOPQRSTUVWXYZ $%*+-./:";

/// Returns true if every character in ``s`` is an ASCII digit 0–9.
let private isNumeric (s: string) : bool =
    s |> Seq.forall (fun c -> c >= '0' && c <= '9')

/// Returns true if every character in ``s`` is in the 45-char alphanumeric set.
let private isAlphanumeric (s: string) : bool =
    s |> Seq.forall (fun c -> alphanumChars.IndexOf(c) >= 0)

/// Select the most compact encoding mode supported by the given config.
///
/// Selection priority: Numeric > Alphanumeric > Byte.
/// Returns None if no supported mode can encode the input.
let private trySelectMode (input: string) (cfg: SymbolConfig) : EncodingMode option =
    if cfg.CcBitsNumeric > 0 && isNumeric input then
        Some Numeric
    elif cfg.AlphaCap > 0 && isAlphanumeric input then
        Some Alphanumeric
    elif cfg.ByteCap > 0 then
        Some Byte
    else
        None

// ============================================================================
// Bit writer
// ============================================================================

/// Accumulates bits MSB-first, then converts to a byte array or bit array.
///
/// QR and Micro QR use big-endian bit ordering within each codeword: the most
/// significant bit is placed first. This is why we iterate from ``count-1``
/// down to ``0`` when appending.
///
/// Example: ``write 0b101 3`` appends bits [1; 0; 1].
type private BitWriter() =
    let mutable bits : int list = []
    let mutable count = 0

    /// Append the ``bitCount`` LSBs of ``value``, MSB first.
    member _.Write (value: int) (bitCount: int) =
        for i = bitCount - 1 downto 0 do
            bits <- ((value >>> i) &&& 1) :: bits
            count <- count + 1

    /// Number of bits written so far.
    member _.BitLen = count

    /// Return the bit stream as a plain int array (each element 0 or 1).
    member _.ToBitArray() =
        bits |> List.rev |> List.toArray

    /// Convert the bit stream to a byte array (MSB-first, zero-padded on right).
    member _.ToBytes() =
        let arr = bits |> List.rev |> List.toArray
        let nBytes = (arr.Length + 7) / 8
        let result = Array.zeroCreate<byte> nBytes
        for i in 0 .. arr.Length - 1 do
            if arr.[i] = 1 then
                result.[i / 8] <- result.[i / 8] ||| byte (1 <<< (7 - (i % 8)))
        result

// ============================================================================
// Data encoding
// ============================================================================

/// Encode a numeric string: groups of 3 → 10 bits; pair → 7 bits; single → 4 bits.
///
/// Example: "12345" → groups "123" (10-bit: 123) and "45" (7-bit: 45).
///
/// This is identical to standard QR Code numeric encoding.
let private encodeNumeric (input: string) (w: BitWriter) =
    let mutable i = 0
    while i + 2 < input.Length do
        let v = (int input.[i] - int '0') * 100
                + (int input.[i+1] - int '0') * 10
                + (int input.[i+2] - int '0')
        w.Write v 10
        i <- i + 3
    if i + 1 < input.Length then
        let v = (int input.[i] - int '0') * 10 + (int input.[i+1] - int '0')
        w.Write v 7
        i <- i + 2
    if i < input.Length then
        w.Write (int input.[i] - int '0') 4

/// Encode an alphanumeric string: pairs → 11 bits; single → 6 bits.
///
/// Each pair is packed as: first_index × 45 + second_index.
///
/// This is identical to standard QR Code alphanumeric encoding.
let private encodeAlphanumeric (input: string) (w: BitWriter) =
    let mutable i = 0
    while i + 1 < input.Length do
        let a = alphanumChars.IndexOf(input.[i])
        let b = alphanumChars.IndexOf(input.[i + 1])
        w.Write (a * 45 + b) 11
        i <- i + 2
    if i < input.Length then
        w.Write (alphanumChars.IndexOf(input.[i])) 6

/// Encode byte mode: each UTF-8 byte → 8 bits.
///
/// Multi-byte UTF-8 sequences are treated as individual byte values.
/// The character count field counts bytes, not Unicode code points.
let private encodeByteMode (input: string) (w: BitWriter) =
    for b in Text.Encoding.UTF8.GetBytes(input) do
        w.Write (int b) 8

/// Return the mode indicator value for the given mode and symbol config.
///
/// ```
/// M1 (0 bits): no indicator — only numeric mode exists
/// M2 (1 bit):  0=numeric, 1=alphanumeric
/// M3 (2 bits): 00=numeric, 01=alphanumeric, 10=byte
/// M4 (3 bits): 000=numeric, 001=alphanumeric, 010=byte
/// ```
let private modeIndicatorValue (mode: EncodingMode) (cfg: SymbolConfig) : int =
    match cfg.ModeIndicatorBits with
    | 0 -> 0
    | 1 -> match mode with Numeric -> 0 | _ -> 1
    | 2 -> match mode with Numeric -> 0b00 | Alphanumeric -> 0b01 | Byte -> 0b10
    | 3 -> match mode with Numeric -> 0b000 | Alphanumeric -> 0b001 | Byte -> 0b010
    | _ -> 0

/// Return the character count field width for the given mode and symbol.
///
/// ```
/// Mode         | M1 | M2 | M3 | M4
/// -------------|----|----|----|----|
/// Numeric      |  3 |  4 |  5 |  6
/// Alphanumeric |  — |  3 |  4 |  5
/// Byte         |  — |  — |  4 |  5
/// ```
let private charCountBits (mode: EncodingMode) (cfg: SymbolConfig) : int =
    match mode with
    | Numeric      -> cfg.CcBitsNumeric
    | Alphanumeric -> cfg.CcBitsAlpha
    | Byte         -> cfg.CcBitsByte

/// Build the complete data codeword byte sequence for the given input and config.
///
/// For normal symbols (not M1):
/// ```
///   [mode indicator] [char count] [data bits] [terminator] [byte-align] [0xEC/0x11 fill]
///   → exactly cfg.DataCw bytes
/// ```
///
/// For M1 (M1HalfCw = true):
/// ```
///   Total capacity = 20 bits = 2 full bytes + 4-bit nibble.
///   The RS encoder receives 3 bytes where byte[2] has data in the
///   upper 4 bits and zero in the lower 4 bits.
/// ```
///
/// The alternating pad bytes 0xEC / 0x11 are the standard QR/Micro QR fill
/// pattern. Any value would work, but this is what decoders expect.
let private buildDataCodewords (input: string) (cfg: SymbolConfig) (mode: EncodingMode) : byte[] =
    // Total usable data bits: M1 uses 3 codewords but last is only 4 bits.
    let totalBits =
        if cfg.M1HalfCw then cfg.DataCw * 8 - 4
        else cfg.DataCw * 8

    let w = BitWriter()

    // Mode indicator (0/1/2/3 bits depending on symbol version)
    if cfg.ModeIndicatorBits > 0 then
        w.Write (modeIndicatorValue mode cfg) cfg.ModeIndicatorBits

    // Character count
    let charCount =
        if mode = Byte then Text.Encoding.UTF8.GetByteCount(input)
        else input.Length
    w.Write charCount (charCountBits mode cfg)

    // Encoded data bits
    match mode with
    | Numeric      -> encodeNumeric      input w
    | Alphanumeric -> encodeAlphanumeric input w
    | Byte         -> encodeByteMode     input w

    // Terminator: up to terminatorBits zero bits, truncated at capacity boundary
    let remaining = totalBits - w.BitLen
    if remaining > 0 then
        w.Write 0 (min cfg.TerminatorBits remaining)

    if cfg.M1HalfCw then
        // M1: pack into exactly 20 bits → 3 bytes (last byte: data in upper nibble)
        let arr = w.ToBitArray()
        // Resize to 20 bits (pad with zeros if shorter)
        let padded = Array.init 20 (fun i -> if i < arr.Length then arr.[i] else 0)

        let pack8 offset =
            (padded.[offset]   <<< 7) ||| (padded.[offset+1] <<< 6) |||
            (padded.[offset+2] <<< 5) ||| (padded.[offset+3] <<< 4) |||
            (padded.[offset+4] <<< 3) ||| (padded.[offset+5] <<< 2) |||
            (padded.[offset+6] <<< 1) |||  padded.[offset+7]

        let b0 = byte (pack8 0)
        let b1 = byte (pack8 8)
        // Last byte: data in upper 4 bits, zeros in lower 4
        let b2 = byte ((padded.[16] <<< 7) ||| (padded.[17] <<< 6) |||
                       (padded.[18] <<< 5) ||| (padded.[19] <<< 4))
        [| b0; b1; b2 |]
    else
        // Byte-align: pad to byte boundary with zero bits
        let rem = w.BitLen % 8
        if rem <> 0 then
            w.Write 0 (8 - rem)

        // Fill remaining codewords with alternating 0xEC / 0x11
        let bytes = w.ToBytes()
        let result = Array.zeroCreate<byte> cfg.DataCw
        Array.blit bytes 0 result 0 (min bytes.Length cfg.DataCw)
        let mutable pad = 0xECuy
        for i = bytes.Length to cfg.DataCw - 1 do
            result.[i] <- pad
            pad <- if pad = 0xECuy then 0x11uy else 0xECuy
        result

// ============================================================================
// Grid construction
// ============================================================================

/// Internal mutable work grid used only during encoding.
///
/// ``modules.[row].[col] = true`` means a dark module.
/// ``reserved.[row].[col] = true`` means the module is structural and must
/// not be touched during data placement or masking.
///
/// After encoding is complete, this is converted to an immutable ``ModuleGrid``.
type private WorkGrid(size: int) =
    let modules  = Array2D.create size size false
    let reserved = Array2D.create size size false

    member _.Size = size
    member _.Modules  = modules
    member _.Reserved = reserved

    member _.Set (row: int) (col: int) (dark: bool) (reserve: bool) =
        modules.[row, col] <- dark
        if reserve then reserved.[row, col] <- true

/// Place the 7×7 finder pattern at the top-left corner (rows 0–6, cols 0–6).
///
/// ```
/// ■ ■ ■ ■ ■ ■ ■
/// ■ □ □ □ □ □ ■
/// ■ □ ■ ■ ■ □ ■
/// ■ □ ■ ■ ■ □ ■
/// ■ □ ■ ■ ■ □ ■
/// ■ □ □ □ □ □ ■
/// ■ ■ ■ ■ ■ ■ ■
/// ```
///
/// This is the same 7×7 finder as regular QR Code. The 1:1:3:1:1 ratio of
/// dark-light-dark columns/rows is what scanners use to locate the symbol.
/// Because Micro QR has only one finder (top-left), orientation is unambiguous.
let private placeFinder (g: WorkGrid) =
    for dr in 0 .. 6 do
        for dc in 0 .. 6 do
            let onBorder = dr = 0 || dr = 6 || dc = 0 || dc = 6
            let inCore   = dr >= 2 && dr <= 4 && dc >= 2 && dc <= 4
            g.Set dr dc (onBorder || inCore) true

/// Place the L-shaped separator (light modules at row 7 cols 0–7 and col 7 rows 0–7).
///
/// Unlike regular QR which surrounds all three finders with a full perimeter
/// separator, Micro QR's single finder only needs separation on its bottom and
/// right sides.
///
/// The corner at (row 7, col 7) is covered by both strips — both iterations
/// write ``false`` (light), so the result is correct.
let private placeSeparator (g: WorkGrid) =
    for i in 0 .. 7 do
        g.Set 7 i false true  // bottom strip: row 7, cols 0–7
        g.Set i 7 false true  // right strip:  col 7, rows 0–7

/// Place timing patterns extending from the finder along row 0 and col 0.
///
/// Micro QR places timing at row 0 and col 0 (outer edges of the finder),
/// extending them to the far edge of the symbol. This differs from regular
/// QR Code, which places timing at row 6 and col 6.
///
/// Positions 0–6: already set by the finder pattern.
/// Position 7: separator (always light).
/// Position 8 onward: alternating dark/light, dark at even index.
let private placeTiming (g: WorkGrid) =
    let sz = g.Size
    // Horizontal timing: row 0, cols 8 to size-1
    for c in 8 .. sz - 1 do
        g.Set 0 c (c % 2 = 0) true
    // Vertical timing: col 0, rows 8 to size-1
    for r in 8 .. sz - 1 do
        g.Set r 0 (r % 2 = 0) true

/// Reserve the 15 format information module positions.
///
/// These modules are reserved before data placement so that the zigzag
/// algorithm skips them. They are filled with the actual format word after
/// mask selection.
///
/// ```
/// Row 8, cols 1–8 → bits f14..f7  (MSB f14 at col 1)
/// Col 8, rows 1–7 → bits f6..f0   (f6 at row 7, f0 at row 1)
/// ```
let private reserveFormatInfo (g: WorkGrid) =
    for c in 1 .. 8 do g.Set 8 c false true
    for r in 1 .. 7 do g.Set r 8 false true

/// Write the 15-bit format word into the reserved L-shaped strip.
///
/// Bit f14 (MSB) is placed first. The strip goes rightward along row 8
/// then upward along col 8:
///
/// ```
/// Row 8, col 1  ← f14  (MSB)
/// Row 8, col 2  ← f13
/// ...
/// Row 8, col 8  ← f7
/// Col 8, row 7  ← f6
/// Col 8, row 6  ← f5
/// ...
/// Col 8, row 1  ← f0   (LSB)
/// ```
let private writeFormatInfo (modules: bool[,]) (fmt: int) =
    // Row 8, cols 1–8: bits f14 down to f7
    for i in 0 .. 7 do
        modules.[8, 1 + i] <- ((fmt >>> (14 - i)) &&& 1) = 1
    // Col 8, rows 7 down to 1: bits f6 down to f0
    for i in 0 .. 6 do
        modules.[7 - i, 8] <- ((fmt >>> (6 - i)) &&& 1) = 1

/// Initialize the grid with all structural modules.
///
/// Order: finder → separator → timing → format info reservation.
/// After this call, all reserved modules are set and the remaining modules
/// are available for data placement.
let private buildGrid (cfg: SymbolConfig) : WorkGrid =
    let g = WorkGrid(cfg.Size)
    placeFinder      g
    placeSeparator   g
    placeTiming      g
    reserveFormatInfo g
    g

// ============================================================================
// Data placement (two-column zigzag)
// ============================================================================

/// Place bits from the final codeword stream into the grid via two-column zigzag.
///
/// Scans from the bottom-right corner, moving left two columns at a time,
/// alternating upward and downward directions. Reserved modules are skipped.
///
/// Unlike regular QR, there is no timing column at col 6 to hop over —
/// Micro QR's timing is at col 0, which is reserved and auto-skipped.
///
/// ```
/// col = size - 1   (start at rightmost column)
/// dir = UP
///
/// while col >= 1:
///   for each row in current direction:
///     for sub_col in [col, col-1]:
///       if reserved: skip
///       place bit
///   flip direction
///   col -= 2
/// ```
let private placeBits (g: WorkGrid) (bits: bool[]) =
    let sz = g.Size
    let mutable bitIdx = 0
    let mutable up = true

    let mutable col = sz - 1
    while col >= 1 do
        for vi in 0 .. sz - 1 do
            let row = if up then sz - 1 - vi else vi
            for dc in 0 .. 1 do
                let c = col - dc
                if not g.Reserved.[row, c] then
                    g.Modules.[row, c] <- bitIdx < bits.Length && bits.[bitIdx]
                    bitIdx <- bitIdx + 1
        up <- not up
        col <- col - 2

// ============================================================================
// Masking
// ============================================================================

/// Returns true if mask pattern ``maskIdx`` should flip module (row, col).
///
/// The 4 Micro QR mask conditions (subset of regular QR's 8):
///
/// ```
/// Pattern 0: (row + col) mod 2 == 0
/// Pattern 1: row mod 2 == 0
/// Pattern 2: col mod 3 == 0
/// Pattern 3: (row + col) mod 3 == 0
/// ```
///
/// The more complex patterns 4–7 from regular QR are not used in Micro QR.
let private maskCondition (maskIdx: int) (row: int) (col: int) : bool =
    match maskIdx with
    | 0 -> (row + col) % 2 = 0
    | 1 -> row % 2 = 0
    | 2 -> col % 3 = 0
    | 3 -> (row + col) % 3 = 0
    | _ -> false

/// Apply mask pattern to all non-reserved modules, returning a new 2D array.
///
/// Masking XORs (flips) module values: if the mask condition is true for a
/// non-reserved module, the module's dark/light value is inverted. Structural
/// modules (finder, separator, timing, format info) are never masked.
let private applyMask (modules: bool[,]) (reserved: bool[,]) (sz: int) (maskIdx: int) : bool[,] =
    let result = Array2D.copy modules
    for r in 0 .. sz - 1 do
        for c in 0 .. sz - 1 do
            if not reserved.[r, c] then
                result.[r, c] <- modules.[r, c] <> maskCondition maskIdx r c
    result

// ============================================================================
// Penalty scoring
// ============================================================================

/// Compute the 4-rule penalty score for a masked symbol.
///
/// The four rules penalize patterns that could interfere with scanner detection.
/// All 4 masks are evaluated and the one with the lowest total penalty is chosen.
///
/// ### Rule 1 — Adjacent run penalty
///
/// Scan each row and each column for runs of ≥5 consecutive modules of the same
/// colour. Add ``run_length − 2`` for each qualifying run. (Run of 5 → +3, etc.)
///
/// ### Rule 2 — 2×2 block penalty
///
/// For each 2×2 square with all four modules the same colour, add 3.
///
/// ### Rule 3 — Finder-pattern-like sequences
///
/// Scan all rows and columns for the 11-module sequences
/// ``1 0 1 1 1 0 1 0 0 0 0`` or its reverse ``0 0 0 0 1 0 1 1 1 0 1``.
/// Each occurrence adds 40.
///
/// ### Rule 4 — Dark-module proportion
///
/// Penalises symbols that deviate heavily from 50% dark. The formula uses
/// the nearest multiples of 5 to the dark percentage:
/// ```
/// penalty = min(|prev5 - 50|, |next5 - 50|) / 5 × 10
/// ```
let private computePenalty (modules: bool[,]) (sz: int) : int =
    let mutable penalty = 0

    // ── Rule 1: adjacent same-colour runs of ≥ 5 ─────────────────────────────
    for a in 0 .. sz - 1 do
        // Row a
        let mutable run = 1
        let mutable prev = modules.[a, 0]
        for i in 1 .. sz - 1 do
            let cur = modules.[a, i]
            if cur = prev then
                run <- run + 1
            else
                if run >= 5 then penalty <- penalty + run - 2
                run <- 1
                prev <- cur
        if run >= 5 then penalty <- penalty + run - 2

        // Column a
        run <- 1
        prev <- modules.[0, a]
        for i in 1 .. sz - 1 do
            let cur = modules.[i, a]
            if cur = prev then
                run <- run + 1
            else
                if run >= 5 then penalty <- penalty + run - 2
                run <- 1
                prev <- cur
        if run >= 5 then penalty <- penalty + run - 2

    // ── Rule 2: 2×2 same-colour blocks ───────────────────────────────────────
    for r in 0 .. sz - 2 do
        for c in 0 .. sz - 2 do
            let d = modules.[r, c]
            if d = modules.[r, c+1] && d = modules.[r+1, c] && d = modules.[r+1, c+1] then
                penalty <- penalty + 3

    // ── Rule 3: finder-pattern-like sequences ─────────────────────────────────
    // Only applies if the symbol is at least 11 modules wide/tall.
    // These patterns look like a finder to a scanner and must be minimised.
    if sz >= 11 then
        let p1 = [| 1; 0; 1; 1; 1; 0; 1; 0; 0; 0; 0 |]
        let p2 = [| 0; 0; 0; 0; 1; 0; 1; 1; 1; 0; 1 |]
        let limit = sz - 11
        for a in 0 .. sz - 1 do
            for b in 0 .. limit do
                let mutable mh1, mh2, mv1, mv2 = true, true, true, true
                for k in 0 .. 10 do
                    let bh = if modules.[a, b + k] then 1 else 0
                    let bv = if modules.[b + k, a] then 1 else 0
                    if bh <> p1.[k] then mh1 <- false
                    if bh <> p2.[k] then mh2 <- false
                    if bv <> p1.[k] then mv1 <- false
                    if bv <> p2.[k] then mv2 <- false
                if mh1 then penalty <- penalty + 40
                if mh2 then penalty <- penalty + 40
                if mv1 then penalty <- penalty + 40
                if mv2 then penalty <- penalty + 40

    // ── Rule 4: dark proportion deviation from 50% ────────────────────────────
    // Count dark modules, compute percentage, find nearest multiples of 5.
    let mutable dark = 0
    for r in 0 .. sz - 1 do
        for c in 0 .. sz - 1 do
            if modules.[r, c] then dark <- dark + 1
    let total = sz * sz
    let darkPct = (dark * 100) / total
    let prev5 = (darkPct / 5) * 5
    let next5 = prev5 + 5
    let r4 = min (abs (prev5 - 50)) (abs (next5 - 50))
    penalty <- penalty + (r4 / 5) * 10

    penalty

// ============================================================================
// Symbol selection
// ============================================================================

/// Convert a symbol string option ("M1"/"M2"/"M3"/"M4") to an ECCLevel filter option.
///
/// This function handles parsing the Symbol option field from MicroQROptions,
/// returning an error if the string is invalid.
let private parseSymbolOption (symOpt: string option) : Result<string option, MicroQRError> =
    match symOpt with
    | None -> Ok None
    | Some s ->
        let upper = s.ToUpperInvariant()
        if upper = "M1" || upper = "M2" || upper = "M3" || upper = "M4" then
            Ok (Some upper)
        else
            Error (InvalidOptions (sprintf "Invalid symbol '%s'. Must be M1, M2, M3, or M4." s))

/// Find the smallest symbol configuration that can hold the given input.
///
/// Algorithm:
///   1. Filter configs by version (if specified) and ECC (if specified).
///   2. For each candidate, determine the best encoding mode.
///   3. Check that the input length fits within the mode capacity.
///   4. Return the first (smallest) config that fits.
///
/// Error conditions:
///   - If no config matches the requested version/ECC filter: InvalidECCLevel
///   - If no config can hold the input: InputTooLong
let private selectConfig
        (input: string)
        (versionFilter: string option)
        (eccFilter: ECCLevel option)
        : Result<SymbolConfig * EncodingMode, MicroQRError> =

    let mutable foundMatchingFilter = false

    let rec loop (i: int) =
        if i >= symbolConfigs.Length then
            if not foundMatchingFilter then
                Error (InvalidECCLevel (
                    sprintf "No symbol configuration matches version=%A ecc=%A" versionFilter eccFilter))
            else
                Error (InputTooLong (
                    sprintf "Input (length %d) does not fit in any Micro QR symbol (version=%A, ecc=%A). Maximum is 35 numeric chars in M4-L."
                        input.Length versionFilter eccFilter))
        else
            let cfg = symbolConfigs.[i]
            let versionOk = match versionFilter with None -> true | Some v -> cfg.Version = v
            let eccOk     = match eccFilter     with None -> true | Some e -> cfg.Ecc = e
            if not (versionOk && eccOk) then loop (i + 1)
            else
                foundMatchingFilter <- true
                match trySelectMode input cfg with
                | None -> loop (i + 1)
                | Some mode ->
                    let len =
                        if mode = Byte then Text.Encoding.UTF8.GetByteCount(input)
                        else input.Length
                    let cap =
                        match mode with
                        | Numeric      -> cfg.NumericCap
                        | Alphanumeric -> cfg.AlphaCap
                        | Byte         -> cfg.ByteCap
                    if cap > 0 && len <= cap then
                        Ok (cfg, mode)
                    else
                        loop (i + 1)

    loop 0

// ============================================================================
// Core encoding logic
// ============================================================================

/// Core encoding: given a resolved config and mode, produce the final ModuleGrid.
///
/// Steps:
///   1. Build data codewords from the input bit stream.
///   2. Compute RS ECC codewords and append to data.
///   3. Flatten codeword stream to a bit array (M1: last data codeword = 4 bits).
///   4. Initialize grid with structural modules.
///   5. Place data bits via two-column zigzag.
///   6. Evaluate all 4 masks, pick lowest penalty.
///   7. Apply best mask and write final format info.
///   8. Convert to immutable ModuleGrid.
let private encodeWithConfig (input: string) (cfg: SymbolConfig) (mode: EncodingMode) : ModuleGrid =

    // Step 1: Build data codewords
    let dataCw = buildDataCodewords input cfg mode

    // Step 2: Compute RS ECC
    let gen   = getGenerator cfg.EccCw
    let eccCw = rsEncode dataCw gen

    // Step 3: Flatten to bit stream
    // For M1: data.[dataCw-1] has data in upper 4 bits → contribute only 4 bits.
    let mutable bitsAcc = ResizeArray<bool>()

    for i in 0 .. dataCw.Length - 1 do
        let bitsInCw = if cfg.M1HalfCw && i = cfg.DataCw - 1 then 4 else 8
        let cw = int dataCw.[i]
        // Extract bits from MSB side of each codeword.
        // For the M1 half-codeword, data lives in the upper 4 bits of the byte.
        for b = bitsInCw - 1 downto 0 do
            bitsAcc.Add(((cw >>> (b + (8 - bitsInCw))) &&& 1) = 1)

    for b in eccCw do
        let cw = int b
        for bit = 7 downto 0 do
            bitsAcc.Add(((cw >>> bit) &&& 1) = 1)

    let bits = bitsAcc.ToArray()

    // Step 4: Initialize grid
    let grid = buildGrid cfg

    // Step 5: Place data bits
    placeBits grid bits

    // Step 6: Evaluate all 4 masks, pick lowest penalty
    let mutable bestMask    = 0
    let mutable bestPenalty = Int32.MaxValue

    for m in 0 .. 3 do
        let masked = applyMask grid.Modules grid.Reserved cfg.Size m
        let fmt = formatTable.[cfg.SymbolIndicator].[m]
        // Write format info into a temporary copy for penalty evaluation
        let tmp = Array2D.copy masked
        writeFormatInfo tmp fmt
        let p = computePenalty tmp cfg.Size
        if p < bestPenalty then
            bestPenalty <- p
            bestMask    <- m

    // Step 7: Apply best mask and write final format info
    let finalModules = applyMask grid.Modules grid.Reserved cfg.Size bestMask
    let finalFmt = formatTable.[cfg.SymbolIndicator].[bestMask]
    writeFormatInfo finalModules finalFmt

    // Step 8: Convert to immutable ModuleGrid
    let sz = cfg.Size
    let immutableModules =
        Array.init sz (fun r ->
            Array.init sz (fun c -> finalModules.[r, c]))
    { Rows = sz; Cols = sz; Modules = immutableModules; ModuleShape = Square }

// ============================================================================
// Public API
// ============================================================================

/// Encode a string to a Micro QR Code ModuleGrid.
///
/// Automatically selects the smallest symbol (M1..M4) and most compact
/// encoding mode that can hold the input. Use ``opts`` to override any part
/// of the auto-selection:
///
///   - ``opts.Symbol``:      "M1"/"M2"/"M3"/"M4" or None for auto
///   - ``opts.ECCLevel``:    Detection/L/M/Q or None for auto
///   - ``opts.MaskPattern``: 0–3 or None for auto (lowest-penalty)
///
/// Returns ``Ok grid`` on success or ``Error e`` describing the failure.
///
/// ### Example — auto-select
///
/// ```fsharp
/// match MicroQR.encode "HELLO" defaultOptions with
/// | Ok grid   -> printfn "Grid is %d×%d" grid.Rows grid.Cols
/// | Error err -> printfn "Failed: %A" err
/// ```
///
/// ### Example — force M4 with L ECC
///
/// ```fsharp
/// let opts = { defaultOptions with Symbol = Some "M4"; ECCLevel = Some L }
/// let Ok grid = MicroQR.encode "https://example.com" opts
/// assert (grid.Rows = 17)
/// ```
///
/// ### Error cases
///
/// ```
/// InputTooLong    — input exceeds M4-L capacity (35 numeric / 15 byte)
/// InvalidECCLevel — Q requested for M1/M2/M3, or Detection requested for M2+
/// InvalidOptions  — Symbol string is not "M1"/"M2"/"M3"/"M4"
/// ```
let encode (data: string) (opts: MicroQROptions) : Result<ModuleGrid, MicroQRError> =
    // 1. Parse and validate the Symbol option (string → version filter)
    match parseSymbolOption opts.Symbol with
    | Error e -> Error e
    | Ok versionFilter ->

    // 2. Validate that ECCLevel is compatible with versionFilter
    // Q is only valid for M4; Detection is only valid for M1.
    let eccValidation =
        match versionFilter, opts.ECCLevel with
        | Some "M1", Some Q        ->
            Error (InvalidECCLevel "Q is not available for M1. Use Detection only.")
        | Some "M1", Some L        ->
            Error (InvalidECCLevel "L is not available for M1. Use Detection only.")
        | Some "M1", Some M        ->
            Error (InvalidECCLevel "M is not available for M1. Use Detection only.")
        | Some "M2", Some Q        ->
            Error (InvalidECCLevel "Q is not available for M2. Use L or M.")
        | Some "M3", Some Q        ->
            Error (InvalidECCLevel "Q is not available for M3. Use L or M.")
        | Some "M2", Some Detection ->
            Error (InvalidECCLevel "Detection is only available for M1.")
        | Some "M3", Some Detection ->
            Error (InvalidECCLevel "Detection is only available for M1.")
        | Some "M4", Some Detection ->
            Error (InvalidECCLevel "Detection is only available for M1.")
        | None, Some Q ->
            // Q without a version filter is valid — only M4-Q exists, auto-select will find it
            Ok ()
        | None, Some Detection ->
            // Detection without a version filter is valid — only M1/Detection exists
            Ok ()
        | _ -> Ok ()

    match eccValidation with
    | Error e -> Error e
    | Ok () ->

    // 3. Select symbol config and encoding mode
    match selectConfig data versionFilter opts.ECCLevel with
    | Error e -> Error e
    | Ok (cfg, mode) ->

    // 4. Encode
    let grid = encodeWithConfig data cfg mode

    // 5. Optionally re-apply a forced mask pattern
    // (encodeWithConfig already picked the best mask; if the caller forces a
    //  specific mask we need to redo the final masking step)
    match opts.MaskPattern with
    | None -> Ok grid
    | Some m ->
        if m < 0 || m > 3 then
            Error (InvalidOptions (sprintf "MaskPattern must be 0–3, got %d" m))
        else
            // Re-encode with the forced mask
            let dataCw = buildDataCodewords data cfg mode
            let gen    = getGenerator cfg.EccCw
            let eccCw  = rsEncode dataCw gen

            let mutable bitsAcc = ResizeArray<bool>()
            for i in 0 .. dataCw.Length - 1 do
                let bitsInCw = if cfg.M1HalfCw && i = cfg.DataCw - 1 then 4 else 8
                let cw = int dataCw.[i]
                for b = bitsInCw - 1 downto 0 do
                    bitsAcc.Add(((cw >>> (b + (8 - bitsInCw))) &&& 1) = 1)
            for b in eccCw do
                let cw = int b
                for bit = 7 downto 0 do
                    bitsAcc.Add(((cw >>> bit) &&& 1) = 1)

            let bits = bitsAcc.ToArray()
            let g2   = buildGrid cfg
            placeBits g2 bits

            let forced  = applyMask g2.Modules g2.Reserved cfg.Size m
            let fmtWord = formatTable.[cfg.SymbolIndicator].[m]
            writeFormatInfo forced fmtWord

            let sz = cfg.Size
            let modules2 = Array.init sz (fun r -> Array.init sz (fun c -> forced.[r, c]))
            Ok { Rows = sz; Cols = sz; Modules = modules2; ModuleShape = Square }
