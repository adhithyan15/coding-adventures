namespace CodingAdventures.Sha256.FSharp

open System
open System.Buffers.Binary
open System.Collections.Generic
open System.Numerics

[<RequireQualifiedAccess>]
module Sha256 =
    [<Literal>]
    let DigestLength = 32

    let private init =
        [|
            0x6a09e667u; 0xbb67ae85u; 0x3c6ef372u; 0xa54ff53au
            0x510e527fu; 0x9b05688cu; 0x1f83d9abu; 0x5be0cd19u
        |]

    let private k =
        [|
            0x428a2f98u; 0x71374491u; 0xb5c0fbcfu; 0xe9b5dba5u; 0x3956c25bu; 0x59f111f1u; 0x923f82a4u; 0xab1c5ed5u
            0xd807aa98u; 0x12835b01u; 0x243185beu; 0x550c7dc3u; 0x72be5d74u; 0x80deb1feu; 0x9bdc06a7u; 0xc19bf174u
            0xe49b69c1u; 0xefbe4786u; 0x0fc19dc6u; 0x240ca1ccu; 0x2de92c6fu; 0x4a7484aau; 0x5cb0a9dcu; 0x76f988dau
            0x983e5152u; 0xa831c66du; 0xb00327c8u; 0xbf597fc7u; 0xc6e00bf3u; 0xd5a79147u; 0x06ca6351u; 0x14292967u
            0x27b70a85u; 0x2e1b2138u; 0x4d2c6dfcu; 0x53380d13u; 0x650a7354u; 0x766a0abbu; 0x81c2c92eu; 0x92722c85u
            0xa2bfe8a1u; 0xa81a664bu; 0xc24b8b70u; 0xc76c51a3u; 0xd192e819u; 0xd6990624u; 0xf40e3585u; 0x106aa070u
            0x19a4c116u; 0x1e376c08u; 0x2748774cu; 0x34b0bcb5u; 0x391c0cb3u; 0x4ed8aa4au; 0x5b9cca4fu; 0x682e6ff3u
            0x748f82eeu; 0x78a5636fu; 0x84c87814u; 0x8cc70208u; 0x90befffau; 0xa4506cebu; 0xbef9a3f7u; 0xc67178f2u
        |]

    let private wrap32 value =
        uint32 (value &&& 0xffffffffUL)

    let private add2 a b =
        wrap32 (uint64 a + uint64 b)

    let private add4 a b c d =
        wrap32 (uint64 a + uint64 b + uint64 c + uint64 d)

    let private add5 a b c d e =
        wrap32 (uint64 a + uint64 b + uint64 c + uint64 d + uint64 e)

    let private bigSigma0 (x: uint32) =
        BitOperations.RotateRight(x, 2) ^^^ BitOperations.RotateRight(x, 13) ^^^ BitOperations.RotateRight(x, 22)

    let private bigSigma1 (x: uint32) =
        BitOperations.RotateRight(x, 6) ^^^ BitOperations.RotateRight(x, 11) ^^^ BitOperations.RotateRight(x, 25)

    let private smallSigma0 (x: uint32) =
        BitOperations.RotateRight(x, 7) ^^^ BitOperations.RotateRight(x, 18) ^^^ (x >>> 3)

    let private smallSigma1 (x: uint32) =
        BitOperations.RotateRight(x, 17) ^^^ BitOperations.RotateRight(x, 19) ^^^ (x >>> 10)

    let private ch (x: uint32) (y: uint32) (z: uint32) = (x &&& y) ^^^ ((~~~x) &&& z)

    let private maj (x: uint32) (y: uint32) (z: uint32) = (x &&& y) ^^^ (x &&& z) ^^^ (y &&& z)

    let private pad (data: byte array) =
        let bitLength = uint64 data.Length * 8UL
        let afterBit = (data.Length + 1) % 64

        let zeroCount =
            if afterBit <= 56 then
                56 - afterBit
            else
                64 + 56 - afterBit

        let result = Array.zeroCreate<byte> (data.Length + 1 + zeroCount + 8)
        Array.Copy(data, result, data.Length)
        result[data.Length] <- 0x80uy
        BinaryPrimitives.WriteUInt64BigEndian(result.AsSpan(result.Length - 8), bitLength)
        result

    let private stateToBytes (state: uint32 array) =
        let digest = Array.zeroCreate<byte> DigestLength

        for index in 0 .. state.Length - 1 do
            BinaryPrimitives.WriteUInt32BigEndian(digest.AsSpan(index * 4, 4), state[index])

        digest

    let private compress (state: uint32 array) (block: ReadOnlySpan<byte>) =
        let w = Array.zeroCreate<uint32> 64

        for index in 0 .. 15 do
            w[index] <- BinaryPrimitives.ReadUInt32BigEndian(block.Slice(index * 4, 4))

        for index in 16 .. 63 do
            w[index] <- add4 (smallSigma1 w[index - 2]) w[index - 7] (smallSigma0 w[index - 15]) w[index - 16]

        let mutable a = state[0]
        let mutable b = state[1]
        let mutable c = state[2]
        let mutable d = state[3]
        let mutable e = state[4]
        let mutable f = state[5]
        let mutable g = state[6]
        let mutable h = state[7]

        for index in 0 .. 63 do
            let t1 = add5 h (bigSigma1 e) (ch e f g) k[index] w[index]
            let t2 = add2 (bigSigma0 a) (maj a b c)
            h <- g
            g <- f
            f <- e
            e <- add2 d t1
            d <- c
            c <- b
            b <- a
            a <- add2 t1 t2

        state[0] <- add2 state[0] a
        state[1] <- add2 state[1] b
        state[2] <- add2 state[2] c
        state[3] <- add2 state[3] d
        state[4] <- add2 state[4] e
        state[5] <- add2 state[5] f
        state[6] <- add2 state[6] g
        state[7] <- add2 state[7] h

    let hash (data: byte array) =
        if isNull data then
            nullArg "data"

        let padded = pad data
        let state = Array.copy init

        for offset in 0 .. 64 .. padded.Length - 64 do
            compress state (ReadOnlySpan<byte>(padded, offset, 64))

        stateToBytes state

    let hashHex (data: byte array) =
        hash data |> Convert.ToHexString |> fun value -> value.ToLowerInvariant()

type Sha256Hasher() =
    let data = List<byte>()

    member this.Update(bytes: byte array) =
        if isNull bytes then
            nullArg "bytes"

        data.AddRange(bytes)
        this

    member _.Digest() =
        data.ToArray() |> Sha256.hash

    member this.HexDigest() =
        this.Digest() |> Convert.ToHexString |> fun value -> value.ToLowerInvariant()

    member _.Copy() =
        let copy = Sha256Hasher()
        copy.Update(data.ToArray()) |> ignore
        copy
