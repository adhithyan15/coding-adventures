// ZipTests.fs — xUnit test suite for CodingAdventures.Zip.FSharp (CMP09)
//
// 12 test cases covering:
//   TC-1:  Round-trip single file, Stored (compress=false)
//   TC-2:  Round-trip single file, DEFLATE (repetitive text gets smaller)
//   TC-3:  Multiple files in one archive
//   TC-4:  Directory entry (name ends with /)
//   TC-5:  CRC-32 mismatch detected (corrupt byte → exception)
//   TC-6:  Random-access read (10 files, read only f5.txt)
//   TC-7:  Incompressible data stored as Stored (method=0)
//   TC-8:  Empty file
//   TC-9:  Large file compressed (100 KB repetitive data)
//   TC-10: Unicode filename (日本語/résumé.txt)
//   TC-11: Nested paths
//   TC-12: Empty archive

module CodingAdventures.Zip.FSharp.Tests

open System
open System.IO
open System.Text
open Xunit
open CodingAdventures.Zip.FSharp

// ── TC-1: Round-trip single file, Stored ─────────────────────────────────
//
// When compress=false the writer must use method=0 (Stored) and the reader
// must return the original bytes verbatim.

[<Fact>]
let ``TC1 RoundTrip SingleFile Stored`` () =
    let data = Encoding.UTF8.GetBytes("hello, world")

    let writer = ZipWriter()
    writer.AddFile("hello.txt", data, compress = false)
    let archive = writer.Finish()

    let entries = ZipArchive.unzip archive
    Assert.Single(entries) |> ignore
    Assert.Equal("hello.txt", entries[0].Name)
    Assert.Equal<byte[]>(data, entries[0].Data)

    // Verify we can also read back via ZipReader directly.
    let reader = ZipReader(archive)
    Assert.Equal<byte[]>(data, reader.Read("hello.txt"))

// ── TC-2: Round-trip single file, DEFLATE ────────────────────────────────
//
// Highly repetitive text should compress to fewer bytes than the original.

[<Fact>]
let ``TC2 RoundTrip SingleFile Deflate`` () =
    let phrase = "the quick brown fox jumps over the lazy dog "
    let text   = Encoding.UTF8.GetBytes(String.concat "" (List.replicate 10 phrase))

    let archive = ZipArchive.zip (seq { { Name = "text.txt"; Data = text } })

    let entries = ZipArchive.unzip archive
    Assert.Single(entries) |> ignore
    Assert.Equal("text.txt", entries[0].Name)
    Assert.Equal<byte[]>(text, entries[0].Data)

    // The archive must be smaller than the raw text (compression worked).
    Assert.True(
        archive.Length < text.Length,
        sprintf "archive (%d bytes) must be smaller than text (%d bytes)" archive.Length text.Length)

// ── TC-3: Multiple files in one archive ──────────────────────────────────

[<Fact>]
let ``TC3 MultipleFiles`` () =
    let allBytes = [| 0uy .. 255uy |]
    let input = [
        { Name = "a.txt";  Data = Encoding.UTF8.GetBytes("file A content") }
        { Name = "b.txt";  Data = Encoding.UTF8.GetBytes("file B content") }
        { Name = "c.bin";  Data = allBytes }
    ]

    let archive = ZipArchive.zip (Seq.ofList input)
    let output  = ZipArchive.unzip archive

    Assert.Equal(3, output.Length)
    for orig in input do
        let found = output |> List.find (fun e -> e.Name = orig.Name)
        Assert.Equal<byte[]>(orig.Data, found.Data)

// ── TC-4: Directory entry ─────────────────────────────────────────────────
//
// Directory entries have name ending with '/', method=0, sizes=0, CRC=0.
// They must appear in the Central Directory but return empty data on Read.

[<Fact>]
let ``TC4 DirectoryEntry`` () =
    let writer = ZipWriter()
    writer.AddDirectory("mydir/")
    writer.AddFile("mydir/file.txt", Encoding.UTF8.GetBytes("contents"))
    let archive = writer.Finish()

    let reader = ZipReader(archive)
    let names  = reader.Entries |> List.map _.Name

    Assert.Contains("mydir/", names)
    Assert.Contains("mydir/file.txt", names)

    // Reading a directory entry returns empty bytes.
    Assert.Empty(reader.Read("mydir/"))

// ── TC-5: CRC-32 mismatch detected ───────────────────────────────────────
//
// Corrupt one byte of the compressed payload in the archive. When the reader
// decompresses and CRC-checks it must throw InvalidDataException mentioning "CRC".

[<Fact>]
let ``TC5 CrcMismatchDetected`` () =
    // Build an archive with a known file.
    let original =
        ZipArchive.zip (seq { { Name = "f.txt"; Data = Encoding.UTF8.GetBytes("test data") } })

    // Find where the file data starts: 30-byte fixed LFH + 5-byte name "f.txt" = offset 35.
    // For a stored file the data follows immediately at byte 35.
    let corrupted = Array.copy original
    // Local header fixed part: 30 bytes. Name "f.txt" = 5 bytes → data at byte 35.
    corrupted[35] <- corrupted[35] ^^^ 0xFFuy

    let reader = ZipReader(corrupted)
    let ex = Assert.Throws<InvalidDataException>(fun () -> reader.Read("f.txt") |> ignore)
    Assert.Contains("CRC", ex.Message, StringComparison.OrdinalIgnoreCase)

// ── TC-6: Random-access read (10 files, read only f5.txt) ─────────────────
//
// ZipReader must be able to read a single entry without reading the others.

[<Fact>]
let ``TC6 RandomAccessRead`` () =
    let entries =
        [ 0 .. 9 ]
        |> List.map (fun i ->
            { Name = sprintf "f%d.txt" i
              Data = Encoding.UTF8.GetBytes(sprintf "content %d" i) })

    let archive = ZipArchive.zip (Seq.ofList entries)
    let reader  = ZipReader(archive)

    let data5 = reader.Read("f5.txt")
    Assert.Equal<byte[]>(Encoding.UTF8.GetBytes("content 5"), data5)

// ── TC-7: Incompressible data stored as Stored (method=0) ─────────────────
//
// When DEFLATE produces output >= original length the writer must fall back
// to method=0. The Central Directory must record method=0 for such entries.

// Helper: scan archive bytes for a Central Directory entry for `entryName`
// and return its compression method.
let private findCdMethod (archive: byte[]) (entryName: string) : int =
    let cdMagic = [| 0x50uy; 0x4Buy; 0x01uy; 0x02uy |]
    let name = Encoding.UTF8.GetBytes(entryName)

    let mutable result = None
    let mutable i = 0
    while i <= archive.Length - 46 && result.IsNone do
        if archive[i] = cdMagic[0] && archive[i+1] = cdMagic[1] &&
           archive[i+2] = cdMagic[2] && archive[i+3] = cdMagic[3] then
            let nameLen = int archive[i + 28] ||| (int archive[i + 29] <<< 8)
            if nameLen = name.Length then
                let nameStart = i + 46
                if nameStart + nameLen <= archive.Length then
                    let nameSlice = archive[nameStart .. nameStart + nameLen - 1]
                    if nameSlice = name then
                        result <- Some (int archive[i + 10] ||| (int archive[i + 11] <<< 8))
        i <- i + 1

    match result with
    | Some m -> m
    | None   -> failwithf "CD entry for '%s' not found" entryName

[<Fact>]
let ``TC7 IncompressibleData StoredMethod`` () =
    // Build pseudo-random data via a simple LCG. This should be incompressible.
    let mutable seed = 42u
    let randomData = Array.init 1024 (fun _ ->
        seed <- seed * 1_664_525u + 1_013_904_223u
        byte (seed >>> 24))

    let archive = ZipArchive.zip (seq { { Name = "random.bin"; Data = randomData } })
    let reader  = ZipReader(archive)

    // Round-trip check.
    let result = reader.Read("random.bin")
    Assert.Equal<byte[]>(randomData, result)

    // Verify method=0 by inspecting the archive bytes.
    let methodInCd = findCdMethod archive "random.bin"
    Assert.Equal(0, methodInCd)  // Stored

// ── TC-8: Empty file ──────────────────────────────────────────────────────

[<Fact>]
let ``TC8 EmptyFile`` () =
    let archive = ZipArchive.zip (seq { { Name = "empty.txt"; Data = [||] } })
    let entries = ZipArchive.unzip archive

    Assert.Single(entries) |> ignore
    Assert.Equal("empty.txt", entries[0].Name)
    Assert.Empty(entries[0].Data)

// ── TC-9: Large file compressed (100 KB repetitive data) ─────────────────
//
// 100 KB of "abcdefghij" repeated must compress to a significantly smaller archive.

[<Fact>]
let ``TC9 LargeFile Compressed`` () =
    // 10 bytes × 10000 repetitions = 100 KB. DEFLATE should compress this well.
    let chunk = Encoding.UTF8.GetBytes("abcdefghij")
    let data  = Array.concat (List.replicate 10_000 chunk)

    let archive = ZipArchive.zip (seq { { Name = "big.bin"; Data = data } })
    let entries = ZipArchive.unzip archive

    Assert.Equal<byte[]>(data, entries[0].Data)
    Assert.True(
        archive.Length < data.Length,
        sprintf "100 KB repetitive data must compress: archive=%d data=%d" archive.Length data.Length)

// ── TC-10: Unicode filename ───────────────────────────────────────────────
//
// ZIP bit 11 = UTF-8. Both the writer and reader must preserve multi-byte filenames.

[<Fact>]
let ``TC10 UnicodeFilename`` () =
    let name    = "日本語/résumé.txt"
    let content = Encoding.UTF8.GetBytes("content")

    let archive = ZipArchive.zip (seq { { Name = name; Data = content } })
    let entries = ZipArchive.unzip archive

    Assert.Single(entries) |> ignore
    Assert.Equal(name, entries[0].Name)
    Assert.Equal<byte[]>(content, entries[0].Data)

// ── TC-11: Nested paths ───────────────────────────────────────────────────

[<Fact>]
let ``TC11 NestedPaths`` () =
    let input = [
        { Name = "root.txt";         Data = Encoding.UTF8.GetBytes("root")   }
        { Name = "dir/file.txt";     Data = Encoding.UTF8.GetBytes("nested") }
        { Name = "dir/sub/deep.txt"; Data = Encoding.UTF8.GetBytes("deep")   }
    ]

    let archive = ZipArchive.zip (Seq.ofList input)
    let output  = ZipArchive.unzip archive

    for orig in input do
        let found = output |> List.find (fun e -> e.Name = orig.Name)
        Assert.Equal<byte[]>(orig.Data, found.Data)

// ── TC-12: Empty archive ──────────────────────────────────────────────────
//
// A writer with no entries must produce a valid (but empty) archive.

[<Fact>]
let ``TC12 EmptyArchive`` () =
    let writer  = ZipWriter()
    let archive = writer.Finish()

    let reader  = ZipReader(archive)
    Assert.Empty(reader.Entries)

    let entries = ZipArchive.unzip archive
    Assert.Empty(entries)
