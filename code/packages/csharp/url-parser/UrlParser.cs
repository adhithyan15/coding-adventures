using System.Globalization;
using System.Text;

namespace CodingAdventures.UrlParser;

/// <summary>Stable URL parser package version.</summary>
public static class UrlParser
{
    /// <summary>Package semantic version.</summary>
    public const string Version = "0.1.0";

    /// <summary>Parse an absolute URL string.</summary>
    public static Url Parse(string input) => Url.Parse(input);

    /// <summary>Percent-encode a string using UTF-8 bytes.</summary>
    public static string PercentEncode(string input) => UrlInternals.PercentEncode(input);

    /// <summary>Percent-decode a string containing percent encoded UTF-8 bytes.</summary>
    public static string PercentDecode(string input) => UrlInternals.PercentDecode(input);
}

/// <summary>Specific URL parser failure modes.</summary>
public enum UrlErrorKind
{
    /// <summary>No scheme was present in the input.</summary>
    MissingScheme,

    /// <summary>The scheme did not match [a-z][a-z0-9+.-]*.</summary>
    InvalidScheme,

    /// <summary>The port could not be parsed as an unsigned 16-bit integer.</summary>
    InvalidPort,

    /// <summary>A percent escape was truncated, non-hex, or invalid UTF-8.</summary>
    InvalidPercentEncoding,

    /// <summary>The authority form supplied host-like metadata without a host.</summary>
    EmptyHost,

    /// <summary>A relative URL was supplied without a base URL.</summary>
    RelativeWithoutBase,
}

/// <summary>Exception raised for URL parser failures.</summary>
public sealed class UrlParseException : Exception
{
    /// <summary>Create a parser exception for a specific URL error kind.</summary>
    public UrlParseException(UrlErrorKind kind, string message)
        : base(message)
    {
        Kind = kind;
    }

    /// <summary>The structured parser error kind.</summary>
    public UrlErrorKind Kind { get; }
}

/// <summary>A parsed URL with separated scheme, authority, path, query, and fragment components.</summary>
public sealed record Url(
    string Scheme,
    string? Userinfo,
    string? Host,
    ushort? Port,
    string Path,
    string? Query,
    string? Fragment,
    string Raw)
{
    /// <summary>Parse an absolute URL string.</summary>
    public static Url Parse(string input)
    {
        ArgumentNullException.ThrowIfNull(input);

        var raw = input;
        var trimmed = input.Trim();
        string scheme;
        string afterScheme;
        bool hasAuthority;

        var schemeSeparator = trimmed.IndexOf("://", StringComparison.Ordinal);
        if (schemeSeparator >= 0)
        {
            scheme = trimmed[..schemeSeparator].ToLowerInvariant();
            afterScheme = trimmed[(schemeSeparator + 3)..];
            hasAuthority = true;
        }
        else
        {
            var colon = trimmed.IndexOf(':', StringComparison.Ordinal);
            if (colon <= 0 || trimmed[..colon].Contains('/'))
            {
                throw UrlInternals.Error(UrlErrorKind.MissingScheme, "missing scheme");
            }

            scheme = trimmed[..colon].ToLowerInvariant();
            afterScheme = trimmed[(colon + 1)..];
            hasAuthority = false;
        }

        UrlInternals.ValidateScheme(scheme);

        var (withoutFragment, fragment) = UrlInternals.SplitFragment(afterScheme);
        var (withoutQuery, query) = UrlInternals.SplitQuery(withoutFragment);

        if (!hasAuthority)
        {
            return new Url(
                scheme,
                Userinfo: null,
                Host: null,
                Port: null,
                Path: withoutQuery,
                Query: query,
                Fragment: fragment,
                Raw: raw);
        }

        var firstSlash = withoutQuery.IndexOf('/', StringComparison.Ordinal);
        var authority = firstSlash >= 0 ? withoutQuery[..firstSlash] : withoutQuery;
        var path = firstSlash >= 0 ? withoutQuery[firstSlash..] : "/";

        string? userinfo = null;
        var at = authority.LastIndexOf('@');
        if (at >= 0)
        {
            userinfo = authority[..at];
            authority = authority[(at + 1)..];
        }

        var (hostText, port) = UrlInternals.SplitHostAndPort(authority);
        var host = string.IsNullOrEmpty(hostText) ? null : hostText.ToLowerInvariant();

        return new Url(
            scheme,
            userinfo,
            host,
            port,
            path,
            query,
            fragment,
            raw);
    }

    /// <summary>Resolve a relative URL reference against this URL as the base.</summary>
    public Url Resolve(string relative)
    {
        ArgumentNullException.ThrowIfNull(relative);

        relative = relative.Trim();
        if (relative.Length == 0)
        {
            var result = this with { Fragment = null };
            return result with { Raw = result.ToUrlString() };
        }

        if (relative.StartsWith('#'))
        {
            var result = this with { Fragment = relative[1..] };
            return result with { Raw = result.ToUrlString() };
        }

        if (UrlInternals.StartsWithScheme(relative))
        {
            return Parse(relative);
        }

        if (relative.StartsWith("//", StringComparison.Ordinal))
        {
            return Parse($"{Scheme}:{relative}");
        }

        var (relativeWithoutFragment, fragment) = UrlInternals.SplitFragment(relative);
        var (relativePath, query) = UrlInternals.SplitQuery(relativeWithoutFragment);

        if (relativePath.StartsWith('/'))
        {
            var result = this with
            {
                Path = UrlInternals.RemoveDotSegments(relativePath),
                Query = query,
                Fragment = fragment,
            };
            return result with { Raw = result.ToUrlString() };
        }

        var merged = UrlInternals.MergePaths(Path, relativePath);
        var resolved = this with
        {
            Path = UrlInternals.RemoveDotSegments(merged),
            Query = query,
            Fragment = fragment,
        };
        return resolved with { Raw = resolved.ToUrlString() };
    }

    /// <summary>Return the explicit port or the scheme default.</summary>
    public ushort? EffectivePort() => Port ?? UrlInternals.DefaultPort(Scheme);

    /// <summary>Reconstruct the authority as [userinfo@]host[:port].</summary>
    public string Authority()
    {
        var builder = new StringBuilder();
        if (Userinfo is not null)
        {
            builder.Append(Userinfo);
            builder.Append('@');
        }

        if (Host is not null)
        {
            builder.Append(Host);
        }

        if (Port is not null)
        {
            builder.Append(':');
            builder.Append(Port.Value.ToString(CultureInfo.InvariantCulture));
        }

        return builder.ToString();
    }

    /// <summary>Serialize this URL back to a string.</summary>
    public string ToUrlString()
    {
        var builder = new StringBuilder();
        builder.Append(Scheme);

        if (Host is not null)
        {
            builder.Append("://");
            builder.Append(Authority());
        }
        else
        {
            builder.Append(':');
        }

        builder.Append(Path);

        if (Query is not null)
        {
            builder.Append('?');
            builder.Append(Query);
        }

        if (Fragment is not null)
        {
            builder.Append('#');
            builder.Append(Fragment);
        }

        return builder.ToString();
    }

    /// <inheritdoc />
    public override string ToString() => ToUrlString();
}

internal static class UrlInternals
{
    private static readonly UTF8Encoding StrictUtf8 = new(encoderShouldEmitUTF8Identifier: false, throwOnInvalidBytes: true);

    public static UrlParseException Error(UrlErrorKind kind, string message) => new(kind, message);

    public static void ValidateScheme(string scheme)
    {
        if (scheme.Length == 0 || !IsAsciiLower(scheme[0]))
        {
            throw Error(UrlErrorKind.InvalidScheme, "scheme must start with a letter");
        }

        foreach (var c in scheme)
        {
            if (!IsAsciiLower(c) && !char.IsAsciiDigit(c) && c != '+' && c != '-' && c != '.')
            {
                throw Error(UrlErrorKind.InvalidScheme, "scheme contains invalid characters");
            }
        }
    }

    public static ushort? DefaultPort(string scheme) =>
        scheme switch
        {
            "http" => 80,
            "https" => 443,
            "ftp" => 21,
            _ => null,
        };

    public static (string Before, string? After) SplitFragment(string input) => SplitFirst(input, '#');

    public static (string Before, string? After) SplitQuery(string input) => SplitFirst(input, '?');

    public static (string Host, ushort? Port) SplitHostAndPort(string hostPort)
    {
        if (hostPort.StartsWith('['))
        {
            var bracket = hostPort.IndexOf(']', StringComparison.Ordinal);
            if (bracket >= 0)
            {
                var host = hostPort[..(bracket + 1)];
                var afterBracket = hostPort[(bracket + 1)..];
                var port = afterBracket.StartsWith(':') ? ParsePort(afterBracket[1..]) : (ushort?)null;
                return (host, port);
            }

            return (hostPort, null);
        }

        var colon = hostPort.LastIndexOf(':');
        if (colon >= 0)
        {
            var maybePort = hostPort[(colon + 1)..];
            if (maybePort.Length > 0 && maybePort.All(char.IsAsciiDigit))
            {
                return (hostPort[..colon], ParsePort(maybePort));
            }
        }

        return (hostPort, null);
    }

    public static ushort ParsePort(string text)
    {
        if (!ushort.TryParse(text, NumberStyles.None, CultureInfo.InvariantCulture, out var port))
        {
            throw Error(UrlErrorKind.InvalidPort, "port must be between 0 and 65535");
        }

        return port;
    }

    public static bool StartsWithScheme(string input)
    {
        var colon = input.IndexOf(':', StringComparison.Ordinal);
        if (colon <= 0 || input.StartsWith('/'))
        {
            return false;
        }

        var candidate = input[..colon];
        return IsAsciiLetter(candidate[0])
            && candidate.All(c => IsAsciiLetter(c) || char.IsAsciiDigit(c) || c is '+' or '-' or '.');
    }

    public static string MergePaths(string basePath, string relativePath)
    {
        var slash = basePath.LastIndexOf('/');
        return slash >= 0 ? basePath[..(slash + 1)] + relativePath : "/" + relativePath;
    }

    public static string RemoveDotSegments(string path)
    {
        var output = new List<string>();
        foreach (var segment in path.Split('/'))
        {
            switch (segment)
            {
                case ".":
                    break;
                case "..":
                    if (output.Count > 0)
                    {
                        output.RemoveAt(output.Count - 1);
                    }

                    break;
                default:
                    output.Add(segment);
                    break;
            }
        }

        var result = string.Join("/", output);
        return path.StartsWith('/') && !result.StartsWith('/') ? "/" + result : result;
    }

    public static string PercentEncode(string input)
    {
        ArgumentNullException.ThrowIfNull(input);

        var builder = new StringBuilder(input.Length);
        foreach (var b in Encoding.UTF8.GetBytes(input))
        {
            if (IsUnreserved(b))
            {
                builder.Append((char)b);
            }
            else
            {
                builder.Append('%');
                builder.Append(b.ToString("X2", CultureInfo.InvariantCulture));
            }
        }

        return builder.ToString();
    }

    public static string PercentDecode(string input)
    {
        ArgumentNullException.ThrowIfNull(input);

        var bytes = new List<byte>(input.Length);
        for (var i = 0; i < input.Length;)
        {
            if (input[i] == '%')
            {
                if (i + 2 >= input.Length)
                {
                    throw Error(UrlErrorKind.InvalidPercentEncoding, "truncated percent escape");
                }

                var hi = HexDigit(input[i + 1]);
                var lo = HexDigit(input[i + 2]);
                bytes.Add((byte)((hi << 4) | lo));
                i += 3;
            }
            else if (char.IsHighSurrogate(input[i]) && i + 1 < input.Length && char.IsLowSurrogate(input[i + 1]))
            {
                bytes.AddRange(Encoding.UTF8.GetBytes(input.Substring(i, 2)));
                i += 2;
            }
            else
            {
                bytes.AddRange(Encoding.UTF8.GetBytes(input[i].ToString()));
                i++;
            }
        }

        try
        {
            return StrictUtf8.GetString(bytes.ToArray());
        }
        catch (DecoderFallbackException ex)
        {
            throw Error(UrlErrorKind.InvalidPercentEncoding, ex.Message);
        }
    }

    private static (string Before, string? After) SplitFirst(string input, char delimiter)
    {
        var index = input.IndexOf(delimiter);
        return index >= 0 ? (input[..index], input[(index + 1)..]) : (input, null);
    }

    private static byte HexDigit(char c) =>
        c switch
        {
            >= '0' and <= '9' => (byte)(c - '0'),
            >= 'a' and <= 'f' => (byte)(c - 'a' + 10),
            >= 'A' and <= 'F' => (byte)(c - 'A' + 10),
            _ => throw Error(UrlErrorKind.InvalidPercentEncoding, "invalid hex digit"),
        };

    private static bool IsUnreserved(byte b) =>
        char.IsAsciiLetterOrDigit((char)b) || b is (byte)'-' or (byte)'_' or (byte)'.' or (byte)'~' or (byte)'/';

    private static bool IsAsciiLower(char c) => c is >= 'a' and <= 'z';

    private static bool IsAsciiLetter(char c) => c is >= 'a' and <= 'z' or >= 'A' and <= 'Z';
}
