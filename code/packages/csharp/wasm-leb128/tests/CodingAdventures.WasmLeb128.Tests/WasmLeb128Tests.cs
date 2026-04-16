using System;
using CodingAdventures.WasmLeb128;
using Xunit;

namespace CodingAdventures.WasmLeb128.Tests;

public class WasmLeb128Tests
{
    [Fact]
    public void HasVersion()
    {
        Assert.Equal("0.1.0", WasmLeb128Version.VERSION);
    }

    [Fact]
    public void ErrorIsExceptionSubclass()
    {
        var error = new LEB128Error("test");
        Assert.IsType<LEB128Error>(error);
        Assert.Equal("test", error.Message);
    }

    [Fact]
    public void DecodeUnsignedHandlesCommonVectors()
    {
        Assert.Equal((0u, 1), WasmLeb128.DecodeUnsigned(new byte[] { 0x00 }));
        Assert.Equal((3u, 1), WasmLeb128.DecodeUnsigned(new byte[] { 0x03 }));
        Assert.Equal((624485u, 3), WasmLeb128.DecodeUnsigned(new byte[] { 0xE5, 0x8E, 0x26 }));
        Assert.Equal((uint.MaxValue, 5), WasmLeb128.DecodeUnsigned(new byte[] { 0xFF, 0xFF, 0xFF, 0xFF, 0x0F }));
    }

    [Fact]
    public void DecodeUnsignedSupportsOffsets()
    {
        var data = new byte[] { 0xAA, 0xE5, 0x8E, 0x26, 0xBB };
        Assert.Equal((624485u, 3), WasmLeb128.DecodeUnsigned(data, 1));
    }

    [Fact]
    public void DecodeUnsignedRejectsInvalidSequences()
    {
        Assert.Throws<LEB128Error>(() => WasmLeb128.DecodeUnsigned(Array.Empty<byte>()));
        Assert.Throws<LEB128Error>(() => WasmLeb128.DecodeUnsigned(new byte[] { 0x80, 0x80 }));
        Assert.Throws<LEB128Error>(() => WasmLeb128.DecodeUnsigned(new byte[] { 0x80, 0x80, 0x80, 0x80, 0x80, 0x01 }));
    }

    [Fact]
    public void DecodeSignedHandlesCommonVectors()
    {
        Assert.Equal((0, 1), WasmLeb128.DecodeSigned(new byte[] { 0x00 }));
        Assert.Equal((-2, 1), WasmLeb128.DecodeSigned(new byte[] { 0x7E }));
        Assert.Equal((int.MaxValue, 5), WasmLeb128.DecodeSigned(new byte[] { 0xFF, 0xFF, 0xFF, 0xFF, 0x07 }));
        Assert.Equal((int.MinValue, 5), WasmLeb128.DecodeSigned(new byte[] { 0x80, 0x80, 0x80, 0x80, 0x78 }));
    }

    [Fact]
    public void DecodeSignedSupportsOffsetsAndErrors()
    {
        Assert.Equal((-2, 1), WasmLeb128.DecodeSigned(new byte[] { 0xFF, 0x7E, 0x00 }, 1));
        Assert.Throws<LEB128Error>(() => WasmLeb128.DecodeSigned(new byte[] { 0x80, 0x80 }));
    }

    [Fact]
    public void EncodeUnsignedHandlesCommonVectors()
    {
        Assert.Equal(new byte[] { 0x00 }, WasmLeb128.EncodeUnsigned(0));
        Assert.Equal(new byte[] { 0x03 }, WasmLeb128.EncodeUnsigned(3));
        Assert.Equal(new byte[] { 0xE5, 0x8E, 0x26 }, WasmLeb128.EncodeUnsigned(624485));
        Assert.Equal(new byte[] { 0xFF, 0xFF, 0xFF, 0xFF, 0x0F }, WasmLeb128.EncodeUnsigned(uint.MaxValue));
    }

    [Fact]
    public void EncodeSignedHandlesCommonVectors()
    {
        Assert.Equal(new byte[] { 0x00 }, WasmLeb128.EncodeSigned(0));
        Assert.Equal(new byte[] { 0x03 }, WasmLeb128.EncodeSigned(3));
        Assert.Equal(new byte[] { 0x7E }, WasmLeb128.EncodeSigned(-2));
        Assert.Equal(new byte[] { 0xFF, 0xFF, 0xFF, 0xFF, 0x07 }, WasmLeb128.EncodeSigned(int.MaxValue));
        Assert.Equal(new byte[] { 0x80, 0x80, 0x80, 0x80, 0x78 }, WasmLeb128.EncodeSigned(int.MinValue));
    }

    [Fact]
    public void RoundTripsUnsignedValues()
    {
        uint[] values = [0u, 1u, 63u, 64u, 127u, 128u, 255u, 256u, 16383u, 16384u, 624485u, 1000000u, 0x7FFFFFFFu, uint.MaxValue];
        foreach (var value in values)
        {
            var encoded = WasmLeb128.EncodeUnsigned(value);
            var (decoded, bytesConsumed) = WasmLeb128.DecodeUnsigned(encoded);
            Assert.Equal(value, decoded);
            Assert.Equal(encoded.Length, bytesConsumed);
        }
    }

    [Fact]
    public void RoundTripsSignedValues()
    {
        int[] values = [0, 1, -1, 63, -64, 64, -65, 127, -128, 128, -129, int.MaxValue, int.MinValue, -1000000, -2];
        foreach (var value in values)
        {
            var encoded = WasmLeb128.EncodeSigned(value);
            var (decoded, bytesConsumed) = WasmLeb128.DecodeSigned(encoded);
            Assert.Equal(value, decoded);
            Assert.Equal(encoded.Length, bytesConsumed);
        }
    }
}
