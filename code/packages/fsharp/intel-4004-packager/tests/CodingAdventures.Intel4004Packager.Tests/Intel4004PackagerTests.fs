namespace CodingAdventures.Intel4004Packager.Tests

open System
open Xunit
open CodingAdventures.Intel4004Packager.FSharp

module Intel4004PackagerTests =
    let private verifyChecksum (line: string) =
        let bytes = Convert.FromHexString(line.Substring(1))
        Assert.Equal(0, (bytes |> Seq.sumBy int) % 256)

    [<Fact>]
    let ``version is exposed`` () =
        Assert.Equal("0.1.0", Intel4004Packager.VERSION)

    [<Fact>]
    let ``encodes records with valid checksums and eof`` () =
        let hex = Intel4004Packager.encodeHexAtZero ([| 0uy .. 16uy |])
        let lines = hex.Split('\n', StringSplitOptions.RemoveEmptyEntries)

        Assert.Equal(3, lines.Length)
        Assert.Equal("10", lines[0].Substring(1, 2))
        Assert.Equal("01", lines[1].Substring(1, 2))
        Assert.Equal(":00000001FF", lines[lines.Length - 1])
        lines |> Array.iter verifyChecksum

    [<Fact>]
    let ``applies origin and round trips`` () =
        let binary = [| 0xD5uy; 0x01uy; 0xC0uy |]
        let hex = Intel4004Packager.encodeHex binary 0x0300
        let firstLine = hex.Split('\n')[0]
        Assert.Equal("0300", firstLine.Substring(3, 4))

        let decoded = Intel4004Packager.decodeHex hex

        Assert.Equal(0x0300, decoded.Origin)
        Assert.Equal<byte>(binary, decoded.Binary)

    [<Fact>]
    let ``decodes sparse segments with zero fill`` () =
        let first = (Intel4004Packager.encodeHex [| 0xAAuy |] 0x0010).Split('\n')[0]
        let second = (Intel4004Packager.encodeHex [| 0xBBuy |] 0x0012).Split('\n')[0]

        let decoded = Intel4004Packager.decodeHex (first + "\n" + second + "\n:00000001FF\n")

        Assert.Equal(0x0010, decoded.Origin)
        Assert.Equal<byte>([| 0xAAuy; 0x00uy; 0xBBuy |], decoded.Binary)

    [<Fact>]
    let ``rejects invalid encode inputs`` () =
        Assert.Throws<ArgumentException>(fun () -> Intel4004Packager.encodeHexAtZero Array.empty |> ignore) |> ignore
        Assert.Throws<ArgumentException>(fun () -> Intel4004Packager.encodeHex [| 0uy |] -1 |> ignore) |> ignore
        Assert.Throws<ArgumentException>(fun () -> Intel4004Packager.encodeHex [| 0uy |] 0x10000 |> ignore) |> ignore
        Assert.Throws<ArgumentException>(fun () -> Intel4004Packager.encodeHex (Array.zeroCreate<byte> 100) 0xFFFF |> ignore) |> ignore

    [<Fact>]
    let ``rejects malformed hex`` () =
        Assert.Throws<FormatException>(fun () -> Intel4004Packager.decodeHex "020000000000D5012A\n" |> ignore) |> ignore
        Assert.Throws<FormatException>(fun () -> Intel4004Packager.decodeHex ":0Z000000D5\n" |> ignore) |> ignore
        Assert.Throws<FormatException>(fun () -> Intel4004Packager.decodeHex ":01000000D500\n:00000001FF\n" |> ignore) |> ignore
        Assert.Throws<FormatException>(fun () -> Intel4004Packager.decodeHex ":10000002D5AA\n" |> ignore) |> ignore
        Assert.Throws<FormatException>(fun () -> Intel4004Packager.decodeHex ":10000000D5\n" |> ignore) |> ignore

    [<Fact>]
    let ``empty decode returns empty binary`` () =
        let decoded = Intel4004Packager.decodeHex ""

        Assert.Equal(0, decoded.Origin)
        Assert.Empty(decoded.Binary)
