namespace CodingAdventures.TestBootstrap;

/// <summary>
/// The single definition of "an exception the engine may recover from".
/// </summary>
/// <remarks>
/// A property, a generator, a user <c>ToString()</c> or an enumerator may throw
/// anything; the engine's contract is that it still returns a verdict (or a
/// problem line). So it catches <see cref="Exception"/>, with one deliberate
/// exception: <see cref="OutOfMemoryException"/> means the runtime could not
/// allocate and cannot be safely continued from, so it propagates.
/// <c>StackOverflowException</c> cannot be caught in .NET at all, which is why every
/// recursive routine in this package has an explicit depth cap.
/// </remarks>
internal static class Safe
{
    /// <summary>True for every exception except <see cref="OutOfMemoryException"/>.</summary>
    internal static bool Recoverable(Exception e)
    {
        return e is not OutOfMemoryException;
    }

    /// <summary>
    /// Text for a value that cannot itself throw while being rendered: its
    /// (clipped) <c>ToString()</c>, or its type name when that throws.
    /// </summary>
    internal static string SafeString(object? value, int limit = 200)
    {
        try
        {
            return CorpusJson.Clip.Apply(value?.ToString() ?? "null", limit);
        }
        catch (Exception e) when (Recoverable(e))
        {
            return value?.GetType().FullName ?? "null";
        }
    }
}
