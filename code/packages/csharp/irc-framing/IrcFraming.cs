namespace CodingAdventures.IrcFraming;

/// <summary>Stable IRC framing package version.</summary>
public static class IrcFraming
{
    /// <summary>Package semantic version.</summary>
    public const string Version = "0.1.0";
}

/// <summary>Stateful byte-stream-to-line-frame converter for IRC CRLF framing.</summary>
public sealed class Framer
{
    /// <summary>RFC 1459 maximum message content length, excluding CRLF.</summary>
    public const int MaxContentBytes = 510;

    private readonly List<byte> _buffer = [];

    /// <summary>Number of bytes currently held as partial input.</summary>
    public int BufferSize => _buffer.Count;

    /// <summary>Append raw bytes to the internal buffer.</summary>
    public void Feed(byte[] data)
    {
        ArgumentNullException.ThrowIfNull(data);
        Feed(data.AsSpan());
    }

    /// <summary>Append raw bytes to the internal buffer.</summary>
    public void Feed(ReadOnlySpan<byte> data)
    {
        foreach (var b in data)
        {
            _buffer.Add(b);
        }
    }

    /// <summary>Drain all complete frames from the buffer, stripping LF and an optional preceding CR.</summary>
    public IReadOnlyList<byte[]> Frames()
    {
        var result = new List<byte[]>();

        while (true)
        {
            var lfPos = _buffer.IndexOf((byte)'\n');
            if (lfPos < 0)
            {
                break;
            }

            var contentEnd = lfPos > 0 && _buffer[lfPos - 1] == (byte)'\r'
                ? lfPos - 1
                : lfPos;

            var line = _buffer.GetRange(0, contentEnd).ToArray();
            _buffer.RemoveRange(0, lfPos + 1);

            if (line.Length <= MaxContentBytes)
            {
                result.Add(line);
            }
        }

        return result;
    }

    /// <summary>Discard all buffered partial input.</summary>
    public void Reset() => _buffer.Clear();
}
