using System.Globalization;
using System.Text;
using CodingAdventures.HttpCore;

namespace CodingAdventures.Http1;

public static class Http1
{
    public const string Version = "0.1.0";

    public static ParsedRequestHead ParseRequestHead(ReadOnlySpan<byte> input)
    {
        var (lines, bodyOffset) = SplitHeadLines(input);
        if (lines.Count == 0)
        {
            throw Http1ParseException.InvalidStartLine(string.Empty);
        }

        var startLine = lines[0];
        var parts = SplitWhitespace(startLine);
        if (parts.Length != 3)
        {
            throw Http1ParseException.InvalidStartLine(startLine);
        }

        HttpVersion version;
        try
        {
            version = HttpVersion.Parse(parts[2]);
        }
        catch (FormatException)
        {
            throw Http1ParseException.InvalidVersion(parts[2]);
        }

        var headers = ParseHeaders(lines.Skip(1));
        var bodyKind = RequestBodyKind(headers);

        return new ParsedRequestHead(
            new RequestHead(parts[0], parts[1], version, headers),
            bodyOffset,
            bodyKind);
    }

    public static ParsedResponseHead ParseResponseHead(ReadOnlySpan<byte> input)
    {
        var (lines, bodyOffset) = SplitHeadLines(input);
        if (lines.Count == 0)
        {
            throw Http1ParseException.InvalidStartLine(string.Empty);
        }

        var statusLine = lines[0];
        var parts = SplitWhitespace(statusLine);
        if (parts.Length < 2)
        {
            throw Http1ParseException.InvalidStartLine(statusLine);
        }

        HttpVersion version;
        try
        {
            version = HttpVersion.Parse(parts[0]);
        }
        catch (FormatException)
        {
            throw Http1ParseException.InvalidVersion(parts[0]);
        }

        if (!ushort.TryParse(parts[1], NumberStyles.None, CultureInfo.InvariantCulture, out var status))
        {
            throw Http1ParseException.InvalidStatus(parts[1]);
        }

        var reason = parts.Length > 2 ? string.Join(" ", parts.Skip(2)) : string.Empty;
        var headers = ParseHeaders(lines.Skip(1));
        var bodyKind = ResponseBodyKind(status, headers);

        return new ParsedResponseHead(
            new ResponseHead(version, status, reason, headers),
            bodyOffset,
            bodyKind);
    }

    private static (List<string> Lines, int BodyOffset) SplitHeadLines(ReadOnlySpan<byte> input)
    {
        var index = 0;
        while (index < input.Length)
        {
            if (input[index..].StartsWith("\r\n"u8))
            {
                index += 2;
            }
            else if (input[index] == (byte)'\n')
            {
                index += 1;
            }
            else
            {
                break;
            }
        }

        var lines = new List<string>();
        while (true)
        {
            if (index >= input.Length)
            {
                throw Http1ParseException.IncompleteHead();
            }

            var lineStart = index;
            while (index < input.Length && input[index] != (byte)'\n')
            {
                index++;
            }

            if (index >= input.Length)
            {
                throw Http1ParseException.IncompleteHead();
            }

            var lineEnd = index > lineStart && input[index - 1] == (byte)'\r'
                ? index - 1
                : index;
            var line = Encoding.Latin1.GetString(input[lineStart..lineEnd]);
            index++;

            if (line.Length == 0)
            {
                return (lines, index);
            }

            lines.Add(line);
        }
    }

    private static IReadOnlyList<Header> ParseHeaders(IEnumerable<string> lines)
    {
        var headers = new List<Header>();
        foreach (var line in lines)
        {
            var colon = line.IndexOf(':');
            if (colon < 0)
            {
                throw Http1ParseException.InvalidHeader(line);
            }

            var name = line[..colon].Trim();
            if (name.Length == 0)
            {
                throw Http1ParseException.InvalidHeader(line);
            }

            headers.Add(new Header(name, line[(colon + 1)..].Trim(' ', '\t')));
        }

        return headers;
    }

    private static BodyKind RequestBodyKind(IReadOnlyList<Header> headers)
    {
        if (HasChunkedTransferEncoding(headers))
        {
            return BodyKind.Chunked();
        }

        return DeclaredContentLength(headers) switch
        {
            null or 0 => BodyKind.None(),
            var length => BodyKind.ContentLength(length.Value),
        };
    }

    private static BodyKind ResponseBodyKind(ushort status, IReadOnlyList<Header> headers)
    {
        if ((status >= 100 && status < 200) || status is 204 or 304)
        {
            return BodyKind.None();
        }

        if (HasChunkedTransferEncoding(headers))
        {
            return BodyKind.Chunked();
        }

        return DeclaredContentLength(headers) switch
        {
            null => BodyKind.UntilEof(),
            0 => BodyKind.None(),
            var length => BodyKind.ContentLength(length.Value),
        };
    }

    private static int? DeclaredContentLength(IEnumerable<Header> headers)
    {
        var value = headers
            .FirstOrDefault(header => header.Name.Equals("Content-Length", StringComparison.OrdinalIgnoreCase))
            ?.Value;
        if (value is null)
        {
            return null;
        }

        if (!int.TryParse(value, NumberStyles.None, CultureInfo.InvariantCulture, out var length) || length < 0)
        {
            throw Http1ParseException.InvalidContentLength(value);
        }

        return length;
    }

    private static bool HasChunkedTransferEncoding(IEnumerable<Header> headers)
    {
        return headers
            .Where(header => header.Name.Equals("Transfer-Encoding", StringComparison.OrdinalIgnoreCase))
            .SelectMany(header => header.Value.Split(','))
            .Any(piece => piece.Trim().Equals("chunked", StringComparison.OrdinalIgnoreCase));
    }

    private static string[] SplitWhitespace(string value)
    {
        return value.Split((char[]?)null, StringSplitOptions.RemoveEmptyEntries);
    }
}

public sealed record ParsedRequestHead(RequestHead Head, int BodyOffset, BodyKind BodyKind);

public sealed record ParsedResponseHead(ResponseHead Head, int BodyOffset, BodyKind BodyKind);

public sealed class Http1ParseException : Exception
{
    private Http1ParseException(string kind, string message) : base(message)
    {
        Kind = kind;
    }

    public string Kind { get; }

    internal static Http1ParseException IncompleteHead() =>
        new("IncompleteHead", "incomplete HTTP/1 head");

    internal static Http1ParseException InvalidStartLine(string line) =>
        new("InvalidStartLine", $"invalid HTTP/1 start line: {line}");

    internal static Http1ParseException InvalidHeader(string line) =>
        new("InvalidHeader", $"invalid HTTP/1 header: {line}");

    internal static Http1ParseException InvalidVersion(string value) =>
        new("InvalidVersion", $"invalid HTTP version: {value}");

    internal static Http1ParseException InvalidStatus(string value) =>
        new("InvalidStatus", $"invalid HTTP status: {value}");

    internal static Http1ParseException InvalidContentLength(string value) =>
        new("InvalidContentLength", $"invalid Content-Length: {value}");
}
