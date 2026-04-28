using CodingAdventures.WriteAheadLog;

namespace CodingAdventures.WriteAheadLog.Tests;

public sealed class WriteAheadLogTests
{
    [Fact]
    public void VersionExists()
    {
        Assert.Equal("0.1.0", WriteAheadLogPackage.Version);
    }

    [Fact]
    public void AppendRecordWritesLengthPrefixedPayloads()
    {
        var path = TempPath();
        try
        {
            using (var writer = new WalWriter(path))
            {
                writer.AppendRecord([1, 2, 3]);
                writer.AppendRecord([4, 5]);
            }

            Assert.Equal([3, 0, 0, 0, 1, 2, 3, 2, 0, 0, 0, 4, 5], File.ReadAllBytes(path));
        }
        finally
        {
            DeleteIfExists(path);
        }
    }

    [Fact]
    public void ReaderReturnsRecordsInOrderThenNullAtEnd()
    {
        var path = TempPath();
        try
        {
            using (var writer = new WalWriter(path))
            {
                writer.AppendRecord([10, 20]);
                writer.AppendRecord([30]);
            }

            using var reader = new WalReader(path);

            Assert.Equal([10, 20], reader.ReadNext());
            Assert.Equal([30], reader.ReadNext());
            Assert.Null(reader.ReadNext());
        }
        finally
        {
            DeleteIfExists(path);
        }
    }

    [Fact]
    public void EmptyRecordsRoundTrip()
    {
        var path = TempPath();
        try
        {
            using (var writer = new WalWriter(path))
            {
                writer.AppendRecord([]);
            }

            using var reader = new WalReader(path);

            var record = reader.ReadNext();
            Assert.NotNull(record);
            Assert.Empty(record);
            Assert.Null(reader.ReadNext());
        }
        finally
        {
            DeleteIfExists(path);
        }
    }

    [Fact]
    public void WriterAppendsToExistingLog()
    {
        var path = TempPath();
        try
        {
            using (var writer = new WalWriter(path))
            {
                writer.AppendRecord([1]);
            }

            using (var writer = new WalWriter(path))
            {
                writer.AppendRecord([2, 3]);
            }

            using var reader = new WalReader(path);

            Assert.Equal([1], reader.ReadNext());
            Assert.Equal([2, 3], reader.ReadNext());
            Assert.Null(reader.ReadNext());
        }
        finally
        {
            DeleteIfExists(path);
        }
    }

    [Fact]
    public void PartialLengthPrefixIsTreatedAsEndOfFile()
    {
        var path = TempPath();
        try
        {
            File.WriteAllBytes(path, [2, 0]);

            using var reader = new WalReader(path);

            Assert.Null(reader.ReadNext());
        }
        finally
        {
            DeleteIfExists(path);
        }
    }

    [Fact]
    public void TruncatedPayloadThrows()
    {
        var path = TempPath();
        try
        {
            File.WriteAllBytes(path, [4, 0, 0, 0, 1, 2]);

            using var reader = new WalReader(path);

            Assert.Throws<EndOfStreamException>(() => reader.ReadNext());
        }
        finally
        {
            DeleteIfExists(path);
        }
    }

    [Fact]
    public void InvalidArgumentsAreRejected()
    {
        Assert.Throws<ArgumentException>(() => new WalWriter(""));
        Assert.Throws<ArgumentException>(() => new WalReader(""));
        Assert.Throws<FileNotFoundException>(() => new WalReader(Path.Combine(Path.GetTempPath(), Guid.NewGuid().ToString("N"))));
    }

    private static string TempPath()
    {
        return Path.Combine(Path.GetTempPath(), $"{Guid.NewGuid():N}.wal");
    }

    private static void DeleteIfExists(string path)
    {
        if (File.Exists(path))
        {
            File.Delete(path);
        }
    }
}
