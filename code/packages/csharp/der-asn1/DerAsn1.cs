using System.Collections.ObjectModel;
using System.Text;
using TlvApi = CodingAdventures.DerTlv.CSharp.DerTlv;

namespace CodingAdventures.DerAsn1.CSharp;

/// <summary>Bounded typed ASN.1 DER values over payload-blind DER framing.</summary>
public static class DerAsn1
{
    public const int DefaultMaxDepth = 32;
    public const long DefaultMaxTotalElements = 16_384;
    public const int DefaultMaxOidArcs = 128;

    public sealed class Asn1Limits
    {
        public Asn1Limits(TlvApi.Limits der, int maxDepth, long maxTotalElements, int maxOidArcs)
        {
            ArgumentNullException.ThrowIfNull(der);
            if (maxDepth < 0) throw new ArgumentOutOfRangeException(nameof(maxDepth));
            if (maxTotalElements < 0) throw new ArgumentOutOfRangeException(nameof(maxTotalElements));
            if (maxOidArcs < 0) throw new ArgumentOutOfRangeException(nameof(maxOidArcs));
            Der = der;
            MaxDepth = maxDepth;
            MaxTotalElements = maxTotalElements;
            MaxOidArcs = maxOidArcs;
        }

        public TlvApi.Limits Der { get; }
        public int MaxDepth { get; }
        public long MaxTotalElements { get; }
        public int MaxOidArcs { get; }
        public static Asn1Limits Default { get; } = new(
            TlvApi.Limits.Default, DefaultMaxDepth, DefaultMaxTotalElements, DefaultMaxOidArcs);
    }

    public enum Asn1ErrorKind
    {
        Framing,
        UnexpectedTag,
        DecoderLimitMismatch,
        DepthLimitExceeded,
        ElementLimitExceeded,
        InvalidBooleanLength,
        InvalidBooleanValue,
        EmptyInteger,
        NonMinimalInteger,
        NegativeInteger,
        IntegerOverflow,
        MissingUnusedBitCount,
        InvalidUnusedBitCount,
        NonZeroBitPadding,
        BitLengthOverflow,
        NonEmptyNull,
        NonAsciiIa5String,
        EmptyObjectIdentifier,
        UnterminatedObjectIdentifier,
        NonMinimalObjectIdentifier,
        ObjectIdentifierOverflow,
        OidArcLimitExceeded,
    }

    public sealed class Error : Exception
    {
        internal Error(Asn1ErrorKind kind, int offset, string? framingKind = null)
            : base($"ASN.1 DER value error {ErrorId(kind)} at byte {offset}")
        {
            Kind = kind;
            Offset = offset;
            FramingKind = framingKind;
        }

        public Asn1ErrorKind Kind { get; }
        public string KindId => ErrorId(Kind);
        public int Offset { get; }
        public string? FramingKind { get; }
    }

    public sealed class Asn1Element
    {
        private readonly byte[] header;
        private readonly byte[] value;
        private readonly byte[] encoded;

        internal Asn1Element(TlvApi.Element element, int depth)
        {
            Tag = element.Tag;
            header = element.Header.ToArray();
            value = element.Value.ToArray();
            encoded = element.Encoded.ToArray();
            Depth = depth;
        }

        public TlvApi.Tag Tag { get; }
        public ReadOnlyMemory<byte> Header => header.ToArray();
        public ReadOnlyMemory<byte> Value => value.ToArray();
        public ReadOnlyMemory<byte> Encoded => encoded.ToArray();
        public int Depth { get; }
        internal int ValueOffset => header.Length;
        internal ReadOnlyMemory<byte> ValueCopy() => value.ToArray();
    }

    public sealed class Asn1Decoder
    {
        public Asn1Decoder() : this(Asn1Limits.Default) { }

        public Asn1Decoder(Asn1Limits limits)
        {
            ArgumentNullException.ThrowIfNull(limits);
            Limits = limits;
        }

        public Asn1Limits Limits { get; }
        public long ElementsRead { get; private set; }

        public Asn1Element DecodeExact(ReadOnlyMemory<byte> input)
        {
            if (Limits.MaxDepth == 0) throw Failure(Asn1ErrorKind.DepthLimitExceeded, 0);
            RequireElementCapacity(0);
            try
            {
                TlvApi.Element element = TlvApi.DecodeExact(input, Limits.Der);
                Asn1Element snapshot = new(element, 0);
                ElementsRead++;
                return snapshot;
            }
            catch (TlvApi.Error error)
            {
                throw Framing(error);
            }
        }

        public Asn1Cursor Sequence(Asn1Element element) => Constructed(element, "universal", 16);
        public Asn1Cursor Set(Asn1Element element) => Constructed(element, "universal", 17);

        public Asn1Element Explicit(Asn1Element element, uint tagNumber)
        {
            ExpectTag(element, "context-specific", true, tagNumber);
            int childDepth = ChildDepth(element);
            RequireElementCapacity(element.ValueOffset);
            try
            {
                TlvApi.Element child = TlvApi.DecodeExact(element.ValueCopy(), Limits.Der);
                Asn1Element snapshot = new(child, childDepth);
                ElementsRead++;
                return snapshot;
            }
            catch (TlvApi.Error error)
            {
                throw Framing(error);
            }
        }

        internal void CommitElement() => ElementsRead++;

        internal void RequireElementCapacity(int offset)
        {
            if (ElementsRead >= Limits.MaxTotalElements)
                throw Failure(Asn1ErrorKind.ElementLimitExceeded, offset);
        }

        private Asn1Cursor Constructed(Asn1Element element, string tagClass, uint number)
        {
            ExpectTag(element, tagClass, true, number);
            int childDepth = ChildDepth(element);
            try
            {
                return new Asn1Cursor(element.ValueCopy(), childDepth, Limits);
            }
            catch (TlvApi.Error error)
            {
                throw Framing(error);
            }
        }

        private int ChildDepth(Asn1Element element)
        {
            int childDepth = checked(element.Depth + 1);
            if (childDepth >= Limits.MaxDepth) throw Failure(Asn1ErrorKind.DepthLimitExceeded, 0);
            return childDepth;
        }
    }

    public sealed class Asn1Cursor
    {
        private readonly TlvApi.Cursor cursor;
        private readonly int childDepth;
        private readonly Asn1Limits limits;

        internal Asn1Cursor(ReadOnlyMemory<byte> input, int childDepth, Asn1Limits limits)
        {
            byte[] snapshot = input.ToArray();
            cursor = new TlvApi.Cursor(snapshot, limits.Der);
            this.childDepth = childDepth;
            this.limits = limits;
        }

        public ReadOnlyMemory<byte> Remaining => cursor.Remaining.ToArray();

        public Asn1Element? Read(Asn1Decoder decoder)
        {
            ArgumentNullException.ThrowIfNull(decoder);
            if (cursor.Remaining.IsEmpty) return null;
            if (!LimitsEqual(decoder.Limits, limits))
                throw Failure(Asn1ErrorKind.DecoderLimitMismatch, 0);
            decoder.RequireElementCapacity(0);
            try
            {
                TlvApi.Element? element = cursor.Read();
                if (element is null) return null;
                Asn1Element snapshot = new(element, childDepth);
                decoder.CommitElement();
                return snapshot;
            }
            catch (TlvApi.Error error)
            {
                throw Framing(error);
            }
        }

        public void Finish()
        {
            try
            {
                cursor.Finish();
            }
            catch (TlvApi.Error error)
            {
                throw Framing(error);
            }
        }
    }

    public sealed class DerInteger
    {
        private readonly byte[] signedBytes;
        private readonly int valueOffset;

        internal DerInteger(ReadOnlyMemory<byte> value, int valueOffset)
        {
            signedBytes = value.ToArray();
            this.valueOffset = valueOffset;
        }

        public ReadOnlyMemory<byte> SignedBytes => signedBytes.ToArray();
        public bool IsNegative => (signedBytes[0] & 0x80) != 0;

        public ulong ToUInt64()
        {
            if (IsNegative) throw Failure(Asn1ErrorKind.NegativeInteger, valueOffset);
            int start = signedBytes[0] == 0 ? 1 : 0;
            if (signedBytes.Length - start > sizeof(ulong))
                throw Failure(Asn1ErrorKind.IntegerOverflow, valueOffset);
            ulong value = 0;
            for (int index = start; index < signedBytes.Length; index++)
                value = checked(value * 256 + signedBytes[index]);
            return value;
        }
    }

    public sealed class DerBitString
    {
        private readonly byte[] bytes;

        internal DerBitString(ReadOnlyMemory<byte> bytes, byte unusedBits, long bitLength)
        {
            this.bytes = bytes.ToArray();
            UnusedBits = unusedBits;
            BitLength = bitLength;
        }

        public ReadOnlyMemory<byte> Bytes => bytes.ToArray();
        public byte UnusedBits { get; }
        public long BitLength { get; }
    }

    public sealed class ObjectIdentifier
    {
        private readonly byte[] encoded;
        private readonly ReadOnlyCollection<ulong> arcs;

        internal ObjectIdentifier(ReadOnlyMemory<byte> encoded, IEnumerable<ulong> arcs)
        {
            this.encoded = encoded.ToArray();
            this.arcs = Array.AsReadOnly(arcs.ToArray());
        }

        public ReadOnlyMemory<byte> Encoded => encoded.ToArray();
        public IReadOnlyList<ulong> Arcs => arcs;
        public int ArcCount => arcs.Count;
    }

    public static bool DecodeBoolean(Asn1Element element)
    {
        ExpectUniversalPrimitive(element, 1);
        ReadOnlySpan<byte> value = element.ValueCopy().Span;
        if (value.Length != 1) throw Failure(Asn1ErrorKind.InvalidBooleanLength, element.ValueOffset);
        return value[0] switch
        {
            0x00 => false,
            0xff => true,
            _ => throw Failure(Asn1ErrorKind.InvalidBooleanValue, element.ValueOffset),
        };
    }

    public static DerInteger DecodeInteger(Asn1Element element)
    {
        ExpectUniversalPrimitive(element, 2);
        ReadOnlyMemory<byte> copy = element.ValueCopy();
        ReadOnlySpan<byte> value = copy.Span;
        if (value.IsEmpty) throw Failure(Asn1ErrorKind.EmptyInteger, element.ValueOffset);
        if (value.Length > 1 &&
            ((value[0] == 0 && (value[1] & 0x80) == 0) ||
             (value[0] == 0xff && (value[1] & 0x80) != 0)))
            throw Failure(Asn1ErrorKind.NonMinimalInteger, element.ValueOffset);
        return new DerInteger(copy, element.ValueOffset);
    }

    public static DerBitString DecodeBitString(Asn1Element element)
    {
        ExpectUniversalPrimitive(element, 3);
        ReadOnlyMemory<byte> copy = element.ValueCopy();
        ReadOnlySpan<byte> value = copy.Span;
        if (value.IsEmpty) throw Failure(Asn1ErrorKind.MissingUnusedBitCount, element.ValueOffset);
        byte unusedBits = value[0];
        ReadOnlyMemory<byte> payload = copy[1..];
        if (unusedBits > 7 || (payload.IsEmpty && unusedBits != 0))
            throw Failure(Asn1ErrorKind.InvalidUnusedBitCount, element.ValueOffset);
        if (unusedBits != 0 && (payload.Span[^1] & ((1 << unusedBits) - 1)) != 0)
            throw Failure(Asn1ErrorKind.NonZeroBitPadding, element.ValueOffset + value.Length - 1);
        long bitLength;
        try
        {
            bitLength = checked((long)payload.Length * 8 - unusedBits);
        }
        catch (OverflowException)
        {
            throw Failure(Asn1ErrorKind.BitLengthOverflow, element.ValueOffset);
        }
        return new DerBitString(payload, unusedBits, bitLength);
    }

    public static ReadOnlyMemory<byte> DecodeOctetString(Asn1Element element)
    {
        ExpectUniversalPrimitive(element, 4);
        return element.ValueCopy();
    }

    public static ReadOnlyMemory<byte> DecodeImplicitOctetString(Asn1Element element, uint tagNumber)
    {
        ExpectContextPrimitive(element, tagNumber);
        return element.ValueCopy();
    }

    public static string DecodeIa5String(Asn1Element element)
    {
        ExpectUniversalPrimitive(element, 22);
        return DecodeIa5Contents(element);
    }

    public static string DecodeImplicitIa5String(Asn1Element element, uint tagNumber)
    {
        ExpectContextPrimitive(element, tagNumber);
        return DecodeIa5Contents(element);
    }

    public static void DecodeNull(Asn1Element element)
    {
        ExpectUniversalPrimitive(element, 5);
        if (!element.ValueCopy().IsEmpty) throw Failure(Asn1ErrorKind.NonEmptyNull, element.ValueOffset);
    }

    public static ObjectIdentifier DecodeObjectIdentifier(Asn1Element element) =>
        DecodeObjectIdentifier(element, Asn1Limits.Default);

    public static ObjectIdentifier DecodeObjectIdentifier(Asn1Element element, Asn1Limits limits)
    {
        ArgumentNullException.ThrowIfNull(limits);
        ExpectUniversalPrimitive(element, 6);
        return DecodeOidContents(element, limits);
    }

    public static ObjectIdentifier DecodeImplicitObjectIdentifier(Asn1Element element, uint tagNumber) =>
        DecodeImplicitObjectIdentifier(element, tagNumber, Asn1Limits.Default);

    public static ObjectIdentifier DecodeImplicitObjectIdentifier(
        Asn1Element element, uint tagNumber, Asn1Limits limits)
    {
        ArgumentNullException.ThrowIfNull(limits);
        ExpectContextPrimitive(element, tagNumber);
        return DecodeOidContents(element, limits);
    }

    public static string ErrorId(Asn1ErrorKind kind) => kind switch
    {
        Asn1ErrorKind.Framing => "framing",
        Asn1ErrorKind.UnexpectedTag => "unexpected-tag",
        Asn1ErrorKind.DecoderLimitMismatch => "decoder-limit-mismatch",
        Asn1ErrorKind.DepthLimitExceeded => "depth-limit-exceeded",
        Asn1ErrorKind.ElementLimitExceeded => "element-limit-exceeded",
        Asn1ErrorKind.InvalidBooleanLength => "invalid-boolean-length",
        Asn1ErrorKind.InvalidBooleanValue => "invalid-boolean-value",
        Asn1ErrorKind.EmptyInteger => "empty-integer",
        Asn1ErrorKind.NonMinimalInteger => "non-minimal-integer",
        Asn1ErrorKind.NegativeInteger => "negative-integer",
        Asn1ErrorKind.IntegerOverflow => "integer-overflow",
        Asn1ErrorKind.MissingUnusedBitCount => "missing-unused-bit-count",
        Asn1ErrorKind.InvalidUnusedBitCount => "invalid-unused-bit-count",
        Asn1ErrorKind.NonZeroBitPadding => "non-zero-bit-padding",
        Asn1ErrorKind.BitLengthOverflow => "bit-length-overflow",
        Asn1ErrorKind.NonEmptyNull => "non-empty-null",
        Asn1ErrorKind.NonAsciiIa5String => "non-ascii-ia5-string",
        Asn1ErrorKind.EmptyObjectIdentifier => "empty-object-identifier",
        Asn1ErrorKind.UnterminatedObjectIdentifier => "unterminated-object-identifier",
        Asn1ErrorKind.NonMinimalObjectIdentifier => "non-minimal-object-identifier",
        Asn1ErrorKind.ObjectIdentifierOverflow => "object-identifier-overflow",
        Asn1ErrorKind.OidArcLimitExceeded => "oid-arc-limit-exceeded",
        _ => throw new ArgumentOutOfRangeException(nameof(kind)),
    };

    private static string DecodeIa5Contents(Asn1Element element)
    {
        byte[] value = element.ValueCopy().ToArray();
        for (int index = 0; index < value.Length; index++)
            if (value[index] > 0x7f)
                throw Failure(Asn1ErrorKind.NonAsciiIa5String, element.ValueOffset + index);
        return Encoding.ASCII.GetString(value);
    }

    private static ObjectIdentifier DecodeOidContents(Asn1Element element, Asn1Limits limits)
    {
        ReadOnlyMemory<byte> encoded = element.ValueCopy();
        if (encoded.IsEmpty) throw Failure(Asn1ErrorKind.EmptyObjectIdentifier, element.ValueOffset);
        (ulong first, int offset) = ParseBase128(encoded.Span, 0, element.ValueOffset);
        List<ulong> arcs = first switch
        {
            < 40 => [0, first],
            < 80 => [1, first - 40],
            _ => [2, first - 80],
        };
        if (arcs.Count > limits.MaxOidArcs)
            throw Failure(Asn1ErrorKind.OidArcLimitExceeded, element.ValueOffset);
        while (offset < encoded.Length)
        {
            int arcStart = offset;
            (ulong arc, offset) = ParseBase128(encoded.Span, offset, element.ValueOffset);
            arcs.Add(arc);
            if (arcs.Count > limits.MaxOidArcs)
                throw Failure(Asn1ErrorKind.OidArcLimitExceeded, element.ValueOffset + arcStart);
        }
        return new ObjectIdentifier(encoded, arcs);
    }

    private static (ulong Value, int NextOffset) ParseBase128(
        ReadOnlySpan<byte> encoded, int start, int valueOffset)
    {
        if (encoded[start] == 0x80)
            throw Failure(Asn1ErrorKind.NonMinimalObjectIdentifier, valueOffset + start);
        ulong value = 0;
        for (int offset = start; ; offset++)
        {
            if (offset >= encoded.Length)
                throw Failure(Asn1ErrorKind.UnterminatedObjectIdentifier, valueOffset + offset);
            byte octet = encoded[offset];
            byte payload = (byte)(octet & 0x7f);
            if (value > (ulong.MaxValue - payload) / 128)
                throw Failure(Asn1ErrorKind.ObjectIdentifierOverflow, valueOffset + offset);
            value = value * 128 + payload;
            if ((octet & 0x80) == 0) return (value, offset + 1);
        }
    }

    private static void ExpectUniversalPrimitive(Asn1Element element, uint number) =>
        ExpectTag(element, "universal", false, number);

    private static void ExpectContextPrimitive(Asn1Element element, uint number) =>
        ExpectTag(element, "context-specific", false, number);

    private static void ExpectTag(Asn1Element element, string tagClass, bool constructed, uint number)
    {
        ArgumentNullException.ThrowIfNull(element);
        TlvApi.Tag tag = element.Tag;
        if (tag.Class != tagClass || tag.Constructed != constructed || tag.Number != number)
            throw Failure(Asn1ErrorKind.UnexpectedTag, 0);
    }

    private static bool LimitsEqual(Asn1Limits left, Asn1Limits right) =>
        left.MaxDepth == right.MaxDepth &&
        left.MaxTotalElements == right.MaxTotalElements &&
        left.MaxOidArcs == right.MaxOidArcs &&
        left.Der.MaxInputLength == right.Der.MaxInputLength &&
        left.Der.MaxValueLength == right.Der.MaxValueLength &&
        left.Der.MaxElements == right.Der.MaxElements &&
        left.Der.MaxTagNumber == right.Der.MaxTagNumber;

    private static Error Failure(Asn1ErrorKind kind, int offset) => new(kind, offset);
    private static Error Framing(TlvApi.Error error) => new(Asn1ErrorKind.Framing, error.Offset, error.Kind);
}
