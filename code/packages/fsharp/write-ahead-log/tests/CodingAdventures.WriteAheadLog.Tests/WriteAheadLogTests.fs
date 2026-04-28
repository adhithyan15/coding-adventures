namespace CodingAdventures.WriteAheadLog.Tests

open System
open System.IO
open CodingAdventures.WriteAheadLog
open Xunit

type WriteAheadLogTests() =
    let tempPath () =
        Path.Combine(Path.GetTempPath(), $"{Guid.NewGuid():N}.wal")

    let deleteIfExists path =
        if File.Exists(path) then
            File.Delete(path)

    [<Fact>]
    member _.VersionExists() =
        Assert.Equal("0.1.0", WriteAheadLogPackage.Version)

    [<Fact>]
    member _.AppendRecordWritesLengthPrefixedPayloads() =
        let path = tempPath ()

        try
            use writer = new WalWriter(path)
            writer.AppendRecord([| 1uy; 2uy; 3uy |])
            writer.AppendRecord([| 4uy; 5uy |])
            (writer :> IDisposable).Dispose()

            Assert.Equal<byte array>(
                [| 3uy; 0uy; 0uy; 0uy; 1uy; 2uy; 3uy; 2uy; 0uy; 0uy; 0uy; 4uy; 5uy |],
                File.ReadAllBytes(path)
            )
        finally
            deleteIfExists path

    [<Fact>]
    member _.ReaderReturnsRecordsInOrderThenNoneAtEnd() =
        let path = tempPath ()

        try
            use writer = new WalWriter(path)
            writer.AppendRecord([| 10uy; 20uy |])
            writer.AppendRecord([| 30uy |])
            (writer :> IDisposable).Dispose()

            use reader = new WalReader(path)

            Assert.Equal<byte array>([| 10uy; 20uy |], reader.ReadNext().Value)
            Assert.Equal<byte array>([| 30uy |], reader.ReadNext().Value)
            Assert.True(reader.ReadNext().IsNone)
        finally
            deleteIfExists path

    [<Fact>]
    member _.EmptyRecordsRoundTrip() =
        let path = tempPath ()

        try
            use writer = new WalWriter(path)
            writer.AppendRecord([||])
            (writer :> IDisposable).Dispose()

            use reader = new WalReader(path)

            Assert.Equal<byte array>([||], reader.ReadNext().Value)
            Assert.True(reader.ReadNext().IsNone)
        finally
            deleteIfExists path

    [<Fact>]
    member _.WriterAppendsToExistingLog() =
        let path = tempPath ()

        try
            use writer = new WalWriter(path)
            writer.AppendRecord([| 1uy |])
            (writer :> IDisposable).Dispose()

            use writer2 = new WalWriter(path)
            writer2.AppendRecord([| 2uy; 3uy |])
            (writer2 :> IDisposable).Dispose()

            use reader = new WalReader(path)

            Assert.Equal<byte array>([| 1uy |], reader.ReadNext().Value)
            Assert.Equal<byte array>([| 2uy; 3uy |], reader.ReadNext().Value)
            Assert.True(reader.ReadNext().IsNone)
        finally
            deleteIfExists path

    [<Fact>]
    member _.PartialLengthPrefixIsTreatedAsEndOfFile() =
        let path = tempPath ()

        try
            File.WriteAllBytes(path, [| 2uy; 0uy |])

            use reader = new WalReader(path)

            Assert.True(reader.ReadNext().IsNone)
        finally
            deleteIfExists path

    [<Fact>]
    member _.TruncatedPayloadThrows() =
        let path = tempPath ()

        try
            File.WriteAllBytes(path, [| 4uy; 0uy; 0uy; 0uy; 1uy; 2uy |])

            use reader = new WalReader(path)

            Assert.Throws<EndOfStreamException>(fun () -> reader.ReadNext() |> ignore) |> ignore
        finally
            deleteIfExists path

    [<Fact>]
    member _.InvalidArgumentsAreRejected() =
        Assert.Throws<ArgumentException>(fun () -> new WalWriter("") |> ignore) |> ignore
        Assert.Throws<ArgumentException>(fun () -> new WalReader("") |> ignore) |> ignore
        Assert.Throws<FileNotFoundException>(fun () -> new WalReader(Path.Combine(Path.GetTempPath(), Guid.NewGuid().ToString("N"))) |> ignore) |> ignore

    [<Fact>]
    member _.WriterRejectsNullPayload() =
        let path = tempPath ()

        try
            use writer = new WalWriter(path)

            Assert.Throws<ArgumentNullException>(fun () -> writer.AppendRecord(Unchecked.defaultof<byte array>)) |> ignore
        finally
            deleteIfExists path
