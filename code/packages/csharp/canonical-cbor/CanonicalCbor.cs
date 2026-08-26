using System.Text;

namespace CodingAdventures.CanonicalCbor.CSharp;

/// <summary>A bounded, zero-production-dependency RFC 8949 section 4.2.3 codec.</summary>
public static class CanonicalCbor
{
    public const int MaxNestingDepth = 128;
    public const int MaxEncodedBytes = 1_048_576;

    private static readonly UTF8Encoding StrictUtf8 = new(false, true);

    /// <summary>Encodes one value without publishing partial bytes on failure.</summary>
    public static byte[] EncodeChecked(CborValue value)
    {
        ArgumentNullException.ThrowIfNull(value);
        Encoder encoder = new();
        encoder.WriteValue(value, 0);
        return encoder.Bytes();
    }

    /// <summary>Appends one complete encoding, leaving the stream unchanged on codec failure.</summary>
    public static void EncodeIntoChecked(CborValue value, MemoryStream destination)
    {
        ArgumentNullException.ThrowIfNull(destination);
        byte[] encoded = EncodeChecked(value);
        destination.Position = destination.Length;
        destination.Write(encoded);
    }

    /// <summary>Decodes exactly one canonical item.</summary>
    public static CborValue Decode(byte[] bytes)
    {
        ArgumentNullException.ThrowIfNull(bytes);
        Cursor cursor = new(bytes);
        CborValue value = cursor.ReadValue(0);
        if (cursor.Remaining != 0)
        {
            throw Error("trailing-bytes");
        }
        return value;
    }

    private static CborException Error(string id) => new(id);

    private sealed class Encoder
    {
        private readonly MemoryStream output = new();

        internal byte[] Bytes() => output.ToArray();

        internal void WriteValue(CborValue value, int depth)
        {
            if (depth > MaxNestingDepth)
            {
                throw Error("encode-too-deep");
            }

            switch (value)
            {
                case CborUnsigned integer:
                    WriteArgument(0, integer.Value);
                    break;
                case CborNegative integer:
                    WriteArgument(1, integer.Value);
                    break;
                case CborByteString bytes:
                    WriteArgument(2, (ulong)bytes.RawValue.Length);
                    WriteBytes(bytes.RawValue);
                    break;
                case CborText text:
                    long length = Utf8Length(text.Value);
                    EnsureFits(ArgumentSize((ulong)length) + length);
                    byte[] payload = StrictUtf8.GetBytes(text.Value);
                    WriteArgument(3, (ulong)length);
                    WriteBytes(payload);
                    break;
                case CborArray array:
                    EnsureFits(ArgumentSize((ulong)array.Values.Count) + array.Values.Count);
                    WriteArgument(4, (ulong)array.Values.Count);
                    foreach (CborValue item in array.Values)
                    {
                        WriteValue(item, depth + 1);
                    }
                    break;
                case CborMap map:
                    WriteMap(map, depth);
                    break;
                case CborTag tag:
                    WriteArgument(6, tag.Number);
                    WriteValue(tag.Value, depth + 1);
                    break;
                case CborBoolean boolean:
                    WriteByte(boolean.Value ? 0xf5 : 0xf4);
                    break;
                case CborNull:
                    WriteByte(0xf6);
                    break;
                default:
                    throw new ArgumentException("unknown CborValue implementation", nameof(value));
            }
        }

        private void WriteMap(CborMap map, int depth)
        {
            long minimumSize = ArgumentSize((ulong)map.Entries.Count) + (long)map.Entries.Count * 2;
            EnsureFits(minimumSize);
            List<EncodedEntry> entries = new(map.Entries.Count);
            long retainedKeyBytes = 0;
            foreach (CborMapEntry entry in map.Entries)
            {
                Encoder keyEncoder = new();
                keyEncoder.WriteValue(entry.Key, depth + 1);
                byte[] key = keyEncoder.Bytes();
                retainedKeyBytes += key.Length;
                long lowerBound = ArgumentSize((ulong)map.Entries.Count) + map.Entries.Count + retainedKeyBytes;
                EnsureFits(lowerBound);
                entries.Add(new EncodedEntry(key, entry.Value));
            }

            entries.Sort(static (left, right) => CompareLengthFirst(left.Key, right.Key));
            for (int index = 1; index < entries.Count; index++)
            {
                if (entries[index - 1].Key.AsSpan().SequenceEqual(entries[index].Key))
                {
                    throw Error("duplicate-map-key");
                }
            }

            WriteArgument(5, (ulong)entries.Count);
            foreach (EncodedEntry entry in entries)
            {
                WriteBytes(entry.Key);
                WriteValue(entry.Value, depth + 1);
            }
        }

        private void WriteArgument(int major, ulong argument)
        {
            int prefix = major << 5;
            if (argument <= 23)
            {
                WriteByte(prefix | (int)argument);
            }
            else if (argument <= byte.MaxValue)
            {
                WriteByte(prefix | 24);
                WriteByte((int)argument);
            }
            else if (argument <= ushort.MaxValue)
            {
                WriteByte(prefix | 25);
                WriteByte((int)(argument >> 8));
                WriteByte((int)argument);
            }
            else if (argument <= uint.MaxValue)
            {
                WriteByte(prefix | 26);
                for (int shift = 24; shift >= 0; shift -= 8)
                {
                    WriteByte((int)(argument >> shift));
                }
            }
            else
            {
                WriteByte(prefix | 27);
                for (int shift = 56; shift >= 0; shift -= 8)
                {
                    WriteByte((int)(argument >> shift));
                }
            }
        }

        private void EnsureFits(long additionalBytes)
        {
            if (additionalBytes > MaxEncodedBytes - output.Length)
            {
                throw Error("encode-too-large");
            }
        }

        private void WriteByte(int value)
        {
            if (output.Length >= MaxEncodedBytes)
            {
                throw Error("encode-too-large");
            }
            output.WriteByte((byte)value);
        }

        private void WriteBytes(ReadOnlySpan<byte> bytes)
        {
            if (bytes.Length > MaxEncodedBytes - output.Length)
            {
                throw Error("encode-too-large");
            }
            output.Write(bytes);
        }
    }

    private sealed record EncodedEntry(byte[] Key, CborValue Value);
    private readonly record struct Header(int Major, int Info, ulong Argument);

    private sealed class Cursor
    {
        private readonly byte[] bytes;
        private int position;

        internal Cursor(byte[] bytes)
        {
            this.bytes = (byte[])bytes.Clone();
        }

        internal int Remaining => bytes.Length - position;

        private int ReadByte()
        {
            if (position >= bytes.Length)
            {
                throw Error("unexpected-eof");
            }
            return bytes[position++];
        }

        private byte[] ReadBytes(int length)
        {
            if (length > Remaining)
            {
                throw Error("unexpected-eof");
            }
            byte[] result = bytes.AsSpan(position, length).ToArray();
            position += length;
            return result;
        }

        private Header ReadHeader()
        {
            int initial = ReadByte();
            int major = initial >> 5;
            int info = initial & 0x1f;
            bool enforceMinimal = major != 7;
            ulong argument;
            if (info <= 23)
            {
                argument = (ulong)info;
            }
            else if (info == 24)
            {
                argument = (ulong)ReadByte();
                EnsureMinimal(argument, 23, enforceMinimal);
            }
            else if (info == 25)
            {
                argument = ReadUnsigned(2);
                EnsureMinimal(argument, byte.MaxValue, enforceMinimal);
            }
            else if (info == 26)
            {
                argument = ReadUnsigned(4);
                EnsureMinimal(argument, ushort.MaxValue, enforceMinimal);
            }
            else if (info == 27)
            {
                argument = ReadUnsigned(8);
                EnsureMinimal(argument, uint.MaxValue, enforceMinimal);
            }
            else if (info <= 30)
            {
                throw Error("reserved");
            }
            else
            {
                throw Error("indefinite");
            }
            return new Header(major, info, argument);
        }

        private ulong ReadUnsigned(int width)
        {
            ulong value = 0;
            for (int index = 0; index < width; index++)
            {
                value = (value << 8) | (uint)ReadByte();
            }
            return value;
        }

        private static void EnsureMinimal(ulong argument, ulong previousMaximum, bool enabled)
        {
            if (enabled && argument <= previousMaximum)
            {
                throw Error("non-minimal-integer");
            }
        }

        internal CborValue ReadValue(int depth)
        {
            if (depth > MaxNestingDepth)
            {
                throw Error("too-deep");
            }
            Header header = ReadHeader();
            return header.Major switch
            {
                0 => new CborUnsigned(header.Argument),
                1 => new CborNegative(header.Argument),
                2 => new CborByteString(ReadBytes(CheckedLength(header.Argument, 1))),
                3 => new CborText(ReadText(CheckedLength(header.Argument, 1))),
                4 => ReadArray(CheckedLength(header.Argument, 1), depth),
                5 => ReadMap(CheckedLength(header.Argument, 2), depth),
                6 => new CborTag(header.Argument, ReadValue(depth + 1)),
                7 => ReadSimple(header.Info),
                _ => throw new InvalidOperationException("three-bit major type escaped range"),
            };
        }

        private int CheckedLength(ulong declared, int minimumBytesPerUnit)
        {
            int maximum = Remaining / minimumBytesPerUnit;
            if (declared > (ulong)maximum)
            {
                throw Error("length-too-large");
            }
            return (int)declared;
        }

        private string ReadText(int length)
        {
            byte[] payload = ReadBytes(length);
            try
            {
                return StrictUtf8.GetString(payload);
            }
            catch (DecoderFallbackException)
            {
                throw Error("invalid-utf8");
            }
        }

        private CborArray ReadArray(int count, int depth)
        {
            CborValue[] values = new CborValue[count];
            for (int index = 0; index < count; index++)
            {
                values[index] = ReadValue(depth + 1);
            }
            return new CborArray(values);
        }

        private CborMap ReadMap(int count, int depth)
        {
            CborMapEntry[] entries = new CborMapEntry[count];
            byte[]? previousKey = null;
            for (int index = 0; index < count; index++)
            {
                int keyStart = position;
                CborValue key = ReadValue(depth + 1);
                byte[] encodedKey = bytes.AsSpan(keyStart, position - keyStart).ToArray();
                if (previousKey is not null && CompareLengthFirst(previousKey, encodedKey) >= 0)
                {
                    throw Error("non-canonical-map-order");
                }
                previousKey = encodedKey;
                entries[index] = new CborMapEntry(key, ReadValue(depth + 1));
            }
            return new CborMap(entries);
        }

        private static CborValue ReadSimple(int info) => info switch
        {
            20 => new CborBoolean(false),
            21 => new CborBoolean(true),
            22 => CborNull.Instance,
            25 or 26 or 27 => throw Error("float-not-supported"),
            _ => throw Error("unsupported-simple"),
        };
    }

    private static int CompareLengthFirst(byte[] left, byte[] right)
    {
        int length = left.Length.CompareTo(right.Length);
        return length != 0 ? length : CompareUnsigned(left, right);
    }

    private static int CompareUnsigned(byte[] left, byte[] right)
    {
        int common = Math.Min(left.Length, right.Length);
        for (int index = 0; index < common; index++)
        {
            int comparison = left[index].CompareTo(right[index]);
            if (comparison != 0)
            {
                return comparison;
            }
        }
        return left.Length.CompareTo(right.Length);
    }

    private static int ArgumentSize(ulong argument) => argument switch
    {
        <= 23 => 1,
        <= byte.MaxValue => 2,
        <= ushort.MaxValue => 3,
        <= uint.MaxValue => 5,
        _ => 9,
    };

    private static long Utf8Length(string text)
    {
        long length = 0;
        for (int index = 0; index < text.Length; index++)
        {
            char unit = text[index];
            if (unit <= 0x7f)
            {
                length++;
            }
            else if (unit <= 0x7ff)
            {
                length += 2;
            }
            else if (char.IsHighSurrogate(unit))
            {
                length += 4;
                index++;
            }
            else
            {
                length += 3;
            }
        }
        return length;
    }
}
