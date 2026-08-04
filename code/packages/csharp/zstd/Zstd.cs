using System.Buffers.Binary;
using System.Numerics;
using CodingAdventures.Lzss;

namespace CodingAdventures.Zstd;

/// <summary>Educational Zstandard encoder and decoder using raw literals and predefined FSE tables.</summary>
public static class Zstd
{
    /// <summary>The RFC 8878 frame magic number.</summary>
    public const uint Magic = 0xFD2FB528;

    /// <summary>The maximum payload size of one Zstandard block.</summary>
    public const int MaxBlockSize = 128 * 1024;

    /// <summary>The maximum decompressed frame size accepted by this implementation.</summary>
    public const int MaxOutputSize = 256 * 1024 * 1024;

    private const int LiteralLengthAccuracyLog = 6;
    private const int MatchLengthAccuracyLog = 6;
    private const int OffsetAccuracyLog = 5;

    private static readonly CodeRange[] LiteralLengthCodes =
    [
        new(0, 0), new(1, 0), new(2, 0), new(3, 0), new(4, 0), new(5, 0),
        new(6, 0), new(7, 0), new(8, 0), new(9, 0), new(10, 0), new(11, 0),
        new(12, 0), new(13, 0), new(14, 0), new(15, 0),
        new(16, 1), new(18, 1), new(20, 1), new(22, 1),
        new(24, 2), new(28, 2), new(32, 3), new(40, 3),
        new(48, 4), new(64, 6), new(128, 7), new(256, 8),
        new(512, 9), new(1024, 10), new(2048, 11), new(4096, 12),
        new(8192, 13), new(16384, 14), new(32768, 15), new(65536, 16),
    ];

    private static readonly CodeRange[] MatchLengthCodes =
    [
        new(3, 0), new(4, 0), new(5, 0), new(6, 0), new(7, 0), new(8, 0),
        new(9, 0), new(10, 0), new(11, 0), new(12, 0), new(13, 0), new(14, 0),
        new(15, 0), new(16, 0), new(17, 0), new(18, 0), new(19, 0), new(20, 0),
        new(21, 0), new(22, 0), new(23, 0), new(24, 0), new(25, 0), new(26, 0),
        new(27, 0), new(28, 0), new(29, 0), new(30, 0), new(31, 0), new(32, 0),
        new(33, 0), new(34, 0), new(35, 1), new(37, 1), new(39, 1), new(41, 1),
        new(43, 2), new(47, 2), new(51, 3), new(59, 3), new(67, 4), new(83, 4),
        new(99, 5), new(131, 7), new(259, 8), new(515, 9), new(1027, 10),
        new(2051, 11), new(4099, 12), new(8195, 13), new(16387, 14),
        new(32771, 15), new(65539, 16),
    ];

    private static readonly int[] LiteralLengthNorm =
    [
        4, 3, 2, 2, 2, 2, 2, 2, 2, 2, 2, 2, 2, 1, 1, 1,
        2, 2, 2, 2, 2, 2, 2, 2, 2, 3, 2, 1, 1, 1, 1, 1,
        -1, -1, -1, -1,
    ];

    private static readonly int[] MatchLengthNorm =
    [
        1, 4, 3, 2, 2, 2, 2, 2, 2, 1, 1, 1, 1, 1, 1, 1,
        1, 1, 1, 1, 1, 1, 1, 1, 1, 1, 1, 1, 1, 1, 1, 1,
        1, 1, 1, 1, 1, 1, 1, 1, 1, 1, 1, 1, 1, 1, -1, -1,
        -1, -1, -1, -1, -1,
    ];

    private static readonly int[] OffsetNorm =
    [
        1, 1, 1, 1, 1, 1, 2, 2, 2, 1, 1, 1, 1, 1, 1, 1,
        1, 1, 1, 1, 1, 1, 1, 1, -1, -1, -1, -1, -1,
    ];

    private readonly record struct CodeRange(int Baseline, int ExtraBits);
    private readonly record struct Sequence(int LiteralLength, int MatchLength, int Offset);
    private readonly record struct DecodeEntry(int Symbol, int Bits, int Baseline);
    private readonly record struct EncodeEntry(int DeltaBits, int DeltaFindState);

    private sealed class ReverseBitWriter
    {
        private readonly List<byte> bytes = [];
        private ulong register;
        private int bitCount;

        internal void AddBits(ulong value, int count)
        {
            if (count == 0)
            {
                return;
            }

            var mask = (1UL << count) - 1;
            register |= (value & mask) << bitCount;
            bitCount += count;
            while (bitCount >= 8)
            {
                bytes.Add((byte)register);
                register >>= 8;
                bitCount -= 8;
            }
        }

        internal byte[] Finish()
        {
            bytes.Add((byte)((register & 0xFF) | (1UL << bitCount)));
            register = 0;
            bitCount = 0;
            return [.. bytes];
        }
    }

    private sealed class ReverseBitReader
    {
        private readonly byte[] bytes;
        private ulong register;
        private int bitCount;
        private int position;

        internal ReverseBitReader(ReadOnlySpan<byte> data)
        {
            if (data.IsEmpty)
            {
                throw new InvalidDataException("empty bitstream");
            }

            bytes = data.ToArray();
            var last = bytes[^1];
            if (last == 0)
            {
                throw new InvalidDataException("bitstream last byte has no sentinel");
            }

            var sentinelPosition = BitOperations.Log2(last);
            bitCount = sentinelPosition;
            var mask = bitCount == 0 ? 0UL : (1UL << bitCount) - 1;
            register = bitCount == 0 ? 0UL : ((ulong)last & mask) << (64 - bitCount);
            position = bytes.Length - 1;
            Reload();
        }

        internal int ReadBits(int count)
        {
            if (count == 0)
            {
                return 0;
            }

            if (count < 0 || count > 32 || count > bitCount)
            {
                throw new InvalidDataException("bitstream is truncated");
            }

            var value = (int)(register >> (64 - count));
            register <<= count;
            bitCount -= count;
            if (bitCount < 24)
            {
                Reload();
            }

            return value;
        }

        private void Reload()
        {
            while (bitCount <= 56 && position > 0)
            {
                position--;
                register |= (ulong)bytes[position] << (64 - bitCount - 8);
                bitCount += 8;
            }
        }
    }

    /// <summary>Compresses bytes into a deterministic educational Zstandard frame.</summary>
    public static byte[] Compress(byte[] data)
    {
        ArgumentNullException.ThrowIfNull(data);
        using var output = new MemoryStream();
        Span<byte> header = stackalloc byte[8];
        BinaryPrimitives.WriteUInt32LittleEndian(header, Magic);
        output.Write(header[..4]);
        output.WriteByte(0xE0);
        BinaryPrimitives.WriteUInt64LittleEndian(header, (ulong)data.Length);
        output.Write(header);

        if (data.Length == 0)
        {
            WriteBlockHeader(output, 0, 0, true);
            return output.ToArray();
        }

        for (var offset = 0; offset < data.Length; offset += MaxBlockSize)
        {
            var length = Math.Min(MaxBlockSize, data.Length - offset);
            var block = data.AsSpan(offset, length);
            var last = offset + length == data.Length;

            if (AllEqual(block))
            {
                WriteBlockHeader(output, length, 1, last);
                output.WriteByte(block[0]);
                continue;
            }

            var compressed = CompressBlock(block);
            if (compressed is not null)
            {
                WriteBlockHeader(output, compressed.Length, 2, last);
                output.Write(compressed);
            }
            else
            {
                WriteBlockHeader(output, length, 0, last);
                output.Write(block);
            }
        }

        return output.ToArray();
    }

    /// <summary>Decompresses one educational Zstandard frame.</summary>
    public static byte[] Decompress(byte[] data)
    {
        ArgumentNullException.ThrowIfNull(data);
        if (data.Length < 5)
        {
            throw new InvalidDataException("frame is too short");
        }

        if (BinaryPrimitives.ReadUInt32LittleEndian(data) != Magic)
        {
            throw new InvalidDataException("bad Zstandard magic number");
        }

        var position = 4;
        var descriptor = data[position++];

        // Frame_Header_Descriptor bit layout (RFC 8878 §3.1.1.1):
        //   bit 7-6: Frame_Content_Size_Flag
        //   bit 5:   Single_Segment_Flag
        //   bit 4:   Unused_bit (decoders MUST ignore it, not reject it)
        //   bit 3:   Reserved (must be 0)
        //   bit 2:   Content_Checksum_Flag
        //   bit 1-0: Dictionary_ID_Flag
        // An earlier revision of this decoder read the checksum flag from
        // bit 4 and rejected any frame with bit 2 set as "reserved" — both
        // backwards from the RFC. Verified empirically against the real
        // `zstd` CLI: `zstd -c` (checksum on by default) emits FHD 0x64;
        // `zstd -c --no-check` emits FHD 0x60 — the differing bit is bit 2.
        // See lessons.md Lesson 95.
        if ((descriptor & 0x08) != 0)
        {
            throw new InvalidDataException("reserved frame-header bits are set");
        }

        var contentSizeFlag = descriptor >> 6;
        var singleSegment = (descriptor & 0x20) != 0;
        var checksum = (descriptor & 0x04) != 0;
        var dictionaryFlag = descriptor & 3;

        if (!singleSegment)
        {
            RequireAvailable(data, position, 1, "window descriptor");
            position++;
        }

        var dictionaryBytes = dictionaryFlag switch { 0 => 0, 1 => 1, 2 => 2, _ => 4 };
        RequireAvailable(data, position, dictionaryBytes, "dictionary id");
        position += dictionaryBytes;

        var contentSizeBytes = contentSizeFlag switch
        {
            0 => singleSegment ? 1 : 0,
            1 => 2,
            2 => 4,
            _ => 8,
        };
        RequireAvailable(data, position, contentSizeBytes, "frame content size");
        position += contentSizeBytes;

        var output = new List<byte>();
        var last = false;
        while (!last)
        {
            RequireAvailable(data, position, 3, "block header");
            var blockHeader = data[position] | (data[position + 1] << 8) | (data[position + 2] << 16);
            position += 3;
            last = (blockHeader & 1) != 0;
            var blockType = (blockHeader >> 1) & 3;
            var blockSize = blockHeader >> 3;
            if (blockSize > MaxBlockSize)
            {
                throw new InvalidDataException("block exceeds the 128 KiB limit");
            }

            switch (blockType)
            {
                case 0:
                    RequireAvailable(data, position, blockSize, "raw block");
                    EnsureOutputLimit(output.Count, blockSize);
                    output.AddRange(data.AsSpan(position, blockSize).ToArray());
                    position += blockSize;
                    break;
                case 1:
                    RequireAvailable(data, position, 1, "RLE block");
                    EnsureOutputLimit(output.Count, blockSize);
                    var value = data[position++];
                    for (var index = 0; index < blockSize; index++)
                    {
                        output.Add(value);
                    }

                    break;
                case 2:
                    RequireAvailable(data, position, blockSize, "compressed block");
                    DecompressBlock(data.AsSpan(position, blockSize), output);
                    position += blockSize;
                    break;
                default:
                    throw new InvalidDataException("reserved block type 3");
            }
        }

        if (checksum)
        {
            RequireAvailable(data, position, 4, "content checksum");
            position += 4;
        }

        if (position != data.Length)
        {
            throw new InvalidDataException("trailing bytes after Zstandard frame");
        }

        return [.. output];
    }

    private static byte[]? CompressBlock(ReadOnlySpan<byte> block)
    {
        var tokens = Lzss.Lzss.Encode(block.ToArray(), windowSize: Lzss.Lzss.DefaultWindowSize, maxMatch: 255, minMatch: 3);
        var literals = new List<byte>();
        var sequences = new List<Sequence>();
        var literalRun = 0;
        foreach (var token in tokens)
        {
            switch (token)
            {
                case LzssLiteral literal:
                    literals.Add(literal.Byte);
                    literalRun++;
                    break;
                case LzssMatch match:
                    sequences.Add(new Sequence(literalRun, match.Length, match.Offset));
                    literalRun = 0;
                    break;
            }
        }

        if (sequences.Count == 0)
        {
            return null;
        }

        using var output = new MemoryStream();
        output.Write(EncodeLiterals(literals));
        output.Write(EncodeSequenceCount(sequences.Count));
        output.WriteByte(0);
        output.Write(EncodeSequences(sequences));
        var result = output.ToArray();
        return result.Length < block.Length ? result : null;
    }

    private static void DecompressBlock(ReadOnlySpan<byte> data, List<byte> output)
    {
        var (literals, literalBytes) = DecodeLiterals(data);
        var position = literalBytes;
        if (position >= data.Length)
        {
            EnsureOutputLimit(output.Count, literals.Length);
            output.AddRange(literals);
            return;
        }

        var (sequenceCount, countBytes) = DecodeSequenceCount(data[position..]);
        position += countBytes;
        if (sequenceCount == 0)
        {
            EnsureOutputLimit(output.Count, literals.Length);
            output.AddRange(literals);
            return;
        }

        RequireAvailable(data, position, 1, "symbol compression modes");
        var modes = data[position++];
        if ((modes & 0xFC) != 0)
        {
            throw new InvalidDataException("only predefined FSE modes are supported");
        }

        var reader = new ReverseBitReader(data[position..]);
        var literalTable = BuildDecodeTable(LiteralLengthNorm, LiteralLengthAccuracyLog);
        var matchTable = BuildDecodeTable(MatchLengthNorm, MatchLengthAccuracyLog);
        var offsetTable = BuildDecodeTable(OffsetNorm, OffsetAccuracyLog);

        // Initial states are read LL, OF, ML — RFC 8878 §3.1.1.3.2.1.2. This
        // is a DIFFERENT order from the per-sequence update order below
        // (LL, ML, OF); the RFC is genuinely asymmetric here.
        var literalState = reader.ReadBits(LiteralLengthAccuracyLog);
        var offsetState = reader.ReadBits(OffsetAccuracyLog);
        var matchState = reader.ReadBits(MatchLengthAccuracyLog);
        var literalPosition = 0;

        for (var sequenceIndex = 0; sequenceIndex < sequenceCount; sequenceIndex++)
        {
            // Step 1 — peek all three symbols from the CURRENT states. This
            // is a bare table lookup (the FSE state IS the decode-table
            // index); it consumes zero bits. Only the state UPDATE in step 3
            // reads bits.
            if ((uint)literalState >= literalTable.Length
                || (uint)offsetState >= offsetTable.Length
                || (uint)matchState >= matchTable.Length)
            {
                throw new InvalidDataException("invalid FSE decoder state");
            }

            var literalEntry = literalTable[literalState];
            var offsetEntry = offsetTable[offsetState];
            var matchEntry = matchTable[matchState];
            var literalCode = literalEntry.Symbol;
            var offsetCode = offsetEntry.Symbol;
            var matchCode = matchEntry.Symbol;

            if ((uint)literalCode >= LiteralLengthCodes.Length || (uint)matchCode >= MatchLengthCodes.Length)
            {
                throw new InvalidDataException("invalid sequence code");
            }

            // Step 2 — read extra value bits, in order OF, ML, LL.
            var rawOffset = (1 << offsetCode) | reader.ReadBits(offsetCode);
            var matchLength = MatchLengthCodes[matchCode].Baseline
                + reader.ReadBits(MatchLengthCodes[matchCode].ExtraBits);
            var literalLength = LiteralLengthCodes[literalCode].Baseline
                + reader.ReadBits(LiteralLengthCodes[literalCode].ExtraBits);
            var matchOffset = rawOffset - 3;

            // Step 3 — update FSE states (consumes bits), in order LL, ML,
            // OF, preparing the states the NEXT sequence's peek will use —
            // but only if this is not the last sequence. There is no "next"
            // sequence to prepare a state for after the last one, and a
            // conformant encoder never wrote any bits for that non-existent
            // transition (see FseInitState in EncodeSequences).
            if (sequenceIndex != sequenceCount - 1)
            {
                literalState = literalEntry.Baseline + reader.ReadBits(literalEntry.Bits);
                matchState = matchEntry.Baseline + reader.ReadBits(matchEntry.Bits);
                offsetState = offsetEntry.Baseline + reader.ReadBits(offsetEntry.Bits);
            }

            if (literalLength < 0 || literalPosition + literalLength > literals.Length)
            {
                throw new InvalidDataException("literal run exceeds the literals section");
            }

            EnsureOutputLimit(output.Count, literalLength);
            for (var index = 0; index < literalLength; index++)
            {
                output.Add(literals[literalPosition++]);
            }

            if (matchOffset < 1 || matchOffset > output.Count)
            {
                throw new InvalidDataException("match offset exceeds decoded output");
            }

            EnsureOutputLimit(output.Count, matchLength);
            var copyStart = output.Count - matchOffset;
            for (var index = 0; index < matchLength; index++)
            {
                output.Add(output[copyStart + index]);
            }
        }

        var remaining = literals.Length - literalPosition;
        EnsureOutputLimit(output.Count, remaining);
        for (var index = literalPosition; index < literals.Length; index++)
        {
            output.Add(literals[index]);
        }
    }

    private static byte[] EncodeLiterals(IReadOnlyCollection<byte> literals)
    {
        using var output = new MemoryStream();
        var count = literals.Count;
        if (count <= 31)
        {
            output.WriteByte((byte)(count << 3));
        }
        else if (count <= 4095)
        {
            var header = (count << 4) | 4;
            output.WriteByte((byte)header);
            output.WriteByte((byte)(header >> 8));
        }
        else
        {
            var header = (count << 4) | 12;
            output.WriteByte((byte)header);
            output.WriteByte((byte)(header >> 8));
            output.WriteByte((byte)(header >> 16));
        }

        foreach (var literal in literals)
        {
            output.WriteByte(literal);
        }

        return output.ToArray();
    }

    private static (byte[] Literals, int BytesConsumed) DecodeLiterals(ReadOnlySpan<byte> data)
    {
        RequireAvailable(data, 0, 1, "literals section");
        var first = data[0];
        if ((first & 3) != 0)
        {
            throw new InvalidDataException("only raw literals are supported");
        }

        var sizeFormat = (first >> 2) & 3;
        int count;
        int headerBytes;
        switch (sizeFormat)
        {
            case 0:
            case 2:
                count = first >> 3;
                headerBytes = 1;
                break;
            case 1:
                RequireAvailable(data, 0, 2, "literals header");
                count = (first >> 4) | (data[1] << 4);
                headerBytes = 2;
                break;
            default:
                RequireAvailable(data, 0, 3, "literals header");
                count = (first >> 4) | (data[1] << 4) | (data[2] << 12);
                headerBytes = 3;
                break;
        }

        RequireAvailable(data, headerBytes, count, "literals data");
        return (data.Slice(headerBytes, count).ToArray(), headerBytes + count);
    }

    private static byte[] EncodeSequenceCount(int count)
    {
        if (count < 128)
        {
            return [(byte)count];
        }

        if (count < 0x7F00)
        {
            return [(byte)(0x80 | (count >> 8)), (byte)count];
        }

        var remainder = count - 0x7F00;
        return [0xFF, (byte)remainder, (byte)(remainder >> 8)];
    }

    private static (int Count, int BytesConsumed) DecodeSequenceCount(ReadOnlySpan<byte> data)
    {
        RequireAvailable(data, 0, 1, "sequence count");
        var first = data[0];
        if (first < 128)
        {
            return (first, 1);
        }

        if (first < 0xFF)
        {
            RequireAvailable(data, 0, 2, "sequence count");
            return (((first & 0x7F) << 8) | data[1], 2);
        }

        RequireAvailable(data, 0, 3, "sequence count");
        return (0x7F00 + data[1] + (data[2] << 8), 3);
    }

    // Per RFC 8878 §3.1.1.3.2.1.2, verified against the reference C source
    // (ZSTD_decodeSequence / FSE_encodeSymbol / FSE_initCState2 in
    // github.com/facebook/zstd), a forward decoder processes each sequence
    // in three strictly ordered steps:
    //   1. Peek all three symbols (LL, OF, ML) from the CURRENT states —
    //      a bare table lookup that consumes zero bits.
    //   2. Read extra value bits, in order OF, ML, LL.
    //   3. Update the FSE states (consumes bits), in order LL, ML, OF — but
    //      ONLY if this is not the last sequence in the block. There is no
    //      "next" sequence to prepare a state for after the last one.
    // The very first read of all (the initial states, before sequence 1) is
    // a separate, differently-ordered step: LL, OF, ML.
    //
    // The encoder mirrors this in exact reverse, since the bitstream is
    // written backward: it processes sequences from last to first. The
    // FIRST sequence it processes (the semantically LAST real sequence) has
    // no incoming bit-consuming transition — mirroring step 3 being skipped
    // on the decode side — so its starting state must be computed directly
    // via FseInitState (zstd's FSE_initCState2), writing zero bits, rather
    // than a normal FseEncodeTransition.
    //
    // An earlier revision of this codec combined peek-and-update into one
    // step (in the wrong LL/OF/ML order), wrote extra bits in the wrong
    // LL/ML/OF order, and flushed a state transition for every sequence
    // uniformly instead of special-casing the last one. All three bugs were
    // self-cancelling in this package's own round-trip tests (encoder and
    // decoder agreed with each other) but produced output the real `zstd`
    // CLI rejected as corrupt. See lessons.md Lesson 96.
    private static byte[] EncodeSequences(IReadOnlyList<Sequence> sequences)
    {
        var (literalEntries, literalStates) = BuildEncodeTables(LiteralLengthNorm, LiteralLengthAccuracyLog);
        var (matchEntries, matchStates) = BuildEncodeTables(MatchLengthNorm, MatchLengthAccuracyLog);
        var (offsetEntries, offsetStates) = BuildEncodeTables(OffsetNorm, OffsetAccuracyLog);
        var literalSize = 1 << LiteralLengthAccuracyLog;
        var matchSize = 1 << MatchLengthAccuracyLog;
        var offsetSize = 1 << OffsetAccuracyLog;
        var literalState = 0;
        var matchState = 0;
        var offsetState = 0;
        var writer = new ReverseBitWriter();
        var first = true;

        for (var index = sequences.Count - 1; index >= 0; index--)
        {
            var sequence = sequences[index];
            var literalCode = ValueToCode(sequence.LiteralLength, LiteralLengthCodes);
            var matchCode = ValueToCode(sequence.MatchLength, MatchLengthCodes);
            var rawOffset = sequence.Offset + 3;
            var offsetCode = BitOperations.Log2((uint)rawOffset);
            var offsetExtra = rawOffset - (1 << offsetCode);
            var matchExtra = sequence.MatchLength - MatchLengthCodes[matchCode].Baseline;
            var literalExtra = sequence.LiteralLength - LiteralLengthCodes[literalCode].Baseline;

            if (first)
            {
                // Last real sequence: no incoming transition to flush. The
                // starting state is derived directly from the symbol, with
                // zero bits written (mirrors FSE_initCState2).
                offsetState = FseInitState(offsetCode, offsetEntries, offsetStates);
                matchState = FseInitState(matchCode, matchEntries, matchStates);
                literalState = FseInitState(literalCode, literalEntries, literalStates);
                first = false;
            }
            else
            {
                // Transition write order OF, ML, LL — a forward decoder
                // consumes these in reverse (LL, ML, OF) as the state
                // UPDATE immediately after decoding this same sequence.
                offsetState = FseEncodeTransition(offsetState, offsetCode, offsetEntries, offsetStates, writer);
                matchState = FseEncodeTransition(matchState, matchCode, matchEntries, matchStates, writer);
                literalState = FseEncodeTransition(literalState, literalCode, literalEntries, literalStates, writer);
            }

            // Extra bits, write order LL, ML, OF — a forward decoder reads
            // these in order OF, ML, LL immediately after peeking symbols.
            writer.AddBits((uint)literalExtra, LiteralLengthCodes[literalCode].ExtraBits);
            writer.AddBits((uint)matchExtra, MatchLengthCodes[matchCode].ExtraBits);
            writer.AddBits((uint)offsetExtra, offsetCode);
        }

        // Flush the initial states (the states used to peek the FIRST real
        // sequence). A forward decoder reads these first, in order LL, OF,
        // ML. Since these are the very last bits written and the stream is
        // read backward, write them in reverse: ML, OF, LL.
        writer.AddBits((uint)(matchState - matchSize), MatchLengthAccuracyLog);
        writer.AddBits((uint)(offsetState - offsetSize), OffsetAccuracyLog);
        writer.AddBits((uint)(literalState - literalSize), LiteralLengthAccuracyLog);
        return writer.Finish();
    }

    // Real FSE table-spread algorithm, verified against the reference C
    // source (FSE_buildDTable_internal's low-probability branch in
    // fse_decompress.c): a SINGLE pass over symbols 0..maxSymbolValue in
    // ascending order, placing each symbol's full count immediately when
    // encountered. There is no "count>1 symbols first, then count==1
    // symbols" split — an earlier revision of this codec used exactly that
    // fabricated two-pass convention, which produces a different table
    // layout that is internally self-consistent (this package's own
    // encoder/decoder agreed with each other) but is rejected as corrupt by
    // the real `zstd` CLI. See lessons.md Lesson 96.
    private static DecodeEntry[] BuildDecodeTable(IReadOnlyList<int> normalized, int accuracyLog)
    {
        var size = 1 << accuracyLog;
        ValidateNormalizedCounts(normalized, size);
        var step = (size >> 1) + (size >> 3) + 3;
        var symbols = new int[size];
        var symbolNext = new int[normalized.Count];
        var high = size - 1;

        for (var symbol = 0; symbol < normalized.Count; symbol++)
        {
            if (normalized[symbol] == -1)
            {
                symbols[high] = symbol;
                if (high > 0)
                {
                    high--;
                }

                symbolNext[symbol] = 1;
            }
        }

        var position = 0;
        for (var symbol = 0; symbol < normalized.Count; symbol++)
        {
            var count = normalized[symbol];
            if (count <= 0)
            {
                continue;
            }

            symbolNext[symbol] = count;
            for (var occurrence = 0; occurrence < count; occurrence++)
            {
                symbols[position] = symbol;
                do
                {
                    position = (position + step) & (size - 1);
                }
                while (position > high);
            }
        }

        var next = (int[])symbolNext.Clone();
        var table = new DecodeEntry[size];
        for (var index = 0; index < size; index++)
        {
            var symbol = symbols[index];
            var nextState = next[symbol]++;
            var bits = accuracyLog - BitOperations.Log2((uint)nextState);
            table[index] = new DecodeEntry(symbol, bits, (nextState << bits) - size);
        }

        return table;
    }

    private static (EncodeEntry[] Entries, int[] States) BuildEncodeTables(
        IReadOnlyList<int> normalized,
        int accuracyLog)
    {
        var size = 1 << accuracyLog;
        ValidateNormalizedCounts(normalized, size);
        var cumulative = new int[normalized.Count];
        var total = 0;
        for (var symbol = 0; symbol < normalized.Count; symbol++)
        {
            cumulative[symbol] = total;
            total += normalized[symbol] == -1 ? 1 : Math.Max(normalized[symbol], 0);
        }

        var step = (size >> 1) + (size >> 3) + 3;
        var spread = new int[size];
        var high = size - 1;
        for (var symbol = 0; symbol < normalized.Count; symbol++)
        {
            if (normalized[symbol] == -1)
            {
                spread[high] = symbol;
                if (high > 0)
                {
                    high--;
                }
            }
        }

        // Single pass over symbols in ascending order — must mirror
        // BuildDecodeTable's spread exactly (see comment there).
        var position = 0;
        for (var symbol = 0; symbol < normalized.Count; symbol++)
        {
            var count = normalized[symbol];
            if (count <= 0)
            {
                continue;
            }

            for (var occurrence = 0; occurrence < count; occurrence++)
            {
                spread[position] = symbol;
                do
                {
                    position = (position + step) & (size - 1);
                }
                while (position > high);
            }
        }

        var occurrences = new int[normalized.Count];
        var states = new int[size];
        for (var index = 0; index < size; index++)
        {
            var symbol = spread[index];
            states[cumulative[symbol] + occurrences[symbol]++] = index + size;
        }

        var entries = new EncodeEntry[normalized.Count];
        for (var symbol = 0; symbol < normalized.Count; symbol++)
        {
            var count = normalized[symbol] == -1 ? 1 : Math.Max(normalized[symbol], 0);
            if (count == 0)
            {
                continue;
            }

            var maxBits = count == 1 ? accuracyLog : accuracyLog - BitOperations.Log2((uint)count);
            entries[symbol] = new EncodeEntry(
                (maxBits << 16) - (count << maxBits),
                cumulative[symbol] - count);
        }

        return (entries, states);
    }

    /// <summary>
    /// Performs a normal FSE encode transition (mirrors real zstd's
    /// <c>FSE_encodeSymbol</c>): flushes the bits needed to move from
    /// <paramref name="state"/> to the next state that will decode as
    /// <paramref name="symbol"/>, and returns that next state.
    /// </summary>
    private static int FseEncodeTransition(
        int state,
        int symbol,
        IReadOnlyList<EncodeEntry> entries,
        IReadOnlyList<int> states,
        ReverseBitWriter writer)
    {
        if ((uint)symbol >= entries.Count)
        {
            throw new InvalidDataException("FSE symbol is outside the predefined table");
        }

        var entry = entries[symbol];
        var bits = (state + entry.DeltaBits) >> 16;
        var value = state & ((1 << bits) - 1);
        writer.AddBits((uint)value, bits);
        var slot = (state >> bits) + entry.DeltaFindState;
        if ((uint)slot >= states.Count)
        {
            throw new InvalidDataException("invalid FSE encoder state");
        }

        return states[slot];
    }

    /// <summary>
    /// Computes an FSE encoder's starting state directly from a symbol,
    /// writing zero bits (mirrors real zstd's <c>FSE_initCState2</c>). Used
    /// for the first symbol processed in the reverse encode loop — the
    /// semantically LAST real sequence — which has no incoming bit-consuming
    /// transition, because a real decoder never performs a state update
    /// after decoding the final sequence in a block.
    /// </summary>
    private static int FseInitState(
        int symbol,
        IReadOnlyList<EncodeEntry> entries,
        IReadOnlyList<int> states)
    {
        if ((uint)symbol >= entries.Count)
        {
            throw new InvalidDataException("FSE symbol is outside the predefined table");
        }

        var entry = entries[symbol];
        var bits = (entry.DeltaBits + (1 << 15)) >> 16;
        var value = (bits << 16) - entry.DeltaBits;
        var slot = (value >> bits) + entry.DeltaFindState;
        if ((uint)slot >= states.Count)
        {
            throw new InvalidDataException("invalid FSE encoder state");
        }

        return states[slot];
    }

    private static int ValueToCode(int value, IReadOnlyList<CodeRange> codes)
    {
        var code = 0;
        for (var index = 0; index < codes.Count && codes[index].Baseline <= value; index++)
        {
            code = index;
        }

        return code;
    }

    private static bool AllEqual(ReadOnlySpan<byte> data)
    {
        for (var index = 1; index < data.Length; index++)
        {
            if (data[index] != data[0])
            {
                return false;
            }
        }

        return true;
    }

    private static void WriteBlockHeader(Stream output, int size, int type, bool last)
    {
        var value = (size << 3) | (type << 1) | (last ? 1 : 0);
        output.WriteByte((byte)value);
        output.WriteByte((byte)(value >> 8));
        output.WriteByte((byte)(value >> 16));
    }

    private static void RequireAvailable(ReadOnlySpan<byte> data, int position, int count, string field)
    {
        if (position < 0 || count < 0 || position > data.Length - count)
        {
            throw new InvalidDataException($"truncated {field}");
        }
    }

    private static void EnsureOutputLimit(int current, int additional)
    {
        if (additional < 0 || current > MaxOutputSize - additional)
        {
            throw new InvalidDataException($"decompressed size exceeds {MaxOutputSize} bytes");
        }
    }

    private static void ValidateNormalizedCounts(IReadOnlyList<int> normalized, int tableSize)
    {
        var total = 0;
        foreach (var count in normalized)
        {
            if (count < -1)
            {
                throw new InvalidDataException("invalid normalized FSE count");
            }

            total += count == -1 ? 1 : count;
        }

        if (total != tableSize)
        {
            throw new InvalidDataException("normalized FSE counts do not fill the table");
        }
    }
}
