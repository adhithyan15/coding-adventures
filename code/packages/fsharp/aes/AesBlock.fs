namespace CodingAdventures.Aes.FSharp

open System

[<RequireQualifiedAccess>]
module AesBlock =
    let private blockSizeBytes = 16

    let private rcon =
        [| 0x00uy; 0x01uy; 0x02uy; 0x04uy; 0x08uy; 0x10uy; 0x20uy; 0x40uy; 0x80uy; 0x1buy; 0x36uy; 0x6cuy; 0xd8uy; 0xabuy; 0x4duy |]

    let private multiply (left: byte) (right: byte) =
        let mutable result = 0
        let mutable a = left
        let mutable b = right

        for _ in 0 .. 7 do
            if (b &&& 1uy) <> 0uy then
                result <- result ^^^ int a

            let carry = (a &&& 0x80uy) <> 0uy
            a <- byte ((int a <<< 1) &&& 0xff)
            if carry then
                a <- a ^^^ 0x1buy

            b <- b >>> 1

        byte result

    let private rotateLeft (value: byte) count =
        byte (((int value <<< count) ||| (int value >>> (8 - count))) &&& 0xff)

    let private multiplicativeInverse value =
        if value = 0uy then
            0uy
        else
            let mutable result = 1uy

            for _ in 0 .. 253 do
                result <- multiply result value

            result

    let private affineTransform value =
        value
        ^^^ rotateLeft value 1
        ^^^ rotateLeft value 2
        ^^^ rotateLeft value 3
        ^^^ rotateLeft value 4
        ^^^ 0x63uy

    let private buildSBoxes () =
        let sbox = Array.zeroCreate<byte> 256
        let invSbox = Array.zeroCreate<byte> 256

        for value in 0 .. 255 do
            let substituted = affineTransform (multiplicativeInverse (byte value))
            sbox[value] <- substituted
            invSbox[int substituted] <- byte value

        sbox, invSbox

    let private sbox, invSbox = buildSBoxes ()

    let private validateBlock (block: byte array) =
        if isNull block then nullArg "block"
        if block.Length <> blockSizeBytes then
            invalidArg "block" "AES block must be exactly 16 bytes."

    let private validateKey (key: byte array) =
        if isNull key then nullArg "key"
        if key.Length <> 16 && key.Length <> 24 && key.Length <> 32 then
            invalidArg "key" "AES key must be 16, 24, or 32 bytes."

    let private rotWord (word: byte array) =
        let first = word[0]
        word[0] <- word[1]
        word[1] <- word[2]
        word[2] <- word[3]
        word[3] <- first

    let private subWord (word: byte array) =
        for index in 0 .. word.Length - 1 do
            word[index] <- sbox[int word[index]]

    let private roundsForKeyLength keyLength =
        match keyLength with
        | 16 -> 10
        | 24 -> 12
        | 32 -> 14
        | _ -> invalidArg "key" "AES key must be 16, 24, or 32 bytes."

    let private expandKey (key: byte array) =
        let rounds = roundsForKeyLength key.Length
        let expanded = Array.zeroCreate<byte> (blockSizeBytes * (rounds + 1))
        Buffer.BlockCopy(key, 0, expanded, 0, key.Length)
        let temp = Array.zeroCreate<byte> 4
        let mutable bytesGenerated = key.Length
        let mutable rconIndex = 1

        while bytesGenerated < expanded.Length do
            for index in 0 .. 3 do
                temp[index] <- expanded[bytesGenerated - 4 + index]

            if bytesGenerated % key.Length = 0 then
                rotWord temp
                subWord temp
                temp[0] <- temp[0] ^^^ rcon[rconIndex]
                rconIndex <- rconIndex + 1
            elif key.Length = 32 && bytesGenerated % key.Length = 16 then
                subWord temp

            for index in 0 .. 3 do
                expanded[bytesGenerated] <- expanded[bytesGenerated - key.Length] ^^^ temp[index]
                bytesGenerated <- bytesGenerated + 1

        expanded

    let private addRoundKey (state: byte array) (expandedKey: byte array) round =
        let offset = round * blockSizeBytes

        for index in 0 .. blockSizeBytes - 1 do
            state[index] <- state[index] ^^^ expandedKey[offset + index]

    let private subBytes (state: byte array) =
        for index in 0 .. state.Length - 1 do
            state[index] <- sbox[int state[index]]

    let private invSubBytes (state: byte array) =
        for index in 0 .. state.Length - 1 do
            state[index] <- invSbox[int state[index]]

    let private shiftRows (state: byte array) =
        let copy = Array.copy state

        for row in 0 .. 3 do
            for column in 0 .. 3 do
                state[row + 4 * column] <- copy[row + 4 * ((column + row) &&& 3)]

    let private invShiftRows (state: byte array) =
        let copy = Array.copy state

        for row in 0 .. 3 do
            for column in 0 .. 3 do
                state[row + 4 * column] <- copy[row + 4 * ((column - row + 4) &&& 3)]

    let private mixColumns (state: byte array) =
        for column in 0 .. 3 do
            let offset = 4 * column
            let s0 = state[offset]
            let s1 = state[offset + 1]
            let s2 = state[offset + 2]
            let s3 = state[offset + 3]

            state[offset] <- multiply 0x02uy s0 ^^^ multiply 0x03uy s1 ^^^ s2 ^^^ s3
            state[offset + 1] <- s0 ^^^ multiply 0x02uy s1 ^^^ multiply 0x03uy s2 ^^^ s3
            state[offset + 2] <- s0 ^^^ s1 ^^^ multiply 0x02uy s2 ^^^ multiply 0x03uy s3
            state[offset + 3] <- multiply 0x03uy s0 ^^^ s1 ^^^ s2 ^^^ multiply 0x02uy s3

    let private invMixColumns (state: byte array) =
        for column in 0 .. 3 do
            let offset = 4 * column
            let s0 = state[offset]
            let s1 = state[offset + 1]
            let s2 = state[offset + 2]
            let s3 = state[offset + 3]

            state[offset] <- multiply 0x0euy s0 ^^^ multiply 0x0buy s1 ^^^ multiply 0x0duy s2 ^^^ multiply 0x09uy s3
            state[offset + 1] <- multiply 0x09uy s0 ^^^ multiply 0x0euy s1 ^^^ multiply 0x0buy s2 ^^^ multiply 0x0duy s3
            state[offset + 2] <- multiply 0x0duy s0 ^^^ multiply 0x09uy s1 ^^^ multiply 0x0euy s2 ^^^ multiply 0x0buy s3
            state[offset + 3] <- multiply 0x0buy s0 ^^^ multiply 0x0duy s1 ^^^ multiply 0x09uy s2 ^^^ multiply 0x0euy s3

    let encryptBlock (block: byte array) (key: byte array) =
        validateBlock block
        validateKey key

        let expandedKey = expandKey key
        let rounds = expandedKey.Length / blockSizeBytes - 1
        let state = Array.copy block

        addRoundKey state expandedKey 0

        for round in 1 .. rounds - 1 do
            subBytes state
            shiftRows state
            mixColumns state
            addRoundKey state expandedKey round

        subBytes state
        shiftRows state
        addRoundKey state expandedKey rounds
        state

    let decryptBlock (block: byte array) (key: byte array) =
        validateBlock block
        validateKey key

        let expandedKey = expandKey key
        let rounds = expandedKey.Length / blockSizeBytes - 1
        let state = Array.copy block

        addRoundKey state expandedKey rounds

        for round in rounds - 1 .. -1 .. 1 do
            invShiftRows state
            invSubBytes state
            addRoundKey state expandedKey round
            invMixColumns state

        invShiftRows state
        invSubBytes state
        addRoundKey state expandedKey 0
        state
