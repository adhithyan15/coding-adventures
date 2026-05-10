using System.Text;
using CodingAdventures.Barcode2D;

namespace CodingAdventures.DataMatrix;

// =============================================================================
// DataMatrix.cs — ISO/IEC 16022:2006 Data Matrix ECC 200 Encoder
// =============================================================================
//
// Data Matrix is a 2D matrix barcode invented in 1989 by RVSI Acuity CiMatrix
// and standardized as ISO/IEC 16022:2006. The ECC 200 variant uses Reed-Solomon
// error correction over GF(256) and has replaced all older ECC 000–140 lineages.
//
// Where Data Matrix appears in the world:
//   - PCBs: etched Data Matrix identifies every board through automated assembly.
//   - Pharmaceuticals: US FDA DSCSA mandates it on unit-dose packages.
//   - Aerospace: dot-peened or laser-etched on metal parts that survive decades.
//   - Medical devices: GS1 DataMatrix on surgical instruments and implants.
//
// ## Why Data Matrix, not QR Code?
//
// Data Matrix:                          QR Code:
//   - 36 symbol sizes                     - 40 symbol sizes
//   - No masking step needed              - 8 mask patterns evaluated
//   - L-shaped finder + clock border      - Three separate 7×7 finder patterns
//   - Diagonal "Utah" placement           - Column-based placement
//   - GF(256) with poly 0x12D            - GF(256) with poly 0x11D
//   - Reed-Solomon roots α¹…αⁿ (b=1)    - Reed-Solomon roots α⁰…αⁿ (b=0)
//
// The diagonal Utah placement is Data Matrix's defining structural feature.
// It distributes codeword bits evenly without masking. The L-finder's asymmetry
// identifies all four 90° rotations in a single border pass.
//
// ## Encoding pipeline
//
//   input string
//     → ASCII encoding  (chars+1; digit pairs packed into one codeword)
//     → symbol selection (smallest symbol whose capacity ≥ codeword count)
//     → pad to capacity  (scrambled-pad codewords fill unused slots)
//     → RS blocks + ECC  (GF(256)/0x12D, b=1 convention)
//     → interleave blocks (data round-robin then ECC round-robin)
//     → grid init        (L-finder + timing border + alignment borders)
//     → Utah placement   (diagonal codeword placement, NO masking)
//     → ModuleGrid       (abstract boolean grid, true = dark)

// =============================================================================
// Exception types
// =============================================================================

/// <summary>Base class for all Data Matrix encoding errors.</summary>
public class DataMatrixException : Exception
{
    /// <summary>Create a Data Matrix exception with a message.</summary>
    public DataMatrixException(string message) : base(message) { }
}

/// <summary>
/// Thrown when the encoded data exceeds the capacity of the largest Data Matrix
/// symbol (144×144, up to 1558 data codewords).
/// </summary>
public sealed class DataMatrixInputTooLongException : DataMatrixException
{
    /// <summary>Number of encoded codewords.</summary>
    public int EncodedCW { get; }

    /// <summary>Maximum data codeword capacity (1558 for 144×144).</summary>
    public int MaxCW { get; }

    /// <summary>Create an input-too-long error.</summary>
    public DataMatrixInputTooLongException(int encodedCW, int maxCW)
        : base($"Data Matrix: input too long — encoded {encodedCW} codewords, maximum is {maxCW} (144×144 symbol)")
    {
        EncodedCW = encodedCW;
        MaxCW = maxCW;
    }
}

// =============================================================================
// DataMatrixOptions — symbol selection control
// =============================================================================

/// <summary>Controls which symbol shapes the encoder considers during selection.</summary>
public enum DataMatrixSymbolShape
{
    /// <summary>
    /// Select from square symbols only (default). Square symbols (10×10 through
    /// 144×144) are the most common Data Matrix variant and preferred for most
    /// applications.
    /// </summary>
    Square,

    /// <summary>
    /// Select from rectangular symbols only. Rectangles (8×18 through 16×48)
    /// are used when print area aspect ratio matters.
    /// </summary>
    Rectangular,

    /// <summary>
    /// Try both square and rectangular, pick the smallest by total module count.
    /// </summary>
    Any,
}

/// <summary>Options controlling Data Matrix encoding.</summary>
public sealed record DataMatrixOptions(DataMatrixSymbolShape Shape = DataMatrixSymbolShape.Square)
{
    /// <summary>Default options: square symbol selection.</summary>
    public static readonly DataMatrixOptions Default = new();
}

// =============================================================================
// SymbolEntry — describes one Data Matrix ECC 200 symbol size
// =============================================================================
//
// A "data region" is one rectangular interior sub-area. Small symbols (≤ 26×26)
// have a single 1×1 region. Larger symbols subdivide into a grid of regions
// separated by alignment borders (2 modules wide each).
//
// The Utah placement algorithm works on the "logical data matrix" — the
// concatenation of all region interiors — then maps back to physical coords.

internal sealed record SymbolEntry(
    int SymbolRows,     // total symbol size including outer border
    int SymbolCols,
    int RegionRows,     // how many data region rows (rr)
    int RegionCols,     // how many data region cols (rc)
    int RegionHeight,   // interior data size per region
    int RegionWidth,
    int DataCW,         // total data codeword capacity
    int EccCW,          // total ECC codewords
    int NumBlocks,      // number of interleaved RS blocks
    int EccPerBlock);   // ECC codewords per block (same for all blocks)

// =============================================================================
// DataMatrix — the public encoder
// =============================================================================

/// <summary>
/// ISO/IEC 16022:2006 Data Matrix ECC 200 encoder.
///
/// <para>
/// Produces a <see cref="ModuleGrid"/> where <c>true</c> = dark module and
/// <c>false</c> = light module. The grid is ready for rendering via
/// <see cref="CodingAdventures.Barcode2D.Barcode2D.Layout"/>.
/// </para>
///
/// <para>
/// This implementation supports all 30 square symbol sizes (10×10 through
/// 144×144) and all 6 rectangular symbol sizes (8×18 through 16×48) from
/// ISO/IEC 16022:2006, Table 7. ASCII mode encoding with digit-pair
/// compression is used. C40, Text, EDIFACT, X12 and Base256 modes are
/// planned for v0.2.0.
/// </para>
///
/// <example>
/// <code>
/// // Smallest symbol that fits "HELLO WORLD"
/// var grid = DataMatrix.Encode("HELLO WORLD");
///
/// // Force rectangular symbols
/// var rect = DataMatrix.Encode("ABC", new DataMatrixOptions(DataMatrixSymbolShape.Rectangular));
///
/// // Raw bytes (useful for binary payloads)
/// var binary = DataMatrix.EncodeBytes(new byte[] { 0x01, 0xFF, 0x7E });
/// </code>
/// </example>
/// </summary>
public static class DataMatrix
{
    /// <summary>Package version.</summary>
    public const string Version = "0.1.0";

    // =========================================================================
    // GF(256) over 0x12D — Data Matrix field
    // =========================================================================
    //
    // Data Matrix uses GF(256) with primitive polynomial 0x12D:
    //
    //   p(x) = x^8 + x^5 + x^4 + x^2 + x + 1  =  0x12D  =  301
    //
    // IMPORTANT: this is DIFFERENT from QR Code's 0x11D polynomial.
    // Both are degree-8 irreducible polynomials over GF(2), but the multiplicative
    // groups they generate have different structure. Never mix tables.
    //
    // The generator α = 2 (polynomial x) is primitive: α^1, α^2, … α^255 = 1
    // covers all 255 non-zero field elements exactly once.
    //
    // Pre-computed tables:
    //   _gfExp[i] = α^i mod 0x12D   (i = 0..255, with index 255 wrapping to 0)
    //   _gfLog[v] = k such that α^k = v  (v = 1..255; _gfLog[0] = 0 sentinel)

    private const int GfPoly = 0x12D;

    private static readonly byte[] GfExp = new byte[256]; // α^i table
    private static readonly byte[] GfLog = new byte[256]; // log_α(v) table

    // Generator polynomial cache: ECC length → polynomial coefficients.
    // Polynomials are built lazily and cached for subsequent calls.
    private static readonly Dictionary<int, byte[]> GenPolyCache = new();

    static DataMatrix()
    {
        // Build exp (antilog) and log tables for GF(256)/0x12D.
        //
        // Algorithm: start with val = 1 (= α^0).
        // Each step: left-shift 1 bit (multiply by α = x).
        // If bit 8 is set (val >= 256), XOR with 0x12D to reduce mod the field poly.
        //
        // After 255 steps we visit all 255 non-zero elements exactly once,
        // proving α = 2 is primitive for polynomial 0x12D.
        var val = 1;
        for (var i = 0; i < 255; i++)
        {
            GfExp[i] = (byte)val;
            GfLog[val] = (byte)i;
            val <<= 1;
            if ((val & 0x100) != 0)
            {
                // Overflow: reduce modulo 0x12D.
                // Since x^8 ≡ x^5 + x^4 + x^2 + x + 1 in this field,
                // we XOR with 0x12D to remove the degree-8 term.
                val ^= GfPoly;
            }
        }
        // α^255 = α^0 = 1 (the multiplicative order of α is 255)
        GfExp[255] = GfExp[0];

        // Pre-build all generator polynomials needed for the symbol table so the
        // first call to Encode() has no per-symbol latency from lazy construction.
        var seen = new HashSet<int>();
        foreach (var e in SquareSizes.Concat(RectSizes))
        {
            if (seen.Add(e.EccPerBlock))
            {
                GetGenerator(e.EccPerBlock);
            }
        }
    }

    // ─────────────────────────────────────────────────────────────────────────
    // GF(256)/0x12D multiplication
    // ─────────────────────────────────────────────────────────────────────────

    /// <summary>
    /// Multiply two GF(256)/0x12D field elements.
    ///
    /// <para>
    /// For a, b ≠ 0: a × b = α^{(log[a] + log[b]) mod 255}.
    /// If either operand is zero the product is zero — zero absorbs multiplication.
    /// </para>
    ///
    /// <para>
    /// Using log/exp tables turns polynomial multiplication + reduction into
    /// two table lookups and a modular addition — effectively O(1).
    /// </para>
    /// </summary>
    private static byte GfMul(byte a, byte b)
    {
        if (a == 0 || b == 0) return 0;
        return GfExp[(uint)(GfLog[a] + GfLog[b]) % 255];
    }

    // ─────────────────────────────────────────────────────────────────────────
    // Generator polynomial construction
    // ─────────────────────────────────────────────────────────────────────────

    /// <summary>
    /// Build the RS generator polynomial for <paramref name="nEcc"/> ECC bytes
    /// over GF(256)/0x12D with the b=1 convention:
    ///
    /// <c>g(x) = (x + α¹)(x + α²)···(x + α^{nEcc})</c>
    ///
    /// <para>
    /// Algorithm: start with g = [1], then for each i from 1 to nEcc, multiply
    /// g by the linear factor (x + α^i):
    ///   for j, coeff in g:
    ///     newG[j]   ^= coeff           (coeff × x term)
    ///     newG[j+1] ^= coeff × α^i     (coeff × constant term)
    /// </para>
    ///
    /// <para>
    /// Result format: big-endian, including the implicit leading 1.
    /// Length = nEcc + 1.
    /// </para>
    /// </summary>
    private static byte[] BuildGenerator(int nEcc)
    {
        var g = new byte[] { 1 };
        for (var i = 1; i <= nEcc; i++)
        {
            var ai = GfExp[i]; // α^i
            var newG = new byte[g.Length + 1];
            for (var j = 0; j < g.Length; j++)
            {
                newG[j] ^= g[j];              // g[j] × x term
                newG[j + 1] ^= GfMul(g[j], ai); // g[j] × α^i constant term
            }
            g = newG;
        }
        return g;
    }

    /// <summary>
    /// Return the generator polynomial for <paramref name="nEcc"/> ECC bytes,
    /// building and caching it on first use.
    /// </summary>
    private static byte[] GetGenerator(int nEcc)
    {
        if (!GenPolyCache.TryGetValue(nEcc, out var g))
        {
            g = BuildGenerator(nEcc);
            GenPolyCache[nEcc] = g;
        }
        return g;
    }

    // =========================================================================
    // Symbol size tables (from ISO/IEC 16022:2006, Table 7)
    // =========================================================================
    //
    // Column order per entry:
    //   SymbolRows, SymbolCols,
    //   RegionRows, RegionCols,
    //   RegionHeight, RegionWidth,
    //   DataCW, EccCW,
    //   NumBlocks, EccPerBlock
    //
    // "Data regions" are the interior rectangular sub-areas within the symbol.
    // Small symbols (≤ 26×26) have a single 1×1 region. Larger symbols split
    // the interior into a grid of regions separated by 2-module alignment borders.

    private static readonly SymbolEntry[] SquareSizes =
    [
        // Single-region symbols (1×1 data region, no interior alignment borders)
        new(10,  10,  1, 1,  8,  8,    3,   5,  1,  5),
        new(12,  12,  1, 1, 10, 10,    5,   7,  1,  7),
        new(14,  14,  1, 1, 12, 12,    8,  10,  1, 10),
        new(16,  16,  1, 1, 14, 14,   12,  12,  1, 12),
        new(18,  18,  1, 1, 16, 16,   18,  14,  1, 14),
        new(20,  20,  1, 1, 18, 18,   22,  18,  1, 18),
        new(22,  22,  1, 1, 20, 20,   30,  20,  1, 20),
        new(24,  24,  1, 1, 22, 22,   36,  24,  1, 24),
        new(26,  26,  1, 1, 24, 24,   44,  28,  1, 28),

        // Multi-region symbols (2×2 data regions, one horizontal + one vertical AB)
        new(32,  32,  2, 2, 14, 14,   62,  36,  2, 18),
        new(36,  36,  2, 2, 16, 16,   86,  42,  2, 21),
        new(40,  40,  2, 2, 18, 18,  114,  48,  2, 24),
        new(44,  44,  2, 2, 20, 20,  144,  56,  4, 14),
        new(48,  48,  2, 2, 22, 22,  174,  68,  4, 17),
        new(52,  52,  2, 2, 24, 24,  204,  84,  4, 21),

        // Large symbols (4×4 data regions)
        new(64,  64,  4, 4, 14, 14,  280, 112,  4, 28),
        new(72,  72,  4, 4, 16, 16,  368, 144,  4, 36),
        new(80,  80,  4, 4, 18, 18,  456, 192,  4, 48),
        new(88,  88,  4, 4, 20, 20,  576, 224,  4, 56),
        new(96,  96,  4, 4, 22, 22,  696, 272,  4, 68),
        new(104, 104, 4, 4, 24, 24,  816, 336,  6, 56),

        // Very large symbols (6×6 data regions)
        new(120, 120, 6, 6, 18, 18, 1050, 408,  6, 68),
        new(132, 132, 6, 6, 20, 20, 1304, 496,  8, 62),
        new(144, 144, 6, 6, 22, 22, 1558, 620, 10, 62),
    ];

    private static readonly SymbolEntry[] RectSizes =
    [
        // Single-region rectangular symbols
        new( 8, 18, 1, 1,  6, 16,   5,   7, 1,  7),
        new( 8, 32, 1, 2,  6, 14,  10,  11, 1, 11),
        new(12, 26, 1, 1, 10, 24,  16,  14, 1, 14),
        new(12, 36, 1, 2, 10, 16,  22,  18, 1, 18),
        new(16, 36, 1, 2, 14, 16,  32,  24, 1, 24),
        new(16, 48, 1, 2, 14, 22,  49,  28, 1, 28),
    ];

    // =========================================================================
    // Public API
    // =========================================================================

    /// <summary>Encode a UTF-8 string with default options (square symbol).</summary>
    /// <param name="data">The text to encode. All ASCII chars (0–127) are supported. Extended ASCII (128–255) uses UPPER_SHIFT and costs two codewords per character.</param>
    /// <returns>A <see cref="ModuleGrid"/> representing the Data Matrix symbol.</returns>
    /// <exception cref="DataMatrixInputTooLongException">
    /// Thrown when the encoded data exceeds the largest symbol's capacity (1558 codewords for 144×144).
    /// </exception>
    public static ModuleGrid Encode(string data) => Encode(data, DataMatrixOptions.Default);

    /// <summary>Encode a UTF-8 string with the given options.</summary>
    public static ModuleGrid Encode(string data, DataMatrixOptions? options)
    {
        ArgumentNullException.ThrowIfNull(data);
        return EncodeBytes(Encoding.UTF8.GetBytes(data), options);
    }

    /// <summary>Encode raw bytes with default options.</summary>
    public static ModuleGrid EncodeBytes(byte[] data) => EncodeBytes(data, DataMatrixOptions.Default);

    /// <summary>
    /// Encode raw bytes with the given options.
    ///
    /// <para>
    /// Full encoding pipeline:
    /// <list type="number">
    ///   <item>ASCII encode (digit-pair compression)</item>
    ///   <item>Symbol selection (smallest fitting)</item>
    ///   <item>Pad to symbol capacity (scrambled pad codewords)</item>
    ///   <item>RS block computation (GF(256)/0x12D, b=1)</item>
    ///   <item>Data + ECC interleaving</item>
    ///   <item>Grid initialization (finder + timing + alignment borders)</item>
    ///   <item>Utah diagonal placement</item>
    ///   <item>Logical-to-physical coordinate mapping</item>
    /// </list>
    /// </para>
    /// </summary>
    public static ModuleGrid EncodeBytes(byte[] data, DataMatrixOptions? options)
    {
        ArgumentNullException.ThrowIfNull(data);
        options ??= DataMatrixOptions.Default;

        // Step 1: ASCII encode with digit-pair compression.
        var codewords = EncodeAscii(data);

        // Step 2: Select the smallest symbol that fits.
        var entry = SelectSymbol(codewords.Length, options.Shape);

        // Step 3: Pad to the symbol's data codeword capacity.
        var padded = PadCodewords(codewords, entry.DataCW);

        // Steps 4–5: Compute RS ECC for each block, then interleave.
        var interleaved = ComputeInterleaved(padded, entry);

        // Step 6: Initialize the physical grid with border and alignment borders.
        var physGrid = InitGrid(entry);

        // Step 7: Run Utah diagonal placement on the logical data matrix.
        var nRows = entry.RegionRows * entry.RegionHeight;
        var nCols = entry.RegionCols * entry.RegionWidth;
        var logicalGrid = UtahPlacement(interleaved, nRows, nCols);

        // Step 8: Map logical coordinates to physical symbol coordinates.
        for (var r = 0; r < nRows; r++)
        {
            for (var c = 0; c < nCols; c++)
            {
                var (pr, pc) = LogicalToPhysical(r, c, entry);
                physGrid[pr][pc] = logicalGrid[r][c];
            }
        }

        // Step 9: Convert the 2D bool array to an immutable ModuleGrid.
        return ToModuleGrid(physGrid, entry.SymbolRows, entry.SymbolCols);
    }

    // =========================================================================
    // ASCII Encoding (ISO/IEC 16022:2006 §5.2)
    // =========================================================================

    /// <summary>
    /// Encode input bytes in Data Matrix ASCII mode.
    ///
    /// <para>
    /// ASCII mode rules:
    /// <list type="bullet">
    ///   <item>
    ///     <term>Digit pair</term>
    ///     <description>
    ///       Two consecutive ASCII digits (0x30–0x39) → one codeword = 130 + (d1×10 + d2).
    ///       This halves the codeword budget for numeric strings — critical for
    ///       manufacturing lot codes and serial numbers.
    ///     </description>
    ///   </item>
    ///   <item>
    ///     <term>Single ASCII char (0–127)</term>
    ///     <description>Codeword = ASCII_value + 1. Example: 'A' (65) → 66.</description>
    ///   </item>
    ///   <item>
    ///     <term>Extended ASCII (128–255)</term>
    ///     <description>
    ///       Two codewords: UPPER_SHIFT (235), then ASCII_value − 127.
    ///       Enables encoding Latin-1 / Windows-1252 characters.
    ///     </description>
    ///   </item>
    /// </list>
    /// </para>
    ///
    /// <example>
    /// <code>
    /// "A"    → [66]           (65 + 1)
    /// "12"   → [142]          (130 + 12, digit pair)
    /// "1234" → [142, 174]     (two digit pairs)
    /// "1A"   → [50, 66]       (one digit + one letter — no pair since 'A' is not a digit)
    /// "00"   → [130]          (130 + 0)
    /// "99"   → [229]          (130 + 99)
    /// </code>
    /// </example>
    /// </summary>
    internal static byte[] EncodeAscii(byte[] input)
    {
        var codewords = new List<byte>(input.Length);
        var i = 0;
        while (i < input.Length)
        {
            var c = input[i];
            // Greedy digit-pair check: if the current byte AND the next byte are
            // both ASCII digits ('0' = 0x30 through '9' = 0x39), pack them together.
            if (c >= 0x30 && c <= 0x39
                && i + 1 < input.Length
                && input[i + 1] >= 0x30 && input[i + 1] <= 0x39)
            {
                var d1 = c - 0x30;
                var d2 = input[i + 1] - 0x30;
                codewords.Add((byte)(130 + d1 * 10 + d2));
                i += 2;
            }
            else if (c <= 127)
            {
                // Standard single ASCII character.
                codewords.Add((byte)(c + 1));
                i++;
            }
            else
            {
                // Extended ASCII (128–255): UPPER_SHIFT prefix then adjusted value.
                codewords.Add(235);                  // UPPER_SHIFT codeword
                codewords.Add((byte)(c - 127));      // adjusted value
                i++;
            }
        }
        return codewords.ToArray();
    }

    // =========================================================================
    // Pad codewords (ISO/IEC 16022:2006 §5.2.3)
    // =========================================================================

    /// <summary>
    /// Pad the encoded codeword slice to exactly <paramref name="dataCW"/> bytes.
    ///
    /// <para>
    /// Padding rules (ISO/IEC 16022:2006 §5.2.3):
    /// <list type="number">
    ///   <item>The first pad byte is always the literal value 129.</item>
    ///   <item>
    ///     Subsequent pads are scrambled using a position-dependent formula:
    ///     <code>scrambled = 129 + (149 × k mod 253) + 1</code>
    ///     where k is the 1-indexed position within the full codeword stream.
    ///     If scrambled &gt; 254: scrambled -= 254.
    ///   </item>
    /// </list>
    /// </para>
    ///
    /// <para>
    /// The scrambling prevents a run of "129 129 129 …" from creating a
    /// degenerate Utah placement pattern. It distributes pad codewords across
    /// the symbol as if they were random data.
    /// </para>
    ///
    /// <example>
    /// For "A" (codeword [66]) in a 10×10 symbol (dataCW = 3):
    /// <code>
    /// k=2: 129                   (first pad — always literal)
    /// k=3: 129 + (149×3 mod 253) + 1 = 129 + 194 + 1 = 324; 324 > 254 → 70
    /// Result: [66, 129, 70]
    /// </code>
    /// </example>
    /// </summary>
    internal static byte[] PadCodewords(byte[] codewords, int dataCW)
    {
        if (codewords.Length >= dataCW)
        {
            return codewords.Take(dataCW).ToArray();
        }

        var padded = new byte[dataCW];
        codewords.CopyTo(padded, 0);

        var isFirst = true;
        // k is 1-indexed from the start of the full codeword stream.
        var k = codewords.Length + 1;
        for (var pos = codewords.Length; pos < dataCW; pos++)
        {
            if (isFirst)
            {
                padded[pos] = 129;
                isFirst = false;
            }
            else
            {
                // Scrambled pad formula from ISO §5.2.3.
                // Note: this uses mod 253 (not 255 like Base256 scrambling).
                var scrambled = 129 + (149 * k) % 253 + 1;
                if (scrambled > 254) scrambled -= 254;
                padded[pos] = (byte)scrambled;
            }
            k++;
        }
        return padded;
    }

    // =========================================================================
    // Symbol selection
    // =========================================================================

    /// <summary>
    /// Select the smallest symbol that can hold <paramref name="codewordCount"/>
    /// data codewords, given the shape preference.
    ///
    /// <para>
    /// Iterates all candidates in ascending capacity order and returns the first
    /// whose <c>DataCW ≥ codewordCount</c>.
    /// </para>
    /// </summary>
    /// <exception cref="DataMatrixInputTooLongException">
    /// Thrown when no symbol fits the data.
    /// </exception>
    internal static SymbolEntry SelectSymbol(int codewordCount, DataMatrixSymbolShape shape)
    {
        IEnumerable<SymbolEntry> candidates = shape switch
        {
            DataMatrixSymbolShape.Square => SquareSizes,
            DataMatrixSymbolShape.Rectangular => RectSizes,
            DataMatrixSymbolShape.Any => SquareSizes
                .Concat(RectSizes)
                .OrderBy(e => e.DataCW)
                .ThenBy(e => e.SymbolRows * e.SymbolCols),
            _ => SquareSizes,
        };

        foreach (var entry in candidates)
        {
            if (entry.DataCW >= codewordCount) return entry;
        }

        throw new DataMatrixInputTooLongException(codewordCount, 1558);
    }

    // =========================================================================
    // Block splitting, ECC, and interleaving
    // =========================================================================

    /// <summary>
    /// Split padded data across RS blocks, compute ECC for each block, then
    /// interleave data and ECC round-robin for placement.
    ///
    /// <para>
    /// Block splitting:
    /// <code>
    /// baseLen     = dataCW / numBlocks
    /// extraBlocks = dataCW mod numBlocks
    /// Blocks 0..extraBlocks-1 get baseLen+1 data codewords.
    /// Blocks extraBlocks..numBlocks-1 get baseLen data codewords.
    /// </code>
    /// </para>
    ///
    /// <para>
    /// Interleaving distributes burst errors across all blocks. A scratch
    /// destroying N contiguous modules affects at most ⌈N/numBlocks⌉
    /// codewords per block — far more likely within each block's correction
    /// capacity than a solid contiguous chunk.
    /// </para>
    ///
    /// <para>
    /// Interleaving order:
    /// <code>
    /// data: for pos in 0..maxDataPerBlock: for blk: append data[blk][pos]
    /// ECC:  for pos in 0..eccPerBlock:     for blk: append ecc[blk][pos]
    /// </code>
    /// </para>
    /// </summary>
    internal static byte[] ComputeInterleaved(byte[] data, SymbolEntry entry)
    {
        var numBlocks = entry.NumBlocks;
        var eccPerBlock = entry.EccPerBlock;
        var dataCW = entry.DataCW;
        var gen = GetGenerator(eccPerBlock);

        // Split data into blocks. Earlier blocks get one extra codeword if
        // dataCW is not evenly divisible by numBlocks.
        var baseLen = dataCW / numBlocks;
        var extraBlocks = dataCW % numBlocks;

        var dataBlocks = new byte[numBlocks][];
        var offset = 0;
        for (var b = 0; b < numBlocks; b++)
        {
            var len = b < extraBlocks ? baseLen + 1 : baseLen;
            dataBlocks[b] = data[offset..(offset + len)];
            offset += len;
        }

        // Compute RS ECC for each block independently.
        var eccBlocks = new byte[numBlocks][];
        for (var b = 0; b < numBlocks; b++)
        {
            eccBlocks[b] = RsEncodeBlock(dataBlocks[b], gen);
        }

        // Interleave: data round-robin, then ECC round-robin.
        var total = dataCW + numBlocks * eccPerBlock;
        var interleaved = new List<byte>(total);

        var maxDataLen = dataBlocks.Max(db => db.Length);
        for (var pos = 0; pos < maxDataLen; pos++)
        {
            foreach (var db in dataBlocks)
            {
                if (pos < db.Length) interleaved.Add(db[pos]);
            }
        }
        for (var pos = 0; pos < eccPerBlock; pos++)
        {
            foreach (var eb in eccBlocks)
            {
                interleaved.Add(eb[pos]);
            }
        }

        return interleaved.ToArray();
    }

    /// <summary>
    /// Compute <paramref name="nEcc"/> ECC bytes for one data block using the
    /// LFSR polynomial division method (identical to the ISO LFSR approach).
    ///
    /// <para>
    /// Algorithm: R(x) = D(x) × x^{nEcc} mod G(x), implemented as:
    /// <code>
    /// for each data byte d:
    ///     feedback = d XOR rem[0]
    ///     shift rem left: rem[i] ← rem[i+1]
    ///     rem[nEcc-1] = 0
    ///     if feedback != 0:
    ///         for i in 0..nEcc-1: rem[i] ^= gen[i+1] × feedback
    /// </code>
    /// </para>
    ///
    /// <para>
    /// The generator array format (big-endian, from <see cref="BuildGenerator"/>):
    /// gen[0] = 1 (leading coefficient), gen[1..nEcc] = remaining coefficients.
    /// </para>
    /// </summary>
    internal static byte[] RsEncodeBlock(byte[] data, byte[] generator)
    {
        var nEcc = generator.Length - 1;
        var rem = new byte[nEcc]; // initialized to 0

        foreach (var d in data)
        {
            var fb = (byte)(d ^ rem[0]);
            // Shift the LFSR register left by one position.
            Array.Copy(rem, 1, rem, 0, nEcc - 1);
            rem[nEcc - 1] = 0;
            if (fb != 0)
            {
                // XOR each register position with generator[i+1] × feedback.
                for (var i = 0; i < nEcc; i++)
                {
                    rem[i] ^= GfMul(generator[i + 1], fb);
                }
            }
        }
        return rem;
    }

    // =========================================================================
    // Grid initialization
    // =========================================================================

    /// <summary>
    /// Allocate and fill the physical module grid with all fixed structural
    /// elements: the finder L-bar, timing clock border, and alignment borders.
    ///
    /// <para>
    /// The "finder + clock" border (outermost ring):
    /// <list type="bullet">
    ///   <item><term>Top row (row 0)</term>
    ///     <description>Alternating dark/light starting dark at col 0 — timing clock.</description></item>
    ///   <item><term>Right column (col C-1)</term>
    ///     <description>Alternating dark/light starting dark at row 0 — timing clock.</description></item>
    ///   <item><term>Left column (col 0)</term>
    ///     <description>All dark — vertical leg of the L-finder.</description></item>
    ///   <item><term>Bottom row (row R-1)</term>
    ///     <description>All dark — horizontal leg of the L-finder.</description></item>
    /// </list>
    /// </para>
    ///
    /// <para>
    /// The L-shaped solid-dark bar (left+bottom) tells a scanner where the
    /// symbol starts and its orientation. The asymmetry between the L-bar
    /// and the alternating timing distinguishes all four 90° rotations.
    /// </para>
    ///
    /// <para>
    /// Writing order: alignment borders first (so outer borders can override
    /// at intersections), then timing rows/columns, then left column, then
    /// bottom row (highest precedence — written last).
    /// </para>
    /// </summary>
    internal static bool[][] InitGrid(SymbolEntry entry)
    {
        var R = entry.SymbolRows;
        var C = entry.SymbolCols;

        // Allocate all-light (false) grid.
        var grid = new bool[R][];
        for (var r = 0; r < R; r++) grid[r] = new bool[C];

        // ── Alignment borders (multi-region symbols only) ──────────────────────
        // Written FIRST so the outer borders overwrite intersections.
        // Horizontal alignment borders (between row-groups of regions):
        for (var rr = 0; rr < entry.RegionRows - 1; rr++)
        {
            // First AB row after data region rr:
            //   1 (outer border) + (rr+1)*regionHeight + rr*2 (previous ABs)
            var abRow0 = 1 + (rr + 1) * entry.RegionHeight + rr * 2;
            var abRow1 = abRow0 + 1;
            for (var c = 0; c < C; c++)
            {
                grid[abRow0][c] = true;         // solid dark bar
                grid[abRow1][c] = c % 2 == 0;  // alternating, starts dark
            }
        }
        // Vertical alignment borders (between column-groups of regions):
        for (var rc = 0; rc < entry.RegionCols - 1; rc++)
        {
            var abCol0 = 1 + (rc + 1) * entry.RegionWidth + rc * 2;
            var abCol1 = abCol0 + 1;
            for (var r = 0; r < R; r++)
            {
                grid[r][abCol0] = true;         // solid dark bar
                grid[r][abCol1] = r % 2 == 0;  // alternating, starts dark
            }
        }

        // ── Top row (row 0): timing clock — alternating dark/light, starts dark ─
        for (var c = 0; c < C; c++) grid[0][c] = c % 2 == 0;

        // ── Right column (col C-1): timing clock — alternating, starts dark ─────
        for (var r = 0; r < R; r++) grid[r][C - 1] = r % 2 == 0;

        // ── Left column (col 0): L-finder left leg — all dark ───────────────────
        // Overrides timing at (0, 0) making it solid dark (correct behavior).
        for (var r = 0; r < R; r++) grid[r][0] = true;

        // ── Bottom row (row R-1): L-finder bottom leg — all dark ─────────────────
        // Written LAST: overrides alignment borders, right-column timing, everything.
        // The bottom-right corner (R-1, C-1) becomes dark.
        for (var c = 0; c < C; c++) grid[R - 1][c] = true;

        return grid;
    }

    // =========================================================================
    // Utah placement algorithm
    // =========================================================================
    //
    // The Utah placement algorithm walks the logical grid (all data region
    // interiors concatenated) in a diagonal zigzag. For each codeword, 8 bits
    // are placed at 8 fixed offsets relative to the current reference position.
    // After each codeword the reference moves diagonally.
    //
    // It is called "Utah" because the 8-module codeword shape vaguely resembles
    // the US state of Utah — a rectangle with a notch cut from the top-left corner.
    //
    // There is NO masking step. The diagonal traversal naturally distributes bits
    // across the symbol without the degenerate clustering that would otherwise
    // require masking (as in QR Code).

    /// <summary>
    /// Apply boundary wrap rules from ISO/IEC 16022:2006 Annex F.
    ///
    /// <para>
    /// When the standard Utah shape extends beyond the logical grid edge,
    /// these rules fold the coordinates back into the valid range.
    /// </para>
    ///
    /// <para>
    /// Four wrap rules (applied in priority order):
    /// <list type="number">
    ///   <item>row &lt; 0 AND col == 0      → (1, 3)  — top-left corner singularity</item>
    ///   <item>row &lt; 0 AND col == nCols  → (0, col-2)  — past right edge at top</item>
    ///   <item>row &lt; 0                   → (row+nRows, col-4)  — wrap top→bottom</item>
    ///   <item>col &lt; 0                   → (row-4, col+nCols)  — wrap left→right</item>
    /// </list>
    /// </para>
    /// </summary>
    private static (int Row, int Col) ApplyWrap(int row, int col, int nRows, int nCols)
    {
        // Case 1: top-left corner singularity
        if (row < 0 && col == 0) return (1, 3);
        // Case 2: wrapped past the right edge at the top
        if (row < 0 && col == nCols) return (0, col - 2);
        // Case 3: wrap row off top → bottom of grid, shift left 4
        if (row < 0) return (row + nRows, col - 4);
        // Case 4: wrap col off left → right of grid, shift up 4
        if (col < 0) return (row - 4, col + nCols);
        return (row, col);
    }

    /// <summary>
    /// Place one codeword using the standard "Utah" 8-module pattern.
    ///
    /// <para>
    /// The Utah shape at reference position (row, col):
    /// <code>
    ///     col:  c-2  c-1   c
    ///   row-2:   .   [1]  [2]
    ///   row-1:  [3]  [4]  [5]
    ///   row  :  [6]  [7]  [8]
    /// </code>
    /// Numbers [1]–[8] correspond to bits 1–8 (1=LSB, 8=MSB) of the codeword.
    /// MSB (bit 8) is placed at (row, col); LSB (bit 1) at (row-2, col-1).
    /// </para>
    ///
    /// <para>
    /// Why this shape? The notch at the top-left (missing module at row-2, col-2)
    /// is what makes it look like Utah. This asymmetry enables the algorithm to
    /// pack codewords tightly in a diagonal pattern without overlap.
    /// </para>
    /// </summary>
    private static void PlaceUtah(byte cw, int row, int col, int nRows, int nCols,
        bool[][] grid, bool[][] used)
    {
        // Placement table: [row offset, col offset, bit index (7=MSB, 0=LSB)]
        ReadOnlySpan<(int Dr, int Dc, int Bit)> placements =
        [
            (0,   0,  7), // bit 8 (MSB) at (row, col)
            (0,  -1,  6), // bit 7
            (0,  -2,  5), // bit 6
            (-1,  0,  4), // bit 5
            (-1, -1,  3), // bit 4
            (-1, -2,  2), // bit 3
            (-2,  0,  1), // bit 2
            (-2, -1,  0), // bit 1 (LSB)
        ];

        foreach (var (dr, dc, bit) in placements)
        {
            var (r, c) = ApplyWrap(row + dr, col + dc, nRows, nCols);
            if (r >= 0 && r < nRows && c >= 0 && c < nCols && !used[r][c])
            {
                grid[r][c] = ((cw >> bit) & 1) == 1;
                used[r][c] = true;
            }
        }
    }

    /// <summary>
    /// Corner pattern 1 — triggered at the top-left boundary.
    ///
    /// <para>
    /// When the diagonal walk reaches the top-left corner the normal Utah shape
    /// would extend off the grid. This corner pattern uses absolute positions
    /// instead of relative offsets to handle the boundary correctly.
    /// </para>
    /// </summary>
    private static void PlaceCorner1(byte cw, int nRows, int nCols,
        bool[][] grid, bool[][] used)
    {
        ReadOnlySpan<(int R, int C, int Bit)> pos =
        [
            (0,        nCols - 2, 7), // bit 8 (MSB)
            (0,        nCols - 1, 6), // bit 7
            (1,        0,         5), // bit 6
            (2,        0,         4), // bit 5
            (nRows - 2, 0,        3), // bit 4
            (nRows - 1, 0,        2), // bit 3
            (nRows - 1, 1,        1), // bit 2
            (nRows - 1, 2,        0), // bit 1 (LSB)
        ];
        foreach (var (r, c, bit) in pos)
        {
            if (r >= 0 && r < nRows && c >= 0 && c < nCols && !used[r][c])
            {
                grid[r][c] = ((cw >> bit) & 1) == 1;
                used[r][c] = true;
            }
        }
    }

    /// <summary>Corner pattern 2 — triggered at the top-right boundary.</summary>
    private static void PlaceCorner2(byte cw, int nRows, int nCols,
        bool[][] grid, bool[][] used)
    {
        ReadOnlySpan<(int R, int C, int Bit)> pos =
        [
            (0,        nCols - 2, 7),
            (0,        nCols - 1, 6),
            (1,        nCols - 1, 5),
            (2,        nCols - 1, 4),
            (nRows - 1, 0,        3),
            (nRows - 1, 1,        2),
            (nRows - 1, 2,        1),
            (nRows - 1, 3,        0),
        ];
        foreach (var (r, c, bit) in pos)
        {
            if (r >= 0 && r < nRows && c >= 0 && c < nCols && !used[r][c])
            {
                grid[r][c] = ((cw >> bit) & 1) == 1;
                used[r][c] = true;
            }
        }
    }

    /// <summary>Corner pattern 3 — triggered at the bottom-left boundary.</summary>
    private static void PlaceCorner3(byte cw, int nRows, int nCols,
        bool[][] grid, bool[][] used)
    {
        ReadOnlySpan<(int R, int C, int Bit)> pos =
        [
            (0,        nCols - 1, 7),
            (1,        0,         6),
            (2,        0,         5),
            (nRows - 2, 0,        4),
            (nRows - 1, 0,        3),
            (nRows - 1, 1,        2),
            (nRows - 1, 2,        1),
            (nRows - 1, 3,        0),
        ];
        foreach (var (r, c, bit) in pos)
        {
            if (r >= 0 && r < nRows && c >= 0 && c < nCols && !used[r][c])
            {
                grid[r][c] = ((cw >> bit) & 1) == 1;
                used[r][c] = true;
            }
        }
    }

    /// <summary>
    /// Corner pattern 4 — used only in matrices where both nRows and nCols
    /// are odd (rectangular symbols and some extended square sizes).
    /// </summary>
    private static void PlaceCorner4(byte cw, int nRows, int nCols,
        bool[][] grid, bool[][] used)
    {
        ReadOnlySpan<(int R, int C, int Bit)> pos =
        [
            (nRows - 3, nCols - 1, 7),
            (nRows - 2, nCols - 1, 6),
            (nRows - 1, nCols - 3, 5),
            (nRows - 1, nCols - 2, 4),
            (nRows - 1, nCols - 1, 3),
            (0,         0,         2),
            (1,         0,         1),
            (2,         0,         0),
        ];
        foreach (var (r, c, bit) in pos)
        {
            if (r >= 0 && r < nRows && c >= 0 && c < nCols && !used[r][c])
            {
                grid[r][c] = ((cw >> bit) & 1) == 1;
                used[r][c] = true;
            }
        }
    }

    /// <summary>
    /// Run the Utah diagonal placement algorithm on the logical data matrix
    /// (nRows × nCols), filling in all codeword bits.
    ///
    /// <para>
    /// The reference position (row, col) starts at (4, 0) and zigzags diagonally
    /// across the logical grid. Each iteration has two legs:
    /// <list type="number">
    ///   <item>
    ///     <term>Upward-right leg</term>
    ///     <description>Place at (row, col), step row-=2, col+=2, until out of bounds, then row+=1, col+=3.</description>
    ///   </item>
    ///   <item>
    ///     <term>Downward-left leg</term>
    ///     <description>Place at (row, col), step row+=2, col-=2, until out of bounds, then row+=3, col+=1.</description>
    ///   </item>
    /// </list>
    /// </para>
    ///
    /// <para>
    /// Between legs, corner patterns fire when the reference position matches
    /// specific trigger conditions (see the four PlaceCornerN methods).
    /// </para>
    ///
    /// <para>
    /// Termination: when row≥nRows AND col≥nCols, all modules have been visited.
    /// Any unvisited modules receive the ISO fill rule: dark if (r+c) mod 2 == 1.
    /// </para>
    /// </summary>
    internal static bool[][] UtahPlacement(byte[] codewords, int nRows, int nCols)
    {
        var grid = new bool[nRows][];
        var used = new bool[nRows][];
        for (var r = 0; r < nRows; r++)
        {
            grid[r] = new bool[nCols];
            used[r] = new bool[nCols];
        }

        var cwIdx = 0;
        var row = 4;
        var col = 0;

        // Local helper: dispatch one codeword (if any remain) to a corner function.
        void Place(Action<byte, int, int, bool[][], bool[][]> fn)
        {
            if (cwIdx < codewords.Length)
            {
                fn(codewords[cwIdx++], nRows, nCols, grid, used);
            }
        }

        while (true)
        {
            // ── Corner special cases (trigger before each diagonal leg) ──────────
            // Corner 1: reference at (nRows, 0) when nRows or nCols divisible by 4.
            if (row == nRows && col == 0 && (nRows % 4 == 0 || nCols % 4 == 0))
                Place(PlaceCorner1);

            // Corner 2: reference at (nRows-2, 0) when nCols mod 4 ≠ 0.
            if (row == nRows - 2 && col == 0 && nCols % 4 != 0)
                Place(PlaceCorner2);

            // Corner 3: reference at (nRows-2, 0) when nCols mod 8 == 4.
            if (row == nRows - 2 && col == 0 && nCols % 8 == 4)
                Place(PlaceCorner3);

            // Corner 4: reference at (nRows+4, 2) when nCols mod 8 == 0.
            if (row == nRows + 4 && col == 2 && nCols % 8 == 0)
                Place(PlaceCorner4);

            // ── Upward-right diagonal leg (row -= 2, col += 2) ──────────────────
            while (true)
            {
                if (row >= 0 && row < nRows && col >= 0 && col < nCols && !used[row][col])
                {
                    if (cwIdx < codewords.Length)
                    {
                        PlaceUtah(codewords[cwIdx++], row, col, nRows, nCols, grid, used);
                    }
                }
                row -= 2;
                col += 2;
                if (row < 0 || col >= nCols) break;
            }

            // Step to next diagonal start.
            row++;
            col += 3;

            // ── Downward-left diagonal leg (row += 2, col -= 2) ─────────────────
            while (true)
            {
                if (row >= 0 && row < nRows && col >= 0 && col < nCols && !used[row][col])
                {
                    if (cwIdx < codewords.Length)
                    {
                        PlaceUtah(codewords[cwIdx++], row, col, nRows, nCols, grid, used);
                    }
                }
                row += 2;
                col -= 2;
                if (row >= nRows || col < 0) break;
            }

            // Step to next diagonal start.
            row += 3;
            col++;

            // ── Termination check ────────────────────────────────────────────────
            if (row >= nRows && col >= nCols) break;
            if (cwIdx >= codewords.Length) break;
        }

        // ── Fill remaining unset modules (ISO right-and-bottom fill rule) ────────
        // Some symbol sizes have residual modules the diagonal walk does not reach.
        // ISO/IEC 16022 §10: these receive dark modules if (r+c) mod 2 == 1.
        for (var r = 0; r < nRows; r++)
        {
            for (var c = 0; c < nCols; c++)
            {
                if (!used[r][c])
                {
                    grid[r][c] = (r + c) % 2 == 1;
                }
            }
        }

        return grid;
    }

    // =========================================================================
    // Logical → Physical coordinate mapping
    // =========================================================================

    /// <summary>
    /// Map a logical data matrix coordinate (r, c) to its physical symbol
    /// coordinate (physRow, physCol).
    ///
    /// <para>
    /// The logical data matrix is the concatenation of all data region interiors
    /// treated as one flat grid. The Utah algorithm works in logical space.
    /// After placement we map back to physical coords, adding:
    /// <list type="bullet">
    ///   <item>1-module outer border (finder + timing) on all four sides</item>
    ///   <item>2-module alignment borders between data regions</item>
    /// </list>
    /// </para>
    ///
    /// <para>
    /// Formula:
    /// <code>
    /// physRow = floor(r / rh) × (rh + 2) + (r mod rh) + 1
    /// physCol = floor(c / rw) × (rw + 2) + (c mod rw) + 1
    /// </code>
    /// The "+2" term accounts for the 2-module alignment border between regions.
    /// The "+1" term accounts for the 1-module outer border (finder + timing).
    /// For single-region symbols this simplifies to physRow = r+1, physCol = c+1.
    /// </para>
    /// </summary>
    private static (int Row, int Col) LogicalToPhysical(int r, int c, SymbolEntry entry)
    {
        var rh = entry.RegionHeight;
        var rw = entry.RegionWidth;
        var physRow = (r / rh) * (rh + 2) + (r % rh) + 1;
        var physCol = (c / rw) * (rw + 2) + (c % rw) + 1;
        return (physRow, physCol);
    }

    // =========================================================================
    // Internal test helpers (not part of the public API)
    // =========================================================================

    /// <summary>
    /// Expose the generator polynomial for a given ECC length (for testing).
    /// </summary>
    internal static byte[] GetGeneratorForTest(int nEcc) => GetGenerator(nEcc);

    /// <summary>
    /// Expose symbol selection for testing (returns the internal <see cref="SymbolEntry"/>).
    /// </summary>
    internal static SymbolEntry SelectSymbolForTest(int codewordCount, DataMatrixSymbolShape shape)
        => SelectSymbol(codewordCount, shape);

    // =========================================================================
    // ModuleGrid construction
    // =========================================================================

    /// <summary>
    /// Convert a jagged bool array to an immutable <see cref="ModuleGrid"/>.
    /// </summary>
    private static ModuleGrid ToModuleGrid(bool[][] physGrid, int rows, int cols)
    {
        var grid = ModuleGrid.Create(rows, cols, ModuleShape.Square);
        for (var r = 0; r < rows; r++)
        {
            for (var c = 0; c < cols; c++)
            {
                if (physGrid[r][c])
                {
                    grid = grid.SetModule(r, c, true);
                }
            }
        }
        return grid;
    }
}
