namespace CodingAdventures.AesModes.FSharp

open System
open System.Buffers.Binary
open CodingAdventures.Aes.FSharp

[<RequireQualifiedAccess>]
module AesModes =
    let blockSizeBytes = 16

    let private gcmNonceSizeBytes = 12
    let private gcmTagSizeBytes = 16

    let private requireBytes name (value: byte array) =
        if isNull value then
            nullArg name

    let private validateKey (key: byte array) =
        requireBytes "key" key
        if key.Length <> 16 && key.Length <> 24 && key.Length <> 32 then
            invalidArg "key" "AES key must be 16, 24, or 32 bytes."

    let private validateLength name label length (value: byte array) =
        requireBytes name value
        if value.Length <> length then
            invalidArg name $"{label} must be {length} bytes."

    let private validateCiphertext name label (ciphertext: byte array) =
        requireBytes name ciphertext
        if ciphertext.Length = 0 || ciphertext.Length % blockSizeBytes <> 0 then
            invalidArg name $"{label} must be a positive multiple of 16 bytes."

    let private readBlock (data: byte array) offset =
        let block = Array.zeroCreate<byte> blockSizeBytes
        Buffer.BlockCopy(data, offset, block, 0, blockSizeBytes)
        block

    let private buildCounterBlock (nonce: byte array) (counter: uint32) =
        let block = Array.zeroCreate<byte> blockSizeBytes
        Buffer.BlockCopy(nonce, 0, block, 0, nonce.Length)
        block.[12] <- byte (counter >>> 24)
        block.[13] <- byte (counter >>> 16)
        block.[14] <- byte (counter >>> 8)
        block.[15] <- byte counter
        block

    let pkcs7Pad (data: byte array) =
        requireBytes "data" data
        let padLength = blockSizeBytes - (data.Length % blockSizeBytes)
        let result = Array.zeroCreate<byte> (data.Length + padLength)
        Buffer.BlockCopy(data, 0, result, 0, data.Length)
        for index in data.Length .. result.Length - 1 do
            result.[index] <- byte padLength
        result

    let pkcs7Unpad (data: byte array) =
        requireBytes "data" data
        if data.Length = 0 || data.Length % blockSizeBytes <> 0 then
            invalidArg "data" "Padded data must be a positive multiple of 16 bytes."

        let padLength = int data.[data.Length - 1]
        if padLength < 1 || padLength > blockSizeBytes then
            invalidArg "data" "Invalid PKCS#7 padding."

        let mutable diff = 0
        for index in data.Length - padLength .. data.Length - 1 do
            diff <- diff ||| (int data.[index] ^^^ padLength)

        if diff <> 0 then
            invalidArg "data" "Invalid PKCS#7 padding."

        let result = Array.zeroCreate<byte> (data.Length - padLength)
        Buffer.BlockCopy(data, 0, result, 0, result.Length)
        result

    let xorBytes (left: byte array) (right: byte array) =
        requireBytes "left" left
        requireBytes "right" right
        if left.Length <> right.Length then
            invalidArg "right" "Byte arrays must have the same length."

        Array.init left.Length (fun index -> byte (int left.[index] ^^^ int right.[index]))

    let ecbEncrypt (plaintext: byte array) (key: byte array) =
        requireBytes "plaintext" plaintext
        validateKey key
        let padded = pkcs7Pad plaintext
        let result = Array.zeroCreate<byte> padded.Length

        for offset in 0 .. blockSizeBytes .. padded.Length - 1 do
            let encrypted = AesBlock.encryptBlock (readBlock padded offset) key
            Buffer.BlockCopy(encrypted, 0, result, offset, blockSizeBytes)

        result

    let ecbDecrypt (ciphertext: byte array) (key: byte array) =
        validateCiphertext "ciphertext" "ECB ciphertext" ciphertext
        validateKey key
        let padded = Array.zeroCreate<byte> ciphertext.Length

        for offset in 0 .. blockSizeBytes .. ciphertext.Length - 1 do
            let decrypted = AesBlock.decryptBlock (readBlock ciphertext offset) key
            Buffer.BlockCopy(decrypted, 0, padded, offset, blockSizeBytes)

        pkcs7Unpad padded

    let cbcEncrypt (plaintext: byte array) (key: byte array) (iv: byte array) =
        requireBytes "plaintext" plaintext
        validateKey key
        validateLength "iv" "CBC IV" blockSizeBytes iv
        let padded = pkcs7Pad plaintext
        let result = Array.zeroCreate<byte> padded.Length
        let mutable previous = Array.copy iv

        for offset in 0 .. blockSizeBytes .. padded.Length - 1 do
            let block = xorBytes (readBlock padded offset) previous
            let encrypted = AesBlock.encryptBlock block key
            Buffer.BlockCopy(encrypted, 0, result, offset, blockSizeBytes)
            previous <- encrypted

        result

    let cbcDecrypt (ciphertext: byte array) (key: byte array) (iv: byte array) =
        validateCiphertext "ciphertext" "CBC ciphertext" ciphertext
        validateKey key
        validateLength "iv" "CBC IV" blockSizeBytes iv
        let padded = Array.zeroCreate<byte> ciphertext.Length
        let mutable previous = Array.copy iv

        for offset in 0 .. blockSizeBytes .. ciphertext.Length - 1 do
            let cipherBlock = readBlock ciphertext offset
            let decrypted = AesBlock.decryptBlock cipherBlock key
            let plainBlock = xorBytes decrypted previous
            Buffer.BlockCopy(plainBlock, 0, padded, offset, blockSizeBytes)
            previous <- cipherBlock

        pkcs7Unpad padded

    let ctrEncrypt (plaintext: byte array) (key: byte array) (nonce: byte array) =
        requireBytes "plaintext" plaintext
        validateKey key
        validateLength "nonce" "CTR nonce" gcmNonceSizeBytes nonce
        let result = Array.zeroCreate<byte> plaintext.Length
        let mutable counter = 1u
        let mutable offset = 0

        while offset < plaintext.Length do
            let keystream = AesBlock.encryptBlock (buildCounterBlock nonce counter) key
            let count = min blockSizeBytes (plaintext.Length - offset)
            for index in 0 .. count - 1 do
                result.[offset + index] <- byte (int plaintext.[offset + index] ^^^ int keystream.[index])

            counter <- counter + 1u
            offset <- offset + blockSizeBytes

        result

    let ctrDecrypt (ciphertext: byte array) (key: byte array) (nonce: byte array) =
        ctrEncrypt ciphertext key nonce

    let private xorInPlace (left: byte array) (right: byte array) =
        for index in 0 .. left.Length - 1 do
            left[index] <- left[index] ^^^ right[index]

    let private getBit (value: byte array) bitIndex =
        (int value[bitIndex / 8] &&& (1 <<< (7 - bitIndex % 8))) <> 0

    let private shiftRightOne (value: byte array) =
        let mutable carry = 0

        for index in 0 .. value.Length - 1 do
            let nextCarry = int value[index] &&& 1
            value[index] <- byte (((int value[index] >>> 1) ||| (carry <<< 7)) &&& 0xff)
            carry <- nextCarry

    let private multiplyGf128 (left: byte array) (right: byte array) =
        let result = Array.zeroCreate<byte> blockSizeBytes
        let value = Array.copy left

        for bit in 0 .. 127 do
            if getBit right bit then
                xorInPlace result value

            let lsbSet = (value[value.Length - 1] &&& 1uy) <> 0uy
            shiftRightOne value

            if lsbSet then
                value[0] <- value[0] ^^^ 0xe1uy

        result

    let private ghashBlocks (value: byte array) (hashSubkey: byte array) (data: byte array) =
        let mutable offset = 0

        while offset < data.Length do
            let block = Array.zeroCreate<byte> blockSizeBytes
            let count = min blockSizeBytes (data.Length - offset)
            Buffer.BlockCopy(data, offset, block, 0, count)
            xorInPlace value block
            let multiplied = multiplyGf128 value hashSubkey
            Buffer.BlockCopy(multiplied, 0, value, 0, blockSizeBytes)
            offset <- offset + blockSizeBytes

    let private ghash hashSubkey aad ciphertext =
        let value = Array.zeroCreate<byte> blockSizeBytes
        ghashBlocks value hashSubkey aad
        ghashBlocks value hashSubkey ciphertext

        let lengthBlock = Array.zeroCreate<byte> blockSizeBytes
        BinaryPrimitives.WriteUInt64BigEndian(lengthBlock.AsSpan(0, 8), uint64 aad.Length * 8UL)
        BinaryPrimitives.WriteUInt64BigEndian(lengthBlock.AsSpan(8, 8), uint64 ciphertext.Length * 8UL)
        xorInPlace value lengthBlock
        multiplyGf128 value hashSubkey

    let private gctr key nonce initialCounter (input: byte array) =
        let result = Array.zeroCreate<byte> input.Length
        let mutable counter = initialCounter
        let mutable offset = 0

        while offset < input.Length do
            let keystream = AesBlock.encryptBlock (buildCounterBlock nonce counter) key
            let count = min blockSizeBytes (input.Length - offset)

            for index in 0 .. count - 1 do
                result[offset + index] <- input[offset + index] ^^^ keystream[index]

            counter <- counter + 1u
            offset <- offset + blockSizeBytes

        result

    let private computeGcmTag key iv aad ciphertext =
        let hashSubkey = AesBlock.encryptBlock (Array.zeroCreate<byte> blockSizeBytes) key
        let tagMask = AesBlock.encryptBlock (buildCounterBlock iv 1u) key
        let authentication = ghash hashSubkey aad ciphertext
        xorInPlace tagMask authentication
        tagMask

    let private fixedTimeEquals (expected: byte array) (actual: byte array) =
        if expected.Length <> actual.Length then
            false
        else
            let mutable diff = 0

            for index in 0 .. expected.Length - 1 do
                diff <- diff ||| (int expected[index] ^^^ int actual[index])

            diff = 0

    let gcmEncrypt (plaintext: byte array) (key: byte array) (iv: byte array) (aad: byte array) =
        requireBytes "plaintext" plaintext
        validateKey key
        validateLength "iv" "GCM IV" gcmNonceSizeBytes iv
        let aadValue = if isNull aad then Array.empty<byte> else aad
        let ciphertext = gctr key iv 2u plaintext
        let tag = computeGcmTag key iv aadValue ciphertext
        ciphertext, tag

    let gcmDecrypt (ciphertext: byte array) (key: byte array) (iv: byte array) (aad: byte array) (tag: byte array) =
        requireBytes "ciphertext" ciphertext
        validateKey key
        validateLength "iv" "GCM IV" gcmNonceSizeBytes iv
        validateLength "tag" "GCM tag" gcmTagSizeBytes tag
        let aadValue = if isNull aad then Array.empty<byte> else aad
        let expectedTag = computeGcmTag key iv aadValue ciphertext

        if not (fixedTimeEquals tag expectedTag) then
            raise (InvalidOperationException "AES-GCM authentication tag mismatch.")

        gctr key iv 2u ciphertext
