namespace CodingAdventures.Blake2b.FSharp

open System
open System.Buffers.Binary
open System.Collections.Generic
open System.Numerics

/// BLAKE2b parameter block options for sequential mode.
type Blake2bOptions =
    {
        /// Requested digest length in bytes.
        DigestSize: int
        /// Optional key for MAC mode.
        Key: byte array
        /// Optional 16-byte salt.
        Salt: byte array
        /// Optional 16-byte personalization string.
        Personal: byte array
    }

    /// Default BLAKE2b options: 64-byte digest, unkeyed, no salt, no personalization.
    static member Default =
        {
            DigestSize = 64
            Key = [||]
            Salt = [||]
            Personal = [||]
        }

    /// Return a copy with a different digest size.
    member this.WithDigestSize(digestSize: int) =
        { this with DigestSize = digestSize }

    /// Return a copy with a different key.
    member this.WithKey(key: byte array) =
        if isNull key then
            nullArg "key"

        { this with Key = Array.copy key }

    /// Return a copy with a different salt.
    member this.WithSalt(salt: byte array) =
        if isNull salt then
            nullArg "salt"

        { this with Salt = Array.copy salt }

    /// Return a copy with a different personalization string.
    member this.WithPersonal(personal: byte array) =
        if isNull personal then
            nullArg "personal"

        { this with Personal = Array.copy personal }

module private Blake2bCore =
    [<Literal>]
    let blockSize = 128

    [<Literal>]
    let maxDigestLength = 64

    [<Literal>]
    let maxKeyLength = 64

    let private iv =
        [|
            0x6A09_E667_F3BC_C908UL
            0xBB67_AE85_84CA_A73BUL
            0x3C6E_F372_FE94_F82BUL
            0xA54F_F53A_5F1D_36F1UL
            0x510E_527F_ADE6_82D1UL
            0x9B05_688C_2B3E_6C1FUL
            0x1F83_D9AB_FB41_BD6BUL
            0x5BE0_CD19_137E_2179UL
        |]

    let private sigma =
        [|
            [| 0; 1; 2; 3; 4; 5; 6; 7; 8; 9; 10; 11; 12; 13; 14; 15 |]
            [| 14; 10; 4; 8; 9; 15; 13; 6; 1; 12; 0; 2; 11; 7; 5; 3 |]
            [| 11; 8; 12; 0; 5; 2; 15; 13; 10; 14; 3; 6; 7; 1; 9; 4 |]
            [| 7; 9; 3; 1; 13; 12; 11; 14; 2; 6; 5; 10; 4; 0; 15; 8 |]
            [| 9; 0; 5; 7; 2; 4; 10; 15; 14; 1; 11; 12; 6; 8; 3; 13 |]
            [| 2; 12; 6; 10; 0; 11; 8; 3; 4; 13; 7; 5; 15; 14; 1; 9 |]
            [| 12; 5; 1; 15; 14; 13; 4; 10; 0; 7; 6; 3; 9; 2; 8; 11 |]
            [| 13; 11; 7; 14; 12; 1; 3; 9; 5; 0; 15; 4; 8; 6; 2; 10 |]
            [| 6; 15; 14; 9; 11; 3; 0; 8; 12; 2; 13; 7; 1; 4; 10; 5 |]
            [| 10; 2; 8; 4; 7; 6; 1; 5; 15; 11; 9; 14; 3; 12; 13; 0 |]
        |]

    let private copyOrEmpty (bytes: byte array) =
        if isNull bytes then
            [||]
        else
            Array.copy bytes

    let normalizeAndValidate (options: Blake2bOptions) =
        if Object.ReferenceEquals(options, null) then
            nullArg "options"

        let normalized =
            {
                DigestSize = options.DigestSize
                Key = copyOrEmpty options.Key
                Salt = copyOrEmpty options.Salt
                Personal = copyOrEmpty options.Personal
            }

        if normalized.DigestSize < 1 || normalized.DigestSize > maxDigestLength then
            raise (ArgumentOutOfRangeException("options", normalized.DigestSize, "Digest size must be in [1, 64]."))

        if normalized.Key.Length > maxKeyLength then
            invalidArg "options" "Key length must be in [0, 64]."

        if normalized.Salt.Length <> 0 && normalized.Salt.Length <> 16 then
            invalidArg "options" "Salt must be empty or exactly 16 bytes."

        if normalized.Personal.Length <> 0 && normalized.Personal.Length <> 16 then
            invalidArg "options" "Personal must be empty or exactly 16 bytes."

        normalized

    let initialState (options: Blake2bOptions) =
        let parameter = Array.zeroCreate<byte> 64
        parameter[0] <- byte options.DigestSize
        parameter[1] <- byte options.Key.Length
        parameter[2] <- 1uy
        parameter[3] <- 1uy

        if options.Salt.Length > 0 then
            Array.Copy(options.Salt, 0, parameter, 32, 16)

        if options.Personal.Length > 0 then
            Array.Copy(options.Personal, 0, parameter, 48, 16)

        let state = Array.copy iv

        for index in 0 .. state.Length - 1 do
            state[index] <- state[index] ^^^ BinaryPrimitives.ReadUInt64LittleEndian(parameter.AsSpan(index * 8, 8))

        state

    let addToCount (low: uint64) (high: uint64) (value: uint64) =
        let newLow = low + value
        let newHigh =
            if newLow < low then
                high + 1UL
            else
                high

        newLow, newHigh

    let private mix (v: uint64 array) a b c d x y =
        v[a] <- v[a] + v[b] + x
        v[d] <- BitOperations.RotateRight(v[d] ^^^ v[a], 32)
        v[c] <- v[c] + v[d]
        v[b] <- BitOperations.RotateRight(v[b] ^^^ v[c], 24)
        v[a] <- v[a] + v[b] + y
        v[d] <- BitOperations.RotateRight(v[d] ^^^ v[a], 16)
        v[c] <- v[c] + v[d]
        v[b] <- BitOperations.RotateRight(v[b] ^^^ v[c], 63)

    let compress (state: uint64 array) (block: ReadOnlySpan<byte>) (counterLow: uint64) (counterHigh: uint64) (isFinal: bool) =
        let m = Array.zeroCreate<uint64> 16

        for index in 0 .. m.Length - 1 do
            m[index] <- BinaryPrimitives.ReadUInt64LittleEndian(block.Slice(index * 8, 8))

        let v = Array.zeroCreate<uint64> 16

        for index in 0 .. 7 do
            v[index] <- state[index]
            v[index + 8] <- iv[index]

        v[12] <- v[12] ^^^ counterLow
        v[13] <- v[13] ^^^ counterHigh

        if isFinal then
            v[14] <- v[14] ^^^ UInt64.MaxValue

        for round in 0 .. 11 do
            let s = sigma[round % 10]
            mix v 0 4 8 12 m[s[0]] m[s[1]]
            mix v 1 5 9 13 m[s[2]] m[s[3]]
            mix v 2 6 10 14 m[s[4]] m[s[5]]
            mix v 3 7 11 15 m[s[6]] m[s[7]]
            mix v 0 5 10 15 m[s[8]] m[s[9]]
            mix v 1 6 11 12 m[s[10]] m[s[11]]
            mix v 2 7 8 13 m[s[12]] m[s[13]]
            mix v 3 4 9 14 m[s[14]] m[s[15]]

        for index in 0 .. 7 do
            state[index] <- state[index] ^^^ v[index] ^^^ v[index + 8]

    let writeDigest (state: uint64 array) (destination: Span<byte>) =
        let full = Array.zeroCreate<byte> maxDigestLength

        for index in 0 .. state.Length - 1 do
            BinaryPrimitives.WriteUInt64LittleEndian(full.AsSpan(index * 8, 8), state[index])

        full.AsSpan(0, destination.Length).CopyTo(destination)

/// Streaming BLAKE2b hasher with non-destructive digest snapshots.
type Blake2bHasher private (initialState: uint64 array, initialBuffer: byte array, initialLow: uint64, initialHigh: uint64, digestSize: int) =
    let state = Array.copy initialState
    let buffer = List<byte>(initialBuffer)
    let mutable counterLow = initialLow
    let mutable counterHigh = initialHigh

    /// Create a hasher with optional BLAKE2b parameters.
    new(?options: Blake2bOptions) =
        let normalized = defaultArg options Blake2bOptions.Default |> Blake2bCore.normalizeAndValidate
        let state = Blake2bCore.initialState normalized

        let initialBuffer =
            if normalized.Key.Length = 0 then
                [||]
            else
                let keyBlock = Array.zeroCreate<byte> Blake2bCore.blockSize
                Array.Copy(normalized.Key, keyBlock, normalized.Key.Length)
                keyBlock

        Blake2bHasher(state, initialBuffer, 0UL, 0UL, normalized.DigestSize)

    /// Append bytes and return this hasher for chaining.
    member this.Update(data: byte array) =
        if isNull data then
            nullArg "data"

        buffer.AddRange(data)

        while buffer.Count > Blake2bCore.blockSize do
            let nextLow, nextHigh = Blake2bCore.addToCount counterLow counterHigh (uint64 Blake2bCore.blockSize)
            counterLow <- nextLow
            counterHigh <- nextHigh

            let block = Array.zeroCreate<byte> Blake2bCore.blockSize
            buffer.CopyTo(0, block, 0, Blake2bCore.blockSize)
            Blake2bCore.compress state (ReadOnlySpan<byte>(block)) counterLow counterHigh false
            buffer.RemoveRange(0, Blake2bCore.blockSize)

        this

    /// Return the current digest without modifying this hasher.
    member _.Digest() =
        let snapshot = Array.copy state
        let finalBlock = Array.zeroCreate<byte> Blake2bCore.blockSize
        buffer.CopyTo(finalBlock)

        let finalLow, finalHigh = Blake2bCore.addToCount counterLow counterHigh (uint64 buffer.Count)
        Blake2bCore.compress snapshot (ReadOnlySpan<byte>(finalBlock)) finalLow finalHigh true

        let digest = Array.zeroCreate<byte> digestSize
        Blake2bCore.writeDigest snapshot (digest.AsSpan())
        digest

    /// Return the current digest as lowercase hexadecimal text.
    member this.HexDigest() =
        this.Digest() |> Convert.ToHexString |> fun value -> value.ToLowerInvariant()

    /// Return an independent copy of the current hasher state.
    member _.Copy() =
        Blake2bHasher(state, buffer.ToArray(), counterLow, counterHigh, digestSize)

[<RequireQualifiedAccess>]
module Blake2b =
    /// BLAKE2b block size in bytes.
    [<Literal>]
    let BlockSize = 128

    /// Maximum BLAKE2b digest length in bytes.
    [<Literal>]
    let MaxDigestLength = 64

    /// Maximum BLAKE2b key length in bytes.
    [<Literal>]
    let MaxKeyLength = 64

    /// Compute a BLAKE2b digest for a complete byte array with explicit options.
    let hashWithOptions (options: Blake2bOptions) (data: byte array) =
        if isNull data then
            nullArg "data"

        let hasher = Blake2bHasher(options)
        hasher.Update(data) |> ignore
        hasher.Digest()

    /// Compute a BLAKE2b digest for a complete byte array.
    let hash (data: byte array) =
        hashWithOptions Blake2bOptions.Default data

    /// Compute BLAKE2b and return a lowercase hexadecimal digest with explicit options.
    let hashHexWithOptions (options: Blake2bOptions) (data: byte array) =
        hashWithOptions options data
        |> Convert.ToHexString
        |> fun value -> value.ToLowerInvariant()

    /// Compute BLAKE2b and return a lowercase hexadecimal digest.
    let hashHex (data: byte array) =
        hashHexWithOptions Blake2bOptions.Default data
