using System.Text;
using CodingAdventures.Barcode2D;
using Gf256Math = CodingAdventures.Gf256.Gf256;

namespace CodingAdventures.QRCode;

/// <summary>Error-correction level for a QR Code symbol.</summary>
public enum EccLevel
{
    /// <summary>Low error correction, highest capacity.</summary>
    L,
    /// <summary>Medium error correction.</summary>
    M,
    /// <summary>Quartile error correction.</summary>
    Q,
    /// <summary>High error correction, lowest capacity.</summary>
    H,
}

/// <summary>Base class for QR Code encoding errors.</summary>
public class QRCodeException : Exception
{
    /// <summary>Create a QR Code exception with a message.</summary>
    public QRCodeException(string message)
        : base(message)
    {
    }
}

/// <summary>The input cannot fit in a version-40 QR Code at the requested ECC level.</summary>
public sealed class InputTooLongException : QRCodeException
{
    /// <summary>Create an input-too-long error.</summary>
    public InputTooLongException(string message)
        : base(message)
    {
    }
}

/// <summary>ISO/IEC 18004 QR Code encoder.</summary>
public static class QRCodeEncoder
{
    /// <summary>Package version.</summary>
    public const string Version = "0.1.0";

    private const string AlphanumChars = "0123456789ABCDEFGHIJKLMNOPQRSTUVWXYZ $%*+-./:";

    private static readonly int[,] EccCodewordsPerBlock =
    {
        { -1, 7, 10, 15, 20, 26, 18, 20, 24, 30, 18, 20, 24, 26, 30, 22, 24, 28, 30, 28, 28, 28, 28, 30, 30, 26, 28, 30, 30, 30, 30, 30, 30, 30, 30, 30, 30, 30, 30, 30, 30 },
        { -1, 10, 16, 26, 18, 24, 16, 18, 22, 22, 26, 30, 22, 22, 24, 24, 28, 28, 26, 26, 26, 26, 28, 28, 28, 28, 28, 28, 28, 28, 28, 28, 28, 28, 28, 28, 28, 28, 28, 28, 28 },
        { -1, 13, 22, 18, 26, 18, 24, 18, 22, 20, 24, 28, 26, 24, 20, 30, 24, 28, 28, 26, 30, 28, 30, 30, 30, 30, 28, 30, 30, 30, 30, 30, 30, 30, 30, 30, 30, 30, 30, 30, 30 },
        { -1, 17, 28, 22, 16, 22, 28, 26, 26, 24, 28, 24, 28, 22, 24, 24, 30, 28, 28, 26, 28, 30, 24, 30, 30, 30, 30, 30, 30, 30, 30, 30, 30, 30, 30, 30, 30, 30, 30, 30, 30 },
    };

    private static readonly int[,] NumBlocks =
    {
        { -1, 1, 1, 1, 1, 1, 2, 2, 2, 2, 4, 4, 4, 4, 4, 6, 6, 6, 6, 7, 8, 8, 9, 9, 10, 12, 12, 12, 13, 14, 15, 16, 17, 18, 19, 19, 20, 21, 22, 24, 25 },
        { -1, 1, 1, 1, 2, 2, 4, 4, 4, 5, 5, 5, 8, 9, 9, 10, 10, 11, 13, 14, 16, 17, 17, 18, 20, 21, 23, 25, 26, 28, 29, 31, 33, 35, 37, 38, 40, 43, 45, 47, 49 },
        { -1, 1, 1, 2, 2, 4, 4, 6, 6, 8, 8, 8, 10, 12, 16, 12, 17, 16, 18, 21, 20, 23, 23, 25, 27, 29, 34, 34, 35, 38, 40, 43, 45, 48, 51, 53, 56, 59, 62, 65, 68 },
        { -1, 1, 1, 2, 4, 4, 4, 5, 6, 8, 8, 11, 11, 16, 16, 18, 16, 19, 21, 25, 25, 25, 34, 30, 32, 35, 37, 40, 42, 45, 48, 51, 54, 57, 60, 63, 66, 70, 74, 77, 80 },
    };

    private static readonly byte[][] AlignmentPositions =
    [
        [],
        [6, 18],
        [6, 22],
        [6, 26],
        [6, 30],
        [6, 34],
        [6, 22, 38],
        [6, 24, 42],
        [6, 26, 46],
        [6, 28, 50],
        [6, 30, 54],
        [6, 32, 58],
        [6, 34, 62],
        [6, 26, 46, 66],
        [6, 26, 48, 70],
        [6, 26, 50, 74],
        [6, 30, 54, 78],
        [6, 30, 56, 82],
        [6, 30, 58, 86],
        [6, 34, 62, 90],
        [6, 28, 50, 72, 94],
        [6, 26, 50, 74, 98],
        [6, 30, 54, 78, 102],
        [6, 28, 54, 80, 106],
        [6, 32, 58, 84, 110],
        [6, 30, 58, 86, 114],
        [6, 34, 62, 90, 118],
        [6, 26, 50, 74, 98, 122],
        [6, 30, 54, 78, 102, 126],
        [6, 26, 52, 78, 104, 130],
        [6, 30, 56, 82, 108, 134],
        [6, 34, 60, 86, 112, 138],
        [6, 30, 58, 86, 114, 142],
        [6, 34, 62, 90, 118, 146],
        [6, 30, 54, 78, 102, 126, 150],
        [6, 24, 50, 76, 102, 128, 154],
        [6, 28, 54, 80, 106, 132, 158],
        [6, 32, 58, 84, 110, 136, 162],
        [6, 26, 54, 82, 110, 138, 166],
        [6, 30, 58, 86, 114, 142, 170],
    ];

    /// <summary>Encode using ECC M.</summary>
    public static ModuleGrid Encode(string input) => Encode(input, EccLevel.M);

    /// <summary>Encode a UTF-8 string into a square QR Code module grid.</summary>
    public static ModuleGrid Encode(string input, EccLevel ecc)
    {
        ArgumentNullException.ThrowIfNull(input);
        if (input.Length > 7089)
        {
            throw new InputTooLongException($"Input length {input.Length} exceeds 7089.");
        }

        var version = SelectVersion(input, ecc);
        var size = SymbolSize(version);
        var dataCodewords = BuildDataCodewords(input, version, ecc);
        var blocks = ComputeBlocks(dataCodewords, version, ecc);
        var interleaved = InterleaveBlocks(blocks);
        var grid = BuildGrid(version);

        PlaceBits(grid, interleaved, version);

        var bestMask = 0;
        var bestPenalty = uint.MaxValue;
        for (var mask = 0; mask < 8; mask++)
        {
            var masked = ApplyMask(grid.Modules, grid.Reserved, size, mask);
            var temp = new WorkGrid(size);
            CopyGrid(masked, grid.Reserved, temp);
            WriteFormatInfo(temp, ComputeFormatBits(ecc, mask));
            var penalty = ComputePenalty(temp.Modules, size);
            if (penalty < bestPenalty)
            {
                bestPenalty = penalty;
                bestMask = mask;
            }
        }

        var finalModules = ApplyMask(grid.Modules, grid.Reserved, size, bestMask);
        var finalGrid = new WorkGrid(size);
        CopyGrid(finalModules, grid.Reserved, finalGrid);
        WriteFormatInfo(finalGrid, ComputeFormatBits(ecc, bestMask));
        WriteVersionInfo(finalGrid, version);
        return ToModuleGrid(finalGrid.Modules);
    }

    private enum EncodingMode
    {
        Numeric,
        Alphanumeric,
        Byte,
    }

    private sealed record Block(byte[] Data, byte[] Ecc);

    private sealed class BitWriter
    {
        private readonly List<bool> _bits = [];

        public int BitLength => _bits.Count;

        public void Write(uint value, int count)
        {
            for (var i = count - 1; i >= 0; i--)
            {
                _bits.Add(((value >> i) & 1u) == 1u);
            }
        }

        public byte[] ToBytes()
        {
            var result = new byte[(_bits.Count + 7) / 8];
            for (var i = 0; i < _bits.Count; i++)
            {
                if (_bits[i])
                {
                    result[i / 8] |= (byte)(1 << (7 - i % 8));
                }
            }

            return result;
        }
    }

    private sealed class WorkGrid(int size)
    {
        public int Size { get; } = size;

        public bool[,] Modules { get; } = new bool[size, size];

        public bool[,] Reserved { get; } = new bool[size, size];

        public void Set(int row, int col, bool dark, bool reserve)
        {
            Modules[row, col] = dark;
            if (reserve)
            {
                Reserved[row, col] = true;
            }
        }

        public void Reserve(int row, int col) => Reserved[row, col] = true;
    }

    private static int Indicator(EccLevel ecc) =>
        ecc switch
        {
            EccLevel.L => 0b01,
            EccLevel.M => 0b00,
            EccLevel.Q => 0b11,
            EccLevel.H => 0b10,
            _ => 0b00,
        };

    private static int RowIndex(EccLevel ecc) =>
        ecc switch
        {
            EccLevel.L => 0,
            EccLevel.M => 1,
            EccLevel.Q => 2,
            EccLevel.H => 3,
            _ => 1,
        };

    private static int SymbolSize(int version) => 4 * version + 17;

    private static int NumRawDataModules(int version)
    {
        var v = (long)version;
        var result = (16 * v + 128) * v + 64;
        if (version >= 2)
        {
            var numAlign = v / 7 + 2;
            result -= (25 * numAlign - 10) * numAlign - 55;
            if (version >= 7)
            {
                result -= 36;
            }
        }

        return (int)result;
    }

    private static int NumDataCodewords(int version, EccLevel ecc)
    {
        var row = RowIndex(ecc);
        var rawCodewords = NumRawDataModules(version) / 8;
        var eccCodewords = NumBlocks[row, version] * EccCodewordsPerBlock[row, version];
        return rawCodewords - eccCodewords;
    }

    private static int NumRemainderBits(int version) => NumRawDataModules(version) % 8;

    private static byte[] BuildGenerator(int count)
    {
        var generator = new byte[] { 1 };
        for (var i = 0; i < count; i++)
        {
            var alpha = Gf256Math.Power(2, i);
            var next = new byte[generator.Length + 1];
            for (var j = 0; j < generator.Length; j++)
            {
                next[j] ^= generator[j];
                next[j + 1] ^= Gf256Math.Multiply(generator[j], alpha);
            }

            generator = next;
        }

        return generator;
    }

    private static byte[] RsEncode(byte[] data, byte[] generator)
    {
        var count = generator.Length - 1;
        var rem = new byte[count];
        foreach (var value in data)
        {
            var feedback = (byte)(value ^ rem[0]);
            Array.Copy(rem, 1, rem, 0, count - 1);
            rem[count - 1] = 0;
            if (feedback == 0)
            {
                continue;
            }

            for (var i = 0; i < count; i++)
            {
                rem[i] ^= Gf256Math.Multiply(generator[i + 1], feedback);
            }
        }

        return rem;
    }

    private static EncodingMode SelectMode(string input)
    {
        if (input.All(char.IsDigit))
        {
            return EncodingMode.Numeric;
        }

        return input.All(c => AlphanumChars.Contains(c, StringComparison.Ordinal))
            ? EncodingMode.Alphanumeric
            : EncodingMode.Byte;
    }

    private static uint ModeIndicator(EncodingMode mode) =>
        mode switch
        {
            EncodingMode.Numeric => 0b0001u,
            EncodingMode.Alphanumeric => 0b0010u,
            EncodingMode.Byte => 0b0100u,
            _ => 0u,
        };

    private static int CharCountBits(EncodingMode mode, int version) =>
        mode switch
        {
            EncodingMode.Numeric => version <= 9 ? 10 : version <= 26 ? 12 : 14,
            EncodingMode.Alphanumeric => version <= 9 ? 9 : version <= 26 ? 11 : 13,
            EncodingMode.Byte => version <= 9 ? 8 : 16,
            _ => 0,
        };

    private static void EncodeNumeric(string input, BitWriter writer)
    {
        var i = 0;
        while (i + 2 < input.Length)
        {
            var value = (uint)((input[i] - '0') * 100 + (input[i + 1] - '0') * 10 + input[i + 2] - '0');
            writer.Write(value, 10);
            i += 3;
        }

        if (i + 1 < input.Length)
        {
            writer.Write((uint)((input[i] - '0') * 10 + input[i + 1] - '0'), 7);
            i += 2;
        }

        if (i < input.Length)
        {
            writer.Write((uint)(input[i] - '0'), 4);
        }
    }

    private static void EncodeAlphanumeric(string input, BitWriter writer)
    {
        var i = 0;
        while (i + 1 < input.Length)
        {
            var value = (uint)(AlphanumChars.IndexOf(input[i], StringComparison.Ordinal) * 45 +
                               AlphanumChars.IndexOf(input[i + 1], StringComparison.Ordinal));
            writer.Write(value, 11);
            i += 2;
        }

        if (i < input.Length)
        {
            writer.Write((uint)AlphanumChars.IndexOf(input[i], StringComparison.Ordinal), 6);
        }
    }

    private static void EncodeByteMode(string input, BitWriter writer)
    {
        foreach (var value in Encoding.UTF8.GetBytes(input))
        {
            writer.Write(value, 8);
        }
    }

    private static byte[] BuildDataCodewords(string input, int version, EccLevel ecc)
    {
        var mode = SelectMode(input);
        var capacity = NumDataCodewords(version, ecc);
        var writer = new BitWriter();

        writer.Write(ModeIndicator(mode), 4);
        var charCount = mode == EncodingMode.Byte
            ? Encoding.UTF8.GetByteCount(input)
            : input.Length;
        writer.Write((uint)charCount, CharCountBits(mode, version));

        switch (mode)
        {
            case EncodingMode.Numeric:
                EncodeNumeric(input, writer);
                break;
            case EncodingMode.Alphanumeric:
                EncodeAlphanumeric(input, writer);
                break;
            case EncodingMode.Byte:
                EncodeByteMode(input, writer);
                break;
        }

        var available = capacity * 8;
        var terminatorLength = Math.Min(4, available - writer.BitLength);
        if (terminatorLength > 0)
        {
            writer.Write(0, terminatorLength);
        }

        var remainder = writer.BitLength % 8;
        if (remainder != 0)
        {
            writer.Write(0, 8 - remainder);
        }

        var bytes = writer.ToBytes().ToList();
        var pad = (byte)0xEC;
        while (bytes.Count < capacity)
        {
            bytes.Add(pad);
            pad = pad == 0xEC ? (byte)0x11 : (byte)0xEC;
        }

        return bytes.ToArray();
    }

    private static Block[] ComputeBlocks(byte[] data, int version, EccLevel ecc)
    {
        var row = RowIndex(ecc);
        var totalBlocks = NumBlocks[row, version];
        var eccLength = EccCodewordsPerBlock[row, version];
        var totalData = NumDataCodewords(version, ecc);
        var shortLength = totalData / totalBlocks;
        var longBlockCount = totalData % totalBlocks;
        var generator = BuildGenerator(eccLength);
        var blocks = new Block[totalBlocks];
        var offset = 0;

        for (var i = 0; i < totalBlocks; i++)
        {
            var length = i < totalBlocks - longBlockCount ? shortLength : shortLength + 1;
            var blockData = new byte[length];
            Array.Copy(data, offset, blockData, 0, length);
            blocks[i] = new Block(blockData, RsEncode(blockData, generator));
            offset += length;
        }

        return blocks;
    }

    private static byte[] InterleaveBlocks(Block[] blocks)
    {
        var output = new List<byte>();
        var maxDataLength = blocks.Max(block => block.Data.Length);
        for (var i = 0; i < maxDataLength; i++)
        {
            foreach (var block in blocks)
            {
                if (i < block.Data.Length)
                {
                    output.Add(block.Data[i]);
                }
            }
        }

        var eccLength = blocks[0].Ecc.Length;
        for (var i = 0; i < eccLength; i++)
        {
            foreach (var block in blocks)
            {
                output.Add(block.Ecc[i]);
            }
        }

        return output.ToArray();
    }

    private static void PlaceFinder(WorkGrid grid, int top, int left)
    {
        for (var dr = 0; dr <= 6; dr++)
        {
            for (var dc = 0; dc <= 6; dc++)
            {
                var onBorder = dr == 0 || dr == 6 || dc == 0 || dc == 6;
                var inCore = dr >= 2 && dr <= 4 && dc >= 2 && dc <= 4;
                grid.Set(top + dr, left + dc, onBorder || inCore, reserve: true);
            }
        }
    }

    private static void PlaceAlignment(WorkGrid grid, int row, int col)
    {
        for (var dr = -2; dr <= 2; dr++)
        {
            for (var dc = -2; dc <= 2; dc++)
            {
                var onBorder = Math.Abs(dr) == 2 || Math.Abs(dc) == 2;
                var isCenter = dr == 0 && dc == 0;
                grid.Set(row + dr, col + dc, onBorder || isCenter, reserve: true);
            }
        }
    }

    private static void PlaceAllAlignments(WorkGrid grid, int version)
    {
        foreach (var rowByte in AlignmentPositions[version - 1])
        {
            foreach (var colByte in AlignmentPositions[version - 1])
            {
                var row = (int)rowByte;
                var col = (int)colByte;
                if (!grid.Reserved[row, col])
                {
                    PlaceAlignment(grid, row, col);
                }
            }
        }
    }

    private static void PlaceTiming(WorkGrid grid)
    {
        var size = grid.Size;
        for (var col = 8; col <= size - 9; col++)
        {
            grid.Set(6, col, col % 2 == 0, reserve: true);
        }

        for (var row = 8; row <= size - 9; row++)
        {
            grid.Set(row, 6, row % 2 == 0, reserve: true);
        }
    }

    private static void ReserveFormatInfo(WorkGrid grid)
    {
        var size = grid.Size;
        for (var col = 0; col <= 8; col++)
        {
            if (col != 6)
            {
                grid.Reserve(8, col);
            }
        }

        for (var row = 0; row <= 8; row++)
        {
            if (row != 6)
            {
                grid.Reserve(row, 8);
            }
        }

        for (var row = size - 7; row <= size - 1; row++)
        {
            grid.Reserve(row, 8);
        }

        for (var col = size - 8; col <= size - 1; col++)
        {
            grid.Reserve(8, col);
        }
    }

    private static void PlaceDarkModule(WorkGrid grid, int version) =>
        grid.Set(4 * version + 9, 8, true, reserve: true);

    private static uint ComputeFormatBits(EccLevel ecc, int mask)
    {
        var data = (uint)((Indicator(ecc) << 3) | mask);
        var rem = data << 10;
        for (var i = 14; i >= 10; i--)
        {
            if (((rem >> i) & 1u) == 1u)
            {
                rem ^= 0x537u << (i - 10);
            }
        }

        return ((data << 10) | (rem & 0x3FFu)) ^ 0x5412u;
    }

    private static void WriteFormatInfo(WorkGrid grid, uint format)
    {
        var size = grid.Size;
        for (var i = 0; i <= 5; i++)
        {
            grid.Modules[8, i] = ((format >> (14 - i)) & 1u) == 1u;
        }

        grid.Modules[8, 7] = ((format >> 8) & 1u) == 1u;
        grid.Modules[8, 8] = ((format >> 7) & 1u) == 1u;
        grid.Modules[7, 8] = ((format >> 6) & 1u) == 1u;

        for (var i = 0; i <= 5; i++)
        {
            grid.Modules[i, 8] = ((format >> i) & 1u) == 1u;
        }

        for (var i = 0; i <= 7; i++)
        {
            grid.Modules[8, size - 1 - i] = ((format >> i) & 1u) == 1u;
        }

        for (var i = 8; i <= 14; i++)
        {
            grid.Modules[size - 15 + i, 8] = ((format >> i) & 1u) == 1u;
        }
    }

    private static void ReserveVersionInfo(WorkGrid grid, int version)
    {
        if (version < 7)
        {
            return;
        }

        var size = grid.Size;
        for (var row = 0; row <= 5; row++)
        {
            for (var dc = 0; dc <= 2; dc++)
            {
                grid.Reserve(row, size - 11 + dc);
            }
        }

        for (var dr = 0; dr <= 2; dr++)
        {
            for (var col = 0; col <= 5; col++)
            {
                grid.Reserve(size - 11 + dr, col);
            }
        }
    }

    private static uint ComputeVersionBits(int version)
    {
        var rem = (uint)version << 12;
        for (var i = 17; i >= 12; i--)
        {
            if (((rem >> i) & 1u) == 1u)
            {
                rem ^= 0x1F25u << (i - 12);
            }
        }

        return ((uint)version << 12) | (rem & 0xFFFu);
    }

    private static void WriteVersionInfo(WorkGrid grid, int version)
    {
        if (version < 7)
        {
            return;
        }

        var size = grid.Size;
        var bits = ComputeVersionBits(version);
        for (var i = 0; i <= 17; i++)
        {
            var dark = ((bits >> i) & 1u) == 1u;
            var a = 5 - i / 3;
            var b = size - 9 - i % 3;
            grid.Modules[a, b] = dark;
            grid.Modules[b, a] = dark;
        }
    }

    private static WorkGrid BuildGrid(int version)
    {
        var size = SymbolSize(version);
        var grid = new WorkGrid(size);

        PlaceFinder(grid, 0, 0);
        PlaceFinder(grid, 0, size - 7);
        PlaceFinder(grid, size - 7, 0);

        for (var i = 0; i <= 7; i++)
        {
            grid.Set(7, i, false, reserve: true);
            grid.Set(i, 7, false, reserve: true);
            grid.Set(7, size - 1 - i, false, reserve: true);
            grid.Set(i, size - 8, false, reserve: true);
            grid.Set(size - 8, i, false, reserve: true);
            grid.Set(size - 1 - i, 7, false, reserve: true);
        }

        PlaceTiming(grid);
        PlaceAllAlignments(grid, version);
        ReserveFormatInfo(grid);
        ReserveVersionInfo(grid, version);
        PlaceDarkModule(grid, version);
        return grid;
    }

    private static void PlaceBits(WorkGrid grid, byte[] codewords, int version)
    {
        var size = grid.Size;
        var bits = new List<bool>();
        foreach (var codeword in codewords)
        {
            for (var bit = 7; bit >= 0; bit--)
            {
                bits.Add(((codeword >> bit) & 1) == 1);
            }
        }

        for (var i = 0; i < NumRemainderBits(version); i++)
        {
            bits.Add(false);
        }

        var bitIndex = 0;
        var goUp = true;
        var col = size - 1;
        var running = true;

        while (running)
        {
            for (var vi = 0; vi < size; vi++)
            {
                var row = goUp ? size - 1 - vi : vi;
                for (var dc = 0; dc <= 1; dc++)
                {
                    var c = col - dc;
                    if (c >= 0 && c != 6 && !grid.Reserved[row, c])
                    {
                        if (bitIndex < bits.Count)
                        {
                            grid.Modules[row, c] = bits[bitIndex];
                        }

                        bitIndex++;
                    }
                }
            }

            goUp = !goUp;
            if (col < 2)
            {
                running = false;
            }
            else
            {
                col -= 2;
                if (col == 6)
                {
                    col = 5;
                }
            }
        }
    }

    private static bool MaskCondition(int mask, int row, int col) =>
        mask switch
        {
            0 => (row + col) % 2 == 0,
            1 => row % 2 == 0,
            2 => col % 3 == 0,
            3 => (row + col) % 3 == 0,
            4 => (row / 2 + col / 3) % 2 == 0,
            5 => (row * col) % 2 + (row * col) % 3 == 0,
            6 => ((row * col) % 2 + (row * col) % 3) % 2 == 0,
            7 => ((row + col) % 2 + (row * col) % 3) % 2 == 0,
            _ => false,
        };

    private static bool[,] ApplyMask(bool[,] modules, bool[,] reserved, int size, int mask)
    {
        var result = (bool[,])modules.Clone();
        for (var row = 0; row < size; row++)
        {
            for (var col = 0; col < size; col++)
            {
                if (!reserved[row, col])
                {
                    result[row, col] = modules[row, col] != MaskCondition(mask, row, col);
                }
            }
        }

        return result;
    }

    private static uint ComputePenalty(bool[,] modules, int size)
    {
        var penalty = 0u;

        for (var axis = 0; axis < size; axis++)
        {
            for (var horizontal = 0; horizontal < 2; horizontal++)
            {
                var run = 1u;
                var previous = horizontal == 1 ? modules[axis, 0] : modules[0, axis];
                for (var i = 1; i < size; i++)
                {
                    var current = horizontal == 1 ? modules[axis, i] : modules[i, axis];
                    if (current == previous)
                    {
                        run++;
                    }
                    else
                    {
                        if (run >= 5)
                        {
                            penalty += run - 2;
                        }

                        run = 1;
                        previous = current;
                    }
                }

                if (run >= 5)
                {
                    penalty += run - 2;
                }
            }
        }

        for (var row = 0; row < size - 1; row++)
        {
            for (var col = 0; col < size - 1; col++)
            {
                var dark = modules[row, col];
                if (dark == modules[row, col + 1] &&
                    dark == modules[row + 1, col] &&
                    dark == modules[row + 1, col + 1])
                {
                    penalty += 3;
                }
            }
        }

        int[] pattern1 = [1, 0, 1, 1, 1, 0, 1, 0, 0, 0, 0];
        int[] pattern2 = [0, 0, 0, 0, 1, 0, 1, 1, 1, 0, 1];
        for (var axis = 0; axis < size; axis++)
        {
            for (var offset = 0; offset < size - 11; offset++)
            {
                var rowPattern1 = true;
                var rowPattern2 = true;
                var colPattern1 = true;
                var colPattern2 = true;
                for (var k = 0; k <= 10; k++)
                {
                    var rowBit = modules[axis, offset + k] ? 1 : 0;
                    var colBit = modules[offset + k, axis] ? 1 : 0;
                    if (rowBit != pattern1[k])
                    {
                        rowPattern1 = false;
                    }

                    if (rowBit != pattern2[k])
                    {
                        rowPattern2 = false;
                    }

                    if (colBit != pattern1[k])
                    {
                        colPattern1 = false;
                    }

                    if (colBit != pattern2[k])
                    {
                        colPattern2 = false;
                    }
                }

                if (rowPattern1)
                {
                    penalty += 40;
                }

                if (rowPattern2)
                {
                    penalty += 40;
                }

                if (colPattern1)
                {
                    penalty += 40;
                }

                if (colPattern2)
                {
                    penalty += 40;
                }
            }
        }

        var darkCount = 0u;
        for (var row = 0; row < size; row++)
        {
            for (var col = 0; col < size; col++)
            {
                if (modules[row, col])
                {
                    darkCount++;
                }
            }
        }

        var ratio = darkCount / (double)(size * size) * 100.0;
        var previousFive = (uint)Math.Floor(ratio / 5.0) * 5;
        var below = previousFive > 50 ? previousFive - 50 : 50 - previousFive;
        var aboveBase = previousFive + 5;
        var above = aboveBase > 50 ? aboveBase - 50 : 50 - aboveBase;
        penalty += Math.Min(below, above) / 5 * 10;

        return penalty;
    }

    private static int SelectVersion(string input, EccLevel ecc)
    {
        var mode = SelectMode(input);
        var byteLength = (uint)Encoding.UTF8.GetByteCount(input);

        for (var version = 1; version <= 40; version++)
        {
            var capacity = NumDataCodewords(version, ecc);
            uint dataBits = mode switch
            {
                EncodingMode.Byte => byteLength * 8,
                EncodingMode.Numeric => ((uint)input.Length * 10 + 2) / 3,
                EncodingMode.Alphanumeric => ((uint)input.Length * 11 + 1) / 2,
                _ => 0,
            };

            var bitsNeeded = (uint)(4 + CharCountBits(mode, version)) + dataBits;
            var codewordsNeeded = (bitsNeeded + 7) / 8;
            if (codewordsNeeded <= capacity)
            {
                return version;
            }
        }

        throw new InputTooLongException($"Input ({input.Length} chars, ECC={ecc}) exceeds version-40 capacity.");
    }

    private static void CopyGrid(bool[,] modules, bool[,] reserved, WorkGrid target)
    {
        var size = target.Size;
        for (var row = 0; row < size; row++)
        {
            for (var col = 0; col < size; col++)
            {
                target.Modules[row, col] = modules[row, col];
                target.Reserved[row, col] = reserved[row, col];
            }
        }
    }

    private static ModuleGrid ToModuleGrid(bool[,] modules)
    {
        var size = modules.GetLength(0);
        var grid = ModuleGrid.Create(size, size, ModuleShape.Square);
        for (var row = 0; row < size; row++)
        {
            for (var col = 0; col < size; col++)
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
