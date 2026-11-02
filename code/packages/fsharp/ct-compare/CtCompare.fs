namespace CodingAdventures.CtCompare

open System.Runtime.CompilerServices

/// Constant-time comparison helpers for public-length byte buffers.
[<RequireQualifiedAccess>]
module CtCompare =
    let ctEq (left: byte array) (right: byte array) =
        if isNull left then
            nullArg (nameof left)
        if isNull right then
            nullArg (nameof right)

        if left.Length <> right.Length then
            false
        else
            let mutable accumulator = 0uy
            for index in 0 .. left.Length - 1 do
                accumulator <- accumulator ||| (left[index] ^^^ right[index])
            accumulator = 0uy

    let ctEqFixed (left: byte array) (right: byte array) =
        ctEq left right

    /// Turn the secret `choice` into 0x00 or 0xFF without branching on it.
    ///
    /// CT01 forbids a branch on the selection bit, so `if choice then 0xFFuy
    /// else 0uy` is out. A `bool` is stored as one byte, normally 0 or 1, but
    /// unsafe code can store any non-zero value as "true", so the stored byte
    /// `b` is read with `Unsafe.As` and normalised arithmetically:
    ///
    ///     b = 0       -> (0 ||| 0)  >>> 31 = 0 -> mask 0x00
    ///     b = 1..255  -> sign bit set >>> 31 = 1 -> mask 0xFF
    ///
    /// The JIT may still compile this however it likes; CT01 does not promise
    /// that a runtime preserves the source's shape.
    let internal choiceMask (choice: bool) : byte =
        let mutable copy = choice
        let stored = int (Unsafe.As<bool, byte>(&copy))
        let one = int ((uint32 (stored ||| -stored)) >>> 31)
        byte (-one &&& 0xFF)

    let ctSelectBytes (left: byte array) (right: byte array) choice =
        if isNull left then
            nullArg (nameof left)
        if isNull right then
            nullArg (nameof right)

        if left.Length <> right.Length then
            invalidArg (nameof right) "ctSelectBytes requires equal-length inputs."

        let mask = choiceMask choice
        Array.init left.Length (fun index ->
            right[index] ^^^ ((left[index] ^^^ right[index]) &&& mask))

    let ctEqUInt64 (left: uint64) (right: uint64) =
        let diff = left ^^^ right
        let folded = (diff ||| (0UL - diff)) >>> 63
        folded = 0UL
