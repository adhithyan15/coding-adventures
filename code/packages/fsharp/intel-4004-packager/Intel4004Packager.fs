namespace CodingAdventures.Intel4004Packager.FSharp

open System
open System.Collections.Generic
open System.Text

type DecodedHex =
    { Origin: int
      Binary: byte array }

[<RequireQualifiedAccess>]
module Intel4004Packager =
    [<Literal>]
    let VERSION = "0.1.0"

    let private bytesPerRecord = 16
    let private recordTypeData = 0uy
    let private recordTypeEof = 1uy
    let private maxImageSize = 0x1000

    let private checksum (fields: byte seq) =
        let total = fields |> Seq.sumBy int
        byte ((0x100 - (total % 0x100)) % 0x100)

    let private dataRecord address (chunk: byte array) =
        let fields = Array.zeroCreate<byte> (4 + chunk.Length)
        fields[0] <- byte chunk.Length
        fields[1] <- byte ((address >>> 8) &&& 0xFF)
        fields[2] <- byte (address &&& 0xFF)
        fields[3] <- recordTypeData
        Array.Copy(chunk, 0, fields, 4, chunk.Length)
        $":{chunk.Length:X2}{address:X4}00{Convert.ToHexString(chunk)}{checksum fields:X2}\n"

    let encodeHex (binary: byte array) (origin: int) =
        if isNull binary then nullArg "binary"
        if binary.Length = 0 then invalidArg "binary" "binary must be non-empty"
        if origin < 0 || origin > 0xFFFF then invalidArg "origin" "origin must be 0-65535"
        if origin + binary.Length > 0x10000 then invalidArg "binary" "image overflows 16-bit address space"

        let builder = StringBuilder()
        let mutable offset = 0
        while offset < binary.Length do
            let count = min bytesPerRecord (binary.Length - offset)
            let chunk = binary[offset .. offset + count - 1]
            builder.Append(dataRecord (origin + offset) chunk) |> ignore
            offset <- offset + count

        builder.Append(":00000001FF\n").ToString()

    let encodeHexAtZero binary = encodeHex binary 0

    let private decodeHexBytes lineNumber (text: string) =
        try
            Convert.FromHexString(text)
        with :? FormatException as ex ->
            raise (FormatException($"line {lineNumber}: invalid hex", ex))

    let decodeHex (text: string) =
        if isNull text then nullArg "text"

        let segments = SortedDictionary<int, byte array>()
        let lines = text.Replace("\r\n", "\n").Split('\n')

        let mutable index = 0
        let mutable doneReading = false
        while index < lines.Length && not doneReading do
            let line = lines[index].Trim()
            let lineNumber = index + 1

            if line.Length > 0 then
                if not (line.StartsWith(":", StringComparison.Ordinal)) then
                    raise (FormatException($"line {lineNumber}: expected ':'"))

                let record = decodeHexBytes lineNumber line[1..]
                if record.Length < 5 then
                    raise (FormatException($"line {lineNumber}: record too short"))

                let byteCount = int record[0]
                let address = (int record[1] <<< 8) ||| int record[2]
                let recordType = record[3]
                let expectedLength = 4 + byteCount + 1

                if record.Length < expectedLength then
                    raise (FormatException($"line {lineNumber}: truncated record"))

                let computed = checksum record[0 .. 3 + byteCount]
                let stored = record[4 + byteCount]
                if computed <> stored then
                    raise (FormatException($"line {lineNumber}: checksum mismatch"))

                if recordType = recordTypeEof then
                    doneReading <- true
                elif recordType <> recordTypeData then
                    raise (FormatException($"line {lineNumber}: unsupported record type"))
                else
                    segments[address] <- record[4 .. 3 + byteCount]

            index <- index + 1

        if segments.Count = 0 then
            { Origin = 0; Binary = Array.empty }
        else
            let origin = segments.Keys |> Seq.min
            let endAddress = segments |> Seq.map (fun pair -> pair.Key + pair.Value.Length) |> Seq.max
            if endAddress - origin > maxImageSize then
                raise (FormatException("decoded image too large"))

            let binary = Array.zeroCreate<byte> (endAddress - origin)
            for pair in segments do
                Array.Copy(pair.Value, 0, binary, pair.Key - origin, pair.Value.Length)

            { Origin = origin; Binary = binary }
