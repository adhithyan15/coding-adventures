using Pbkdf2Algorithm = CodingAdventures.Pbkdf2.Pbkdf2;

namespace CodingAdventures.Pbkdf2.Tests;

public sealed class Pbkdf2Tests
{
    [Fact]
    public void Sha1MatchesRfc6070Vectors()
    {
        Assert.Equal(
            "0c60c80f961f0e71f3a9b524af6012062fe037a6",
            Pbkdf2Algorithm.Pbkdf2HmacSha1Hex("password"u8.ToArray(), "salt"u8.ToArray(), 1, 20));
        Assert.Equal(
            "4b007901b765489abead49d926f721d065a429c1",
            Pbkdf2Algorithm.Pbkdf2HmacSha1Hex("password"u8.ToArray(), "salt"u8.ToArray(), 4096, 20));
        Assert.Equal(
            "56fa6aa75548099dcc37d7f03425e0c3",
            Pbkdf2Algorithm.Pbkdf2HmacSha1Hex("pass\0word"u8.ToArray(), "sa\0lt"u8.ToArray(), 4096, 16));
    }

    [Fact]
    public void Sha256MatchesKnownVectorAndCanExtendPastOneBlock()
    {
        var key = Pbkdf2Algorithm.Pbkdf2HmacSha256("passwd"u8.ToArray(), "salt"u8.ToArray(), 1, 64);
        Assert.Equal(
            "55ac046e56e3089fec1691c22544b605f94185216dde0465e68b9d57c20dacbc49ca9cccf179b645991664b39d77ef317c71b845b1e30bd509112041d3a19783",
            Convert.ToHexString(key).ToLowerInvariant());

        Assert.Equal(
            Pbkdf2Algorithm.Pbkdf2HmacSha256("password"u8.ToArray(), "salt"u8.ToArray(), 1, 32),
            Pbkdf2Algorithm.Pbkdf2HmacSha256("password"u8.ToArray(), "salt"u8.ToArray(), 1, 64)[..32]);
    }

    [Fact]
    public void Sha512SupportsCustomKeyLengths()
    {
        var full = Pbkdf2Algorithm.Pbkdf2HmacSha512("secret"u8.ToArray(), "nacl"u8.ToArray(), 1, 64);
        var shortKey = Pbkdf2Algorithm.Pbkdf2HmacSha512("secret"u8.ToArray(), "nacl"u8.ToArray(), 1, 32);

        Assert.Equal(64, full.Length);
        Assert.Equal(shortKey, full[..32]);
        Assert.Equal(128, Pbkdf2Algorithm.Pbkdf2HmacSha512("key"u8.ToArray(), "salt"u8.ToArray(), 1, 128).Length);
    }

    [Fact]
    public void HexHelpersMatchByteOutput()
    {
        Assert.Equal(
            Convert.ToHexString(Pbkdf2Algorithm.Pbkdf2HmacSha1("password"u8.ToArray(), "salt"u8.ToArray(), 1, 20)).ToLowerInvariant(),
            Pbkdf2Algorithm.Pbkdf2HmacSha1Hex("password"u8.ToArray(), "salt"u8.ToArray(), 1, 20));
        Assert.Equal(
            Convert.ToHexString(Pbkdf2Algorithm.Pbkdf2HmacSha256("passwd"u8.ToArray(), "salt"u8.ToArray(), 1, 32)).ToLowerInvariant(),
            Pbkdf2Algorithm.Pbkdf2HmacSha256Hex("passwd"u8.ToArray(), "salt"u8.ToArray(), 1, 32));
        Assert.Equal(
            Convert.ToHexString(Pbkdf2Algorithm.Pbkdf2HmacSha512("secret"u8.ToArray(), "nacl"u8.ToArray(), 1, 64)).ToLowerInvariant(),
            Pbkdf2Algorithm.Pbkdf2HmacSha512Hex("secret"u8.ToArray(), "nacl"u8.ToArray(), 1, 64));
    }

    [Fact]
    public void ValidationRejectsInvalidInputs()
    {
        Assert.Throws<ArgumentNullException>(() => Pbkdf2Algorithm.Pbkdf2HmacSha256(null!, "salt"u8.ToArray(), 1, 32));
        Assert.Throws<ArgumentNullException>(() => Pbkdf2Algorithm.Pbkdf2HmacSha256("password"u8.ToArray(), null!, 1, 32));
        Assert.Throws<ArgumentException>(() => Pbkdf2Algorithm.Pbkdf2HmacSha256([], "salt"u8.ToArray(), 1, 32));
        Assert.Throws<ArgumentOutOfRangeException>(() => Pbkdf2Algorithm.Pbkdf2HmacSha256("pw"u8.ToArray(), "salt"u8.ToArray(), 0, 32));
        Assert.Throws<ArgumentOutOfRangeException>(() => Pbkdf2Algorithm.Pbkdf2HmacSha256("pw"u8.ToArray(), "salt"u8.ToArray(), 1, 0));
        Assert.Throws<ArgumentOutOfRangeException>(() => Pbkdf2Algorithm.Pbkdf2HmacSha256("pw"u8.ToArray(), "salt"u8.ToArray(), 1, (1 << 20) + 1));
    }

    [Fact]
    public void EmptySaltIsAllowedAndEmptyPasswordCanBeExplicitlyAllowed()
    {
        Assert.Equal(32, Pbkdf2Algorithm.Pbkdf2HmacSha256("password"u8.ToArray(), [], 1, 32).Length);
        Assert.Equal(32, Pbkdf2Algorithm.Pbkdf2HmacSha256([], "salt"u8.ToArray(), 1, 32, allowEmptyPassword: true).Length);
    }

    [Fact]
    public void SaltPasswordAndIterationsAffectOutput()
    {
        var baseKey = Pbkdf2Algorithm.Pbkdf2HmacSha256("password"u8.ToArray(), "salt"u8.ToArray(), 1, 32);

        Assert.NotEqual(baseKey, Pbkdf2Algorithm.Pbkdf2HmacSha256("password"u8.ToArray(), "salt2"u8.ToArray(), 1, 32));
        Assert.NotEqual(baseKey, Pbkdf2Algorithm.Pbkdf2HmacSha256("password2"u8.ToArray(), "salt"u8.ToArray(), 1, 32));
        Assert.NotEqual(baseKey, Pbkdf2Algorithm.Pbkdf2HmacSha256("password"u8.ToArray(), "salt"u8.ToArray(), 2, 32));
    }
}
