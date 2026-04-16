namespace CodingAdventures.WasmLeb128.FSharp.Tests

open System
open CodingAdventures.WasmLeb128.FSharp
open Xunit

module WasmLeb128Tests =
    [<Fact>]
    let ``has version`` () =
        Assert.Equal("0.1.0", Version.VERSION)

    [<Fact>]
    let ``error is exception subclass`` () =
        let error = LEB128Error("test")
        Assert.IsType<LEB128Error>(error) |> ignore
        Assert.Equal("test", error.Message)

    [<Fact>]
    let ``decode unsigned handles common vectors`` () =
        Assert.Equal((0u, 1), WasmLeb128.decodeUnsigned [| 0x00uy |])
        Assert.Equal((3u, 1), WasmLeb128.decodeUnsigned [| 0x03uy |])
        Assert.Equal((624485u, 3), WasmLeb128.decodeUnsigned [| 0xE5uy; 0x8Euy; 0x26uy |])
        Assert.Equal((UInt32.MaxValue, 5), WasmLeb128.decodeUnsigned [| 0xFFuy; 0xFFuy; 0xFFuy; 0xFFuy; 0x0Fuy |])

    [<Fact>]
    let ``decode signed handles common vectors`` () =
        Assert.Equal((0, 1), WasmLeb128.decodeSigned [| 0x00uy |])
        Assert.Equal((-2, 1), WasmLeb128.decodeSigned [| 0x7Euy |])
        Assert.Equal((Int32.MaxValue, 5), WasmLeb128.decodeSigned [| 0xFFuy; 0xFFuy; 0xFFuy; 0xFFuy; 0x07uy |])
        Assert.Equal((Int32.MinValue, 5), WasmLeb128.decodeSigned [| 0x80uy; 0x80uy; 0x80uy; 0x80uy; 0x78uy |])

    [<Fact>]
    let ``decode supports offsets and validation`` () =
        Assert.Equal((624485u, 3), WasmLeb128.decodeUnsignedAt [| 0xAAuy; 0xE5uy; 0x8Euy; 0x26uy; 0xBBuy |] 1)
        Assert.Equal((-2, 1), WasmLeb128.decodeSignedAt [| 0xFFuy; 0x7Euy; 0x00uy |] 1)
        Assert.Throws<LEB128Error>(fun () -> WasmLeb128.decodeUnsigned [||] |> ignore) |> ignore
        Assert.Throws<LEB128Error>(fun () -> WasmLeb128.decodeUnsigned [| 0x80uy; 0x80uy |] |> ignore) |> ignore
        Assert.Throws<LEB128Error>(fun () -> WasmLeb128.decodeSigned [| 0x80uy; 0x80uy |] |> ignore) |> ignore

    [<Fact>]
    let ``encode handles common vectors`` () =
        Assert.Equal<byte[]>([| 0x00uy |], WasmLeb128.encodeUnsigned 0u)
        Assert.Equal<byte[]>([| 0x03uy |], WasmLeb128.encodeUnsigned 3u)
        Assert.Equal<byte[]>([| 0xE5uy; 0x8Euy; 0x26uy |], WasmLeb128.encodeUnsigned 624485u)
        Assert.Equal<byte[]>([| 0x7Euy |], WasmLeb128.encodeSigned -2)

    [<Fact>]
    let ``round trips unsigned values`` () =
        let values = [| 0u; 1u; 63u; 64u; 127u; 128u; 255u; 256u; 16383u; 16384u; 624485u; 1000000u; 0x7FFFFFFFu; UInt32.MaxValue |]
        for value in values do
            let encoded = WasmLeb128.encodeUnsigned value
            let decoded, bytesConsumed = WasmLeb128.decodeUnsigned encoded
            Assert.Equal(value, decoded)
            Assert.Equal(encoded.Length, bytesConsumed)

    [<Fact>]
    let ``round trips signed values`` () =
        let values = [| 0; 1; -1; 63; -64; 64; -65; 127; -128; 128; -129; Int32.MaxValue; Int32.MinValue; -1000000; -2 |]
        for value in values do
            let encoded = WasmLeb128.encodeSigned value
            let decoded, bytesConsumed = WasmLeb128.decodeSigned encoded
            Assert.Equal(value, decoded)
            Assert.Equal(encoded.Length, bytesConsumed)
