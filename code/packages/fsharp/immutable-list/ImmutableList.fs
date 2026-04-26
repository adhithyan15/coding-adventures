namespace CodingAdventures.ImmutableList

open System
open System.Collections
open System.Collections.Generic

/// Persistent list where updates return new list instances and leave prior versions unchanged.
type ImmutableList<'T> private (items: 'T array) =
    static let empty = ImmutableList<'T>(Array.empty)

    new() = ImmutableList<'T>(Array.empty)

    static member Empty = empty

    static member FromSeq(values: seq<'T>) =
        if isNull (box values) then
            nullArg (nameof values)

        let array = Seq.toArray values
        if Array.isEmpty array then empty else ImmutableList<'T>(array)

    static member FromSlice(values: seq<'T>) =
        ImmutableList<'T>.FromSeq(values)

    member _.Count = items.Length
    member _.Length = items.Length
    member _.IsEmpty = items.Length = 0

    member _.Item
        with get index =
            if index < 0 || index >= items.Length then
                raise (ArgumentOutOfRangeException(nameof index, "Index is outside the bounds of the list."))

            items[index]

    member _.Get(index: int) =
        if index >= 0 && index < items.Length then
            Some items[index]
        else
            None

    member _.TryGet(index: int, value: byref<'T>) =
        if index >= 0 && index < items.Length then
            value <- items[index]
            true
        else
            value <- Unchecked.defaultof<'T>
            false

    member _.Push(value: 'T) =
        let next = Array.zeroCreate<'T> (items.Length + 1)
        Array.Copy(items, next, items.Length)
        next[next.Length - 1] <- value
        ImmutableList<'T>(next)

    member _.Set(index: int, value: 'T) =
        if index < 0 || index >= items.Length then
            raise (ArgumentOutOfRangeException(nameof index, "Index is outside the bounds of the list."))

        let next = Array.copy items
        next[index] <- value
        ImmutableList<'T>(next)

    member _.Pop() =
        if items.Length = 0 then
            invalidOp "Cannot pop from an empty list."

        let value = items[items.Length - 1]

        if items.Length = 1 then
            empty, value
        else
            let next = Array.zeroCreate<'T> (items.Length - 1)
            Array.Copy(items, next, next.Length)
            ImmutableList<'T>(next), value

    member _.ToArray() =
        Array.copy items

    member _.ToList() =
        Array.toList items

    override _.ToString() =
        $"ImmutableList(count={items.Length})"

    interface IEnumerable<'T> with
        member _.GetEnumerator() =
            ((items :> seq<'T>).GetEnumerator())

    interface IEnumerable with
        member _.GetEnumerator() =
            ((items :> IEnumerable).GetEnumerator())
