namespace CodingAdventures.Zip.FSharp.PortableTests

open System
open System.Diagnostics
open System.IO
open System.Text
open System.Text.Json
open CodingAdventures.Zip.FSharp
open Xunit

module PortableConformanceTests =
    let private expectedErrorCodes =
        [| "invalid-output-limit"; "unexpected-eof"; "reserved-block-type"
           "stored-length-mismatch"; "huffman-oversubscribed"
           "incomplete-code-length-tree"; "incomplete-literal-length-tree"
           "incomplete-distance-tree"; "repeat-without-previous"; "repeat-overrun"
           "invalid-literal-length-symbol"; "reserved-distance-symbol"
           "invalid-back-reference"; "output-limit-exceeded" |]

    let private findFixture () =
        let rec search (directory: DirectoryInfo) =
            if isNull directory then
                raise (FileNotFoundException "zip-raw-rfc1951-v1 fixture not found")
            let candidate =
                Path.Combine(directory.FullName, "code", "specs", "fixtures", "zip-raw-rfc1951-v1", "cases.json")
            if File.Exists candidate then candidate else search directory.Parent
        search (DirectoryInfo AppContext.BaseDirectory)

    let private fromHex (value: string) = Convert.FromHexString value

    let private materialize (output: JsonElement) =
        let mutable hex = Unchecked.defaultof<JsonElement>
        if output.TryGetProperty("hex", &hex) then fromHex (hex.GetString())
        else
            let unit = fromHex (output.GetProperty("repeat_hex").GetString())
            Array.create (output.GetProperty("count").GetInt32()) unit[0]

    let private pythonCodec mode (input: byte[]) =
        let start = ProcessStartInfo()
        start.FileName <- if OperatingSystem.IsWindows() then "python" else "python3"
        start.RedirectStandardInput <- true
        start.RedirectStandardOutput <- true
        start.RedirectStandardError <- true
        start.UseShellExecute <- false
        start.CreateNoWindow <- true
        start.ArgumentList.Add("-c")
        start.ArgumentList.Add("import sys,zlib;m=sys.argv[1];d=sys.stdin.buffer.read();c=zlib.compressobj(9,zlib.DEFLATED,-15);r=(c.compress(d)+c.flush()) if m=='compress' else zlib.decompress(d,-15);sys.stdout.buffer.write(r)")
        start.ArgumentList.Add(mode)
        use childProcess = Process.Start(start)
        if isNull childProcess then invalidOp "python oracle did not start"
        childProcess.StandardInput.BaseStream.Write(input)
        childProcess.StandardInput.Close()
        use output = new MemoryStream()
        childProcess.StandardOutput.BaseStream.CopyTo(output)
        let diagnostic = childProcess.StandardError.ReadToEnd()
        childProcess.WaitForExit()
        if childProcess.ExitCode <> 0 then invalidOp ("python oracle failed: " + diagnostic)
        output.ToArray()

    let private rawZip (name: string) (compressed: byte[]) (plain: byte[]) (declaredSize: int) (methodId: uint16) =
        use archive = new MemoryStream()
        use writer = new BinaryWriter(archive, Encoding.UTF8, true)
        let nameBytes = Encoding.UTF8.GetBytes(name)
        let checksum = RawRfc1951.crc32 plain 0u
        writer.Write(0x04034b50u); writer.Write(20us); writer.Write(0x0800us); writer.Write(methodId)
        writer.Write(0us); writer.Write(0us); writer.Write(checksum); writer.Write(uint32 compressed.Length)
        writer.Write(uint32 declaredSize); writer.Write(uint16 nameBytes.Length); writer.Write(0us)
        writer.Write(nameBytes); writer.Write(compressed)
        let centralOffset = uint32 archive.Length
        writer.Write(0x02014b50u); writer.Write(0x031eus); writer.Write(20us); writer.Write(0x0800us); writer.Write(methodId)
        writer.Write(0us); writer.Write(0us); writer.Write(checksum); writer.Write(uint32 compressed.Length); writer.Write(uint32 declaredSize)
        writer.Write(uint16 nameBytes.Length); writer.Write(0us); writer.Write(0us); writer.Write(0us); writer.Write(0us)
        writer.Write(0u); writer.Write(0u); writer.Write(nameBytes)
        let centralSize = uint32 archive.Length - centralOffset
        writer.Write(0x06054b50u); writer.Write(0us); writer.Write(0us); writer.Write(1us); writer.Write(1us)
        writer.Write(centralSize); writer.Write(centralOffset); writer.Write(0us)
        archive.ToArray()

    [<Fact>]
    let ``closed portable corpus passes`` () =
        use fixture = JsonDocument.Parse(File.ReadAllText(findFixture()))
        let root = fixture.RootElement
        Assert.Equal(1, root.GetProperty("schema_version").GetInt32())
        Assert.Equal("zip-owned-raw-rfc1951-v1", root.GetProperty("profile").GetString())
        Assert.Equal(RawRfc1951.maxOutput, root.GetProperty("limits").GetProperty("default_max_output").GetInt32())
        Assert.Equal(RawRfc1951.maxOutput, root.GetProperty("limits").GetProperty("hard_max_output").GetInt32())
        Assert.Equal<string>(expectedErrorCodes, RawRfc1951.errorCodes)
        let fixtureErrors = root.GetProperty("error_ids").EnumerateArray() |> Seq.map (fun value -> value.GetString()) |> Seq.toArray
        Assert.Equal<string>(expectedErrorCodes, fixtureErrors)
        let cases = root.GetProperty("cases").EnumerateArray() |> Seq.toArray
        Assert.Equal(34, cases.Length)

        for testCase in cases do
            let id = testCase.GetProperty("id").GetString()
            let operation = testCase.GetProperty("operation").GetString()
            let mutable maximum = Unchecked.defaultof<JsonElement>
            let limit = if testCase.TryGetProperty("max_output", &maximum) then maximum.GetInt32() else RawRfc1951.maxOutput
            match operation with
            | "inflate" ->
                let input = fromHex (testCase.GetProperty("input_hex").GetString())
                let expected = materialize (testCase.GetProperty("expected").GetProperty("output"))
                let result = RawRfc1951.rawInflateCounted input limit
                Assert.True(result.Output = expected, id)
                Assert.Equal(testCase.GetProperty("expected").GetProperty("bytes_consumed").GetInt32(), result.BytesConsumed)
                Assert.True(RawRfc1951.rawInflate input limit = expected, id)
            | "inflate-error" ->
                let input = fromHex (testCase.GetProperty("input_hex").GetString())
                let expected = testCase.GetProperty("expected").GetProperty("error_id").GetString()
                let error = Assert.Throws<RawInflateError>(fun () -> RawRfc1951.rawInflateCounted input limit |> ignore)
                Assert.Equal(expected, error.Code)
                Assert.Equal(expected, error.Message)
                Assert.Empty(error.Data)
            | "deflate-interoperability" ->
                let input = fromHex (testCase.GetProperty("input_hex").GetString())
                let expected = materialize (testCase.GetProperty("expected").GetProperty("output"))
                Assert.True(pythonCodec "decompress" (RawRfc1951.rawDeflate input) = expected, id)
            | "crc32" ->
                let mutable initial = Unchecked.defaultof<JsonElement>
                let mutable checksum = if testCase.TryGetProperty("initial_crc32_hex", &initial) then Convert.ToUInt32(initial.GetString(), 16) else 0u
                for chunk in testCase.GetProperty("chunks_hex").EnumerateArray() do
                    checksum <- RawRfc1951.crc32 (fromHex (chunk.GetString())) checksum
                Assert.Equal(testCase.GetProperty("expected").GetProperty("crc32_hex").GetString(), checksum.ToString("x8"))
            | _ -> Assert.Fail(id + ": unsupported fixture operation " + operation)

    [<Fact>]
    let ``foreign full-window stream passes`` () =
        let prefix = Array.init 32768 (fun index -> byte ((index * 73 + index / 251) &&& 0xff))
        let expected = Array.append prefix prefix
        let foreign = pythonCodec "compress" expected
        Assert.Equal<byte>(expected, RawRfc1951.rawInflate foreign expected.Length)

    [<Fact>]
    let ``ZIP reader enforces exact compressed and uncompressed sizes`` () =
        let compressed = fromHex "0dc28911c0200c03b0d8f97028ec3f6ed129cab7dd96a0c2445bdb93809663a5d303f6b265e20c2b79ea03379d227e"
        let plain = fromHex "0406030b000e070909010906010a04070007000000000501010908030108050302030401000401000207090009020a0a020605020d060c01020b020302090201"
        Assert.Equal<byte>(plain, ZipReader(rawZip "dynamic.bin" compressed plain plain.Length 8us).Read("dynamic.bin"))
        let cavity = Array.concat [ compressed; [| 0xdeuy; 0xaduy |] ]
        let suffixError = Assert.Throws<InvalidDataException>(fun () -> ZipReader(rawZip "cavity.bin" cavity plain plain.Length 8us).Read("cavity.bin") |> ignore)
        Assert.Equal("zip: compressed payload contains trailing bytes", suffixError.Message)
        let sizeError = Assert.Throws<InvalidDataException>(fun () -> ZipReader(rawZip "size.bin" compressed plain (plain.Length + 1) 8us).Read("size.bin") |> ignore)
        Assert.Equal("zip: uncompressed size does not match the directory", sizeError.Message)
        let storedError = Assert.Throws<InvalidDataException>(fun () -> ZipReader(rawZip "stored.bin" plain plain (plain.Length + 1) 0us).Read("stored.bin") |> ignore)
        Assert.Equal("zip: stored entry sizes do not match", storedError.Message)
        let malformedError = Assert.Throws<InvalidDataException>(fun () -> ZipReader(rawZip "malformed.bin" [| 0x07uy |] [||] 0 8us).Read("malformed.bin") |> ignore)
        Assert.Equal("zip: raw inflate failed: reserved-block-type", malformedError.Message)

    [<Fact>]
    let ``one-shot extraction enforces an aggregate output budget`` () =
        let payload = Array.create 700 0x41uy
        let archive =
            ZipArchive.zip
                [ { Name = "first.bin"; Data = payload }
                  { Name = "second.bin"; Data = payload } ]
        Assert.Throws<InvalidDataException>(fun () -> ZipArchive.unzipWithLimit archive 1024L |> ignore)
        |> ignore
        let extracted = ZipArchive.unzipWithLimit archive 2048L
        Assert.Equal(2, extracted.Length)
