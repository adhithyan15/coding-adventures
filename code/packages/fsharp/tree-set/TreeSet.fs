namespace CodingAdventures.TreeSet

open System
open System.Collections
open System.Collections.Generic

type TreeSet<'T when 'T: comparison>(values: seq<'T>) =
    let mutable items = Set.ofSeq values

    let throwIfNull name value =
        if isNull (box value) then
            nullArg name

    new() = TreeSet<'T>(Seq.empty)

    member _.Count = items.Count
    member _.Size = items.Count
    member _.IsEmpty = items.IsEmpty

    member this.Add(value: 'T) =
        throwIfNull (nameof value) value
        items <- items.Add value
        this

    member _.Remove(value: 'T) =
        if isNull (box value) then
            false
        else
            let present = items.Contains value
            items <- items.Remove value
            present

    member this.Delete(value: 'T) = this.Remove value
    member this.Discard(value: 'T) = this.Remove value

    member _.Contains(value: 'T) =
        not (isNull (box value)) && items.Contains value

    member this.Has(value: 'T) = this.Contains value

    member _.Min() =
        if items.IsEmpty then None else Some items.MinimumElement

    member _.Max() =
        if items.IsEmpty then None else Some items.MaximumElement

    member this.First() = this.Min()
    member this.Last() = this.Max()

    member _.Predecessor(value: 'T) =
        if isNull (box value) then
            None
        else
            items |> Seq.filter (fun current -> current < value) |> Seq.tryLast

    member _.Successor(value: 'T) =
        if isNull (box value) then
            None
        else
            items |> Seq.tryFind (fun current -> current > value)

    member _.Rank(value: 'T) =
        if isNull (box value) then
            0
        else
            items |> Seq.takeWhile (fun current -> current < value) |> Seq.length

    member _.ByRank(rank: int) =
        if rank < 0 || rank >= items.Count then
            None
        else
            items |> Seq.item rank |> Some

    member this.KthSmallest(k: int) =
        if k <= 0 then None else this.ByRank(k - 1)

    member _.Range(low: 'T, high: 'T, ?inclusive: bool) =
        throwIfNull (nameof low) low
        throwIfNull (nameof high) high

        if low > high then
            []
        else
            let includeBounds = defaultArg inclusive true

            items
            |> Seq.filter (fun value ->
                if includeBounds then
                    value >= low && value <= high
                else
                    value > low && value < high)
            |> List.ofSeq

    member _.ToList() = List.ofSeq items
    member this.ToSortedArray() = this.ToList()

    member _.Union(other: TreeSet<'T>) =
        ArgumentNullException.ThrowIfNull(other)
        TreeSet<'T>(Seq.append items other.Values)

    member _.Intersection(other: TreeSet<'T>) =
        ArgumentNullException.ThrowIfNull(other)
        TreeSet<'T>(Set.intersect items other.Values)

    member _.Difference(other: TreeSet<'T>) =
        ArgumentNullException.ThrowIfNull(other)
        TreeSet<'T>(Set.difference items other.Values)

    member _.SymmetricDifference(other: TreeSet<'T>) =
        ArgumentNullException.ThrowIfNull(other)
        let left = Set.difference items other.Values
        let right = Set.difference other.Values items
        TreeSet<'T>(Set.union left right)

    member _.IsSubset(other: TreeSet<'T>) =
        ArgumentNullException.ThrowIfNull(other)
        Set.isSubset items other.Values

    member _.IsSuperset(other: TreeSet<'T>) =
        ArgumentNullException.ThrowIfNull(other)
        Set.isSuperset items other.Values

    member _.IsDisjoint(other: TreeSet<'T>) =
        ArgumentNullException.ThrowIfNull(other)
        Set.intersect items other.Values |> Set.isEmpty

    member internal _.Values = items

    override _.Equals(other: obj) =
        match other with
        | :? TreeSet<'T> as tree -> items = tree.Values
        | _ -> false

    override _.GetHashCode() =
        hash items

    override this.ToString() =
        let body = this.ToList() |> List.map string |> String.concat ", "
        $"TreeSet([{body}])"

    interface IEnumerable<'T> with
        member _.GetEnumerator() = (items :> seq<'T>).GetEnumerator()

    interface IEnumerable with
        member _.GetEnumerator() = (items :> IEnumerable).GetEnumerator()
