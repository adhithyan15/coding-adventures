using System.Text;

namespace CodingAdventures.IrcProto;

/// <summary>
/// A single parsed IRC protocol message.
/// </summary>
public sealed record Message(string? Prefix, string Command, IReadOnlyList<string> Params);

/// <summary>
/// Raised when a raw line cannot be parsed as an IRC message.
/// </summary>
public sealed class ParseError : Exception
{
    /// <summary>Create a parse error with a human-readable message.</summary>
    public ParseError(string message) : base(message)
    {
    }
}

/// <summary>
/// Pure IRC message parsing and serialization helpers.
/// </summary>
public static class IrcProto
{
    /// <summary>The package version.</summary>
    public const string Version = "0.1.0";

    private const int MaxParams = 15;

    /// <summary>Parse one IRC line with any trailing CRLF already stripped.</summary>
    public static Message Parse(string line)
    {
        ArgumentNullException.ThrowIfNull(line);
        if (line.Length == 0 || string.IsNullOrWhiteSpace(line))
        {
            throw new ParseError($"empty or whitespace-only line: {line}");
        }

        var rest = line;
        string? prefix = null;
        if (rest.StartsWith(':'))
        {
            var spacePosition = rest.IndexOf(' ', StringComparison.Ordinal);
            if (spacePosition == -1)
            {
                throw new ParseError($"line has prefix but no command: {line}");
            }

            prefix = rest[1..spacePosition];
            rest = rest[(spacePosition + 1)..];
        }

        var commandEnd = rest.IndexOf(' ', StringComparison.Ordinal);
        string commandRaw;
        if (commandEnd == -1)
        {
            commandRaw = rest;
            rest = string.Empty;
        }
        else
        {
            commandRaw = rest[..commandEnd];
            rest = rest[(commandEnd + 1)..];
        }

        var command = commandRaw.ToUpperInvariant();
        if (command.Length == 0)
        {
            throw new ParseError($"could not extract command from line: {line}");
        }

        var parameters = new List<string>();
        while (rest.Length > 0)
        {
            if (rest.StartsWith(':'))
            {
                parameters.Add(rest[1..]);
                break;
            }

            var spacePosition = rest.IndexOf(' ', StringComparison.Ordinal);
            if (spacePosition == -1)
            {
                parameters.Add(rest);
                break;
            }

            parameters.Add(rest[..spacePosition]);
            rest = rest[(spacePosition + 1)..];

            if (parameters.Count == MaxParams)
            {
                break;
            }
        }

        return new Message(prefix, command, parameters.AsReadOnly());
    }

    /// <summary>Serialize a message to CRLF-terminated UTF-8 IRC wire bytes.</summary>
    public static byte[] Serialize(Message message)
    {
        ArgumentNullException.ThrowIfNull(message);
        var parts = new List<string>();
        if (message.Prefix is not null)
        {
            parts.Add($":{message.Prefix}");
        }

        parts.Add(message.Command);
        for (var i = 0; i < message.Params.Count; i++)
        {
            var parameter = message.Params[i];
            var isLast = i == message.Params.Count - 1;
            parts.Add(isLast && parameter.Contains(' ') ? $":{parameter}" : parameter);
        }

        return Encoding.UTF8.GetBytes(string.Join(" ", parts) + "\r\n");
    }
}
