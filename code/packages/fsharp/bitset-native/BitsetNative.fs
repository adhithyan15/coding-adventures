namespace CodingAdventures.BitsetNative

open System
open System.Collections
open System.Collections.Generic
open System.Runtime.InteropServices

// BitsetNative.fs -- F# wrapper over the Rust bitset C ABI
// =========================================================
//
// The .NET runtime already knows how to call native code through P/Invoke.
// That means F# can reuse the same Rust `bitset-c` shared library as C#:
//
//   F# API -> DllImport -> bitset-c -> bitset
//
// The wrapper keeps the public API close to the pure F# bitset package while
// taking responsibility for one native concern: releasing the opaque handle.

/// Raised when the Rust native layer rejects an operation such as parsing an invalid binary string.
type BitsetError(message: string) =
    inherit Exception(message)

module private NativeMethods =
    [<Literal>]
    let LibraryName = "bitset_c"

    [<DllImport(LibraryName, EntryPoint = "bitset_c_new", CallingConvention = CallingConvention.Cdecl)>]
    extern nativeint bitset_c_new(uint64 size)

    [<DllImport(LibraryName, EntryPoint = "bitset_c_from_u128", CallingConvention = CallingConvention.Cdecl)>]
    extern nativeint bitset_c_from_u128(uint64 low, uint64 high)

    [<DllImport(LibraryName, EntryPoint = "bitset_c_from_binary_str", CallingConvention = CallingConvention.Cdecl)>]
    extern nativeint bitset_c_from_binary_str([<MarshalAs(UnmanagedType.LPUTF8Str)>] string binary)

    [<DllImport(LibraryName, EntryPoint = "bitset_c_free", CallingConvention = CallingConvention.Cdecl)>]
    extern void bitset_c_free(nativeint handle)

    [<DllImport(LibraryName, EntryPoint = "bitset_c_set", CallingConvention = CallingConvention.Cdecl)>]
    extern void bitset_c_set(nativeint handle, uint64 index)

    [<DllImport(LibraryName, EntryPoint = "bitset_c_clear", CallingConvention = CallingConvention.Cdecl)>]
    extern void bitset_c_clear(nativeint handle, uint64 index)

    [<DllImport(LibraryName, EntryPoint = "bitset_c_test", CallingConvention = CallingConvention.Cdecl)>]
    extern byte bitset_c_test(nativeint handle, uint64 index)

    [<DllImport(LibraryName, EntryPoint = "bitset_c_toggle", CallingConvention = CallingConvention.Cdecl)>]
    extern void bitset_c_toggle(nativeint handle, uint64 index)

    [<DllImport(LibraryName, EntryPoint = "bitset_c_and", CallingConvention = CallingConvention.Cdecl)>]
    extern nativeint bitset_c_and(nativeint left, nativeint right)

    [<DllImport(LibraryName, EntryPoint = "bitset_c_or", CallingConvention = CallingConvention.Cdecl)>]
    extern nativeint bitset_c_or(nativeint left, nativeint right)

    [<DllImport(LibraryName, EntryPoint = "bitset_c_xor", CallingConvention = CallingConvention.Cdecl)>]
    extern nativeint bitset_c_xor(nativeint left, nativeint right)

    [<DllImport(LibraryName, EntryPoint = "bitset_c_not", CallingConvention = CallingConvention.Cdecl)>]
    extern nativeint bitset_c_not(nativeint handle)

    [<DllImport(LibraryName, EntryPoint = "bitset_c_and_not", CallingConvention = CallingConvention.Cdecl)>]
    extern nativeint bitset_c_and_not(nativeint left, nativeint right)

    [<DllImport(LibraryName, EntryPoint = "bitset_c_popcount", CallingConvention = CallingConvention.Cdecl)>]
    extern uint64 bitset_c_popcount(nativeint handle)

    [<DllImport(LibraryName, EntryPoint = "bitset_c_len", CallingConvention = CallingConvention.Cdecl)>]
    extern uint64 bitset_c_len(nativeint handle)

    [<DllImport(LibraryName, EntryPoint = "bitset_c_capacity", CallingConvention = CallingConvention.Cdecl)>]
    extern uint64 bitset_c_capacity(nativeint handle)

    [<DllImport(LibraryName, EntryPoint = "bitset_c_any", CallingConvention = CallingConvention.Cdecl)>]
    extern byte bitset_c_any(nativeint handle)

    [<DllImport(LibraryName, EntryPoint = "bitset_c_all", CallingConvention = CallingConvention.Cdecl)>]
    extern byte bitset_c_all(nativeint handle)

    [<DllImport(LibraryName, EntryPoint = "bitset_c_none", CallingConvention = CallingConvention.Cdecl)>]
    extern byte bitset_c_none(nativeint handle)

    [<DllImport(LibraryName, EntryPoint = "bitset_c_is_empty", CallingConvention = CallingConvention.Cdecl)>]
    extern byte bitset_c_is_empty(nativeint handle)

    [<DllImport(LibraryName, EntryPoint = "bitset_c_to_u64", CallingConvention = CallingConvention.Cdecl)>]
    extern byte bitset_c_to_u64(nativeint handle, uint64& value)

    [<DllImport(LibraryName, EntryPoint = "bitset_c_equals", CallingConvention = CallingConvention.Cdecl)>]
    extern byte bitset_c_equals(nativeint left, nativeint right)

    [<DllImport(LibraryName, EntryPoint = "bitset_c_last_error_message", CallingConvention = CallingConvention.Cdecl)>]
    extern nativeint bitset_c_last_error_message()

module private Interop =
    let readLastErrorMessage operation =
        let ptr = NativeMethods.bitset_c_last_error_message()
        let message =
            if ptr = 0n then
                "bitset-c reported an unknown error."
            else
                match Marshal.PtrToStringUTF8(ptr) with
                | null -> "bitset-c reported an unreadable UTF-8 error message."
                | text -> text

        $"{operation} failed: {message}"

    let raiseBitsetError operation =
        raise (BitsetError(readLastErrorMessage operation))

    let validateIndex index =
        if index < 0 then
            raise (ArgumentOutOfRangeException("index", "Bit indices cannot be negative."))

    let checkedInt operation value =
        if value > uint64 Int32.MaxValue then
            raise (OverflowException($"{operation} exceeded Int32.MaxValue."))

        int value

/// A native-backed compact boolean array packed into 64-bit words.
[<AllowNullLiteral>]
type Bitset private (nativeHandle: nativeint) =
    let mutable handle = nativeHandle

    new(size: int) =
        if size < 0 then
            raise (ArgumentOutOfRangeException("size", "Bitset size cannot be negative."))

        let handle = NativeMethods.bitset_c_new(uint64 size)
        if handle = 0n then
            Interop.raiseBitsetError "bitset_c_new"

        new Bitset(handle)

    static member FromInteger(value: UInt128) =
        let low = uint64 value
        let high = uint64 (value >>> 64)
        let handle = NativeMethods.bitset_c_from_u128(low, high)
        if handle = 0n then
            Interop.raiseBitsetError "bitset_c_from_u128"

        new Bitset(handle)

    static member FromBinaryString(value: string) =
        if isNull value then
            nullArg "value"

        let handle = NativeMethods.bitset_c_from_binary_str(value)
        if handle = 0n then
            Interop.raiseBitsetError "bitset_c_from_binary_str"

        new Bitset(handle)

    member private _.Handle =
        if handle = 0n then
            raise (ObjectDisposedException("Bitset"))
        else
            handle

    member private _.ReleaseHandle() =
        if handle <> 0n then
            NativeMethods.bitset_c_free(handle)
            handle <- 0n

    /// Logical size: the number of addressable bits.
    member this.Length = NativeMethods.bitset_c_len(this.Handle) |> Interop.checkedInt "Length"

    /// Allocated size rounded to a multiple of 64.
    member this.Capacity = NativeMethods.bitset_c_capacity(this.Handle) |> Interop.checkedInt "Capacity"

    /// Whether the bitset has zero logical length.
    member this.IsEmpty = NativeMethods.bitset_c_is_empty(this.Handle) <> 0uy

    /// Set a bit to 1, growing the bitset if needed.
    member this.Set(index: int) =
        Interop.validateIndex index
        NativeMethods.bitset_c_set(this.Handle, uint64 index)

    /// Set a bit to 0. Clearing beyond the logical length is a no-op.
    member this.Clear(index: int) =
        Interop.validateIndex index
        NativeMethods.bitset_c_clear(this.Handle, uint64 index)

    /// Return whether a bit is set. Testing beyond the logical length is false.
    member this.Test(index: int) =
        Interop.validateIndex index
        NativeMethods.bitset_c_test(this.Handle, uint64 index) <> 0uy

    /// Flip a bit, growing the bitset if needed.
    member this.Toggle(index: int) =
        Interop.validateIndex index
        NativeMethods.bitset_c_toggle(this.Handle, uint64 index)

    /// Bitwise AND. The result length is the longer input length.
    member this.And(other: Bitset) =
        if isNull (box other) then
            nullArg "other"

        let handle = NativeMethods.bitset_c_and(this.Handle, other.Handle)
        if handle = 0n then
            Interop.raiseBitsetError "bitset_c_and"

        new Bitset(handle)

    /// Bitwise OR. The result length is the longer input length.
    member this.Or(other: Bitset) =
        if isNull (box other) then
            nullArg "other"

        let handle = NativeMethods.bitset_c_or(this.Handle, other.Handle)
        if handle = 0n then
            Interop.raiseBitsetError "bitset_c_or"

        new Bitset(handle)

    /// Bitwise XOR. The result length is the longer input length.
    member this.Xor(other: Bitset) =
        if isNull (box other) then
            nullArg "other"

        let handle = NativeMethods.bitset_c_xor(this.Handle, other.Handle)
        if handle = 0n then
            Interop.raiseBitsetError "bitset_c_xor"

        new Bitset(handle)

    /// Bitwise complement within the logical length of the bitset.
    member this.Not() =
        let handle = NativeMethods.bitset_c_not(this.Handle)
        if handle = 0n then
            Interop.raiseBitsetError "bitset_c_not"

        new Bitset(handle)

    /// Set difference: keep bits set in this bitset that are not set in the other one.
    member this.AndNot(other: Bitset) =
        if isNull (box other) then
            nullArg "other"

        let handle = NativeMethods.bitset_c_and_not(this.Handle, other.Handle)
        if handle = 0n then
            Interop.raiseBitsetError "bitset_c_and_not"

        new Bitset(handle)

    /// Count how many bits are set to 1.
    member this.PopCount() = NativeMethods.bitset_c_popcount(this.Handle) |> Interop.checkedInt "PopCount"

    /// Return whether at least one bit is set.
    member this.Any() = NativeMethods.bitset_c_any(this.Handle) <> 0uy

    /// Return whether all logical bits are set.
    member this.All() = NativeMethods.bitset_c_all(this.Handle) <> 0uy

    /// Return whether no bits are set.
    member this.None() = NativeMethods.bitset_c_none(this.Handle) <> 0uy

    /// Convert to a 64-bit integer when the value fits in a single word.
    member this.ToInteger() =
        let mutable value = 0UL
        if NativeMethods.bitset_c_to_u64(this.Handle, &value) <> 0uy then
            Some value
        else
            None

    /// Convert to a conventional binary string with the highest bit on the left.
    member this.ToBinaryString() =
        let length = this.Length

        if length = 0 then
            String.Empty
        else
            Array.init length (fun i -> if this.Test(length - 1 - i) then '1' else '0')
            |> fun chars -> String(chars)

    /// Iterate over the set-bit indices in ascending order.
    member this.IterSetBits() : seq<int> =
        seq {
            let length = this.Length
            for i in 0 .. length - 1 do
                if this.Test(i) then
                    yield i
        }

    /// Convenience alias for Test.
    member this.Contains(index: int) = this.Test(index)

    override this.ToString() = $"Bitset({this.ToBinaryString()})"

    member this.Equals(other: Bitset) =
        if isNull (box other) then
            false
        elif Object.ReferenceEquals(this, other) then
            true
        else
            NativeMethods.bitset_c_equals(this.Handle, other.Handle) <> 0uy

    override this.Equals(obj: obj) =
        match obj with
        | :? Bitset as other -> this.Equals(other)
        | _ -> false

    override this.GetHashCode() =
        let mutable hash = HashCode.Combine(this.Length)

        for bit in this.IterSetBits() do
            hash <- HashCode.Combine(hash, bit)

        hash

    interface IEquatable<Bitset> with
        member this.Equals(other) = this.Equals(other)

    interface IEnumerable<int> with
        member this.GetEnumerator() = (this.IterSetBits()).GetEnumerator()

    interface IEnumerable with
        member this.GetEnumerator() = (this.IterSetBits() :> IEnumerable).GetEnumerator()

    interface IDisposable with
        member this.Dispose() =
            this.ReleaseHandle()
            GC.SuppressFinalize(this)

    override this.Finalize() =
        this.ReleaseHandle()
