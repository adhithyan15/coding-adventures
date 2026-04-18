namespace CodingAdventures.Lzss.FSharp

open System
open System.Buffers.Binary
open System.Collections.Generic

type LzssToken =
    | Literal of byte: byte
    | Match of offset: int * length: int

[<AbstractClass; Sealed>]
type Lzss =
    static member DefaultWindowSize = 4096
    static member DefaultMaxMatch = 255
    static member DefaultMinMatch = 3

    static member Literal(value: byte) = Literal value
    static member Match(offset: int, length: int) = Match(offset, length)

    static member private ValidateParameters(windowSize: int, maxMatch: int, minMatch: int) =
        if windowSize <= 0 then invalidArg "windowSize" "windowSize must be positive"
        if maxMatch <= 0 then invalidArg "maxMatch" "maxMatch must be positive"
        if minMatch <= 0 then invalidArg "minMatch" "minMatch must be positive"

    static member private FindLongestMatch(data: byte array, cursor: int, windowStart: int, maxMatch: int) =
        let mutable bestLength = 0
        let mutable bestOffset = 0
        let lookaheadEnd = min (cursor + maxMatch) data.Length

        for position in windowStart .. cursor - 1 do
            let mutable length = 0
            while cursor + length < lookaheadEnd && data[position + length] = data[cursor + length] do
                length <- length + 1

            if length > bestLength then
                bestLength <- length
                bestOffset <- cursor - position

        bestOffset, bestLength

    static member Encode(data: byte array, ?windowSize: int, ?maxMatch: int, ?minMatch: int) =
        if isNull data then nullArg "data"

        let windowSize = defaultArg windowSize Lzss.DefaultWindowSize
        let maxMatch = defaultArg maxMatch Lzss.DefaultMaxMatch
        let minMatch = defaultArg minMatch Lzss.DefaultMinMatch
        Lzss.ValidateParameters(windowSize, maxMatch, minMatch)

        let tokens = ResizeArray<LzssToken>()
        let mutable cursor = 0

        while cursor < data.Length do
            let windowStart = max 0 (cursor - windowSize)
            let offset, length = Lzss.FindLongestMatch(data, cursor, windowStart, maxMatch)
            if length >= minMatch then
                tokens.Add(Match(offset, length))
                cursor <- cursor + length
            else
                tokens.Add(Literal data[cursor])
                cursor <- cursor + 1

        List.ofSeq tokens

    static member Decode(tokens: seq<LzssToken>, ?originalLength: int) =
        if isNull (box tokens) then nullArg "tokens"

        let originalLength = defaultArg originalLength -1
        let output = ResizeArray<byte>()

        for token in tokens do
            match token with
            | Literal value -> output.Add value
            | Match(offset, length) ->
                if offset <= 0 then
                    invalidOp "Match offsets must be positive"

                let start = output.Count - offset
                if start < 0 then
                    invalidOp "Match offset extends before the output buffer"

                for index in 0 .. length - 1 do
                    output.Add(output[start + index])

        if originalLength >= 0 then
            output |> Seq.truncate originalLength |> Array.ofSeq
        else
            output.ToArray()

    static member SerialiseTokens(tokens: IReadOnlyList<LzssToken>, originalLength: int) =
        if isNull (box tokens) then nullArg "tokens"
        if originalLength < 0 then invalidArg "originalLength" "originalLength must be non-negative"

        let blocks = ResizeArray<byte array>()
        let mutable tokenIndex = 0
        while tokenIndex < tokens.Count do
            let chunk = tokens |> Seq.skip tokenIndex |> Seq.take (min 8 (tokens.Count - tokenIndex)) |> Seq.toList
            let bytes = ResizeArray<byte>()
            let mutable flag = 0uy

            for bit = 0 to chunk.Length - 1 do
                match chunk[bit] with
                | Match(offset, length) ->
                    flag <- flag ||| byte (1 <<< bit)
                    bytes.Add(byte ((offset >>> 8) &&& 0xff))
                    bytes.Add(byte (offset &&& 0xff))
                    bytes.Add(byte (length &&& 0xff))
                | Literal value ->
                    bytes.Add value

            blocks.Add(Array.append [| flag |] (bytes.ToArray()))
            tokenIndex <- tokenIndex + 8

        let totalSize = 8 + (blocks |> Seq.sumBy _.Length)
        let output = Array.zeroCreate<byte> totalSize
        BinaryPrimitives.WriteUInt32BigEndian(output.AsSpan(0, 4), uint32 originalLength)
        BinaryPrimitives.WriteUInt32BigEndian(output.AsSpan(4, 4), uint32 blocks.Count)

        let mutable position = 8
        for block in blocks do
            block.CopyTo(output, position)
            position <- position + block.Length

        output

    static member DeserialiseTokens(data: byte array) =
        if isNull data then nullArg "data"
        if data.Length < 8 then
            [], 0
        else
            let originalLength = int (BinaryPrimitives.ReadUInt32BigEndian(data.AsSpan(0, 4)))
            let mutable blockCount = BinaryPrimitives.ReadUInt32BigEndian(data.AsSpan(4, 4))
            let maxPossibleBlocks = uint32 (data.Length - 8)
            if blockCount > maxPossibleBlocks then
                blockCount <- maxPossibleBlocks

            let tokens = ResizeArray<LzssToken>()
            let mutable position = 8

            if blockCount > 0u then
                for _ in 0u .. blockCount - 1u do
                    if position < data.Length then
                        let flag = data[position]
                        position <- position + 1
                        for bit = 0 to 7 do
                            if position < data.Length then
                                if (flag &&& byte (1 <<< bit)) <> 0uy then
                                    if position + 3 <= data.Length then
                                        tokens.Add(Match(((int data[position]) <<< 8) ||| int data[position + 1], int data[position + 2]))
                                        position <- position + 3
                                else
                                    tokens.Add(Literal data[position])
                                    position <- position + 1

            List.ofSeq tokens, originalLength

    static member Compress(data: byte array, ?windowSize: int, ?maxMatch: int, ?minMatch: int) =
        if isNull data then nullArg "data"
        let tokens = Lzss.Encode(data, ?windowSize = windowSize, ?maxMatch = maxMatch, ?minMatch = minMatch)
        Lzss.SerialiseTokens(tokens, data.Length)

    static member Decompress(data: byte array) =
        if isNull data then nullArg "data"
        let tokens, originalLength = Lzss.DeserialiseTokens(data)
        Lzss.Decode(tokens, originalLength)
