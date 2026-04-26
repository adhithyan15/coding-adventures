namespace CodingAdventures.Rng.Tests

open System
open System.Collections.Generic
open CodingAdventures.Rng
open Xunit

type LcgTests() =
    [<Fact>]
    member _.NextU32MatchesKnownVectorsSeed1() =
        let generator = Lcg 1UL
        Assert.Equal(1817669548u, generator.NextU32())
        Assert.Equal(2187888307u, generator.NextU32())
        Assert.Equal(2784682393u, generator.NextU32())

    [<Fact>]
    member _.SameSeedIsDeterministicAndDifferentSeedsDiverge() =
        let left = Lcg 0UL
        let right = Lcg 0UL

        for _ in 1 .. 10 do
            Assert.Equal(left.NextU32(), right.NextU32())

        Assert.NotEqual((Lcg 1UL).NextU32(), (Lcg 2UL).NextU32())

    [<Fact>]
    member _.NextU64ComposesTwoU32Draws() =
        let combined = (Lcg 1UL).NextU64()
        let reference = Lcg 1UL
        let hi = uint64 (reference.NextU32())
        let lo = uint64 (reference.NextU32())
        Assert.Equal((hi <<< 32) ||| lo, combined)

    [<Fact>]
    member _.FloatAndRangeHelpersRespectBounds() =
        let floatGenerator = Lcg 42UL

        for _ in 1 .. 1000 do
            let value = floatGenerator.NextFloat()
            Assert.True(value >= 0.0 && value < 1.0)

        let rangeGenerator = Lcg 7UL
        let seen = HashSet<int64>()

        for _ in 1 .. 10000 do
            let value = rangeGenerator.NextIntInRange(1L, 6L)
            Assert.InRange(value, 1L, 6L)
            seen.Add value |> ignore

        for value in 1L .. 6L do
            Assert.Contains(value, seen)

        Assert.Equal(42L, (Lcg 123UL).NextIntInRange(42L, 42L))
        Assert.InRange((Lcg 5UL).NextIntInRange(-10L, -1L), -10L, -1L)
        Assert.Throws<ArgumentException>(fun () -> (Lcg 1UL).NextIntInRange(6L, 1L) |> ignore) |> ignore

type Xorshift64Tests() =
    [<Fact>]
    member _.NextU32MatchesKnownVectorsSeed1() =
        let generator = Xorshift64 1UL
        Assert.Equal(1082269761u, generator.NextU32())
        Assert.Equal(201397313u, generator.NextU32())
        Assert.Equal(1854285353u, generator.NextU32())

    [<Fact>]
    member _.SeedZeroIsReplacedWithOneAndSequencesAreDeterministic() =
        let zero = Xorshift64 0UL
        let one = Xorshift64 1UL
        Assert.Equal(one.NextU32(), zero.NextU32())
        Assert.Equal(one.NextU32(), zero.NextU32())

        let left = Xorshift64 99UL
        let right = Xorshift64 99UL

        for _ in 1 .. 20 do
            Assert.Equal(left.NextU32(), right.NextU32())

    [<Fact>]
    member _.NextU64ComposesTwoU32Draws() =
        let combined = (Xorshift64 17UL).NextU64()
        let reference = Xorshift64 17UL
        let hi = uint64 (reference.NextU32())
        let lo = uint64 (reference.NextU32())
        Assert.Equal((hi <<< 32) ||| lo, combined)

    [<Fact>]
    member _.FloatAndRangeHelpersRespectBounds() =
        let floatGenerator = Xorshift64 42UL

        for _ in 1 .. 1000 do
            let value = floatGenerator.NextFloat()
            Assert.True(value >= 0.0 && value < 1.0)

        let rangeGenerator = Xorshift64 3UL
        let seen = HashSet<int64>()

        for _ in 1 .. 10000 do
            let value = rangeGenerator.NextIntInRange(1L, 6L)
            Assert.InRange(value, 1L, 6L)
            seen.Add value |> ignore

        for value in 1L .. 6L do
            Assert.Contains(value, seen)

        Assert.Equal(-5L, (Xorshift64 7UL).NextIntInRange(-5L, -5L))
        Assert.Throws<ArgumentException>(fun () -> (Xorshift64 1UL).NextIntInRange(6L, 1L) |> ignore) |> ignore

type Pcg32Tests() =
    [<Fact>]
    member _.NextU32MatchesKnownVectorsSeed1() =
        let generator = Pcg32 1UL
        Assert.Equal(1412771199u, generator.NextU32())
        Assert.Equal(1791099446u, generator.NextU32())
        Assert.Equal(124312908u, generator.NextU32())

    [<Fact>]
    member _.SameSeedIsDeterministicAndDifferentSeedsDiverge() =
        let left = Pcg32 0UL
        let right = Pcg32 0UL

        for _ in 1 .. 10 do
            Assert.Equal(left.NextU32(), right.NextU32())

        Assert.NotEqual((Pcg32 1UL).NextU32(), (Pcg32 2UL).NextU32())

    [<Fact>]
    member _.NextU64ComposesTwoU32Draws() =
        let combined = (Pcg32 55UL).NextU64()
        let reference = Pcg32 55UL
        let hi = uint64 (reference.NextU32())
        let lo = uint64 (reference.NextU32())
        Assert.Equal((hi <<< 32) ||| lo, combined)

    [<Fact>]
    member _.FloatAndRangeHelpersRespectBounds() =
        let floatGenerator = Pcg32 42UL
        let mutable sawLow = false
        let mutable sawHigh = false

        for _ in 1 .. 1000 do
            let value = floatGenerator.NextFloat()
            Assert.True(value >= 0.0 && value < 1.0)
            if value < 0.5 then sawLow <- true else sawHigh <- true

        Assert.True(sawLow && sawHigh)

        let rangeGenerator = Pcg32 9UL
        let seen = HashSet<int64>()

        for _ in 1 .. 10000 do
            let value = rangeGenerator.NextIntInRange(1L, 6L)
            Assert.InRange(value, 1L, 6L)
            seen.Add value |> ignore

        for value in 1L .. 6L do
            Assert.Contains(value, seen)

        Assert.Equal(100L, (Pcg32 111UL).NextIntInRange(100L, 100L))
        Assert.InRange((Pcg32 22UL).NextIntInRange(-20L, -10L), -20L, -10L)
        Assert.Throws<ArgumentException>(fun () -> (Pcg32 1UL).NextIntInRange(6L, 1L) |> ignore) |> ignore

    [<Fact>]
    member _.AllThreeGeneratorsProduceDifferentOutputsForSameSeed() =
        let lcg = (Lcg 1UL).NextU32()
        let xorshift = (Xorshift64 1UL).NextU32()
        let pcg = (Pcg32 1UL).NextU32()

        Assert.NotEqual(lcg, xorshift)
        Assert.NotEqual(xorshift, pcg)
        Assert.NotEqual(lcg, pcg)
