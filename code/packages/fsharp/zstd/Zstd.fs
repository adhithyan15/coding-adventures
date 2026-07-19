namespace CodingAdventures.Zstd.FSharp

open System
open System.Buffers.Binary
open System.Collections.Generic
open System.IO
open System.Numerics
open CodingAdventures.Lzss.FSharp

type private CodeRange = { Baseline: int; ExtraBits: int }
type private Sequence = { LiteralLength: int; MatchLength: int; Offset: int }
type private DecodeEntry = { Symbol: int; Bits: int; Baseline: int }
type private EncodeEntry = { DeltaBits: int; DeltaFindState: int }

type private ReverseBitWriter() =
    let bytes = ResizeArray<byte>()
    let mutable register = 0UL
    let mutable bitCount = 0

    member _.AddBits(value: uint64, count: int) =
        if count > 0 then
            let mask = (1UL <<< count) - 1UL
            register <- register ||| ((value &&& mask) <<< bitCount)
            bitCount <- bitCount + count

            while bitCount >= 8 do
                bytes.Add(byte register)
                register <- register >>> 8
                bitCount <- bitCount - 8

    member _.Finish() =
        bytes.Add(byte ((register &&& 0xFFUL) ||| (1UL <<< bitCount)))
        register <- 0UL
        bitCount <- 0
        bytes.ToArray()

type private ReverseBitReader(data: byte array) =
    do
        if isNull data then nullArg "data"
        if data.Length = 0 then raise (InvalidDataException("empty bitstream"))
        if data[data.Length - 1] = 0uy then raise (InvalidDataException("bitstream last byte has no sentinel"))

    let mutable register = 0UL
    let mutable bitCount = 0
    let mutable position = data.Length - 1

    let reload () =
        while bitCount <= 56 && position > 0 do
            position <- position - 1
            register <- register ||| (uint64 data[position] <<< (64 - bitCount - 8))
            bitCount <- bitCount + 8

    do
        let last = data[data.Length - 1]
        let sentinelPosition = BitOperations.Log2(uint32 last)
        bitCount <- sentinelPosition
        let mask = if bitCount = 0 then 0UL else (1UL <<< bitCount) - 1UL
        register <- if bitCount = 0 then 0UL else (uint64 last &&& mask) <<< (64 - bitCount)
        reload ()

    member _.ReadBits(count: int) =
        if count = 0 then
            0
        elif count < 0 || count > 32 || count > bitCount then
            raise (InvalidDataException("bitstream is truncated"))
        else
            let value = int (register >>> (64 - count))
            register <- register <<< count
            bitCount <- bitCount - count
            if bitCount < 24 then reload ()
            value

/// Educational Zstandard encoder and decoder using raw literals and predefined FSE tables.
[<AbstractClass; Sealed>]
type Zstd private () =
    static let magic = 0xFD2FB528u
    static let maxBlockSize = 128 * 1024
    static let maxOutputSize = 256 * 1024 * 1024
    static let literalAccuracyLog = 6
    static let matchAccuracyLog = 6
    static let offsetAccuracyLog = 5

    static let literalCodes =
        [| { Baseline = 0; ExtraBits = 0 }; { Baseline = 1; ExtraBits = 0 }
           { Baseline = 2; ExtraBits = 0 }; { Baseline = 3; ExtraBits = 0 }
           { Baseline = 4; ExtraBits = 0 }; { Baseline = 5; ExtraBits = 0 }
           { Baseline = 6; ExtraBits = 0 }; { Baseline = 7; ExtraBits = 0 }
           { Baseline = 8; ExtraBits = 0 }; { Baseline = 9; ExtraBits = 0 }
           { Baseline = 10; ExtraBits = 0 }; { Baseline = 11; ExtraBits = 0 }
           { Baseline = 12; ExtraBits = 0 }; { Baseline = 13; ExtraBits = 0 }
           { Baseline = 14; ExtraBits = 0 }; { Baseline = 15; ExtraBits = 0 }
           { Baseline = 16; ExtraBits = 1 }; { Baseline = 18; ExtraBits = 1 }
           { Baseline = 20; ExtraBits = 1 }; { Baseline = 22; ExtraBits = 1 }
           { Baseline = 24; ExtraBits = 2 }; { Baseline = 28; ExtraBits = 2 }
           { Baseline = 32; ExtraBits = 3 }; { Baseline = 40; ExtraBits = 3 }
           { Baseline = 48; ExtraBits = 4 }; { Baseline = 64; ExtraBits = 6 }
           { Baseline = 128; ExtraBits = 7 }; { Baseline = 256; ExtraBits = 8 }
           { Baseline = 512; ExtraBits = 9 }; { Baseline = 1024; ExtraBits = 10 }
           { Baseline = 2048; ExtraBits = 11 }; { Baseline = 4096; ExtraBits = 12 }
           { Baseline = 8192; ExtraBits = 13 }; { Baseline = 16384; ExtraBits = 14 }
           { Baseline = 32768; ExtraBits = 15 }; { Baseline = 65536; ExtraBits = 16 } |]

    static let matchCodes =
        Array.append
            [| for value in 3..34 -> { Baseline = value; ExtraBits = 0 } |]
            [| { Baseline = 35; ExtraBits = 1 }; { Baseline = 37; ExtraBits = 1 }
               { Baseline = 39; ExtraBits = 1 }; { Baseline = 41; ExtraBits = 1 }
               { Baseline = 43; ExtraBits = 2 }; { Baseline = 47; ExtraBits = 2 }
               { Baseline = 51; ExtraBits = 3 }; { Baseline = 59; ExtraBits = 3 }
               { Baseline = 67; ExtraBits = 4 }; { Baseline = 83; ExtraBits = 4 }
               { Baseline = 99; ExtraBits = 5 }; { Baseline = 131; ExtraBits = 7 }
               { Baseline = 259; ExtraBits = 8 }; { Baseline = 515; ExtraBits = 9 }
               { Baseline = 1027; ExtraBits = 10 }; { Baseline = 2051; ExtraBits = 11 }
               { Baseline = 4099; ExtraBits = 12 }; { Baseline = 8195; ExtraBits = 13 }
               { Baseline = 16387; ExtraBits = 14 }; { Baseline = 32771; ExtraBits = 15 }
               { Baseline = 65539; ExtraBits = 16 } |]

    static let literalNorm =
        [| 4; 3; 2; 2; 2; 2; 2; 2; 2; 2; 2; 2; 2; 1; 1; 1
           2; 2; 2; 2; 2; 2; 2; 2; 2; 3; 2; 1; 1; 1; 1; 1
           -1; -1; -1; -1 |]

    static let matchNorm =
        [| 1; 4; 3; 2; 2; 2; 2; 2; 2; 1; 1; 1; 1; 1; 1; 1
           1; 1; 1; 1; 1; 1; 1; 1; 1; 1; 1; 1; 1; 1; 1; 1
           1; 1; 1; 1; 1; 1; 1; 1; 1; 1; 1; 1; 1; 1; -1; -1
           -1; -1; -1; -1; -1 |]

    static let offsetNorm =
        [| 1; 1; 1; 1; 1; 1; 2; 2; 2; 1; 1; 1; 1; 1; 1; 1
           1; 1; 1; 1; 1; 1; 1; 1; -1; -1; -1; -1; -1 |]

    static member Magic = magic
    static member MaxBlockSize = maxBlockSize
    static member MaxOutputSize = maxOutputSize

    static member private WriteBytes(stream: MemoryStream, bytes: byte array) =
        stream.Write(bytes, 0, bytes.Length)

    static member private RequireAvailable(length: int, position: int, count: int, field: string) =
        if position < 0 || count < 0 || position > length - count then
            raise (InvalidDataException($"truncated {field}"))

    static member private EnsureOutputLimit(current: int, additional: int) =
        if additional < 0 || current > maxOutputSize - additional then
            raise (InvalidDataException($"decompressed size exceeds {maxOutputSize} bytes"))

    static member private ValidateNormalizedCounts(normalized: int array, tableSize: int) =
        let mutable total = 0
        for count in normalized do
            if count < -1 then raise (InvalidDataException("invalid normalized FSE count"))
            total <- total + (if count = -1 then 1 else count)
        if total <> tableSize then raise (InvalidDataException("normalized FSE counts do not fill the table"))

    static member private BuildDecodeTable(normalized: int array, accuracyLog: int) =
        let size = 1 <<< accuracyLog
        Zstd.ValidateNormalizedCounts(normalized, size)
        let step = (size >>> 1) + (size >>> 3) + 3
        let symbols = Array.zeroCreate<int> size
        let symbolNext = Array.zeroCreate<int> normalized.Length
        let mutable high = size - 1

        for symbol in 0..normalized.Length - 1 do
            if normalized[symbol] = -1 then
                symbols[high] <- symbol
                high <- high - 1
                symbolNext[symbol] <- 1

        let mutable position = 0
        for pass in 0..1 do
            for symbol in 0..normalized.Length - 1 do
                let count = normalized[symbol]
                if count > 0 && ((pass = 0) = (count > 1)) then
                    symbolNext[symbol] <- count
                    for _ in 1..count do
                        symbols[position] <- symbol
                        position <- (position + step) &&& (size - 1)
                        while position > high do
                            position <- (position + step) &&& (size - 1)

        let next = Array.copy symbolNext
        Array.init size (fun index ->
            let symbol = symbols[index]
            let nextState = next[symbol]
            next[symbol] <- nextState + 1
            let bits = accuracyLog - BitOperations.Log2(uint32 nextState)
            { Symbol = symbol; Bits = bits; Baseline = (nextState <<< bits) - size })

    static member private BuildEncodeTables(normalized: int array, accuracyLog: int) =
        let size = 1 <<< accuracyLog
        Zstd.ValidateNormalizedCounts(normalized, size)
        let cumulative = Array.zeroCreate<int> normalized.Length
        let mutable total = 0
        for symbol in 0..normalized.Length - 1 do
            cumulative[symbol] <- total
            total <- total + (if normalized[symbol] = -1 then 1 else max normalized[symbol] 0)

        let step = (size >>> 1) + (size >>> 3) + 3
        let spread = Array.zeroCreate<int> size
        let mutable high = size - 1
        for symbol in 0..normalized.Length - 1 do
            if normalized[symbol] = -1 then
                spread[high] <- symbol
                high <- high - 1

        let mutable position = 0
        for pass in 0..1 do
            for symbol in 0..normalized.Length - 1 do
                let count = normalized[symbol]
                if count > 0 && ((pass = 0) = (count > 1)) then
                    for _ in 1..count do
                        spread[position] <- symbol
                        position <- (position + step) &&& (size - 1)
                        while position > high do
                            position <- (position + step) &&& (size - 1)

        let occurrences = Array.zeroCreate<int> normalized.Length
        let states = Array.zeroCreate<int> size
        for index in 0..size - 1 do
            let symbol = spread[index]
            states[cumulative[symbol] + occurrences[symbol]] <- index + size
            occurrences[symbol] <- occurrences[symbol] + 1

        let entries =
            Array.init normalized.Length (fun symbol ->
                let count = if normalized[symbol] = -1 then 1 else max normalized[symbol] 0
                if count = 0 then
                    { DeltaBits = 0; DeltaFindState = 0 }
                else
                    let maxBits = if count = 1 then accuracyLog else accuracyLog - BitOperations.Log2(uint32 count)
                    { DeltaBits = (maxBits <<< 16) - (count <<< maxBits)
                      DeltaFindState = cumulative[symbol] - count })
        entries, states

    static member private EncodeSymbol(state: int, symbol: int, entries: EncodeEntry array, states: int array) =
        if symbol < 0 || symbol >= entries.Length then
            raise (InvalidDataException("FSE symbol is outside the predefined table"))
        let entry = entries[symbol]
        let bits = (state + entry.DeltaBits) >>> 16
        let value = state &&& ((1 <<< bits) - 1)
        let slot = (state >>> bits) + entry.DeltaFindState
        if slot < 0 || slot >= states.Length then raise (InvalidDataException("invalid FSE encoder state"))
        states[slot], bits, value

    static member private DecodeSymbol(state: int, table: DecodeEntry array, reader: ReverseBitReader) =
        if state < 0 || state >= table.Length then raise (InvalidDataException("invalid FSE decoder state"))
        let entry = table[state]
        entry.Symbol, entry.Baseline + reader.ReadBits entry.Bits

    static member private ValueToCode(value: int, codes: CodeRange array) =
        let mutable code = 0
        let mutable index = 0
        while index < codes.Length && codes[index].Baseline <= value do
            code <- index
            index <- index + 1
        code

    static member private AllEqual(data: byte array) =
        let mutable equal = true
        let mutable index = 1
        while equal && index < data.Length do
            equal <- data[index] = data[0]
            index <- index + 1
        equal

    static member private WriteBlockHeader(output: MemoryStream, size: int, blockType: int, last: bool) =
        let value = (size <<< 3) ||| (blockType <<< 1) ||| (if last then 1 else 0)
        output.WriteByte(byte value)
        output.WriteByte(byte (value >>> 8))
        output.WriteByte(byte (value >>> 16))

    static member private EncodeLiterals(literals: ResizeArray<byte>) =
        use output = new MemoryStream()
        let count = literals.Count
        if count <= 31 then
            output.WriteByte(byte (count <<< 3))
        elif count <= 4095 then
            let header = (count <<< 4) ||| 4
            output.WriteByte(byte header)
            output.WriteByte(byte (header >>> 8))
        else
            let header = (count <<< 4) ||| 12
            output.WriteByte(byte header)
            output.WriteByte(byte (header >>> 8))
            output.WriteByte(byte (header >>> 16))
        for literal in literals do output.WriteByte literal
        output.ToArray()

    static member private DecodeLiterals(data: byte array) =
        Zstd.RequireAvailable(data.Length, 0, 1, "literals section")
        let first = data[0]
        if (first &&& 3uy) <> 0uy then raise (InvalidDataException("only raw literals are supported"))
        let sizeFormat = int ((first >>> 2) &&& 3uy)
        let count, headerBytes =
            match sizeFormat with
            | 0 | 2 -> int (first >>> 3), 1
            | 1 ->
                Zstd.RequireAvailable(data.Length, 0, 2, "literals header")
                (int (first >>> 4) ||| (int data[1] <<< 4)), 2
            | _ ->
                Zstd.RequireAvailable(data.Length, 0, 3, "literals header")
                (int (first >>> 4) ||| (int data[1] <<< 4) ||| (int data[2] <<< 12)), 3
        Zstd.RequireAvailable(data.Length, headerBytes, count, "literals data")
        data[headerBytes..headerBytes + count - 1], headerBytes + count

    static member private EncodeSequenceCount(count: int) =
        if count < 128 then
            [| byte count |]
        elif count < 0x7F00 then
            [| byte (0x80 ||| (count >>> 8)); byte count |]
        else
            let remainder = count - 0x7F00
            [| 0xFFuy; byte remainder; byte (remainder >>> 8) |]

    static member private DecodeSequenceCount(data: byte array) =
        Zstd.RequireAvailable(data.Length, 0, 1, "sequence count")
        let first = data[0]
        if first < 128uy then
            int first, 1
        elif first < 0xFFuy then
            Zstd.RequireAvailable(data.Length, 0, 2, "sequence count")
            ((int first &&& 0x7F) <<< 8) ||| int data[1], 2
        else
            Zstd.RequireAvailable(data.Length, 0, 3, "sequence count")
            0x7F00 + int data[1] + (int data[2] <<< 8), 3

    static member private EncodeSequences(sequences: ResizeArray<Sequence>) =
        let literalEntries, literalStates = Zstd.BuildEncodeTables(literalNorm, literalAccuracyLog)
        let matchEntries, matchStates = Zstd.BuildEncodeTables(matchNorm, matchAccuracyLog)
        let offsetEntries, offsetStates = Zstd.BuildEncodeTables(offsetNorm, offsetAccuracyLog)
        let literalSize = 1 <<< literalAccuracyLog
        let matchSize = 1 <<< matchAccuracyLog
        let offsetSize = 1 <<< offsetAccuracyLog
        let mutable literalState = literalSize
        let mutable matchState = matchSize
        let mutable offsetState = offsetSize
        let writer = ReverseBitWriter()

        for index in sequences.Count - 1 .. -1 .. 0 do
            let sequence = sequences[index]
            let literalCode = Zstd.ValueToCode(sequence.LiteralLength, literalCodes)
            let matchCode = Zstd.ValueToCode(sequence.MatchLength, matchCodes)
            let rawOffset = sequence.Offset + 3
            let offsetCode = BitOperations.Log2(uint32 rawOffset)
            writer.AddBits(uint64 (rawOffset - (1 <<< offsetCode)), offsetCode)
            writer.AddBits(uint64 (sequence.MatchLength - matchCodes[matchCode].Baseline), matchCodes[matchCode].ExtraBits)
            writer.AddBits(uint64 (sequence.LiteralLength - literalCodes[literalCode].Baseline), literalCodes[literalCode].ExtraBits)

            let newMatchState, matchBits, matchValue = Zstd.EncodeSymbol(matchState, matchCode, matchEntries, matchStates)
            matchState <- newMatchState
            writer.AddBits(uint64 matchValue, matchBits)
            let newOffsetState, offsetBits, offsetValue = Zstd.EncodeSymbol(offsetState, offsetCode, offsetEntries, offsetStates)
            offsetState <- newOffsetState
            writer.AddBits(uint64 offsetValue, offsetBits)
            let newLiteralState, literalBits, literalValue = Zstd.EncodeSymbol(literalState, literalCode, literalEntries, literalStates)
            literalState <- newLiteralState
            writer.AddBits(uint64 literalValue, literalBits)

        writer.AddBits(uint64 (offsetState - offsetSize), offsetAccuracyLog)
        writer.AddBits(uint64 (matchState - matchSize), matchAccuracyLog)
        writer.AddBits(uint64 (literalState - literalSize), literalAccuracyLog)
        writer.Finish()

    static member private CompressBlock(block: byte array) =
        let tokens = Lzss.Encode(block, windowSize = Lzss.DefaultWindowSize, maxMatch = 255, minMatch = 3)
        let literals = ResizeArray<byte>()
        let sequences = ResizeArray<Sequence>()
        let mutable literalRun = 0
        for token in tokens do
            match token with
            | Literal value ->
                literals.Add value
                literalRun <- literalRun + 1
            | Match(offset, length) ->
                sequences.Add({ LiteralLength = literalRun; MatchLength = length; Offset = offset })
                literalRun <- 0

        if sequences.Count = 0 then
            None
        else
            use output = new MemoryStream()
            Zstd.WriteBytes(output, Zstd.EncodeLiterals literals)
            Zstd.WriteBytes(output, Zstd.EncodeSequenceCount sequences.Count)
            output.WriteByte 0uy
            Zstd.WriteBytes(output, Zstd.EncodeSequences sequences)
            let result = output.ToArray()
            if result.Length < block.Length then Some result else None

    static member private DecompressBlock(data: byte array, output: ResizeArray<byte>) =
        let literals, literalBytes = Zstd.DecodeLiterals data
        let mutable position = literalBytes
        if position >= data.Length then
            Zstd.EnsureOutputLimit(output.Count, literals.Length)
            output.AddRange literals
        else
            let sequenceCount, countBytes = Zstd.DecodeSequenceCount data[position..]
            position <- position + countBytes
            if sequenceCount = 0 then
                Zstd.EnsureOutputLimit(output.Count, literals.Length)
                output.AddRange literals
            else
                Zstd.RequireAvailable(data.Length, position, 1, "symbol compression modes")
                let modes = data[position]
                position <- position + 1
                if (modes &&& 0xFCuy) <> 0uy then
                    raise (InvalidDataException("only predefined FSE modes are supported"))

                let reader = ReverseBitReader data[position..]
                let literalTable = Zstd.BuildDecodeTable(literalNorm, literalAccuracyLog)
                let matchTable = Zstd.BuildDecodeTable(matchNorm, matchAccuracyLog)
                let offsetTable = Zstd.BuildDecodeTable(offsetNorm, offsetAccuracyLog)
                let mutable literalState = reader.ReadBits literalAccuracyLog
                let mutable matchState = reader.ReadBits matchAccuracyLog
                let mutable offsetState = reader.ReadBits offsetAccuracyLog
                let mutable literalPosition = 0

                for _ in 1..sequenceCount do
                    let literalCode, nextLiteralState = Zstd.DecodeSymbol(literalState, literalTable, reader)
                    let offsetCode, nextOffsetState = Zstd.DecodeSymbol(offsetState, offsetTable, reader)
                    let matchCode, nextMatchState = Zstd.DecodeSymbol(matchState, matchTable, reader)
                    literalState <- nextLiteralState
                    offsetState <- nextOffsetState
                    matchState <- nextMatchState
                    if literalCode < 0 || literalCode >= literalCodes.Length || matchCode < 0 || matchCode >= matchCodes.Length then
                        raise (InvalidDataException("invalid sequence code"))

                    let literalLength = literalCodes[literalCode].Baseline + reader.ReadBits literalCodes[literalCode].ExtraBits
                    let matchLength = matchCodes[matchCode].Baseline + reader.ReadBits matchCodes[matchCode].ExtraBits
                    let rawOffset = (1 <<< offsetCode) ||| reader.ReadBits offsetCode
                    let matchOffset = rawOffset - 3
                    if literalLength < 0 || literalPosition + literalLength > literals.Length then
                        raise (InvalidDataException("literal run exceeds the literals section"))

                    Zstd.EnsureOutputLimit(output.Count, literalLength)
                    for _ in 1..literalLength do
                        output.Add literals[literalPosition]
                        literalPosition <- literalPosition + 1

                    if matchOffset < 1 || matchOffset > output.Count then
                        raise (InvalidDataException("match offset exceeds decoded output"))
                    Zstd.EnsureOutputLimit(output.Count, matchLength)
                    let copyStart = output.Count - matchOffset
                    for index in 0..matchLength - 1 do
                        output.Add output[copyStart + index]

                let remaining = literals.Length - literalPosition
                Zstd.EnsureOutputLimit(output.Count, remaining)
                for index in literalPosition..literals.Length - 1 do output.Add literals[index]

    /// Compresses bytes into a deterministic educational Zstandard frame.
    static member Compress(data: byte array) =
        if isNull data then nullArg "data"
        use output = new MemoryStream()
        Zstd.WriteBytes(output, [| 0x28uy; 0xB5uy; 0x2Fuy; 0xFDuy |])
        output.WriteByte 0xE0uy
        let contentSize = Array.zeroCreate<byte> 8
        BinaryPrimitives.WriteUInt64LittleEndian(contentSize.AsSpan(), uint64 data.Length)
        Zstd.WriteBytes(output, contentSize)

        if data.Length = 0 then
            Zstd.WriteBlockHeader(output, 0, 0, true)
        else
            let mutable offset = 0
            while offset < data.Length do
                let length = min maxBlockSize (data.Length - offset)
                let block = data[offset..offset + length - 1]
                let last = offset + length = data.Length
                if Zstd.AllEqual block then
                    Zstd.WriteBlockHeader(output, length, 1, last)
                    output.WriteByte block[0]
                else
                    match Zstd.CompressBlock block with
                    | Some compressed ->
                        Zstd.WriteBlockHeader(output, compressed.Length, 2, last)
                        Zstd.WriteBytes(output, compressed)
                    | None ->
                        Zstd.WriteBlockHeader(output, length, 0, last)
                        Zstd.WriteBytes(output, block)
                offset <- offset + length
        output.ToArray()

    /// Decompresses one educational Zstandard frame.
    static member Decompress(data: byte array) =
        if isNull data then nullArg "data"
        if data.Length < 5 then raise (InvalidDataException("frame is too short"))
        if BinaryPrimitives.ReadUInt32LittleEndian(data.AsSpan(0, 4)) <> magic then
            raise (InvalidDataException("bad Zstandard magic number"))

        let mutable position = 4
        let descriptor = data[position]
        position <- position + 1
        if (descriptor &&& 0x0Cuy) <> 0uy then raise (InvalidDataException("reserved frame-header bits are set"))
        let contentSizeFlag = int (descriptor >>> 6)
        let singleSegment = (descriptor &&& 0x20uy) <> 0uy
        let checksum = (descriptor &&& 0x10uy) <> 0uy
        let dictionaryFlag = int (descriptor &&& 3uy)

        if not singleSegment then
            Zstd.RequireAvailable(data.Length, position, 1, "window descriptor")
            position <- position + 1
        let dictionaryBytes = match dictionaryFlag with 0 -> 0 | 1 -> 1 | 2 -> 2 | _ -> 4
        Zstd.RequireAvailable(data.Length, position, dictionaryBytes, "dictionary id")
        position <- position + dictionaryBytes
        let contentSizeBytes =
            match contentSizeFlag with
            | 0 -> if singleSegment then 1 else 0
            | 1 -> 2
            | 2 -> 4
            | _ -> 8
        Zstd.RequireAvailable(data.Length, position, contentSizeBytes, "frame content size")
        position <- position + contentSizeBytes

        let output = ResizeArray<byte>()
        let mutable last = false
        while not last do
            Zstd.RequireAvailable(data.Length, position, 3, "block header")
            let blockHeader = int data[position] ||| (int data[position + 1] <<< 8) ||| (int data[position + 2] <<< 16)
            position <- position + 3
            last <- (blockHeader &&& 1) <> 0
            let blockType = (blockHeader >>> 1) &&& 3
            let blockSize = blockHeader >>> 3
            if blockSize > maxBlockSize then raise (InvalidDataException("block exceeds the 128 KiB limit"))

            match blockType with
            | 0 ->
                Zstd.RequireAvailable(data.Length, position, blockSize, "raw block")
                Zstd.EnsureOutputLimit(output.Count, blockSize)
                output.AddRange data[position..position + blockSize - 1]
                position <- position + blockSize
            | 1 ->
                Zstd.RequireAvailable(data.Length, position, 1, "RLE block")
                Zstd.EnsureOutputLimit(output.Count, blockSize)
                let value = data[position]
                position <- position + 1
                for _ in 1..blockSize do output.Add value
            | 2 ->
                Zstd.RequireAvailable(data.Length, position, blockSize, "compressed block")
                Zstd.DecompressBlock(data[position..position + blockSize - 1], output)
                position <- position + blockSize
            | _ -> raise (InvalidDataException("reserved block type 3"))

        if checksum then
            Zstd.RequireAvailable(data.Length, position, 4, "content checksum")
            position <- position + 4
        if position <> data.Length then raise (InvalidDataException("trailing bytes after Zstandard frame"))
        output.ToArray()
