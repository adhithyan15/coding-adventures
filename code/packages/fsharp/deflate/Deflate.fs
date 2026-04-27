namespace CodingAdventures.Deflate.FSharp

open System
open System.Buffers.Binary
open System.Collections.Generic
open System.Text
open CodingAdventures.HuffmanTree.FSharp
open CodingAdventures.Lzss.FSharp

module private Tables =
    let LengthTable =
        [|
            (257, 3, 0); (258, 4, 0); (259, 5, 0); (260, 6, 0)
            (261, 7, 0); (262, 8, 0); (263, 9, 0); (264, 10, 0)
            (265, 11, 1); (266, 13, 1); (267, 15, 1); (268, 17, 1)
            (269, 19, 2); (270, 23, 2); (271, 27, 2); (272, 31, 2)
            (273, 35, 3); (274, 43, 3); (275, 51, 3); (276, 59, 3)
            (277, 67, 4); (278, 83, 4); (279, 99, 4); (280, 115, 4)
            (281, 131, 5); (282, 163, 5); (283, 195, 5); (284, 227, 5)
        |]

    let DistTable =
        [|
            (0, 1, 0); (1, 2, 0); (2, 3, 0); (3, 4, 0)
            (4, 5, 1); (5, 7, 1); (6, 9, 2); (7, 13, 2)
            (8, 17, 3); (9, 25, 3); (10, 33, 4); (11, 49, 4)
            (12, 65, 5); (13, 97, 5); (14, 129, 6); (15, 193, 6)
            (16, 257, 7); (17, 385, 7); (18, 513, 8); (19, 769, 8)
            (20, 1025, 9); (21, 1537, 9); (22, 2049, 10); (23, 3073, 10)
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
type Deflate =
    static member Compress
        (
            data: byte array,
            ?windowSize: int,
            ?maxMatch: int,
            ?minMatch: int
        ) =
        if isNull data then nullArg "data"

        let windowSize = defaultArg windowSize Lzss.DefaultWindowSize
        let maxMatch = defaultArg maxMatch Lzss.DefaultMaxMatch
        let minMatch = defaultArg minMatch Lzss.DefaultMinMatch

        if data.Length = 0 then
            let empty = Array.zeroCreate<byte> 12
            BinaryPrimitives.WriteUInt32BigEndian(empty.AsSpan(0, 4), 0u)
            BinaryPrimitives.WriteUInt16BigEndian(empty.AsSpan(4, 2), 1us)
            BinaryPrimitives.WriteUInt16BigEndian(empty.AsSpan(6, 2), 0us)
            BinaryPrimitives.WriteUInt16BigEndian(empty.AsSpan(8, 2), 256us)
            empty[10] <- 1uy
            empty[11] <- 0uy
            empty
        else
            let tokens = Lzss.Encode(data, windowSize, maxMatch, minMatch)
            let llFreq = Dictionary<int, int>()
            let distFreq = Dictionary<int, int>()

            for token in tokens do
                match token with
                | Literal value ->
                    llFreq[value |> int] <- (if llFreq.ContainsKey(int value) then llFreq[int value] else 0) + 1
                | Match(offset, length) ->
                    let lengthSymbol, _, _ = Deflate.FindLengthEntry(length)
                    let distanceCode, _, _ = Deflate.FindDistEntry(offset)
                    llFreq[lengthSymbol] <- (if llFreq.ContainsKey(lengthSymbol) then llFreq[lengthSymbol] else 0) + 1
                    distFreq[distanceCode] <- (if distFreq.ContainsKey(distanceCode) then distFreq[distanceCode] else 0) + 1

            llFreq[256] <- (if llFreq.ContainsKey(256) then llFreq[256] else 0) + 1

            let llCodes =
                HuffmanTree.Build(llFreq |> Seq.map (fun pair -> pair.Key, pair.Value)).CanonicalCodeTable()

            let distCodes : IReadOnlyDictionary<int, string> =
                if distFreq.Count > 0 then
                    HuffmanTree.Build(distFreq |> Seq.map (fun pair -> pair.Key, pair.Value)).CanonicalCodeTable()
                else
                    upcast Dictionary<int, string>()

            let bits = BitBuilder()
            for token in tokens do
                match token with
                | Literal value ->
                    bits.WriteBitString(llCodes[int value])
                | Match(offset, length) ->
                    let lengthSymbol, lengthBase, lengthExtra = Deflate.FindLengthEntry(length)
                    bits.WriteBitString(llCodes[lengthSymbol])
                    bits.WriteRawBitsLsb(length - lengthBase, lengthExtra)

                    let distanceCode, distBase, distExtra = Deflate.FindDistEntry(offset)
                    bits.WriteBitString(distCodes[distanceCode])
                    bits.WriteRawBitsLsb(offset - distBase, distExtra)

            bits.WriteBitString(llCodes[256])
            let packedBits = bits.ToArray()

            let llPairs =
                llCodes
                |> Seq.map (fun pair -> pair.Key, pair.Value.Length)
                |> Seq.sortBy (fun (symbol, length) -> length, symbol)
                |> Seq.toList

            let distPairs =
                distCodes
                |> Seq.map (fun pair -> pair.Key, pair.Value.Length)
                |> Seq.sortBy (fun (symbol, length) -> length, symbol)
                |> Seq.toList

            let totalSize = 8 + (llPairs.Length * 3) + (distPairs.Length * 3) + packedBits.Length
            let output = Array.zeroCreate<byte> totalSize
            BinaryPrimitives.WriteUInt32BigEndian(output.AsSpan(0, 4), uint32 data.Length)
            BinaryPrimitives.WriteUInt16BigEndian(output.AsSpan(4, 2), uint16 llPairs.Length)
            BinaryPrimitives.WriteUInt16BigEndian(output.AsSpan(6, 2), uint16 distPairs.Length)

            let mutable offset = 8
            for (symbol, length) in llPairs do
                BinaryPrimitives.WriteUInt16BigEndian(output.AsSpan(offset, 2), uint16 symbol)
                output[offset + 2] <- byte length
                offset <- offset + 3

            for (symbol, length) in distPairs do
                BinaryPrimitives.WriteUInt16BigEndian(output.AsSpan(offset, 2), uint16 symbol)
                output[offset + 2] <- byte length
                offset <- offset + 3

            Array.Copy(packedBits, 0, output, offset, packedBits.Length)
            output

    static member Decompress(data: byte array) =
        if isNull data then nullArg "data"
        if data.Length < 8 then
            Array.empty
        else
            let originalLength = int (BinaryPrimitives.ReadUInt32BigEndian(data.AsSpan(0, 4)))
            let llEntryCount = int (BinaryPrimitives.ReadUInt16BigEndian(data.AsSpan(4, 2)))
            let distEntryCount = int (BinaryPrimitives.ReadUInt16BigEndian(data.AsSpan(6, 2)))

            if originalLength = 0 then
                Array.empty
            else
                let mutable offset = 8
                let llLengths = ResizeArray<int * int>()
                let distLengths = ResizeArray<int * int>()

                let mutable valid = true
                for _ in 0 .. llEntryCount - 1 do
                    if offset + 3 > data.Length then
                        valid <- false
                    elif valid then
                        llLengths.Add(
                            int (BinaryPrimitives.ReadUInt16BigEndian(data.AsSpan(offset, 2))),
                            int data[offset + 2]
                        )
                        offset <- offset + 3

                for _ in 0 .. distEntryCount - 1 do
                    if offset + 3 > data.Length then
                        valid <- false
                    elif valid then
                        distLengths.Add(
                            int (BinaryPrimitives.ReadUInt16BigEndian(data.AsSpan(offset, 2))),
                            int data[offset + 2]
                        )
                        offset <- offset + 3

                if not valid then
                    Array.empty
                else
                    let llCodes = Deflate.ReconstructCanonicalCodes(llLengths)
                    let distCodes = Deflate.ReconstructCanonicalCodes(distLengths)
                    let bits = Deflate.UnpackBits(data[offset..])
                    let output = ResizeArray<byte>(originalLength)
                    let mutable bitPos = 0
                    let mutable keepReading = true

                    while keepReading do
                        let llSymbol = Deflate.NextHuffmanSymbol(llCodes, bits, &bitPos)
                        if llSymbol = 256 then
                            keepReading <- false
                        elif llSymbol < 256 then
                            output.Add(byte llSymbol)
                        else
                            let _, lengthBase, lengthExtra = Deflate.FindLengthEntryBySymbol(llSymbol)
                            let length = lengthBase + Deflate.ReadRawBits(bits, &bitPos, lengthExtra)

                            if distCodes.Count = 0 then
                                invalidOp "Compressed stream references distance codes without a distance tree"

                            let distSymbol = Deflate.NextHuffmanSymbol(distCodes, bits, &bitPos)
                            let _, distBase, distExtra = Deflate.FindDistEntryByCode(distSymbol)
                            let distance = distBase + Deflate.ReadRawBits(bits, &bitPos, distExtra)

                            let start = output.Count - distance
                            if start < 0 then
                                invalidOp "Distance extends before the output buffer"

                            for index in 0 .. length - 1 do
                                output.Add(output[start + index])

                    output |> Seq.truncate originalLength |> Array.ofSeq

    static member private FindLengthEntry(length: int) =
        Tables.LengthTable
        |> Array.tryFind (fun (_, baseLength, extraBits) -> length <= baseLength + ((1 <<< extraBits) - 1))
        |> Option.defaultValue Tables.LengthTable[Tables.LengthTable.Length - 1]

    static member private FindLengthEntryBySymbol(symbol: int) =
        Tables.LengthTable
        |> Array.tryFind (fun (entrySymbol, _, _) -> entrySymbol = symbol)
        |> Option.defaultWith (fun () -> invalidOp $"Unknown length symbol {symbol}")

    static member private FindDistEntry(distance: int) =
        Tables.DistTable
        |> Array.tryFind (fun (_, baseDistance, extraBits) -> distance <= baseDistance + ((1 <<< extraBits) - 1))
        |> Option.defaultValue Tables.DistTable[Tables.DistTable.Length - 1]

    static member private FindDistEntryByCode(code: int) =
        Tables.DistTable
        |> Array.tryFind (fun (entryCode, _, _) -> entryCode = code)
        |> Option.defaultWith (fun () -> invalidOp $"Unknown distance symbol {code}")

    static member private ReconstructCanonicalCodes(lengths: seq<int * int>) =
        let ordered =
            lengths
            |> Seq.filter (fun (_, length) -> length > 0)
            |> Seq.sortBy (fun (symbol, length) -> length, symbol)
            |> Seq.toList

        let result = Dictionary<string, int>()
        match ordered with
        | [] -> result :> IReadOnlyDictionary<string, int>
        | [ symbol, _ ] ->
            result["0"] <- symbol
            result :> IReadOnlyDictionary<string, int>
        | _ ->
            let mutable code = 0
            let mutable previousLength = ordered |> List.head |> snd
            for (symbol, length) in ordered do
                if length > previousLength then
                    code <- code <<< (length - previousLength)

                result[Convert.ToString(code, 2).PadLeft(length, '0')] <- symbol
                code <- code + 1
                previousLength <- length

            result :> IReadOnlyDictionary<string, int>

    static member private UnpackBits(data: byte array) =
        let builder = StringBuilder(data.Length * 8)
        for value in data do
            for bit = 0 to 7 do
                builder.Append(if ((int value >>> bit) &&& 1) <> 0 then '1' else '0') |> ignore

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
        let mutable found: int option = None
        while Option.isNone found && bitPos < bits.Length do
            builder.Append(bits[bitPos]) |> ignore
            bitPos <- bitPos + 1
            match codes.TryGetValue(builder.ToString()) with
            | true, symbol -> found <- Some symbol
            | _ -> ()

        found |> Option.defaultWith (fun () -> invalidOp "Unexpected end of compressed bit stream")
