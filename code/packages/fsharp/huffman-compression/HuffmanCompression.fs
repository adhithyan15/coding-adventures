namespace CodingAdventures.HuffmanCompression.FSharp

open System
open System.Buffers.Binary
open System.Collections.Generic
open System.Text
open CodingAdventures.HuffmanTree.FSharp

[<AbstractClass; Sealed>]
type HuffmanCompression =
    // Symbols are single bytes, so a valid table has at most 256 entries.
    static member private MaxSymbols = 256

    // Longest code length the decoder accepts; keeps every 64-bit shift safe.
    static member private MaxCodeLength = 56

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

    // The canonical reconstruction rule (same as DEFLATE):
    //   code = 0
    //   for each (symbol, length) in sorted order:
    //     if length > previous: code <<= (length - previous)
    //     assign code to symbol; code++
    //
    // SECURITY: the table is hostile input, so we reject
    //   * lengths of 0 or above MaxCodeLength (a zero-length code matches
    //     nothing; a long one could overflow the 64-bit shift),
    //   * tables not sorted by length (the encoder always sorts), and
    //   * over-subscribed tables (Kraft sum > 1): before each assignment the
    //     running code must still fit in `length` bits (code >>> length = 0).
    // F# masks a uint64 shift count to 6 bits; the 1..56 bound keeps every shift
    // below 64 so the mask can never change a result.
    // Codes are keyed by (length <<< 56 ||| code) so no strings are built.
    static member private BuildCanonicalDecodeTable(lengths: (int * int) seq) =
        let ordered = lengths |> Seq.toList
        let table = Dictionary<uint64, byte>()
        let mutable codeValue = 0UL
        let mutable previousLength = if List.isEmpty ordered then 0 else ordered |> List.head |> snd
        let mutable maxLength = 0

        for symbol, length in ordered do
            if length = 0 || length > HuffmanCompression.MaxCodeLength then
                invalidOp (sprintf "Code lengths must be positive and at most %d" HuffmanCompression.MaxCodeLength)

            if length < previousLength then
                invalidOp "Code-length table is not sorted by length"

            codeValue <- codeValue <<< (length - previousLength)
            if (codeValue >>> length) <> 0UL then
                invalidOp "Code-length table over-subscribes the code space"

            table.[(uint64 length <<< 56) ||| codeValue] <- byte symbol
            codeValue <- codeValue + 1UL
            previousLength <- length
            maxLength <- length

        table, maxLength

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
                // SECURITY: the header is attacker-controlled. Symbols are single
                // bytes, so a table can name at most 256 of them. Check before
                // computing the table size or sizing anything from it.
                if symbolCount > uint32 HuffmanCompression.MaxSymbols then
                    invalidOp (sprintf "symbol_count %d exceeds %d" symbolCount HuffmanCompression.MaxSymbols)

                let tableLength = int symbolCount * 2
                let bitStreamOffset = 8 + tableLength
                if bitStreamOffset > data.Length then
                    invalidOp "Compressed data ended before the code-length table completed"

                let lengths = ResizeArray<int * int>()
                let seen = Array.zeroCreate<bool> 256
                for index in 0 .. int symbolCount - 1 do
                    let offset = 8 + (index * 2)
                    let symbol = int data[offset]
                    let length = int data[offset + 1]

                    // A byte value can have only one code; a repeat makes the table ambiguous.
                    if seen[symbol] then
                        invalidOp (sprintf "Symbol %d appears twice in the code-length table" symbol)

                    seen[symbol] <- true
                    lengths.Add(symbol, length)

                let codeToSymbol, maxLength = HuffmanCompression.BuildCanonicalDecodeTable lengths

                // SECURITY: every code is at least one bit, so N payload bytes hold
                // at most 8*N symbols. Reject a larger declared length BEFORE
                // allocating the output array (which would otherwise be up to 2 GiB).
                let totalBits = int64 (data.Length - bitStreamOffset) * 8L
                if int64 originalLength > totalBits then
                    invalidOp (sprintf "Bit stream exhausted: declared length %d exceeds the %d bits available" originalLength totalBits)

                if originalLength > uint32 Int32.MaxValue then
                    invalidOp "CMP04 header fields exceed supported .NET array sizes"

                // Decode straight from the payload bytes. Bits are LSB-first within
                // each byte, so bit i is (data[offset + i/8] >>> (i%8)) &&& 1 -- O(1),
                // no expanded bit string. The candidate code is built MSB-first as an
                // integer; canonical codes are prefix-free so the first
                // (length, value) hit is the symbol. Once the accumulated length
                // reaches the longest code without a match, no later bit can match
                // (an incomplete table fed unmatched bits), so we fail immediately.
                let output = Array.zeroCreate<byte> (int originalLength)
                let mutable decoded = 0
                let mutable position = 0L
                let mutable accumulated = 0UL
                let mutable accumulatedLength = 0

                while decoded < output.Length do
                    if position >= totalBits then
                        invalidOp "Bit stream exhausted before decoding all symbols"

                    let bit = (int data[bitStreamOffset + int (position >>> 3)] >>> int (position &&& 7L)) &&& 1
                    accumulated <- (accumulated <<< 1) ||| uint64 bit
                    accumulatedLength <- accumulatedLength + 1
                    position <- position + 1L

                    match codeToSymbol.TryGetValue((uint64 accumulatedLength <<< 56) ||| accumulated) with
                    | true, symbol ->
                        output[decoded] <- symbol
                        decoded <- decoded + 1
                        accumulated <- 0UL
                        accumulatedLength <- 0
                    | _ ->
                        if accumulatedLength >= maxLength then
                            invalidOp "Bit pattern does not match any code"

                output
