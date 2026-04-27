namespace CodingAdventures.Lz77.FSharp

open System
open System.Buffers.Binary
open System.Collections.Generic

type Lz77Token =
    { Offset: int
      Length: int
      NextChar: byte }

[<AbstractClass; Sealed>]
type Lz77 =
    static member Token(offset: int, length: int, nextChar: byte) =
        { Offset = offset
          Length = length
          NextChar = nextChar }

    static member private ValidateParameters(windowSize: int, maxMatch: int, minMatch: int) =
        if windowSize <= 0 then
            invalidArg "windowSize" "windowSize must be positive"

        if maxMatch <= 0 then
            invalidArg "maxMatch" "maxMatch must be positive"

        if minMatch <= 0 then
            invalidArg "minMatch" "minMatch must be positive"

    static member private FindLongestMatch(data: byte array, cursor: int, windowSize: int, maxMatch: int) =
        let mutable bestOffset = 0
        let mutable bestLength = 0
        let searchStart = max 0 (cursor - windowSize)
        let lookaheadEnd = min (cursor + maxMatch) (data.Length - 1)

        for position in searchStart .. cursor - 1 do
            let mutable length = 0
            while cursor + length < lookaheadEnd && data[position + length] = data[cursor + length] do
                length <- length + 1

            if length > bestLength then
                bestLength <- length
                bestOffset <- cursor - position

        bestOffset, bestLength

    static member Encode(data: byte array, ?windowSize: int, ?maxMatch: int, ?minMatch: int) =
        if isNull data then nullArg "data"

        let windowSize = defaultArg windowSize 4096
        let maxMatch = defaultArg maxMatch 255
        let minMatch = defaultArg minMatch 3
        Lz77.ValidateParameters(windowSize, maxMatch, minMatch)

        let tokens = ResizeArray<Lz77Token>()
        let mutable cursor = 0

        while cursor < data.Length do
            if cursor = data.Length - 1 then
                tokens.Add(Lz77.Token(0, 0, data[cursor]))
                cursor <- cursor + 1
            else
                let offset, length = Lz77.FindLongestMatch(data, cursor, windowSize, maxMatch)
                if length >= minMatch then
                    tokens.Add(Lz77.Token(offset, length, data[cursor + length]))
                    cursor <- cursor + length + 1
                else
                    tokens.Add(Lz77.Token(0, 0, data[cursor]))
                    cursor <- cursor + 1

        List.ofSeq tokens

    static member Decode(tokens: seq<Lz77Token>, ?initialBuffer: byte array) =
        if isNull (box tokens) then nullArg "tokens"

        let output = ResizeArray<byte>(defaultArg initialBuffer [||])
        for token in tokens do
            if token.Length > 0 then
                if token.Offset <= 0 then
                    invalidOp "Backreference offsets must be positive"

                let start = output.Count - token.Offset
                if start < 0 then
                    invalidOp "Backreference offset extends before the output buffer"

                for index in 0 .. token.Length - 1 do
                    output.Add(output[start + index])

            output.Add token.NextChar

        output.ToArray()

    static member SerialiseTokens(tokens: IReadOnlyList<Lz77Token>) =
        if isNull (box tokens) then nullArg "tokens"

        let output = Array.zeroCreate<byte> (4 + (tokens.Count * 4))
        BinaryPrimitives.WriteUInt32BigEndian(output.AsSpan(0, 4), uint32 tokens.Count)

        for index in 0 .. tokens.Count - 1 do
            let token = tokens[index]
            if token.Offset < 0 || token.Offset > int UInt16.MaxValue then
                invalidOp "Offset does not fit in the teaching uint16 token format"

            if token.Length < 0 || token.Length > int Byte.MaxValue then
                invalidOp "Length does not fit in the teaching uint8 token format"

            let start = 4 + (index * 4)
            BinaryPrimitives.WriteUInt16BigEndian(output.AsSpan(start, 2), uint16 token.Offset)
            output[start + 2] <- byte token.Length
            output[start + 3] <- token.NextChar

        output

    static member DeserialiseTokens(data: byte array) =
        if isNull data then nullArg "data"
        if data.Length < 4 then
            []
        else
            let count = BinaryPrimitives.ReadUInt32BigEndian(data.AsSpan(0, 4))
            if count = 0u then
                []
            else
                let tokens = ResizeArray<Lz77Token>()
                for index in 0u .. count - 1u do
                    let start = 4 + (int index * 4)
                    if start + 4 <= data.Length then
                        tokens.Add(
                            { Offset = int (BinaryPrimitives.ReadUInt16BigEndian(data.AsSpan(start, 2)))
                              Length = int data[start + 2]
                              NextChar = data[start + 3] })

                List.ofSeq tokens

    static member Compress(data: byte array, ?windowSize: int, ?maxMatch: int, ?minMatch: int) =
        let tokens = Lz77.Encode(data, ?windowSize = windowSize, ?maxMatch = maxMatch, ?minMatch = minMatch)
        Lz77.SerialiseTokens(tokens)

    static member Decompress(data: byte array) =
        Lz77.Decode(Lz77.DeserialiseTokens(data))
