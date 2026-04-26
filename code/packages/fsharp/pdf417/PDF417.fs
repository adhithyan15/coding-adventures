/// PDF417.fs — ISO/IEC 15438:2015-compliant PDF417 stacked linear barcode encoder for F#
///
/// PDF417 (Portable Data File 417) was invented by Ynjiun P. Wang at Symbol
/// Technologies in 1991. The name encodes its geometry: each codeword has
/// exactly **4** bars and **4** spaces (8 elements), and every codeword
/// occupies exactly **17** modules of horizontal space. "417" = 4 × 17.
///
/// ## Where PDF417 is deployed
///
/// | Application    | Detail                                               |
/// |----------------|------------------------------------------------------|
/// | AAMVA          | North American driver's licences and government IDs  |
/// | IATA BCBP      | Airline boarding passes                              |
/// | USPS           | Domestic shipping labels                             |
/// | US immigration | Form I-94, customs declarations                      |
/// | Healthcare     | Patient wristbands, medication labels                |
///
/// ## Encoding pipeline
///
/// ```
/// raw bytes
///   → byte compaction     (codeword 924 latch + 6-bytes-to-5-codewords base-900)
///   → length descriptor   (codeword 0 = total codewords in symbol)
///   → RS ECC              (GF(929) Reed-Solomon, b=3 convention, α=3)
///   → dimension selection (auto: roughly square symbol)
///   → padding             (codeword 900 fills unused slots)
///   → row indicators      (LRI + RRI per row, encode R/C/ECC level)
///   → cluster table lookup (codeword → 17-module bar/space pattern)
///   → start/stop patterns (fixed per row)
///   → ModuleGrid          (abstract boolean grid)
/// ```
///
/// ## v0.1.0 scope
///
/// This release implements **byte compaction only**. Text and numeric
/// compaction are planned for v0.2.0.
///
/// ## GF(929) — Why a prime field?
///
/// PDF417 uses Reed-Solomon over GF(929), not GF(256). The codeword alphabet
/// has exactly 929 elements (values 0–928). Since 929 is prime, GF(929) is
/// simply the integers modulo 929. No primitive polynomial needed — just
/// modular arithmetic.
///
/// Generator (primitive root): α = 3. Verify: 3^928 ≡ 1 (mod 929) by
/// Fermat's little theorem.
///
/// ## Three codeword clusters
///
/// Each row uses one of three cluster tables (0, 3, or 6) cycling as
/// row_number mod 3. This lets a scanner identify which row it is reading
/// without knowing the row number in advance — codeword patterns differ
/// by cluster.

module CodingAdventures.PDF417

open System
open CodingAdventures.Barcode2D

// ============================================================================
// Version
// ============================================================================

/// Package version string.
let VERSION = "0.1.0"

// ============================================================================
// Error types
// ============================================================================

/// Errors that can occur during PDF417 encoding.
type PDF417Error =
    /// The input data is too long to fit in any valid PDF417 symbol.
    | InputTooLong of string
    /// The specified rows or columns are outside the valid range (3–90 / 1–30).
    | InvalidDimensions of string
    /// The specified ECC level is outside the valid range 0–8.
    | InvalidECCLevel of string

    override e.ToString() =
        match e with
        | InputTooLong msg      -> sprintf "InputTooLong: %s" msg
        | InvalidDimensions msg -> sprintf "InvalidDimensions: %s" msg
        | InvalidECCLevel msg   -> sprintf "InvalidECCLevel: %s" msg

// ============================================================================
// Options
// ============================================================================

/// Configuration for the PDF417 encoder.
///
/// All fields are optional. Unset fields use automatic/default values:
///   - EccLevel: auto-selected based on data length (see autoEccLevel)
///   - Columns: auto-selected for a roughly square symbol
///   - RowHeight: 3 (minimum recommended for scan reliability)
type PDF417Options =
    {
        /// Reed-Solomon ECC level (0–8). None = auto-select.
        EccLevel: int option
        /// Number of data columns (1–30). None = auto-select.
        Columns: int option
        /// Module-rows per logical PDF417 row (1–10). Default: 3.
        RowHeight: int option
    }

/// Default options — all auto-selected or at recommended defaults.
let defaultOptions =
    { EccLevel = None; Columns = None; RowHeight = None }

// ============================================================================
// Constants
// ============================================================================

/// GF(929) prime modulus. All arithmetic is mod 929.
let [<Literal>] GF929_PRIME = 929

/// Generator element α = 3 (primitive root mod 929, per ISO/IEC 15438 Annex A.4).
let [<Literal>] GF929_ALPHA = 3

/// Multiplicative group order = PRIME - 1 = 928.
let [<Literal>] GF929_ORDER = 928

/// Latch-to-byte-compaction codeword (alternate form, works for any length).
/// Using 924 (not 901/902) because it is the most universally compatible.
let [<Literal>] LATCH_BYTE = 924

/// Padding codeword: neutral filler. Value 900 = "latch to text compaction",
/// which silently absorbs without producing output. Safe as padding.
let [<Literal>] PADDING_CW = 900

let [<Literal>] MIN_ROWS = 3
let [<Literal>] MAX_ROWS = 90
let [<Literal>] MIN_COLS = 1
let [<Literal>] MAX_COLS = 30

// ============================================================================
// GF(929) arithmetic
// ============================================================================
//
// GF(929) is the integers modulo 929. Since 929 is prime, every non-zero
// element has a multiplicative inverse. We use log/antilog lookup tables for
// O(1) multiplication, built once at module-load time.
//
// The tables take ~3.7 KB total (929 × 2 bytes × 2 arrays) and are built
// in a negligible fraction of a millisecond.

/// GF_EXP.[i] = α^i mod 929   (α = 3)
///
/// Built lazily once. GF_EXP.[928] = GF_EXP.[0] = 1, for wrap-around
/// convenience in gfMul when the sum of two logs equals 928.
let private GF_EXP : int[] =
    let tbl = Array.zeroCreate 929
    let mutable v = 1
    for i in 0 .. GF929_ORDER - 1 do
        tbl.[i] <- v
        v <- (v * GF929_ALPHA) % GF929_PRIME
    // GF_EXP.[928] = GF_EXP.[0] = 1, for wrap-around in gfMul.
    tbl.[GF929_ORDER] <- tbl.[0]
    tbl

/// GF_LOG.[v] = discrete log base α of v, for v in 1..928.
///
/// GF_LOG.[0] is intentionally left as 0 (log of zero is undefined;
/// gfMul short-circuits before consulting GF_LOG for zero operands).
let private GF_LOG : int[] =
    let tbl = Array.zeroCreate 929
    let mutable v = 1
    for i in 0 .. GF929_ORDER - 1 do
        tbl.[v] <- i
        v <- (v * GF929_ALPHA) % GF929_PRIME
    tbl

/// GF(929) multiply using log/antilog tables. Returns 0 if either operand is 0.
///
/// mul(a, b) = α^(log(a) + log(b))  using the identity:
///   log(a×b) = log(a) + log(b)  (mod 928)
///
/// The mod-928 wrap is needed because α^928 = α^0 = 1 (Fermat's little
/// theorem: α^{p-1} = 1 in GF(p)).
let private gfMul (a: int) (b: int) : int =
    if a = 0 || b = 0 then 0
    else GF_EXP.[(GF_LOG.[a] + GF_LOG.[b]) % GF929_ORDER]

/// GF(929) add: (a + b) mod 929.
///
/// Unlike GF(256) where addition = XOR, GF(929) uses ordinary integer
/// addition modulo 929 (the field has odd characteristic).
let private gfAdd (a: int) (b: int) : int =
    (a + b) % GF929_PRIME

// ============================================================================
// Reed-Solomon generator polynomial
// ============================================================================
//
// For ECC level L, k = 2^(L+1) ECC codewords are appended to the data.
// The generator polynomial uses the b=3 convention (roots start at α^3):
//
//   g(x) = (x − α^3)(x − α^4) ··· (x − α^{k+2})
//
// We build g iteratively by multiplying in each linear factor (x − α^j).
// Note: −α^j in GF(929) = 929 − α^j (since (a + (929 − a)) mod 929 = 0).

/// Build the RS generator polynomial for a given ECC level.
///
/// Returns an array of k+1 int coefficients [g_k, g_{k-1}, ..., g_0]
/// where k = 2^(eccLevel+1) and g_k = 1 (leading coefficient = 1).
///
/// The polynomial is: g(x) = x^k + g_{k-1}·x^{k-1} + ... + g_0
let private buildGenerator (eccLevel: int) : int[] =
    // Number of ECC codewords = 2^(eccLevel+1)
    let k = 1 <<< (eccLevel + 1)
    let mutable g = [| 1 |]

    // Multiply g by (x − α^j) for each root j from 3 to k+2 (inclusive).
    for j in 3 .. k + 2 do
        // α^j mod 929
        let root = GF_EXP.[j % GF929_ORDER]
        // −α^j in GF(929) = 929 − root
        let negRoot = GF929_PRIME - root
        // New polynomial = old polynomial × (x + negRoot)
        // [multiplying out: x·g(x) + negRoot·g(x)]
        let newG = Array.zeroCreate (g.Length + 1)
        for i in 0 .. g.Length - 1 do
            newG.[i]   <- gfAdd newG.[i]   g.[i]
            newG.[i+1] <- gfAdd newG.[i+1] (gfMul g.[i] negRoot)
        g <- newG
    g

// ============================================================================
// Reed-Solomon encoder
// ============================================================================
//
// Given data codewords D = [d₀, d₁, ..., d_{n-1}] and generator g(x)
// of degree k, compute k ECC codewords by the shift-register (LFSR) method:
//
//   R(x) = D(x) × x^k mod g(x)
//
// This is the standard polynomial long-division algorithm, equivalent to
// feeding each data symbol through a k-stage feedback register.
//
// No interleaving: all data feeds a single RS encoder (simpler than QR Code).

/// Compute `k` RS ECC codewords for `data` over GF(929) with b=3 convention.
let private rsEncode (data: int[]) (eccLevel: int) : int[] =
    let g = buildGenerator eccLevel
    let k = g.Length - 1
    let ecc = Array.zeroCreate k

    for d in data do
        let feedback = gfAdd d ecc.[0]
        // Shift register left — discard ecc.[0], slide others down by one.
        for i in 0 .. k - 2 do
            ecc.[i] <- ecc.[i+1]
        ecc.[k-1] <- 0
        // Add feedback × generator coefficient to each stage.
        // g.(k - i) is the coefficient for position i in the shifted array.
        for i in 0 .. k - 1 do
            ecc.[i] <- gfAdd ecc.[i] (gfMul g.[k - i] feedback)
    ecc

// ============================================================================
// Byte compaction
// ============================================================================
//
// Byte compaction converts raw bytes to PDF417 codewords (values 0–928):
//
//   1. Emit latch codeword 924.
//   2. For every full group of 6 bytes:
//      - Treat them as a 48-bit big-endian integer n.
//      - Express n in base 900 → exactly 5 codewords.
//      - This packs 6 bytes into 5 codewords (1.2 bytes/codeword).
//   3. For remaining 1–5 bytes: one codeword per byte (direct mapping).
//
// The 6→5 compression works because:
//   256^6 = 281,474,976,710,656  <  590,490,000,000,000 = 900^5
// So every 48-bit value fits in five base-900 digits.
//
// We use int64 (F# `int64` = System.Int64 = 64-bit signed) for the 48-bit
// arithmetic. 256^6 ≈ 2.81×10^14, which fits comfortably in int64 (max ≈ 9.2×10^18).

/// Encode raw bytes using byte compaction (codeword 924 latch).
///
/// Returns [924; c1; c2; ...] where each c_i is a byte-compacted codeword.
let private byteCompact (bytes: byte[]) : int[] =
    let result = System.Collections.Generic.List<int>()
    result.Add(LATCH_BYTE)

    let len = bytes.Length
    let mutable i = 0

    // Process full 6-byte groups → 5 codewords each.
    while i + 6 <= len do
        // Build the 48-bit big-endian integer.
        let mutable n = 0L
        for j in 0 .. 5 do
            n <- n * 256L + int64 bytes.[i + j]
        // Convert n to base 900 → 5 codewords, most-significant first.
        let group = Array.zeroCreate 5
        let mutable nn = n
        for j in 4 .. -1 .. 0 do
            group.[j] <- int (nn % 900L)
            nn <- nn / 900L
        for cw in group do result.Add(cw)
        i <- i + 6

    // Remaining bytes: 1 codeword per byte (direct byte value).
    while i < len do
        result.Add(int bytes.[i])
        i <- i + 1

    result.ToArray()

// ============================================================================
// ECC level auto-selection
// ============================================================================
//
// The recommended minimum ECC level depends on the total number of data
// codewords (including the byte-compaction prefix). Higher data density
// benefits from more ECC redundancy.

/// Select the minimum recommended ECC level based on data codeword count.
///
/// From the spec:
///   ≤ 40  → level 2 (8 ECC codewords)
///   ≤ 160 → level 3 (16 ECC codewords)
///   ≤ 320 → level 4 (32 ECC codewords)
///   ≤ 863 → level 5 (64 ECC codewords)
///   else  → level 6 (128 ECC codewords)
let autoEccLevel (dataCount: int) : int =
    if dataCount <= 40  then 2
    elif dataCount <= 160 then 3
    elif dataCount <= 320 then 4
    elif dataCount <= 863 then 5
    else 6

// ============================================================================
// Dimension selection
// ============================================================================
//
// We need to choose rows (3–90) and data columns (1–30) such that
// rows × cols ≥ total_codewords.
//
// Heuristic: c = ceil(sqrt(total / 3)), clamped to 1–30.
// Then r = ceil(total / c), clamped to 3–90.
//
// The divisor of 3 approximates the typical aspect ratio of a PDF417 symbol:
// each row is about 3 modules tall and each column is 17 modules wide,
// so a "square" symbol has roughly 3× as many rows as data-columns.

/// Choose the number of data columns and rows for the symbol.
///
/// Returns (cols, rows) or Error if the data is too large to fit.
let private chooseDimensions (total: int) : Result<int * int, PDF417Error> =
    let c = max MIN_COLS (min MAX_COLS (int (Math.Ceiling(Math.Sqrt(float total / 3.0)))))
    let r = max MIN_ROWS (int (Math.Ceiling(float total / float c)))
    if r > MAX_ROWS then
        Error (InputTooLong (sprintf "Cannot fit %d codewords in any valid symbol (max 90 rows × 30 cols)" total))
    else
        Ok (c, r)

// ============================================================================
// Row indicator computation
// ============================================================================
//
// Each logical PDF417 row carries two "indicator" codewords that encode
// metadata about the whole symbol:
//
//   R_info = (R - 1) / 3         R = total number of rows (3..90)
//   C_info = C - 1               C = number of data columns (1..30)
//   L_info = 3×L + (R-1) mod 3  L = ECC level (0..8)
//
// The three quantities are distributed across the three cluster types so
// that any three consecutive rows (one of each cluster) can reconstruct
// R, C, and L independently of reading from the top.
//
// Note: the RRI formula (Cluster 0 → C_info, Cluster 1 → R_info, Cluster 2 → L_info)
// follows the Python pdf417 library and TypeScript reference rather than the
// original spec text, because the Python/TS libraries produce verified
// scannable symbols.

/// Compute the Left Row Indicator codeword value for row `r`.
///
/// The LRI codeword is looked up in the cluster table just like any data
/// codeword — its 17-module bar/space pattern encodes the indicator value.
let computeLRI (r: int) (rows: int) (cols: int) (eccLevel: int) : int =
    let rInfo    = (rows - 1) / 3
    let cInfo    = cols - 1
    let lInfo    = 3 * eccLevel + (rows - 1) % 3
    let rowGroup = r / 3
    let cluster  = r % 3
    match cluster with
    | 0 -> 30 * rowGroup + rInfo
    | 1 -> 30 * rowGroup + lInfo
    | _ -> 30 * rowGroup + cInfo

/// Compute the Right Row Indicator codeword value for row `r`.
let computeRRI (r: int) (rows: int) (cols: int) (eccLevel: int) : int =
    let rInfo    = (rows - 1) / 3
    let cInfo    = cols - 1
    let lInfo    = 3 * eccLevel + (rows - 1) % 3
    let rowGroup = r / 3
    let cluster  = r % 3
    match cluster with
    | 0 -> 30 * rowGroup + cInfo
    | 1 -> 30 * rowGroup + rInfo
    | _ -> 30 * rowGroup + lInfo

// ============================================================================
// Cluster tables (static constants)
// ============================================================================
//
// PDF417 uses three distinct codeword-to-bar/space mappings (clusters 0, 3, 6),
// cycling row by row. This lets a scanner identify which row it is reading
// without needing to count from the top.
//
// Each cluster has 929 entries (one per codeword value 0–928). Each entry
// packs 8 element widths (b1,s1,b2,s2,b3,s3,b4,s4) into a 32-bit integer:
//   bits 31..28 = b1, bits 27..24 = s1, ..., bits 3..0 = s4
//
// These tables are extracted from the Python pdf417 library (MIT License)
// and match ISO/IEC 15438:2015 Annex B.
//
// Three arrays × 929 entries × 4 bytes = 11,148 bytes total — acceptable
// to embed as compile-time constants.

/// Start pattern: 17 modules, 6 elements.
/// Binary: 11111111010101000
/// Bar/space widths: [8; 1; 1; 1; 1; 1; 1; 3]
let private START_WIDTHS = [| 8; 1; 1; 1; 1; 1; 1; 3 |]

/// Stop pattern: 18 modules, 9 elements.
/// Binary: 111111101000101001
/// Bar/space widths: [7; 1; 1; 3; 1; 1; 1; 2; 1]
let private STOP_WIDTHS = [| 7; 1; 1; 3; 1; 1; 1; 2; 1 |]

/// Cluster 0 — used for rows where (row mod 3) = 0.
/// 929 packed uint32 entries. Each entry = (b1<<28)|(s1<<24)|(b2<<20)|(s2<<16)|(b3<<12)|(s3<<8)|(b4<<4)|s4
let private CLUSTER0 : uint32[] = [|
    0x31111136u; 0x41111144u; 0x51111152u; 0x31111235u; 0x41111243u; 0x51111251u; 0x21111326u; 0x31111334u; 0x21111425u; 0x11111516u; 0x21111524u; 0x11111615u; 0x21112136u
    0x31112144u; 0x41112152u; 0x21112235u; 0x31112243u; 0x41112251u; 0x11112326u; 0x21112334u; 0x11112425u; 0x11113136u; 0x21113144u; 0x31113152u; 0x11113235u; 0x21113243u
    0x31113251u; 0x11113334u; 0x21113342u; 0x11114144u; 0x21114152u; 0x11114243u; 0x21114251u; 0x11115152u; 0x51116111u; 0x31121135u; 0x41121143u; 0x51121151u; 0x21121226u
    0x31121234u; 0x41121242u; 0x21121325u; 0x31121333u; 0x11121416u; 0x21121424u; 0x31121432u; 0x11121515u; 0x21121523u; 0x11121614u; 0x21122135u; 0x31122143u; 0x41122151u
    0x11122226u; 0x21122234u; 0x31122242u; 0x11122325u; 0x21122333u; 0x31122341u; 0x11122424u; 0x21122432u; 0x11123135u; 0x21123143u; 0x31123151u; 0x11123234u; 0x21123242u
    0x11123333u; 0x21123341u; 0x11124143u; 0x21124151u; 0x11124242u; 0x11124341u; 0x21131126u; 0x31131134u; 0x41131142u; 0x21131225u; 0x31131233u; 0x41131241u; 0x11131316u
    0x21131324u; 0x31131332u; 0x11131415u; 0x21131423u; 0x11131514u; 0x11131613u; 0x11132126u; 0x21132134u; 0x31132142u; 0x11132225u; 0x21132233u; 0x31132241u; 0x11132324u
    0x21132332u; 0x11132423u; 0x11132522u; 0x11133134u; 0x21133142u; 0x11133233u; 0x21133241u; 0x11133332u; 0x11134142u; 0x21141125u; 0x31141133u; 0x41141141u; 0x11141216u
    0x21141224u; 0x31141232u; 0x11141315u; 0x21141323u; 0x31141331u; 0x11141414u; 0x21141422u; 0x11141513u; 0x21141521u; 0x11142125u; 0x21142133u; 0x31142141u; 0x11142224u
    0x21142232u; 0x11142323u; 0x21142331u; 0x11142422u; 0x11142521u; 0x21143141u; 0x11143331u; 0x11151116u; 0x21151124u; 0x31151132u; 0x11151215u; 0x21151223u; 0x31151231u
    0x11151314u; 0x21151322u; 0x11151413u; 0x21151421u; 0x11151512u; 0x11152124u; 0x11152223u; 0x11152322u; 0x11161115u; 0x31161131u; 0x21161222u; 0x21161321u; 0x11161511u
    0x32111135u; 0x42111143u; 0x52111151u; 0x22111226u; 0x32111234u; 0x42111242u; 0x22111325u; 0x32111333u; 0x42111341u; 0x12111416u; 0x22111424u; 0x12111515u; 0x22112135u
    0x32112143u; 0x42112151u; 0x12112226u; 0x22112234u; 0x32112242u; 0x12112325u; 0x22112333u; 0x12112424u; 0x12112523u; 0x12113135u; 0x22113143u; 0x32113151u; 0x12113234u
    0x22113242u; 0x12113333u; 0x12113432u; 0x12114143u; 0x22114151u; 0x12114242u; 0x12115151u; 0x31211126u; 0x41211134u; 0x51211142u; 0x31211225u; 0x41211233u; 0x51211241u
    0x21211316u; 0x31211324u; 0x41211332u; 0x21211415u; 0x31211423u; 0x41211431u; 0x21211514u; 0x31211522u; 0x22121126u; 0x32121134u; 0x42121142u; 0x21212126u; 0x22121225u
    0x32121233u; 0x42121241u; 0x21212225u; 0x31212233u; 0x41212241u; 0x11212316u; 0x12121415u; 0x22121423u; 0x32121431u; 0x11212415u; 0x21212423u; 0x11212514u; 0x12122126u
    0x22122134u; 0x32122142u; 0x11213126u; 0x12122225u; 0x22122233u; 0x32122241u; 0x11213225u; 0x21213233u; 0x31213241u; 0x11213324u; 0x12122423u; 0x11213423u; 0x12123134u
    0x22123142u; 0x11214134u; 0x12123233u; 0x22123241u; 0x11214233u; 0x21214241u; 0x11214332u; 0x12124142u; 0x11215142u; 0x12124241u; 0x11215241u; 0x31221125u; 0x41221133u
    0x51221141u; 0x21221216u; 0x31221224u; 0x41221232u; 0x21221315u; 0x31221323u; 0x41221331u; 0x21221414u; 0x31221422u; 0x21221513u; 0x21221612u; 0x22131125u; 0x32131133u
    0x42131141u; 0x21222125u; 0x22131224u; 0x32131232u; 0x11222216u; 0x12131315u; 0x31222232u; 0x32131331u; 0x11222315u; 0x12131414u; 0x22131422u; 0x11222414u; 0x21222422u
    0x22131521u; 0x12131612u; 0x12132125u; 0x22132133u; 0x32132141u; 0x11223125u; 0x12132224u; 0x22132232u; 0x11223224u; 0x21223232u; 0x22132331u; 0x11223323u; 0x12132422u
    0x12132521u; 0x12133133u; 0x22133141u; 0x11224133u; 0x12133232u; 0x11224232u; 0x12133331u; 0x11224331u; 0x11225141u; 0x21231116u; 0x31231124u; 0x41231132u; 0x21231215u
    0x31231223u; 0x41231231u; 0x21231314u; 0x31231322u; 0x21231413u; 0x31231421u; 0x21231512u; 0x21231611u; 0x12141116u; 0x22141124u; 0x32141132u; 0x11232116u; 0x12141215u
    0x22141223u; 0x32141231u; 0x11232215u; 0x21232223u; 0x31232231u; 0x11232314u; 0x12141413u; 0x22141421u; 0x11232413u; 0x21232421u; 0x11232512u; 0x12142124u; 0x22142132u
    0x11233124u; 0x12142223u; 0x22142231u; 0x11233223u; 0x21233231u; 0x11233322u; 0x12142421u; 0x11233421u; 0x11234132u; 0x11234231u; 0x21241115u; 0x31241123u; 0x41241131u
    0x21241214u; 0x31241222u; 0x21241313u; 0x31241321u; 0x21241412u; 0x21241511u; 0x12151115u; 0x22151123u; 0x32151131u; 0x11242115u; 0x12151214u; 0x22151222u; 0x11242214u
    0x21242222u; 0x22151321u; 0x11242313u; 0x12151412u; 0x11242412u; 0x12151511u; 0x12152123u; 0x11243123u; 0x11243222u; 0x11243321u; 0x31251122u; 0x31251221u; 0x21251411u
    0x22161122u; 0x12161213u; 0x11252213u; 0x11252312u; 0x11252411u; 0x23111126u; 0x33111134u; 0x43111142u; 0x23111225u; 0x33111233u; 0x13111316u; 0x23111324u; 0x33111332u
    0x13111415u; 0x23111423u; 0x13111514u; 0x13111613u; 0x13112126u; 0x23112134u; 0x33112142u; 0x13112225u; 0x23112233u; 0x33112241u; 0x13112324u; 0x23112332u; 0x13112423u
    0x13112522u; 0x13113134u; 0x23113142u; 0x13113233u; 0x23113241u; 0x13113332u; 0x13114142u; 0x13114241u; 0x32211125u; 0x42211133u; 0x52211141u; 0x22211216u; 0x32211224u
    0x42211232u; 0x22211315u; 0x32211323u; 0x42211331u; 0x22211414u; 0x32211422u; 0x22211513u; 0x32211521u; 0x23121125u; 0x33121133u; 0x43121141u; 0x22212125u; 0x23121224u
    0x33121232u; 0x12212216u; 0x13121315u; 0x32212232u; 0x33121331u; 0x12212315u; 0x22212323u; 0x23121422u; 0x12212414u; 0x13121513u; 0x12212513u; 0x13122125u; 0x23122133u
    0x33122141u; 0x12213125u; 0x13122224u; 0x32213141u; 0x12213224u; 0x22213232u; 0x23122331u; 0x12213323u; 0x13122422u; 0x12213422u; 0x13123133u; 0x23123141u; 0x12214133u
    0x13123232u; 0x12214232u; 0x13123331u; 0x13124141u; 0x12215141u; 0x31311116u; 0x41311124u; 0x51311132u; 0x31311215u; 0x41311223u; 0x51311231u; 0x31311314u; 0x41311322u
    0x31311413u; 0x41311421u; 0x31311512u; 0x22221116u; 0x32221124u; 0x42221132u; 0x21312116u; 0x22221215u; 0x41312132u; 0x42221231u; 0x21312215u; 0x31312223u; 0x41312231u
    0x21312314u; 0x22221413u; 0x32221421u; 0x21312413u; 0x31312421u; 0x22221611u; 0x13131116u; 0x23131124u; 0x33131132u; 0x12222116u; 0x13131215u; 0x23131223u; 0x33131231u
    0x11313116u; 0x12222215u; 0x22222223u; 0x32222231u; 0x11313215u; 0x21313223u; 0x31313231u; 0x23131421u; 0x11313314u; 0x12222413u; 0x22222421u; 0x11313413u; 0x13131611u
    0x13132124u; 0x23132132u; 0x12223124u; 0x13132223u; 0x23132231u; 0x11314124u; 0x12223223u; 0x22223231u; 0x11314223u; 0x21314231u; 0x13132421u; 0x12223421u; 0x13133132u
    0x12224132u; 0x13133231u; 0x11315132u; 0x12224231u; 0x31321115u; 0x41321123u; 0x51321131u; 0x31321214u; 0x41321222u; 0x31321313u; 0x41321321u; 0x31321412u; 0x31321511u
    0x22231115u; 0x32231123u; 0x42231131u; 0x21322115u; 0x22231214u; 0x41322131u; 0x21322214u; 0x31322222u; 0x32231321u; 0x21322313u; 0x22231412u; 0x21322412u; 0x22231511u
    0x21322511u; 0x13141115u; 0x23141123u; 0x33141131u; 0x12232115u; 0x13141214u; 0x23141222u; 0x11323115u; 0x12232214u; 0x22232222u; 0x23141321u; 0x11323214u; 0x21323222u
    0x13141412u; 0x11323313u; 0x12232412u; 0x13141511u; 0x12232511u; 0x13142123u; 0x23142131u; 0x12233123u; 0x13142222u; 0x11324123u; 0x12233222u; 0x13142321u; 0x11324222u
    0x12233321u; 0x13143131u; 0x11325131u; 0x31331114u; 0x41331122u; 0x31331213u; 0x41331221u; 0x31331312u; 0x31331411u; 0x22241114u; 0x32241122u; 0x21332114u; 0x22241213u
    0x32241221u; 0x21332213u; 0x31332221u; 0x21332312u; 0x22241411u; 0x21332411u; 0x13151114u; 0x23151122u; 0x12242114u; 0x13151213u; 0x23151221u; 0x11333114u; 0x12242213u
    0x22242221u; 0x11333213u; 0x21333221u; 0x13151411u; 0x11333312u; 0x12242411u; 0x11333411u; 0x12243122u; 0x11334122u; 0x11334221u; 0x41341121u; 0x31341311u; 0x32251121u
    0x22251212u; 0x22251311u; 0x13161113u; 0x12252113u; 0x11343113u; 0x13161311u; 0x12252311u; 0x24111125u; 0x14111216u; 0x24111224u; 0x14111315u; 0x24111323u; 0x34111331u
    0x14111414u; 0x24111422u; 0x14111513u; 0x24111521u; 0x14112125u; 0x24112133u; 0x34112141u; 0x14112224u; 0x24112232u; 0x14112323u; 0x24112331u; 0x14112422u; 0x14112521u
    0x14113133u; 0x24113141u; 0x14113232u; 0x14113331u; 0x14114141u; 0x23211116u; 0x33211124u; 0x43211132u; 0x23211215u; 0x33211223u; 0x23211314u; 0x33211322u; 0x23211413u
    0x33211421u; 0x23211512u; 0x14121116u; 0x24121124u; 0x34121132u; 0x13212116u; 0x14121215u; 0x33212132u; 0x34121231u; 0x13212215u; 0x23212223u; 0x33212231u; 0x13212314u
    0x14121413u; 0x24121421u; 0x13212413u; 0x23212421u; 0x14121611u; 0x14122124u; 0x24122132u; 0x13213124u; 0x14122223u; 0x24122231u; 0x13213223u; 0x23213231u; 0x13213322u
    0x14122421u; 0x14123132u; 0x13214132u; 0x14123231u; 0x13214231u; 0x32311115u; 0x42311123u; 0x52311131u; 0x32311214u; 0x42311222u; 0x32311313u; 0x42311321u; 0x32311412u
    0x32311511u; 0x23221115u; 0x33221123u; 0x22312115u; 0x23221214u; 0x33221222u; 0x22312214u; 0x32312222u; 0x33221321u; 0x22312313u; 0x23221412u; 0x22312412u; 0x23221511u
    0x22312511u; 0x14131115u; 0x24131123u; 0x13222115u; 0x14131214u; 0x33222131u; 0x12313115u; 0x13222214u; 0x23222222u; 0x24131321u; 0x12313214u; 0x22313222u; 0x14131412u
    0x12313313u; 0x13222412u; 0x14131511u; 0x13222511u; 0x14132123u; 0x24132131u; 0x13223123u; 0x14132222u; 0x12314123u; 0x13223222u; 0x14132321u; 0x12314222u; 0x13223321u
    0x14133131u; 0x13224131u; 0x12315131u; 0x41411114u; 0x51411122u; 0x41411213u; 0x51411221u; 0x41411312u; 0x41411411u; 0x32321114u; 0x42321122u; 0x31412114u; 0x41412122u
    0x42321221u; 0x31412213u; 0x41412221u; 0x31412312u; 0x32321411u; 0x31412411u; 0x23231114u; 0x33231122u; 0x22322114u; 0x23231213u; 0x33231221u; 0x21413114u; 0x22322213u
    0x32322221u; 0x21413213u; 0x31413221u; 0x23231411u; 0x21413312u; 0x22322411u; 0x21413411u; 0x14141114u; 0x24141122u; 0x13232114u; 0x14141213u; 0x24141221u; 0x12323114u
    0x13232213u; 0x23232221u; 0x11414114u; 0x12323213u; 0x22323221u; 0x14141411u; 0x11414213u; 0x21414221u; 0x13232411u; 0x11414312u; 0x14142122u; 0x13233122u; 0x14142221u
    0x12324122u; 0x13233221u; 0x11415122u; 0x12324221u; 0x11415221u; 0x41421113u; 0x51421121u; 0x41421212u; 0x41421311u; 0x32331113u; 0x42331121u; 0x31422113u; 0x41422121u
    0x31422212u; 0x32331311u; 0x31422311u; 0x23241113u; 0x33241121u; 0x22332113u; 0x23241212u; 0x21423113u; 0x22332212u; 0x23241311u; 0x21423212u; 0x22332311u; 0x21423311u
    0x14151113u; 0x24151121u; 0x13242113u; 0x23242121u; 0x12333113u; 0x13242212u; 0x14151311u; 0x11424113u; 0x12333212u; 0x13242311u; 0x11424212u; 0x12333311u; 0x11424311u
    0x13243121u; 0x11425121u; 0x41431211u; 0x31432112u; 0x31432211u; 0x22342112u; 0x21433112u; 0x21433211u; 0x13252112u; 0x12343112u; 0x11434112u; 0x11434211u; 0x15111116u
    0x15111215u; 0x25111223u; 0x15111314u; 0x15111413u; 0x15111512u; 0x15112124u; 0x15112223u; 0x15112322u; 0x15112421u; 0x15113132u; 0x15113231u; 0x24211115u; 0x24211214u
    0x34211222u; 0x24211313u; 0x34211321u; 0x24211412u; 0x24211511u; 0x15121115u; 0x25121123u; 0x14212115u; 0x24212123u; 0x25121222u; 0x14212214u; 0x24212222u; 0x14212313u
    0x24212321u; 0x14212412u; 0x15121511u; 0x14212511u; 0x15122123u; 0x25122131u; 0x14213123u; 0x24213131u; 0x14213222u; 0x15122321u; 0x14213321u; 0x15123131u; 0x14214131u
    0x33311114u; 0x33311213u; 0x33311312u; 0x33311411u; 0x24221114u; 0x23312114u; 0x33312122u; 0x34221221u; 0x23312213u; 0x33312221u; 0x23312312u; 0x24221411u; 0x23312411u
    0x15131114u; 0x14222114u; 0x15131213u; 0x25131221u; 0x13313114u; 0x14222213u; 0x15131312u; 0x13313213u; 0x14222312u; 0x15131411u; 0x13313312u; 0x14222411u; 0x15132122u
    0x14223122u; 0x15132221u; 0x13314122u; 0x14223221u; 0x13314221u; 0x42411113u; 0x42411212u; 0x42411311u; 0x33321113u; 0x32412113u; 0x42412121u; 0x32412212u; 0x33321311u
    0x32412311u; 0x24231113u; 0x34231121u; 0x23322113u; 0x33322121u; 0x22413113u; 0x23322212u; 0x24231311u; 0x22413212u; 0x23322311u; 0x22413311u; 0x15141113u; 0x25141121u
    0x14232113u; 0x24232121u; 0x13323113u; 0x14232212u; 0x15141311u; 0x12414113u; 0x13323212u; 0x14232311u; 0x12414212u; 0x13323311u; 0x15142121u; 0x14233121u; 0x13324121u
    0x12415121u; 0x51511112u; 0x51511211u; 0x42421112u; 0x41512112u; 0x42421211u; 0x41512211u; 0x33331112u; 0x32422112u; 0x33331211u; 0x31513112u; 0x32422211u; 0x31513211u
    0x24241112u; 0x23332112u; 0x24241211u; 0x22423112u; 0x23332211u; 0x21514112u
|]

/// Cluster 1 — used for rows where (row mod 3) = 1.
let private CLUSTER1 : uint32[] = [|
    0x51111125u; 0x61111133u; 0x41111216u; 0x51111224u; 0x61111232u; 0x41111315u; 0x51111323u; 0x61111331u; 0x41111414u; 0x51111422u; 0x41111513u; 0x51111521u; 0x41111612u
    0x41112125u; 0x51112133u; 0x61112141u; 0x31112216u; 0x41112224u; 0x51112232u; 0x31112315u; 0x41112323u; 0x51112331u; 0x31112414u; 0x41112422u; 0x31112513u; 0x41112521u
    0x31112612u; 0x31113125u; 0x41113133u; 0x51113141u; 0x21113216u; 0x31113224u; 0x41113232u; 0x21113315u; 0x31113323u; 0x41113331u; 0x21113414u; 0x31113422u; 0x21113513u
    0x31113521u; 0x21113612u; 0x21114125u; 0x31114133u; 0x41114141u; 0x11114216u; 0x21114224u; 0x31114232u; 0x11114315u; 0x21114323u; 0x31114331u; 0x11114414u; 0x21114422u
    0x11114513u; 0x21114521u; 0x11115125u; 0x21115133u; 0x31115141u; 0x11115224u; 0x21115232u; 0x11115323u; 0x21115331u; 0x11115422u; 0x11116133u; 0x21116141u; 0x11116232u
    0x11116331u; 0x41121116u; 0x51121124u; 0x61121132u; 0x41121215u; 0x51121223u; 0x61121231u; 0x41121314u; 0x51121322u; 0x41121413u; 0x51121421u; 0x41121512u; 0x41121611u
    0x31122116u; 0x41122124u; 0x51122132u; 0x31122215u; 0x41122223u; 0x51122231u; 0x31122314u; 0x41122322u; 0x31122413u; 0x41122421u; 0x31122512u; 0x31122611u; 0x21123116u
    0x31123124u; 0x41123132u; 0x21123215u; 0x31123223u; 0x41123231u; 0x21123314u; 0x31123322u; 0x21123413u; 0x31123421u; 0x21123512u; 0x21123611u; 0x11124116u; 0x21124124u
    0x31124132u; 0x11124215u; 0x21124223u; 0x31124231u; 0x11124314u; 0x21124322u; 0x11124413u; 0x21124421u; 0x11124512u; 0x11125124u; 0x21125132u; 0x11125223u; 0x21125231u
    0x11125322u; 0x11125421u; 0x11126132u; 0x11126231u; 0x41131115u; 0x51131123u; 0x61131131u; 0x41131214u; 0x51131222u; 0x41131313u; 0x51131321u; 0x41131412u; 0x41131511u
    0x31132115u; 0x41132123u; 0x51132131u; 0x31132214u; 0x41132222u; 0x31132313u; 0x41132321u; 0x31132412u; 0x31132511u; 0x21133115u; 0x31133123u; 0x41133131u; 0x21133214u
    0x31133222u; 0x21133313u; 0x31133321u; 0x21133412u; 0x21133511u; 0x11134115u; 0x21134123u; 0x31134131u; 0x11134214u; 0x21134222u; 0x11134313u; 0x21134321u; 0x11134412u
    0x11134511u; 0x11135123u; 0x21135131u; 0x11135222u; 0x11135321u; 0x11136131u; 0x41141114u; 0x51141122u; 0x41141213u; 0x51141221u; 0x41141312u; 0x41141411u; 0x31142114u
    0x41142122u; 0x31142213u; 0x41142221u; 0x31142312u; 0x31142411u; 0x21143114u; 0x31143122u; 0x21143213u; 0x31143221u; 0x21143312u; 0x21143411u; 0x11144114u; 0x21144122u
    0x11144213u; 0x21144221u; 0x11144312u; 0x11144411u; 0x11145122u; 0x11145221u; 0x41151113u; 0x51151121u; 0x41151212u; 0x41151311u; 0x31152113u; 0x41152121u; 0x31152212u
    0x31152311u; 0x21153113u; 0x31153121u; 0x21153212u; 0x21153311u; 0x11154113u; 0x21154121u; 0x11154212u; 0x11154311u; 0x41161112u; 0x41161211u; 0x31162112u; 0x31162211u
    0x21163112u; 0x21163211u; 0x42111116u; 0x52111124u; 0x62111132u; 0x42111215u; 0x52111223u; 0x62111231u; 0x42111314u; 0x52111322u; 0x42111413u; 0x52111421u; 0x42111512u
    0x42111611u; 0x32112116u; 0x42112124u; 0x52112132u; 0x32112215u; 0x42112223u; 0x52112231u; 0x32112314u; 0x42112322u; 0x32112413u; 0x42112421u; 0x32112512u; 0x32112611u
    0x22113116u; 0x32113124u; 0x42113132u; 0x22113215u; 0x32113223u; 0x42113231u; 0x22113314u; 0x32113322u; 0x22113413u; 0x32113421u; 0x22113512u; 0x22113611u; 0x12114116u
    0x22114124u; 0x32114132u; 0x12114215u; 0x22114223u; 0x32114231u; 0x12114314u; 0x22114322u; 0x12114413u; 0x22114421u; 0x12114512u; 0x12115124u; 0x22115132u; 0x12115223u
    0x22115231u; 0x12115322u; 0x12115421u; 0x12116132u; 0x12116231u; 0x51211115u; 0x61211123u; 0x11211164u; 0x51211214u; 0x61211222u; 0x11211263u; 0x51211313u; 0x61211321u
    0x11211362u; 0x51211412u; 0x51211511u; 0x42121115u; 0x52121123u; 0x62121131u; 0x41212115u; 0x42121214u; 0x61212131u; 0x41212214u; 0x51212222u; 0x52121321u; 0x41212313u
    0x42121412u; 0x41212412u; 0x42121511u; 0x41212511u; 0x32122115u; 0x42122123u; 0x52122131u; 0x31213115u; 0x32122214u; 0x42122222u; 0x31213214u; 0x41213222u; 0x42122321u
    0x31213313u; 0x32122412u; 0x31213412u; 0x32122511u; 0x31213511u; 0x22123115u; 0x32123123u; 0x42123131u; 0x21214115u; 0x22123214u; 0x32123222u; 0x21214214u; 0x31214222u
    0x32123321u; 0x21214313u; 0x22123412u; 0x21214412u; 0x22123511u; 0x21214511u; 0x12124115u; 0x22124123u; 0x32124131u; 0x11215115u; 0x12124214u; 0x22124222u; 0x11215214u
    0x21215222u; 0x22124321u; 0x11215313u; 0x12124412u; 0x11215412u; 0x12124511u; 0x12125123u; 0x22125131u; 0x11216123u; 0x12125222u; 0x11216222u; 0x12125321u; 0x11216321u
    0x12126131u; 0x51221114u; 0x61221122u; 0x11221163u; 0x51221213u; 0x61221221u; 0x11221262u; 0x51221312u; 0x11221361u; 0x51221411u; 0x42131114u; 0x52131122u; 0x41222114u
    0x42131213u; 0x52131221u; 0x41222213u; 0x51222221u; 0x41222312u; 0x42131411u; 0x41222411u; 0x32132114u; 0x42132122u; 0x31223114u; 0x32132213u; 0x42132221u; 0x31223213u
    0x41223221u; 0x31223312u; 0x32132411u; 0x31223411u; 0x22133114u; 0x32133122u; 0x21224114u; 0x22133213u; 0x32133221u; 0x21224213u; 0x31224221u; 0x21224312u; 0x22133411u
    0x21224411u; 0x12134114u; 0x22134122u; 0x11225114u; 0x12134213u; 0x22134221u; 0x11225213u; 0x21225221u; 0x11225312u; 0x12134411u; 0x11225411u; 0x12135122u; 0x11226122u
    0x12135221u; 0x11226221u; 0x51231113u; 0x61231121u; 0x11231162u; 0x51231212u; 0x11231261u; 0x51231311u; 0x42141113u; 0x52141121u; 0x41232113u; 0x51232121u; 0x41232212u
    0x42141311u; 0x41232311u; 0x32142113u; 0x42142121u; 0x31233113u; 0x32142212u; 0x31233212u; 0x32142311u; 0x31233311u; 0x22143113u; 0x32143121u; 0x21234113u; 0x31234121u
    0x21234212u; 0x22143311u; 0x21234311u; 0x12144113u; 0x22144121u; 0x11235113u; 0x12144212u; 0x11235212u; 0x12144311u; 0x11235311u; 0x12145121u; 0x11236121u; 0x51241112u
    0x11241161u; 0x51241211u; 0x42151112u; 0x41242112u; 0x42151211u; 0x41242211u; 0x32152112u; 0x31243112u; 0x32152211u; 0x31243211u; 0x22153112u; 0x21244112u; 0x22153211u
    0x21244211u; 0x12154112u; 0x11245112u; 0x12154211u; 0x11245211u; 0x51251111u; 0x42161111u; 0x41252111u; 0x32162111u; 0x31253111u; 0x22163111u; 0x21254111u; 0x43111115u
    0x53111123u; 0x63111131u; 0x43111214u; 0x53111222u; 0x43111313u; 0x53111321u; 0x43111412u; 0x43111511u; 0x33112115u; 0x43112123u; 0x53112131u; 0x33112214u; 0x43112222u
    0x33112313u; 0x43112321u; 0x33112412u; 0x33112511u; 0x23113115u; 0x33113123u; 0x43113131u; 0x23113214u; 0x33113222u; 0x23113313u; 0x33113321u; 0x23113412u; 0x23113511u
    0x13114115u; 0x23114123u; 0x33114131u; 0x13114214u; 0x23114222u; 0x13114313u; 0x23114321u; 0x13114412u; 0x13114511u; 0x13115123u; 0x23115131u; 0x13115222u; 0x13115321u
    0x13116131u; 0x52211114u; 0x62211122u; 0x12211163u; 0x52211213u; 0x62211221u; 0x12211262u; 0x52211312u; 0x12211361u; 0x52211411u; 0x43121114u; 0x53121122u; 0x42212114u
    0x43121213u; 0x53121221u; 0x42212213u; 0x52212221u; 0x42212312u; 0x43121411u; 0x42212411u; 0x33122114u; 0x43122122u; 0x32213114u; 0x33122213u; 0x43122221u; 0x32213213u
    0x42213221u; 0x32213312u; 0x33122411u; 0x32213411u; 0x23123114u; 0x33123122u; 0x22214114u; 0x23123213u; 0x33123221u; 0x22214213u; 0x32214221u; 0x22214312u; 0x23123411u
    0x22214411u; 0x13124114u; 0x23124122u; 0x12215114u; 0x13124213u; 0x23124221u; 0x12215213u; 0x22215221u; 0x12215312u; 0x13124411u; 0x12215411u; 0x13125122u; 0x12216122u
    0x13125221u; 0x12216221u; 0x61311113u; 0x11311154u; 0x21311162u; 0x61311212u; 0x11311253u; 0x21311261u; 0x61311311u; 0x11311352u; 0x11311451u; 0x52221113u; 0x62221121u
    0x12221162u; 0x51312113u; 0x61312121u; 0x11312162u; 0x12221261u; 0x51312212u; 0x52221311u; 0x11312261u; 0x51312311u; 0x43131113u; 0x53131121u; 0x42222113u; 0x43131212u
    0x41313113u; 0x51313121u; 0x43131311u; 0x41313212u; 0x42222311u; 0x41313311u; 0x33132113u; 0x43132121u; 0x32223113u; 0x33132212u; 0x31314113u; 0x32223212u; 0x33132311u
    0x31314212u; 0x32223311u; 0x31314311u; 0x23133113u; 0x33133121u; 0x22224113u; 0x23133212u; 0x21315113u; 0x22224212u; 0x23133311u; 0x21315212u; 0x22224311u; 0x21315311u
    0x13134113u; 0x23134121u; 0x12225113u; 0x13134212u; 0x11316113u; 0x12225212u; 0x13134311u; 0x11316212u; 0x12225311u; 0x11316311u; 0x13135121u; 0x12226121u; 0x61321112u
    0x11321153u; 0x21321161u; 0x61321211u; 0x11321252u; 0x11321351u; 0x52231112u; 0x12231161u; 0x51322112u; 0x52231211u; 0x11322161u; 0x51322211u; 0x43141112u; 0x42232112u
    0x43141211u; 0x41323112u; 0x42232211u; 0x41323211u; 0x33142112u; 0x32233112u; 0x33142211u; 0x31324112u; 0x32233211u; 0x31324211u; 0x23143112u; 0x22234112u; 0x23143211u
    0x21325112u; 0x22234211u; 0x21325211u; 0x13144112u; 0x12235112u; 0x13144211u; 0x11326112u; 0x12235211u; 0x11326211u; 0x61331111u; 0x11331152u; 0x11331251u; 0x52241111u
    0x51332111u; 0x43151111u; 0x42242111u; 0x41333111u; 0x33152111u; 0x32243111u; 0x31334111u; 0x23153111u; 0x22244111u; 0x21335111u; 0x13154111u; 0x12245111u; 0x11336111u
    0x11341151u; 0x44111114u; 0x54111122u; 0x44111213u; 0x54111221u; 0x44111312u; 0x44111411u; 0x34112114u; 0x44112122u; 0x34112213u; 0x44112221u; 0x34112312u; 0x34112411u
    0x24113114u; 0x34113122u; 0x24113213u; 0x34113221u; 0x24113312u; 0x24113411u; 0x14114114u; 0x24114122u; 0x14114213u; 0x24114221u; 0x14114312u; 0x14114411u; 0x14115122u
    0x14115221u; 0x53211113u; 0x63211121u; 0x13211162u; 0x53211212u; 0x13211261u; 0x53211311u; 0x44121113u; 0x54121121u; 0x43212113u; 0x44121212u; 0x43212212u; 0x44121311u
    0x43212311u; 0x34122113u; 0x44122121u; 0x33213113u; 0x34122212u; 0x33213212u; 0x34122311u; 0x33213311u; 0x24123113u; 0x34123121u; 0x23214113u; 0x24123212u; 0x23214212u
    0x24123311u; 0x23214311u; 0x14124113u; 0x24124121u; 0x13215113u; 0x14124212u; 0x13215212u; 0x14124311u; 0x13215311u; 0x14125121u; 0x13216121u; 0x62311112u; 0x12311153u
    0x22311161u; 0x62311211u; 0x12311252u; 0x12311351u; 0x53221112u; 0x13221161u; 0x52312112u; 0x53221211u; 0x12312161u; 0x52312211u; 0x44131112u; 0x43222112u; 0x44131211u
    0x42313112u; 0x43222211u; 0x42313211u; 0x34132112u; 0x33223112u; 0x34132211u; 0x32314112u; 0x33223211u; 0x32314211u; 0x24133112u; 0x23224112u; 0x24133211u; 0x22315112u
    0x23224211u; 0x22315211u; 0x14134112u; 0x13225112u; 0x14134211u; 0x12316112u; 0x13225211u; 0x12316211u; 0x11411144u; 0x21411152u; 0x11411243u; 0x21411251u; 0x11411342u
    0x11411441u; 0x62321111u; 0x12321152u; 0x61412111u; 0x11412152u; 0x12321251u; 0x11412251u; 0x53231111u; 0x52322111u; 0x51413111u; 0x44141111u; 0x43232111u; 0x42323111u
    0x41414111u; 0x34142111u; 0x33233111u; 0x32324111u; 0x31415111u; 0x24143111u; 0x23234111u; 0x22325111u; 0x21416111u; 0x14144111u; 0x13235111u; 0x12326111u; 0x11421143u
    0x21421151u; 0x11421242u; 0x11421341u; 0x12331151u; 0x11422151u; 0x11431142u; 0x11431241u; 0x11441141u; 0x45111113u; 0x45111212u; 0x45111311u; 0x35112113u; 0x45112121u
    0x35112212u; 0x35112311u; 0x25113113u; 0x35113121u; 0x25113212u; 0x25113311u; 0x15114113u; 0x25114121u; 0x15114212u; 0x15114311u; 0x15115121u; 0x54211112u; 0x14211161u
    0x54211211u; 0x45121112u; 0x44212112u; 0x45121211u; 0x44212211u; 0x35122112u; 0x34213112u; 0x35122211u; 0x34213211u; 0x25123112u; 0x24214112u; 0x25123211u; 0x24214211u
    0x15124112u; 0x14215112u; 0x15124211u; 0x14215211u; 0x63311111u; 0x13311152u; 0x13311251u; 0x54221111u; 0x53312111u; 0x45131111u; 0x44222111u; 0x43313111u; 0x35132111u
    0x34223111u; 0x33314111u; 0x25133111u; 0x24224111u; 0x23315111u; 0x15134111u; 0x14225111u; 0x13316111u; 0x12411143u; 0x22411151u; 0x12411242u; 0x12411341u; 0x13321151u
    0x12412151u; 0x11511134u; 0x21511142u; 0x11511233u; 0x21511241u; 0x11511332u; 0x11511431u; 0x12421142u; 0x11512142u; 0x12421241u; 0x11512241u; 0x11521133u; 0x21521141u
    0x11521232u; 0x11521331u; 0x12431141u; 0x11522141u; 0x11531132u; 0x11531231u; 0x11541131u; 0x36112112u; 0x36112211u; 0x26113112u; 0x26113211u; 0x16114112u; 0x16114211u
    0x45212111u; 0x36122111u; 0x35213111u; 0x26123111u; 0x25214111u; 0x16124111u; 0x15215111u; 0x14311151u; 0x13411142u; 0x13411241u; 0x12511133u; 0x22511141u; 0x12511232u
    0x12511331u; 0x13421141u; 0x12512141u; 0x11611124u; 0x21611132u; 0x11611223u; 0x21611231u; 0x11611322u; 0x11611421u; 0x12521132u; 0x11612132u; 0x12521231u; 0x11612231u
    0x11621123u; 0x21621131u; 0x11621222u; 0x11621321u; 0x12531131u; 0x11622131u; 0x11631122u; 0x11631221u; 0x14411141u; 0x13511132u; 0x13511231u; 0x12611123u; 0x22611131u
    0x12611222u; 0x12611321u; 0x13521131u; 0x12612131u; 0x12621122u; 0x12621221u
|]

/// Cluster 2 — used for rows where (row mod 3) = 2.
let private CLUSTER2 : uint32[] = [|
    0x21111155u; 0x31111163u; 0x11111246u; 0x21111254u; 0x31111262u; 0x11111345u; 0x21111353u; 0x31111361u; 0x11111444u; 0x21111452u; 0x11111543u; 0x61112114u; 0x11112155u
    0x21112163u; 0x61112213u; 0x11112254u; 0x21112262u; 0x61112312u; 0x11112353u; 0x21112361u; 0x61112411u; 0x11112452u; 0x51113114u; 0x61113122u; 0x11113163u; 0x51113213u
    0x61113221u; 0x11113262u; 0x51113312u; 0x11113361u; 0x51113411u; 0x41114114u; 0x51114122u; 0x41114213u; 0x51114221u; 0x41114312u; 0x41114411u; 0x31115114u; 0x41115122u
    0x31115213u; 0x41115221u; 0x31115312u; 0x31115411u; 0x21116114u; 0x31116122u; 0x21116213u; 0x31116221u; 0x21116312u; 0x11121146u; 0x21121154u; 0x31121162u; 0x11121245u
    0x21121253u; 0x31121261u; 0x11121344u; 0x21121352u; 0x11121443u; 0x21121451u; 0x11121542u; 0x61122113u; 0x11122154u; 0x21122162u; 0x61122212u; 0x11122253u; 0x21122261u
    0x61122311u; 0x11122352u; 0x11122451u; 0x51123113u; 0x61123121u; 0x11123162u; 0x51123212u; 0x11123261u; 0x51123311u; 0x41124113u; 0x51124121u; 0x41124212u; 0x41124311u
    0x31125113u; 0x41125121u; 0x31125212u; 0x31125311u; 0x21126113u; 0x31126121u; 0x21126212u; 0x21126311u; 0x11131145u; 0x21131153u; 0x31131161u; 0x11131244u; 0x21131252u
    0x11131343u; 0x21131351u; 0x11131442u; 0x11131541u; 0x61132112u; 0x11132153u; 0x21132161u; 0x61132211u; 0x11132252u; 0x11132351u; 0x51133112u; 0x11133161u; 0x51133211u
    0x41134112u; 0x41134211u; 0x31135112u; 0x31135211u; 0x21136112u; 0x21136211u; 0x11141144u; 0x21141152u; 0x11141243u; 0x21141251u; 0x11141342u; 0x11141441u; 0x61142111u
    0x11142152u; 0x11142251u; 0x51143111u; 0x41144111u; 0x31145111u; 0x11151143u; 0x21151151u; 0x11151242u; 0x11151341u; 0x11152151u; 0x11161142u; 0x11161241u; 0x12111146u
    0x22111154u; 0x32111162u; 0x12111245u; 0x22111253u; 0x32111261u; 0x12111344u; 0x22111352u; 0x12111443u; 0x22111451u; 0x12111542u; 0x62112113u; 0x12112154u; 0x22112162u
    0x62112212u; 0x12112253u; 0x22112261u; 0x62112311u; 0x12112352u; 0x12112451u; 0x52113113u; 0x62113121u; 0x12113162u; 0x52113212u; 0x12113261u; 0x52113311u; 0x42114113u
    0x52114121u; 0x42114212u; 0x42114311u; 0x32115113u; 0x42115121u; 0x32115212u; 0x32115311u; 0x22116113u; 0x32116121u; 0x22116212u; 0x22116311u; 0x21211145u; 0x31211153u
    0x41211161u; 0x11211236u; 0x21211244u; 0x31211252u; 0x11211335u; 0x21211343u; 0x31211351u; 0x11211434u; 0x21211442u; 0x11211533u; 0x21211541u; 0x11211632u; 0x12121145u
    0x22121153u; 0x32121161u; 0x11212145u; 0x12121244u; 0x22121252u; 0x11212244u; 0x21212252u; 0x22121351u; 0x11212343u; 0x12121442u; 0x11212442u; 0x12121541u; 0x11212541u
    0x62122112u; 0x12122153u; 0x22122161u; 0x61213112u; 0x62122211u; 0x11213153u; 0x12122252u; 0x61213211u; 0x11213252u; 0x12122351u; 0x11213351u; 0x52123112u; 0x12123161u
    0x51214112u; 0x52123211u; 0x11214161u; 0x51214211u; 0x42124112u; 0x41215112u; 0x42124211u; 0x41215211u; 0x32125112u; 0x31216112u; 0x32125211u; 0x31216211u; 0x22126112u
    0x22126211u; 0x11221136u; 0x21221144u; 0x31221152u; 0x11221235u; 0x21221243u; 0x31221251u; 0x11221334u; 0x21221342u; 0x11221433u; 0x21221441u; 0x11221532u; 0x11221631u
    0x12131144u; 0x22131152u; 0x11222144u; 0x12131243u; 0x22131251u; 0x11222243u; 0x21222251u; 0x11222342u; 0x12131441u; 0x11222441u; 0x62132111u; 0x12132152u; 0x61223111u
    0x11223152u; 0x12132251u; 0x11223251u; 0x52133111u; 0x51224111u; 0x42134111u; 0x41225111u; 0x32135111u; 0x31226111u; 0x22136111u; 0x11231135u; 0x21231143u; 0x31231151u
    0x11231234u; 0x21231242u; 0x11231333u; 0x21231341u; 0x11231432u; 0x11231531u; 0x12141143u; 0x22141151u; 0x11232143u; 0x12141242u; 0x11232242u; 0x12141341u; 0x11232341u
    0x12142151u; 0x11233151u; 0x11241134u; 0x21241142u; 0x11241233u; 0x21241241u; 0x11241332u; 0x11241431u; 0x12151142u; 0x11242142u; 0x12151241u; 0x11242241u; 0x11251133u
    0x21251141u; 0x11251232u; 0x11251331u; 0x12161141u; 0x11252141u; 0x11261132u; 0x11261231u; 0x13111145u; 0x23111153u; 0x33111161u; 0x13111244u; 0x23111252u; 0x13111343u
    0x23111351u; 0x13111442u; 0x13111541u; 0x63112112u; 0x13112153u; 0x23112161u; 0x63112211u; 0x13112252u; 0x13112351u; 0x53113112u; 0x13113161u; 0x53113211u; 0x43114112u
    0x43114211u; 0x33115112u; 0x33115211u; 0x23116112u; 0x23116211u; 0x12211136u; 0x22211144u; 0x32211152u; 0x12211235u; 0x22211243u; 0x32211251u; 0x12211334u; 0x22211342u
    0x12211433u; 0x22211441u; 0x12211532u; 0x12211631u; 0x13121144u; 0x23121152u; 0x12212144u; 0x13121243u; 0x23121251u; 0x12212243u; 0x22212251u; 0x12212342u; 0x13121441u
    0x12212441u; 0x63122111u; 0x13122152u; 0x62213111u; 0x12213152u; 0x13122251u; 0x12213251u; 0x53123111u; 0x52214111u; 0x43124111u; 0x42215111u; 0x33125111u; 0x32216111u
    0x23126111u; 0x21311135u; 0x31311143u; 0x41311151u; 0x11311226u; 0x21311234u; 0x31311242u; 0x11311325u; 0x21311333u; 0x31311341u; 0x11311424u; 0x21311432u; 0x11311523u
    0x21311531u; 0x11311622u; 0x12221135u; 0x22221143u; 0x32221151u; 0x11312135u; 0x12221234u; 0x22221242u; 0x11312234u; 0x21312242u; 0x22221341u; 0x11312333u; 0x12221432u
    0x11312432u; 0x12221531u; 0x11312531u; 0x13131143u; 0x23131151u; 0x12222143u; 0x13131242u; 0x11313143u; 0x12222242u; 0x13131341u; 0x11313242u; 0x12222341u; 0x11313341u
    0x13132151u; 0x12223151u; 0x11314151u; 0x11321126u; 0x21321134u; 0x31321142u; 0x11321225u; 0x21321233u; 0x31321241u; 0x11321324u; 0x21321332u; 0x11321423u; 0x21321431u
    0x11321522u; 0x11321621u; 0x12231134u; 0x22231142u; 0x11322134u; 0x12231233u; 0x22231241u; 0x11322233u; 0x21322241u; 0x11322332u; 0x12231431u; 0x11322431u; 0x13141142u
    0x12232142u; 0x13141241u; 0x11323142u; 0x12232241u; 0x11323241u; 0x11331125u; 0x21331133u; 0x31331141u; 0x11331224u; 0x21331232u; 0x11331323u; 0x21331331u; 0x11331422u
    0x11331521u; 0x12241133u; 0x22241141u; 0x11332133u; 0x12241232u; 0x11332232u; 0x12241331u; 0x11332331u; 0x13151141u; 0x12242141u; 0x11333141u; 0x11341124u; 0x21341132u
    0x11341223u; 0x21341231u; 0x11341322u; 0x11341421u; 0x12251132u; 0x11342132u; 0x12251231u; 0x11342231u; 0x11351123u; 0x21351131u; 0x11351222u; 0x11351321u; 0x12261131u
    0x11352131u; 0x11361122u; 0x11361221u; 0x14111144u; 0x24111152u; 0x14111243u; 0x24111251u; 0x14111342u; 0x14111441u; 0x14112152u; 0x14112251u; 0x54113111u; 0x44114111u
    0x34115111u; 0x24116111u; 0x13211135u; 0x23211143u; 0x33211151u; 0x13211234u; 0x23211242u; 0x13211333u; 0x23211341u; 0x13211432u; 0x13211531u; 0x14121143u; 0x24121151u
    0x13212143u; 0x14121242u; 0x13212242u; 0x14121341u; 0x13212341u; 0x14122151u; 0x13213151u; 0x12311126u; 0x22311134u; 0x32311142u; 0x12311225u; 0x22311233u; 0x32311241u
    0x12311324u; 0x22311332u; 0x12311423u; 0x22311431u; 0x12311522u; 0x12311621u; 0x13221134u; 0x23221142u; 0x12312134u; 0x13221233u; 0x23221241u; 0x12312233u; 0x13221332u
    0x12312332u; 0x13221431u; 0x12312431u; 0x14131142u; 0x13222142u; 0x14131241u; 0x12313142u; 0x13222241u; 0x12313241u; 0x21411125u; 0x31411133u; 0x41411141u; 0x11411216u
    0x21411224u; 0x31411232u; 0x11411315u; 0x21411323u; 0x31411331u; 0x11411414u; 0x21411422u; 0x11411513u; 0x21411521u; 0x11411612u; 0x12321125u; 0x22321133u; 0x32321141u
    0x11412125u; 0x12321224u; 0x22321232u; 0x11412224u; 0x21412232u; 0x22321331u; 0x11412323u; 0x12321422u; 0x11412422u; 0x12321521u; 0x11412521u; 0x13231133u; 0x23231141u
    0x12322133u; 0x13231232u; 0x11413133u; 0x12322232u; 0x13231331u; 0x11413232u; 0x12322331u; 0x11413331u; 0x14141141u; 0x13232141u; 0x12323141u; 0x11414141u; 0x11421116u
    0x21421124u; 0x31421132u; 0x11421215u; 0x21421223u; 0x31421231u; 0x11421314u; 0x21421322u; 0x11421413u; 0x21421421u; 0x11421512u; 0x11421611u; 0x12331124u; 0x22331132u
    0x11422124u; 0x12331223u; 0x22331231u; 0x11422223u; 0x21422231u; 0x11422322u; 0x12331421u; 0x11422421u; 0x13241132u; 0x12332132u; 0x13241231u; 0x11423132u; 0x12332231u
    0x11423231u; 0x11431115u; 0x21431123u; 0x31431131u; 0x11431214u; 0x21431222u; 0x11431313u; 0x21431321u; 0x11431412u; 0x11431511u; 0x12341123u; 0x22341131u; 0x11432123u
    0x12341222u; 0x11432222u; 0x12341321u; 0x11432321u; 0x13251131u; 0x12342131u; 0x11433131u; 0x11441114u; 0x21441122u; 0x11441213u; 0x21441221u; 0x11441312u; 0x11441411u
    0x12351122u; 0x11442122u; 0x12351221u; 0x11442221u; 0x11451113u; 0x21451121u; 0x11451212u; 0x11451311u; 0x12361121u; 0x11452121u; 0x15111143u; 0x25111151u; 0x15111242u
    0x15111341u; 0x15112151u; 0x14211134u; 0x24211142u; 0x14211233u; 0x24211241u; 0x14211332u; 0x14211431u; 0x15121142u; 0x14212142u; 0x15121241u; 0x14212241u; 0x13311125u
    0x23311133u; 0x33311141u; 0x13311224u; 0x23311232u; 0x13311323u; 0x23311331u; 0x13311422u; 0x13311521u; 0x14221133u; 0x24221141u; 0x13312133u; 0x14221232u; 0x13312232u
    0x14221331u; 0x13312331u; 0x15131141u; 0x14222141u; 0x13313141u; 0x12411116u; 0x22411124u; 0x32411132u; 0x12411215u; 0x22411223u; 0x32411231u; 0x12411314u; 0x22411322u
    0x12411413u; 0x22411421u; 0x12411512u; 0x12411611u; 0x13321124u; 0x23321132u; 0x12412124u; 0x13321223u; 0x23321231u; 0x12412223u; 0x22412231u; 0x12412322u; 0x13321421u
    0x12412421u; 0x14231132u; 0x13322132u; 0x14231231u; 0x12413132u; 0x13322231u; 0x12413231u; 0x21511115u; 0x31511123u; 0x41511131u; 0x21511214u; 0x31511222u; 0x21511313u
    0x31511321u; 0x21511412u; 0x21511511u; 0x12421115u; 0x22421123u; 0x32421131u; 0x11512115u; 0x12421214u; 0x22421222u; 0x11512214u; 0x21512222u; 0x22421321u; 0x11512313u
    0x12421412u; 0x11512412u; 0x12421511u; 0x11512511u; 0x13331123u; 0x23331131u; 0x12422123u; 0x13331222u; 0x11513123u; 0x12422222u; 0x13331321u; 0x11513222u; 0x12422321u
    0x11513321u; 0x14241131u; 0x13332131u; 0x12423131u; 0x11514131u; 0x21521114u; 0x31521122u; 0x21521213u; 0x31521221u; 0x21521312u; 0x21521411u; 0x12431114u; 0x22431122u
    0x11522114u; 0x12431213u; 0x22431221u; 0x11522213u; 0x21522221u; 0x11522312u; 0x12431411u; 0x11522411u; 0x13341122u; 0x12432122u; 0x13341221u; 0x11523122u; 0x12432221u
    0x11523221u; 0x21531113u; 0x31531121u; 0x21531212u; 0x21531311u; 0x12441113u; 0x22441121u; 0x11532113u; 0x12441212u; 0x11532212u; 0x12441311u; 0x11532311u; 0x13351121u
    0x12442121u; 0x11533121u; 0x21541112u; 0x21541211u; 0x12451112u; 0x11542112u; 0x12451211u; 0x11542211u; 0x16111142u; 0x16111241u; 0x15211133u; 0x25211141u; 0x15211232u
    0x15211331u; 0x16121141u; 0x15212141u; 0x14311124u; 0x24311132u; 0x14311223u; 0x24311231u; 0x14311322u; 0x14311421u; 0x15221132u; 0x14312132u; 0x15221231u; 0x14312231u
    0x13411115u; 0x23411123u; 0x33411131u; 0x13411214u; 0x23411222u; 0x13411313u; 0x23411321u; 0x13411412u; 0x13411511u; 0x14321123u; 0x24321131u; 0x13412123u; 0x23412131u
    0x13412222u; 0x14321321u; 0x13412321u; 0x15231131u; 0x14322131u; 0x13413131u; 0x22511114u; 0x32511122u; 0x22511213u; 0x32511221u; 0x22511312u; 0x22511411u; 0x13421114u
    0x23421122u; 0x12512114u; 0x22512122u; 0x23421221u; 0x12512213u; 0x13421312u; 0x12512312u; 0x13421411u; 0x12512411u; 0x14331122u; 0x13422122u; 0x14331221u; 0x12513122u
    0x13422221u; 0x12513221u; 0x31611113u; 0x41611121u; 0x31611212u; 0x31611311u; 0x22521113u; 0x32521121u; 0x21612113u; 0x22521212u; 0x21612212u; 0x22521311u; 0x21612311u
    0x13431113u; 0x23431121u; 0x12522113u; 0x13431212u; 0x11613113u; 0x12522212u; 0x13431311u; 0x11613212u; 0x12522311u; 0x11613311u; 0x14341121u; 0x13432121u; 0x12523121u
    0x11614121u; 0x31621112u; 0x31621211u; 0x22531112u; 0x21622112u; 0x22531211u; 0x21622211u; 0x13441112u; 0x12532112u; 0x13441211u; 0x11623112u; 0x12532211u; 0x11623211u
    0x31631111u; 0x22541111u; 0x21632111u; 0x13451111u; 0x12542111u; 0x11633111u; 0x16211132u; 0x16211231u; 0x15311123u; 0x25311131u; 0x15311222u; 0x15311321u; 0x16221131u
    0x15312131u; 0x14411114u; 0x24411122u; 0x14411213u; 0x24411221u; 0x14411312u; 0x14411411u; 0x15321122u; 0x14412122u; 0x15321221u; 0x14412221u; 0x23511113u; 0x33511121u
    0x23511212u; 0x23511311u; 0x14421113u; 0x24421121u; 0x13512113u; 0x23512121u; 0x13512212u; 0x14421311u; 0x13512311u; 0x15331121u; 0x14422121u; 0x13513121u; 0x32611112u
    0x32611211u; 0x23521112u; 0x22612112u; 0x23521211u; 0x22612211u; 0x14431112u; 0x13522112u; 0x14431211u; 0x12613112u; 0x13522211u; 0x12613211u; 0x32621111u; 0x23531111u
    0x22622111u; 0x14441111u; 0x13532111u; 0x12623111u; 0x16311122u; 0x16311221u; 0x15411113u; 0x25411121u; 0x15411212u; 0x15411311u; 0x16321121u; 0x15412121u; 0x24511112u
    0x24511211u; 0x15421112u; 0x14512112u; 0x15421211u; 0x14512211u; 0x33611111u
|]

/// All three cluster tables indexed by (row mod 3).
let private CLUSTER_TABLES = [| CLUSTER0; CLUSTER1; CLUSTER2 |]

// ============================================================================
// Pattern expansion helpers
// ============================================================================

/// Expand a packed bar/space pattern into module booleans.
///
/// The 8 element widths are stored 4 bits each in the packed uint32:
///   bits 31..28 = b1, bits 27..24 = s1, bits 23..20 = b2, bits 19..16 = s2,
///   bits 15..12 = b3, bits 11..8  = s3, bits  7..4  = b4, bits  3..0  = s4
///
/// We alternate: bar (dark = true), space (dark = false), bar, space, ...
/// This produces exactly 17 boolean module values (sum of all widths = 17).
let private expandPattern (packed: uint32) (buf: bool[]) (offset: int) : unit =
    let b1 = int ((packed >>> 28) &&& 0xfu)
    let s1 = int ((packed >>> 24) &&& 0xfu)
    let b2 = int ((packed >>> 20) &&& 0xfu)
    let s2 = int ((packed >>> 16) &&& 0xfu)
    let b3 = int ((packed >>> 12) &&& 0xfu)
    let s3 = int ((packed >>>  8) &&& 0xfu)
    let b4 = int ((packed >>>  4) &&& 0xfu)
    let s4 = int ( packed         &&& 0xfu)
    let mutable pos = offset
    for _ in 1 .. b1 do buf.[pos] <- true;  pos <- pos + 1
    for _ in 1 .. s1 do buf.[pos] <- false; pos <- pos + 1
    for _ in 1 .. b2 do buf.[pos] <- true;  pos <- pos + 1
    for _ in 1 .. s2 do buf.[pos] <- false; pos <- pos + 1
    for _ in 1 .. b3 do buf.[pos] <- true;  pos <- pos + 1
    for _ in 1 .. s3 do buf.[pos] <- false; pos <- pos + 1
    for _ in 1 .. b4 do buf.[pos] <- true;  pos <- pos + 1
    for _ in 1 .. s4 do buf.[pos] <- false; pos <- pos + 1

/// Expand a bar/space width array into module booleans.
///
/// The first element is always a bar (dark = true). Each subsequent element
/// alternates between space and bar.
let private expandWidths (widths: int[]) (buf: bool[]) (offset: int) : int =
    let mutable dark = true
    let mutable pos = offset
    for w in widths do
        for _ in 1 .. w do
            buf.[pos] <- dark
            pos <- pos + 1
        dark <- not dark
    pos

// ============================================================================
// Rasterization
// ============================================================================

/// Convert the flat codeword sequence into a ModuleGrid.
///
/// Row anatomy (modules, left to right):
///   start(17) | LRI(17) | data[0..c-1](17 each) | RRI(17) | stop(18)
///   Total = 17 + 17 + 17c + 17 + 18 = 69 + 17c modules per row.
///
/// Each logical PDF417 row is written `rowHeight` times into the grid
/// (identical module rows, stacked vertically).
let private rasterize
    (sequence : int[])
    (rows     : int)
    (cols     : int)
    (eccLevel : int)
    (rowHeight: int) : ModuleGrid =

    // Total module columns and rows in the output grid.
    let moduleWidth  = 69 + 17 * cols
    let moduleHeight = rows * rowHeight

    // Precompute start and stop module sequences (same for every row).
    let startModules = Array.zeroCreate 17
    let _ = expandWidths START_WIDTHS startModules 0

    let stopModules = Array.zeroCreate 18
    let _ = expandWidths STOP_WIDTHS stopModules 0

    // Build the grid as a plain bool[][] first (faster than immutable setModule).
    let rawGrid = Array.init moduleHeight (fun _ -> Array.create moduleWidth false)

    for r in 0 .. rows - 1 do
        let cluster = r % 3
        let clusterTable = CLUSTER_TABLES.[cluster]

        // Build the complete module row for row r.
        let rowBuf = Array.create moduleWidth false

        // 1. Start pattern (17 modules).
        Array.blit startModules 0 rowBuf 0 17
        let mutable pos = 17

        // 2. Left Row Indicator (17 modules).
        let lri = computeLRI r rows cols eccLevel
        expandPattern clusterTable.[lri] rowBuf pos
        pos <- pos + 17

        // 3. Data codewords (17 modules each).
        for j in 0 .. cols - 1 do
            let cw = sequence.[r * cols + j]
            expandPattern clusterTable.[cw] rowBuf pos
            pos <- pos + 17

        // 4. Right Row Indicator (17 modules).
        let rri = computeRRI r rows cols eccLevel
        expandPattern clusterTable.[rri] rowBuf pos
        pos <- pos + 17

        // 5. Stop pattern (18 modules).
        Array.blit stopModules 0 rowBuf pos 18

        // Write this module row `rowHeight` times.
        let moduleRowBase = r * rowHeight
        for h in 0 .. rowHeight - 1 do
            rawGrid.[moduleRowBase + h] <- Array.copy rowBuf

    // Wrap in a ModuleGrid record (re-use the raw arrays directly).
    { Rows = moduleHeight
      Cols = moduleWidth
      Modules = rawGrid
      ModuleShape = Square }

// ============================================================================
// Main encoder: encode
// ============================================================================

/// Encode raw bytes as a PDF417 symbol and return the ModuleGrid.
///
/// ### Arguments
///   bytes   — the raw byte payload to encode
///   options — encoding options (ECC level, columns, row height)
///
/// ### Returns
///   Ok ModuleGrid  on success
///   Error PDF417Error on invalid options or data too large
///
/// ### Algorithm (full pipeline)
///
///   1. Byte-compact the input (codeword 924 latch + base-900 groups).
///   2. Choose ECC level (from options or auto-select).
///   3. Compute the length descriptor (total codewords including ECC).
///   4. Reed-Solomon encode over GF(929) with b=3 convention.
///   5. Choose symbol dimensions (rows × cols).
///   6. Pad to fill the grid exactly.
///   7. Rasterize: start pattern + LRI + data + RRI + stop pattern per row.
///   8. Return the ModuleGrid.
let encode (bytes: byte[]) (options: PDF417Options) : Result<ModuleGrid, PDF417Error> =
    // ── Validate ECC level ────────────────────────────────────────────────────
    match options.EccLevel with
    | Some l when l < 0 || l > 8 ->
        Error (InvalidECCLevel (sprintf "ECC level must be 0–8, got %d" l))
    | _ ->

    // ── Validate columns ──────────────────────────────────────────────────────
    match options.Columns with
    | Some c when c < MIN_COLS || c > MAX_COLS ->
        Error (InvalidDimensions (sprintf "columns must be %d–%d, got %d" MIN_COLS MAX_COLS c))
    | _ ->

    // ── Byte compaction ───────────────────────────────────────────────────────
    let dataCwords = byteCompact bytes

    // ── Auto-select ECC level ─────────────────────────────────────────────────
    let eccLevel = options.EccLevel |> Option.defaultWith (fun () -> autoEccLevel (dataCwords.Length + 1))
    let eccCount = 1 <<< (eccLevel + 1)   // 2^(eccLevel+1)

    // ── Length descriptor ─────────────────────────────────────────────────────
    // The length descriptor (codeword index 0) counts: itself + all data
    // codewords + all ECC codewords. It does NOT include padding.
    let lengthDesc = 1 + dataCwords.Length + eccCount
    let fullData = Array.append [| lengthDesc |] dataCwords

    // ── RS ECC ────────────────────────────────────────────────────────────────
    let eccCwords = rsEncode fullData eccLevel

    // ── Choose dimensions ──────────────────────────────────────────────────────
    let totalCwords = fullData.Length + eccCwords.Length

    match options.Columns with
    | Some userCols ->
        let rows = max MIN_ROWS (int (Math.Ceiling(float totalCwords / float userCols)))
        if rows > MAX_ROWS then
            Error (InputTooLong (sprintf "Data requires %d rows (max %d) with %d columns" rows MAX_ROWS userCols))
        elif userCols * rows < totalCwords then
            Error (InputTooLong (sprintf "Cannot fit %d codewords in %d×%d grid" totalCwords rows userCols))
        else
            let paddingCount = userCols * rows - totalCwords
            let paddedData = Array.append fullData (Array.create paddingCount PADDING_CW)
            let fullSequence = Array.append paddedData eccCwords
            let rowHeight = max 1 (options.RowHeight |> Option.defaultValue 3)
            Ok (rasterize fullSequence rows userCols eccLevel rowHeight)

    | None ->
        match chooseDimensions totalCwords with
        | Error e -> Error e
        | Ok (cols, rows) ->
            let paddingCount = cols * rows - totalCwords
            let paddedData = Array.append fullData (Array.create paddingCount PADDING_CW)
            let fullSequence = Array.append paddedData eccCwords
            let rowHeight = max 1 (options.RowHeight |> Option.defaultValue 3)
            Ok (rasterize fullSequence rows cols eccLevel rowHeight)

/// Encode a UTF-8 string as a PDF417 symbol and return the ModuleGrid.
let encodeString (text: string) (options: PDF417Options) : Result<ModuleGrid, PDF417Error> =
    encode (Text.Encoding.UTF8.GetBytes(text)) options

// ============================================================================
// Exported internals for testing
// ============================================================================

/// Internal functions exported for unit testing.
/// Not part of the public API — do not depend on these in production code.
module Internal =
    let gfMulExported    = gfMul
    let gfAddExported    = gfAdd
    let GF_EXP_TABLE     = GF_EXP
    let GF_LOG_TABLE     = GF_LOG
    let byteCompactExported   = byteCompact
    let rsEncodeExported      = rsEncode
    let buildGeneratorExported = buildGenerator
    let autoEccLevelExported  = autoEccLevel
