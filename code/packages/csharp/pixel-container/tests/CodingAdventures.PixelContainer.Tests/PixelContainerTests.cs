using CodingAdventures.PixelContainer;

namespace CodingAdventures.PixelContainer.Tests;

public sealed class PixelContainerTests
{
    [Fact]
    public void Version_IsSemver()
    {
        Assert.Equal("0.1.0", PixelContainers.VERSION);
    }

    [Fact]
    public void Create_AllocatesWidthTimesHeightTimesFourBytes()
    {
        var pixels = PixelContainers.Create(4, 3);

        Assert.Equal(4, pixels.Width);
        Assert.Equal(3, pixels.Height);
        Assert.Equal(48, pixels.Data.Length);
    }

    [Fact]
    public void Constructor_WithData_RequiresExactLength()
    {
        var error = Assert.Throws<ArgumentException>(() => new PixelContainer(2, 2, new byte[15]));

        Assert.Contains("width * height * 4", error.Message);
    }

    [Fact]
    public void NewContainer_StartsTransparentBlack()
    {
        var pixels = PixelContainers.Create(2, 2);

        Assert.All(pixels.Data, value => Assert.Equal((byte)0, value));
        Assert.Equal(new Rgba(0, 0, 0, 0), pixels.GetPixel(1, 1));
    }

    [Fact]
    public void GetPixel_UsesRowMajorOffsets()
    {
        var pixels = PixelContainers.Create(3, 2);
        pixels.Data[20] = 11;
        pixels.Data[21] = 22;
        pixels.Data[22] = 33;
        pixels.Data[23] = 44;

        Assert.Equal(new Rgba(11, 22, 33, 44), pixels.GetPixel(2, 1));
    }

    [Fact]
    public void GetPixel_ReturnsTransparentBlackWhenOutOfBounds()
    {
        var pixels = PixelContainers.Create(3, 3);

        Assert.Equal(default, pixels.GetPixel(-1, 0));
        Assert.Equal(default, pixels.GetPixel(3, 0));
        Assert.Equal(default, pixels.GetPixel(0, 3));
    }

    [Fact]
    public void SetPixel_WritesRGBAValues()
    {
        var pixels = PixelContainers.Create(2, 2);
        pixels.SetPixel(1, 0, 200, 100, 50, 255);

        Assert.Equal(new Rgba(200, 100, 50, 255), pixels.GetPixel(1, 0));
    }

    [Fact]
    public void SetPixel_IsNoOpWhenOutOfBounds()
    {
        var pixels = PixelContainers.Create(2, 2);
        pixels.SetPixel(50, 50, 1, 2, 3, 4);

        Assert.All(pixels.Data, value => Assert.Equal((byte)0, value));
    }

    [Fact]
    public void Fill_OverwritesWholeBuffer()
    {
        var pixels = PixelContainers.Create(3, 2);
        pixels.SetPixel(0, 0, 1, 2, 3, 4);
        pixels.Fill(100, 150, 200, 255);

        for (var y = 0; y < pixels.Height; y++)
        {
            for (var x = 0; x < pixels.Width; x++)
            {
                Assert.Equal(new Rgba(100, 150, 200, 255), pixels.GetPixel(x, y));
            }
        }
    }

    [Fact]
    public void ImageCodec_CanBeImplementedByPlainObject()
    {
        var codec = new StubCodec();
        var pixels = PixelContainers.Create(3, 2);
        var encoded = codec.Encode(pixels);
        var decoded = codec.Decode(encoded);

        Assert.Equal("image/test", codec.MimeType);
        Assert.Equal(new byte[] { 3, 2 }, encoded);
        Assert.Equal(3, decoded.Width);
        Assert.Equal(2, decoded.Height);
    }

    private sealed class StubCodec : IImageCodec
    {
        public string MimeType => "image/test";

        public byte[] Encode(PixelContainer pixels) => [(byte)pixels.Width, (byte)pixels.Height];

        public PixelContainer Decode(byte[] bytes) => PixelContainers.Create(bytes[0], bytes[1]);
    }
}
