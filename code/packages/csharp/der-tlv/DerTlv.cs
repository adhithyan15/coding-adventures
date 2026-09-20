namespace CodingAdventures.DerTlv.CSharp;

/// <summary>Bounded, payload-blind DER tag-length-value framing.</summary>
public static class DerTlv
{
    private static readonly string[] TagClasses = ["universal", "application", "context-specific", "private"];

    public sealed class Limits
    {
        public Limits(long maxInputLength, long maxValueLength, long maxElements, uint maxTagNumber)
        {
            if (maxInputLength < 0) throw new ArgumentOutOfRangeException(nameof(maxInputLength));
            if (maxValueLength < 0) throw new ArgumentOutOfRangeException(nameof(maxValueLength));
            if (maxElements < 0) throw new ArgumentOutOfRangeException(nameof(maxElements));
            MaxInputLength = maxInputLength;
            MaxValueLength = maxValueLength;
            MaxElements = maxElements;
            MaxTagNumber = maxTagNumber;
        }

        public long MaxInputLength { get; }
        public long MaxValueLength { get; }
        public long MaxElements { get; }
        public uint MaxTagNumber { get; }
        public static Limits Default { get; } = new(1_048_576, 1_048_576, 4_096, uint.MaxValue);
    }

    public sealed class Error : Exception
    {
        internal Error(string kind, int offset) : base($"DER framing error {kind} at byte {offset}")
        {
            Kind = kind;
            Offset = offset;
        }

        public string Kind { get; }
        public int Offset { get; }
    }

    public sealed record Tag(string Class, bool Constructed, uint Number);

    public sealed class Element
    {
        private readonly ReadOnlyMemory<byte> input;
        private readonly int headerLength;
        private readonly int encodedLength;

        internal Element(Tag tag, ReadOnlyMemory<byte> input, int headerLength, int encodedLength)
        {
            Tag = tag;
            this.input = input;
            this.headerLength = headerLength;
            this.encodedLength = encodedLength;
        }

        public Tag Tag { get; }
        public ReadOnlyMemory<byte> Header => input[..headerLength];
        public ReadOnlyMemory<byte> Value => input.Slice(headerLength, encodedLength - headerLength);
        public ReadOnlyMemory<byte> Encoded => input[..encodedLength];
    }

    public sealed record Decoded(Element Element, ReadOnlyMemory<byte> Remainder);

    public static Decoded DecodeOne(ReadOnlyMemory<byte> input) => DecodeOne(input, Limits.Default);

    public static Decoded DecodeOne(ReadOnlyMemory<byte> input, Limits limits)
    {
        ArgumentNullException.ThrowIfNull(limits);
        DecodeResult result = DecodeAt(input, 0, limits);
        return new(result.Element, input[result.Consumed..]);
    }

    public static Element DecodeExact(ReadOnlyMemory<byte> input) => DecodeExact(input, Limits.Default);

    public static Element DecodeExact(ReadOnlyMemory<byte> input, Limits limits)
    {
        Decoded decoded = DecodeOne(input, limits);
        if (!decoded.Remainder.IsEmpty) throw Failure("trailing-data", decoded.Element.Encoded.Length);
        return decoded.Element;
    }

    public sealed class Cursor
    {
        private readonly ReadOnlyMemory<byte> input;
        private readonly Limits limits;
        private int offset;

        public Cursor(ReadOnlyMemory<byte> input) : this(input, Limits.Default) { }

        public Cursor(ReadOnlyMemory<byte> input, Limits limits)
        {
            ArgumentNullException.ThrowIfNull(limits);
            this.input = input;
            this.limits = limits;
            if (input.Length > limits.MaxInputLength) throw Failure("input-limit-exceeded", 0);
        }

        public long ElementsRead { get; private set; }
        public ReadOnlyMemory<byte> Remaining => input[offset..];

        public Element? Read()
        {
            if (Remaining.IsEmpty) return null;
            if (ElementsRead >= limits.MaxElements) throw Failure("element-limit-exceeded", offset);
            DecodeResult result = DecodeAt(Remaining, offset, limits);
            offset += result.Consumed;
            ElementsRead++;
            return result.Element;
        }

        public void Finish()
        {
            if (!Remaining.IsEmpty) throw Failure("trailing-data", offset);
        }
    }

    private sealed record DecodeResult(Element Element, int Consumed);

    private static DecodeResult DecodeAt(ReadOnlyMemory<byte> input, int baseOffset, Limits limits)
    {
        if (input.Length > limits.MaxInputLength) throw Failure("input-limit-exceeded", baseOffset);
        if (input.IsEmpty) throw Failure("empty-input", baseOffset);
        ReadOnlySpan<byte> bytes = input.Span;
        byte first = bytes[0];
        string tagClass = TagClasses[first >> 6];
        bool constructed = (first & 0x20) != 0;
        uint number;
        int identifierLength;
        if ((first & 0x1f) != 0x1f)
        {
            number = (uint)(first & 0x1f);
            identifierLength = 1;
            if (number > limits.MaxTagNumber) throw Failure("tag-limit-exceeded", baseOffset);
        }
        else
        {
            (number, identifierLength) = DecodeHighTag(bytes, baseOffset, limits);
        }
        if (tagClass == "universal" && number == 0) throw Failure("end-of-contents", baseOffset);

        (ulong valueLength, int lengthLength, int lengthOffset) = DecodeLength(bytes, baseOffset, identifierLength);
        if (valueLength > (ulong)int.MaxValue) throw Failure("length-host-overflow", lengthOffset);
        if (valueLength > (ulong)limits.MaxValueLength) throw Failure("value-limit-exceeded", lengthOffset);
        int headerLength = identifierLength + lengthLength;
        if (valueLength > (ulong)(int.MaxValue - headerLength)) throw Failure("length-host-overflow", lengthOffset);
        int encodedLength = headerLength + (int)valueLength;
        if (encodedLength > input.Length) throw Failure("truncated-value", baseOffset + input.Length);
        return new(new Element(new(tagClass, constructed, number), input, headerLength, encodedLength), encodedLength);
    }

    private static (uint Number, int Length) DecodeHighTag(ReadOnlySpan<byte> input, int baseOffset, Limits limits)
    {
        ulong number = 0;
        int index = 1;
        while (true)
        {
            if (index >= input.Length) throw Failure("truncated-high-tag", baseOffset + index);
            byte octet = input[index];
            uint payload = (uint)(octet & 0x7f);
            if (index == 1 && payload == 0) throw Failure("non-minimal-tag", baseOffset + index);
            if (number > (uint.MaxValue - payload) / 128) throw Failure("tag-overflow", baseOffset + index);
            number = number * 128 + payload;
            if (number > limits.MaxTagNumber) throw Failure("tag-limit-exceeded", baseOffset + index);
            index++;
            if ((octet & 0x80) == 0) break;
        }
        if (number < 31) throw Failure("non-minimal-tag", baseOffset);
        return ((uint)number, index);
    }

    private static (ulong Value, int Length, int Offset) DecodeLength(ReadOnlySpan<byte> input, int baseOffset, int identifierLength)
    {
        int lengthOffset = baseOffset + identifierLength;
        if (identifierLength >= input.Length) throw Failure("truncated-length", lengthOffset);
        byte first = input[identifierLength];
        if (first < 0x80) return (first, 1, lengthOffset);
        if (first == 0x80) throw Failure("indefinite-length", lengthOffset);
        if (first == 0xff) throw Failure("reserved-length", lengthOffset);
        int count = first & 0x7f;
        if (count > 8) throw Failure("length-too-wide", lengthOffset);
        if (identifierLength + 1 + count > input.Length) throw Failure("truncated-length", baseOffset + input.Length);
        if (input[identifierLength + 1] == 0) throw Failure("non-minimal-length", lengthOffset + 1);
        ulong value = 0;
        for (int index = 0; index < count; index++) value = value * 256 + input[identifierLength + 1 + index];
        if (value < 128) throw Failure("non-minimal-length", lengthOffset);
        return (value, count + 1, lengthOffset);
    }

    private static Error Failure(string kind, int offset) => new(kind, offset);
}
