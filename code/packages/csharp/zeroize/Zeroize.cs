namespace CodingAdventures.Zeroize;

/// <summary>
/// Helpers for clearing sensitive managed buffers in place.
/// </summary>
public static class Zeroize
{
    /// <summary>Overwrite a byte buffer with zeroes.</summary>
    public static void ZeroizeBytes(byte[] buffer)
    {
        ArgumentNullException.ThrowIfNull(buffer);
        for (var index = 0; index < buffer.Length; index++)
        {
            buffer[index] = 0;
        }
    }

    /// <summary>Clear a character buffer.</summary>
    public static void ZeroizeChars(char[] buffer)
    {
        ArgumentNullException.ThrowIfNull(buffer);
        Array.Clear(buffer);
    }

    /// <summary>Clear any managed array using the element type's default value.</summary>
    public static void ZeroizeArray<T>(T[] buffer)
    {
        ArgumentNullException.ThrowIfNull(buffer);
        Array.Clear(buffer);
    }
}

/// <summary>
/// Disposable wrapper that zeroizes its byte buffer when disposed.
/// </summary>
public sealed class ZeroizingBuffer : IDisposable
{
    private bool _disposed;

    /// <summary>Create a wrapper around an existing byte buffer.</summary>
    public ZeroizingBuffer(byte[] buffer)
    {
        ArgumentNullException.ThrowIfNull(buffer);
        Buffer = buffer;
    }

    /// <summary>The wrapped mutable byte buffer.</summary>
    public byte[] Buffer { get; }

    /// <inheritdoc />
    public void Dispose()
    {
        if (_disposed)
        {
            return;
        }

        Zeroize.ZeroizeBytes(Buffer);
        _disposed = true;
    }
}
