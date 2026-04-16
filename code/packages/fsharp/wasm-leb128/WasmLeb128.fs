namespace CodingAdventures.WasmLeb128.FSharp

open System
open System.Collections.Generic

module Version =
    [<Literal>]
    let VERSION = "0.1.0"

type LEB128Error(message: string) =
    inherit Exception(message)

module WasmLeb128 =
    let private continuationBit = 0x80
    let private payloadMask = 0x7F
    let private maxLeb128Bytes32 = 5

    let decodeUnsignedAt (data: byte array) (offset: int) =
        let mutable result = 0u
        let mutable shift = 0
        let mutable bytesConsumed = 0
        let mutable index = offset
        let mutable finished = false

        while index < data.Length && not finished do
            if bytesConsumed >= maxLeb128Bytes32 then
                raise (LEB128Error(sprintf "LEB128 sequence exceeds maximum %d bytes for a 32-bit value" maxLeb128Bytes32))

            let currentByte = data[index]
            let payload = uint32 (int currentByte &&& payloadMask)
            result <- result ||| (payload <<< shift)
            shift <- shift + 7
            bytesConsumed <- bytesConsumed + 1
            index <- index + 1

            if (int currentByte &&& continuationBit) = 0 then
                finished <- true

        if not finished then
            raise (LEB128Error(sprintf "LEB128 sequence is unterminated: reached end of data at offset %d without finding a byte with continuation bit = 0" (offset + bytesConsumed)))

        result, bytesConsumed

    let decodeUnsigned (data: byte array) =
        decodeUnsignedAt data 0

    let decodeSignedAt (data: byte array) (offset: int) =
        let mutable result = 0
        let mutable shift = 0
        let mutable bytesConsumed = 0
        let mutable index = offset
        let mutable finished = false
        let mutable lastByte = 0uy

        while index < data.Length && not finished do
            if bytesConsumed >= maxLeb128Bytes32 then
                raise (LEB128Error(sprintf "LEB128 sequence exceeds maximum %d bytes for a 32-bit value" maxLeb128Bytes32))

            let currentByte = data[index]
            lastByte <- currentByte
            let payload = int currentByte &&& payloadMask
            result <- result ||| (payload <<< shift)
            shift <- shift + 7
            bytesConsumed <- bytesConsumed + 1
            index <- index + 1

            if (int currentByte &&& continuationBit) = 0 then
                finished <- true

        if not finished then
            raise (LEB128Error(sprintf "LEB128 sequence is unterminated: reached end of data at offset %d without finding a byte with continuation bit = 0" (offset + bytesConsumed)))

        if shift < 32 && ((int lastByte &&& 0x40) <> 0) then
            result <- result ||| -(1 <<< shift)

        result, bytesConsumed

    let decodeSigned (data: byte array) =
        decodeSignedAt data 0

    let encodeUnsigned (value: uint32) =
        let bytes = ResizeArray<byte>()
        let mutable remaining = value
        let mutable keepEncoding = true

        while keepEncoding do
            let mutable currentByte = byte (remaining &&& uint32 payloadMask)
            remaining <- remaining >>> 7

            if remaining <> 0u then
                currentByte <- byte (int currentByte ||| continuationBit)

            bytes.Add(currentByte)
            keepEncoding <- remaining <> 0u

        bytes.ToArray()

    let encodeUnsignedInt (value: int) =
        encodeUnsigned (uint32 value)

    let encodeSigned (value: int) =
        let bytes = ResizeArray<byte>()
        let mutable remaining = value
        let mutable doneEncoding = false

        while not doneEncoding do
            let mutable currentByte = byte (remaining &&& payloadMask)
            remaining <- remaining >>> 7

            doneEncoding <-
                (remaining = 0 && (int currentByte &&& 0x40) = 0)
                || (remaining = -1 && (int currentByte &&& 0x40) <> 0)

            if not doneEncoding then
                currentByte <- byte (int currentByte ||| continuationBit)

            bytes.Add(currentByte)

        bytes.ToArray()
