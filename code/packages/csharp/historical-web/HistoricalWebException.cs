namespace CodingAdventures.HistoricalWeb;

/// <summary>Classifies invalid input in the early Web teaching profiles.</summary>
public enum HistoricalWebErrorKind
{
    InvalidAddress,
    InvalidRequest,
    InvalidResponse,
    InvalidUnicode,
}

/// <summary>A malformed value with a profile-specific kind and source offset.</summary>
public sealed class HistoricalWebException : FormatException
{
    public HistoricalWebException(HistoricalWebErrorKind kind, int? offset, string message)
        : base(message)
    {
        Kind = kind;
        Offset = offset;
    }

    public HistoricalWebErrorKind Kind { get; }

    /// <summary>Byte offset for HTTP input; UTF-16 code-unit offset for HTML source.</summary>
    public int? Offset { get; }
}
