namespace CodingAdventures.Sha512.Tests

open System
open System.Globalization
open System.IO
open System.Text
open System.Text.RegularExpressions
open Xunit
open CodingAdventures.Sha512.FSharp

module Sha512Tests =
    let private matches (pattern: string) (value: string) = Regex.IsMatch(value, pattern, RegexOptions.CultureInvariant)

    let private validateRow (line: string) =
        let fields = line.Split('|')
        if fields.Length <> 5 || not (matches "^[a-z0-9]+(?:-[a-z0-9]+)*$" fields[1]) then invalidArg "line" "malformed fixture row"
        match fields[0] with
        | "V" ->
            if fields[2].Length > 8192 || not (matches "^(?:[0-9a-f]{2})*$" fields[2]) || not (matches "^[0-9a-f]{128}$" fields[4]) then
                invalidArg "line" "malformed vector row"
            let byteCount = fields[2].Length / 2
            if byteCount > 4096 || ((byteCount = 0) <> (fields[3].Length = 0)) then invalidArg "line" "malformed vector size"
            let chunks =
                if fields[3].Length = 0 then [||]
                elif matches "^[1-9][0-9]*(?:,[1-9][0-9]*)*$" fields[3] then
                    fields[3].Split(',') |> Array.map (fun raw -> match Int32.TryParse raw with | true, value when value <= 4096 -> value | _ -> invalidArg "line" "malformed chunk")
                else invalidArg "line" "malformed chunks"
            if chunks.Length > 64 || Array.sum chunks <> byteCount then invalidArg "line" "malformed chunks"
        | "L" ->
            if not (matches "^[0-9a-f]{16}$" fields[2]) || not (matches "^[0-9a-f]{16}$" fields[3]) ||
               (fields[4] <> "ERROR" && not (matches "^[0-9a-f]{32}$" fields[4])) then invalidArg "line" "malformed length row"
        | _ -> invalidArg "line" "unknown fixture row tag"

    let private parseFixture (raw: byte array) =
        if raw.Length < 1 || raw.Length > 4096 then invalidArg "raw" "fixture byte size outside bounds"
        let lines = UTF8Encoding(false, true).GetString(raw).Split('\n') |> Array.map (fun line -> line.TrimEnd('\r')) |> Array.filter (fun line -> line.Length > 0 && line[0] <> '#')
        if lines.Length < 1 || lines.Length > 64 then invalidArg "raw" "fixture row count outside bounds"
        lines |> Array.iter validateRow
        lines

    let fixtureLines () =
        let path = "../../../../../../../../specs/fixtures/sha512-v1/cases.txt"
        let lines = File.ReadAllBytes(path) |> parseFixture
        Assert.Equal(9, lines |> Array.filter (fun line -> line.StartsWith("V|", StringComparison.Ordinal)) |> Array.length)
        Assert.Equal(6, lines |> Array.filter (fun line -> line.StartsWith("L|", StringComparison.Ordinal)) |> Array.length)
        lines

    [<Fact>]
    let ``fixture reader rejects malformed rows`` () =
        let digest = String('0', 128)
        for row in [ "X|id|||"; $"V|UPPER|00|1|{digest}"; $"V|bad|0|1|{digest}"; $"V|bad|00|2|{digest}";
                     "L|bad|000000000000000G|0000000000000000|ERROR"; "L|bad|0000000000000000|0000000000000000|ABC" ] do
            Assert.Throws<ArgumentException>(fun () -> Encoding.ASCII.GetBytes(row) |> parseFixture |> ignore) |> ignore

    [<Fact>]
    let ``portable corpus matches one shot and streaming`` () =
        for line in fixtureLines () |> Array.filter (fun line -> line.StartsWith("V|", StringComparison.Ordinal)) do
            let fields = line.Split('|')
            let data = Convert.FromHexString(fields[2])
            Assert.Equal(fields[4], Sha512.hashHex data)
            let hasher = Sha512Hasher()
            let mutable offset = 0
            let chunks = if fields[3].Length = 0 then [||] else fields[3].Split(',') |> Array.map int
            Assert.InRange(chunks.Length, 0, 64)
            for size in chunks do
                Assert.True(size > 0 && offset + size <= data.Length)
                hasher.Update(data[offset .. offset + size - 1]) |> ignore
                offset <- offset + size
            Assert.Equal(data.Length, offset)
            Assert.Equal(fields[4], hasher.HexDigest())

    [<Fact>]
    let ``portable length corpus exercises full 128 bit field`` () =
        for line in fixtureLines () |> Array.filter (fun line -> line.StartsWith("L|", StringComparison.Ordinal)) do
            let fields = line.Split('|')
            let high = UInt64.Parse(fields[2], NumberStyles.HexNumber, CultureInfo.InvariantCulture)
            let low = UInt64.Parse(fields[3], NumberStyles.HexNumber, CultureInfo.InvariantCulture)
            if fields[4] = "ERROR" then
                Assert.Throws<ArgumentException>(fun () -> Sha512.encodeBitLength high low |> ignore) |> ignore
            else
                Assert.Equal(fields[4], Convert.ToHexString(Sha512.encodeBitLength high low).ToLowerInvariant())

    [<Theory>]
    [<InlineData("", "cf83e1357eefb8bdf1542850d66d8007d620e4050b5715dc83f4a921d36ce9ce47d0d13c5d85f2b0ff8318d2877eec2f63b931bd47417a81a538327af927da3e")>]
    [<InlineData("abc", "ddaf35a193617abacc417349ae20413112e6fa4e89a97ea20a9eeee64b55d39a2192992a274fc1a836ba3c23a3feebbd454d4423643ce80e2a9ac94fa54ca49f")>]
    [<InlineData("abcdefghbcdefghicdefghijdefghijkefghijklfghijklmghijklmnhijklmnoijklmnopjklmnopqklmnopqrlmnopqrsmnopqrstnopqrstu", "8e959b75dae313da8cf4f72814fc143f8f7779c6eb9f7fa17299aeadb6889018501d289e4900f7e4331b99dec4b5433ac7d329eeb6dd26545e96e55b874be909")>]
    let ``hash matches FIPS vectors`` (input: string) (expected: string) =
        let data = Encoding.ASCII.GetBytes input

        Assert.Equal(Sha512.DigestLength, (Sha512.hash data).Length)
        Assert.Equal(expected, Sha512.hashHex data)

    [<Fact>]
    let ``hash handles large input`` () =
        let data = Array.create 1_000_000 (byte 'a')

        Assert.Equal(
            "e718483d0ce769644e2e42c7bc15b4638e1f98b13b2044285632a803afa973ebde0ff244877ea60a4cb0432ce577c31beb009c5c2c49aa2e4eadb217ad8cc09b",
            Sha512.hashHex data)

    [<Fact>]
    let ``hex digest is lowercase`` () =
        let hex = Sha512.hashHex (Encoding.ASCII.GetBytes "abc")

        Assert.Equal(128, hex.Length)
        Assert.Equal(hex.ToLowerInvariant(), hex)

    [<Fact>]
    let ``hash rejects null`` () =
        Assert.Throws<ArgumentNullException>(fun () -> Sha512.hash null |> ignore) |> ignore
        Assert.Throws<ArgumentNullException>(fun () -> Sha512.hashHex null |> ignore) |> ignore

    [<Fact>]
    let ``streaming matches one shot across chunking`` () =
        let data = [| 0 .. 255 |] |> Array.map byte

        for chunkSize in [ 1; 7; 13; 32; 63; 64; 65; 100; 128; 256 ] do
            let hasher = Sha512Hasher()
            for offset in 0 .. chunkSize .. data.Length - 1 do
                data.[offset .. min (offset + chunkSize - 1) (data.Length - 1)]
                |> hasher.Update
                |> ignore

            Assert.Equal<byte array>(Sha512.hash data, hasher.Digest())

    [<Fact>]
    let ``streaming digest is nondestructive and chainable`` () =
        let hasher = Sha512Hasher()

        let result =
            hasher
                .Update("a"B)
                .Update("b"B)
                .Update("c"B)

        Assert.Same(hasher, result)
        Assert.Equal<byte array>(hasher.Digest(), hasher.Digest())
        Assert.Equal(Sha512.hashHex "abc"B, hasher.HexDigest())

    [<Fact>]
    let ``streaming can continue after digest`` () =
        let hasher = Sha512Hasher()

        hasher.Update("ab"B) |> ignore
        hasher.Digest() |> ignore
        hasher.Update("c"B) |> ignore

        Assert.Equal<byte array>(Sha512.hash "abc"B, hasher.Digest())

    [<Fact>]
    let ``copy is independent`` () =
        let baseHasher = Sha512Hasher()
        baseHasher.Update("ab"B) |> ignore

        let copy = baseHasher.Copy()
        copy.Update("c"B) |> ignore
        baseHasher.Update("x"B) |> ignore

        Assert.Equal<byte array>(Sha512.hash "abc"B, copy.Digest())
        Assert.Equal<byte array>(Sha512.hash "abx"B, baseHasher.Digest())

    [<Fact>]
    let ``update rejects null`` () =
        Assert.Throws<ArgumentNullException>(fun () -> Sha512Hasher().Update(null) |> ignore) |> ignore

    [<Fact>]
    let ``update consumes input without aliasing`` () =
        let input = "abc"B
        let hasher = Sha512Hasher().Update(input)
        input[0] <- byte 'z'
        Assert.Equal<byte array>(Sha512.hash "abc"B, hasher.Digest())
