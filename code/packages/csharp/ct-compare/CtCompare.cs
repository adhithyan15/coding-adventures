namespace CodingAdventures.CtCompare;

/// <summary>
/// Constant-time comparison helpers for public-length byte buffers.
/// </summary>
public static class CtCompare
{
    /// <summary>
    /// Compare two byte sequences without short-circuiting on the first differing byte.
    /// </summary>
    public static bool CtEq(ReadOnlySpan<byte> left, ReadOnlySpan<byte> right)
    {
        if (left.Length != right.Length)
        {
            return false;
        }

        byte accumulator = 0;
        for (var i = 0; i < left.Length; i++)
        {
            accumulator |= (byte)(left[i] ^ right[i]);
        }

        return accumulator == 0;
    }

    /// <summary>
    /// Compare two same-size byte arrays. Runtime length mismatch returns false.
    /// </summary>
    public static bool CtEqFixed(ReadOnlySpan<byte> left, ReadOnlySpan<byte> right) => CtEq(left, right);

    /// <summary>
    /// Select between two same-length byte arrays using a byte mask.
    /// </summary>
    public static byte[] CtSelectBytes(ReadOnlySpan<byte> left, ReadOnlySpan<byte> right, bool choice)
    {
        if (left.Length != right.Length)
        {
            throw new ArgumentException("CtSelectBytes requires equal-length inputs.");
        }

        var mask = unchecked((byte)(0 - (choice ? 1 : 0)));
        var output = new byte[left.Length];
        for (var i = 0; i < left.Length; i++)
        {
            output[i] = (byte)(right[i] ^ ((left[i] ^ right[i]) & mask));
        }

        return output;
    }

    /// <summary>
    /// Compare two UInt64 values without data-dependent ordering branches.
    /// </summary>
    public static bool CtEqUInt64(ulong left, ulong right)
    {
        var diff = left ^ right;
        var folded = (diff | (0UL - diff)) >> 63;
        return folded == 0;
    }
}
