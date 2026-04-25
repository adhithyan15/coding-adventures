namespace CodingAdventures.BloomFilter

open System
open System.Text

module private Hashing =
    let fnv1a32 (data: byte array) =
        let mutable hash = 0x811C9DC5u

        for value in data do
            hash <- hash ^^^ uint32 value
            hash <- hash * 0x01000193u

        hash

    let djb2_32 (data: byte array) =
        let mutable hash = 5381UL

        for value in data do
            hash <- (hash <<< 5) + hash + uint64 value

        uint32 ((hash ^^^ (hash >>> 32)) &&& 0xFFFFFFFFUL)

    let fmix32 (value: uint32) =
        let mutable hash = value
        hash <- hash ^^^ (hash >>> 16)
        hash <- hash * 0x85EBCA6Bu
        hash <- hash ^^^ (hash >>> 13)
        hash <- hash * 0xC2B2AE35u
        hash <- hash ^^^ (hash >>> 16)
        hash

type BloomFilter<'T> private (bitCount: int, hashCount: int, expectedItems: int) =
    static let maxBits = 1 <<< 30

    let bits = Array.zeroCreate<byte> ((bitCount + 7) / 8)
    let mutable bitsSet = 0
    let mutable count = 0

    do
        if bitCount <= 0 then
            invalidArg (nameof bitCount) "Bit count must be positive."

        if hashCount <= 0 then
            invalidArg (nameof hashCount) "Hash count must be positive."

        if bitCount > maxBits then
            invalidArg (nameof bitCount) "Bit count exceeds the maximum size."

    new (expectedItems: int, falsePositiveRate: float) =
        if expectedItems <= 0 then
            invalidArg (nameof expectedItems) "Expected items must be positive."

        if falsePositiveRate <= 0.0 || falsePositiveRate >= 1.0 then
            invalidArg (nameof falsePositiveRate) "False-positive rate must be in (0, 1)."

        let bitCount = BloomFilter<'T>.OptimalM(int64 expectedItems, falsePositiveRate)

        if bitCount > int64 maxBits then
            invalidArg (nameof expectedItems) "Required bit array exceeds the maximum size."

        let hashCount = BloomFilter<'T>.OptimalK(bitCount, int64 expectedItems)
        BloomFilter<'T>(int bitCount, hashCount, expectedItems)

    new (bitCount: int, hashCount: int, explicitParameters: bool) =
        BloomFilter<'T>(bitCount, hashCount, 0)

    member _.BitCount = bitCount
    member _.HashCount = hashCount
    member _.BitsSet = bitsSet
    member _.Count = count
    member _.Size = count
    member _.SizeBytes = bits.Length
    member _.FillRatio = float bitsSet / float bitCount

    member this.EstimatedFalsePositiveRate =
        if bitsSet = 0 then
            0.0
        else
            Math.Pow(this.FillRatio, hashCount)

    member _.IsOverCapacity =
        expectedItems <> 0 && count > expectedItems

    member private _.HashIndices(element: 'T) =
        if Object.ReferenceEquals(box element, null) then
            nullArg (nameof element)

        let raw = Encoding.UTF8.GetBytes(element.ToString())
        let h1 = Hashing.fmix32 (Hashing.fnv1a32 raw) |> uint64
        let h2 = (Hashing.fmix32 (Hashing.djb2_32 raw) ||| 1u) |> uint64

        Array.init hashCount (fun index ->
            int ((h1 + uint64 index * h2) % uint64 bitCount))

    member this.Add(element: 'T) =
        for index in this.HashIndices(element) do
            let byteIndex = index >>> 3
            let bitMask = byte (1 <<< (index &&& 7))

            if bits[byteIndex] &&& bitMask = 0uy then
                bits[byteIndex] <- bits[byteIndex] ||| bitMask
                bitsSet <- bitsSet + 1

        count <- count + 1

    member this.Contains(element: 'T) =
        this.HashIndices(element)
        |> Array.forall (fun index ->
            let byteIndex = index >>> 3
            let bitMask = byte (1 <<< (index &&& 7))
            bits[byteIndex] &&& bitMask <> 0uy)

    static member OptimalM(n: int64, p: float) =
        if n <= 0L then
            invalidArg (nameof n) "n must be positive."

        if p <= 0.0 || p >= 1.0 then
            invalidArg (nameof p) "p must be in (0, 1)."

        let ln2 = Math.Log 2.0
        int64 (Math.Ceiling(-float n * Math.Log(p) / (ln2 * ln2)))

    static member OptimalK(m: int64, n: int64) =
        if n <= 0L then
            invalidArg (nameof n) "n must be positive."

        max 1 (int (Math.Round(float m / float n * Math.Log 2.0)))

    static member CapacityForMemory(memoryBytes: int64, p: float) =
        if memoryBytes <= 0L then
            invalidArg (nameof memoryBytes) "Memory budget must be positive."

        if p <= 0.0 || p >= 1.0 then
            invalidArg (nameof p) "p must be in (0, 1)."

        let bits = float memoryBytes * 8.0
        let ln2 = Math.Log 2.0
        int64 (-bits * ln2 * ln2 / Math.Log p)

    override this.ToString() =
        $"BloomFilter(m={bitCount}, k={hashCount}, bitsSet={bitsSet}/{bitCount} ({this.FillRatio * 100.0:F2}%%), ~fp={this.EstimatedFalsePositiveRate * 100.0:F4}%%)"
