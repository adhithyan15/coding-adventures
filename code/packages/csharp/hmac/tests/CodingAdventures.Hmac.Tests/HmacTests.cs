using System.Security.Cryptography;
using System.Text;
using HmacAlgorithm = CodingAdventures.Hmac.Hmac;

namespace CodingAdventures.Hmac.Tests;

public sealed class HmacTests
{
    [Fact]
    public void HmacSha256MatchesRfc4231Vectors()
    {
        Assert.Equal(
            "b0344c61d8db38535ca8afceaf0bf12b881dc200c9833da726e9376c2e32cff7",
            HmacAlgorithm.HmacSha256Hex(Enumerable.Repeat((byte)0x0b, 20).ToArray(), "Hi There"u8.ToArray()));
        Assert.Equal(
            "5bdcc146bf60754e6a042426089575c75a003f089d2739839dec58b964ec3843",
            HmacAlgorithm.HmacSha256Hex("Jefe"u8.ToArray(), Encoding.ASCII.GetBytes("what do ya want for nothing?")));
    }

    [Fact]
    public void HmacSha512MatchesRfc4231Vectors()
    {
        Assert.Equal(
            "87aa7cdea5ef619d4ff0b4241a1d6cb02379f4e2ce4ec2787ad0b30545e17cdedaa833b7d6b8a702038b274eaea3f4e4be9d914eeb61f1702e696c203a126854",
            HmacAlgorithm.HmacSha512Hex(Enumerable.Repeat((byte)0x0b, 20).ToArray(), "Hi There"u8.ToArray()));
        Assert.Equal(
            "164b7a7bfcf819e2e395fbe73b56e0a387bd64222e831fd610270cd7ea2505549758bf75c05a994a6d034f65f8f0e6fdcaeab1a34d4a6b4b636e070a38bce737",
            HmacAlgorithm.HmacSha512Hex("Jefe"u8.ToArray(), Encoding.ASCII.GetBytes("what do ya want for nothing?")));
    }

    [Fact]
    public void LegacyVariantsMatchRfc2202Vectors()
    {
        var key = Enumerable.Repeat((byte)0x0b, 16).ToArray();
        Assert.Equal("9294727a3638bb1c13f48ef8158bfc9d", HmacAlgorithm.HmacMd5Hex(key, "Hi There"u8.ToArray()));

        var sha1Key = Enumerable.Repeat((byte)0x0b, 20).ToArray();
        Assert.Equal("b617318655057264e28bc0b6fb378c8ef146be00", HmacAlgorithm.HmacSha1Hex(sha1Key, "Hi There"u8.ToArray()));
        Assert.Equal("effcdf6ae5eb2fa2d27416d5f184df9c259a7c79", HmacAlgorithm.HmacSha1Hex("Jefe"u8.ToArray(), Encoding.ASCII.GetBytes("what do ya want for nothing?")));
    }

    [Fact]
    public void ReturnLengthsMatchHashFamilies()
    {
        var key = "key"u8.ToArray();
        var message = "message"u8.ToArray();

        Assert.Equal(16, HmacAlgorithm.HmacMd5(key, message).Length);
        Assert.Equal(20, HmacAlgorithm.HmacSha1(key, message).Length);
        Assert.Equal(32, HmacAlgorithm.HmacSha256(key, message).Length);
        Assert.Equal(64, HmacAlgorithm.HmacSha512(key, message).Length);
        Assert.Equal(64, HmacAlgorithm.HmacSha256Hex(key, message).Length);
        Assert.Equal(128, HmacAlgorithm.HmacSha512Hex(key, message).Length);
    }

    [Fact]
    public void GenericComputeMatchesNamedVariant()
    {
        var key = Enumerable.Repeat((byte)0x01, 100).ToArray();
        var message = "msg"u8.ToArray();

        Assert.Equal(HmacAlgorithm.HmacSha256(key, message), HmacAlgorithm.Compute(SHA256.HashData, 64, key, message));
        Assert.Equal(HmacAlgorithm.HmacSha512(key, message), HmacAlgorithm.Compute(SHA512.HashData, 128, key, message));
    }

    [Fact]
    public void VerifyUsesConstantTimeComparisonSemantics()
    {
        var tag = HmacAlgorithm.HmacSha256("key"u8.ToArray(), "message"u8.ToArray());

        Assert.True(HmacAlgorithm.Verify(tag, tag.ToArray()));
        Assert.False(HmacAlgorithm.Verify(tag, HmacAlgorithm.HmacSha256("key2"u8.ToArray(), "message"u8.ToArray())));
        Assert.False(HmacAlgorithm.Verify(tag, tag[..^1]));
    }

    [Fact]
    public void EmptyKeyIsRejectedButEmptyMessageIsAllowed()
    {
        Assert.Throws<ArgumentException>(() => HmacAlgorithm.HmacSha256([], "message"u8.ToArray()));
        Assert.Equal(32, HmacAlgorithm.HmacSha256("key"u8.ToArray(), []).Length);
    }

    [Fact]
    public void NullInputsAreRejected()
    {
        Assert.Throws<ArgumentNullException>(() => HmacAlgorithm.Compute(null!, 64, "key"u8.ToArray(), "message"u8.ToArray()));
        Assert.Throws<ArgumentNullException>(() => HmacAlgorithm.HmacSha256(null!, "message"u8.ToArray()));
        Assert.Throws<ArgumentNullException>(() => HmacAlgorithm.HmacSha256("key"u8.ToArray(), null!));
        Assert.Throws<ArgumentNullException>(() => HmacAlgorithm.Verify(null!, []));
        Assert.Throws<ArgumentNullException>(() => HmacAlgorithm.Verify([], null!));
    }

    [Fact]
    public void InvalidBlockSizeIsRejected()
    {
        Assert.Throws<ArgumentOutOfRangeException>(() =>
            HmacAlgorithm.Compute(SHA256.HashData, 0, "key"u8.ToArray(), "message"u8.ToArray()));
    }
}
