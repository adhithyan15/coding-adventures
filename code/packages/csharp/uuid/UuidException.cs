namespace CodingAdventures.Uuid;

/// <summary>
/// Thrown when a UUID string or byte sequence cannot be parsed.
/// </summary>
public sealed class UuidException : ArgumentException
{
    /// <summary>
    /// Create a UUID exception with a message.
    /// </summary>
    public UuidException(string message)
        : base(message)
    {
    }

    /// <summary>
    /// Create a UUID exception with a message and inner exception.
    /// </summary>
    public UuidException(string message, Exception innerException)
        : base(message, innerException)
    {
    }
}
