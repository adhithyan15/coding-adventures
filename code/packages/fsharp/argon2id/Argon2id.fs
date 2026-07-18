namespace CodingAdventures.Argon2id.FSharp

open System
open System.Buffers.Binary
open System.Numerics
open CodingAdventures.Blake2b.FSharp

/// Optional Argon2id inputs from RFC 9106.
type Argon2idOptions =
    {
        Key: byte array
        AssociatedData: byte array
        Version: uint32
    }

    static member Default =
        {
            Key = [||]
            AssociatedData = [||]
            Version = 0x13u
        }

/// Pure F# Argon2id password hashing as specified by RFC 9106.
[<RequireQualifiedAccess>]
module Argon2id =
    [<Literal>]
    let Version = 0x13u

    [<Literal>]
    let private Mask32 = 0xffff_ffffUL

    [<Literal>]
    let private BlockSize = 1024

    [<Literal>]
    let private BlockWords = 128

    [<Literal>]
    let private SyncPoints = 4

    [<Literal>]
    let private TypeId = 2u

    let private uint32Bytes value =
        let result = Array.zeroCreate<byte> sizeof<uint32>
        BinaryPrimitives.WriteUInt32LittleEndian(result.AsSpan(), value)
        result

    let private concat (parts: byte array array) = Array.concat parts

    let private hash digestSize input =
        Blake2b.hashWithOptions (Blake2bOptions.Default.WithDigestSize(digestSize)) input

    let private addWithProduct left right =
        left + right + 2UL * (left &&& Mask32) * (right &&& Mask32)

    let private mix (values: uint64 array) a b c d =
        let mutable va = values[a]
        let mutable vb = values[b]
        let mutable vc = values[c]
        let mutable vd = values[d]

        va <- addWithProduct va vb
        vd <- BitOperations.RotateRight(vd ^^^ va, 32)
        vc <- addWithProduct vc vd
        vb <- BitOperations.RotateRight(vb ^^^ vc, 24)
        va <- addWithProduct va vb
        vd <- BitOperations.RotateRight(vd ^^^ va, 16)
        vc <- addWithProduct vc vd
        vb <- BitOperations.RotateRight(vb ^^^ vc, 63)

        values[a] <- va
        values[b] <- vb
        values[c] <- vc
        values[d] <- vd

    let private permute values =
        mix values 0 4 8 12
        mix values 1 5 9 13
        mix values 2 6 10 14
        mix values 3 7 11 15
        mix values 0 5 10 15
        mix values 1 6 11 12
        mix values 2 7 8 13
        mix values 3 4 9 14

    let private compress (x: uint64 array) (y: uint64 array) =
        let result = Array.init BlockWords (fun index -> x[index] ^^^ y[index])
        let q = Array.copy result

        for rowIndex = 0 to 7 do
            let row = Array.zeroCreate<uint64> 16
            Array.Copy(q, rowIndex * 16, row, 0, row.Length)
            permute row
            Array.Copy(row, 0, q, rowIndex * 16, row.Length)

        for columnIndex = 0 to 7 do
            let column = Array.zeroCreate<uint64> 16

            for rowIndex = 0 to 7 do
                column[2 * rowIndex] <- q[rowIndex * 16 + 2 * columnIndex]
                column[2 * rowIndex + 1] <- q[rowIndex * 16 + 2 * columnIndex + 1]

            permute column

            for rowIndex = 0 to 7 do
                q[rowIndex * 16 + 2 * columnIndex] <- column[2 * rowIndex]
                q[rowIndex * 16 + 2 * columnIndex + 1] <- column[2 * rowIndex + 1]

        for index = 0 to BlockWords - 1 do
            result[index] <- result[index] ^^^ q[index]

        result

    let private blockToBytes (block: uint64 array) =
        let result = Array.zeroCreate<byte> BlockSize

        for index = 0 to BlockWords - 1 do
            BinaryPrimitives.WriteUInt64LittleEndian(
                result.AsSpan(index * sizeof<uint64>, sizeof<uint64>),
                block[index]
            )

        result

    let private bytesToBlock (data: byte array) =
        if data.Length <> BlockSize then
            invalidArg "data" $"Block must be {BlockSize} bytes, got {data.Length}."

        Array.init BlockWords (fun index ->
            BinaryPrimitives.ReadUInt64LittleEndian(data.AsSpan(index * sizeof<uint64>, sizeof<uint64>)))

    let private blake2bLong outputLength input =
        let prefix = uint32Bytes (uint32 outputLength)

        if outputLength <= 64 then
            hash outputLength (concat [| prefix; input |])
        else
            let rounds = (outputLength + 31) / 32 - 2
            let mutable value = hash 64 (concat [| prefix; input |])
            let output = ResizeArray<byte>(outputLength)
            output.AddRange(value[0..31])

            for _round = 0 to rounds - 2 do
                value <- hash 64 value
                output.AddRange(value[0..31])

            value <- hash (outputLength - 32 * rounds) value
            output.AddRange(value)
            output.ToArray()

    let private indexAlpha j1 pass slice index sameLane laneLength segmentLength =
        let mutable start = 0

        let window =
            if pass = 0 then
                if slice = 0 then
                    uint64 (index - 1)
                elif sameLane then
                    uint64 (slice * segmentLength + index - 1)
                else
                    uint64 (slice * segmentLength - (if index = 0 then 1 else 0))
            else
                start <- ((slice + 1) * segmentLength) % laneLength

                if sameLane then
                    uint64 (laneLength - segmentLength + index - 1)
                else
                    uint64 (laneLength - segmentLength - (if index = 0 then 1 else 0))

        let x = (j1 * j1) >>> 32
        let y = (window * x) >>> 32
        let relative = window - 1UL - y
        int ((uint64 start + relative) % uint64 laneLength)

    let private fillSegment
        (memory: uint64 array array array)
        pass
        lane
        slice
        laneLength
        segmentLength
        parallelism
        adjustedMemoryCost
        timeCost
        =
        let dataIndependent = pass = 0 && slice < 2
        let inputBlock = Array.zeroCreate<uint64> BlockWords
        let mutable addressBlock = Array.zeroCreate<uint64> BlockWords
        let zeroBlock = Array.zeroCreate<uint64> BlockWords
        inputBlock[0] <- uint64 pass
        inputBlock[1] <- uint64 lane
        inputBlock[2] <- uint64 slice
        inputBlock[3] <- uint64 adjustedMemoryCost
        inputBlock[4] <- uint64 timeCost
        inputBlock[5] <- uint64 TypeId

        let nextAddresses () =
            inputBlock[6] <- inputBlock[6] + 1UL
            let intermediate = compress zeroBlock inputBlock
            addressBlock <- compress zeroBlock intermediate

        let startingColumn = if pass = 0 && slice = 0 then 2 else 0

        if dataIndependent && startingColumn <> 0 then
            nextAddresses ()

        for index = startingColumn to segmentLength - 1 do
            if
                dataIndependent
                && index % BlockWords = 0
                && not (pass = 0 && slice = 0 && index = 2)
            then
                nextAddresses ()

            let column = slice * segmentLength + index
            let previousColumn = if column > 0 then column - 1 else laneLength - 1
            let previousBlock = memory.[lane].[previousColumn]

            let pseudoRandom =
                if dataIndependent then
                    addressBlock[index % BlockWords]
                else
                    previousBlock[0]

            let j1 = pseudoRandom &&& Mask32
            let j2 = pseudoRandom >>> 32

            let referenceLane =
                if pass = 0 && slice = 0 then lane else int (j2 % uint64 parallelism)

            let referenceColumn =
                indexAlpha
                    j1
                    pass
                    slice
                    index
                    (referenceLane = lane)
                    laneLength
                    segmentLength

            let newBlock = compress previousBlock memory.[referenceLane].[referenceColumn]

            if pass = 0 then
                memory.[lane].[column] <- newBlock
            else
                for word = 0 to BlockWords - 1 do
                    memory.[lane].[column].[word] <- memory.[lane].[column].[word] ^^^ newBlock[word]

    let private validate (salt: byte array) timeCost memoryCost parallelism tagLength (version: uint32) =
        if salt.Length < 8 then
            invalidArg "salt" $"Salt must be at least 8 bytes, got {salt.Length}."

        if tagLength < 4 then
            invalidArg "tagLength" $"Tag length must be at least 4 bytes, got {tagLength}."

        if parallelism < 1 || parallelism > 0x00ff_ffff then
            invalidArg "parallelism" "Parallelism must be in [1, 2^24-1]."

        if memoryCost < 8 * parallelism then
            invalidArg
                "memoryCost"
                $"Memory cost must be at least 8*parallelism ({8 * parallelism}), got {memoryCost}."

        if timeCost < 1 then
            invalidArg "timeCost" "Time cost must be at least 1."

        if version <> Version then
            invalidArg "version" $"Only Argon2 v1.3 (0x13) is supported; got 0x{version:x2}."

    let private initialHash
        (password: byte array)
        (salt: byte array)
        timeCost
        memoryCost
        parallelism
        tagLength
        (options: Argon2idOptions)
        =
        concat
            [|
                uint32Bytes (uint32 parallelism)
                uint32Bytes (uint32 tagLength)
                uint32Bytes (uint32 memoryCost)
                uint32Bytes (uint32 timeCost)
                uint32Bytes options.Version
                uint32Bytes TypeId
                uint32Bytes (uint32 password.Length)
                password
                uint32Bytes (uint32 salt.Length)
                salt
                uint32Bytes (uint32 options.Key.Length)
                options.Key
                uint32Bytes (uint32 options.AssociatedData.Length)
                options.AssociatedData
            |]
        |> hash 64

    /// Compute an Argon2id tag.
    let derive
        (password: byte array)
        (salt: byte array)
        timeCost
        memoryCost
        parallelism
        tagLength
        (options: Argon2idOptions)
        =
        if isNull password then nullArg "password"
        if isNull salt then nullArg "salt"
        if obj.ReferenceEquals(options, null) then nullArg "options"
        if isNull options.Key then nullArg "options.Key"
        if isNull options.AssociatedData then nullArg "options.AssociatedData"

        validate salt timeCost memoryCost parallelism tagLength options.Version

        let segmentLength = memoryCost / (SyncPoints * parallelism)
        let adjustedMemoryCost = segmentLength * SyncPoints * parallelism
        let laneLength = adjustedMemoryCost / parallelism
        let initial = initialHash password salt timeCost memoryCost parallelism tagLength options

        let memory =
            Array.init parallelism (fun _ ->
                Array.init laneLength (fun _ -> Array.zeroCreate<uint64> BlockWords))

        for lane = 0 to parallelism - 1 do
            memory.[lane].[0] <-
                concat [| initial; uint32Bytes 0u; uint32Bytes (uint32 lane) |]
                |> blake2bLong BlockSize
                |> bytesToBlock

            memory.[lane].[1] <-
                concat [| initial; uint32Bytes 1u; uint32Bytes (uint32 lane) |]
                |> blake2bLong BlockSize
                |> bytesToBlock

        for pass = 0 to timeCost - 1 do
            for slice = 0 to SyncPoints - 1 do
                for lane = 0 to parallelism - 1 do
                    fillSegment
                        memory
                        pass
                        lane
                        slice
                        laneLength
                        segmentLength
                        parallelism
                        adjustedMemoryCost
                        timeCost

        let finalBlock = Array.copy memory.[0].[laneLength - 1]

        for lane = 1 to parallelism - 1 do
            for word = 0 to BlockWords - 1 do
                finalBlock[word] <- finalBlock[word] ^^^ memory.[lane].[laneLength - 1].[word]

        finalBlock |> blockToBytes |> blake2bLong tagLength

    /// Compute an Argon2id tag with empty key and associated data.
    let deriveDefault password salt timeCost memoryCost parallelism tagLength =
        derive password salt timeCost memoryCost parallelism tagLength Argon2idOptions.Default

    /// Compute an Argon2id tag and return lowercase hexadecimal.
    let deriveHex password salt timeCost memoryCost parallelism tagLength options =
        derive password salt timeCost memoryCost parallelism tagLength options
        |> Convert.ToHexString
        |> fun value -> value.ToLowerInvariant()

    /// Compute a default-option Argon2id tag and return lowercase hexadecimal.
    let deriveHexDefault password salt timeCost memoryCost parallelism tagLength =
        deriveHex password salt timeCost memoryCost parallelism tagLength Argon2idOptions.Default
