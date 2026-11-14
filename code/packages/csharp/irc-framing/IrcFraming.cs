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

    /// <summary>
    /// The most bytes held for one line that has not yet seen its LF: 510 content bytes plus a CR.
    /// A legal line can be 510 bytes plus a CR whose LF has not arrived yet, i.e. 511 bytes.
    /// One more byte with no LF and the line can no longer be legal (SEC-011).
    /// </summary>
    public const int MaxPendingBytes = MaxContentBytes + 1;

    private readonly List<byte> _buffer = [];

    // Length of the unterminated tail of _buffer (bytes after the last LF).
    // Invariant: _tailLen <= MaxPendingBytes after every Feed.
    private int _tailLen;

    // True while we are skipping the rest of an overlong line: everything up
    // to and including the next LF is thrown away, unread.
    private bool _discarding;

    /// <summary>Number of bytes currently held as partial input.</summary>
    public int BufferSize => _buffer.Count;

    /// <summary>Append raw bytes to the internal buffer.</summary>
    public void Feed(byte[] data)
    {
        ArgumentNullException.ThrowIfNull(data);
        Feed(data.AsSpan());
    }

    /// <summary>
    /// Append raw bytes to the internal buffer.
    /// <para>
    /// Memory bound (SEC-011, CWE-400/770): the unterminated tail (bytes after the last LF) never
    /// exceeds <see cref="MaxPendingBytes"/>. When it would, the partial line is dropped and the
    /// framer discards input up to the next LF: fail closed, the same silent discard that already
    /// applies to a complete overlong line, so the API is unchanged. Complete lines already
    /// buffered are kept; the caller bounds those by calling <see cref="Frames"/> after each read.
    /// </para>
    /// </summary>
    public void Feed(ReadOnlySpan<byte> data)
    {
        // Step 1: while discarding an overlong line, skip to just past its LF.
        if (_discarding)
        {
            var pos = data.IndexOf((byte)'\n');
            if (pos < 0)
            {
                return; // still inside the overlong line
            }

            _discarding = false;
            data = data[(pos + 1)..];
        }

        // Step 2: split data at its last LF. The head (through that LF) completes lines and is
        // stored; only the tail can be flooded.
        //
        //   data = [ ...complete lines... LF ][ unterminated tail ]
        //                                     ^ only this part can be flooded
        var head = data[..(data.LastIndexOf((byte)'\n') + 1)];
        var tail = data[head.Length..];
        if (head.Length > 0)
        {
            foreach (var b in head)
            {
                _buffer.Add(b);
            }

            _tailLen = 0;
        }

        // Step 3: store the tail only if the pending line still fits. We check *before* copying
        // so a huge single feed is never copied.
        if (tail.Length > MaxPendingBytes - _tailLen)
        {
            _buffer.RemoveRange(_buffer.Count - _tailLen, _tailLen); // drop the partial line
            _tailLen = 0;
            _discarding = true;
        }
        else
        {
            foreach (var b in tail)
            {
                _buffer.Add(b);
            }

            _tailLen += tail.Length;
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

        // Only the unterminated tail remains in the buffer.
        _tailLen = _buffer.Count;
        return result;
    }

    /// <summary>Discard all buffered partial input, including the skip-to-LF discard state.</summary>
    public void Reset()
    {
        _buffer.Clear();
        _tailLen = 0;
        _discarding = false;
    }
}
