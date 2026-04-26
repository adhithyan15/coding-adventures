using HkdfAlgorithm = CodingAdventures.Hkdf.Hkdf;
using HkdfHashAlgorithm = CodingAdventures.Hkdf.HkdfHash;

namespace CodingAdventures.Hkdf.Tests;

public sealed class HkdfTests
{
    [Fact]
    public void Sha256MatchesRfc5869TestCase1()
    {
        var ikm = Enumerable.Repeat((byte)0x0b, 22).ToArray();
        var salt = Convert.FromHexString("000102030405060708090a0b0c");
        var info = Convert.FromHexString("f0f1f2f3f4f5f6f7f8f9");

        Assert.Equal(
            "077709362c2e32df0ddc3f0dc47bba6390b6c73bb50f9c3122ec844ad7c2b3e5",
            Convert.ToHexString(HkdfAlgorithm.ExtractSha256(salt, ikm)).ToLowerInvariant());
        Assert.Equal(
            "3cb25f25faacd57a90434f64d0362f2a2d2d0a90cf1a5a4c5db02d56ecc4c5bf34007208d5b887185865",
            Convert.ToHexString(HkdfAlgorithm.DeriveSha256(salt, ikm, info, 42)).ToLowerInvariant());
    }

    [Fact]
    public void Sha256EmptySaltMatchesRfc5869TestCase3()
    {
        var ikm = Enumerable.Repeat((byte)0x0b, 22).ToArray();

        Assert.Equal(
            "19ef24a32c717b167f33a91d6f648bdf96596776afdb6377ac434c1c293ccb04",
            Convert.ToHexString(HkdfAlgorithm.Extract([], ikm)).ToLowerInvariant());
        Assert.Equal(
            "8da4e775a563c18f715f802a063c5a31b8a11f5c5ee1879ec3454e5f3c738d2d9d201395faa4b61a96c8",
            Convert.ToHexString(HkdfAlgorithm.Derive([], ikm, [], 42)).ToLowerInvariant());
    }

    [Fact]
    public void ExpandSupportsBoundsAndInfoDomainSeparation()
    {
        var prk = Enumerable.Repeat((byte)0x01, 32).ToArray();

        Assert.Single(HkdfAlgorithm.ExpandSha256(prk, [], 1));
        Assert.Equal(255 * 32, HkdfAlgorithm.ExpandSha256(prk, [], 255 * 32).Length);
        Assert.NotEqual(
            HkdfAlgorithm.ExpandSha256(prk, "purpose-a"u8.ToArray(), 32),
            HkdfAlgorithm.ExpandSha256(prk, "purpose-b"u8.ToArray(), 32));
    }

    [Fact]
    public void Sha512VariantUsesLargerDigestAndBounds()
    {
        var ikm = Enumerable.Repeat((byte)0x0b, 22).ToArray();
        var prk = HkdfAlgorithm.ExtractSha512([], ikm);

        Assert.Equal(64, prk.Length);
        Assert.Equal(64, HkdfAlgorithm.ExpandSha512(prk, "info"u8.ToArray(), 64).Length);
        Assert.Equal(255 * 64, HkdfAlgorithm.ExpandSha512(Enumerable.Repeat((byte)0x01, 64).ToArray(), [], 255 * 64).Length);
    }

    [Fact]
    public void DeriveEqualsManualExtractThenExpandAndDefaultIsSha256()
    {
        var salt = "salt"u8.ToArray();
        var ikm = "input keying material"u8.ToArray();
        var info = "context"u8.ToArray();

        var combined = HkdfAlgorithm.Derive(salt, ikm, info, 42);
        var manual = HkdfAlgorithm.Expand(HkdfAlgorithm.Extract(salt, ikm), info, 42);
        Assert.Equal(manual, combined);
        Assert.Equal(HkdfAlgorithm.ExtractSha256(salt, ikm), HkdfAlgorithm.Extract(salt, ikm));
    }

    [Fact]
    public void ValidationRejectsInvalidInputs()
    {
        Assert.Throws<ArgumentNullException>(() => HkdfAlgorithm.Extract(null!, "ikm"u8.ToArray()));
        Assert.Throws<ArgumentNullException>(() => HkdfAlgorithm.Extract([], null!));
        Assert.Throws<ArgumentNullException>(() => HkdfAlgorithm.Expand(null!, [], 1));
        Assert.Throws<ArgumentNullException>(() => HkdfAlgorithm.Expand([], null!, 1));
        Assert.Throws<ArgumentOutOfRangeException>(() => HkdfAlgorithm.Expand([], [], 0));
        Assert.Throws<ArgumentOutOfRangeException>(() => HkdfAlgorithm.Expand(Enumerable.Repeat((byte)0x01, 32).ToArray(), [], 255 * 32 + 1));
        Assert.Throws<ArgumentOutOfRangeException>(() => HkdfAlgorithm.Extract([], [], (HkdfHashAlgorithm)99));
    }
}
