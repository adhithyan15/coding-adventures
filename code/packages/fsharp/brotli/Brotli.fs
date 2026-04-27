namespace CodingAdventures.Brotli.FSharp

open System
open System.Buffers.Binary
open System.Collections.Generic
open System.Text
open CodingAdventures.HuffmanTree.FSharp

type private IccEntry =
    {
        InsertBase: int
        InsertExtra: int
        CopyBase: int
        CopyExtra: int
    }

type private DistEntry =
    {
        Code: int
        Base: int
        ExtraBits: int
    }

type private Command =
    {
        InsertLength: int
        CopyLength: int
        CopyDistance: int
        Literals: byte array
    }

module private Tables =
    let IccTable =
        [|
            { InsertBase = 0; InsertExtra = 0; CopyBase = 4; CopyExtra = 0 }
            { InsertBase = 0; InsertExtra = 0; CopyBase = 5; CopyExtra = 0 }
            { InsertBase = 0; InsertExtra = 0; CopyBase = 6; CopyExtra = 0 }
            { InsertBase = 0; InsertExtra = 0; CopyBase = 8; CopyExtra = 1 }
            { InsertBase = 0; InsertExtra = 0; CopyBase = 10; CopyExtra = 1 }
            { InsertBase = 0; InsertExtra = 0; CopyBase = 14; CopyExtra = 2 }
            { InsertBase = 0; InsertExtra = 0; CopyBase = 18; CopyExtra = 2 }
            { InsertBase = 0; InsertExtra = 0; CopyBase = 26; CopyExtra = 3 }
            { InsertBase = 0; InsertExtra = 0; CopyBase = 34; CopyExtra = 3 }
            { InsertBase = 0; InsertExtra = 0; CopyBase = 50; CopyExtra = 4 }
            { InsertBase = 0; InsertExtra = 0; CopyBase = 66; CopyExtra = 4 }
            { InsertBase = 0; InsertExtra = 0; CopyBase = 98; CopyExtra = 5 }
            { InsertBase = 0; InsertExtra = 0; CopyBase = 130; CopyExtra = 5 }
            { InsertBase = 0; InsertExtra = 0; CopyBase = 194; CopyExtra = 6 }
            { InsertBase = 0; InsertExtra = 0; CopyBase = 258; CopyExtra = 7 }
            { InsertBase = 0; InsertExtra = 0; CopyBase = 514; CopyExtra = 8 }
            { InsertBase = 1; InsertExtra = 0; CopyBase = 4; CopyExtra = 0 }
            { InsertBase = 1; InsertExtra = 0; CopyBase = 5; CopyExtra = 0 }
            { InsertBase = 1; InsertExtra = 0; CopyBase = 6; CopyExtra = 0 }
            { InsertBase = 1; InsertExtra = 0; CopyBase = 8; CopyExtra = 1 }
            { InsertBase = 1; InsertExtra = 0; CopyBase = 10; CopyExtra = 1 }
            { InsertBase = 1; InsertExtra = 0; CopyBase = 14; CopyExtra = 2 }
            { InsertBase = 1; InsertExtra = 0; CopyBase = 18; CopyExtra = 2 }
            { InsertBase = 1; InsertExtra = 0; CopyBase = 26; CopyExtra = 3 }
            { InsertBase = 2; InsertExtra = 0; CopyBase = 4; CopyExtra = 0 }
            { InsertBase = 2; InsertExtra = 0; CopyBase = 5; CopyExtra = 0 }
            { InsertBase = 2; InsertExtra = 0; CopyBase = 6; CopyExtra = 0 }
            { InsertBase = 2; InsertExtra = 0; CopyBase = 8; CopyExtra = 1 }
            { InsertBase = 2; InsertExtra = 0; CopyBase = 10; CopyExtra = 1 }
            { InsertBase = 2; InsertExtra = 0; CopyBase = 14; CopyExtra = 2 }
            { InsertBase = 2; InsertExtra = 0; CopyBase = 18; CopyExtra = 2 }
            { InsertBase = 2; InsertExtra = 0; CopyBase = 26; CopyExtra = 3 }
            { InsertBase = 3; InsertExtra = 1; CopyBase = 4; CopyExtra = 0 }
            { InsertBase = 3; InsertExtra = 1; CopyBase = 5; CopyExtra = 0 }
            { InsertBase = 3; InsertExtra = 1; CopyBase = 6; CopyExtra = 0 }
            { InsertBase = 3; InsertExtra = 1; CopyBase = 8; CopyExtra = 1 }
            { InsertBase = 3; InsertExtra = 1; CopyBase = 10; CopyExtra = 1 }
            { InsertBase = 3; InsertExtra = 1; CopyBase = 14; CopyExtra = 2 }
            { InsertBase = 3; InsertExtra = 1; CopyBase = 18; CopyExtra = 2 }
            { InsertBase = 3; InsertExtra = 1; CopyBase = 26; CopyExtra = 3 }
            { InsertBase = 5; InsertExtra = 2; CopyBase = 4; CopyExtra = 0 }
            { InsertBase = 5; InsertExtra = 2; CopyBase = 5; CopyExtra = 0 }
            { InsertBase = 5; InsertExtra = 2; CopyBase = 6; CopyExtra = 0 }
            { InsertBase = 5; InsertExtra = 2; CopyBase = 8; CopyExtra = 1 }
            { InsertBase = 5; InsertExtra = 2; CopyBase = 10; CopyExtra = 1 }
            { InsertBase = 5; InsertExtra = 2; CopyBase = 14; CopyExtra = 2 }
            { InsertBase = 5; InsertExtra = 2; CopyBase = 18; CopyExtra = 2 }
            { InsertBase = 5; InsertExtra = 2; CopyBase = 26; CopyExtra = 3 }
            { InsertBase = 9; InsertExtra = 3; CopyBase = 4; CopyExtra = 0 }
            { InsertBase = 9; InsertExtra = 3; CopyBase = 5; CopyExtra = 0 }
            { InsertBase = 9; InsertExtra = 3; CopyBase = 6; CopyExtra = 0 }
            { InsertBase = 9; InsertExtra = 3; CopyBase = 8; CopyExtra = 1 }
            { InsertBase = 9; InsertExtra = 3; CopyBase = 10; CopyExtra = 1 }
            { InsertBase = 9; InsertExtra = 3; CopyBase = 14; CopyExtra = 2 }
            { InsertBase = 9; InsertExtra = 3; CopyBase = 18; CopyExtra = 2 }
            { InsertBase = 9; InsertExtra = 3; CopyBase = 26; CopyExtra = 3 }
            { InsertBase = 17; InsertExtra = 4; CopyBase = 4; CopyExtra = 0 }
            { InsertBase = 17; InsertExtra = 4; CopyBase = 5; CopyExtra = 0 }
            { InsertBase = 17; InsertExtra = 4; CopyBase = 6; CopyExtra = 0 }
            { InsertBase = 17; InsertExtra = 4; CopyBase = 8; CopyExtra = 1 }
            { InsertBase = 17; InsertExtra = 4; CopyBase = 10; CopyExtra = 1 }
            { InsertBase = 17; InsertExtra = 4; CopyBase = 14; CopyExtra = 2 }
            { InsertBase = 17; InsertExtra = 4; CopyBase = 18; CopyExtra = 2 }
            { InsertBase = 0; InsertExtra = 0; CopyBase = 0; CopyExtra = 0 }
        |]

    let DistTable =
        [|
            { Code = 0; Base = 1; ExtraBits = 0 }
            { Code = 1; Base = 2; ExtraBits = 0 }
            { Code = 2; Base = 3; ExtraBits = 0 }
            { Code = 3; Base = 4; ExtraBits = 0 }
            { Code = 4; Base = 5; ExtraBits = 1 }
            { Code = 5; Base = 7; ExtraBits = 1 }
            { Code = 6; Base = 9; ExtraBits = 2 }
            { Code = 7; Base = 13; ExtraBits = 2 }
            { Code = 8; Base = 17; ExtraBits = 3 }
            { Code = 9; Base = 25; ExtraBits = 3 }
            { Code = 10; Base = 33; ExtraBits = 4 }
            { Code = 11; Base = 49; ExtraBits = 4 }
            { Code = 12; Base = 65; ExtraBits = 5 }
            { Code = 13; Base = 97; ExtraBits = 5 }
            { Code = 14; Base = 129; ExtraBits = 6 }
            { Code = 15; Base = 193; ExtraBits = 6 }
            { Code = 16; Base = 257; ExtraBits = 7 }
            { Code = 17; Base = 385; ExtraBits = 7 }
            { Code = 18; Base = 513; ExtraBits = 8 }
            { Code = 19; Base = 769; ExtraBits = 8 }
            { Code = 20; Base = 1025; ExtraBits = 9 }
            { Code = 21; Base = 1537; ExtraBits = 9 }
            { Code = 22; Base = 2049; ExtraBits = 10 }
            { Code = 23; Base = 3073; ExtraBits = 10 }
            { Code = 24; Base = 4097; ExtraBits = 11 }
            { Code = 25; Base = 6145; ExtraBits = 11 }
            { Code = 26; Base = 8193; ExtraBits = 12 }
            { Code = 27; Base = 12289; ExtraBits = 12 }
            { Code = 28; Base = 16385; ExtraBits = 13 }
            { Code = 29; Base = 24577; ExtraBits = 13 }
            { Code = 30; Base = 32769; ExtraBits = 14 }
            { Code = 31; Base = 49153; ExtraBits = 14 }
        |]

type BitBuilder() =
    let output = ResizeArray<byte>()
    let mutable buffer = 0
    let mutable bitPos = 0

    member _.WriteBitString(bits: string) =
        for bit in bits do
            if bit = '1' then
                buffer <- buffer ||| (1 <<< bitPos)

            bitPos <- bitPos + 1
            if bitPos = 8 then
                output.Add(byte (buffer &&& 0xFF))
                buffer <- 0
                bitPos <- 0

    member _.WriteRawBitsLsb(value: int, count: int) =
        for index in 0 .. count - 1 do
            if ((value >>> index) &&& 1) <> 0 then
                buffer <- buffer ||| (1 <<< bitPos)

            bitPos <- bitPos + 1
            if bitPos = 8 then
                output.Add(byte (buffer &&& 0xFF))
                buffer <- 0
                bitPos <- 0

    member _.ToArray() =
        if bitPos > 0 then
            output.Add(byte (buffer &&& 0xFF))
            buffer <- 0
            bitPos <- 0

        output.ToArray()

[<AbstractClass; Sealed>]
type Brotli =
    static member private MaxWindow = 65535
    static member private MinMatch = 4
    static member private MaxMatch = 258
    static member private MaxInsertPerIcc = 32

    static member Compress(data: byte array) =
        if isNull data then nullArg "data"

        if data.Length = 0 then
            [|
                0uy; 0uy; 0uy; 0uy
                1uy; 0uy; 0uy; 0uy; 0uy; 0uy
                63uy; 1uy
                0uy
            |]
        else
            let commands, flushLiterals = Brotli.BuildCommands(data)
            let literalFreq =
                [|
                    Dictionary<int, int>()
                    Dictionary<int, int>()
                    Dictionary<int, int>()
                    Dictionary<int, int>()
                |]

            let iccFreq = Dictionary<int, int>()
            let distFreq = Dictionary<int, int>()
            let history = ResizeArray<byte>()

            for command in commands do
                if command.CopyLength > 0 then
                    let icc = Brotli.FindIccCode(command.InsertLength, command.CopyLength)
                    iccFreq[icc] <- (if iccFreq.ContainsKey icc then iccFreq[icc] else 0) + 1

                    let distanceCode = Brotli.DistCode(command.CopyDistance)
                    distFreq[distanceCode] <- (if distFreq.ContainsKey distanceCode then distFreq[distanceCode] else 0) + 1

                    for literal in command.Literals do
                        let ctx = Brotli.LiteralContext(if history.Count > 0 then int history[history.Count - 1] else -1)
                        literalFreq[ctx][int literal] <- (if literalFreq[ctx].ContainsKey(int literal) then literalFreq[ctx][int literal] else 0) + 1
                        history.Add(literal)

                    let start = history.Count - command.CopyDistance
                    for index in 0 .. command.CopyLength - 1 do
                        history.Add(history[start + index])

            iccFreq[63] <- (if iccFreq.ContainsKey 63 then iccFreq[63] else 0) + 1

            let mutable previousFlush = if history.Count > 0 then int history[history.Count - 1] else -1
            for literal in flushLiterals do
                let ctx = Brotli.LiteralContext(previousFlush)
                literalFreq[ctx][int literal] <- (if literalFreq[ctx].ContainsKey(int literal) then literalFreq[ctx][int literal] else 0) + 1
                previousFlush <- int literal

            let iccCodeTable =
                HuffmanTree.Build(iccFreq |> Seq.map (fun pair -> pair.Key, pair.Value)).CanonicalCodeTable()

            let distCodeTable : IReadOnlyDictionary<int, string> =
                if distFreq.Count > 0 then
                    HuffmanTree.Build(distFreq |> Seq.map (fun pair -> pair.Key, pair.Value)).CanonicalCodeTable()
                else
                    upcast Dictionary<int, string>()

            let literalCodeTables =
                [|
                    for ctx in 0 .. 3 ->
                        if literalFreq[ctx].Count > 0 then
                            HuffmanTree.Build(literalFreq[ctx] |> Seq.map (fun pair -> pair.Key, pair.Value)).CanonicalCodeTable()
                        else
                            upcast Dictionary<int, string>()
                |]

            let builder = BitBuilder()
            let encodedHistory = ResizeArray<byte>()

            for command in commands do
                if command.CopyLength = 0 then
                    builder.WriteBitString(iccCodeTable[63])

                    let mutable flushPrevious = if encodedHistory.Count > 0 then int encodedHistory[encodedHistory.Count - 1] else -1
                    for literal in flushLiterals do
                        let ctx = Brotli.LiteralContext(flushPrevious)
                        builder.WriteBitString(literalCodeTables[ctx][int literal])
                        flushPrevious <- int literal
                else
                    let icc = Brotli.FindIccCode(command.InsertLength, command.CopyLength)
                    let entry = Tables.IccTable[icc]
                    builder.WriteBitString(iccCodeTable[icc])
                    builder.WriteRawBitsLsb(command.InsertLength - entry.InsertBase, entry.InsertExtra)
                    builder.WriteRawBitsLsb(command.CopyLength - entry.CopyBase, entry.CopyExtra)

                    for literal in command.Literals do
                        let ctx = Brotli.LiteralContext(if encodedHistory.Count > 0 then int encodedHistory[encodedHistory.Count - 1] else -1)
                        builder.WriteBitString(literalCodeTables[ctx][int literal])
                        encodedHistory.Add(literal)

                    let distanceCode = Brotli.DistCode(command.CopyDistance)
                    let distanceEntry = Tables.DistTable[distanceCode]
                    builder.WriteBitString(distCodeTable[distanceCode])
                    builder.WriteRawBitsLsb(command.CopyDistance - distanceEntry.Base, distanceEntry.ExtraBits)

                    let start = encodedHistory.Count - command.CopyDistance
                    for index in 0 .. command.CopyLength - 1 do
                        encodedHistory.Add(encodedHistory[start + index])

            let packedBits = builder.ToArray()
            let iccPairs = Brotli.SortedPairs(iccCodeTable)
            let distPairs = Brotli.SortedPairs(distCodeTable)
            let literalPairs = literalCodeTables |> Array.map Brotli.SortedPairs

            Brotli.EnsureCountFits("ICC entries", iccPairs.Length)
            Brotli.EnsureCountFits("distance entries", distPairs.Length)
            for ctx in 0 .. literalPairs.Length - 1 do
                Brotli.EnsureCountFits($"literal context {ctx} entries", literalPairs[ctx].Length)

            let totalSize =
                10
                + (iccPairs.Length * 2)
                + (distPairs.Length * 2)
                + (literalPairs |> Array.sumBy (fun pairs -> pairs.Length * 3))
                + packedBits.Length

            let output = Array.zeroCreate<byte> totalSize
            BinaryPrimitives.WriteUInt32BigEndian(output.AsSpan(0, 4), uint32 data.Length)
            output[4] <- byte iccPairs.Length
            output[5] <- byte distPairs.Length
            output[6] <- byte literalPairs[0].Length
            output[7] <- byte literalPairs[1].Length
            output[8] <- byte literalPairs[2].Length
            output[9] <- byte literalPairs[3].Length

            let mutable offset = 10
            for (symbol, length) in iccPairs do
                output[offset] <- byte symbol
                output[offset + 1] <- byte length
                offset <- offset + 2

            for (symbol, length) in distPairs do
                output[offset] <- byte symbol
                output[offset + 1] <- byte length
                offset <- offset + 2

            for pairs in literalPairs do
                for (symbol, length) in pairs do
                    BinaryPrimitives.WriteUInt16BigEndian(output.AsSpan(offset, 2), uint16 symbol)
                    output[offset + 2] <- byte length
                    offset <- offset + 3

            Array.Copy(packedBits, 0, output, offset, packedBits.Length)
            output

    static member Decompress(data: byte array) =
        if isNull data then nullArg "data"
        if data.Length < 10 then
            Array.empty
        else
            let originalLength = int (BinaryPrimitives.ReadUInt32BigEndian(data.AsSpan(0, 4)))
            if originalLength = 0 then
                Array.empty
            else
                let iccEntryCount = int data[4]
                let distEntryCount = int data[5]
                let literalEntryCounts = [| int data[6]; int data[7]; int data[8]; int data[9] |]
                let mutable offset = 10

                if offset + (iccEntryCount * 2) > data.Length then
                    Array.empty
                else
                    let iccLengths = ResizeArray<int * int>()
                    for _ in 0 .. iccEntryCount - 1 do
                        iccLengths.Add(int data[offset], int data[offset + 1])
                        offset <- offset + 2

                    if offset + (distEntryCount * 2) > data.Length then
                        Array.empty
                    else
                        let distLengths = ResizeArray<int * int>()
                        for _ in 0 .. distEntryCount - 1 do
                            distLengths.Add(int data[offset], int data[offset + 1])
                            offset <- offset + 2

                        let literalLengths = Array.init 4 (fun _ -> ResizeArray<int * int>())
                        let mutable valid = true
                        for ctx in 0 .. 3 do
                            let count = literalEntryCounts[ctx]
                            if offset + (count * 3) > data.Length then
                                valid <- false
                            elif count > 0 then
                                for _ in 0 .. count - 1 do
                                    literalLengths[ctx].Add(
                                        int (BinaryPrimitives.ReadUInt16BigEndian(data.AsSpan(offset, 2))),
                                        int data[offset + 2])
                                    offset <- offset + 3

                        if not valid then
                            Array.empty
                        else
                            let iccCodes = Brotli.ReconstructCanonicalCodes(iccLengths)
                            let distCodes = Brotli.ReconstructCanonicalCodes(distLengths)
                            let literalCodes = literalLengths |> Array.map Brotli.ReconstructCanonicalCodes
                            let bits = Brotli.UnpackBits(data.AsSpan(offset))
                            let output = ResizeArray<byte>(originalLength)
                            let mutable bitPos = 0
                            let mutable previous = -1

                            while output.Count < originalLength do
                                let icc = Brotli.NextHuffmanSymbol(iccCodes, bits, &bitPos)

                                if icc = 63 then
                                    while output.Count < originalLength do
                                        let ctx = Brotli.LiteralContext(previous)
                                        let literal = Brotli.NextHuffmanSymbol(literalCodes[ctx], bits, &bitPos)
                                        output.Add(byte literal)
                                        previous <- literal
                                else
                                    let entry = Tables.IccTable[icc]
                                    let insertLength = entry.InsertBase + Brotli.ReadRawBits(bits, &bitPos, entry.InsertExtra)
                                    let copyLength = entry.CopyBase + Brotli.ReadRawBits(bits, &bitPos, entry.CopyExtra)

                                    for _ in 0 .. insertLength - 1 do
                                        let ctx = Brotli.LiteralContext(previous)
                                        let literal = Brotli.NextHuffmanSymbol(literalCodes[ctx], bits, &bitPos)
                                        output.Add(byte literal)
                                        previous <- literal

                                    if copyLength > 0 then
                                        if distCodes.Count = 0 then
                                            invalidOp "Compressed stream references a distance code without a distance tree"

                                        let distanceCode = Brotli.NextHuffmanSymbol(distCodes, bits, &bitPos)
                                        let distanceEntry = Tables.DistTable[distanceCode]
                                        let copyDistance = distanceEntry.Base + Brotli.ReadRawBits(bits, &bitPos, distanceEntry.ExtraBits)
                                        let start = output.Count - copyDistance
                                        if start < 0 then
                                            invalidOp "Distance extends before the output buffer"

                                        for index in 0 .. copyLength - 1 do
                                            let value = output[start + index]
                                            output.Add(value)
                                            previous <- int value

                            output |> Seq.truncate originalLength |> Array.ofSeq

    static member private LiteralContext(previousByte: int) =
        if previousByte >= 0x61 && previousByte <= 0x7A then
            3
        elif previousByte >= 0x41 && previousByte <= 0x5A then
            2
        elif previousByte >= 0x30 && previousByte <= 0x39 then
            1
        else
            0

    static member private FindIccCode(insertLength: int, copyLength: int) =
        let mutable result = -1
        for code in 0 .. 62 do
            if result = -1 then
                let entry = Tables.IccTable[code]
                let maxInsert = entry.InsertBase + (1 <<< entry.InsertExtra) - 1
                let maxCopy = entry.CopyBase + (1 <<< entry.CopyExtra) - 1
                if insertLength >= entry.InsertBase
                   && insertLength <= maxInsert
                   && copyLength >= entry.CopyBase
                   && copyLength <= maxCopy then
                    result <- code

        if result >= 0 then
            result
        else
            let mutable fallback = 0
            let mutable found = false
            for code in 0 .. 15 do
                if not found then
                    let entry = Tables.IccTable[code]
                    let maxCopy = entry.CopyBase + (1 <<< entry.CopyExtra) - 1
                    if copyLength >= entry.CopyBase && copyLength <= maxCopy then
                        fallback <- code
                        found <- true

            fallback

    static member private FindBestIccCopy(insertLength: int, copyLength: int) =
        let mutable best = 0
        for code in 0 .. 62 do
            let entry = Tables.IccTable[code]
            let maxInsert = entry.InsertBase + (1 <<< entry.InsertExtra) - 1
            if insertLength >= entry.InsertBase && insertLength <= maxInsert then
                let maxCopy = entry.CopyBase + (1 <<< entry.CopyExtra) - 1
                if copyLength >= entry.CopyBase && copyLength <= maxCopy then
                    best <- copyLength
                elif maxCopy <= copyLength && maxCopy > best then
                    best <- maxCopy

        max best Brotli.MinMatch

    static member private DistCode(distance: int) =
        let mutable code = 31
        let mutable found = false
        for entry in Tables.DistTable do
            if not found then
                let maxDistance = entry.Base + (1 <<< entry.ExtraBits) - 1
                if distance <= maxDistance then
                    code <- entry.Code
                    found <- true

        code

    static member private FindLongestMatch(data: byte array, pos: int) =
        let windowStart = max 0 (pos - Brotli.MaxWindow)
        let mutable bestLength = 0
        let mutable bestOffset = 0

        for start = pos - 1 downto windowStart do
            if data[start] = data[pos] then
                let maxLength = min Brotli.MaxMatch (data.Length - pos)
                let mutable matchLength = 0
                while matchLength < maxLength && data[start + matchLength] = data[pos + matchLength] do
                    matchLength <- matchLength + 1

                if matchLength > bestLength then
                    bestLength <- matchLength
                    bestOffset <- pos - start

        if bestLength < Brotli.MinMatch then
            0, 0
        else
            bestOffset, bestLength

    static member private BuildCommands(data: byte array) =
        let commands = ResizeArray<Command>()
        let insertBuffer = ResizeArray<byte>()
        let mutable pos = 0

        while pos < data.Length do
            let offset, length = Brotli.FindLongestMatch(data, pos)
            if length >= Brotli.MinMatch && insertBuffer.Count <= Brotli.MaxInsertPerIcc then
                let actualCopy = Brotli.FindBestIccCopy(insertBuffer.Count, length)
                commands.Add(
                    {
                        InsertLength = insertBuffer.Count
                        CopyLength = actualCopy
                        CopyDistance = offset
                        Literals = insertBuffer.ToArray()
                    })
                insertBuffer.Clear()
                pos <- pos + actualCopy
            else
                insertBuffer.Add(data[pos])
                pos <- pos + 1

        let flushLiterals = insertBuffer.ToArray()
        commands.Add({ InsertLength = 0; CopyLength = 0; CopyDistance = 0; Literals = Array.empty })
        commands.ToArray(), flushLiterals

    static member private SortedPairs(table: IReadOnlyDictionary<int, string>) : (int * int) array =
        table
        |> Seq.map (fun pair -> pair.Key, pair.Value.Length)
        |> Seq.sortBy (fun (symbol, length) -> length, symbol)
        |> Seq.toArray

    static member private EnsureCountFits(label: string, count: int) =
        if count > int Byte.MaxValue then
            invalidOp $"{label} exceed the CMP06 one-byte header limit"

    static member private ReconstructCanonicalCodes(lengths: seq<int * int>) : IReadOnlyDictionary<string, int> =
        let ordered =
            lengths
            |> Seq.filter (fun (_, length) -> length > 0)
            |> Seq.sortBy (fun (symbol, length) -> length, symbol)
            |> Seq.toArray

        let codes = Dictionary<string, int>()
        if ordered.Length = 0 then
            codes :> IReadOnlyDictionary<string, int>
        elif ordered.Length = 1 then
            let (symbol, _) = ordered[0]
            codes["0"] <- symbol
            codes :> IReadOnlyDictionary<string, int>
        else
            let mutable codeValue = 0
            let mutable previousLength = ordered[0] |> snd
            for (symbol, length) in ordered do
                if length > previousLength then
                    codeValue <- codeValue <<< (length - previousLength)

                codes[Convert.ToString(codeValue, 2).PadLeft(length, '0')] <- symbol
                codeValue <- codeValue + 1
                previousLength <- length

            codes :> IReadOnlyDictionary<string, int>

    static member private UnpackBits(data: Span<byte>) =
        let builder = StringBuilder(data.Length * 8)
        for value in data do
            for bit = 0 to 7 do
                builder.Append(if (((int value) >>> bit) &&& 1) <> 0 then '1' else '0') |> ignore

        builder.ToString()

    static member private ReadRawBits(bits: string, bitPos: byref<int>, count: int) =
        let mutable value = 0
        for index in 0 .. count - 1 do
            if bitPos + index >= bits.Length then
                invalidOp "Unexpected end of compressed bit stream"

            if bits[bitPos + index] = '1' then
                value <- value ||| (1 <<< index)

        bitPos <- bitPos + count
        value

    static member private NextHuffmanSymbol(codes: IReadOnlyDictionary<string, int>, bits: string, bitPos: byref<int>) =
        if codes.Count = 0 then
            invalidOp "Missing Huffman codes"

        let builder = StringBuilder()
        let mutable result: int option = None
        while Option.isNone result && bitPos < bits.Length do
            builder.Append(bits[bitPos]) |> ignore
            bitPos <- bitPos + 1
            match codes.TryGetValue(builder.ToString()) with
            | true, symbol -> result <- Some symbol
            | _ -> ()

        result |> Option.defaultWith (fun () -> invalidOp "Unexpected end of compressed bit stream")
