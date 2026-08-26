namespace CodingAdventures.CanonicalCbor.FSharp

open System
open System.IO
open System.Text
open System.Collections.Generic

module private ErrorMessages =
    let messageFor id =
        match id with
        | "unexpected-eof" -> "canonical-cbor: unexpected end of input"
        | "trailing-bytes" -> "canonical-cbor: trailing bytes after decoded item"
        | "reserved" -> "canonical-cbor: reserved additional-info value"
        | "indefinite" -> "canonical-cbor: indefinite item rejected"
        | "non-minimal-integer" -> "canonical-cbor: argument is not in smallest form"
        | "invalid-utf8" -> "canonical-cbor: text is not valid UTF-8"
        | "non-canonical-map-order" -> "canonical-cbor: map key order is not canonical"
        | "unsupported-simple" -> "canonical-cbor: unsupported simple value"
        | "float-not-supported" -> "canonical-cbor: floats are not supported"
        | "too-deep" -> "canonical-cbor: decoded nesting is too deep"
        | "length-too-large" -> "canonical-cbor: declared length is too large"
        | "duplicate-map-key" -> "canonical-cbor: duplicate canonical map key"
        | "encode-too-deep" -> "canonical-cbor: encoded nesting is too deep"
        | "encode-too-large" -> "canonical-cbor: encoded item is too large"
        | _ -> invalidArg (nameof id) "unknown canonical CBOR error identifier"

/// A stable, payload-blind CBR01 conformance error.
type CborException internal (id: string) =
    inherit Exception(ErrorMessages.messageFor id)
    member _.Id = id

/// The deliberately small value algebra supported by CBR01.
[<StructuralEquality; NoComparison>]
type CborValue =
    private
    | Unsigned of uint64
    | Negative of uint64
    | Bytes of byte array
    | Text of string
    | Array of CborValue list
    | Map of (CborValue * CborValue) list
    | Tag of uint64 * CborValue
    | Bool of bool
    | Null

/// Safe constructors for the closed CBR01 value algebra.
module CborValue =
    let unsigned value = Unsigned value
    let negative value = Negative value

    let bytes (value: byte array) =
        ArgumentNullException.ThrowIfNull(value)
        Bytes(Array.copy value)

    let tryBytes (value: CborValue) =
        match value with
        | Bytes payload -> Some(Array.copy payload)
        | _ -> None

    let tryUnsigned (value: CborValue) =
        match value with
        | Unsigned integer -> Some integer
        | _ -> None

    let tryNegative (value: CborValue) =
        match value with
        | Negative integer -> Some integer
        | _ -> None

    let tryText (value: CborValue) =
        match value with
        | Text text -> Some text
        | _ -> None

    let tryArray (value: CborValue) =
        match value with
        | Array values -> Some values
        | _ -> None

    let tryMap (value: CborValue) =
        match value with
        | Map entries -> Some entries
        | _ -> None

    let tryTag (value: CborValue) =
        match value with
        | Tag(number, tagged) -> Some(number, tagged)
        | _ -> None

    let tryBoolean (value: CborValue) =
        match value with
        | Bool boolean -> Some boolean
        | _ -> None

    let isNull (value: CborValue) =
        match value with
        | Null -> true
        | _ -> false

    let private validateScalarText (value: string) =
        let mutable index = 0
        while index < value.Length do
            let unit = value[index]
            if Char.IsHighSurrogate(unit) then
                if index + 1 >= value.Length || not (Char.IsLowSurrogate(value[index + 1])) then
                    invalidArg (nameof value) "canonical-cbor: text is not Unicode scalar data"
                index <- index + 2
            elif Char.IsLowSurrogate(unit) then
                invalidArg (nameof value) "canonical-cbor: text is not Unicode scalar data"
            else
                index <- index + 1

    let text (value: string) =
        ArgumentNullException.ThrowIfNull(value)
        validateScalarText value
        Text value

    let array (values: CborValue seq) =
        if Object.ReferenceEquals(box values, null) then nullArg (nameof values)
        let copied = List.ofSeq values
        if copied |> List.exists (fun value -> Object.ReferenceEquals(box value, null)) then
            invalidArg (nameof values) "CBOR arrays cannot contain null references"
        Array copied

    let map (entries: (CborValue * CborValue) seq) =
        entries
        |> Seq.map (fun (key, value) ->
            if Object.ReferenceEquals(box key, null) || Object.ReferenceEquals(box value, null) then
                invalidArg (nameof entries) "CBOR maps cannot contain null references"
            key, value)
        |> List.ofSeq
        |> Map

    let tag (number: uint64) (value: CborValue) =
        if Object.ReferenceEquals(box value, null) then nullArg (nameof value)
        Tag(number, value)

    let boolean (value: bool) = Bool value
    let nullValue = Null

/// A bounded, zero-production-dependency RFC 8949 section 4.2.3 codec.
module CanonicalCbor =
    [<Literal>]
    let MaxNestingDepth = 128

    [<Literal>]
    let MaxEncodedBytes = 1_048_576

    let private strictUtf8 = UTF8Encoding(false, true)
    let private error id = CborException(id)

    let private argumentSize argument =
        if argument <= 23UL then 1L
        elif argument <= uint64 Byte.MaxValue then 2L
        elif argument <= uint64 UInt16.MaxValue then 3L
        elif argument <= uint64 UInt32.MaxValue then 5L
        else 9L

    let private compareUnsigned (left: byte array) (right: byte array) =
        let mutable result = 0
        let mutable index = 0
        let common = min left.Length right.Length
        while result = 0 && index < common do
            result <- compare left[index] right[index]
            index <- index + 1
        if result <> 0 then result else compare left.Length right.Length

    let private compareLengthFirst (left: byte array) (right: byte array) =
        let length = compare left.Length right.Length
        if length <> 0 then length else compareUnsigned left right

    type private EncodedEntry = { Key: byte array; Value: CborValue }

    type private Encoder() =
        let output = new MemoryStream()

        member _.Bytes() = output.ToArray()

        member private _.EnsureFits(additionalBytes: int64) =
            if additionalBytes > int64 MaxEncodedBytes - output.Length then
                raise (error "encode-too-large")

        member private _.WriteByte(value: int) =
            if output.Length >= int64 MaxEncodedBytes then
                raise (error "encode-too-large")
            output.WriteByte(byte value)

        member private _.WriteBytes(value: byte array) =
            if int64 value.Length > int64 MaxEncodedBytes - output.Length then
                raise (error "encode-too-large")
            output.Write(value, 0, value.Length)

        member private this.WriteArgument(major: int, argument: uint64) =
            let prefix = major <<< 5
            if argument <= 23UL then
                this.WriteByte(prefix ||| int argument)
            elif argument <= uint64 Byte.MaxValue then
                this.WriteByte(prefix ||| 24)
                this.WriteByte(int argument)
            elif argument <= uint64 UInt16.MaxValue then
                this.WriteByte(prefix ||| 25)
                this.WriteByte(int (argument >>> 8))
                this.WriteByte(int argument)
            elif argument <= uint64 UInt32.MaxValue then
                this.WriteByte(prefix ||| 26)
                for shift in [ 24; 16; 8; 0 ] do
                    this.WriteByte(int (argument >>> shift))
            else
                this.WriteByte(prefix ||| 27)
                for shift in [ 56; 48; 40; 32; 24; 16; 8; 0 ] do
                    this.WriteByte(int (argument >>> shift))

        member this.WriteValue(value: CborValue, depth: int) =
            if depth > MaxNestingDepth then
                raise (error "encode-too-deep")

            match value with
            | Unsigned integer -> this.WriteArgument(0, integer)
            | Negative integer -> this.WriteArgument(1, integer)
            | Bytes payload ->
                this.WriteArgument(2, uint64 payload.Length)
                this.WriteBytes(payload)
            | Text text ->
                let length = strictUtf8.GetByteCount(text)
                this.EnsureFits(argumentSize (uint64 length) + int64 length)
                let payload = strictUtf8.GetBytes(text)
                this.WriteArgument(3, uint64 length)
                this.WriteBytes(payload)
            | Array values ->
                this.EnsureFits(argumentSize (uint64 values.Length) + int64 values.Length)
                this.WriteArgument(4, uint64 values.Length)
                for item in values do
                    this.WriteValue(item, depth + 1)
            | Map entries -> this.WriteMap(entries, depth)
            | Tag(number, tagged) ->
                this.WriteArgument(6, number)
                this.WriteValue(tagged, depth + 1)
            | Bool boolean -> this.WriteByte(if boolean then 0xf5 else 0xf4)
            | Null -> this.WriteByte(0xf6)

        member private this.WriteMap(entries: (CborValue * CborValue) list, depth: int) =
            let count = entries.Length
            this.EnsureFits(argumentSize (uint64 count) + int64 count * 2L)
            let encoded = ResizeArray<EncodedEntry>(count)
            let mutable retainedKeyBytes = 0L
            for key, value in entries do
                let keyEncoder = Encoder()
                keyEncoder.WriteValue(key, depth + 1)
                let keyBytes = keyEncoder.Bytes()
                retainedKeyBytes <- retainedKeyBytes + int64 keyBytes.Length
                this.EnsureFits(argumentSize (uint64 count) + int64 count + retainedKeyBytes)
                encoded.Add({ Key = keyBytes; Value = value })

            encoded.Sort(Comparison<EncodedEntry>(fun left right -> compareLengthFirst left.Key right.Key))
            for index = 1 to encoded.Count - 1 do
                if encoded[index - 1].Key.AsSpan().SequenceEqual(encoded[index].Key) then
                    raise (error "duplicate-map-key")

            this.WriteArgument(5, uint64 encoded.Count)
            for entry in encoded do
                this.WriteBytes(entry.Key)
                this.WriteValue(entry.Value, depth + 1)

    type private Header = { Major: int; Info: int; Argument: uint64 }

    type private Cursor(input: byte array) =
        let bytes = Array.copy input
        let mutable position = 0

        member _.Remaining = bytes.Length - position

        member private _.ReadByte() =
            if position >= bytes.Length then
                raise (error "unexpected-eof")
            let value = int bytes[position]
            position <- position + 1
            value

        member private this.ReadBytes(length: int) =
            if length > this.Remaining then
                raise (error "unexpected-eof")
            let result = bytes.AsSpan(position, length).ToArray()
            position <- position + length
            result

        member private this.ReadUnsigned(width: int) =
            let mutable value = 0UL
            for _ = 1 to width do
                value <- (value <<< 8) ||| uint64 (this.ReadByte())
            value

        member private this.ReadHeader() =
            let initial = this.ReadByte()
            let major = initial >>> 5
            let info = initial &&& 0x1f
            let enforceMinimal = major <> 7
            let argument =
                if info <= 23 then uint64 info
                elif info = 24 then
                    let value = uint64 (this.ReadByte())
                    if enforceMinimal && value <= 23UL then raise (error "non-minimal-integer")
                    value
                elif info = 25 then
                    let value = this.ReadUnsigned(2)
                    if enforceMinimal && value <= uint64 Byte.MaxValue then raise (error "non-minimal-integer")
                    value
                elif info = 26 then
                    let value = this.ReadUnsigned(4)
                    if enforceMinimal && value <= uint64 UInt16.MaxValue then raise (error "non-minimal-integer")
                    value
                elif info = 27 then
                    let value = this.ReadUnsigned(8)
                    if enforceMinimal && value <= uint64 UInt32.MaxValue then raise (error "non-minimal-integer")
                    value
                elif info <= 30 then raise (error "reserved")
                else raise (error "indefinite")
            { Major = major; Info = info; Argument = argument }

        member private this.CheckedLength(declared: uint64, minimumBytesPerUnit: int) =
            let maximum = this.Remaining / minimumBytesPerUnit
            if declared > uint64 maximum then
                raise (error "length-too-large")
            int declared

        member private this.ReadText(length: int) =
            let payload = this.ReadBytes(length)
            try
                strictUtf8.GetString(payload)
            with :? DecoderFallbackException ->
                raise (error "invalid-utf8")

        member this.ReadValue(depth: int) =
            if depth > MaxNestingDepth then
                raise (error "too-deep")

            let header = this.ReadHeader()
            match header.Major with
            | 0 -> Unsigned header.Argument
            | 1 -> Negative header.Argument
            | 2 -> Bytes(this.ReadBytes(this.CheckedLength(header.Argument, 1)))
            | 3 -> Text(this.ReadText(this.CheckedLength(header.Argument, 1)))
            | 4 ->
                let count = this.CheckedLength(header.Argument, 1)
                List.init count (fun _ -> this.ReadValue(depth + 1)) |> Array
            | 5 -> this.ReadMap(this.CheckedLength(header.Argument, 2), depth)
            | 6 -> Tag(header.Argument, this.ReadValue(depth + 1))
            | 7 ->
                match header.Info with
                | 20 -> Bool false
                | 21 -> Bool true
                | 22 -> Null
                | 25 | 26 | 27 -> raise (error "float-not-supported")
                | _ -> raise (error "unsupported-simple")
            | _ -> invalidOp "three-bit major type escaped range"

        member private this.ReadMap(count: int, depth: int) =
            let entries = ResizeArray<CborValue * CborValue>(count)
            let mutable previousKey: byte array option = None
            for _ = 1 to count do
                let keyStart = position
                let key = this.ReadValue(depth + 1)
                let encodedKey = bytes.AsSpan(keyStart, position - keyStart).ToArray()
                match previousKey with
                | Some previous when compareLengthFirst previous encodedKey >= 0 ->
                    raise (error "non-canonical-map-order")
                | _ -> ()
                previousKey <- Some encodedKey
                entries.Add(key, this.ReadValue(depth + 1))
            Map(List.ofSeq entries)

    /// Encode one value without publishing partial bytes on failure.
    let encodeChecked (value: CborValue) =
        if Object.ReferenceEquals(box value, null) then nullArg (nameof value)
        let encoder = Encoder()
        encoder.WriteValue(value, 0)
        encoder.Bytes()

    /// Append one complete encoding, leaving the stream unchanged on codec failure.
    let encodeIntoChecked value (destination: MemoryStream) =
        ArgumentNullException.ThrowIfNull(destination)
        let encoded = encodeChecked value
        destination.Position <- destination.Length
        destination.Write(encoded, 0, encoded.Length)

    /// Decode exactly one canonical item.
    let decode (bytes: byte array) =
        ArgumentNullException.ThrowIfNull(bytes)
        let cursor = Cursor(bytes)
        let value = cursor.ReadValue(0)
        if cursor.Remaining <> 0 then
            raise (error "trailing-bytes")
        value
