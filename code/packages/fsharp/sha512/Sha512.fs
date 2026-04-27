namespace CodingAdventures.Sha512.FSharp

open System
open System.Buffers.Binary
open System.Collections.Generic
open System.Numerics

[<RequireQualifiedAccess>]
module Sha512 =
    [<Literal>]
    let DigestLength = 64

    let private init =
        [|
            0x6a09e667f3bcc908UL; 0xbb67ae8584caa73bUL; 0x3c6ef372fe94f82bUL; 0xa54ff53a5f1d36f1UL
            0x510e527fade682d1UL; 0x9b05688c2b3e6c1fUL; 0x1f83d9abfb41bd6bUL; 0x5be0cd19137e2179UL
        |]

    let private k =
        [|
            0x428a2f98d728ae22UL; 0x7137449123ef65cdUL; 0xb5c0fbcfec4d3b2fUL; 0xe9b5dba58189dbbcUL
            0x3956c25bf348b538UL; 0x59f111f1b605d019UL; 0x923f82a4af194f9bUL; 0xab1c5ed5da6d8118UL
            0xd807aa98a3030242UL; 0x12835b0145706fbeUL; 0x243185be4ee4b28cUL; 0x550c7dc3d5ffb4e2UL
            0x72be5d74f27b896fUL; 0x80deb1fe3b1696b1UL; 0x9bdc06a725c71235UL; 0xc19bf174cf692694UL
            0xe49b69c19ef14ad2UL; 0xefbe4786384f25e3UL; 0x0fc19dc68b8cd5b5UL; 0x240ca1cc77ac9c65UL
            0x2de92c6f592b0275UL; 0x4a7484aa6ea6e483UL; 0x5cb0a9dcbd41fbd4UL; 0x76f988da831153b5UL
            0x983e5152ee66dfabUL; 0xa831c66d2db43210UL; 0xb00327c898fb213fUL; 0xbf597fc7beef0ee4UL
            0xc6e00bf33da88fc2UL; 0xd5a79147930aa725UL; 0x06ca6351e003826fUL; 0x142929670a0e6e70UL
            0x27b70a8546d22ffcUL; 0x2e1b21385c26c926UL; 0x4d2c6dfc5ac42aedUL; 0x53380d139d95b3dfUL
            0x650a73548baf63deUL; 0x766a0abb3c77b2a8UL; 0x81c2c92e47edaee6UL; 0x92722c851482353bUL
            0xa2bfe8a14cf10364UL; 0xa81a664bbc423001UL; 0xc24b8b70d0f89791UL; 0xc76c51a30654be30UL
            0xd192e819d6ef5218UL; 0xd69906245565a910UL; 0xf40e35855771202aUL; 0x106aa07032bbd1b8UL
            0x19a4c116b8d2d0c8UL; 0x1e376c085141ab53UL; 0x2748774cdf8eeb99UL; 0x34b0bcb5e19b48a8UL
            0x391c0cb3c5c95a63UL; 0x4ed8aa4ae3418acbUL; 0x5b9cca4f7763e373UL; 0x682e6ff3d6b2b8a3UL
            0x748f82ee5defb2fcUL; 0x78a5636f43172f60UL; 0x84c87814a1f0ab72UL; 0x8cc702081a6439ecUL
            0x90befffa23631e28UL; 0xa4506cebde82bde9UL; 0xbef9a3f7b2c67915UL; 0xc67178f2e372532bUL
            0xca273eceea26619cUL; 0xd186b8c721c0c207UL; 0xeada7dd6cde0eb1eUL; 0xf57d4f7fee6ed178UL
            0x06f067aa72176fbaUL; 0x0a637dc5a2c898a6UL; 0x113f9804bef90daeUL; 0x1b710b35131c471bUL
            0x28db77f523047d84UL; 0x32caab7b40c72493UL; 0x3c9ebe0a15c9bebcUL; 0x431d67c49c100d4cUL
            0x4cc5d4becb3e42b6UL; 0x597f299cfc657e2aUL; 0x5fcb6fab3ad6faecUL; 0x6c44198c4a475817UL
        |]

    let private bigSigma0 (x: uint64) =
        BitOperations.RotateRight(x, 28) ^^^ BitOperations.RotateRight(x, 34) ^^^ BitOperations.RotateRight(x, 39)

    let private bigSigma1 (x: uint64) =
        BitOperations.RotateRight(x, 14) ^^^ BitOperations.RotateRight(x, 18) ^^^ BitOperations.RotateRight(x, 41)

    let private smallSigma0 (x: uint64) =
        BitOperations.RotateRight(x, 1) ^^^ BitOperations.RotateRight(x, 8) ^^^ (x >>> 7)

    let private smallSigma1 (x: uint64) =
        BitOperations.RotateRight(x, 19) ^^^ BitOperations.RotateRight(x, 61) ^^^ (x >>> 6)

    let private ch (x: uint64) (y: uint64) (z: uint64) = (x &&& y) ^^^ ((~~~x) &&& z)

    let private maj (x: uint64) (y: uint64) (z: uint64) = (x &&& y) ^^^ (x &&& z) ^^^ (y &&& z)

    let private pad (data: byte array) =
        let bitLength = uint64 data.Length * 8UL
        let afterBit = (data.Length + 1) % 128

        let zeroCount =
            if afterBit <= 112 then
                112 - afterBit
            else
                128 + 112 - afterBit

        let result = Array.zeroCreate<byte> (data.Length + 1 + zeroCount + 16)
        Array.Copy(data, result, data.Length)
        result[data.Length] <- 0x80uy
        BinaryPrimitives.WriteUInt64BigEndian(result.AsSpan(result.Length - 8), bitLength)
        result

    let private stateToBytes (state: uint64 array) =
        let digest = Array.zeroCreate<byte> DigestLength

        for index in 0 .. state.Length - 1 do
            BinaryPrimitives.WriteUInt64BigEndian(digest.AsSpan(index * 8, 8), state[index])

        digest

    let private compress (state: uint64 array) (block: ReadOnlySpan<byte>) =
        let w = Array.zeroCreate<uint64> 80

        for index in 0 .. 15 do
            w[index] <- BinaryPrimitives.ReadUInt64BigEndian(block.Slice(index * 8, 8))

        for index in 16 .. 79 do
            w[index] <- smallSigma1 w[index - 2] + w[index - 7] + smallSigma0 w[index - 15] + w[index - 16]

        let mutable a = state[0]
        let mutable b = state[1]
        let mutable c = state[2]
        let mutable d = state[3]
        let mutable e = state[4]
        let mutable f = state[5]
        let mutable g = state[6]
        let mutable h = state[7]

        for index in 0 .. 79 do
            let t1 = h + bigSigma1 e + ch e f g + k[index] + w[index]
            let t2 = bigSigma0 a + maj a b c
            h <- g
            g <- f
            f <- e
            e <- d + t1
            d <- c
            c <- b
            b <- a
            a <- t1 + t2

        state[0] <- state[0] + a
        state[1] <- state[1] + b
        state[2] <- state[2] + c
        state[3] <- state[3] + d
        state[4] <- state[4] + e
        state[5] <- state[5] + f
        state[6] <- state[6] + g
        state[7] <- state[7] + h

    let hash (data: byte array) =
        if isNull data then
            nullArg "data"

        let padded = pad data
        let state = Array.copy init

        for offset in 0 .. 128 .. padded.Length - 128 do
            compress state (ReadOnlySpan<byte>(padded, offset, 128))

        stateToBytes state

    let hashHex (data: byte array) =
        hash data |> Convert.ToHexString |> fun value -> value.ToLowerInvariant()

type Sha512Hasher() =
    let data = List<byte>()

    member this.Update(bytes: byte array) =
        if isNull bytes then
            nullArg "bytes"

        data.AddRange(bytes)
        this

    member _.Digest() =
        data.ToArray() |> Sha512.hash

    member this.HexDigest() =
        this.Digest() |> Convert.ToHexString |> fun value -> value.ToLowerInvariant()

    member _.Copy() =
        let copy = Sha512Hasher()
        copy.Update(data.ToArray()) |> ignore
        copy
