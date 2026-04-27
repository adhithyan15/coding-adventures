using CsprngAlgorithm = CodingAdventures.Csprng.Csprng;

namespace CodingAdventures.Csprng.Tests;

public sealed class CsprngTests
{
    [Fact]
    public void RandomBytesReturnsRequestedLengthAndEntropy()
    {
        var bytes = CsprngAlgorithm.RandomBytes(32);

        Assert.Equal(32, bytes.Length);
        Assert.Contains(bytes, value => value != 0);
    }

    [Fact]
    public void FillRandomMutatesBuffer()
    {
        var buffer = new byte[32];

        CsprngAlgorithm.FillRandom(buffer);

        Assert.Contains(buffer, value => value != 0);
    }

    [Fact]
    public void RandomIntegersReturnValues()
    {
        _ = CsprngAlgorithm.RandomUInt32();
        _ = CsprngAlgorithm.RandomUInt64();
    }

    [Fact]
    public void ValidationRejectsInvalidRequests()
    {
        Assert.Throws<ArgumentNullException>(() => CsprngAlgorithm.FillRandom(null!));
        Assert.Throws<ArgumentException>(() => CsprngAlgorithm.FillRandom([]));
        Assert.Throws<ArgumentOutOfRangeException>(() => CsprngAlgorithm.RandomBytes(0));
        Assert.Throws<ArgumentOutOfRangeException>(() => CsprngAlgorithm.RandomBytes(-1));
    }
}
