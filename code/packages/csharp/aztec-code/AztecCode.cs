using System.Text;
using CodingAdventures.Barcode2D;

namespace CodingAdventures.AztecCode;

/// <summary>Options controlling Aztec Code encoding.</summary>
public sealed record AztecOptions(int MinEccPercent = 23)
{
    /// <summary>Default options: 23 percent minimum error correction.</summary>
    public static readonly AztecOptions Default = new();
}

/// <summary>Base class for Aztec Code encoding errors.</summary>
public class AztecCodeException : Exception
{
    /// <summary>Create an Aztec Code exception with a message.</summary>
    public AztecCodeException(string message)
        : base(message)
    {
    }
}

/// <summary>The input is too long for the largest supported Aztec symbol.</summary>
public sealed class InputTooLongException : AztecCodeException
{
    /// <summary>Create an input-too-long error.</summary>
    public InputTooLongException(string message)
        : base(message)
    {
    }
}

/// <summary>Encoder options are outside the supported range.</summary>
public sealed class InvalidAztecOptionsException : AztecCodeException
{
    /// <summary>Create an invalid-options error.</summary>
    public InvalidAztecOptionsException(string message)
        : base(message)
    {
    }
}

/// <summary>
/// ISO/IEC 24778 Aztec Code encoder.
///
/// <para>
/// Version 0.1.0 implements the same byte-mode pipeline as the F# package:
/// binary-shift input bytes, automatic compact/full symbol selection,
/// GF(256)/0x12D Reed-Solomon ECC, bit stuffing, GF(16) mode-message ECC,
/// and clockwise layer placement into a <see cref="ModuleGrid"/>.
/// </para>
/// </summary>
public static class AztecCodeEncoder
{
    /// <summary>Package version.</summary>
    public const string Version = "0.1.0";

    private const int MaxInputBytes = 4096;
    private const int Gf256Poly = 0x12D;

    private static readonly int[] Log16 =
    [
        -1, 0, 1, 4, 2, 8, 5, 10, 3, 14, 9, 7, 6, 13, 11, 12,
    ];

    private static readonly int[] Alog16 =
    [
        1, 2, 4, 8, 3, 6, 12, 11, 5, 10, 7, 14, 15, 13, 9, 1,
    ];

    private static readonly int[] Exp12D = new int[512];
    private static readonly int[] Log12D = new int[256];

    private static readonly (int TotalBits, int MaxBytes8)[] CompactCapacity =
    [
        (0, 0),
        (72, 9),
        (200, 25),
        (392, 49),
        (648, 81),
    ];

    private static readonly (int TotalBits, int MaxBytes8)[] FullCapacity =
    [
        (0, 0),
        (88, 11),
        (216, 27),
        (360, 45),
        (520, 65),
        (696, 87),
        (888, 111),
        (1096, 137),
        (1320, 165),
        (1560, 195),
        (1816, 227),
        (2088, 261),
        (2376, 297),
        (2680, 335),
        (3000, 375),
        (3336, 417),
        (3688, 461),
        (4056, 507),
        (4440, 555),
        (4840, 605),
        (5256, 657),
        (5688, 711),
        (6136, 767),
        (6600, 825),
        (7080, 885),
        (7576, 947),
        (8088, 1011),
        (8616, 1077),
        (9160, 1145),
        (9720, 1215),
        (10296, 1287),
        (10888, 1361),
        (11496, 1437),
    ];

    static AztecCodeEncoder()
    {
        var value = 1;
        for (var i = 0; i < 255; i++)
        {
            Exp12D[i] = value;
            Exp12D[i + 255] = value;
            Log12D[value] = i;
            value <<= 1;
            if ((value & 0x100) != 0)
            {
                value ^= Gf256Poly;
            }

            value &= 0xFF;
        }

        Exp12D[255] = 1;
    }

    /// <summary>Encode a UTF-8 string with default options.</summary>
    public static ModuleGrid Encode(string input) => Encode(input, AztecOptions.Default);

    /// <summary>Encode a UTF-8 string with caller-supplied options.</summary>
    public static ModuleGrid Encode(string input, AztecOptions? options)
    {
        ArgumentNullException.ThrowIfNull(input);
        return EncodeBytes(Encoding.UTF8.GetBytes(input), options);
    }

    /// <summary>Encode raw bytes with default options.</summary>
    public static ModuleGrid EncodeBytes(byte[] input) => EncodeBytes(input, AztecOptions.Default);

    /// <summary>Encode raw bytes with caller-supplied options.</summary>
    public static ModuleGrid EncodeBytes(byte[] input, AztecOptions? options)
    {
        ArgumentNullException.ThrowIfNull(input);
        options ??= AztecOptions.Default;

        if (options.MinEccPercent < 10 || options.MinEccPercent > 90)
        {
            throw new InvalidAztecOptionsException(
                $"MinEccPercent must be between 10 and 90, got {options.MinEccPercent}.");
        }

        if (input.Length > MaxInputBytes)
        {
            throw new InputTooLongException(
                $"Input is {input.Length} bytes; maximum supported is {MaxInputBytes}.");
        }

        var dataBits = EncodeBytesAsBits(input);
        var spec = SelectSymbol(dataBits.Length, options.MinEccPercent);

        var paddedBits = PadToBytes(dataBits, spec.DataCwCount);
        var dataBytes = new int[spec.DataCwCount];
        for (var i = 0; i < spec.DataCwCount; i++)
        {
            var codeword = 0;
            for (var b = 0; b < 8; b++)
            {
                codeword = (codeword << 1) | paddedBits[i * 8 + b];
            }

            dataBytes[i] = codeword == 0 && i == spec.DataCwCount - 1
                ? 0xFF
                : codeword;
        }

        var eccBytes = Gf256RsEncode(dataBytes, spec.EccCwCount);
        var allBytes = dataBytes.Concat(eccBytes).ToArray();
        var rawBits = new int[allBytes.Length * 8];
        for (var i = 0; i < allBytes.Length; i++)
        {
            var codeword = allBytes[i];
            for (var b = 0; b < 8; b++)
            {
                rawBits[i * 8 + b] = (codeword >> (7 - b)) & 1;
            }
        }

        var stuffedBits = StuffBits(rawBits);
        var modeMessage = EncodeModeMessage(spec.Compact, spec.Layers, spec.DataCwCount);
        return BuildGrid(spec, stuffedBits, modeMessage);
    }

    private sealed record SymbolSpec(
        bool Compact,
        int Layers,
        int DataCwCount,
        int EccCwCount,
        int TotalBits);

    private static int Gf16Mul(int a, int b) =>
        a == 0 || b == 0 ? 0 : Alog16[(Log16[a] + Log16[b]) % 15];

    private static int[] BuildGf16Generator(int n)
    {
        var generator = new[] { 1 };
        for (var i = 1; i <= n; i++)
        {
            var alpha = Alog16[i % 15];
            var next = new int[generator.Length + 1];
            for (var j = 0; j < generator.Length; j++)
            {
                next[j + 1] ^= generator[j];
                next[j] ^= Gf16Mul(alpha, generator[j]);
            }

            generator = next;
        }

        return generator;
    }

    private static int[] Gf16RsEncode(int[] data, int checkCount)
    {
        var generator = BuildGf16Generator(checkCount);
        var rem = new int[checkCount];
        foreach (var nibble in data)
        {
            var feedback = nibble ^ rem[0];
            for (var i = 0; i < checkCount - 1; i++)
            {
                rem[i] = rem[i + 1] ^ Gf16Mul(generator[i + 1], feedback);
            }

            rem[checkCount - 1] = Gf16Mul(generator[checkCount], feedback);
        }

        return rem;
    }

    private static int Gf256Mul(int a, int b) =>
        a == 0 || b == 0 ? 0 : Exp12D[Log12D[a] + Log12D[b]];

    private static int[] BuildGf256Generator(int checkCount)
    {
        var generator = new[] { 1 };
        for (var i = 1; i <= checkCount; i++)
        {
            var alpha = Exp12D[i];
            var next = new int[generator.Length + 1];
            for (var j = 0; j < generator.Length; j++)
            {
                next[j] ^= generator[j];
                next[j + 1] ^= Gf256Mul(generator[j], alpha);
            }

            generator = next;
        }

        return generator;
    }

    private static int[] Gf256RsEncode(int[] data, int checkCount)
    {
        var generator = BuildGf256Generator(checkCount);
        var rem = new int[checkCount];
        foreach (var codeword in data)
        {
            var feedback = codeword ^ rem[0];
            for (var i = 0; i < checkCount - 1; i++)
            {
                rem[i] = rem[i + 1] ^ Gf256Mul(generator[i + 1], feedback);
            }

            rem[checkCount - 1] = Gf256Mul(generator[checkCount], feedback);
        }

        return rem;
    }

    private static int[] EncodeBytesAsBits(byte[] input)
    {
        var bits = new List<int>();
        WriteBits(bits, 31, 5);

        if (input.Length <= 31)
        {
            WriteBits(bits, input.Length, 5);
        }
        else
        {
            WriteBits(bits, 0, 5);
            WriteBits(bits, input.Length, 11);
        }

        foreach (var value in input)
        {
            WriteBits(bits, value, 8);
        }

        return bits.ToArray();
    }

    private static void WriteBits(List<int> bits, int value, int count)
    {
        for (var i = count - 1; i >= 0; i--)
        {
            bits.Add((value >> i) & 1);
        }
    }

    private static SymbolSpec SelectSymbol(int dataBitCount, int minEccPercent)
    {
        var stuffedBitCount = (dataBitCount * 12 + 9) / 10;
        var compact = TryFitFromTable(CompactCapacity, isCompact: true, maxLayers: 4, stuffedBitCount, minEccPercent);
        if (compact is not null)
        {
            return compact;
        }

        var full = TryFitFromTable(FullCapacity, isCompact: false, maxLayers: 32, stuffedBitCount, minEccPercent);
        if (full is not null)
        {
            return full;
        }

        throw new InputTooLongException(
            $"Input is too long to fit in any Aztec Code symbol ({dataBitCount} bits needed).");
    }

    private static SymbolSpec? TryFitFromTable(
        (int TotalBits, int MaxBytes8)[] table,
        bool isCompact,
        int maxLayers,
        int stuffedBitCount,
        int minEccPercent)
    {
        for (var layers = 1; layers <= maxLayers; layers++)
        {
            var (totalBits, totalBytes) = table[layers];
            var eccCwCount = (minEccPercent * totalBytes + 99) / 100;
            var dataCwCount = totalBytes - eccCwCount;
            var neededBytes = (stuffedBitCount + 7) / 8;
            if (dataCwCount > 0 && neededBytes <= dataCwCount)
            {
                return new SymbolSpec(isCompact, layers, dataCwCount, eccCwCount, totalBits);
            }
        }

        return null;
    }

    private static int[] PadToBytes(int[] bits, int targetBytes)
    {
        var target = targetBytes * 8;
        if (bits.Length >= target)
        {
            return bits.Take(target).ToArray();
        }

        var output = new int[target];
        Array.Copy(bits, output, bits.Length);
        return output;
    }

    private static int[] StuffBits(int[] bits)
    {
        var stuffed = new List<int>();
        var runValue = -1;
        var runLength = 0;

        foreach (var bit in bits)
        {
            if (bit == runValue)
            {
                runLength++;
            }
            else
            {
                runValue = bit;
                runLength = 1;
            }

            stuffed.Add(bit);

            if (runLength == 4)
            {
                var stuffBit = 1 - bit;
                stuffed.Add(stuffBit);
                runValue = stuffBit;
                runLength = 1;
            }
        }

        return stuffed.ToArray();
    }

    private static int[] EncodeModeMessage(bool compact, int layers, int dataCwCount)
    {
        int[] dataNibbles;
        int numEcc;

        if (compact)
        {
            var mode = ((layers - 1) << 6) | (dataCwCount - 1);
            dataNibbles = [mode & 0xF, (mode >> 4) & 0xF];
            numEcc = 5;
        }
        else
        {
            var mode = ((layers - 1) << 11) | (dataCwCount - 1);
            dataNibbles = [mode & 0xF, (mode >> 4) & 0xF, (mode >> 8) & 0xF, (mode >> 12) & 0xF];
            numEcc = 6;
        }

        var allNibbles = dataNibbles.Concat(Gf16RsEncode(dataNibbles, numEcc)).ToArray();
        var bits = new List<int>(allNibbles.Length * 4);
        foreach (var nibble in allNibbles)
        {
            WriteBits(bits, nibble, 4);
        }

        return bits.ToArray();
    }

    private static ModuleGrid BuildGrid(SymbolSpec spec, int[] stuffedBits, int[] modeMessage)
    {
        var size = SymbolSize(spec.Compact, spec.Layers);
        var center = size / 2;
        var modules = new bool[size, size];
        var reserved = new bool[size, size];

        if (!spec.Compact)
        {
            DrawReferenceGrid(modules, reserved, center, center, size);
        }

        DrawBullseye(modules, reserved, center, center, spec.Compact);
        var modeRingRemaining = DrawOrientationAndModeMessage(modules, reserved, center, center, spec.Compact, modeMessage);
        PlaceDataBits(modules, reserved, stuffedBits, center, center, spec.Compact, spec.Layers, modeRingRemaining);
        return ToModuleGrid(modules);
    }

    private static int SymbolSize(bool compact, int layers) =>
        compact ? 11 + 4 * layers : 15 + 4 * layers;

    private static int BullseyeRadius(bool compact) => compact ? 5 : 7;

    private static void DrawBullseye(bool[,] modules, bool[,] reserved, int cx, int cy, bool compact)
    {
        var radius = BullseyeRadius(compact);
        for (var row = cy - radius; row <= cy + radius; row++)
        {
            for (var col = cx - radius; col <= cx + radius; col++)
            {
                var distance = Math.Max(Math.Abs(col - cx), Math.Abs(row - cy));
                var dark = distance <= 1 || distance % 2 == 1;
                modules[row, col] = dark;
                reserved[row, col] = true;
            }
        }
    }

    private static void DrawReferenceGrid(bool[,] modules, bool[,] reserved, int cx, int cy, int size)
    {
        for (var row = 0; row < size; row++)
        {
            for (var col = 0; col < size; col++)
            {
                var onHorizontal = (cy - row) % 16 == 0;
                var onVertical = (cx - col) % 16 == 0;
                if (!onHorizontal && !onVertical)
                {
                    continue;
                }

                var dark = onHorizontal && onVertical
                    ? true
                    : onHorizontal
                        ? (cx - col) % 2 == 0
                        : (cy - row) % 2 == 0;
                modules[row, col] = dark;
                reserved[row, col] = true;
            }
        }
    }

    private static List<(int Col, int Row)> DrawOrientationAndModeMessage(
        bool[,] modules,
        bool[,] reserved,
        int cx,
        int cy,
        bool compact,
        int[] modeMessageBits)
    {
        var ring = BullseyeRadius(compact) + 1;
        var nonCorner = new List<(int Col, int Row)>();

        for (var col = cx - ring + 1; col <= cx + ring - 1; col++)
        {
            nonCorner.Add((col, cy - ring));
        }

        for (var row = cy - ring + 1; row <= cy + ring - 1; row++)
        {
            nonCorner.Add((cx + ring, row));
        }

        for (var col = cx + ring - 1; col >= cx - ring + 1; col--)
        {
            nonCorner.Add((col, cy + ring));
        }

        for (var row = cy + ring - 1; row >= cy - ring + 1; row--)
        {
            nonCorner.Add((cx - ring, row));
        }

        (int Col, int Row)[] corners =
        [
            (cx - ring, cy - ring),
            (cx + ring, cy - ring),
            (cx + ring, cy + ring),
            (cx - ring, cy + ring),
        ];

        foreach (var (col, row) in corners)
        {
            modules[row, col] = true;
            reserved[row, col] = true;
        }

        var count = Math.Min(modeMessageBits.Length, nonCorner.Count);
        for (var i = 0; i < count; i++)
        {
            var (col, row) = nonCorner[i];
            modules[row, col] = modeMessageBits[i] == 1;
            reserved[row, col] = true;
        }

        return nonCorner.Skip(modeMessageBits.Length).ToList();
    }

    private static void PlaceDataBits(
        bool[,] modules,
        bool[,] reserved,
        int[] bits,
        int cx,
        int cy,
        bool compact,
        int layers,
        List<(int Col, int Row)> modeRingRemainingPositions)
    {
        var size = modules.GetLength(0);
        var bitIndex = 0;

        void PlaceBit(int col, int row)
        {
            if (row < 0 || row >= size || col < 0 || col >= size || reserved[row, col])
            {
                return;
            }

            if (bitIndex < bits.Length)
            {
                modules[row, col] = bits[bitIndex] == 1;
            }

            bitIndex++;
        }

        foreach (var (col, row) in modeRingRemainingPositions)
        {
            if (bitIndex < bits.Length)
            {
                modules[row, col] = bits[bitIndex] == 1;
            }

            bitIndex++;
        }

        var startRadius = BullseyeRadius(compact) + 2;
        for (var layer = 0; layer < layers; layer++)
        {
            var inner = startRadius + 2 * layer;
            var outer = inner + 1;

            for (var col = cx - inner + 1; col <= cx + inner; col++)
            {
                PlaceBit(col, cy - outer);
                PlaceBit(col, cy - inner);
            }

            for (var row = cy - inner + 1; row <= cy + inner; row++)
            {
                PlaceBit(cx + outer, row);
                PlaceBit(cx + inner, row);
            }

            for (var col = cx + inner; col >= cx - inner + 1; col--)
            {
                PlaceBit(col, cy + outer);
                PlaceBit(col, cy + inner);
            }

            for (var row = cy + inner; row >= cy - inner + 1; row--)
            {
                PlaceBit(cx - outer, row);
                PlaceBit(cx - inner, row);
            }
        }
    }

    private static ModuleGrid ToModuleGrid(bool[,] modules)
    {
        var rows = modules.GetLength(0);
        var cols = modules.GetLength(1);
        var grid = ModuleGrid.Create(rows, cols, ModuleShape.Square);

        for (var row = 0; row < rows; row++)
        {
            for (var col = 0; col < cols; col++)
            {
                if (modules[row, col])
                {
                    grid = grid.SetModule(row, col, true);
                }
            }
        }

        return grid;
    }
}
