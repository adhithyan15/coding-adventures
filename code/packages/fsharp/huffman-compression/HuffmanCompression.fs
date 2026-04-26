namespace CodingAdventures.HuffmanCompression.FSharp

open System
open System.Buffers.Binary
open System.Collections.Generic
open System.Text
open CodingAdventures.HuffmanTree.FSharp

[<AbstractClass; Sealed>]
type HuffmanCompression =
    static member private PackBits(data: byte array, table: IReadOnlyDictionary<int, string>) =
        let bitCount = data |> Array.sumBy (fun value -> table.[int value].Length)
        let output = Array.zeroCreate<byte> ((bitCount + 7) / 8)
        let mutable bitIndex = 0

        for value in data do
            for bit in table.[int value] do
                if bit = '1' then
                    let byteIndex = bitIndex / 8
                    let bitPosition = bitIndex % 8
                    output[byteIndex] <- output[byteIndex] ||| byte (1 <<< bitPosition)

                bitIndex <- bitIndex + 1

        output

    static member private BuildCanonicalDecodeTable(lengths: (int * int) seq) =
        let ordered = lengths |> Seq.toList
        let table = Dictionary<string, byte>()
        let mutable codeValue = 0
        let mutable previousLength = if List.isEmpty ordered then 0 else ordered |> List.head |> snd

        for symbol, length in ordered do
            if length > previousLength then
                codeValue <- codeValue <<< (length - previousLength)

            table.[Convert.ToString(codeValue, 2).PadLeft(length, '0')] <- byte symbol
            codeValue <- codeValue + 1
            previousLength <- length

        table

    static member Compress(data: byte array) =
        if isNull data || data.Length = 0 then
            Array.zeroCreate<byte> 8
        else
            let frequencies = Array.zeroCreate<int> 256
            for value in data do
                frequencies[int value] <- frequencies[int value] + 1

            let weights =
                seq {
                    for symbol in 0 .. 255 do
                        if frequencies[symbol] > 0 then
                            symbol, frequencies[symbol]
                }

            let table = (HuffmanTree.Build weights).CanonicalCodeTable()

            let lengths =
                table
                |> Seq.map (fun pair -> pair.Key, pair.Value.Length)
                |> Seq.sortBy (fun (symbol, length) -> length, symbol)
                |> Seq.toArray

            let bitBytes = HuffmanCompression.PackBits(data, table)
            let tableLength = lengths.Length * 2
            let output = Array.zeroCreate<byte> (8 + tableLength + bitBytes.Length)

            BinaryPrimitives.WriteUInt32BigEndian(output.AsSpan(0, 4), uint32 data.Length)
            BinaryPrimitives.WriteUInt32BigEndian(output.AsSpan(4, 4), uint32 lengths.Length)

            for index in 0 .. lengths.Length - 1 do
                let symbol, length = lengths[index]
                let offset = 8 + (index * 2)
                output[offset] <- byte symbol
                output[offset + 1] <- byte length

            Array.Copy(bitBytes, 0, output, 8 + tableLength, bitBytes.Length)
            output

    static member Decompress(data: byte array) =
        if isNull data || data.Length < 8 then
            [||]
        else
            let originalLength = BinaryPrimitives.ReadUInt32BigEndian(data.AsSpan(0, 4))
            let symbolCount = BinaryPrimitives.ReadUInt32BigEndian(data.AsSpan(4, 4))

            if originalLength = 0u then
                [||]
            else
                if originalLength > uint32 Int32.MaxValue
                   || symbolCount > uint32 ((Int32.MaxValue - 8) / 2) then
                    invalidOp "CMP04 header fields exceed supported .NET array sizes"

                let tableLength = int symbolCount * 2
                let bitStreamOffset = 8 + tableLength
                if bitStreamOffset > data.Length then
                    invalidOp "Compressed data ended before the code-length table completed"

                let lengths = ResizeArray<int * int>()
                for index in 0 .. int symbolCount - 1 do
                    let offset = 8 + (index * 2)
                    let symbol = int data[offset]
                    let length = int data[offset + 1]
                    if length = 0 then
                        invalidOp "Code lengths must be positive"

                    lengths.Add(symbol, length)

                let codeToSymbol = HuffmanCompression.BuildCanonicalDecodeTable lengths
                let output = Array.zeroCreate<byte> (int originalLength)
                let accumulated = StringBuilder()
                let mutable decoded = 0
                let mutable offset = bitStreamOffset

                while offset < data.Length && decoded < output.Length do
                    let value = data[offset]
                    for bit in 0 .. 7 do
                        if decoded < output.Length then
                            accumulated.Append(if ((int value >>> bit) &&& 1) = 1 then '1' else '0') |> ignore
                            match codeToSymbol.TryGetValue(accumulated.ToString()) with
                            | true, symbol ->
                                output[decoded] <- symbol
                                decoded <- decoded + 1
                                accumulated.Clear() |> ignore
                            | _ -> ()

                    offset <- offset + 1

                if decoded < output.Length then
                    invalidOp "Bit stream exhausted before decoding all symbols"

                output
