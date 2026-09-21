namespace CodingAdventures.DerTlv.FSharp

open System

/// Explicit resource and representation limits.
type DerLimits(maxInputLength: int64, maxValueLength: int64, maxElements: int64, maxTagNumber: uint32) =
    do
        if maxInputLength < 0L then raise (ArgumentOutOfRangeException(nameof maxInputLength))
        if maxValueLength < 0L then raise (ArgumentOutOfRangeException(nameof maxValueLength))
        if maxElements < 0L then raise (ArgumentOutOfRangeException(nameof maxElements))
    member _.MaxInputLength = maxInputLength
    member _.MaxValueLength = maxValueLength
    member _.MaxElements = maxElements
    member _.MaxTagNumber = maxTagNumber
    static member Default = DerLimits(1_048_576L, 1_048_576L, 4_096L, UInt32.MaxValue)

/// A stable payload-blind framing failure.
type DerError internal (id: string, offset: int) =
    inherit Exception(sprintf "DER framing error %s at byte %d" id offset)
    member _.Id = id
    member _.Offset = offset

type DerTag =
    { Class: string
      Constructed: bool
      Number: uint32 }

/// One frame represented by ranges into caller-provided memory.
type DerElement internal (tag: DerTag, input: ReadOnlyMemory<byte>, headerLength: int, encodedLength: int) =
    member _.Tag = tag
    member _.Header = input.Slice(0, headerLength)
    member _.Value = input.Slice(headerLength, encodedLength - headerLength)
    member _.Encoded = input.Slice(0, encodedLength)

type Decoded =
    { Element: DerElement
      Remainder: ReadOnlyMemory<byte> }

module DerTlv =
    let private fail id offset = raise (DerError(id, offset))
    let private tagClasses = [| "universal"; "application"; "context-specific"; "private" |]

    let private decodeHighTag (input: ReadOnlySpan<byte>) baseOffset (limits: DerLimits) =
        let mutable number = 0UL
        let mutable index = 1
        let mutable complete = false
        while not complete do
            if index >= input.Length then fail "truncated-high-tag" (baseOffset + index)
            let octet = input[index]
            let payload = uint64 (octet &&& 0x7fuy)
            if index = 1 && payload = 0UL then fail "non-minimal-tag" (baseOffset + index)
            if number > (uint64 UInt32.MaxValue - payload) / 128UL then fail "tag-overflow" (baseOffset + index)
            number <- number * 128UL + payload
            if number > uint64 limits.MaxTagNumber then fail "tag-limit-exceeded" (baseOffset + index)
            index <- index + 1
            complete <- octet &&& 0x80uy = 0uy
        if number < 31UL then fail "non-minimal-tag" baseOffset
        uint32 number, index

    let private decodeLength (input: ReadOnlySpan<byte>) baseOffset identifierLength =
        let lengthOffset = baseOffset + identifierLength
        if identifierLength >= input.Length then fail "truncated-length" lengthOffset
        let first = input[identifierLength]
        if first < 0x80uy then uint64 first, 1, lengthOffset
        else
            if first = 0x80uy then fail "indefinite-length" lengthOffset
            if first = 0xffuy then fail "reserved-length" lengthOffset
            let count = int (first &&& 0x7fuy)
            if count > 8 then fail "length-too-wide" lengthOffset
            if identifierLength + 1 + count > input.Length then fail "truncated-length" (baseOffset + input.Length)
            if input[identifierLength + 1] = 0uy then fail "non-minimal-length" (lengthOffset + 1)
            let mutable value = 0UL
            for index in 0 .. count - 1 do
                value <- value * 256UL + uint64 input[identifierLength + 1 + index]
            if value < 128UL then fail "non-minimal-length" lengthOffset
            value, count + 1, lengthOffset

    let private decodeAt (input: ReadOnlyMemory<byte>) baseOffset (limits: DerLimits) =
        if int64 input.Length > limits.MaxInputLength then fail "input-limit-exceeded" baseOffset
        if input.IsEmpty then fail "empty-input" baseOffset
        let bytes = input.Span
        let first = bytes[0]
        let tagClass = tagClasses[int (first >>> 6)]
        let constructed = first &&& 0x20uy <> 0uy
        let low = first &&& 0x1fuy
        let number, identifierLength =
            if low <> 0x1fuy then
                let number = uint32 low
                if number > limits.MaxTagNumber then fail "tag-limit-exceeded" baseOffset
                number, 1
            else decodeHighTag bytes baseOffset limits
        if tagClass = "universal" && number = 0u then fail "end-of-contents" baseOffset
        let valueLength, lengthLength, lengthOffset = decodeLength bytes baseOffset identifierLength
        if valueLength > uint64 Int32.MaxValue then fail "length-host-overflow" lengthOffset
        if valueLength > uint64 limits.MaxValueLength then fail "value-limit-exceeded" lengthOffset
        let headerLength = identifierLength + lengthLength
        if valueLength > uint64 (Int32.MaxValue - headerLength) then fail "length-host-overflow" lengthOffset
        let encodedLength = headerLength + int valueLength
        if encodedLength > input.Length then fail "truncated-value" (baseOffset + input.Length)
        let tag = { Class = tagClass; Constructed = constructed; Number = number }
        DerElement(tag, input, headerLength, encodedLength), encodedLength

    let internal decodeOneAt (limits: DerLimits) (input: ReadOnlyMemory<byte>) baseOffset =
        ArgumentNullException.ThrowIfNull(limits)
        let element, consumed = decodeAt input baseOffset limits
        { Element = element; Remainder = input.Slice(consumed) }

    let decodeOneWith (limits: DerLimits) (input: ReadOnlyMemory<byte>) =
        decodeOneAt limits input 0

    let decodeOne input = decodeOneWith DerLimits.Default input

    let decodeExactWith limits input =
        let decoded = decodeOneWith limits input
        if not decoded.Remainder.IsEmpty then fail "trailing-data" decoded.Element.Encoded.Length
        decoded.Element

    let decodeExact input = decodeExactWith DerLimits.Default input

/// Iterative sibling decoder whose failed reads preserve state.
type DerCursor(input: ReadOnlyMemory<byte>, limits: DerLimits) =
    let mutable offset = 0
    let mutable elementsRead = 0L
    do
        ArgumentNullException.ThrowIfNull(limits)
        if int64 input.Length > limits.MaxInputLength then raise (DerError("input-limit-exceeded", 0))
    new(input: ReadOnlyMemory<byte>) = DerCursor(input, DerLimits.Default)
    member _.ElementsRead = elementsRead
    member _.Remaining = input.Slice(offset)
    member this.Read() =
        if this.Remaining.IsEmpty then None
        elif elementsRead >= limits.MaxElements then raise (DerError("element-limit-exceeded", offset))
        else
            let decoded = DerTlv.decodeOneAt limits this.Remaining offset
            offset <- offset + decoded.Element.Encoded.Length
            elementsRead <- elementsRead + 1L
            Some decoded.Element
    member this.Finish() =
        if not this.Remaining.IsEmpty then raise (DerError("trailing-data", offset))
