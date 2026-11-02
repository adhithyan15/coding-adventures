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
    member _.``ctSelectBytes treats any non-zero stored bool as true``() =
        // Unsafe code can store a non-0/1 byte in a bool; it still means true.
        let mutable stored = 2uy
        let oddTrue = System.Runtime.CompilerServices.Unsafe.As<byte, bool>(&stored)
        Assert.Equal<byte array>([| 7uy |], CtCompare.ctSelectBytes [| 7uy |] [| 8uy |] oddTrue)

    [<Fact>]
    member _.``null arrays are rejected, never treated as empty``() =
        let missing: byte array = null
        Assert.Throws<ArgumentNullException>(fun () -> CtCompare.ctEq missing [||] |> ignore) |> ignore
        Assert.Throws<ArgumentNullException>(fun () -> CtCompare.ctEq [||] missing |> ignore) |> ignore
        Assert.Throws<ArgumentNullException>(fun () -> CtCompare.ctSelectBytes missing [||] true |> ignore) |> ignore
        Assert.Throws<ArgumentNullException>(fun () -> CtCompare.ctSelectBytes [||] missing false |> ignore) |> ignore

    [<Fact>]
    member _.``ctEq returns false on a length mismatch``() =
        Assert.False(CtCompare.ctEq [| 1uy |] [| 1uy; 2uy |])

    [<Fact>]
    member _.``ctSelectBytes returns a fresh copy``() =
        let left = [| 1uy; 2uy; 3uy |]
        let picked = CtCompare.ctSelectBytes left [| 4uy; 5uy; 6uy |] true
        Assert.NotSame(left, picked)
        left[0] <- 99uy
        Assert.Equal<byte array>([| 1uy; 2uy; 3uy |], picked)

    [<Fact>]
    member _.``ctEqUInt64 handles edges``() =
        Assert.True(CtCompare.ctEqUInt64 0UL 0UL)
        Assert.True(CtCompare.ctEqUInt64 UInt64.MaxValue UInt64.MaxValue)
        Assert.False(CtCompare.ctEqUInt64 0UL (1UL <<< 63))

        let baseline = 0x123456789ABCDEF0UL
        for bit in 0 .. 63 do
            Assert.False(CtCompare.ctEqUInt64 baseline (baseline ^^^ (1UL <<< bit)))
