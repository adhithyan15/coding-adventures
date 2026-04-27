namespace CodingAdventures.HashFunctions.Tests;

public sealed class HashFunctionsTests
{
    [Fact]
    public void Fnv1aKnownVectors()
    {
        Assert.Equal(0x811C9DC5u, HashFunctions.Fnv1a32(Array.Empty<byte>()));
        Assert.Equal(0xE40C292Cu, HashFunctions.Fnv1a32("a"));
        Assert.Equal(0x1A47E90Bu, HashFunctions.Fnv1a32("abc"));
        Assert.Equal(1_335_831_723u, HashFunctions.Fnv1a32("hello"));
        Assert.Equal(3_214_735_720u, HashFunctions.Fnv1a32("foobar"));

        Assert.Equal(0xCBF29CE484222325UL, HashFunctions.Fnv1a64(Array.Empty<byte>()));
        Assert.Equal(0xAF63DC4C8601EC8CUL, HashFunctions.Fnv1a64("a"));
        Assert.Equal(0xE71FA2190541574BUL, HashFunctions.Fnv1a64("abc"));
        Assert.Equal(0xA430D84680AABD0BUL, HashFunctions.Fnv1a64("hello"));
    }

    [Fact]
    public void Djb2AndPolynomialKnownVectors()
    {
        Assert.Equal(5_381UL, HashFunctions.Djb2(Array.Empty<byte>()));
        Assert.Equal(177_670UL, HashFunctions.Djb2("a"));
        Assert.Equal(193_485_963UL, HashFunctions.Djb2("abc"));

        Assert.Equal(0UL, HashFunctions.PolynomialRolling(Array.Empty<byte>()));
        Assert.Equal(97UL, HashFunctions.PolynomialRolling("a"));
        Assert.Equal(3_105UL, HashFunctions.PolynomialRolling("ab"));
        Assert.Equal(96_354UL, HashFunctions.PolynomialRolling("abc"));
        Assert.Equal(((97UL * 37 + 98) * 37 + 99), HashFunctions.PolynomialRolling("abc", 37, 1_000_000_007));
        Assert.Throws<ArgumentOutOfRangeException>(() => HashFunctions.PolynomialRolling("abc", modulus: 0));
    }

    [Fact]
    public void Murmur3KnownVectors()
    {
        Assert.Equal(0u, HashFunctions.Murmur3_32(Array.Empty<byte>()));
        Assert.Equal(0x514E28B7u, HashFunctions.Murmur3_32(Array.Empty<byte>(), 1));
        Assert.Equal(0x3C2569B2u, HashFunctions.Murmur3_32("a"));
        Assert.Equal(0xB3DD93FAu, HashFunctions.Murmur3_32("abc"));
        Assert.Equal(0x43ED676Au, HashFunctions.Murmur3_32("abcd"));
    }

    [Fact]
    public void SipHashKnownVectorsAndStringHelpers()
    {
        var key = Enumerable.Range(0, 16).Select(value => (byte)value).ToArray();

        Assert.Equal(0x726FDB47DD0E0E31UL, HashFunctions.SipHash24(Array.Empty<byte>(), key));
        Assert.Equal(0x74F839C593DC67FDUL, HashFunctions.SipHash24(new byte[] { 0 }, key));
        Assert.Equal(HashFunctions.Fnv1a32("hello"), HashFunctions.HashStringFnv1a32("hello"));
        Assert.Equal(HashFunctions.SipHash24("hello"u8.ToArray(), key), HashFunctions.HashStringSipHash("hello", key));
        Assert.Throws<ArgumentException>(() => HashFunctions.SipHash24(Array.Empty<byte>(), new byte[8]));
    }

    [Fact]
    public void StrategyTypesForwardToFreeFunctions()
    {
        IHashFunction[] strategies =
        [
            new Fnv1a32(),
            new Fnv1a64(),
            new Djb2Hash(),
            new PolynomialRollingHash(),
            new Murmur3_32(),
            new SipHash24(new byte[16]),
        ];

        var input = "abc"u8.ToArray();

        Assert.Equal(HashFunctions.Fnv1a32(input), (uint)strategies[0].Hash(input));
        Assert.Equal(HashFunctions.Fnv1a64(input), strategies[1].Hash(input));
        Assert.Equal(HashFunctions.Djb2(input), strategies[2].Hash(input));
        Assert.Equal(HashFunctions.PolynomialRolling(input), strategies[3].Hash(input));
        Assert.Equal(HashFunctions.Murmur3_32(input), (uint)strategies[4].Hash(input));
        Assert.Equal(HashFunctions.SipHash24(input, new byte[16]), strategies[5].Hash(input));
        Assert.Equal([32, 64, 64, 64, 32, 64], strategies.Select(strategy => strategy.OutputBits).ToArray());
    }

    [Fact]
    public void DistributionTestMatchesExactConstantHashMath()
    {
        var inputs = new[]
        {
            "a"u8.ToArray(),
            "b"u8.ToArray(),
            "c"u8.ToArray(),
            "d"u8.ToArray(),
        };

        Assert.Equal(12.0, HashFunctions.DistributionTest(_ => 0UL, inputs, 4));
        Assert.Throws<ArgumentOutOfRangeException>(() => HashFunctions.DistributionTest(_ => 0UL, inputs, 0));
        Assert.Throws<ArgumentException>(() => HashFunctions.DistributionTest(_ => 0UL, Array.Empty<byte[]>(), 4));
    }

    [Fact]
    public void AvalancheScoreHandlesSmallSamples()
    {
        Assert.Equal(0.0, HashFunctions.AvalancheScore(_ => 0UL, 32, sampleSize: 1));
        Assert.Throws<ArgumentOutOfRangeException>(() => HashFunctions.AvalancheScore(_ => 0UL, 65, sampleSize: 1));
        Assert.Throws<ArgumentOutOfRangeException>(() => HashFunctions.AvalancheScore(_ => 0UL, 32, sampleSize: 0));
    }
}
