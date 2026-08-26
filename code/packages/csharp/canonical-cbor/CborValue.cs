namespace CodingAdventures.CanonicalCbor.CSharp;

/// <summary>The deliberately small value algebra supported by CBR01.</summary>
public abstract record CborValue;

/// <summary>Major type 0.</summary>
public sealed record CborUnsigned(ulong Value) : CborValue;

/// <summary>Major type 1, represented by the unsigned argument in -1 - n.</summary>
public sealed record CborNegative(ulong Value) : CborValue;

/// <summary>Major type 2 with defensive-copy value semantics.</summary>
public sealed record CborByteString : CborValue
{
    private readonly byte[] value;

    public CborByteString(byte[] value)
    {
        ArgumentNullException.ThrowIfNull(value);
        this.value = (byte[])value.Clone();
    }

    public byte[] Value => (byte[])value.Clone();

    internal ReadOnlySpan<byte> RawValue => value;

    public bool Equals(CborByteString? other) =>
        other is not null && value.AsSpan().SequenceEqual(other.value);

    public override int GetHashCode()
    {
        HashCode hash = new();
        foreach (byte item in value)
        {
            hash.Add(item);
        }
        return hash.ToHashCode();
    }

    public override string ToString() => $"CborByteString(Length={value.Length})";
}

/// <summary>Major type 3. Lone UTF-16 surrogates are rejected.</summary>
public sealed record CborText : CborValue
{
    public CborText(string value)
    {
        ArgumentNullException.ThrowIfNull(value);
        ValidateScalarText(value);
        Value = value;
    }

    public string Value { get; }

    private static void ValidateScalarText(string value)
    {
        for (int index = 0; index < value.Length; index++)
        {
            char unit = value[index];
            if (char.IsHighSurrogate(unit))
            {
                if (index + 1 >= value.Length || !char.IsLowSurrogate(value[index + 1]))
                {
                    throw new ArgumentException("canonical-cbor: text is not Unicode scalar data", nameof(value));
                }
                index++;
            }
            else if (char.IsLowSurrogate(unit))
            {
                throw new ArgumentException("canonical-cbor: text is not Unicode scalar data", nameof(value));
            }
        }
    }
}

/// <summary>Major type 4.</summary>
public sealed record CborArray : CborValue
{
    public CborArray(IEnumerable<CborValue> values)
    {
        ArgumentNullException.ThrowIfNull(values);
        Values = Array.AsReadOnly(values.ToArray());
        if (Values.Any(static value => value is null))
        {
            throw new ArgumentException("CBOR arrays cannot contain null references", nameof(values));
        }
    }

    public IReadOnlyList<CborValue> Values { get; }

    public bool Equals(CborArray? other) =>
        other is not null && Values.SequenceEqual(other.Values);

    public override int GetHashCode()
    {
        HashCode hash = new();
        foreach (CborValue item in Values)
        {
            hash.Add(item);
        }
        return hash.ToHashCode();
    }
}

/// <summary>One pre-canonicalization map entry.</summary>
public sealed record CborMapEntry
{
    public CborMapEntry(CborValue key, CborValue value)
    {
        ArgumentNullException.ThrowIfNull(key);
        ArgumentNullException.ThrowIfNull(value);
        Key = key;
        Value = value;
    }

    public CborValue Key { get; }
    public CborValue Value { get; }
}

/// <summary>Major type 5. The encoder sorts encoded keys.</summary>
public sealed record CborMap : CborValue
{
    public CborMap(IEnumerable<CborMapEntry> entries)
    {
        ArgumentNullException.ThrowIfNull(entries);
        Entries = Array.AsReadOnly(entries.ToArray());
        if (Entries.Any(static entry => entry is null))
        {
            throw new ArgumentException("CBOR maps cannot contain null entries", nameof(entries));
        }
    }

    public IReadOnlyList<CborMapEntry> Entries { get; }

    public bool Equals(CborMap? other) =>
        other is not null && Entries.SequenceEqual(other.Entries);

    public override int GetHashCode()
    {
        HashCode hash = new();
        foreach (CborMapEntry entry in Entries)
        {
            hash.Add(entry);
        }
        return hash.ToHashCode();
    }
}

/// <summary>Major type 6. Tag semantics remain opaque.</summary>
public sealed record CborTag : CborValue
{
    public CborTag(ulong number, CborValue value)
    {
        ArgumentNullException.ThrowIfNull(value);
        Number = number;
        Value = value;
    }

    public ulong Number { get; }
    public CborValue Value { get; }
}

/// <summary>Major type 7 simple values 20 and 21.</summary>
public sealed record CborBoolean(bool Value) : CborValue;

/// <summary>Major type 7 simple value 22.</summary>
public sealed record CborNull : CborValue
{
    private CborNull() { }
    public static CborNull Instance { get; } = new();
}
