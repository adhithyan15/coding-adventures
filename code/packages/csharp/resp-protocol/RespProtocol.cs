using System.Text;

namespace CodingAdventures.RespProtocol;

public abstract class RespValue
{
    protected RespValue(string kind)
    {
        Kind = kind;
    }

    public string Kind { get; }
}

public sealed class RespSimpleString : RespValue
{
    public RespSimpleString(string value) : base("simple-string") => Value = value;
    public string Value { get; }
}

public sealed class RespErrorValue : RespValue
{
    public RespErrorValue(string value) : base("error") => Value = value;
    public string Value { get; }
}

public sealed class RespInteger : RespValue
{
    public RespInteger(long value) : base("integer") => Value = value;
    public long Value { get; }
}

public sealed class RespBulkString : RespValue
{
    public RespBulkString(string? value) : base("bulk-string") => Value = value;
    public string? Value { get; }
}

public sealed class RespArray : RespValue
{
    public RespArray(IReadOnlyList<RespValue>? value) : base("array") => Value = value;
    public IReadOnlyList<RespValue>? Value { get; }
}

public sealed class RespDecodeResult
{
    public RespDecodeResult(RespValue value, int consumed)
    {
        Value = value;
        Consumed = consumed;
    }

    public RespValue Value { get; }
    public int Consumed { get; }
}

public sealed class RespDecodeAllResult
{
    public RespDecodeAllResult(IReadOnlyList<RespValue> values, int consumed)
    {
        Values = values;
        Consumed = consumed;
    }

    public IReadOnlyList<RespValue> Values { get; }
    public int Consumed { get; }
}

public sealed class RespDecodeError : Exception
{
    public RespDecodeError(string message) : base(message)
    {
    }
}

public sealed class RespEncodeError : Exception
{
    public RespEncodeError(string message) : base(message)
    {
    }
}

public sealed class RespDecoder
{
    private byte[] _buffer = [];
    private readonly Queue<RespValue> _queue = new();
    private RespDecodeError? _error;

    public void Feed(string data) => Feed(Encoding.UTF8.GetBytes(data));

    public void Feed(byte[] data)
    {
        _buffer = RespProtocol.ConcatBytes([_buffer, data]);
        Drain();
    }

    public bool HasMessage() => _queue.Count > 0;

    public RespValue GetMessage()
    {
        if (_error is not null)
        {
            throw _error;
        }

        return _queue.Count == 0
            ? throw new RespDecodeError("decoder buffer is empty")
            : _queue.Dequeue();
    }

    private void Drain()
    {
        if (_error is not null)
        {
            return;
        }

        while (_buffer.Length > 0)
        {
            var result = RespProtocol.Decode(_buffer);
            if (result is null)
            {
                return;
            }

            _queue.Enqueue(result.Value);
            _buffer = _buffer[result.Consumed..];
        }
    }
}

public static class RespProtocol
{
    public static RespSimpleString SimpleString(string value) => new(value);
    public static RespErrorValue ErrorValue(string value) => new(value);
    public static RespInteger Integer(long value) => new(value);
    public static RespBulkString BulkString(string? value) => new(value);
    public static RespArray Array(IReadOnlyList<RespValue>? value) => new(value);

    public static byte[] Encode(RespValue value) =>
        value switch
        {
            RespSimpleString simple => EncodeSimpleString(simple.Value),
            RespErrorValue error => EncodeError(error.Value),
            RespInteger integer => EncodeInteger(integer.Value),
            RespBulkString bulk => EncodeBulkString(bulk.Value),
            RespArray array => EncodeArray(array.Value),
            _ => throw new RespEncodeError($"Unknown RESP value type: {value.GetType().Name}")
        };

    public static byte[] EncodeSimpleString(string value) => Encoding.UTF8.GetBytes($"+{value}\r\n");

    public static byte[] EncodeError(string value) => Encoding.UTF8.GetBytes($"-{value}\r\n");

    public static byte[] EncodeInteger(long value) => Encoding.UTF8.GetBytes($":{value}\r\n");

    public static byte[] EncodeBulkString(string? value)
    {
        if (value is null)
        {
            return Encoding.UTF8.GetBytes("$-1\r\n");
        }

        var bytes = Encoding.UTF8.GetBytes(value);
        return ConcatBytes([Encoding.UTF8.GetBytes($"${bytes.Length}\r\n"), bytes, "\r\n"u8.ToArray()]);
    }

    public static byte[] EncodeArray(IReadOnlyList<RespValue>? values)
    {
        if (values is null)
        {
            return Encoding.UTF8.GetBytes("*-1\r\n");
        }

        var parts = new List<byte[]> { Encoding.UTF8.GetBytes($"*{values.Count}\r\n") };
        parts.AddRange(values.Select(Encode));
        return ConcatBytes(parts);
    }

    public static RespDecodeResult? Decode(string input) => Decode(Encoding.UTF8.GetBytes(input));

    public static RespDecodeResult? Decode(byte[] buffer)
    {
        if (buffer.Length == 0)
        {
            return null;
        }

        return buffer[0] switch
        {
            (byte)'+' => DecodeSimpleString(buffer),
            (byte)'-' => DecodeError(buffer),
            (byte)':' => DecodeInteger(buffer),
            (byte)'$' => DecodeBulkString(buffer),
            (byte)'*' => DecodeArray(buffer),
            _ => DecodeInlineCommand(buffer)
        };
    }

    public static RespDecodeAllResult DecodeAll(string input) => DecodeAll(Encoding.UTF8.GetBytes(input));

    public static RespDecodeAllResult DecodeAll(byte[] buffer)
    {
        var values = new List<RespValue>();
        var offset = 0;
        while (offset < buffer.Length)
        {
            var result = Decode(buffer[offset..]);
            if (result is null)
            {
                break;
            }

            values.Add(result.Value);
            offset += result.Consumed;
        }

        return new RespDecodeAllResult(values, offset);
    }

    public static byte[] ConcatBytes(IEnumerable<byte[]> chunks)
    {
        var arrays = chunks.ToList();
        var length = arrays.Sum(chunk => chunk.Length);
        var result = new byte[length];
        var offset = 0;
        foreach (var chunk in arrays)
        {
            Buffer.BlockCopy(chunk, 0, result, offset, chunk.Length);
            offset += chunk.Length;
        }

        return result;
    }

    private static RespDecodeResult? DecodeSimpleString(byte[] buffer)
    {
        var line = ReadLine(buffer, 1);
        return line is null
            ? null
            : new RespDecodeResult(SimpleString(Encoding.UTF8.GetString(line.Value.Line)), line.Value.Consumed);
    }

    private static RespDecodeResult? DecodeError(byte[] buffer)
    {
        var line = ReadLine(buffer, 1);
        return line is null
            ? null
            : new RespDecodeResult(ErrorValue(Encoding.UTF8.GetString(line.Value.Line)), line.Value.Consumed);
    }

    private static RespDecodeResult? DecodeInteger(byte[] buffer)
    {
        var line = ReadLine(buffer, 1);
        if (line is null)
        {
            return null;
        }

        if (!long.TryParse(Encoding.UTF8.GetString(line.Value.Line), out var value))
        {
            throw new RespDecodeError("invalid RESP integer");
        }

        return new RespDecodeResult(Integer(value), line.Value.Consumed);
    }

    private static RespDecodeResult? DecodeBulkString(byte[] buffer)
    {
        var line = ReadLine(buffer, 1);
        if (line is null)
        {
            return null;
        }

        if (!int.TryParse(Encoding.UTF8.GetString(line.Value.Line), out var length))
        {
            throw new RespDecodeError("invalid RESP bulk string length");
        }

        if (length == -1)
        {
            return new RespDecodeResult(BulkString(null), line.Value.Consumed);
        }

        if (length < -1)
        {
            throw new RespDecodeError("bulk string length cannot be negative");
        }

        var bodyStart = line.Value.Consumed;
        var bodyEnd = bodyStart + length;
        var tailEnd = bodyEnd + 2;
        if (buffer.Length < tailEnd)
        {
            return null;
        }

        if (buffer[bodyEnd] != '\r' || buffer[bodyEnd + 1] != '\n')
        {
            throw new RespDecodeError("missing trailing CRLF after bulk string body");
        }

        return new RespDecodeResult(BulkString(Encoding.UTF8.GetString(buffer[bodyStart..bodyEnd])), tailEnd);
    }

    private static RespDecodeResult? DecodeArray(byte[] buffer)
    {
        var line = ReadLine(buffer, 1);
        if (line is null)
        {
            return null;
        }

        if (!int.TryParse(Encoding.UTF8.GetString(line.Value.Line), out var count))
        {
            throw new RespDecodeError("invalid RESP array length");
        }

        if (count == -1)
        {
            return new RespDecodeResult(Array(null), line.Value.Consumed);
        }

        if (count < -1)
        {
            throw new RespDecodeError("array length cannot be negative");
        }

        var values = new List<RespValue>();
        var offset = line.Value.Consumed;
        for (var i = 0; i < count; i++)
        {
            var result = Decode(buffer[offset..]);
            if (result is null)
            {
                return null;
            }

            values.Add(result.Value);
            offset += result.Consumed;
        }

        return new RespDecodeResult(Array(values), offset);
    }

    private static RespDecodeResult? DecodeInlineCommand(byte[] buffer)
    {
        var line = ReadLine(buffer, 0);
        if (line is null)
        {
            return null;
        }

        var text = Encoding.UTF8.GetString(line.Value.Line).Trim();
        var tokens = string.IsNullOrEmpty(text)
            ? []
            : text.Split((char[]?)null, StringSplitOptions.RemoveEmptyEntries)
                .Select(token => (RespValue)BulkString(token))
                .ToList();

        return new RespDecodeResult(Array(tokens), line.Value.Consumed);
    }

    private static (byte[] Line, int Consumed)? ReadLine(byte[] buffer, int start)
    {
        for (var i = start; i < buffer.Length - 1; i++)
        {
            if (buffer[i] == '\r' && buffer[i + 1] == '\n')
            {
                return (buffer[start..i], i + 2);
            }
        }

        return null;
    }
}
