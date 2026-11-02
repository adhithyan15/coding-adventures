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

// ── Bounded draws (spec rng.md, "Rejection Sampling", v2) ──────────────────
//
// A uniform int64 in [min, max], drawn from a stream of 32-bit outputs,
// identically in every language of the repository; all three generators
// share it.
//
// span = max - min + 1 lies between 1 and 2^64. Taking r % span of a raw draw
// over-samples small offsets whenever the number of possible draws is not a
// multiple of span; rejecting the `threshold` lowest draws removes exactly the
// excess. Three regimes, chosen by span alone:
//
//     span <= 2^32    draw 32 bits,  threshold = (2^32 - span) mod span
//     span <  2^64    draw 64 bits,  threshold = (2^64 - span) mod span
//     span = 2^64     the whole int64 domain: every uint64 is a valid offset
//
// F#'s default arithmetic is unchecked, so uint64 wraps: the whole domain's
// span, 2^64, wraps to 0, and min + offset wraps back into [min, max].
//
// v1 took the threshold modulo 2^64 while drawing 32 bits: biased, never more
// than 2^32 - 1 above min, a division by zero for the whole domain, and for
// spans such as 2^63 + 1 every draw was rejected, so it never returned.
[<AutoOpen>]
module private Helpers =
    let intInRange (next32: unit -> uint32) (min: int64) (max: int64) : int64 =
        if min > max then
            invalidArg (nameof min) $"NextIntInRange requires min <= max, got {min} > {max}"

        let span = uint64 max - uint64 min + 1UL // 2^64 wraps to 0
        let twoTo32 = 1UL <<< 32
        let next64 () =
            let hi = uint64 (next32 ())
            (hi <<< 32) ||| uint64 (next32 ())

        let offset =
            if span = 0UL then
                next64 ()
            elif span <= twoTo32 then
                let threshold = (twoTo32 - span) % span
                let mutable r = uint64 (next32 ())
                while r < threshold do
                    r <- uint64 (next32 ())
                r % span
            else
                let threshold = (0UL - span) % span // (2^64 - span) mod span
                let mutable r = next64 ()
                while r < threshold do
                    r <- next64 ()
                r % span

        int64 (uint64 min + offset)

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

    /// A uniform int64 in [min, max]; throws ArgumentException if min > max, before any draw.
    member this.NextIntInRange(min: int64, max: int64) = intInRange this.NextU32 min max

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

    /// A uniform int64 in [min, max]; throws ArgumentException if min > max, before any draw.
    member this.NextIntInRange(min: int64, max: int64) = intInRange this.NextU32 min max

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

    /// A uniform int64 in [min, max]; throws ArgumentException if min > max, before any draw.
    member this.NextIntInRange(min: int64, max: int64) = intInRange this.NextU32 min max
