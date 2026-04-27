namespace CodingAdventures.Md5

open System
open System.Buffers.Binary
open System.Numerics
open System.Text

// Md5.fs -- RFC 1321 digests with explicit little-endian block parsing
// ====================================================================
//
// MD5 is built from 64-byte blocks, four 32-bit state words, and arithmetic
// modulo 2^32. Its most error-prone detail is endianness: unlike SHA-1 and
// SHA-256, MD5 reads and writes words in little-endian order.

[<RequireQualifiedAccess>]
module Md5 =
    [<Literal>]
    let VERSION = "0.1.0"

    let private initA = 0x67452301u
    let private initB = 0xefcdab89u
    let private initC = 0x98badcfeu
    let private initD = 0x10325476u

    let private tTable =
        Array.init 64 (fun index ->
            uint32 (Math.Floor(Math.Abs(Math.Sin(float (index + 1))) * 4294967296.0)))

    let private shiftTable =
        [|
            7; 12; 17; 22; 7; 12; 17; 22; 7; 12; 17; 22; 7; 12; 17; 22
            5; 9; 14; 20; 5; 9; 14; 20; 5; 9; 14; 20; 5; 9; 14; 20
            4; 11; 16; 23; 4; 11; 16; 23; 4; 11; 16; 23; 4; 11; 16; 23
            6; 10; 15; 21; 6; 10; 15; 21; 6; 10; 15; 21; 6; 10; 15; 21
        |]

    let private wrap32 (value: uint64) = uint32 (value &&& 0xFFFFFFFFUL)

    let private add32 (left: uint32) (right: uint32) =
        wrap32 (uint64 left + uint64 right)

    let private add32_4 (a: uint32) (b: uint32) (c: uint32) (d: uint32) =
        wrap32 (uint64 a + uint64 b + uint64 c + uint64 d)

    let toHex (bytes: byte array) =
        if isNull bytes then
            nullArg "bytes"

        let builder = StringBuilder(bytes.Length * 2)

        for value in bytes do
            builder.Append(value.ToString("x2")) |> ignore

        builder.ToString()

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
        BinaryPrimitives.WriteUInt64LittleEndian(result.AsSpan(result.Length - 8), bitLength)
        result

    let private stateToBytes a b c d =
        let digest = Array.zeroCreate<byte> 16
        BinaryPrimitives.WriteUInt32LittleEndian(digest.AsSpan(0, 4), a)
        BinaryPrimitives.WriteUInt32LittleEndian(digest.AsSpan(4, 4), b)
        BinaryPrimitives.WriteUInt32LittleEndian(digest.AsSpan(8, 4), c)
        BinaryPrimitives.WriteUInt32LittleEndian(digest.AsSpan(12, 4), d)
        digest

    let private compressState stateA stateB stateC stateD (block: ReadOnlySpan<byte>) =
        let words = Array.zeroCreate<uint32> 16

        for index in 0 .. 15 do
            words[index] <- BinaryPrimitives.ReadUInt32LittleEndian(block.Slice(index * 4, 4))

        let mutable a = stateA
        let mutable b = stateB
        let mutable c = stateC
        let mutable d = stateD

        for index in 0 .. 63 do
            let f, g =
                if index < 16 then
                    ((b &&& c) ||| ((~~~b) &&& d), index)
                elif index < 32 then
                    ((d &&& b) ||| ((~~~d) &&& c), (5 * index + 1) % 16)
                elif index < 48 then
                    (b ^^^ c ^^^ d, (3 * index + 5) % 16)
                else
                    (c ^^^ (b ||| (~~~d)), (7 * index) % 16)

            let inner = add32_4 a f words[g] tTable[index]
            let temp = add32 b (BitOperations.RotateLeft(inner, shiftTable[index]))
            a <- d
            d <- c
            c <- b
            b <- temp

        add32 stateA a, add32 stateB b, add32 stateC c, add32 stateD d

    let private finalizeDigest a b c d (buffer: byte array) bufferLength byteCount =
        let afterBit = (bufferLength + 1) % 64

        let zeroCount =
            if afterBit <= 56 then
                56 - afterBit
            else
                64 + 56 - afterBit

        let finalBlock = Array.zeroCreate<byte> (bufferLength + 1 + zeroCount + 8)
        Array.Copy(buffer, finalBlock, bufferLength)
        finalBlock[bufferLength] <- 0x80uy
        BinaryPrimitives.WriteUInt64LittleEndian(finalBlock.AsSpan(finalBlock.Length - 8), byteCount * 8UL)

        let mutable aState = a
        let mutable bState = b
        let mutable cState = c
        let mutable dState = d

        for offset in 0 .. 64 .. finalBlock.Length - 64 do
            let nextA, nextB, nextC, nextD =
                compressState aState bState cState dState (ReadOnlySpan<byte>(finalBlock, offset, 64))

            aState <- nextA
            bState <- nextB
            cState <- nextC
            dState <- nextD

        stateToBytes aState bState cState dState

    /// Compute the 16-byte MD5 digest of the provided data.
    let sumMd5 (data: byte array) =
        if isNull data then
            nullArg "data"

        let padded = pad data
        let mutable a = initA
        let mutable b = initB
        let mutable c = initC
        let mutable d = initD

        for offset in 0 .. 64 .. padded.Length - 64 do
            let nextA, nextB, nextC, nextD = compressState a b c d (ReadOnlySpan<byte>(padded, offset, 64))
            a <- nextA
            b <- nextB
            c <- nextC
            d <- nextD

        stateToBytes a b c d

    /// Compute MD5 and render it as a 32-character lowercase hexadecimal string.
    let hexString (data: byte array) =
        sumMd5 data |> toHex

    /// Streaming MD5 hasher that accepts data in multiple chunks.
    type Md5Hasher
        (
            initialA: uint32,
            initialB: uint32,
            initialC: uint32,
            initialD: uint32,
            initialBuffer: byte array,
            initialBufferLength: int,
            initialByteCount: uint64
        ) =
        let mutable a = initialA
        let mutable b = initialB
        let mutable c = initialC
        let mutable d = initialD
        let buffer = Array.zeroCreate<byte> 64
        let mutable bufferLength = initialBufferLength
        let mutable byteCount = initialByteCount

        do
            Array.Copy(initialBuffer, buffer, initialBuffer.Length)

        new () = Md5Hasher(initA, initB, initC, initD, Array.zeroCreate<byte> 64, 0, 0UL)

        /// Feed more bytes into the hash state.
        member this.Update(data: byte array) =
            if isNull data then
                nullArg "data"

            byteCount <- byteCount + uint64 data.Length
            let mutable offset = 0

            if bufferLength > 0 then
                let needed = 64 - bufferLength
                let take = min needed data.Length
                Array.Copy(data, 0, buffer, bufferLength, take)
                bufferLength <- bufferLength + take
                offset <- take

                if bufferLength = 64 then
                    let nextA, nextB, nextC, nextD =
                        compressState a b c d (ReadOnlySpan<byte>(buffer, 0, 64))

                    a <- nextA
                    b <- nextB
                    c <- nextC
                    d <- nextD
                    bufferLength <- 0

            while offset + 64 <= data.Length do
                let nextA, nextB, nextC, nextD =
                    compressState a b c d (ReadOnlySpan<byte>(data, offset, 64))

                a <- nextA
                b <- nextB
                c <- nextC
                d <- nextD
                offset <- offset + 64

            if offset < data.Length then
                Array.Copy(data, offset, buffer, bufferLength, data.Length - offset)
                bufferLength <- bufferLength + (data.Length - offset)

            this

        /// Return the current digest without mutating the hasher state.
        member _.Digest() =
            let bufferCopy = Array.zeroCreate<byte> bufferLength
            Array.Copy(buffer, bufferCopy, bufferLength)
            finalizeDigest a b c d bufferCopy bufferLength byteCount

        /// Alias for Digest.
        member this.SumMd5() = this.Digest()

        /// Return the current digest as lowercase hexadecimal.
        member this.HexDigest() =
            this.Digest() |> toHex

        /// Clone the current streaming state.
        member _.Copy() =
            Md5Hasher(a, b, c, d, Array.copy buffer, bufferLength, byteCount)
