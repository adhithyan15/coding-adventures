namespace CodingAdventures.Bitset

open System
open System.Collections
open System.Collections.Generic
open System.Numerics

module private BitsetHelpers =
    [<Literal>]
    let BitsPerWord = 64

    let wordsNeeded bitCount =
        if bitCount = 0 then
            0
        else
            ((bitCount - 1) / BitsPerWord) + 1

    let wordIndex index = index / BitsPerWord

    let bitmask index = 1UL <<< (index % BitsPerWord)

    let trailingMask bitCount =
        if bitCount = BitsPerWord then
            UInt64.MaxValue
        else
            (1UL <<< bitCount) - 1UL

    let validateIndex index =
        if index < 0 then
            raise (ArgumentOutOfRangeException("index", "Bit indices cannot be negative."))

// Bitset.fs -- Dense boolean arrays for the .NET side of the repo
// ================================================================
//
// The bitset packages are foundation pieces, so the implementation is careful
// about two invariants:
//
//   1. bits are stored LSB-first inside 64-bit words
//   2. any bits beyond Length are always kept at zero
//
// That second rule matters more than it first appears. It keeps popcount,
// equality, binary-string conversion, and logical queries trustworthy even when
// the underlying storage has grown beyond the current logical tail.

/// Raised when a binary-string constructor receives characters other than 0 and 1.
type BitsetError(input: string) =
    inherit Exception($"Invalid binary string: \"{input}\".")

    member _.Input = input

/// A compact boolean array packed into 64-bit words.
type Bitset(size: int) =
    let words = ResizeArray<uint64>(BitsetHelpers.wordsNeeded size)
    let mutable length = 0

    do
        if size < 0 then
            raise (ArgumentOutOfRangeException("size", "Bitset size cannot be negative."))

        length <- size

        for _ in 1 .. BitsetHelpers.wordsNeeded size do
            words.Add 0UL

    new() = Bitset(0)

    static member FromInteger(value: UInt128) =
        if value = UInt128.Zero then
            Bitset(0)
        else
            let low = uint64 value
            let high = uint64 (value >>> 64)

            let logicalLength =
                if high <> 0UL then
                    64 + (64 - BitOperations.LeadingZeroCount(high))
                else
                    64 - BitOperations.LeadingZeroCount(low)

            let bitset = Bitset(logicalLength)
            bitset.SetWord(0, low)

            if high <> 0UL then
                bitset.SetWord(1, high)

            bitset.CleanTrailingBits()
            bitset

    static member FromBinaryString(value: string) =
        if isNull value then
            nullArg "value"

        if value |> Seq.exists (fun ch -> ch <> '0' && ch <> '1') then
            raise (BitsetError(value))

        let bitset = Bitset(value.Length)

        for bitIndex in 0 .. value.Length - 1 do
            let ch = value[value.Length - 1 - bitIndex]
            if ch = '1' then
                let wi = BitsetHelpers.wordIndex bitIndex
                bitset.SetWord(wi, bitset.GetWord(wi) ||| BitsetHelpers.bitmask bitIndex)

        bitset.CleanTrailingBits()
        bitset

    /// Logical size: the number of addressable bits.
    member _.Length = length

    /// Allocated size rounded to a multiple of 64.
    member _.Capacity = words.Count * BitsetHelpers.BitsPerWord

    /// Whether the bitset has zero logical length.
    member _.IsEmpty = (length = 0)

    /// Set a bit to 1, growing the bitset if needed.
    member this.Set(index: int) =
        this.EnsureCapacity(index)
        let wi = BitsetHelpers.wordIndex index
        this.SetWord(wi, this.GetWord(wi) ||| BitsetHelpers.bitmask index)

    /// Set a bit to 0. Clearing beyond the current length is a no-op.
    member this.Clear(index: int) =
        BitsetHelpers.validateIndex index

        if index < length then
            let wi = BitsetHelpers.wordIndex index
            this.SetWord(wi, this.GetWord(wi) &&& (~~~(BitsetHelpers.bitmask index)))

    /// Test whether a bit is set. Testing beyond the current length returns false.
    member this.Test(index: int) =
        BitsetHelpers.validateIndex index

        if index >= length then
            false
        else
            let wi = BitsetHelpers.wordIndex index
            (this.GetWord(wi) &&& BitsetHelpers.bitmask index) <> 0UL

    /// Flip a bit, growing the bitset if needed.
    member this.Toggle(index: int) =
        this.EnsureCapacity(index)
        let wi = BitsetHelpers.wordIndex index
        this.SetWord(wi, this.GetWord(wi) ^^^ BitsetHelpers.bitmask index)
        this.CleanTrailingBits()

    /// Bitwise AND. The result length is the longer input length.
    member this.And(other: Bitset) =
        this.BinaryOp(other, fun left right -> left &&& right)

    /// Bitwise OR. The result length is the longer input length.
    member this.Or(other: Bitset) =
        this.BinaryOp(other, fun left right -> left ||| right)

    /// Bitwise XOR. The result length is the longer input length.
    member this.Xor(other: Bitset) =
        this.BinaryOp(other, fun left right -> left ^^^ right)

    /// Bitwise complement within the logical length of the bitset.
    member this.Not() =
        let result = Bitset(length)

        for wi in 0 .. BitsetHelpers.wordsNeeded length - 1 do
            result.SetWord(wi, ~~~(this.GetWord(wi)))

        result.CleanTrailingBits()
        result

    /// Set difference: keep bits set in this bitset that are not set in the other one.
    member this.AndNot(other: Bitset) =
        this.BinaryOp(other, fun left right -> left &&& (~~~right))

    /// Count how many bits are set to 1.
    member this.PopCount() =
        let mutable count = 0

        for wi in 0 .. BitsetHelpers.wordsNeeded length - 1 do
            count <- count + BitOperations.PopCount(this.GetWord(wi))

        count

    /// Return whether at least one bit is set.
    member this.Any() =
        let mutable found = false
        let mutable wi = 0
        let relevantWords = BitsetHelpers.wordsNeeded length

        while not found && wi < relevantWords do
            if this.GetWord(wi) <> 0UL then
                found <- true

            wi <- wi + 1

        found

    /// Return whether every logical bit is set. Empty bitsets satisfy this vacuously.
    member this.All() =
        if length = 0 then
            true
        else
            let fullWords = length / BitsetHelpers.BitsPerWord
            let mutable allSet = true
            let mutable wi = 0

            while allSet && wi < fullWords do
                if this.GetWord(wi) <> UInt64.MaxValue then
                    allSet <- false

                wi <- wi + 1

            if not allSet then
                false
            else
                let remainingBits = length % BitsetHelpers.BitsPerWord
                if remainingBits = 0 then
                    true
                else
                    this.GetWord(fullWords) = BitsetHelpers.trailingMask remainingBits

    /// Return whether no bits are set.
    member this.None() = not (this.Any())

    /// Iterate over the set-bit indices in ascending order.
    member this.IterSetBits() : seq<int> =
        seq {
            for wi in 0 .. BitsetHelpers.wordsNeeded length - 1 do
                let mutable word = this.GetWord(wi)

                while word <> 0UL do
                    let offset = BitOperations.TrailingZeroCount(word)
                    yield (wi * BitsetHelpers.BitsPerWord) + offset
                    word <- word &&& (word - 1UL)
        }

    /// Convert to a 64-bit integer when the value fits in a single word.
    member this.ToInteger() : uint64 option =
        let relevantWords = BitsetHelpers.wordsNeeded length

        if relevantWords = 0 then
            Some 0UL
        else
            let mutable fits = true
            let mutable wi = 1

            while fits && wi < relevantWords do
                if this.GetWord(wi) <> 0UL then
                    fits <- false

                wi <- wi + 1

            if fits then
                Some(this.GetWord(0))
            else
                None

    /// Convert to a conventional binary string with the highest bit on the left.
    member this.ToBinaryString() =
        if length = 0 then
            String.Empty
        else
            Array.init length (fun i -> if this.Test(length - 1 - i) then '1' else '0')
            |> fun chars -> String(chars)

    /// Convenience alias for Test.
    member this.Contains(index: int) = this.Test(index)

    override this.ToString() = $"Bitset({this.ToBinaryString()})"

    member this.Equals(other: Bitset) =
        if isNull (box other) || length <> other.Length then
            false
        else
            let relevantWords = BitsetHelpers.wordsNeeded length
            let mutable equal = true
            let mutable wi = 0

            while equal && wi < relevantWords do
                if this.GetWord(wi) <> other.GetWord(wi) then
                    equal <- false

                wi <- wi + 1

            equal

    override this.Equals(obj: obj) =
        match obj with
        | :? Bitset as other -> this.Equals(other)
        | _ -> false

    override _.GetHashCode() =
        let mutable hash = HashCode.Combine(length)

        for wi in 0 .. BitsetHelpers.wordsNeeded length - 1 do
            hash <- HashCode.Combine(hash, words[wi])

        hash

    interface IEquatable<Bitset> with
        member this.Equals(other) = this.Equals(other)

    interface IEnumerable<int> with
        member this.GetEnumerator() = (this.IterSetBits()).GetEnumerator()

    interface IEnumerable with
        member this.GetEnumerator() = (this.IterSetBits() :> IEnumerable).GetEnumerator()

    member private _.GetWord(index: int) =
        if index < words.Count then
            words[index]
        else
            0UL

    member private _.SetWord(index: int, value: uint64) =
        words[index] <- value

    member private this.BinaryOp(other: Bitset, operation: uint64 -> uint64 -> uint64) =
        if isNull (box other) then
            nullArg "other"

        let resultLength = max length other.Length
        let result = Bitset(resultLength)

        for wi in 0 .. BitsetHelpers.wordsNeeded resultLength - 1 do
            result.SetWord(wi, operation (this.GetWord(wi)) (other.GetWord(wi)))

        result.CleanTrailingBits()
        result

    member private this.EnsureCapacity(index: int) =
        BitsetHelpers.validateIndex index

        if index >= this.Capacity then
            let mutable newCapacity =
                if this.Capacity = 0 then
                    BitsetHelpers.BitsPerWord
                else
                    this.Capacity

            while index >= newCapacity do
                newCapacity <- newCapacity * 2

            let targetWords = BitsetHelpers.wordsNeeded newCapacity

            while words.Count < targetWords do
                words.Add 0UL

        if index + 1 > length then
            length <- index + 1

    member private _.CleanTrailingBits() =
        let relevantWords = BitsetHelpers.wordsNeeded length

        for wi in relevantWords .. words.Count - 1 do
            words[wi] <- 0UL

        if relevantWords > 0 then
            let remainingBits = length % BitsetHelpers.BitsPerWord

            if remainingBits <> 0 then
                words[relevantWords - 1] <- words[relevantWords - 1] &&& BitsetHelpers.trailingMask remainingBits
