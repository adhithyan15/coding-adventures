using System;
using System.Collections;
using System.Collections.Generic;
using System.Runtime.InteropServices;

namespace CodingAdventures.BitsetNative;

// BitsetNative.cs -- Managed .NET wrapper over the Rust bitset C ABI
// ==================================================================
//
// .NET does have a native interop story: P/Invoke. The runtime can call a
// stable C ABI exported from a shared library, so this package simply wraps the
// Rust `bitset-c` shim in an API that mirrors the pure C# bitset package.
//
// The handle lifetime is the one thing the managed wrapper must own carefully.
// Each Bitset instance is backed by an opaque native pointer allocated in Rust.
// We release it via IDisposable/finalization and keep the public API otherwise
// familiar: constructors, single-bit operations, bulk operations, queries, and
// conversion helpers.

/// <summary>
/// Raised when the Rust native layer rejects an operation, such as parsing an
/// invalid binary string.
/// </summary>
public sealed class BitsetError : Exception
{
    public BitsetError(string message)
        : base(message)
    {
    }
}

/// <summary>
/// A native-backed compact boolean array packed into 64-bit words.
/// </summary>
public sealed class Bitset : IEquatable<Bitset>, IEnumerable<int>, IDisposable
{
    private nint _handle;

    /// <summary>
    /// Create a zero-filled bitset with the requested logical length.
    /// </summary>
    public Bitset(int size)
    {
        if (size < 0)
        {
            throw new ArgumentOutOfRangeException(nameof(size), "Bitset size cannot be negative.");
        }

        _handle = NativeMethods.bitset_c_new((ulong)size);
        ThrowIfCreateFailed(_handle, "bitset_c_new");
    }

    private Bitset(nint handle)
    {
        _handle = handle;
        ThrowIfCreateFailed(_handle, "native bitset operation");
    }

    /// <summary>
    /// Logical size: the number of addressable bits.
    /// </summary>
    public int Length => CheckedInt(NativeMethods.bitset_c_len(Handle), nameof(Length));

    /// <summary>
    /// Allocated size rounded to a multiple of 64.
    /// </summary>
    public int Capacity => CheckedInt(NativeMethods.bitset_c_capacity(Handle), nameof(Capacity));

    /// <summary>
    /// Whether the bitset has zero logical length.
    /// </summary>
    public bool IsEmpty => NativeMethods.bitset_c_is_empty(Handle) != 0;

    /// <summary>
    /// Create a bitset from a non-negative integer using LSB-first ordering.
    /// </summary>
    public static Bitset FromInteger(UInt128 value)
    {
        var low = (ulong)value;
        var high = (ulong)(value >> 64);
        return CreateChecked(NativeMethods.bitset_c_from_u128(low, high), "bitset_c_from_u128");
    }

    /// <summary>
    /// Create a bitset from a string whose leftmost character is the highest bit.
    /// </summary>
    public static Bitset FromBinaryString(string value)
    {
        ArgumentNullException.ThrowIfNull(value);
        return CreateChecked(NativeMethods.bitset_c_from_binary_str(value), "bitset_c_from_binary_str");
    }

    /// <summary>
    /// Set a bit to 1, growing the bitset if needed.
    /// </summary>
    public void Set(int index)
    {
        ValidateIndex(index);
        NativeMethods.bitset_c_set(Handle, (ulong)index);
    }

    /// <summary>
    /// Set a bit to 0. Clearing beyond <see cref="Length"/> is a no-op.
    /// </summary>
    public void Clear(int index)
    {
        ValidateIndex(index);
        NativeMethods.bitset_c_clear(Handle, (ulong)index);
    }

    /// <summary>
    /// Return whether a bit is set. Testing beyond <see cref="Length"/> is false.
    /// </summary>
    public bool Test(int index)
    {
        ValidateIndex(index);
        return NativeMethods.bitset_c_test(Handle, (ulong)index) != 0;
    }

    /// <summary>
    /// Flip a bit, growing the bitset if needed.
    /// </summary>
    public void Toggle(int index)
    {
        ValidateIndex(index);
        NativeMethods.bitset_c_toggle(Handle, (ulong)index);
    }

    /// <summary>
    /// Bitwise AND. The result length is the longer input length.
    /// </summary>
    public Bitset And(Bitset other) => CreateChecked(NativeMethods.bitset_c_and(Handle, RequiredHandle(other)), "bitset_c_and");

    /// <summary>
    /// Bitwise OR. The result length is the longer input length.
    /// </summary>
    public Bitset Or(Bitset other) => CreateChecked(NativeMethods.bitset_c_or(Handle, RequiredHandle(other)), "bitset_c_or");

    /// <summary>
    /// Bitwise XOR. The result length is the longer input length.
    /// </summary>
    public Bitset Xor(Bitset other) => CreateChecked(NativeMethods.bitset_c_xor(Handle, RequiredHandle(other)), "bitset_c_xor");

    /// <summary>
    /// Bitwise complement within the logical length of the bitset.
    /// </summary>
    public Bitset Not() => CreateChecked(NativeMethods.bitset_c_not(Handle), "bitset_c_not");

    /// <summary>
    /// Set difference: keep bits set in this bitset that are not set in <paramref name="other"/>.
    /// </summary>
    public Bitset AndNot(Bitset other) => CreateChecked(NativeMethods.bitset_c_and_not(Handle, RequiredHandle(other)), "bitset_c_and_not");

    /// <summary>
    /// Count how many bits are set to 1.
    /// </summary>
    public int PopCount() => CheckedInt(NativeMethods.bitset_c_popcount(Handle), nameof(PopCount));

    /// <summary>
    /// Return whether at least one bit is set.
    /// </summary>
    public bool Any() => NativeMethods.bitset_c_any(Handle) != 0;

    /// <summary>
    /// Return whether all bits within <see cref="Length"/> are set.
    /// </summary>
    public bool All() => NativeMethods.bitset_c_all(Handle) != 0;

    /// <summary>
    /// Return whether no bits are set.
    /// </summary>
    public bool None() => NativeMethods.bitset_c_none(Handle) != 0;

    /// <summary>
    /// Convert to a 64-bit integer when the value fits in a single word.
    /// </summary>
    public ulong? ToInteger()
    {
        return NativeMethods.bitset_c_to_u64(Handle, out var value) != 0
            ? value
            : null;
    }

    /// <summary>
    /// Convert to a conventional binary string with the highest bit on the left.
    /// </summary>
    public string ToBinaryString()
    {
        var length = Length;
        if (length == 0)
        {
            return string.Empty;
        }

        var chars = new char[length];
        for (var i = 0; i < length; i++)
        {
            chars[length - 1 - i] = Test(i) ? '1' : '0';
        }

        return new string(chars);
    }

    /// <summary>
    /// Iterate over the indices of set bits in ascending order.
    /// </summary>
    public IEnumerable<int> IterSetBits()
    {
        var length = Length;
        for (var i = 0; i < length; i++)
        {
            if (Test(i))
            {
                yield return i;
            }
        }
    }

    /// <summary>
    /// Convenience alias for <see cref="Test"/>.
    /// </summary>
    public bool Contains(int index) => Test(index);

    /// <summary>
    /// Release the native Rust handle.
    /// </summary>
    public void Dispose()
    {
        Dispose(disposing: true);
        GC.SuppressFinalize(this);
    }

    /// <summary>
    /// Return a debug-friendly representation such as <c>Bitset(101)</c>.
    /// </summary>
    public override string ToString() => $"Bitset({ToBinaryString()})";

    /// <summary>
    /// Compare two bitsets by logical length and logical bits.
    /// </summary>
    public bool Equals(Bitset? other)
    {
        if (other is null)
        {
            return false;
        }

        if (ReferenceEquals(this, other))
        {
            return true;
        }

        return NativeMethods.bitset_c_equals(Handle, other.Handle) != 0;
    }

    /// <summary>
    /// Compare this bitset to another object.
    /// </summary>
    public override bool Equals(object? obj) => obj is Bitset other && Equals(other);

    /// <summary>
    /// Compute a hash from the logical length and set-bit positions.
    /// </summary>
    public override int GetHashCode()
    {
        var hash = new HashCode();
        hash.Add(Length);
        foreach (var bit in this)
        {
            hash.Add(bit);
        }

        return hash.ToHashCode();
    }

    /// <summary>
    /// Enumerate the indices of all set bits.
    /// </summary>
    public IEnumerator<int> GetEnumerator() => IterSetBits().GetEnumerator();

    IEnumerator IEnumerable.GetEnumerator() => GetEnumerator();

    /// <summary>
    /// Operator shorthand for <see cref="And"/>.
    /// </summary>
    public static Bitset operator &(Bitset left, Bitset right)
    {
        ArgumentNullException.ThrowIfNull(left);
        return left.And(right);
    }

    /// <summary>
    /// Operator shorthand for <see cref="Or"/>.
    /// </summary>
    public static Bitset operator |(Bitset left, Bitset right)
    {
        ArgumentNullException.ThrowIfNull(left);
        return left.Or(right);
    }

    /// <summary>
    /// Operator shorthand for <see cref="Xor"/>.
    /// </summary>
    public static Bitset operator ^(Bitset left, Bitset right)
    {
        ArgumentNullException.ThrowIfNull(left);
        return left.Xor(right);
    }

    /// <summary>
    /// Operator shorthand for <see cref="Not"/>.
    /// </summary>
    public static Bitset operator ~(Bitset value)
    {
        ArgumentNullException.ThrowIfNull(value);
        return value.Not();
    }

    /// <summary>
    /// Equality operator based on logical length and logical bits.
    /// </summary>
    public static bool operator ==(Bitset? left, Bitset? right)
    {
        if (ReferenceEquals(left, right))
        {
            return true;
        }

        if (left is null || right is null)
        {
            return false;
        }

        return left.Equals(right);
    }

    /// <summary>
    /// Inequality operator based on logical length and logical bits.
    /// </summary>
    public static bool operator !=(Bitset? left, Bitset? right) => !(left == right);

    ~Bitset()
    {
        Dispose(disposing: false);
    }

    private nint Handle => _handle != 0
        ? _handle
        : throw new ObjectDisposedException(nameof(Bitset));

    private static Bitset CreateChecked(nint handle, string operation)
    {
        ThrowIfCreateFailed(handle, operation);
        return new Bitset(handle);
    }

    private static nint RequiredHandle(Bitset? bitset)
    {
        ArgumentNullException.ThrowIfNull(bitset);
        return bitset.Handle;
    }

    private static void ThrowIfCreateFailed(nint handle, string operation)
    {
        if (handle == 0)
        {
            throw new BitsetError($"{operation} failed: {ReadLastErrorMessage()}");
        }
    }

    private static string ReadLastErrorMessage()
    {
        var pointer = NativeMethods.bitset_c_last_error_message();
        return pointer == 0
            ? "bitset-c reported an unknown error."
            : Marshal.PtrToStringUTF8(pointer) ?? "bitset-c reported an unreadable UTF-8 error message.";
    }

    private static void ValidateIndex(int index)
    {
        if (index < 0)
        {
            throw new ArgumentOutOfRangeException(nameof(index), "Bit indices cannot be negative.");
        }
    }

    private static int CheckedInt(ulong value, string operation)
    {
        if (value > int.MaxValue)
        {
            throw new OverflowException($"{operation} exceeded Int32.MaxValue.");
        }

        return (int)value;
    }

    private void Dispose(bool disposing)
    {
        if (_handle != 0)
        {
            NativeMethods.bitset_c_free(_handle);
            _handle = 0;
        }
    }

    private static class NativeMethods
    {
        private const string LibraryName = "bitset_c";

        [DllImport(LibraryName, EntryPoint = "bitset_c_new", CallingConvention = CallingConvention.Cdecl)]
        internal static extern nint bitset_c_new(ulong size);

        [DllImport(LibraryName, EntryPoint = "bitset_c_from_u128", CallingConvention = CallingConvention.Cdecl)]
        internal static extern nint bitset_c_from_u128(ulong low, ulong high);

        [DllImport(LibraryName, EntryPoint = "bitset_c_from_binary_str", CallingConvention = CallingConvention.Cdecl)]
        internal static extern nint bitset_c_from_binary_str([MarshalAs(UnmanagedType.LPUTF8Str)] string binary);

        [DllImport(LibraryName, EntryPoint = "bitset_c_free", CallingConvention = CallingConvention.Cdecl)]
        internal static extern void bitset_c_free(nint handle);

        [DllImport(LibraryName, EntryPoint = "bitset_c_set", CallingConvention = CallingConvention.Cdecl)]
        internal static extern void bitset_c_set(nint handle, ulong index);

        [DllImport(LibraryName, EntryPoint = "bitset_c_clear", CallingConvention = CallingConvention.Cdecl)]
        internal static extern void bitset_c_clear(nint handle, ulong index);

        [DllImport(LibraryName, EntryPoint = "bitset_c_test", CallingConvention = CallingConvention.Cdecl)]
        internal static extern byte bitset_c_test(nint handle, ulong index);

        [DllImport(LibraryName, EntryPoint = "bitset_c_toggle", CallingConvention = CallingConvention.Cdecl)]
        internal static extern void bitset_c_toggle(nint handle, ulong index);

        [DllImport(LibraryName, EntryPoint = "bitset_c_and", CallingConvention = CallingConvention.Cdecl)]
        internal static extern nint bitset_c_and(nint left, nint right);

        [DllImport(LibraryName, EntryPoint = "bitset_c_or", CallingConvention = CallingConvention.Cdecl)]
        internal static extern nint bitset_c_or(nint left, nint right);

        [DllImport(LibraryName, EntryPoint = "bitset_c_xor", CallingConvention = CallingConvention.Cdecl)]
        internal static extern nint bitset_c_xor(nint left, nint right);

        [DllImport(LibraryName, EntryPoint = "bitset_c_not", CallingConvention = CallingConvention.Cdecl)]
        internal static extern nint bitset_c_not(nint handle);

        [DllImport(LibraryName, EntryPoint = "bitset_c_and_not", CallingConvention = CallingConvention.Cdecl)]
        internal static extern nint bitset_c_and_not(nint left, nint right);

        [DllImport(LibraryName, EntryPoint = "bitset_c_popcount", CallingConvention = CallingConvention.Cdecl)]
        internal static extern ulong bitset_c_popcount(nint handle);

        [DllImport(LibraryName, EntryPoint = "bitset_c_len", CallingConvention = CallingConvention.Cdecl)]
        internal static extern ulong bitset_c_len(nint handle);

        [DllImport(LibraryName, EntryPoint = "bitset_c_capacity", CallingConvention = CallingConvention.Cdecl)]
        internal static extern ulong bitset_c_capacity(nint handle);

        [DllImport(LibraryName, EntryPoint = "bitset_c_any", CallingConvention = CallingConvention.Cdecl)]
        internal static extern byte bitset_c_any(nint handle);

        [DllImport(LibraryName, EntryPoint = "bitset_c_all", CallingConvention = CallingConvention.Cdecl)]
        internal static extern byte bitset_c_all(nint handle);

        [DllImport(LibraryName, EntryPoint = "bitset_c_none", CallingConvention = CallingConvention.Cdecl)]
        internal static extern byte bitset_c_none(nint handle);

        [DllImport(LibraryName, EntryPoint = "bitset_c_is_empty", CallingConvention = CallingConvention.Cdecl)]
        internal static extern byte bitset_c_is_empty(nint handle);

        [DllImport(LibraryName, EntryPoint = "bitset_c_to_u64", CallingConvention = CallingConvention.Cdecl)]
        internal static extern byte bitset_c_to_u64(nint handle, out ulong value);

        [DllImport(LibraryName, EntryPoint = "bitset_c_equals", CallingConvention = CallingConvention.Cdecl)]
        internal static extern byte bitset_c_equals(nint left, nint right);

        [DllImport(LibraryName, EntryPoint = "bitset_c_last_error_message", CallingConvention = CallingConvention.Cdecl)]
        internal static extern nint bitset_c_last_error_message();
    }
}
