namespace CodingAdventures.HashFunctions.FSharp

open System
open System.Numerics
open System.Text

type HashFunction =
    abstract Hash: byte array -> uint64
    abstract OutputBits: int

[<RequireQualifiedAccess>]
module HashFunctions =
    [<Literal>]
    let fnv32OffsetBasis = 0x811C9DC5u

    [<Literal>]
    let fnv32Prime = 0x01000193u

    [<Literal>]
    let fnv64OffsetBasis = 0xCBF29CE484222325UL

    [<Literal>]
    let fnv64Prime = 0x00000100000001B3UL

    [<Literal>]
    let djb2OffsetBasis = 5381UL

    [<Literal>]
    let polynomialRollingDefaultBase = 31UL

    [<Literal>]
    let polynomialRollingDefaultModulus = (1UL <<< 61) - 1UL

    let private murmurC1 = 0xCC9E2D51u
    let private murmurC2 = 0x1B873593u

    let private sipHashV0 = 0x736F6D6570736575UL
    let private sipHashV1 = 0x646F72616E646F6DUL
    let private sipHashV2 = 0x6C7967656E657261UL
    let private sipHashV3 = 0x7465646279746573UL

    let private requireBytes name (data: byte array) =
        if isNull data then
            nullArg name

    let private toBytes (text: string) =
        if isNull text then
            nullArg "text"
        Encoding.UTF8.GetBytes text

    let fnv1a32Bytes (data: byte array) =
        requireBytes "data" data
        let mutable hash = fnv32OffsetBasis
        for value in data do
            hash <- hash ^^^ uint32 value
            hash <- hash * fnv32Prime
        hash

    let fnv1a32 (text: string) =
        fnv1a32Bytes (toBytes text)

    let fnv1a64Bytes (data: byte array) =
        requireBytes "data" data
        let mutable hash = fnv64OffsetBasis
        for value in data do
            hash <- hash ^^^ uint64 value
            hash <- hash * fnv64Prime
        hash

    let fnv1a64 (text: string) =
        fnv1a64Bytes (toBytes text)

    let djb2Bytes (data: byte array) =
        requireBytes "data" data
        let mutable hash = djb2OffsetBasis
        for value in data do
            hash <- (hash <<< 5) + hash + uint64 value
        hash

    let djb2 (text: string) =
        djb2Bytes (toBytes text)

    let polynomialRollingBytes (data: byte array) (baseValue: uint64) (modulus: uint64) =
        requireBytes "data" data
        if modulus = 0UL then
            invalidArg "modulus" "Modulus must be positive."

        let mutable hash = BigInteger.Zero
        let b = BigInteger baseValue
        let m = BigInteger modulus
        for value in data do
            hash <- (hash * b + BigInteger(int value)) % m
        uint64 hash

    let polynomialRolling (text: string) =
        polynomialRollingBytes (toBytes text) polynomialRollingDefaultBase polynomialRollingDefaultModulus

    let polynomialRollingWithParams (text: string) (baseValue: uint64) (modulus: uint64) =
        polynomialRollingBytes (toBytes text) baseValue modulus

    let private rotateLeft32 (value: uint32) shift =
        (value <<< shift) ||| (value >>> (32 - shift))

    let private rotateLeft64 (value: uint64) shift =
        (value <<< shift) ||| (value >>> (64 - shift))

    let private fmix32 (input: uint32) =
        let mutable hash = input
        hash <- hash ^^^ (hash >>> 16)
        hash <- hash * 0x85EBCA6Bu
        hash <- hash ^^^ (hash >>> 13)
        hash <- hash * 0xC2B2AE35u
        hash <- hash ^^^ (hash >>> 16)
        hash

    let private nextSplitMix64 (state: byref<uint64>) =
        state <- state + 0x9E3779B97F4A7C15UL
        let mutable value = state
        value <- (value ^^^ (value >>> 30)) * 0xBF58476D1CE4E5B9UL
        value <- (value ^^^ (value >>> 27)) * 0x94D049BB133111EBUL
        value ^^^ (value >>> 31)

    let private fillDeterministicSample (input: byte array) (state: byref<uint64>) =
        let mutable offset = 0

        while offset < input.Length do
            let value = nextSplitMix64 &state
            let mutable index = 0

            while index < 8 && offset < input.Length do
                input[offset] <- byte ((value >>> (index * 8)) &&& 0xffUL)
                offset <- offset + 1
                index <- index + 1

    let murmur3_32BytesWithSeed (data: byte array) (seed: uint32) =
        requireBytes "data" data
        let mutable hash = seed
        let mutable offset = 0

        while offset + 4 <= data.Length do
            let mutable k =
                uint32 data.[offset]
                ||| (uint32 data.[offset + 1] <<< 8)
                ||| (uint32 data.[offset + 2] <<< 16)
                ||| (uint32 data.[offset + 3] <<< 24)

            k <- k * murmurC1
            k <- rotateLeft32 k 15
            k <- k * murmurC2

            hash <- hash ^^^ k
            hash <- rotateLeft32 hash 13
            hash <- hash * 5u + 0xE6546B64u
            offset <- offset + 4

        let remaining = data.Length - offset
        let mutable tail = 0u
        if remaining >= 3 then
            tail <- tail ^^^ (uint32 data.[offset + 2] <<< 16)
        if remaining >= 2 then
            tail <- tail ^^^ (uint32 data.[offset + 1] <<< 8)
        if remaining >= 1 then
            tail <- tail ^^^ uint32 data.[offset]
            tail <- tail * murmurC1
            tail <- rotateLeft32 tail 15
            tail <- tail * murmurC2
            hash <- hash ^^^ tail

        hash <- hash ^^^ uint32 data.Length
        fmix32 hash

    let murmur3_32Bytes (data: byte array) =
        murmur3_32BytesWithSeed data 0u

    let murmur3_32 (text: string) =
        murmur3_32Bytes (toBytes text)

    let murmur3_32WithSeed (text: string) (seed: uint32) =
        murmur3_32BytesWithSeed (toBytes text) seed

    let private readUInt64LittleEndian (data: byte array) offset =
        let mutable value = 0UL
        for index in 0 .. 7 do
            value <- value ||| (uint64 data.[offset + index] <<< (index * 8))
        value

    let private validateKey (key: byte array) =
        requireBytes "key" key
        if key.Length <> 16 then
            invalidArg "key" "SipHash key must be 16 bytes."

    let private sipRound (v0: byref<uint64>) (v1: byref<uint64>) (v2: byref<uint64>) (v3: byref<uint64>) =
        v0 <- v0 + v1
        v1 <- rotateLeft64 v1 13
        v1 <- v1 ^^^ v0
        v0 <- rotateLeft64 v0 32

        v2 <- v2 + v3
        v3 <- rotateLeft64 v3 16
        v3 <- v3 ^^^ v2

        v0 <- v0 + v3
        v3 <- rotateLeft64 v3 21
        v3 <- v3 ^^^ v0

        v2 <- v2 + v1
        v1 <- rotateLeft64 v1 17
        v1 <- v1 ^^^ v2
        v2 <- rotateLeft64 v2 32

    let sipHash24 (data: byte array) (key: byte array) =
        requireBytes "data" data
        validateKey key

        let k0 = readUInt64LittleEndian key 0
        let k1 = readUInt64LittleEndian key 8
        let mutable v0 = sipHashV0 ^^^ k0
        let mutable v1 = sipHashV1 ^^^ k1
        let mutable v2 = sipHashV2 ^^^ k0
        let mutable v3 = sipHashV3 ^^^ k1

        let mutable offset = 0
        while offset + 8 <= data.Length do
            let m = readUInt64LittleEndian data offset
            v3 <- v3 ^^^ m
            sipRound &v0 &v1 &v2 &v3
            sipRound &v0 &v1 &v2 &v3
            v0 <- v0 ^^^ m
            offset <- offset + 8

        let mutable last = (uint64 data.Length &&& 0xffUL) <<< 56
        for index in 0 .. data.Length - offset - 1 do
            last <- last ||| (uint64 data.[offset + index] <<< (index * 8))

        v3 <- v3 ^^^ last
        sipRound &v0 &v1 &v2 &v3
        sipRound &v0 &v1 &v2 &v3
        v0 <- v0 ^^^ last

        v2 <- v2 ^^^ 0xffUL
        sipRound &v0 &v1 &v2 &v3
        sipRound &v0 &v1 &v2 &v3
        sipRound &v0 &v1 &v2 &v3
        sipRound &v0 &v1 &v2 &v3

        v0 ^^^ v1 ^^^ v2 ^^^ v3

    let hashStringFnv1a32 (text: string) =
        fnv1a32 text

    let hashStringSipHash (text: string) (key: byte array) =
        sipHash24 (toBytes text) key

    let avalancheScore (hashFunction: byte array -> uint64) outputBits sampleSize =
        if isNull (box hashFunction) then
            nullArg "hashFunction"
        if outputBits < 1 || outputBits > 64 then
            invalidArg "outputBits" "Output bits must be in 1..=64."
        if sampleSize <= 0 then
            invalidArg "sampleSize" "Sample size must be positive."

        let input = Array.zeroCreate<byte> 8
        let mutable sampleState = 0x9E3779B97F4A7C15UL
        let mutable totalBitFlips = 0UL
        let mutable totalTrials = 0UL
        for _ in 1 .. sampleSize do
            fillDeterministicSample input &sampleState
            let h1 = hashFunction input

            for bitPosition in 0 .. input.Length * 8 - 1 do
                let flipped = Array.copy input
                flipped.[bitPosition / 8] <- flipped.[bitPosition / 8] ^^^ byte (1 <<< (bitPosition % 8))
                let h2 = hashFunction flipped
                totalBitFlips <- totalBitFlips + uint64 (BitOperations.PopCount(h1 ^^^ h2))
                totalTrials <- totalTrials + uint64 outputBits

        float totalBitFlips / float totalTrials

    let distributionTest (hashFunction: byte array -> uint64) (inputs: seq<byte array>) numBuckets =
        if isNull (box hashFunction) then
            nullArg "hashFunction"
        if isNull (box inputs) then
            nullArg "inputs"
        if numBuckets <= 0 then
            invalidArg "numBuckets" "Number of buckets must be positive."

        let counts = Array.zeroCreate<uint64> numBuckets
        let mutable total = 0UL
        for input in inputs do
            let bucket = int (hashFunction input % uint64 numBuckets)
            counts.[bucket] <- counts.[bucket] + 1UL
            total <- total + 1UL

        if total = 0UL then
            invalidArg "inputs" "Inputs must not be empty."

        let expected = float total / float numBuckets
        counts
        |> Array.sumBy (fun count ->
            let delta = float count - expected
            delta * delta / expected)

type Fnv1a32() =
    interface HashFunction with
        member _.Hash(data) = uint64 (HashFunctions.fnv1a32Bytes data)
        member _.OutputBits = 32

type Fnv1a64() =
    interface HashFunction with
        member _.Hash(data) = HashFunctions.fnv1a64Bytes data
        member _.OutputBits = 64

type Djb2Hash() =
    interface HashFunction with
        member _.Hash(data) = HashFunctions.djb2Bytes data
        member _.OutputBits = 64

type PolynomialRollingHash(?baseValue: uint64, ?modulus: uint64) =
    let baseValue = defaultArg baseValue HashFunctions.polynomialRollingDefaultBase
    let modulus = defaultArg modulus HashFunctions.polynomialRollingDefaultModulus

    interface HashFunction with
        member _.Hash(data) = HashFunctions.polynomialRollingBytes data baseValue modulus
        member _.OutputBits = 64

type Murmur3_32(?seed: uint32) =
    let seed = defaultArg seed 0u

    interface HashFunction with
        member _.Hash(data) = uint64 (HashFunctions.murmur3_32BytesWithSeed data seed)
        member _.OutputBits = 32

type SipHash24(key: byte array) =
    let key = Array.copy key

    interface HashFunction with
        member _.Hash(data) = HashFunctions.sipHash24 data key
        member _.OutputBits = 64
