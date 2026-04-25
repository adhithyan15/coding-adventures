namespace CodingAdventures.CtCompare.Tests

open System
open Xunit
open CodingAdventures.CtCompare

type CtCompareTests() =
    [<Fact>]
    member _.``ctEq matches byte equality``() =
        Assert.True(CtCompare.ctEq [| 1uy; 2uy; 3uy |] [| 1uy; 2uy; 3uy |])
        Assert.True(CtCompare.ctEq [||] [||])
        Assert.False(CtCompare.ctEq [| 1uy; 2uy; 3uy |] [| 1uy; 2uy; 4uy |])
        Assert.False(CtCompare.ctEq [| 1uy; 2uy; 3uy |] [| 0uy; 2uy; 3uy |])
        Assert.False(CtCompare.ctEq [| 1uy; 2uy; 3uy |] [| 1uy; 2uy; 3uy; 4uy |])

    [<Fact>]
    member _.``ctEq detects every single bit position``() =
        let baseline = Array.create 32 0x42uy
        for index in 0 .. baseline.Length - 1 do
            for bit in 0 .. 7 do
                let flipped = Array.copy baseline
                flipped[index] <- flipped[index] ^^^ byte (1 <<< bit)
                Assert.False(CtCompare.ctEq baseline flipped)

    [<Fact>]
    member _.``ctEqFixed is dynamic alias``() =
        Assert.True(CtCompare.ctEqFixed (Array.zeroCreate 16) (Array.zeroCreate 16))
        let different = Array.zeroCreate<byte> 16
        different[15] <- 1uy
        Assert.False(CtCompare.ctEqFixed (Array.zeroCreate 16) different)

    [<Fact>]
    member _.``ctSelectBytes chooses without mutating inputs``() =
        let left = [| 0uy .. 255uy |]
        let right = Array.rev left
        Assert.Equal<byte array>(left, CtCompare.ctSelectBytes left right true)
        Assert.Equal<byte array>(right, CtCompare.ctSelectBytes left right false)
        Assert.Empty(CtCompare.ctSelectBytes [||] [||] true)
        Assert.Throws<ArgumentException>(fun () -> CtCompare.ctSelectBytes [| 1uy |] [| 1uy; 2uy |] true |> ignore) |> ignore

    [<Fact>]
    member _.``ctEqUInt64 handles edges``() =
        Assert.True(CtCompare.ctEqUInt64 0UL 0UL)
        Assert.True(CtCompare.ctEqUInt64 UInt64.MaxValue UInt64.MaxValue)
        Assert.False(CtCompare.ctEqUInt64 0UL (1UL <<< 63))

        let baseline = 0x123456789ABCDEF0UL
        for bit in 0 .. 63 do
            Assert.False(CtCompare.ctEqUInt64 baseline (baseline ^^^ (1UL <<< bit)))
