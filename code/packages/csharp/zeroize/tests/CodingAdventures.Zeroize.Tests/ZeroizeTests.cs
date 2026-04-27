using ZeroizeAlgorithm = CodingAdventures.Zeroize.Zeroize;

namespace CodingAdventures.Zeroize.Tests;

public sealed class ZeroizeTests
{
    [Fact]
    public void ZeroizeBytesClearsBuffer()
    {
        var secret = new byte[] { 1, 2, 3, 4 };

        ZeroizeAlgorithm.ZeroizeBytes(secret);

        Assert.All(secret, value => Assert.Equal(0, value));
    }

    [Fact]
    public void ZeroizeCharsAndArraysClearBuffers()
    {
        var chars = "secret".ToCharArray();
        var numbers = new[] { 1, 2, 3 };

        ZeroizeAlgorithm.ZeroizeChars(chars);
        ZeroizeAlgorithm.ZeroizeArray(numbers);

        Assert.All(chars, value => Assert.Equal('\0', value));
        Assert.All(numbers, value => Assert.Equal(0, value));
    }

    [Fact]
    public void DisposableBufferZeroizesOnDisposeAndIsIdempotent()
    {
        var secret = new byte[] { 9, 8, 7 };
        var wrapper = new ZeroizingBuffer(secret);

        wrapper.Dispose();
        wrapper.Dispose();

        Assert.Same(secret, wrapper.Buffer);
        Assert.All(secret, value => Assert.Equal(0, value));
    }

    [Fact]
    public void ValidationRejectsNull()
    {
        Assert.Throws<ArgumentNullException>(() => ZeroizeAlgorithm.ZeroizeBytes(null!));
        Assert.Throws<ArgumentNullException>(() => ZeroizeAlgorithm.ZeroizeChars(null!));
        Assert.Throws<ArgumentNullException>(() => ZeroizeAlgorithm.ZeroizeArray<int>(null!));
        Assert.Throws<ArgumentNullException>(() => new ZeroizingBuffer(null!));
    }
}
