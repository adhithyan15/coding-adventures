namespace CodingAdventures.WriteAheadLog;

using System.Buffers.Binary;

public static class WriteAheadLogPackage
{
    public const string Version = "0.1.0";
}

public sealed class WalWriter : IDisposable
{
    private readonly FileStream _file;

    public WalWriter(string path)
    {
        ArgumentException.ThrowIfNullOrWhiteSpace(path);
        _file = new FileStream(path, FileMode.Append, FileAccess.Write, FileShare.Read, 4096, FileOptions.WriteThrough);
    }

    public void AppendRecord(ReadOnlySpan<byte> data)
    {
        Span<byte> lengthBytes = stackalloc byte[4];
        BinaryPrimitives.WriteUInt32LittleEndian(lengthBytes, checked((uint)data.Length));
        _file.Write(lengthBytes);
        _file.Write(data);
        _file.Flush(flushToDisk: true);
    }

    public void Dispose()
    {
        _file.Dispose();
    }
}

public sealed class WalReader : IDisposable
{
    private readonly FileStream _file;

    public WalReader(string path)
    {
        ArgumentException.ThrowIfNullOrWhiteSpace(path);
        _file = new FileStream(path, FileMode.Open, FileAccess.Read, FileShare.ReadWrite);
    }

    public byte[]? ReadNext()
    {
        Span<byte> lengthBytes = stackalloc byte[4];
        if (!TryReadLength(lengthBytes))
        {
            return null;
        }

        var length = BinaryPrimitives.ReadUInt32LittleEndian(lengthBytes);
        if (length > int.MaxValue)
        {
            throw new InvalidDataException("WAL record is too large for this runtime.");
        }

        var record = new byte[length];
        ReadExactly(record);
        return record;
    }

    public void Dispose()
    {
        _file.Dispose();
    }

    private bool TryReadLength(Span<byte> buffer)
    {
        var total = 0;
        while (total < buffer.Length)
        {
            var read = _file.Read(buffer[total..]);
            if (read == 0)
            {
                return false;
            }

            total += read;
        }

        return true;
    }

    private void ReadExactly(Span<byte> buffer)
    {
        var total = 0;
        while (total < buffer.Length)
        {
            var read = _file.Read(buffer[total..]);
            if (read == 0)
            {
                throw new EndOfStreamException("WAL record ended before its length-prefixed payload was complete.");
            }

            total += read;
        }
    }
}
