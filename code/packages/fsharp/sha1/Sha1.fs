namespace CodingAdventures.Sha1.FSharp

open System
open System.Buffers.Binary
open System.Collections.Generic
open System.Numerics

[<RequireQualifiedAccess>]
module Sha1 =
    [<Literal>]
    let DigestLength = 20

    let private init =
        [|
            0x67452301u
            0xefcdab89u
            0x98badcfeu
            0x10325476u
            0xc3d2e1f0u
        |]

    let private k =
        [| 0x5a827999u; 0x6ed9eba1u; 0x8f1bbcdcu; 0xca62c1d6u |]

    let private wrap32 value =
        uint32 (value &&& 0xffffffffUL)

    let private add32_2 a b =
        wrap32 (uint64 a + uint64 b)

    let private add32_5 a b c d e =
        wrap32 (uint64 a + uint64 b + uint64 c + uint64 d + uint64 e)

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
        let w = Array.zeroCreate<uint32> 80

        for index in 0 .. 15 do
            w[index] <- BinaryPrimitives.ReadUInt32BigEndian(block.Slice(index * 4, 4))

        for index in 16 .. 79 do
            w[index] <- BitOperations.RotateLeft(w[index - 3] ^^^ w[index - 8] ^^^ w[index - 14] ^^^ w[index - 16], 1)

        let mutable a = state[0]
        let mutable b = state[1]
        let mutable c = state[2]
        let mutable d = state[3]
        let mutable e = state[4]

        for index in 0 .. 79 do
            let f, roundK =
                if index < 20 then
                    (b &&& c) ||| ((~~~b) &&& d), k[0]
                elif index < 40 then
                    b ^^^ c ^^^ d, k[1]
                elif index < 60 then
                    (b &&& c) ||| (b &&& d) ||| (c &&& d), k[2]
                else
                    b ^^^ c ^^^ d, k[3]

            let temp = add32_5 (BitOperations.RotateLeft(a, 5)) f e roundK w[index]
            e <- d
            d <- c
            c <- BitOperations.RotateLeft(b, 30)
            b <- a
            a <- temp

        state[0] <- add32_2 state[0] a
        state[1] <- add32_2 state[1] b
        state[2] <- add32_2 state[2] c
        state[3] <- add32_2 state[3] d
        state[4] <- add32_2 state[4] e

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

type Sha1Hasher() =
    let data = List<byte>()

    member this.Update(bytes: byte array) =
        if isNull bytes then
            nullArg "bytes"

        data.AddRange(bytes)
        this

    member _.Digest() =
        data.ToArray() |> Sha1.hash

    member this.HexDigest() =
        this.Digest() |> Convert.ToHexString |> fun value -> value.ToLowerInvariant()

    member _.Copy() =
        let copy = Sha1Hasher()
        copy.Update(data.ToArray()) |> ignore
        copy
