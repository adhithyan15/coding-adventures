namespace CodingAdventures.Rng

open System
open System.Numerics

module private Constants =
    [<Literal>]
    let LcgMultiplier = 6364136223846793005UL

    [<Literal>]
    let LcgIncrement = 1442695040888963407UL

    [<Literal>]
    let FloatDiv = 4294967296.0

[<AutoOpen>]
module private Helpers =
    let rangeThreshold (rangeSize: uint64) =
        (UInt64.MaxValue - rangeSize + 1UL) % rangeSize

    let validateRange min max =
        if min > max then
            invalidArg (nameof min) $"NextIntInRange requires min <= max, got {min} > {max}"

[<Sealed>]
type Lcg(seed: uint64) =
    let mutable state = seed

    member _.NextU32() =
        state <- state * Constants.LcgMultiplier + Constants.LcgIncrement
        uint32 (state >>> 32)

    member this.NextU64() =
        let hi = uint64 (this.NextU32())
        let lo = uint64 (this.NextU32())
        (hi <<< 32) ||| lo

    member this.NextFloat() =
        double (this.NextU32()) / Constants.FloatDiv

    member this.NextIntInRange(min: int64, max: int64) =
        validateRange min max
        let rangeSize = uint64 (max - min + 1L)
        let threshold = rangeThreshold rangeSize
        let mutable result = None

        while result.IsNone do
            let r = uint64 (this.NextU32())

            if r >= threshold then
                result <- Some(min + int64 (r % rangeSize))

        result.Value

[<Sealed>]
type Xorshift64(seed: uint64) =
    let mutable state =
        if seed = 0UL then
            1UL
        else
            seed

    member _.NextU32() =
        let mutable x = state
        x <- x ^^^ (x <<< 13)
        x <- x ^^^ (x >>> 7)
        x <- x ^^^ (x <<< 17)
        state <- x
        uint32 x

    member this.NextU64() =
        let hi = uint64 (this.NextU32())
        let lo = uint64 (this.NextU32())
        (hi <<< 32) ||| lo

    member this.NextFloat() =
        double (this.NextU32()) / Constants.FloatDiv

    member this.NextIntInRange(min: int64, max: int64) =
        validateRange min max
        let rangeSize = uint64 (max - min + 1L)
        let threshold = rangeThreshold rangeSize
        let mutable result = None

        while result.IsNone do
            let r = uint64 (this.NextU32())

            if r >= threshold then
                result <- Some(min + int64 (r % rangeSize))

        result.Value

[<Sealed>]
type Pcg32(seed: uint64) =
    let increment = Constants.LcgIncrement ||| 1UL
    let mutable state = 0UL

    do
        state <- state * Constants.LcgMultiplier + increment
        state <- state + seed
        state <- state * Constants.LcgMultiplier + increment

    member _.NextU32() =
        let oldState = state
        state <- oldState * Constants.LcgMultiplier + increment
        let xorshifted = uint32 (((oldState >>> 18) ^^^ oldState) >>> 27)
        let rotation = int (oldState >>> 59)
        BitOperations.RotateRight(xorshifted, rotation)

    member this.NextU64() =
        let hi = uint64 (this.NextU32())
        let lo = uint64 (this.NextU32())
        (hi <<< 32) ||| lo

    member this.NextFloat() =
        double (this.NextU32()) / Constants.FloatDiv

    member this.NextIntInRange(min: int64, max: int64) =
        validateRange min max
        let rangeSize = uint64 (max - min + 1L)
        let threshold = rangeThreshold rangeSize
        let mutable result = None

        while result.IsNone do
            let r = uint64 (this.NextU32())

            if r >= threshold then
                result <- Some(min + int64 (r % rangeSize))

        result.Value
