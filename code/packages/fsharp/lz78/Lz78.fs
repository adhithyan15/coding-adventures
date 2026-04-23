namespace CodingAdventures.Lz78.FSharp

open System
open System.Buffers.Binary
open System.Collections.Generic

type Lz78Token =
    { DictIndex: int
      NextChar: byte }

[<AbstractClass; Sealed>]
type Lz78 =
    static member Token(dictIndex: int, nextChar: byte) =
        { DictIndex = dictIndex
          NextChar = nextChar }

    static member Encode(data: byte array, ?maxDictSize: int) =
        if isNull data then nullArg "data"

        let maxDictSize = defaultArg maxDictSize 65536
        if maxDictSize <= 0 then
            invalidArg "maxDictSize" "maxDictSize must be positive"

        let dictionary = Dictionary<(int * byte), int>()
        let tokens = ResizeArray<Lz78Token>()
        let mutable nextId = 1
        let mutable currentId = 0

        for value in data do
            match dictionary.TryGetValue((currentId, value)) with
            | true, childId -> currentId <- childId
            | _ ->
                tokens.Add(Lz78.Token(currentId, value))
                if nextId < maxDictSize then
                    dictionary[(currentId, value)] <- nextId
                    nextId <- nextId + 1

                currentId <- 0

        if currentId <> 0 then
            tokens.Add(Lz78.Token(currentId, 0uy))

        List.ofSeq tokens

    static member private Reconstruct(table: IReadOnlyList<int * byte>, index: int) =
        if index = 0 then
            []
        else
            let reversed = ResizeArray<byte>()
            let mutable current = index
            while current <> 0 do
                let parentId, value = table[current]
                reversed.Add value
                current <- parentId

            reversed |> Seq.rev |> Array.ofSeq |> List.ofArray

    static member Decode(tokens: seq<Lz78Token>, ?originalLength: int) =
        if isNull (box tokens) then nullArg "tokens"

        let originalLength = defaultArg originalLength -1
        let table = ResizeArray<int * byte>()
        table.Add(0, 0uy)
        let output = ResizeArray<byte>()

        for token in tokens do
            if token.DictIndex < 0 || token.DictIndex >= table.Count then
                invalidOp "Token references a dictionary entry that does not exist"

            output.AddRange(Lz78.Reconstruct(table, token.DictIndex))
            if originalLength < 0 || output.Count < originalLength then
                output.Add token.NextChar

            table.Add(token.DictIndex, token.NextChar)

        if originalLength >= 0 then
            output |> Seq.truncate originalLength |> Array.ofSeq
        else
            output.ToArray()

    static member SerialiseTokens(tokens: IReadOnlyList<Lz78Token>, originalLength: int) =
        if isNull (box tokens) then nullArg "tokens"
        if originalLength < 0 then invalidArg "originalLength" "originalLength must be non-negative"

        let output = Array.zeroCreate<byte> (8 + (tokens.Count * 4))
        BinaryPrimitives.WriteUInt32BigEndian(output.AsSpan(0, 4), uint32 originalLength)
        BinaryPrimitives.WriteUInt32BigEndian(output.AsSpan(4, 4), uint32 tokens.Count)

        for index in 0 .. tokens.Count - 1 do
            let token = tokens[index]
            if token.DictIndex < 0 || token.DictIndex > int UInt16.MaxValue then
                invalidOp "Dictionary index does not fit in the teaching uint16 token format"

            let start = 8 + (index * 4)
            BinaryPrimitives.WriteUInt16BigEndian(output.AsSpan(start, 2), uint16 token.DictIndex)
            output[start + 2] <- token.NextChar
            output[start + 3] <- 0uy

        output

    static member DeserialiseTokens(data: byte array) =
        if isNull data then nullArg "data"
        if data.Length < 8 then
            [], 0
        else
            let originalLength = int (BinaryPrimitives.ReadUInt32BigEndian(data.AsSpan(0, 4)))
            let mutable tokenCount = BinaryPrimitives.ReadUInt32BigEndian(data.AsSpan(4, 4))
            let maxPossibleTokens = uint32 ((data.Length - 8) / 4)
            if tokenCount > maxPossibleTokens then
                tokenCount <- maxPossibleTokens
            let tokens = ResizeArray<Lz78Token>()

            if tokenCount > 0u then
                for index in 0u .. tokenCount - 1u do
                    let start = 8 + (int index * 4)
                    tokens.Add(
                        { DictIndex = int (BinaryPrimitives.ReadUInt16BigEndian(data.AsSpan(start, 2)))
                          NextChar = data[start + 2] })

            List.ofSeq tokens, originalLength

    static member Compress(data: byte array, ?maxDictSize: int) =
        if isNull data then nullArg "data"
        let tokens = Lz78.Encode(data, ?maxDictSize = maxDictSize)
        Lz78.SerialiseTokens(tokens, data.Length)

    static member Decompress(data: byte array) =
        if isNull data then nullArg "data"
        let tokens, originalLength = Lz78.DeserialiseTokens(data)
        Lz78.Decode(tokens, originalLength)
