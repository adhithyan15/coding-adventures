namespace CodingAdventures.Lzw.FSharp

open System
open System.Buffers.Binary
open System.Collections.Generic

module private Constants =
    [<Literal>]
    let ClearCode = 256

    [<Literal>]
    let StopCode = 257

    [<Literal>]
    let InitialNextCode = 258

    [<Literal>]
    let InitialCodeSize = 9

    [<Literal>]
    let MaxCodeSize = 16

    let MaxEntries = 1 <<< MaxCodeSize

type BitWriter() =
    let bytes = ResizeArray<byte>()
    let mutable buffer = 0UL
    let mutable bitCount = 0

    member _.Write(code: int, codeSize: int) =
        if codeSize <= 0 || codeSize > Constants.MaxCodeSize then
            invalidArg "codeSize" "codeSize must be between 1 and MAX_CODE_SIZE"

        buffer <- buffer ||| ((uint64 code) <<< bitCount)
        bitCount <- bitCount + codeSize

        while bitCount >= 8 do
            bytes.Add(byte (buffer &&& 0xFFUL))
            buffer <- buffer >>> 8
            bitCount <- bitCount - 8

    member _.Flush() =
        if bitCount > 0 then
            bytes.Add(byte (buffer &&& 0xFFUL))
            buffer <- 0UL
            bitCount <- 0

    member _.ToArray() = bytes.ToArray()

type BitReader(data: byte array) =
    do
        if isNull data then nullArg "data"

    let mutable position = 0
    let mutable buffer = 0UL
    let mutable bitCount = 0

    member _.Read(codeSize: int) =
        if codeSize <= 0 || codeSize > Constants.MaxCodeSize then
            invalidArg "codeSize" "codeSize must be between 1 and MAX_CODE_SIZE"

        while bitCount < codeSize do
            if position >= data.Length then
                invalidOp "unexpected end of bit stream"

            buffer <- buffer ||| ((uint64 data[position]) <<< bitCount)
            position <- position + 1
            bitCount <- bitCount + 8

        let mask = (1UL <<< codeSize) - 1UL
        let code = int (buffer &&& mask)
        buffer <- buffer >>> codeSize
        bitCount <- bitCount - codeSize
        code

    member _.Exhausted = position >= data.Length && bitCount = 0

[<AbstractClass; Sealed>]
type Lzw =
    static member CLEAR_CODE = Constants.ClearCode
    static member STOP_CODE = Constants.StopCode
    static member INITIAL_NEXT_CODE = Constants.InitialNextCode
    static member INITIAL_CODE_SIZE = Constants.InitialCodeSize
    static member MAX_CODE_SIZE = Constants.MaxCodeSize

    static member private SingleByteString(value: byte) = string (char value)

    static member private CreateEncoderDictionary() =
        let dictionary = Dictionary<string, int>(512)
        for value in 0 .. 255 do
            dictionary[Lzw.SingleByteString(byte value)] <- value

        dictionary

    static member private ResetDecoderDictionary(dictionary: ResizeArray<byte array>) =
        dictionary.Clear()
        for value in 0 .. 255 do
            dictionary.Add([| byte value |])

        dictionary.Add(Array.empty<byte>)
        dictionary.Add(Array.empty<byte>)

    static member private CreateDecoderDictionary() =
        let dictionary = ResizeArray<byte array>(258)
        Lzw.ResetDecoderDictionary(dictionary)
        dictionary

    static member private AppendByte(prefix: byte array, value: byte) =
        let output = Array.zeroCreate<byte> (prefix.Length + 1)
        Array.Copy(prefix, output, prefix.Length)
        output[output.Length - 1] <- value
        output

    static member EncodeCodes(data: byte array) =
        if isNull data then
            nullArg "data"

        let mutable dictionary = Lzw.CreateEncoderDictionary()
        let codes = ResizeArray<int>()
        codes.Add(Constants.ClearCode)
        let mutable nextCode = Constants.InitialNextCode
        let mutable current = String.Empty

        for value in data do
            let candidate = current + Lzw.SingleByteString(value)
            match dictionary.TryGetValue(candidate) with
            | true, _ ->
                current <- candidate
            | _ ->
                codes.Add(dictionary[current])

                if nextCode < Constants.MaxEntries then
                    dictionary[candidate] <- nextCode
                    nextCode <- nextCode + 1
                else
                    codes.Add(Constants.ClearCode)
                    dictionary <- Lzw.CreateEncoderDictionary()
                    nextCode <- Constants.InitialNextCode

                current <- Lzw.SingleByteString(value)

        if current.Length > 0 then
            codes.Add(dictionary[current])

        codes.Add(Constants.StopCode)
        List.ofSeq codes, data.Length

    static member DecodeCodes(codes: seq<int>, ?originalLength: int) =
        if isNull (box codes) then
            nullArg "codes"

        let originalLength = defaultArg originalLength -1
        let dictionary = Lzw.CreateDecoderDictionary()
        let output = ResizeArray<byte>()
        let mutable nextCode = Constants.InitialNextCode
        let mutable previousCode: int option = None
        let mutable stop = false

        for code in codes do
            if not stop then
                if code = Constants.ClearCode then
                    Lzw.ResetDecoderDictionary(dictionary)
                    nextCode <- Constants.InitialNextCode
                    previousCode <- None
                elif code = Constants.StopCode then
                    stop <- true
                else
                    let entry =
                        if code >= 0 && code < dictionary.Count then
                            dictionary[code]
                        elif code = nextCode then
                            match previousCode with
                            | Some previous ->
                                let previousEntry = dictionary[previous]
                                Lzw.AppendByte(previousEntry, previousEntry[0])
                            | None -> invalidOp "invalid LZW code"
                        else
                            invalidOp "invalid LZW code"

                    output.AddRange(entry)

                    match previousCode with
                    | Some previous when nextCode < Constants.MaxEntries ->
                        let previousEntry = dictionary[previous]
                        dictionary.Add(Lzw.AppendByte(previousEntry, entry[0]))
                        nextCode <- nextCode + 1
                    | _ -> ()

                    previousCode <- Some code

        let finalOutput = output.ToArray()
        if originalLength >= 0 then
            finalOutput |> Seq.truncate originalLength |> Array.ofSeq
        else
            finalOutput

    static member PackCodes(codes: seq<int>, originalLength: int) =
        if isNull (box codes) then
            nullArg "codes"
        if originalLength < 0 then
            invalidArg "originalLength" "originalLength must be non-negative"

        let writer = BitWriter()
        let mutable nextCode = Constants.InitialNextCode
        let mutable codeSize = Constants.InitialCodeSize

        for code in codes do
            if code < 0 || code >= Constants.MaxEntries then
                invalidOp "Code does not fit in the CMP03 code space"

            writer.Write(code, codeSize)

            if code = Constants.ClearCode then
                nextCode <- Constants.InitialNextCode
                codeSize <- Constants.InitialCodeSize
            elif code <> Constants.StopCode && nextCode < Constants.MaxEntries then
                nextCode <- nextCode + 1
                if nextCode > (1 <<< codeSize) && codeSize < Constants.MaxCodeSize then
                    codeSize <- codeSize + 1

        writer.Flush()
        let payload = writer.ToArray()
        let output = Array.zeroCreate<byte> (4 + payload.Length)
        BinaryPrimitives.WriteUInt32BigEndian(output.AsSpan(0, 4), uint32 originalLength)
        Array.Copy(payload, 0, output, 4, payload.Length)
        output

    static member UnpackCodes(data: byte array) =
        if isNull data then
            nullArg "data"
        if data.Length < 4 then
            [], 0
        else
            let originalLength = int (BinaryPrimitives.ReadUInt32BigEndian(data.AsSpan(0, 4)))
            let reader = BitReader(data[4..])
            let codes = ResizeArray<int>()
            let mutable nextCode = Constants.InitialNextCode
            let mutable codeSize = Constants.InitialCodeSize
            let mutable keepReading = true

            while keepReading do
                try
                    let code = reader.Read(codeSize)
                    codes.Add(code)

                    if code = Constants.StopCode then
                        keepReading <- false
                    elif code = Constants.ClearCode then
                        nextCode <- Constants.InitialNextCode
                        codeSize <- Constants.InitialCodeSize
                    elif nextCode < Constants.MaxEntries then
                        nextCode <- nextCode + 1
                        if nextCode > (1 <<< codeSize) && codeSize < Constants.MaxCodeSize then
                            codeSize <- codeSize + 1
                with :? InvalidOperationException ->
                    keepReading <- false

            List.ofSeq codes, originalLength

    static member Compress(data: byte array) =
        if isNull data then
            nullArg "data"
        let codes, originalLength = Lzw.EncodeCodes(data)
        Lzw.PackCodes(codes, originalLength)

    static member Decompress(data: byte array) =
        if isNull data then
            nullArg "data"
        let codes, originalLength = Lzw.UnpackCodes(data)
        Lzw.DecodeCodes(codes, originalLength)
