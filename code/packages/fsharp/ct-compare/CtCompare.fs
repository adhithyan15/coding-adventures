namespace CodingAdventures.CtCompare

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

    let ctSelectBytes (left: byte array) (right: byte array) choice =
        if isNull left then
            nullArg (nameof left)
        if isNull right then
            nullArg (nameof right)

        if left.Length <> right.Length then
            invalidArg (nameof right) "ctSelectBytes requires equal-length inputs."

        let mask = if choice then 0xFFuy else 0uy
        Array.init left.Length (fun index ->
            right[index] ^^^ ((left[index] ^^^ right[index]) &&& mask))

    let ctEqUInt64 (left: uint64) (right: uint64) =
        let diff = left ^^^ right
        let folded = (diff ||| (0UL - diff)) >>> 63
        folded = 0UL
