namespace CodingAdventures.ChaCha20Poly1305.FSharp

open System
open System.Numerics
open System.Security.Cryptography

/// The result of ChaCha20-Poly1305 authenticated encryption.
type AeadResult =
    { Ciphertext: byte array
      Tag: byte array }

/// Pure F# ChaCha20-Poly1305 authenticated encryption from RFC 8439.
/// This educational implementation uses BigInteger for Poly1305 arithmetic,
/// which is not guaranteed to execute in constant time.
[<RequireQualifiedAccess>]
module ChaCha20Poly1305 =
    [<Literal>]
    let KeyLength = 32

    [<Literal>]
    let NonceLength = 12

    [<Literal>]
    let TagLength = 16

    [<Literal>]
    let private BlockLength = 64

    let private poly1305Prime = (BigInteger.One <<< 130) - 5I
    let private tagMask = (BigInteger.One <<< 128) - 1I

    let private validateLength expectedLength parameterName displayName (value: byte array) =
        if isNull value then
            nullArg parameterName

        if value.Length <> expectedLength then
            invalidArg parameterName $"{displayName} must be {expectedLength} bytes, got {value.Length}."

    let private readUInt32 (value: byte array) offset =
        uint32 value[offset]
        ||| (uint32 value[offset + 1] <<< 8)
        ||| (uint32 value[offset + 2] <<< 16)
        ||| (uint32 value[offset + 3] <<< 24)

    let private writeUInt32 (buffer: byte array) offset (value: uint32) =
        buffer[offset] <- byte value
        buffer[offset + 1] <- byte (value >>> 8)
        buffer[offset + 2] <- byte (value >>> 16)
        buffer[offset + 3] <- byte (value >>> 24)

    let private quarterRound (state: uint32 array) a b c d =
        state[a] <- state[a] + state[b]
        state[d] <- BitOperations.RotateLeft(state[d] ^^^ state[a], 16)
        state[c] <- state[c] + state[d]
        state[b] <- BitOperations.RotateLeft(state[b] ^^^ state[c], 12)
        state[a] <- state[a] + state[b]
        state[d] <- BitOperations.RotateLeft(state[d] ^^^ state[a], 8)
        state[c] <- state[c] + state[d]
        state[b] <- BitOperations.RotateLeft(state[b] ^^^ state[c], 7)

    /// Generate one 64-byte ChaCha20 keystream block.
    let chacha20Block (key: byte array) (counter: uint32) (nonce: byte array) =
        validateLength KeyLength "key" "Key" key
        validateLength NonceLength "nonce" "Nonce" nonce

        let state =
            [| 0x61707865u
               0x3320646eu
               0x79622d32u
               0x6b206574u
               readUInt32 key 0
               readUInt32 key 4
               readUInt32 key 8
               readUInt32 key 12
               readUInt32 key 16
               readUInt32 key 20
               readUInt32 key 24
               readUInt32 key 28
               counter
               readUInt32 nonce 0
               readUInt32 nonce 4
               readUInt32 nonce 8 |]

        let initial = Array.copy state

        for _ = 1 to 10 do
            quarterRound state 0 4 8 12
            quarterRound state 1 5 9 13
            quarterRound state 2 6 10 14
            quarterRound state 3 7 11 15
            quarterRound state 0 5 10 15
            quarterRound state 1 6 11 12
            quarterRound state 2 7 8 13
            quarterRound state 3 4 9 14

        let block = Array.zeroCreate<byte> BlockLength

        for index = 0 to state.Length - 1 do
            state[index] <- state[index] + initial[index]
            writeUInt32 block (index * 4) state[index]

        block

    /// Encrypt or decrypt bytes with the ChaCha20 stream cipher.
    let chacha20Encrypt
        (data: byte array)
        (key: byte array)
        (nonce: byte array)
        (counter: uint32)
        =
        if isNull data then
            nullArg "data"

        validateLength KeyLength "key" "Key" key
        validateLength NonceLength "nonce" "Nonce" nonce

        let result = Array.zeroCreate<byte> data.Length
        let mutable offset = 0
        let mutable currentCounter = counter

        while offset < data.Length do
            let keystream = chacha20Block key currentCounter nonce
            let chunkLength = min BlockLength (data.Length - offset)

            for index = 0 to chunkLength - 1 do
                result[offset + index] <- data[offset + index] ^^^ keystream[index]

            offset <- offset + chunkLength
            currentCounter <- currentCounter + 1u

        result

    let private decodeLittleEndian (value: byte array) offset count =
        let mutable result = BigInteger.Zero

        for index = offset + count - 1 downto offset do
            result <- (result <<< 8) ||| bigint value[index]

        result

    let private encodeLittleEndian length value =
        let result = Array.zeroCreate<byte> length
        let mutable remaining = value

        for index = 0 to length - 1 do
            result[index] <- byte (remaining &&& 255I)
            remaining <- remaining >>> 8

        result

    /// Compute a 16-byte Poly1305 one-time authenticator.
    let poly1305Mac (message: byte array) (key: byte array) =
        if isNull message then
            nullArg "message"

        validateLength KeyLength "key" "Poly1305 key" key

        let rBytes = key[..15]
        rBytes[3] <- rBytes[3] &&& 0x0fuy
        rBytes[7] <- rBytes[7] &&& 0x0fuy
        rBytes[11] <- rBytes[11] &&& 0x0fuy
        rBytes[15] <- rBytes[15] &&& 0x0fuy
        rBytes[4] <- rBytes[4] &&& 0xfcuy
        rBytes[8] <- rBytes[8] &&& 0xfcuy
        rBytes[12] <- rBytes[12] &&& 0xfcuy

        let r = decodeLittleEndian rBytes 0 rBytes.Length
        let s = decodeLittleEndian key 16 16
        let mutable accumulator = BigInteger.Zero
        let mutable offset = 0

        while offset < message.Length do
            let chunkLength = min 16 (message.Length - offset)
            let augmented = Array.zeroCreate<byte> (chunkLength + 1)
            Array.blit message offset augmented 0 chunkLength
            augmented[chunkLength] <- 1uy
            let block = decodeLittleEndian augmented 0 augmented.Length
            accumulator <- ((accumulator + block) * r) % poly1305Prime
            offset <- offset + chunkLength

        encodeLittleEndian TagLength ((accumulator + s) &&& tagMask)

    let private paddingLength length = (16 - (length % 16)) % 16

    let private writeUInt64 (buffer: byte array) offset (value: uint64) =
        for index = 0 to 7 do
            buffer[offset + index] <- byte (value >>> (index * 8))

    let private buildMacData (additionalData: byte array) (ciphertext: byte array) =
        let aadPadding = paddingLength additionalData.Length
        let ciphertextPadding = paddingLength ciphertext.Length

        let result =
            Array.zeroCreate<byte>
                (additionalData.Length + aadPadding + ciphertext.Length + ciphertextPadding + 16)

        let mutable offset = 0
        Array.blit additionalData 0 result offset additionalData.Length
        offset <- offset + additionalData.Length + aadPadding
        Array.blit ciphertext 0 result offset ciphertext.Length
        offset <- offset + ciphertext.Length + ciphertextPadding
        writeUInt64 result offset (uint64 additionalData.Length)
        writeUInt64 result (offset + 8) (uint64 ciphertext.Length)
        result

    let private constantTimeEquals (left: byte array) (right: byte array) =
        if left.Length <> right.Length then
            false
        else
            let mutable difference = 0

            for index = 0 to left.Length - 1 do
                difference <- difference ||| int (left[index] ^^^ right[index])

            difference = 0

    /// Encrypt and authenticate data using RFC 8439 AEAD.
    let aeadEncrypt
        (plaintext: byte array)
        (key: byte array)
        (nonce: byte array)
        (additionalData: byte array)
        =
        if isNull plaintext then
            nullArg "plaintext"

        validateLength KeyLength "key" "Key" key
        validateLength NonceLength "nonce" "Nonce" nonce
        let aad = if isNull additionalData then Array.empty else additionalData
        let polyKey = (chacha20Block key 0u nonce)[.. KeyLength - 1]
        let ciphertext = chacha20Encrypt plaintext key nonce 1u
        let tag = poly1305Mac (buildMacData aad ciphertext) polyKey

        { Ciphertext = ciphertext
          Tag = tag }

    /// Authenticate and decrypt RFC 8439 AEAD ciphertext.
    let aeadDecrypt
        (ciphertext: byte array)
        (key: byte array)
        (nonce: byte array)
        (additionalData: byte array)
        (tag: byte array)
        =
        if isNull ciphertext then
            nullArg "ciphertext"

        validateLength KeyLength "key" "Key" key
        validateLength NonceLength "nonce" "Nonce" nonce
        validateLength TagLength "tag" "Tag" tag
        let aad = if isNull additionalData then Array.empty else additionalData
        let polyKey = (chacha20Block key 0u nonce)[.. KeyLength - 1]
        let expectedTag = poly1305Mac (buildMacData aad ciphertext) polyKey

        if not (constantTimeEquals expectedTag tag) then
            raise (CryptographicException("Authentication failed: tag mismatch."))

        chacha20Encrypt ciphertext key nonce 1u
