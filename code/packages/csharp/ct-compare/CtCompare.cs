using System.Runtime.CompilerServices;

namespace CodingAdventures.CtCompare;

/// <summary>
/// Constant-time comparison helpers for public-length byte buffers: the C#
/// lane of CT01 (<c>code/specs/CT01-constant-time-compare.md</c>).
/// </summary>
/// <remarks>
/// <para>
/// <b>Why there are <c>byte[]</c> overloads.</b> C# converts a <c>byte[]</c>
/// to a <c>ReadOnlySpan&lt;byte&gt;</c> implicitly, and a <c>null</c> array
/// becomes an <i>empty</i> span. With only span overloads,
/// <c>CtEq(null, Array.Empty&lt;byte&gt;())</c> therefore returned
/// <c>true</c>: a missing value compared equal to an empty one. The array
/// overloads are chosen first for array arguments (including a <c>null</c>
/// literal) and reject <c>null</c> with <see cref="ArgumentNullException"/>.
/// Callers who already hold spans keep the span overloads.
/// </para>
/// <para>
/// <b>Why the selection mask avoids <c>?:</c>.</b> CT01 forbids a branch on
/// the secret <c>choice</c> bit. <c>choice ? 1 : 0</c> is such a branch, so
/// the bool's stored byte is read directly with
/// <see cref="Unsafe.As{TFrom, TTo}(ref TFrom)"/> and normalised to 0 or 1
/// arithmetically (see <see cref="ChoiceMask"/>). The JIT may still compile
/// this however it likes; CT01 does not promise that a runtime preserves the
/// source's shape, and neither does this lane.
/// </para>
/// </remarks>
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
    /// Compare two byte arrays. A <c>null</c> array is rejected rather than
    /// being treated as empty.
    /// </summary>
    /// <exception cref="ArgumentNullException">Either array is <c>null</c>.</exception>
    public static bool CtEq(byte[] left, byte[] right)
    {
        ArgumentNullException.ThrowIfNull(left);
        ArgumentNullException.ThrowIfNull(right);
        return CtEq((ReadOnlySpan<byte>)left, (ReadOnlySpan<byte>)right);
    }

    /// <summary>
    /// Compare two same-size byte arrays. Runtime length mismatch returns false.
    /// </summary>
    public static bool CtEqFixed(ReadOnlySpan<byte> left, ReadOnlySpan<byte> right) => CtEq(left, right);

    /// <summary>
    /// Compare two same-size byte arrays, rejecting <c>null</c>.
    /// </summary>
    /// <exception cref="ArgumentNullException">Either array is <c>null</c>.</exception>
    public static bool CtEqFixed(byte[] left, byte[] right) => CtEq(left, right);

    /// <summary>
    /// Select between two same-length byte arrays using a byte mask, returning
    /// a fresh copy of <paramref name="left"/> when <paramref name="choice"/> is
    /// true and of <paramref name="right"/> otherwise.
    /// </summary>
    public static byte[] CtSelectBytes(ReadOnlySpan<byte> left, ReadOnlySpan<byte> right, bool choice)
    {
        if (left.Length != right.Length)
        {
            throw new ArgumentException("CtSelectBytes requires equal-length inputs.");
        }

        var mask = ChoiceMask(choice);
        var output = new byte[left.Length];
        for (var i = 0; i < left.Length; i++)
        {
            output[i] = (byte)(right[i] ^ ((left[i] ^ right[i]) & mask));
        }

        return output;
    }

    /// <summary>
    /// Select between two same-length byte arrays, rejecting <c>null</c>.
    /// </summary>
    /// <exception cref="ArgumentNullException">Either array is <c>null</c>.</exception>
    public static byte[] CtSelectBytes(byte[] left, byte[] right, bool choice)
    {
        ArgumentNullException.ThrowIfNull(left);
        ArgumentNullException.ThrowIfNull(right);
        return CtSelectBytes((ReadOnlySpan<byte>)left, (ReadOnlySpan<byte>)right, choice);
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

    /// <summary>
    /// Turn the secret <paramref name="choice"/> into an all-ones or all-zeros
    /// byte without branching on it.
    /// </summary>
    /// <remarks>
    /// A <c>bool</c> is stored as one byte, normally 0 or 1. Unsafe code can
    /// store any non-zero value as "true", so the byte <c>b</c> is normalised:
    /// <c>(b | -b)</c> has its sign bit set exactly when <c>b</c> is non-zero,
    /// and shifting it down by 31 leaves 0 or 1. Negating that gives 0 or -1,
    /// whose low byte is <c>0x00</c> or <c>0xFF</c>.
    /// <code>
    /// b = 0      → (0 | 0)  &gt;&gt;&gt; 31 = 0 → mask 0x00
    /// b = 1      → (1 | -1) &gt;&gt;&gt; 31 = 1 → mask 0xFF
    /// b = 2..255 → sign set &gt;&gt;&gt; 31  = 1 → mask 0xFF
    /// </code>
    /// </remarks>
    internal static byte ChoiceMask(bool choice)
    {
        int stored = Unsafe.As<bool, byte>(ref choice);
        var one = (int)((uint)(stored | -stored) >> 31);
        return unchecked((byte)-one);
    }
}
