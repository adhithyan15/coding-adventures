namespace CodingAdventures.WriteAheadLog

open System
open System.Buffers.Binary
open System.IO

module WriteAheadLogPackage =
    [<Literal>]
    let Version = "0.1.0"

type WalWriter(path: string) =
    do
        if String.IsNullOrWhiteSpace(path) then
            invalidArg (nameof path) "A WAL path is required."

    let file =
        new FileStream(path, FileMode.Append, FileAccess.Write, FileShare.Read, 4096, FileOptions.WriteThrough)

    member _.AppendRecord(data: byte array) =
        if isNull data then
            nullArg (nameof data)

        let lengthBytes = Array.zeroCreate<byte> 4
        BinaryPrimitives.WriteUInt32LittleEndian(Span<byte>(lengthBytes), uint32 data.Length)
        file.Write(lengthBytes, 0, lengthBytes.Length)
        file.Write(data, 0, data.Length)
        file.Flush(true)

    interface IDisposable with
        member _.Dispose() =
            file.Dispose()

type WalReader(path: string) =
    do
        if String.IsNullOrWhiteSpace(path) then
            invalidArg (nameof path) "A WAL path is required."

    let file = new FileStream(path, FileMode.Open, FileAccess.Read, FileShare.ReadWrite)

    let rec readInto (buffer: byte array) offset count =
        if count = 0 then
            offset
        else
            let read = file.Read(buffer, offset, count)

            if read = 0 then
                offset
            else
                readInto buffer (offset + read) (count - read)

    member _.ReadNext() =
        let lengthBytes = Array.zeroCreate<byte> 4
        let lengthRead = readInto lengthBytes 0 lengthBytes.Length

        if lengthRead < lengthBytes.Length then
            None
        else
            let length = BinaryPrimitives.ReadUInt32LittleEndian(ReadOnlySpan<byte>(lengthBytes))

            if length > uint32 Int32.MaxValue then
                raise (InvalidDataException("WAL record is too large for this runtime."))

            let record = Array.zeroCreate<byte> (int length)
            let payloadRead = readInto record 0 record.Length

            if payloadRead < record.Length then
                raise (EndOfStreamException("WAL record ended before its length-prefixed payload was complete."))

            Some record

    interface IDisposable with
        member _.Dispose() =
            file.Dispose()
