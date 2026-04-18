namespace CodingAdventures.PixelContainer;

/// <summary>
/// Package-level entry points for IC00. The plural name avoids colliding with
/// the <see cref="PixelContainer"/> type that owns the actual pixel buffer.
/// </summary>
public static class PixelContainers
{
    public const string VERSION = "0.1.0";

    public static PixelContainer Create(int width, int height) => new(width, height);

    public static PixelContainer FromData(int width, int height, byte[] data) => new(width, height, data);
}

/// <summary>
/// Four 8-bit colour channels in RGBA order.
/// </summary>
public readonly record struct Rgba(byte R, byte G, byte B, byte A);

/// <summary>
/// A fixed-format RGBA8 pixel buffer with row-major layout and a top-left origin.
///
/// The offset math is the same in every language port:
///
///   offset = (y * width + x) * 4
///
/// That consistency matters because codecs, renderers, and tests all need to
/// agree on where one pixel ends and the next begins.
/// </summary>
public sealed class PixelContainer
{
    public PixelContainer(int width, int height)
        : this(width, height, new byte[CheckedBufferLength(width, height)])
    {
    }

    public PixelContainer(int width, int height, byte[] data)
    {
        if (width < 0)
        {
            throw new ArgumentOutOfRangeException(nameof(width), "width must be non-negative");
        }

        if (height < 0)
        {
            throw new ArgumentOutOfRangeException(nameof(height), "height must be non-negative");
        }

        ArgumentNullException.ThrowIfNull(data);

        var expectedLength = CheckedBufferLength(width, height);
        if (data.Length != expectedLength)
        {
            throw new ArgumentException($"data length must be width * height * 4 ({expectedLength})", nameof(data));
        }

        Width = width;
        Height = height;
        Data = data;
    }

    public int Width { get; }

    public int Height { get; }

    public byte[] Data { get; }

    public Rgba GetPixel(int x, int y)
    {
        if (x < 0 || x >= Width || y < 0 || y >= Height)
        {
            return default;
        }

        var index = ((y * Width) + x) * 4;
        return new Rgba(Data[index], Data[index + 1], Data[index + 2], Data[index + 3]);
    }

    public void SetPixel(int x, int y, byte r, byte g, byte b, byte a)
    {
        if (x < 0 || x >= Width || y < 0 || y >= Height)
        {
            return;
        }

        var index = ((y * Width) + x) * 4;
        Data[index] = r;
        Data[index + 1] = g;
        Data[index + 2] = b;
        Data[index + 3] = a;
    }

    public void SetPixel(int x, int y, Rgba rgba) => SetPixel(x, y, rgba.R, rgba.G, rgba.B, rgba.A);

    public void Fill(byte r, byte g, byte b, byte a)
    {
        for (var index = 0; index < Data.Length; index += 4)
        {
            Data[index] = r;
            Data[index + 1] = g;
            Data[index + 2] = b;
            Data[index + 3] = a;
        }
    }

    public void Fill(Rgba rgba) => Fill(rgba.R, rgba.G, rgba.B, rgba.A);

    private static int CheckedBufferLength(int width, int height) => checked(width * height * 4);
}

/// <summary>
/// The codec contract that lets image formats speak in raw pixels without
/// importing any of the higher-level paint abstractions.
/// </summary>
public interface IImageCodec
{
    string MimeType { get; }

    byte[] Encode(PixelContainer pixels);

    PixelContainer Decode(byte[] bytes);
}
