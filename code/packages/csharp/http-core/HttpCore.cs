using System.Globalization;

namespace CodingAdventures.HttpCore;

public static class HttpCore
{
    public const string Version = "0.1.0";

    public static string? FindHeader(IEnumerable<Header> headers, string name) =>
        headers.FirstOrDefault(header => header.Name.Equals(name, StringComparison.OrdinalIgnoreCase))?.Value;

    public static int? ParseContentLength(IEnumerable<Header> headers)
    {
        var value = FindHeader(headers, "Content-Length");
        return int.TryParse(value, NumberStyles.None, CultureInfo.InvariantCulture, out var length) && length >= 0
            ? length
            : null;
    }

    public static (string MediaType, string? Charset)? ParseContentType(IEnumerable<Header> headers)
    {
        var value = FindHeader(headers, "Content-Type");
        if (value is null)
        {
            return null;
        }

        var pieces = value.Split(';', StringSplitOptions.TrimEntries);
        var mediaType = pieces.FirstOrDefault() ?? string.Empty;
        if (mediaType.Length == 0)
        {
            return null;
        }

        string? charset = null;
        foreach (var piece in pieces.Skip(1))
        {
            var pair = piece.Split('=', 2, StringSplitOptions.TrimEntries);
            if (pair.Length == 2 && pair[0].Equals("charset", StringComparison.OrdinalIgnoreCase))
            {
                charset = pair[1].Trim('"');
                break;
            }
        }

        return (mediaType, charset);
    }

    public static IReadOnlyList<string> SplitPathSegments(string path)
    {
        ArgumentNullException.ThrowIfNull(path);
        return path == "/"
            ? []
            : path.Split('/', StringSplitOptions.RemoveEmptyEntries);
    }
}

public sealed record Header(string Name, string Value);

public readonly record struct HttpVersion(ushort Major, ushort Minor)
{
    public static HttpVersion Parse(string text)
    {
        if (!text.StartsWith("HTTP/", StringComparison.Ordinal))
        {
            throw new FormatException($"invalid HTTP version: {text}");
        }

        var rest = text[5..];
        var dot = rest.IndexOf('.');
        if (dot < 0
            || !ushort.TryParse(rest[..dot], NumberStyles.None, CultureInfo.InvariantCulture, out var major)
            || !ushort.TryParse(rest[(dot + 1)..], NumberStyles.None, CultureInfo.InvariantCulture, out var minor))
        {
            throw new FormatException($"invalid HTTP version: {text}");
        }

        return new HttpVersion(major, minor);
    }

    public override string ToString() => $"HTTP/{Major}.{Minor}";
}

public sealed record BodyKind(string Mode, int? Length = null)
{
    public static BodyKind None() => new("none");

    public static BodyKind ContentLength(int length) => new("content-length", length);

    public static BodyKind UntilEof() => new("until-eof");

    public static BodyKind Chunked() => new("chunked");
}

public sealed record RequestHead(string Method, string Target, HttpVersion Version, IReadOnlyList<Header> Headers)
{
    public string? Header(string name) => HttpCore.FindHeader(Headers, name);

    public int? ContentLength() => HttpCore.ParseContentLength(Headers);

    public (string MediaType, string? Charset)? ContentType() => HttpCore.ParseContentType(Headers);
}

public sealed record ResponseHead(HttpVersion Version, ushort Status, string Reason, IReadOnlyList<Header> Headers)
{
    public string? Header(string name) => HttpCore.FindHeader(Headers, name);

    public int? ContentLength() => HttpCore.ParseContentLength(Headers);

    public (string MediaType, string? Charset)? ContentType() => HttpCore.ParseContentType(Headers);
}

public abstract record RouteSegment
{
    public sealed record Literal(string Value) : RouteSegment;

    public sealed record Param(string Name) : RouteSegment;
}

public sealed record RoutePattern(IReadOnlyList<RouteSegment> Segments)
{
    public static RoutePattern Parse(string pattern)
    {
        var segments = HttpCore.SplitPathSegments(pattern)
            .Select(segment => segment.StartsWith(':')
                ? (RouteSegment)new RouteSegment.Param(segment[1..])
                : new RouteSegment.Literal(segment))
            .ToArray();
        return new RoutePattern(segments);
    }

    public IReadOnlyList<(string Name, string Value)>? MatchPath(string path)
    {
        var pathSegments = HttpCore.SplitPathSegments(path);
        if (pathSegments.Count != Segments.Count)
        {
            return null;
        }

        var parameters = new List<(string Name, string Value)>();
        for (var i = 0; i < Segments.Count; i++)
        {
            switch (Segments[i])
            {
                case RouteSegment.Literal literal when literal.Value == pathSegments[i]:
                    break;
                case RouteSegment.Param param:
                    parameters.Add((param.Name, pathSegments[i]));
                    break;
                default:
                    return null;
            }
        }

        return parameters;
    }
}
