namespace CodingAdventures.DerAsn1.FSharp

open System
open System.Collections.Generic
open System.Text
open CodingAdventures.DerTlv.FSharp

type Asn1Limits(der: DerLimits, maxDepth: int, maxTotalElements: int64, maxOidArcs: int) =
    do
        ArgumentNullException.ThrowIfNull(der)
        if maxDepth < 0 then raise (ArgumentOutOfRangeException(nameof maxDepth))
        if maxTotalElements < 0L then raise (ArgumentOutOfRangeException(nameof maxTotalElements))
        if maxOidArcs < 0 then raise (ArgumentOutOfRangeException(nameof maxOidArcs))
    member _.Der = der
    member _.MaxDepth = maxDepth
    member _.MaxTotalElements = maxTotalElements
    member _.MaxOidArcs = maxOidArcs
    static member Default = Asn1Limits(DerLimits.Default, 32, 16_384L, 128)

type Asn1ErrorKind =
    | Framing | UnexpectedTag | DecoderLimitMismatch | DepthLimitExceeded | ElementLimitExceeded
    | InvalidBooleanLength | InvalidBooleanValue | EmptyInteger | NonMinimalInteger
    | NegativeInteger | IntegerOverflow | MissingUnusedBitCount | InvalidUnusedBitCount
    | NonZeroBitPadding | BitLengthOverflow | NonEmptyNull | NonAsciiIa5String
    | EmptyObjectIdentifier | UnterminatedObjectIdentifier | NonMinimalObjectIdentifier
    | ObjectIdentifierOverflow | OidArcLimitExceeded

module private ErrorIds =
    let ofKind = function
        | Framing -> "framing"
        | UnexpectedTag -> "unexpected-tag"
        | DecoderLimitMismatch -> "decoder-limit-mismatch"
        | DepthLimitExceeded -> "depth-limit-exceeded"
        | ElementLimitExceeded -> "element-limit-exceeded"
        | InvalidBooleanLength -> "invalid-boolean-length"
        | InvalidBooleanValue -> "invalid-boolean-value"
        | EmptyInteger -> "empty-integer"
        | NonMinimalInteger -> "non-minimal-integer"
        | NegativeInteger -> "negative-integer"
        | IntegerOverflow -> "integer-overflow"
        | MissingUnusedBitCount -> "missing-unused-bit-count"
        | InvalidUnusedBitCount -> "invalid-unused-bit-count"
        | NonZeroBitPadding -> "non-zero-bit-padding"
        | BitLengthOverflow -> "bit-length-overflow"
        | NonEmptyNull -> "non-empty-null"
        | NonAsciiIa5String -> "non-ascii-ia5-string"
        | EmptyObjectIdentifier -> "empty-object-identifier"
        | UnterminatedObjectIdentifier -> "unterminated-object-identifier"
        | NonMinimalObjectIdentifier -> "non-minimal-object-identifier"
        | ObjectIdentifierOverflow -> "object-identifier-overflow"
        | OidArcLimitExceeded -> "oid-arc-limit-exceeded"

type Asn1Error internal (kind: Asn1ErrorKind, offset: int, framingKind: string option) =
    inherit Exception(sprintf "ASN.1 DER value error %s at byte %d" (ErrorIds.ofKind kind) offset)
    member _.Kind = kind
    member _.KindId = ErrorIds.ofKind kind
    member _.Offset = offset
    member _.FramingKind = framingKind

module private Internal =
    let failure kind offset = Asn1Error(kind, offset, None)
    let framing (error: DerError) = Asn1Error(Framing, error.Offset, Some error.Id)
    let limitsEqual (left: Asn1Limits) (right: Asn1Limits) =
        left.MaxDepth = right.MaxDepth
        && left.MaxTotalElements = right.MaxTotalElements
        && left.MaxOidArcs = right.MaxOidArcs
        && left.Der.MaxInputLength = right.Der.MaxInputLength
        && left.Der.MaxValueLength = right.Der.MaxValueLength
        && left.Der.MaxElements = right.Der.MaxElements
        && left.Der.MaxTagNumber = right.Der.MaxTagNumber
type Asn1Element internal (element: DerElement, depth: int) =
    let header = element.Header.ToArray()
    let value = element.Value.ToArray()
    let encoded = element.Encoded.ToArray()
    member _.Tag = element.Tag
    member _.Header = ReadOnlyMemory<byte>(Array.copy header)
    member _.Value = ReadOnlyMemory<byte>(Array.copy value)
    member _.Encoded = ReadOnlyMemory<byte>(Array.copy encoded)
    member _.Depth = depth
    member internal _.ValueOffset = header.Length
    member internal _.ValueCopy() = ReadOnlyMemory<byte>(Array.copy value)

module private TagChecks =
    let expectTag (element: Asn1Element) tagClass constructed number =
        ArgumentNullException.ThrowIfNull(element)
        let tag = element.Tag
        if tag.Class <> tagClass || tag.Constructed <> constructed || tag.Number <> number then
            raise (Internal.failure UnexpectedTag 0)

type Asn1Decoder(limits: Asn1Limits) =
    let mutable elementsRead = 0L
    do ArgumentNullException.ThrowIfNull(limits)
    new() = Asn1Decoder(Asn1Limits.Default)
    member _.Limits = limits
    member _.ElementsRead = elementsRead
    member private _.RequireElementCapacity(offset: int) =
        if elementsRead >= limits.MaxTotalElements then raise (Internal.failure ElementLimitExceeded offset)
    member private _.ChildDepth(element: Asn1Element) =
        if element.Depth = Int32.MaxValue then raise (Internal.failure DepthLimitExceeded 0)
        let childDepth = element.Depth + 1
        if childDepth >= limits.MaxDepth then raise (Internal.failure DepthLimitExceeded 0)
        childDepth
    member this.DecodeExact(input: ReadOnlyMemory<byte>) =
        if limits.MaxDepth = 0 then raise (Internal.failure DepthLimitExceeded 0)
        this.RequireElementCapacity(0)
        try
            let snapshot = Asn1Element(DerTlv.decodeExactWith limits.Der input, 0)
            elementsRead <- elementsRead + 1L
            snapshot
        with :? DerError as error -> raise (Internal.framing error)
    member private this.Constructed(element: Asn1Element, tagClass: string, number: uint32) =
        TagChecks.expectTag element tagClass true number
        let childDepth = this.ChildDepth(element)
        try Asn1Cursor(element.ValueCopy(), childDepth, limits)
        with :? DerError as error -> raise (Internal.framing error)
    member this.Sequence(element: Asn1Element) = this.Constructed(element, "universal", 16u)
    member this.Set(element: Asn1Element) = this.Constructed(element, "universal", 17u)
    member this.Explicit(element: Asn1Element, tagNumber: uint32) =
        TagChecks.expectTag element "context-specific" true tagNumber
        let childDepth = this.ChildDepth(element)
        this.RequireElementCapacity(element.ValueOffset)
        try
            let snapshot = Asn1Element(DerTlv.decodeExactWith limits.Der (element.ValueCopy()), childDepth)
            elementsRead <- elementsRead + 1L
            snapshot
        with :? DerError as error -> raise (Internal.framing error)
    member internal _.CommitElement() = elementsRead <- elementsRead + 1L
    member internal this.RequireCapacity(offset: int) = this.RequireElementCapacity(offset)

and Asn1Cursor internal (input: ReadOnlyMemory<byte>, childDepth: int, limits: Asn1Limits) =
    let cursor = DerCursor(ReadOnlyMemory<byte>(input.ToArray()), limits.Der)
    member _.Remaining = ReadOnlyMemory<byte>(cursor.Remaining.ToArray())
    member _.Read(decoder: Asn1Decoder) =
        ArgumentNullException.ThrowIfNull(decoder)
        if cursor.Remaining.IsEmpty then None
        elif not (Internal.limitsEqual decoder.Limits limits) then raise (Internal.failure DecoderLimitMismatch 0)
        else
            decoder.RequireCapacity(0)
            try
                match cursor.Read() with
                | None -> None
                | Some element ->
                    let value = Asn1Element(element, childDepth)
                    decoder.CommitElement()
                    Some value
            with :? DerError as error -> raise (Internal.framing error)
    member _.Finish() =
        try cursor.Finish()
        with :? DerError as error -> raise (Internal.framing error)

type DerInteger internal (value: ReadOnlyMemory<byte>, valueOffset: int) =
    let signedBytes = value.ToArray()
    member _.SignedBytes = ReadOnlyMemory<byte>(Array.copy signedBytes)
    member _.IsNegative = signedBytes[0] &&& 0x80uy <> 0uy
    member this.ToUInt64() =
        if this.IsNegative then raise (Internal.failure NegativeInteger valueOffset)
        let start = if signedBytes[0] = 0uy then 1 else 0
        if signedBytes.Length - start > sizeof<uint64> then raise (Internal.failure IntegerOverflow valueOffset)
        let mutable result = 0UL
        for index in start .. signedBytes.Length - 1 do result <- result * 256UL + uint64 signedBytes[index]
        result

type DerBitString internal (bytes: ReadOnlyMemory<byte>, unusedBits: byte, bitLength: int64) =
    let snapshot = bytes.ToArray()
    member _.Bytes = ReadOnlyMemory<byte>(Array.copy snapshot)
    member _.UnusedBits = unusedBits
    member _.BitLength = bitLength

type ObjectIdentifier internal (encoded: ReadOnlyMemory<byte>, arcs: uint64 array) =
    let snapshot = encoded.ToArray()
    let arcSnapshot = Array.copy arcs
    let readOnlyArcs: IReadOnlyList<uint64> = Array.AsReadOnly(arcSnapshot)
    member _.Encoded = ReadOnlyMemory<byte>(Array.copy snapshot)
    member _.Arcs = readOnlyArcs
    member _.ArcCount = arcSnapshot.Length
    member _.EqualsArcs(expected: IEnumerable<uint64>) =
        ArgumentNullException.ThrowIfNull(expected)
        use enumerator = expected.GetEnumerator()
        let mutable index = 0
        let mutable equal = true
        while equal && index < arcSnapshot.Length do
            if not (enumerator.MoveNext()) || enumerator.Current <> arcSnapshot[index] then equal <- false
            else index <- index + 1
        equal && not (enumerator.MoveNext())

[<RequireQualifiedAccess>]
module DerAsn1 =
    let errorId kind = ErrorIds.ofKind kind
    let private expectUniversalPrimitive element number = TagChecks.expectTag element "universal" false number
    let private expectContextPrimitive element number = TagChecks.expectTag element "context-specific" false number
    let decodeBoolean (element: Asn1Element) =
        expectUniversalPrimitive element 1u
        let value = element.ValueCopy().Span
        if value.Length <> 1 then raise (Internal.failure InvalidBooleanLength element.ValueOffset)
        match value[0] with | 0x00uy -> false | 0xffuy -> true | _ -> raise (Internal.failure InvalidBooleanValue element.ValueOffset)
    let decodeInteger (element: Asn1Element) =
        expectUniversalPrimitive element 2u
        let copy = element.ValueCopy()
        let value = copy.Span
        if value.IsEmpty then raise (Internal.failure EmptyInteger element.ValueOffset)
        if value.Length > 1 && ((value[0] = 0uy && value[1] &&& 0x80uy = 0uy) || (value[0] = 0xffuy && value[1] &&& 0x80uy <> 0uy)) then
            raise (Internal.failure NonMinimalInteger element.ValueOffset)
        DerInteger(copy, element.ValueOffset)
    let decodeBitString (element: Asn1Element) =
        expectUniversalPrimitive element 3u
        let copy = element.ValueCopy()
        let value = copy.Span
        if value.IsEmpty then raise (Internal.failure MissingUnusedBitCount element.ValueOffset)
        let unusedBits = value[0]
        let payload = copy.Slice(1)
        if unusedBits > 7uy || (payload.IsEmpty && unusedBits <> 0uy) then raise (Internal.failure InvalidUnusedBitCount element.ValueOffset)
        if unusedBits <> 0uy then
            let mask = byte ((1 <<< int unusedBits) - 1)
            if payload.Span[payload.Length - 1] &&& mask <> 0uy then raise (Internal.failure NonZeroBitPadding (element.ValueOffset + value.Length - 1))
        let bitLength = int64 payload.Length * 8L - int64 unusedBits
        DerBitString(payload, unusedBits, bitLength)
    let decodeOctetString (element: Asn1Element) = expectUniversalPrimitive element 4u; element.ValueCopy()
    let decodeImplicitOctetString (element: Asn1Element) tagNumber = expectContextPrimitive element tagNumber; element.ValueCopy()
    let private decodeIa5Contents (element: Asn1Element) =
        let value = element.ValueCopy().ToArray()
        for index in 0 .. value.Length - 1 do
            if value[index] > 0x7fuy then raise (Internal.failure NonAsciiIa5String (element.ValueOffset + index))
        Encoding.ASCII.GetString(value)
    let decodeIa5String (element: Asn1Element) = expectUniversalPrimitive element 22u; decodeIa5Contents element
    let decodeImplicitIa5String (element: Asn1Element) tagNumber = expectContextPrimitive element tagNumber; decodeIa5Contents element
    let decodeNull (element: Asn1Element) =
        expectUniversalPrimitive element 5u
        if not (element.ValueCopy().IsEmpty) then raise (Internal.failure NonEmptyNull element.ValueOffset)
    let private parseBase128 (encoded: ReadOnlySpan<byte>) start valueOffset =
        if encoded[start] = 0x80uy then raise (Internal.failure NonMinimalObjectIdentifier (valueOffset + start))
        let mutable value = 0UL
        let mutable offset = start
        let mutable complete = false
        while not complete do
            if offset >= encoded.Length then raise (Internal.failure UnterminatedObjectIdentifier (valueOffset + offset))
            let octet = encoded[offset]
            let payload = uint64 (octet &&& 0x7fuy)
            if value > (UInt64.MaxValue - payload) / 128UL then raise (Internal.failure ObjectIdentifierOverflow (valueOffset + offset))
            value <- value * 128UL + payload
            offset <- offset + 1
            complete <- octet &&& 0x80uy = 0uy
        value, offset
    let private decodeOidContents (element: Asn1Element) (limits: Asn1Limits) =
        let encoded = element.ValueCopy()
        if encoded.IsEmpty then raise (Internal.failure EmptyObjectIdentifier element.ValueOffset)
        let first, initialOffset = parseBase128 encoded.Span 0 element.ValueOffset
        let arcs = ResizeArray<uint64>()
        if first < 40UL then arcs.Add(0UL); arcs.Add(first)
        elif first < 80UL then arcs.Add(1UL); arcs.Add(first - 40UL)
        else arcs.Add(2UL); arcs.Add(first - 80UL)
        if arcs.Count > limits.MaxOidArcs then raise (Internal.failure OidArcLimitExceeded element.ValueOffset)
        let mutable offset = initialOffset
        while offset < encoded.Length do
            let arcStart = offset
            let arc, nextOffset = parseBase128 encoded.Span offset element.ValueOffset
            arcs.Add(arc)
            if arcs.Count > limits.MaxOidArcs then raise (Internal.failure OidArcLimitExceeded (element.ValueOffset + arcStart))
            offset <- nextOffset
        ObjectIdentifier(encoded, arcs.ToArray())
    let decodeObjectIdentifierWith (limits: Asn1Limits) (element: Asn1Element) =
        ArgumentNullException.ThrowIfNull(limits); expectUniversalPrimitive element 6u; decodeOidContents element limits
    let decodeObjectIdentifier element = decodeObjectIdentifierWith Asn1Limits.Default element
    let decodeImplicitObjectIdentifierWith (limits: Asn1Limits) (element: Asn1Element) tagNumber =
        ArgumentNullException.ThrowIfNull(limits); expectContextPrimitive element tagNumber; decodeOidContents element limits
    let decodeImplicitObjectIdentifier element tagNumber = decodeImplicitObjectIdentifierWith Asn1Limits.Default element tagNumber
