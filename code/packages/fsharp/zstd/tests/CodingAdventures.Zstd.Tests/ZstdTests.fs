namespace CodingAdventures.Zstd.FSharp.Tests

open System
open System.Collections.Generic
open System.IO
open System.Text
open CodingAdventures.Zstd.FSharp
open Xunit

type ZstdTests() =
    let blockTypes (frame: byte array) =
        let mutable position = 13
        let result = ResizeArray<int>()
        let mutable last = false

        while not last do
            let header = int frame[position] ||| (int frame[position + 1] <<< 8) ||| (int frame[position + 2] <<< 16)
            position <- position + 3
            last <- (header &&& 1) <> 0
            let blockType = (header >>> 1) &&& 3
            let size = header >>> 3
            result.Add blockType
            position <- position + (if blockType = 1 then 1 else size)

        result.ToArray()

    [<Fact>]
    member _.``empty input round trips and produces a frame``() =
        let compressed = Zstd.Compress [||]
        Assert.Empty(Zstd.Decompress compressed)
        Assert.Equal(16, compressed.Length)
        Assert.Equal<byte array>([| 0x28uy; 0xB5uy; 0x2Fuy; 0xFDuy |], compressed[..3])

    [<Theory>]
    [<InlineData(0)>]
    [<InlineData(0x42)>]
    [<InlineData(0xFF)>]
    member _.``single bytes round trip``(value: int) =
        let input = [| byte value |]
        Assert.Equal<byte array>(input, Zstd.Decompress(Zstd.Compress input))

    [<Fact>]
    member _.``all byte values round trip as raw data``() =
        let input = Array.init 256 byte
        let compressed = Zstd.Compress input
        Assert.Equal<byte array>(input, Zstd.Decompress compressed)
        Assert.Equal<int array>([| 0 |], blockTypes compressed)

    [<Theory>]
    [<InlineData(0)>]
    [<InlineData(0x41)>]
    [<InlineData(0xFF)>]
    member _.``repeated bytes use RLE``(value: int) =
        let input = Array.create 1024 (byte value)
        let compressed = Zstd.Compress input
        Assert.Equal<byte array>(input, Zstd.Decompress compressed)
        Assert.True(compressed.Length < 30)
        Assert.Equal<int array>([| 1 |], blockTypes compressed)

    [<Fact>]
    member _.``English prose uses compressed blocks``() =
        let input = Encoding.UTF8.GetBytes(String.concat "" (List.replicate 25 "the quick brown fox jumps over the lazy dog "))
        let compressed = Zstd.Compress input
        Assert.Equal<byte array>(input, Zstd.Decompress compressed)
        Assert.True(float compressed.Length < float input.Length * 0.8)
        Assert.Equal<int array>([| 2 |], blockTypes compressed)

    [<Fact>]
    member _.``deterministic binary data round trips``() =
        let mutable seed = 42u
        let input = Array.zeroCreate<byte> 512

        for index in 0 .. input.Length - 1 do
            seed <- seed * 1_664_525u + 1_013_904_223u
            input[index] <- byte seed

        Assert.Equal<byte array>(input, Zstd.Decompress(Zstd.Compress input))

    [<Fact>]
    member _.``multi-block RLE frame round trips``() =
        let input = Array.create (200 * 1024) (byte 'x')
        let compressed = Zstd.Compress input
        Assert.Equal<byte array>(input, Zstd.Decompress compressed)
        Assert.Equal<int array>([| 1; 1 |], blockTypes compressed)
        Assert.True(compressed.Length < 50)

    [<Fact>]
    member _.``multi-block compressed frame round trips``() =
        let input = Encoding.ASCII.GetBytes(String.concat "" (List.replicate 10_000 "ABCDEFGHIJKLMNOP"))
        let compressed = Zstd.Compress input
        let types = blockTypes compressed
        Assert.Equal<byte array>(input, Zstd.Decompress compressed)
        Assert.True(types.Length > 1)
        Assert.All<int>(types, fun blockType -> Assert.Equal(2, blockType))

    [<Fact>]
    member _.``repeat distance pattern compresses efficiently``() =
        let bytes = ResizeArray<byte>(Encoding.ASCII.GetBytes "ABCDEFGH")

        for _ in 0..9 do
            bytes.AddRange(Array.create 128 (byte 'X'))
            bytes.AddRange(Encoding.ASCII.GetBytes "ABCDEFGH")

        let input = bytes.ToArray()
        let compressed = Zstd.Compress input
        Assert.Equal<byte array>(input, Zstd.Decompress compressed)
        Assert.True(float compressed.Length < float input.Length * 0.7)

    [<Fact>]
    member _.``compression is deterministic``() =
        let input = Encoding.ASCII.GetBytes(String.concat "" (List.replicate 50 "hello zstd world! "))
        Assert.Equal<byte array>(Zstd.Compress input, Zstd.Compress input)

    [<Fact>]
    member _.``hand-crafted raw frame decodes``() =
        let frame = [| 0x28uy; 0xB5uy; 0x2Fuy; 0xFDuy; 0x20uy; 0x05uy; 0x29uy; 0uy; 0uy; byte 'h'; byte 'e'; byte 'l'; byte 'l'; byte 'o' |]
        Assert.Equal<byte array>(Encoding.ASCII.GetBytes "hello", Zstd.Decompress frame)

    [<Fact>]
    member _.``hand-crafted RLE frame decodes``() =
        let frame = [| 0x28uy; 0xB5uy; 0x2Fuy; 0xFDuy; 0x20uy; 0x0Auy; 0x53uy; 0uy; 0uy; 0x41uy |]
        Assert.Equal<byte array>(Array.create 10 (byte 'A'), Zstd.Decompress frame)

    [<Fact>]
    member _.``multi-segment headers and checksums are consumed``() =
        let frame = [| 0x28uy; 0xB5uy; 0x2Fuy; 0xFDuy; 0x10uy; 0uy; 0x09uy; 0uy; 0uy; byte 'x'; 1uy; 2uy; 3uy; 4uy |]
        Assert.Equal<byte array>([| byte 'x' |], Zstd.Decompress frame)

    [<Fact>]
    member _.``dictionary ID and content-size forms are skipped``() =
        for descriptor in [| 0x21uy; 0x62uy; 0xA3uy |] do
            let dictionaryBytes = match descriptor &&& 3uy with | 1uy -> 1 | 2uy -> 2 | _ -> 4
            let contentBytes = match descriptor >>> 6 with | 0uy -> 1 | 1uy -> 2 | 2uy -> 4 | _ -> 8
            let frame = ResizeArray<byte>([| 0x28uy; 0xB5uy; 0x2Fuy; 0xFDuy; descriptor |])
            frame.AddRange(Array.zeroCreate (dictionaryBytes + contentBytes))
            frame.AddRange([| 1uy; 0uy; 0uy |])
            Assert.Empty(Zstd.Decompress(frame.ToArray()))

    [<Fact>]
    member _.``null inputs are rejected``() =
        let nullBytes = Unchecked.defaultof<byte array>
        Assert.Throws<ArgumentNullException>(fun () -> Zstd.Compress nullBytes |> ignore) |> ignore
        Assert.Throws<ArgumentNullException>(fun () -> Zstd.Decompress nullBytes |> ignore) |> ignore

    [<Fact>]
    member _.``malformed frames are rejected``() =
        let malformed =
            [| [||]
               [| 0x28uy; 0xB5uy; 0x2Fuy |]
               Encoding.ASCII.GetBytes "not zstd data"
               [| 0x28uy; 0xB5uy; 0x2Fuy; 0xFDuy; 0x2Cuy |]
               [| 0x28uy; 0xB5uy; 0x2Fuy; 0xFDuy; 0x20uy |]
               [| 0x28uy; 0xB5uy; 0x2Fuy; 0xFDuy; 0x20uy; 0uy; 0x29uy; 0uy; 0uy; byte 'h' |]
               [| 0x28uy; 0xB5uy; 0x2Fuy; 0xFDuy; 0x20uy; 0uy; 0x53uy; 0uy; 0uy |]
               [| 0x28uy; 0xB5uy; 0x2Fuy; 0xFDuy; 0x20uy; 0uy; 0x07uy; 0uy; 0uy |]
               [| 0x28uy; 0xB5uy; 0x2Fuy; 0xFDuy; 0x20uy; 0uy; 0x01uy; 0uy; 0uy; 0xFFuy |]
               [| 0x28uy; 0xB5uy; 0x2Fuy; 0xFDuy; 0x20uy; 0uy; 0x05uy; 0uy; 0uy |] |]

        for frame in malformed do
            Assert.Throws<InvalidDataException>(fun () -> Zstd.Decompress frame |> ignore) |> ignore
