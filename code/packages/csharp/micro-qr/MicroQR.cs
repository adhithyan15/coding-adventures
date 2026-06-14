using System.Text;
using CodingAdventures.Barcode2D;
using CodingAdventures.Gf256;

namespace CodingAdventures.MicroQR;

// MicroQR.cs — ISO/IEC 18004:2015 Annex E Micro QR Code encoder
// ==============================================================
//
// Micro QR Code is the compact variant of standard QR Code. Where a regular
// QR Code starts at 21×21 modules (version 1), Micro QR fits inside an 11×11
// to 17×17 square — small enough to label a surface-mount component.
//
// The key structural difference is the single finder pattern. Regular QR uses
// three identical 7×7 "eyes" at three corners so a scanner can detect
// orientation from any angle. Micro QR places just one finder in the top-left,
// which is always unambiguous because the data area is to the bottom-right.
// That single omission is responsible for most of the space saving.
//
// Symbol sizes:
//
//   M1: 11×11   M2: 13×13   M3: 15×15   M4: 17×17
//   formula: size = 2 × version_number + 9
//
// Encoding pipeline:
//
//   input string
//     → auto-select smallest symbol (M1..M4) and mode (numeric/alphanumeric/byte)
//     → build bit stream (mode indicator + char count + data + terminator + padding)
//     → Reed-Solomon ECC (GF(256)/0x11D, b=0, single block)
//     → initialize grid (finder, separator, timing at row0/col0, format reserved)
//     → zigzag data placement (two-column snake from bottom-right)
//     → evaluate 4 mask patterns, pick lowest penalty
//     → write format information (15 bits, single copy, XOR 0x4445)
//     → return ModuleGrid

// ─────────────────────────────────────────────────────────────────────────────
// Public error types
// ─────────────────────────────────────────────────────────────────────────────

/// <summary>Base class for all Micro QR encoding errors.</summary>
public class MicroQRException : Exception
{
    /// <summary>Create a Micro QR exception with a message.</summary>
    public MicroQRException(string message) : base(message) { }
}

/// <summary>Input is too long for any M1–M4 symbol at any ECC level.</summary>
public sealed class InputTooLongException : MicroQRException
{
    /// <summary>Create an input-too-long error.</summary>
    public InputTooLongException(string message) : base(message) { }
}

/// <summary>The requested encoding mode is not available for the chosen symbol.</summary>
public sealed class UnsupportedModeException : MicroQRException
{
    /// <summary>Create an unsupported-mode error.</summary>
    public UnsupportedModeException(string message) : base(message) { }
}

/// <summary>A character cannot be encoded in the selected mode.</summary>
public sealed class InvalidCharacterException : MicroQRException
{
    /// <summary>Create an invalid-character error.</summary>
    public InvalidCharacterException(string message) : base(message) { }
}

/// <summary>The requested ECC level is not available for the chosen symbol.</summary>
public sealed class EccNotAvailableException : MicroQRException
{
    /// <summary>Create an ECC-not-available error.</summary>
    public EccNotAvailableException(string message) : base(message) { }
}

// ─────────────────────────────────────────────────────────────────────────────
// Public enumerations
// ─────────────────────────────────────────────────────────────────────────────

/// <summary>
/// Micro QR symbol designator (M1 through M4).
///
/// Each step up adds two rows/columns (size = 2×version+9), increasing
/// data capacity. M1 is the smallest (11×11) and supports only numeric mode
/// with no real error correction. M4 (17×17) supports all four modes and
/// reaches 35 numeric digits at ECC-L.
/// </summary>
public enum MicroQRVersion
{
    /// <summary>11×11, numeric only, detection only.</summary>
    M1,
    /// <summary>13×13, numeric + alphanumeric + byte, L and M ECC.</summary>
    M2,
    /// <summary>15×15, numeric + alphanumeric + byte, L and M ECC.</summary>
    M3,
    /// <summary>17×17, all modes, L, M, and Q ECC.</summary>
    M4,
}

/// <summary>
/// Error correction level.
///
/// Unlike regular QR which has L/M/Q/H, Micro QR has a reduced set:
///
/// <list type="table">
///   <listheader><term>Level</term><description>Available in / Recovery</description></listheader>
///   <item><term>Detection</term><description>M1 only — detects errors only</description></item>
///   <item><term>L</term><description>M2–M4 — ~7% of codewords</description></item>
///   <item><term>M</term><description>M2–M4 — ~15% of codewords</description></item>
///   <item><term>Q</term><description>M4 only — ~25% of codewords</description></item>
/// </list>
///
/// Level H (30%) is not available in any Micro QR symbol.
/// </summary>
public enum MicroQREccLevel
{
    /// <summary>Error detection only. M1 exclusively.</summary>
    Detection,
    /// <summary>~7% recovery. Available in M2, M3, M4.</summary>
    L,
    /// <summary>~15% recovery. Available in M2, M3, M4.</summary>
    M,
    /// <summary>~25% recovery. Available in M4 only.</summary>
    Q,
}

// ─────────────────────────────────────────────────────────────────────────────
// Symbol configuration table
// ─────────────────────────────────────────────────────────────────────────────

/// <summary>
/// All per-symbol constants from ISO 18004:2015 Annex E.
///
/// There are exactly 8 valid (version, ECC) combinations in Micro QR.
/// The symbolIndicator value (0–7) is what gets encoded in the 15-bit
/// format information word.
/// </summary>
internal sealed record SymbolConfig(
    MicroQRVersion Version,
    MicroQREccLevel Ecc,
    int SymbolIndicator,    // 0–7, used in format information
    int Size,               // symbol side length in modules
    int DataCW,             // data codewords (full bytes)
    int EccCW,              // ECC codewords
    int NumericCap,         // max numeric characters
    int AlphaCap,           // max alphanumeric chars (−1 = not supported)
    int ByteCap,            // max byte chars (−1 = not supported)
    int KanjiCap,           // max kanji chars (−1 = not supported)
    int TerminatorBits,     // 3/5/7/9 zero bits appended after data
    int ModeIndicatorBits,  // 0/1/2/3 bits for the mode indicator field
    int CharCountBitsNumeric,
    int CharCountBitsAlpha,
    int CharCountBitsByte,
    int CharCountBitsKanji,
    bool M1HalfCW           // true for M1: last data "codeword" is 4 bits
);

/// <summary>
/// Encoding mode — the three modes supported in this encoder.
///
/// Kanji mode (M4 only) is a future extension; it requires a Shift-JIS
/// encoding table that is out of scope for this version.
/// </summary>
internal enum EncodingMode { Numeric, Alphanumeric, Byte }

// ─────────────────────────────────────────────────────────────────────────────
// Static tables (compile-time constants)
// ─────────────────────────────────────────────────────────────────────────────

internal static class Tables
{
    // ── Symbol configurations ────────────────────────────────────────────────
    //
    // All 8 valid (version, ECC) combinations from ISO 18004:2015 Annex E.
    // Order matches symbolIndicator values 0–7 exactly.

    internal static readonly SymbolConfig[] SymbolConfigs =
    [
        // M1 / Detection
        new(MicroQRVersion.M1, MicroQREccLevel.Detection, 0, 11,
            DataCW: 3, EccCW: 2,
            NumericCap: 5, AlphaCap: -1, ByteCap: -1, KanjiCap: -1,
            TerminatorBits: 3, ModeIndicatorBits: 0,
            CharCountBitsNumeric: 3, CharCountBitsAlpha: 0,
            CharCountBitsByte: 0, CharCountBitsKanji: 0,
            M1HalfCW: true),

        // M2 / L
        new(MicroQRVersion.M2, MicroQREccLevel.L, 1, 13,
            DataCW: 5, EccCW: 5,
            NumericCap: 10, AlphaCap: 6, ByteCap: 4, KanjiCap: -1,
            TerminatorBits: 5, ModeIndicatorBits: 1,
            CharCountBitsNumeric: 4, CharCountBitsAlpha: 3,
            CharCountBitsByte: 4, CharCountBitsKanji: 0,
            M1HalfCW: false),

        // M2 / M
        new(MicroQRVersion.M2, MicroQREccLevel.M, 2, 13,
            DataCW: 4, EccCW: 6,
            NumericCap: 8, AlphaCap: 5, ByteCap: 3, KanjiCap: -1,
            TerminatorBits: 5, ModeIndicatorBits: 1,
            CharCountBitsNumeric: 4, CharCountBitsAlpha: 3,
            CharCountBitsByte: 4, CharCountBitsKanji: 0,
            M1HalfCW: false),

        // M3 / L
        new(MicroQRVersion.M3, MicroQREccLevel.L, 3, 15,
            DataCW: 11, EccCW: 6,
            NumericCap: 23, AlphaCap: 14, ByteCap: 9, KanjiCap: -1,
            TerminatorBits: 7, ModeIndicatorBits: 2,
            CharCountBitsNumeric: 5, CharCountBitsAlpha: 4,
            CharCountBitsByte: 4, CharCountBitsKanji: 0,
            M1HalfCW: false),

        // M3 / M
        new(MicroQRVersion.M3, MicroQREccLevel.M, 4, 15,
            DataCW: 9, EccCW: 8,
            NumericCap: 18, AlphaCap: 11, ByteCap: 7, KanjiCap: -1,
            TerminatorBits: 7, ModeIndicatorBits: 2,
            CharCountBitsNumeric: 5, CharCountBitsAlpha: 4,
            CharCountBitsByte: 4, CharCountBitsKanji: 0,
            M1HalfCW: false),

        // M4 / L
        new(MicroQRVersion.M4, MicroQREccLevel.L, 5, 17,
            DataCW: 16, EccCW: 8,
            NumericCap: 35, AlphaCap: 21, ByteCap: 15, KanjiCap: 9,
            TerminatorBits: 9, ModeIndicatorBits: 3,
            CharCountBitsNumeric: 6, CharCountBitsAlpha: 5,
            CharCountBitsByte: 5, CharCountBitsKanji: 4,
            M1HalfCW: false),

        // M4 / M
        new(MicroQRVersion.M4, MicroQREccLevel.M, 6, 17,
            DataCW: 14, EccCW: 10,
            NumericCap: 30, AlphaCap: 18, ByteCap: 13, KanjiCap: 8,
            TerminatorBits: 9, ModeIndicatorBits: 3,
            CharCountBitsNumeric: 6, CharCountBitsAlpha: 5,
            CharCountBitsByte: 5, CharCountBitsKanji: 4,
            M1HalfCW: false),

        // M4 / Q
        new(MicroQRVersion.M4, MicroQREccLevel.Q, 7, 17,
            DataCW: 10, EccCW: 14,
            NumericCap: 21, AlphaCap: 13, ByteCap: 9, KanjiCap: 6,
            TerminatorBits: 9, ModeIndicatorBits: 3,
            CharCountBitsNumeric: 6, CharCountBitsAlpha: 5,
            CharCountBitsByte: 5, CharCountBitsKanji: 4,
            M1HalfCW: false),
    ];

    // ── RS generator polynomials ─────────────────────────────────────────────
    //
    // Monic RS generator polynomials over GF(256)/0x11D, b=0 convention:
    //   g(x) = (x+α⁰)(x+α¹)···(x+α^{n−1})
    //
    // Coefficients listed highest-degree first; leading monic term (1) included.
    // We only need counts {2, 5, 6, 8, 10, 14} for Micro QR.

    internal static readonly IReadOnlyDictionary<int, byte[]> RsGenerators =
        new Dictionary<int, byte[]>
        {
            // 2 ECC codewords (M1 detection)
            // g(x) = (x+1)(x+α) = x² + 3x + 2
            [2]  = [0x01, 0x03, 0x02],

            // 5 ECC codewords (M2-L)
            [5]  = [0x01, 0x1f, 0xf6, 0x44, 0xd9, 0x68],

            // 6 ECC codewords (M2-M, M3-L)
            [6]  = [0x01, 0x3f, 0x4e, 0x17, 0x9b, 0x05, 0x37],

            // 8 ECC codewords (M3-M, M4-L)
            [8]  = [0x01, 0x63, 0x0d, 0x60, 0x6d, 0x5b, 0x10, 0xa2, 0xa3],

            // 10 ECC codewords (M4-M)
            [10] = [0x01, 0xf6, 0x75, 0xa8, 0xd0, 0xc3, 0xe3, 0x36, 0xe1, 0x3c, 0x45],

            // 14 ECC codewords (M4-Q)
            [14] = [0x01, 0xf6, 0x9a, 0x60, 0x97, 0x8a, 0xf1, 0xa4, 0xa1, 0x8e, 0xfc, 0x7a, 0x52, 0xad, 0xac],
        };

    // ── Pre-computed format information table ────────────────────────────────
    //
    // All 32 format information words (after XOR with 0x4445).
    // Indexed as FormatTable[symbolIndicator][maskPattern].
    //
    // The 15-bit format word encodes:
    //   [symbol_indicator (3 bits)] [mask_pattern (2 bits)] [BCH-10 remainder]
    // then XOR-masked with 0x4445 (not 0x5412 like regular QR).
    //
    // | Symbol+ECC | Mask 0 | Mask 1 | Mask 2 | Mask 3 |
    // |-----------|--------|--------|--------|--------|
    // | M1 (000)  | 0x4445 | 0x4172 | 0x4E2B | 0x4B1C |
    // | M2-L(001) | 0x5528 | 0x501F | 0x5F46 | 0x5A71 |
    // | M2-M(010) | 0x6649 | 0x637E | 0x6C27 | 0x6910 |
    // | M3-L(011) | 0x7764 | 0x7253 | 0x7D0A | 0x783D |
    // | M3-M(100) | 0x06DE | 0x03E9 | 0x0CB0 | 0x0987 |
    // | M4-L(101) | 0x17F3 | 0x12C4 | 0x1D9D | 0x18AA |
    // | M4-M(110) | 0x24B2 | 0x2185 | 0x2EDC | 0x2BEB |
    // | M4-Q(111) | 0x359F | 0x30A8 | 0x3FF1 | 0x3AC6 |

    internal static readonly int[][] FormatTable =
    [
        [0x4445, 0x4172, 0x4E2B, 0x4B1C],  // M1
        [0x5528, 0x501F, 0x5F46, 0x5A71],  // M2-L
        [0x6649, 0x637E, 0x6C27, 0x6910],  // M2-M
        [0x7764, 0x7253, 0x7D0A, 0x783D],  // M3-L
        [0x06DE, 0x03E9, 0x0CB0, 0x0987],  // M3-M
        [0x17F3, 0x12C4, 0x1D9D, 0x18AA],  // M4-L
        [0x24B2, 0x2185, 0x2EDC, 0x2BEB],  // M4-M
        [0x359F, 0x30A8, 0x3FF1, 0x3AC6],  // M4-Q
    ];

    // ── Alphanumeric character set ───────────────────────────────────────────
    //
    // The 45-character set used by QR and Micro QR alphanumeric mode.
    // Pairs of characters pack into 11 bits: (first×45 + second).
    // A trailing single character uses 6 bits.

    internal const string AlphanumChars = "0123456789ABCDEFGHIJKLMNOPQRSTUVWXYZ $%*+-./:";

    // ── Mask conditions ──────────────────────────────────────────────────────
    //
    // The 4 mask conditions for Micro QR (patterns 0–3).
    // These are the same as the first four patterns in regular QR's set of 8.
    // If the condition is true for a data/ECC module, that module is flipped.
    //
    // | Pattern | Condition |
    // |---------|-----------|
    // | 0       | (row + col) mod 2 == 0 |
    // | 1       | row mod 2 == 0 |
    // | 2       | col mod 3 == 0 |
    // | 3       | (row + col) mod 3 == 0 |

    internal static readonly Func<int, int, bool>[] MaskConditions =
    [
        (r, c) => (r + c) % 2 == 0,
        (r, _) => r % 2 == 0,
        (_, c) => c % 3 == 0,
        (r, c) => (r + c) % 3 == 0,
    ];
}

// ─────────────────────────────────────────────────────────────────────────────
// Bit-writer utility
// ─────────────────────────────────────────────────────────────────────────────

/// <summary>
/// Accumulates individual bits and flushes them as a byte array.
///
/// The bit stream for Micro QR is MSB-first within each codeword:
/// the first bit written ends up as the most-significant bit of byte 0.
/// This mirrors a serial bus where the most significant bit is transmitted first.
/// </summary>
internal sealed class BitWriter
{
    private readonly List<int> _bits = [];

    /// <summary>Total number of bits written so far.</summary>
    public int BitLength => _bits.Count;

    /// <summary>Append <paramref name="count"/> bits from <paramref name="value"/>, MSB first.</summary>
    public void Write(int value, int count)
    {
        for (var i = count - 1; i >= 0; i--)
            _bits.Add((value >> i) & 1);
    }

    /// <summary>Return all accumulated bits as a list (a copy).</summary>
    public List<int> ToBits() => new(_bits);

    /// <summary>
    /// Return all accumulated bits as a packed byte array.
    /// If the bit count is not a multiple of 8, the last byte is
    /// zero-padded on the right (least-significant bits are 0).
    /// </summary>
    public List<byte> ToBytes()
    {
        var bytes = new List<byte>();
        for (var i = 0; i < _bits.Count; i += 8)
        {
            var b = 0;
            for (var j = 0; j < 8; j++)
                b = (b << 1) | (i + j < _bits.Count ? _bits[i + j] : 0);
            bytes.Add((byte)b);
        }
        return bytes;
    }
}

// ─────────────────────────────────────────────────────────────────────────────
// Working grid
// ─────────────────────────────────────────────────────────────────────────────

/// <summary>
/// Internal mutable grid used while building the symbol.
///
/// Tracks both module values (dark/light) and which positions are "reserved"
/// (structural) so the data placement and masking steps know which modules
/// to skip.
/// </summary>
internal sealed class WorkGrid
{
    internal readonly int Size;
    internal readonly bool[,] Modules;   // true = dark
    internal readonly bool[,] Reserved;  // true = structural

    internal WorkGrid(int size)
    {
        Size = size;
        Modules  = new bool[size, size];
        Reserved = new bool[size, size];
    }

    internal void Set(int row, int col, bool dark, bool reserve = false)
    {
        Modules[row, col] = dark;
        if (reserve) Reserved[row, col] = true;
    }
}

// ─────────────────────────────────────────────────────────────────────────────
// Main encoder
// ─────────────────────────────────────────────────────────────────────────────

/// <summary>
/// Micro QR Code encoder — ISO/IEC 18004:2015 Annex E.
///
/// <para>
/// The main entry point is <see cref="Encode(string, MicroQRVersion?, MicroQREccLevel?)"/>,
/// which selects the smallest symbol that fits the input and returns a
/// <see cref="ModuleGrid"/> ready for layout and rendering.
/// </para>
///
/// <para>
/// Auto-selection examples:
/// <list type="bullet">
///   <item>"1" → M1 (11×11, detection only)</item>
///   <item>"12345" → M1 (exactly fills 5-digit numeric capacity)</item>
///   <item>"HELLO" → M2-L (5 alphanumeric chars)</item>
///   <item>"hello" → M3-L (5 bytes, lowercase not in alphanumeric set)</item>
///   <item>"https://a.b" → M4-L (11 bytes)</item>
/// </list>
/// </para>
/// </summary>
public static class MicroQR
{
    /// <summary>Package version.</summary>
    public const string Version = "0.1.0";

    // ── Public API ────────────────────────────────────────────────────────────

    /// <summary>
    /// Encode an input string to a Micro QR Code module grid.
    ///
    /// <para>
    /// When <paramref name="version"/> and <paramref name="ecc"/> are null the
    /// encoder auto-selects the smallest symbol + ECC combination that fits
    /// the input.
    /// </para>
    /// </summary>
    /// <param name="input">The string to encode. Must not exceed M4 capacity.</param>
    /// <param name="version">Force a specific symbol version, or null for auto.</param>
    /// <param name="ecc">Force a specific ECC level, or null for auto.</param>
    /// <returns>A <see cref="ModuleGrid"/> containing the final symbol modules.</returns>
    /// <exception cref="InputTooLongException">Input exceeds all Micro QR capacity.</exception>
    /// <exception cref="UnsupportedModeException">Input requires a mode the chosen symbol does not support.</exception>
    /// <exception cref="InvalidCharacterException">Input contains a character not encodable in the selected mode.</exception>
    /// <exception cref="EccNotAvailableException">No configuration matches the requested version + ECC combination.</exception>
    public static ModuleGrid Encode(
        string input,
        MicroQRVersion? version = null,
        MicroQREccLevel? ecc = null)
    {
        // 1. Choose the smallest fitting symbol configuration
        var cfg = SelectConfig(input, version, ecc);

        // 2. Build data codewords (mode indicator + char count + data + pad)
        var dataCW = BuildDataCodewords(input, cfg);

        // 3. Compute Reed-Solomon ECC
        var eccCW = RsEncode(dataCW, cfg.EccCW);

        // 4. Flatten codeword stream to a bit sequence
        //    For M1: last data codeword contributes only 4 bits (the upper nibble).
        var finalCW = new List<byte>(dataCW);
        finalCW.AddRange(eccCW);

        var bits = new List<bool>();
        for (var cwIdx = 0; cwIdx < finalCW.Count; cwIdx++)
        {
            var cw = finalCW[cwIdx];
            // M1's last data codeword (index dataCW-1) contributes 4 bits only.
            var bitsInCW = cfg.M1HalfCW && cwIdx == cfg.DataCW - 1 ? 4 : 8;
            for (var b = bitsInCW - 1; b >= 0; b--)
                bits.Add(((cw >> (b + (8 - bitsInCW))) & 1) == 1);
        }

        // 5. Build the working grid (finder, separator, timing, format reserved)
        var grid = BuildGrid(cfg);

        // 6. Place data bits into the grid using the two-column zigzag
        PlaceBits(grid, bits);

        // 7. Evaluate all 4 mask patterns and choose the one with lowest penalty
        var bestMask = 0;
        var bestPenalty = int.MaxValue;
        for (var m = 0; m < 4; m++)
        {
            var maskedModules = ApplyMask(grid.Modules, grid.Reserved, cfg.Size, m);
            var fmtBits = Tables.FormatTable[cfg.SymbolIndicator][m];
            // Write format info into a temporary copy for penalty evaluation
            var tmpModules = (bool[,])maskedModules.Clone();
            WriteFormatInfoTo(tmpModules, fmtBits);
            var p = ComputePenalty(tmpModules, cfg.Size);
            if (p < bestPenalty)
            {
                bestPenalty = p;
                bestMask = m;
            }
        }

        // 8. Apply best mask and write final format information
        var finalModules = ApplyMask(grid.Modules, grid.Reserved, cfg.Size, bestMask);
        WriteFormatInfoTo(finalModules, Tables.FormatTable[cfg.SymbolIndicator][bestMask]);

        // 9. Convert bool[,] to ModuleGrid using the Create + SetModule API.
        //    ModuleGrid.Create() initialises everything to false (light);
        //    we then flip only the dark modules.
        var result = ModuleGrid.Create(cfg.Size, cfg.Size, ModuleShape.Square);
        for (var r = 0; r < cfg.Size; r++)
        for (var c = 0; c < cfg.Size; c++)
        {
            if (finalModules[r, c])
                result = result.SetModule(r, c, true);
        }

        return result;
    }

    // ── Version / config selection ────────────────────────────────────────────

    /// <summary>
    /// Find the smallest symbol configuration that can hold the given input.
    ///
    /// <para>
    /// Auto-selection iterates SYMBOL_CONFIGS in M1→M4-Q order and returns
    /// the first configuration where the input fits within the symbol's
    /// character capacity for the chosen encoding mode.
    /// </para>
    /// </summary>
    internal static SymbolConfig SelectConfig(
        string input,
        MicroQRVersion? version,
        MicroQREccLevel? ecc)
    {
        var candidates = Tables.SymbolConfigs
            .Where(c => (version == null || c.Version == version) &&
                        (ecc == null || c.Ecc == ecc))
            .ToArray();

        if (candidates.Length == 0)
            throw new EccNotAvailableException(
                $"No symbol configuration matches version={version?.ToString() ?? "any"} " +
                $"ecc={ecc?.ToString() ?? "any"}.");

        foreach (var cfg in candidates)
        {
            try
            {
                var mode = SelectMode(input, cfg);
                var byteLen = Encoding.UTF8.GetByteCount(input);
                var len = mode == EncodingMode.Byte ? byteLen : input.Length;
                var cap = mode == EncodingMode.Numeric   ? cfg.NumericCap :
                          mode == EncodingMode.Alphanumeric ? cfg.AlphaCap :
                          cfg.ByteCap;
                if (cap >= 0 && len <= cap)
                    return cfg;
            }
            catch (UnsupportedModeException)
            {
                // Mode not supported or input doesn't fit — try next config
            }
        }

        throw new InputTooLongException(
            $"Input \"{(input.Length > 20 ? input[..20] + "…" : input)}\" " +
            $"(length {input.Length}) does not fit in any Micro QR symbol " +
            $"(version={version?.ToString() ?? "any"}, ecc={ecc?.ToString() ?? "any"}). " +
            $"Maximum is 35 numeric characters in M4-L.");
    }

    // ── Mode selection ────────────────────────────────────────────────────────

    /// <summary>
    /// Determine the most compact encoding mode that covers the full input
    /// and is supported by the given symbol configuration.
    ///
    /// <para>
    /// Selection order (most compact to least):
    /// <list type="number">
    ///   <item>Numeric — all chars are 0–9</item>
    ///   <item>Alphanumeric — all chars in the 45-char set</item>
    ///   <item>Byte — raw UTF-8 bytes</item>
    /// </list>
    /// </para>
    /// </summary>
    internal static EncodingMode SelectMode(string input, SymbolConfig cfg)
    {
        var isNumeric = input.Length == 0 || input.All(c => c >= '0' && c <= '9');
        if (isNumeric && cfg.CharCountBitsNumeric > 0)
            return EncodingMode.Numeric;

        var isAlpha = input.All(c => Tables.AlphanumChars.Contains(c));
        if (isAlpha && cfg.AlphaCap > 0)
            return EncodingMode.Alphanumeric;

        if (cfg.ByteCap > 0)
            return EncodingMode.Byte;

        throw new UnsupportedModeException(
            $"Input cannot be encoded in any mode supported by {cfg.Version}-{cfg.Ecc}. " +
            $"Use a higher version or switch to byte mode.");
    }

    // ── Data codeword assembly ────────────────────────────────────────────────

    /// <summary>
    /// Build the complete data codeword byte sequence.
    ///
    /// <para>
    /// For all symbols except M1:
    /// <code>
    ///   [mode indicator] [char count] [data bits] [terminator] [byte-align pad] [0xEC/0x11 fill]
    ///   Total: exactly cfg.DataCW bytes.
    /// </code>
    /// </para>
    ///
    /// <para>
    /// For M1 (M1HalfCW = true):
    /// Total data capacity is 20 bits. The RS encoder receives 3 bytes where
    /// byte[2] has data in the upper 4 bits and 0000 in the lower 4 bits.
    /// No 0xEC/0x11 padding is used for M1.
    /// </para>
    /// </summary>
    internal static List<byte> BuildDataCodewords(string input, SymbolConfig cfg)
    {
        var mode = SelectMode(input, cfg);

        // Total usable data bits
        var totalBits = cfg.M1HalfCW
            ? cfg.DataCW * 8 - 4   // M1: 3×8 − 4 = 20 usable bits
            : cfg.DataCW * 8;

        var w = new BitWriter();

        // Mode indicator (0 bits for M1, otherwise 1/2/3 bits)
        if (cfg.ModeIndicatorBits > 0)
            w.Write(ModeIndicatorValue(mode, cfg), cfg.ModeIndicatorBits);

        // Character count field
        var ccBits  = CharCountBits(mode, cfg);
        var byteInput = Encoding.UTF8.GetBytes(input);
        var charCount = mode == EncodingMode.Byte ? byteInput.Length : input.Length;
        w.Write(charCount, ccBits);

        // Encoded data
        switch (mode)
        {
            case EncodingMode.Numeric:      EncodeNumeric(input, w);      break;
            case EncodingMode.Alphanumeric: EncodeAlphanumeric(input, w); break;
            default:                        EncodeByte(input, w);         break;
        }

        // Terminator: up to terminatorBits zero bits (truncated if capacity is full)
        var remaining = totalBits - w.BitLength;
        if (remaining > 0)
            w.Write(0, Math.Min(cfg.TerminatorBits, remaining));

        if (cfg.M1HalfCW)
        {
            // M1: pad to exactly 20 bits, then pack into 3 bytes.
            // The last byte has data in the upper 4 bits, lower 4 bits = 0000.
            var bits = w.ToBits();
            while (bits.Count < 20) bits.Add(0);
            if (bits.Count > 20) bits.RemoveRange(20, bits.Count - 20);

            var b0 = PackByte(bits, 0);
            var b1 = PackByte(bits, 8);
            // Last byte: upper nibble only
            var b2 = (byte)(
                (bits[16] << 7) | (bits[17] << 6) | (bits[18] << 5) | (bits[19] << 4));
            return [b0, b1, b2];
        }

        // Pad to byte boundary
        var rem = w.BitLength % 8;
        if (rem != 0) w.Write(0, 8 - rem);

        // Fill remaining data codewords with alternating 0xEC / 0x11
        var bytes = w.ToBytes();
        var padByte = (byte)0xEC;
        while (bytes.Count < cfg.DataCW)
        {
            bytes.Add(padByte);
            padByte = padByte == 0xEC ? (byte)0x11 : (byte)0xEC;
        }
        return bytes;
    }

    private static byte PackByte(List<int> bits, int offset)
    {
        var b = 0;
        for (var i = 0; i < 8; i++)
            b = (b << 1) | (offset + i < bits.Count ? bits[offset + i] : 0);
        return (byte)b;
    }

    private static int ModeIndicatorValue(EncodingMode mode, SymbolConfig cfg) =>
        cfg.ModeIndicatorBits switch
        {
            0 => 0,  // M1: no indicator
            1 => mode == EncodingMode.Numeric ? 0 : 1,
            2 => mode == EncodingMode.Numeric ? 0b00 :
                 mode == EncodingMode.Alphanumeric ? 0b01 : 0b10,
            _ => mode == EncodingMode.Numeric ? 0b000 :
                 mode == EncodingMode.Alphanumeric ? 0b001 : 0b010,
        };

    private static int CharCountBits(EncodingMode mode, SymbolConfig cfg) =>
        mode == EncodingMode.Numeric      ? cfg.CharCountBitsNumeric :
        mode == EncodingMode.Alphanumeric ? cfg.CharCountBitsAlpha   :
        cfg.CharCountBitsByte;

    // ── Numeric mode encoding ─────────────────────────────────────────────────
    //
    // Groups of 3 digits → 10 bits (values 0–999).
    // Remaining pair     →  7 bits (values 0–99).
    // Single trailing    →  4 bits (values 0–9).
    //
    // Example: "12345" → "123" (10b=0001111011) + "45" (7b=0101101) = 17 bits.

    private static void EncodeNumeric(string input, BitWriter w)
    {
        var i = 0;
        while (i + 2 < input.Length)
        {
            w.Write(int.Parse(input.Substring(i, 3)), 10);
            i += 3;
        }
        if (i + 1 < input.Length)
        {
            w.Write(int.Parse(input.Substring(i, 2)), 7);
            i += 2;
        }
        if (i < input.Length)
            w.Write(int.Parse(input[i].ToString()), 4);
    }

    // ── Alphanumeric mode encoding ────────────────────────────────────────────
    //
    // Pairs encode as (firstIndex × 45 + secondIndex) in 11 bits.
    // A trailing single character uses 6 bits.

    private static void EncodeAlphanumeric(string input, BitWriter w)
    {
        var i = 0;
        while (i + 1 < input.Length)
        {
            var a = Tables.AlphanumChars.IndexOf(input[i]);
            var b = Tables.AlphanumChars.IndexOf(input[i + 1]);
            if (a < 0 || b < 0)
                throw new InvalidCharacterException(
                    $"Character not in alphanumeric set: '{(a < 0 ? input[i] : input[i + 1])}'");
            w.Write(a * 45 + b, 11);
            i += 2;
        }
        if (i < input.Length)
        {
            var a = Tables.AlphanumChars.IndexOf(input[i]);
            if (a < 0)
                throw new InvalidCharacterException(
                    $"Character not in alphanumeric set: '{input[i]}'");
            w.Write(a, 6);
        }
    }

    // ── Byte mode encoding ────────────────────────────────────────────────────
    //
    // Each UTF-8 byte is written as 8 bits.
    // Multi-byte UTF-8 code points: each byte counts separately in the
    // character count and contributes 8 bits to the stream.

    private static void EncodeByte(string input, BitWriter w)
    {
        foreach (var b in Encoding.UTF8.GetBytes(input))
            w.Write(b, 8);
    }

    // ── Reed-Solomon encoder ──────────────────────────────────────────────────

    /// <summary>
    /// Compute <paramref name="eccCount"/> ECC bytes using GF(256)/0x11D.
    ///
    /// <para>
    /// This is the LFSR (Linear Feedback Shift Register) implementation of
    /// polynomial remainder division. The resulting array is the remainder of
    /// D(x)·x^n mod G(x).
    /// </para>
    ///
    /// <code>
    /// ecc = [0] × n
    /// for each data byte b:
    ///   feedback = b XOR ecc[0]
    ///   shift ecc left (drop ecc[0], append 0)
    ///   for i in 0..n-1:
    ///     ecc[i] ^= G[i+1] × feedback   (GF multiplication)
    /// </code>
    /// </summary>
    internal static byte[] RsEncode(List<byte> data, int eccCount)
    {
        if (!Tables.RsGenerators.TryGetValue(eccCount, out var gen))
            throw new MicroQRException($"No generator polynomial for eccCount={eccCount}");

        var n = eccCount;
        var rem = new byte[n];

        foreach (var b in data)
        {
            var fb = (byte)(b ^ rem[0]);
            // Shift register left: drop rem[0], shift everything down, append 0
            Array.Copy(rem, 1, rem, 0, n - 1);
            rem[n - 1] = 0;
            if (fb != 0)
            {
                for (var i = 0; i < n; i++)
                    rem[i] ^= CodingAdventures.Gf256.Gf256.Multiply((byte)gen[i + 1], fb);
            }
        }
        return rem;
    }

    // ── Grid initialization ────────────────────────────────────────────────────

    private static WorkGrid BuildGrid(SymbolConfig cfg)
    {
        var g = new WorkGrid(cfg.Size);
        PlaceFinder(g);
        PlaceSeparator(g);
        PlaceTiming(g);
        ReserveFormatInfo(g);
        return g;
    }

    /// <summary>
    /// Place the 7×7 finder pattern at the top-left corner (rows 0–6, cols 0–6).
    ///
    /// <code>
    /// ■ ■ ■ ■ ■ ■ ■
    /// ■ □ □ □ □ □ ■
    /// ■ □ ■ ■ ■ □ ■
    /// ■ □ ■ ■ ■ □ ■
    /// ■ □ ■ ■ ■ □ ■
    /// ■ □ □ □ □ □ ■
    /// ■ ■ ■ ■ ■ ■ ■
    /// </code>
    ///
    /// Dark modules form the outer border and a 3×3 inner core.
    /// This 1:1:3:1:1 dark:light ratio is what scanners detect.
    /// </summary>
    private static void PlaceFinder(WorkGrid g)
    {
        for (var dr = 0; dr < 7; dr++)
        for (var dc = 0; dc < 7; dc++)
        {
            var onBorder = dr == 0 || dr == 6 || dc == 0 || dc == 6;
            var inCore   = dr >= 2 && dr <= 4 && dc >= 2 && dc <= 4;
            g.Set(dr, dc, onBorder || inCore, reserve: true);
        }
    }

    /// <summary>
    /// Place the L-shaped separator (light modules bordering the finder on
    /// its bottom and right sides).
    ///
    /// In regular QR, each finder has separators on all four sides. In Micro
    /// QR, the finder is in the top-left corner so the top and left edges are
    /// the symbol boundary — only the bottom and right need separators:
    ///
    /// <code>
    ///   Row 7, cols 0–7  (bottom of finder + corner)
    ///   Col 7, rows 0–7  (right of finder + corner)
    /// </code>
    /// </summary>
    private static void PlaceSeparator(WorkGrid g)
    {
        for (var i = 0; i <= 7; i++)
        {
            g.Set(7, i, dark: false, reserve: true);  // bottom row
            g.Set(i, 7, dark: false, reserve: true);  // right column
        }
    }

    /// <summary>
    /// Place the timing pattern extensions.
    ///
    /// Unlike regular QR where timing runs along row 6 and col 6, Micro QR
    /// places timing along row 0 and col 0. The first 7 positions overlap the
    /// finder; position 7 is the separator (light). Positions 8+ alternate:
    ///
    /// <code>
    ///   Dark at even index: col/row 8, 10, 12, ...
    ///   Light at odd index: col/row 9, 11, 13, ...
    /// </code>
    /// </summary>
    private static void PlaceTiming(WorkGrid g)
    {
        var sz = g.Size;
        for (var c = 8; c < sz; c++) g.Set(0, c, c % 2 == 0, reserve: true);
        for (var r = 8; r < sz; r++) g.Set(r, 0, r % 2 == 0, reserve: true);
    }

    /// <summary>
    /// Reserve format information module positions.
    ///
    /// The 15 format modules form an L-shape:
    ///
    /// <code>
    ///   Row 8, cols 1–8  → 8 modules (hold bits f14..f7, MSB first)
    ///   Col 8, rows 1–7  → 7 modules (hold bits f6..f0, LSB at row 1)
    /// </code>
    ///
    /// Note: unlike regular QR which has TWO copies of format info, Micro QR
    /// has only ONE.
    /// </summary>
    private static void ReserveFormatInfo(WorkGrid g)
    {
        for (var c = 1; c <= 8; c++) g.Set(8, c, dark: false, reserve: true);
        for (var r = 1; r <= 7; r++) g.Set(r, 8, dark: false, reserve: true);
    }

    // ── Data placement ─────────────────────────────────────────────────────────

    /// <summary>
    /// Place the final codeword stream into the grid using the two-column zigzag.
    ///
    /// <para>
    /// The zigzag scans from the bottom-right corner, moving left two columns at
    /// a time, alternating upward and downward direction. Reserved modules
    /// (finder/separator/timing/format) are skipped automatically.
    /// </para>
    ///
    /// <para>
    /// Key difference from regular QR: no timing column skip at col 6.
    /// Micro QR timing is at col 0, which is fully reserved and thus skipped
    /// automatically by the reserved-module check.
    /// </para>
    /// </summary>
    private static void PlaceBits(WorkGrid g, List<bool> bits)
    {
        var sz = g.Size;
        var bitIdx = 0;
        var up = true;

        for (var col = sz - 1; col >= 1; col -= 2)
        {
            // Build row sequence depending on direction
            var rows = new int[sz];
            for (var i = 0; i < sz; i++)
                rows[i] = up ? (sz - 1 - i) : i;

            foreach (var row in rows)
            {
                for (var dc = 0; dc <= 1; dc++)
                {
                    var c = col - dc;
                    if (g.Reserved[row, c]) continue;
                    g.Modules[row, c] = bitIdx < bits.Count ? bits[bitIdx++] : false;
                }
            }
            up = !up;
        }
    }

    // ── Masking ────────────────────────────────────────────────────────────────

    /// <summary>
    /// Apply mask pattern <paramref name="maskIdx"/> to all non-reserved modules.
    /// Returns a new module array; the original is not modified.
    /// </summary>
    private static bool[,] ApplyMask(bool[,] modules, bool[,] reserved, int sz, int maskIdx)
    {
        var cond = Tables.MaskConditions[maskIdx];
        var result = (bool[,])modules.Clone();
        for (var r = 0; r < sz; r++)
        for (var c = 0; c < sz; c++)
        {
            if (!reserved[r, c])
                result[r, c] = modules[r, c] != cond(r, c);
        }
        return result;
    }

    // ── Format information placement ───────────────────────────────────────────

    /// <summary>
    /// Write format information bits into the reserved positions of
    /// <paramref name="modules"/> (modified in place).
    ///
    /// <para>
    /// Placement (f14 = MSB):
    /// <list type="bullet">
    ///   <item>Row 8, col 1 ← f14, col 2 ← f13, …, col 8 ← f7</item>
    ///   <item>Col 8, row 7 ← f6, row 6 ← f5, …, row 1 ← f0 (LSB)</item>
    /// </list>
    /// </para>
    /// </summary>
    private static void WriteFormatInfoTo(bool[,] modules, int fmtBits)
    {
        // Row 8, cols 1–8: bits f14 down to f7
        for (var i = 0; i < 8; i++)
            modules[8, 1 + i] = ((fmtBits >> (14 - i)) & 1) == 1;

        // Col 8, rows 7 down to 1: bits f6 down to f0
        for (var i = 0; i < 7; i++)
            modules[7 - i, 8] = ((fmtBits >> (6 - i)) & 1) == 1;
    }

    // ── Penalty scoring ────────────────────────────────────────────────────────

    /// <summary>
    /// Compute the 4-rule penalty score for a candidate masked grid.
    ///
    /// <para>
    /// <b>Rule 1</b> — Adjacent same-color runs of 5+ modules in any row or column.
    /// Score += (run_length − 2) for each run of length ≥ 5.
    /// </para>
    ///
    /// <para>
    /// <b>Rule 2</b> — 2×2 all-same-color blocks.
    /// Score += 3 for each qualifying 2×2 square.
    /// </para>
    ///
    /// <para>
    /// <b>Rule 3</b> — Finder-pattern-like 11-module sequences.
    /// Score += 40 for each occurrence of 1 0 1 1 1 0 1 0 0 0 0 or its reverse
    /// in any row or column.
    /// </para>
    ///
    /// <para>
    /// <b>Rule 4</b> — Dark/light proportion deviation from 50%.
    /// Score += min(|prev5 − 50|, |next5 − 50|) / 5 × 10.
    /// </para>
    /// </summary>
    internal static int ComputePenalty(bool[,] modules, int sz)
    {
        var penalty = 0;

        // ── Rule 1: adjacent same-color runs of ≥ 5 ──────────────────────────
        for (var a = 0; a < sz; a++)
        {
            foreach (var horiz in new[] { true, false })
            {
                var run = 1;
                var prev = horiz ? modules[a, 0] : modules[0, a];
                for (var i = 1; i < sz; i++)
                {
                    var cur = horiz ? modules[a, i] : modules[i, a];
                    if (cur == prev)
                    {
                        run++;
                    }
                    else
                    {
                        if (run >= 5) penalty += run - 2;
                        run = 1;
                        prev = cur;
                    }
                }
                if (run >= 5) penalty += run - 2;
            }
        }

        // ── Rule 2: 2×2 same-color blocks ────────────────────────────────────
        for (var r = 0; r < sz - 1; r++)
        for (var c = 0; c < sz - 1; c++)
        {
            var d = modules[r, c];
            if (d == modules[r, c + 1] && d == modules[r + 1, c] && d == modules[r + 1, c + 1])
                penalty += 3;
        }

        // ── Rule 3: finder-pattern-like sequences ─────────────────────────────
        // These 11-module patterns mimic a finder pattern and must be penalized.
        var p1 = new[] { 1, 0, 1, 1, 1, 0, 1, 0, 0, 0, 0 };
        var p2 = new[] { 0, 0, 0, 0, 1, 0, 1, 1, 1, 0, 1 };
        for (var a = 0; a < sz; a++)
        for (var b = 0; b <= sz - 11; b++)
        {
            bool mH1 = true, mH2 = true, mV1 = true, mV2 = true;
            for (var k = 0; k < 11; k++)
            {
                var bH = modules[a, b + k] ? 1 : 0;
                var bV = modules[b + k, a] ? 1 : 0;
                if (bH != p1[k]) mH1 = false;
                if (bH != p2[k]) mH2 = false;
                if (bV != p1[k]) mV1 = false;
                if (bV != p2[k]) mV2 = false;
            }
            if (mH1) penalty += 40;
            if (mH2) penalty += 40;
            if (mV1) penalty += 40;
            if (mV2) penalty += 40;
        }

        // ── Rule 4: dark proportion deviation ────────────────────────────────
        var dark = 0;
        for (var r = 0; r < sz; r++)
        for (var c = 0; c < sz; c++)
            if (modules[r, c]) dark++;

        var darkPct = (double)dark / (sz * sz) * 100.0;
        var prev5 = (int)(darkPct / 5) * 5;
        penalty += (int)(Math.Min(Math.Abs(prev5 - 50), Math.Abs(prev5 + 5 - 50)) / 5.0 * 10);

        return penalty;
    }
}
