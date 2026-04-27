namespace CodingAdventures.FenwickTree

open System
open System.Globalization

module private FenwickHelpers =
    let lowBit index = index &&& -index

    let highestPowerOfTwoAtMost value =
        let mutable result = 1

        while result <= value / 2 do
            result <- result <<< 1

        result

    let formatFloat (value: float) =
        value.ToString("G17", CultureInfo.InvariantCulture)

// FenwickTree.fs -- Prefix sums encoded as overlapping power-of-two ranges
// ==========================================================================
//
// A Fenwick tree stores partial sums in a 1-based array. Index i is
// responsible for the range whose width is lowBit(i):
//
//   i = 12 = 1100b
//   lowBit(i) = 0100b = 4
//
// So slot 12 stores the total for the last four values ending at 12.
// Moving upward by adding lowBit(i) reaches larger covering ranges; moving
// downward by subtracting lowBit(i) peels those ranges back off.

/// Base class for Fenwick-tree specific errors.
type FenwickError(message: string) =
    inherit Exception(message)

/// Raised when an operation uses an index outside the legal range.
type FenwickIndexOutOfRangeError(message: string) =
    inherit FenwickError(message)

/// Raised when findKth is called on an empty tree.
type FenwickEmptyTreeError(message: string) =
    inherit FenwickError(message)

/// Binary Indexed Tree for prefix sums with point updates.
type FenwickTree(length: int) =
    let treeLength = length
    let bit = Array.zeroCreate<float> (length + 1)

    do
        if length < 0 then
            raise (FenwickError(sprintf "Size must be a non-negative integer, got %d" length))

    static member FromList(values: seq<float>) =
        if isNull (box values) then
            nullArg "values"

        let data = values |> Seq.toArray
        let tree = FenwickTree(data.Length)

        for index in 1 .. tree.Length do
            tree.SetTreeValue(index, tree.GetTreeValue(index) + data[index - 1])
            let parent = index + FenwickHelpers.lowBit index

            if parent <= tree.Length then
                tree.SetTreeValue(parent, tree.GetTreeValue(parent) + tree.GetTreeValue(index))

        tree

    /// Number of values stored in the logical array.
    member _.Length = treeLength

    /// Whether the tree has zero elements.
    member _.IsEmpty = treeLength = 0

    /// Add delta to the value at index.
    member this.Update(index: int, delta: float) =
        this.CheckIndex(index)
        let mutable current = index

        while current <= treeLength do
            bit[current] <- bit[current] + delta
            current <- current + FenwickHelpers.lowBit current

    /// Return the sum of elements in the inclusive range 1..index.
    member _.PrefixSum(index: int) =
        if index < 0 || index > treeLength then
            raise (
                FenwickIndexOutOfRangeError(
                    sprintf "prefixSum index %d out of range [0, %d]" index treeLength
                )
            )

        let mutable total = 0.0
        let mutable current = index

        while current > 0 do
            total <- total + bit[current]
            current <- current - FenwickHelpers.lowBit current

        total

    /// Return the sum of the inclusive range left..right.
    member this.RangeSum(left: int, right: int) =
        if left > right then
            raise (FenwickError(sprintf "left (%d) must be <= right (%d)" left right))

        this.CheckIndex(left)
        this.CheckIndex(right)

        if left = 1 then
            this.PrefixSum(right)
        else
            this.PrefixSum(right) - this.PrefixSum(left - 1)

    /// Return the exact value stored at one index.
    member this.PointQuery(index: int) =
        this.CheckIndex(index)
        this.RangeSum(index, index)

    /// Find the smallest index whose prefix sum is at least target.
    member this.FindKth(target: float) =
        if treeLength = 0 then
            raise (FenwickEmptyTreeError("findKth called on empty tree"))

        if target <= 0.0 then
            raise (FenwickError(sprintf "k must be positive, got %s" (FenwickHelpers.formatFloat target)))

        let total = this.PrefixSum(treeLength)

        if target > total then
            raise (
                FenwickError(
                    sprintf
                        "k (%s) exceeds total sum of the tree (%s)"
                        (FenwickHelpers.formatFloat target)
                        (FenwickHelpers.formatFloat total)
                )
            )

        let mutable index = 0
        let mutable remaining = target
        let mutable step = FenwickHelpers.highestPowerOfTwoAtMost treeLength

        while step > 0 do
            let nextIndex = index + step

            if nextIndex <= treeLength && bit[nextIndex] < remaining then
                index <- nextIndex
                remaining <- remaining - bit[nextIndex]

            step <- step >>> 1

        index + 1

    override _.ToString() =
        let rendered =
            bit
            |> Array.skip 1
            |> Array.map FenwickHelpers.formatFloat
            |> String.concat ", "

        sprintf "FenwickTree(n=%d, bit=[%s])" treeLength rendered

    member private _.CheckIndex(index: int) =
        if index < 1 || index > treeLength then
            raise (
                FenwickIndexOutOfRangeError(
                    sprintf "Index %d out of range [1, %d]" index treeLength
                )
            )

    member private _.GetTreeValue(index: int) = bit[index]

    member private _.SetTreeValue(index: int, value: float) =
        bit[index] <- value
