using System;
using System.Linq;
using System.Text;
using Md5HasherType = CodingAdventures.Md5.Md5Hasher;
using Md5Type = CodingAdventures.Md5.Md5;

namespace CodingAdventures.Md5.Tests;

public sealed class Md5Tests
{
    [Fact]
    public void VersionAndHexUtilitiesMatchExpectedOutput()
    {
        Assert.Equal("0.1.0", Md5Type.VERSION);
        Assert.Equal(string.Empty, Md5Type.ToHex(Array.Empty<byte>()));
        Assert.Equal("00", Md5Type.ToHex([0x00]));
        Assert.Equal("ff", Md5Type.ToHex([0xff]));
        Assert.Equal("d41d8cd9", Md5Type.ToHex([0xd4, 0x1d, 0x8c, 0xd9]));
    }

    [Fact]
    public void Rfc1321VectorsAllMatch()
    {
        Assert.Equal("d41d8cd98f00b204e9800998ecf8427e", Hex(""));
        Assert.Equal("0cc175b9c0f1b6a831c399e269772661", Hex("a"));
        Assert.Equal("900150983cd24fb0d6963f7d28e17f72", Hex("abc"));
        Assert.Equal("f96b697d7cb7938d525a2f31aaf161d0", Hex("message digest"));
        Assert.Equal("c3fcd3d76192e4007dfb496cca67e13b", Hex("abcdefghijklmnopqrstuvwxyz"));
        Assert.Equal(
            "d174ab98d277d9f5a5611c2c9f419d9f",
            Hex("ABCDEFGHIJKLMNOPQRSTUVWXYZabcdefghijklmnopqrstuvwxyz0123456789"));
        Assert.Equal(
            "57edf4a22be3c955ac49da2e2107b67a",
            Hex("12345678901234567890123456789012345678901234567890123456789012345678901234567890"));
    }

    [Fact]
    public void KnownDigestsAndLittleEndianChecksMatch()
    {
        Assert.Equal("9e107d9d372bb6826bd81d3542a419d6", Hex("The quick brown fox jumps over the lazy dog"));
        Assert.Equal("e4d909c290d0fb1ca068ffaddf22cbd0", Hex("The quick brown fox jumps over the lazy dog."));
        Assert.Equal("93b885adfe0da089cdf634904fd59f71", Md5Type.HexString([0x00]));
        Assert.Equal("00594fd4f42ba43fc1ca0427a0576295", Md5Type.HexString([0xff]));

        var emptyDigest = Md5Type.SumMd5(Array.Empty<byte>());
        Assert.Equal(16, emptyDigest.Length);
        Assert.Equal(0xd4, emptyDigest[0]);
        Assert.Equal(0x1d, emptyDigest[1]);
    }

    [Fact]
    public void OneShotDigestAlwaysReturnsSixteenBytesAndLowercaseHex()
    {
        Assert.Equal(16, Md5Type.SumMd5(Encode("")).Length);
        Assert.Equal(16, Md5Type.SumMd5(Encode("abc")).Length);
        Assert.Equal(16, Md5Type.SumMd5(new byte[1000]).Length);

        var hex = Md5Type.HexString(Encode("hello world"));
        Assert.Equal(32, hex.Length);
        Assert.Matches("^[0-9a-f]{32}$", hex);
        Assert.Equal(hex, Md5Type.ToHex(Md5Type.SumMd5(Encode("hello world"))));
    }

    [Fact]
    public void BlockBoundaryLengthsRemainStable()
    {
        foreach (var length in new[] { 55, 56, 63, 64, 65, 128 })
        {
            var data = Enumerable.Repeat((byte)'a', length).ToArray();
            var oneShot = Md5Type.HexString(data);

            var hasher = new Md5HasherType();
            hasher.Update(data[..Math.Min(13, data.Length)]);
            hasher.Update(data[Math.Min(13, data.Length)..]);

            Assert.Equal(oneShot, hasher.HexDigest());
        }
    }

    [Fact]
    public void StreamingHasherMatchesOneShotAcrossArbitraryChunks()
    {
        var data = Enumerable.Range(0, 256).Select(value => (byte)value).ToArray();
        var expected = Md5Type.HexString(data);

        var hasher = new Md5HasherType();
        hasher.Update(data[..7]);
        hasher.Update(data[7..111]);
        hasher.Update(data[111..192]);
        hasher.Update(data[192..]);

        Assert.Equal(expected, hasher.HexDigest());
        Assert.Equal(expected, Md5Type.ToHex(hasher.Digest()));
    }

    [Fact]
    public void DigestIsNonDestructiveAndHasherCanContinueAfterDigesting()
    {
        var hasher = new Md5HasherType();
        hasher.Update(Encode("hello"));

        var first = hasher.HexDigest();
        Assert.Equal(first, hasher.HexDigest());
        Assert.Equal(Md5Type.HexString(Encode("hello")), first);

        hasher.Update(Encode(" world"));
        Assert.Equal(Md5Type.HexString(Encode("hello world")), hasher.HexDigest());
    }

    [Fact]
    public void CopyCreatesIndependentStreamingStates()
    {
        var original = new Md5HasherType();
        original.Update(Encode("ab"));

        var copy = original.Copy();
        copy.Update(Encode("c"));
        original.Update(Encode("x"));

        Assert.Equal(Md5Type.HexString(Encode("abc")), copy.HexDigest());
        Assert.Equal(Md5Type.HexString(Encode("abx")), original.HexDigest());
    }

    private static byte[] Encode(string value) => Encoding.UTF8.GetBytes(value);

    private static string Hex(string value) => Md5Type.HexString(Encode(value));
}
