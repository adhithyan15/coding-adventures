using System.Text;

namespace CodingAdventures.Argon2id.Tests;

public sealed class Argon2idTests
{
    private static readonly byte[] RfcPassword = Enumerable.Repeat((byte)0x01, 32).ToArray();
    private static readonly byte[] RfcSalt = Enumerable.Repeat((byte)0x02, 16).ToArray();

    [Fact]
    public void MatchesRfc9106Vector()
    {
        var options = new Argon2idOptions
        {
            Key = Enumerable.Repeat((byte)0x03, 8).ToArray(),
            AssociatedData = Enumerable.Repeat((byte)0x04, 12).ToArray(),
        };

        var tag = Argon2id.Derive(RfcPassword, RfcSalt, 3, 32, 4, 32, options);

        Assert.Equal("0d640df58d78766c08c037a34a8b53c9d01ef0452d75b65eb52520e96b01e659", Convert.ToHexString(tag).ToLowerInvariant());
    }

    [Fact]
    public void HexMatchesByteForm()
    {
        var tag = Argon2id.Derive(RfcPassword, RfcSalt, 3, 32, 4, 32);
        Assert.Equal(Convert.ToHexString(tag).ToLowerInvariant(), Argon2id.DeriveHex(RfcPassword, RfcSalt, 3, 32, 4, 32));
    }

    [Theory]
    [InlineData(4)]
    [InlineData(16)]
    [InlineData(32)]
    [InlineData(64)]
    [InlineData(65)]
    [InlineData(128)]
    public void SupportsVariableTagLengths(int tagLength)
    {
        Assert.Equal(tagLength, Argon2id.Derive("password"u8.ToArray(), "saltsalt"u8.ToArray(), 1, 8, 1, tagLength).Length);
    }

    [Fact]
    public void SecretInputsBindTheOutput()
    {
        var baseline = DeriveSmall("password", "saltsalt");
        var withKey = Argon2id.Derive(
            "password"u8.ToArray(),
            "saltsalt"u8.ToArray(),
            1,
            8,
            1,
            32,
            new Argon2idOptions { Key = "secret!!"u8.ToArray() });
        var withAssociatedData = Argon2id.Derive(
            "password"u8.ToArray(),
            "saltsalt"u8.ToArray(),
            1,
            8,
            1,
            32,
            new Argon2idOptions { AssociatedData = "context"u8.ToArray() });

        Assert.NotEqual(baseline, withKey);
        Assert.NotEqual(baseline, withAssociatedData);
    }

    [Fact]
    public void PasswordSaltAndPassCountChangeOutput()
    {
        var baseline = DeriveSmall("password", "saltsalt");
        Assert.NotEqual(baseline, DeriveSmall("password2", "saltsalt"));
        Assert.NotEqual(baseline, DeriveSmall("password", "saltsal2"));
        Assert.NotEqual(baseline, Argon2id.Derive("password"u8.ToArray(), "saltsalt"u8.ToArray(), 2, 8, 1, 32));
    }

    [Fact]
    public void RepeatsDeterministicallyAcrossHybridAddressBoundary()
    {
        var first = Argon2id.Derive("password"u8.ToArray(), "saltsalt"u8.ToArray(), 1, 520, 1, 16);
        var second = Argon2id.Derive("password"u8.ToArray(), "saltsalt"u8.ToArray(), 1, 520, 1, 16);
        Assert.Equal(first, second);
    }

    [Fact]
    public void RejectsNullInputs()
    {
        Assert.Throws<ArgumentNullException>(() => Argon2id.Derive(null!, "saltsalt"u8.ToArray(), 1, 8, 1, 32));
        Assert.Throws<ArgumentNullException>(() => Argon2id.Derive([], null!, 1, 8, 1, 32));
        Assert.Throws<ArgumentNullException>(() => Argon2id.Derive([], "saltsalt"u8.ToArray(), 1, 8, 1, 32, new Argon2idOptions { Key = null! }));
        Assert.Throws<ArgumentNullException>(() => Argon2id.Derive([], "saltsalt"u8.ToArray(), 1, 8, 1, 32, new Argon2idOptions { AssociatedData = null! }));
    }

    [Fact]
    public void RejectsInvalidParameters()
    {
        Assert.Throws<ArgumentException>(() => Argon2id.Derive([], "short"u8.ToArray(), 1, 8, 1, 32));
        Assert.Throws<ArgumentOutOfRangeException>(() => Argon2id.Derive([], "saltsalt"u8.ToArray(), 1, 8, 1, 3));
        Assert.Throws<ArgumentOutOfRangeException>(() => Argon2id.Derive([], "saltsalt"u8.ToArray(), 1, 8, 0, 32));
        Assert.Throws<ArgumentOutOfRangeException>(() => Argon2id.Derive([], "saltsalt"u8.ToArray(), 1, 8, 0x0100_0000, 32));
        Assert.Throws<ArgumentOutOfRangeException>(() => Argon2id.Derive([], "saltsalt"u8.ToArray(), 1, 7, 1, 32));
        Assert.Throws<ArgumentOutOfRangeException>(() => Argon2id.Derive([], "saltsalt"u8.ToArray(), 0, 8, 1, 32));
        Assert.Throws<ArgumentOutOfRangeException>(() => Argon2id.Derive([], "saltsalt"u8.ToArray(), 1, 8, 1, 32, new Argon2idOptions { Version = 0x10 }));
    }

    private static byte[] DeriveSmall(string password, string salt) =>
        Argon2id.Derive(Encoding.ASCII.GetBytes(password), Encoding.ASCII.GetBytes(salt), 1, 8, 1, 32);
}
