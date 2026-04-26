using CodingAdventures.Uuid;

namespace CodingAdventures.Uuid.Tests;

public sealed class UuidTests
{
    [Fact]
    public void FromStringAcceptsSupportedForms()
    {
        const string expected = "550e8400-e29b-41d4-a716-446655440000";

        Assert.Equal(expected, Uuid.FromString(expected).ToString());
        Assert.Equal(expected, Uuid.FromString("550E8400-E29B-41D4-A716-446655440000").ToString());
        Assert.Equal(expected, Uuid.FromString("550e8400e29b41d4a716446655440000").ToString());
        Assert.Equal(expected, Uuid.FromString("{550e8400-e29b-41d4-a716-446655440000}").ToString());
        Assert.Equal(expected, Uuid.FromString("urn:uuid:550e8400-e29b-41d4-a716-446655440000").ToString());
        Assert.Equal(expected, Uuid.FromString("  550e8400-e29b-41d4-a716-446655440000  ").ToString());
    }

    [Fact]
    public void FromStringRejectsInvalidText()
    {
        Assert.Throws<UuidException>(() => Uuid.FromString("not-a-uuid"));
        Assert.Throws<UuidException>(() => Uuid.FromString("550e8400-e29b-41d4-a716-44665544000z"));
        Assert.Throws<UuidException>(() => Uuid.FromString(""));
        Assert.Throws<UuidException>(() => Uuid.FromString(null));
    }

    [Fact]
    public void FromBytesRoundTripsAndRejectsWrongLength()
    {
        var uuid = Uuid.FromString("550e8400-e29b-41d4-a716-446655440000");
        var bytes = uuid.ToBytes();

        Assert.Equal(16, bytes.Length);
        Assert.Equal(uuid, Uuid.FromBytes(bytes));
        Assert.Equal(uuid, Uuid.FromBytes((ReadOnlySpan<byte>)bytes));
        Assert.Throws<UuidException>(() => Uuid.FromBytes(new byte[15]));
        Assert.Throws<UuidException>(() => Uuid.FromBytes(new byte[17]));
        Assert.Throws<UuidException>(() => Uuid.FromBytes(null));
        Assert.Throws<UuidException>(() => Uuid.FromBytes(ReadOnlySpan<byte>.Empty));
    }

    [Fact]
    public void PropertiesExposeVersionVariantAndExtremes()
    {
        Assert.Equal(4, Uuid.FromString("550e8400-e29b-41d4-a716-446655440000").Version);
        Assert.Equal(1, Uuid.FromString("6ba7b810-9dad-11d1-80b4-00c04fd430c8").Version);
        Assert.Equal(5, Uuid.FromString("886313e1-3b8a-5372-9b90-0c9aee199e5d").Version);
        Assert.True(Uuid.NIL.IsNil);
        Assert.False(Uuid.V4().IsNil);
        Assert.True(Uuid.MAX.IsMax);
        Assert.False(Uuid.V4().IsMax);
        Assert.Equal("00000000-0000-0000-0000-000000000000", Uuid.NIL.ToString());
        Assert.Equal("ffffffff-ffff-ffff-ffff-ffffffffffff", Uuid.MAX.ToString());
    }

    [Fact]
    public void VariantClassifiesAllFamilies()
    {
        foreach (var nibble in new[] { "8", "9", "a", "b" })
        {
            var uuid = Uuid.FromString($"550e8400-e29b-41d4-{nibble}716-446655440000");
            Assert.Equal("rfc4122", uuid.Variant);
        }

        Assert.Equal("ncs", new Uuid(0UL, 0x0000_0000_0000_0000UL).Variant);
        Assert.Equal("ncs", new Uuid(0UL, 0x4000_0000_0000_0000UL).Variant);
        Assert.Equal("microsoft", new Uuid(0UL, 0xC000_0000_0000_0000UL).Variant);
        Assert.Equal("reserved", new Uuid(0UL, 0xE000_0000_0000_0000UL).Variant);
    }

    [Fact]
    public void EqualityHashAndOrderingUseUnsignedBytes()
    {
        var first = Uuid.FromString("00000000-0000-0000-0000-000000000001");
        var second = Uuid.FromString("00000000-0000-0000-0000-000000000002");
        var high = Uuid.FromString("ffffffff-ffff-ffff-ffff-ffffffffffff");

        Assert.Equal(first, Uuid.FromString(first.ToString()));
        Assert.Equal(first.GetHashCode(), Uuid.FromString(first.ToString()).GetHashCode());
        Assert.True(first.CompareTo(second) < 0);
        Assert.True(second.CompareTo(first) > 0);
        Assert.True(first.CompareTo(high) < 0);
        Assert.Equal(0, first.CompareTo(first));
        Assert.Single(new HashSet<Uuid> { first, Uuid.FromString(first.ToString()) });
    }

    [Fact]
    public void IsValidMatchesParser()
    {
        Assert.True(Uuid.IsValid("550e8400-e29b-41d4-a716-446655440000"));
        Assert.True(Uuid.IsValid("550e8400e29b41d4a716446655440000"));
        Assert.True(Uuid.IsValid("{550e8400-e29b-41d4-a716-446655440000}"));
        Assert.True(Uuid.IsValid("urn:uuid:550e8400-e29b-41d4-a716-446655440000"));
        Assert.False(Uuid.IsValid("not-a-uuid"));
        Assert.False(Uuid.IsValid(""));
        Assert.False(Uuid.IsValid(null));
        Assert.False(Uuid.IsValid("550e8400-e29b-41d4-a716-44665544000z"));
    }

    [Fact]
    public void V4HasVersionVariantFormatAndUniqueness()
    {
        var value = Uuid.V4();
        var text = value.ToString();

        Assert.Equal(4, value.Version);
        Assert.Equal("rfc4122", value.Variant);
        Assert.Equal('4', text[14]);
        Assert.Contains(text[19], new[] { '8', '9', 'a', 'b' });
        Assert.Equal(1000, Enumerable.Range(0, 1000).Select(_ => Uuid.V4()).ToHashSet().Count);
    }

    [Fact]
    public void V7HasVersionVariantTimestampAndUniqueness()
    {
        var first = Uuid.V7();
        var second = Uuid.V7();
        var firstTimestamp = first.Msb >> 16;
        var secondTimestamp = second.Msb >> 16;

        Assert.Equal(7, first.Version);
        Assert.Equal("rfc4122", first.Variant);
        Assert.True(firstTimestamp <= secondTimestamp);
        Assert.Equal(100, Enumerable.Range(0, 100).Select(_ => Uuid.V7()).ToHashSet().Count);
    }

    [Fact]
    public void V1HasVersionVariantAndUniqueness()
    {
        var value = Uuid.V1();

        Assert.Equal(1, value.Version);
        Assert.Equal("rfc4122", value.Variant);
        Assert.Equal(100, Enumerable.Range(0, 100).Select(_ => Uuid.V1()).ToHashSet().Count);
    }

    [Fact]
    public void V5MatchesRfcVectorAndIsDeterministic()
    {
        Assert.Equal("886313e1-3b8a-5372-9b90-0c9aee199e5d", Uuid.V5(Uuid.NAMESPACE_DNS, "python.org").ToString());
        Assert.Equal(Uuid.V5(Uuid.NAMESPACE_DNS, "example.com"), Uuid.V5(Uuid.NAMESPACE_DNS, "example.com"));
        Assert.NotEqual(Uuid.V5(Uuid.NAMESPACE_DNS, "example.com"), Uuid.V5(Uuid.NAMESPACE_DNS, "example.org"));
        Assert.NotEqual(Uuid.V5(Uuid.NAMESPACE_DNS, "example.com"), Uuid.V5(Uuid.NAMESPACE_URL, "example.com"));
        Assert.Equal(5, Uuid.V5(Uuid.NAMESPACE_DNS, "test").Version);
        Assert.Equal("rfc4122", Uuid.V5(Uuid.NAMESPACE_DNS, "test").Variant);
        Assert.Throws<ArgumentNullException>(() => Uuid.V5(Uuid.NAMESPACE_DNS, null!));
    }

    [Fact]
    public void V3MatchesRfcVectorAndIsDeterministic()
    {
        Assert.Equal("6fa459ea-ee8a-3ca4-894e-db77e160355e", Uuid.V3(Uuid.NAMESPACE_DNS, "python.org").ToString());
        Assert.Equal(Uuid.V3(Uuid.NAMESPACE_DNS, "example.com"), Uuid.V3(Uuid.NAMESPACE_DNS, "example.com"));
        Assert.NotEqual(Uuid.V3(Uuid.NAMESPACE_DNS, "python.org"), Uuid.V5(Uuid.NAMESPACE_DNS, "python.org"));
        Assert.Equal(3, Uuid.V3(Uuid.NAMESPACE_DNS, "test").Version);
        Assert.Throws<ArgumentNullException>(() => Uuid.V3(Uuid.NAMESPACE_DNS, null!));
    }

    [Fact]
    public void NamespaceConstantsMatchRfcValues()
    {
        Assert.Equal("6ba7b810-9dad-11d1-80b4-00c04fd430c8", Uuid.NAMESPACE_DNS.ToString());
        Assert.Equal("6ba7b811-9dad-11d1-80b4-00c04fd430c8", Uuid.NAMESPACE_URL.ToString());
        Assert.Equal("6ba7b812-9dad-11d1-80b4-00c04fd430c8", Uuid.NAMESPACE_OID.ToString());
        Assert.Equal("6ba7b814-9dad-11d1-80b4-00c04fd430c8", Uuid.NAMESPACE_X500.ToString());
    }

    [Fact]
    public void ExceptionCanWrapCause()
    {
        var inner = new InvalidOperationException("boom");
        var exception = new UuidException("wrapped", inner);

        Assert.Equal("wrapped", exception.Message);
        Assert.Same(inner, exception.InnerException);
    }
}
