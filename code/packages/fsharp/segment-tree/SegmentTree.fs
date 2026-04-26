namespace CodingAdventures.SegmentTree

open System
open System.Collections.Generic

type SegmentTree<'T>(values: seq<'T>, combine: 'T -> 'T -> 'T, identity: 'T) =
    do
        if isNull (box values) then
            nullArg (nameof values)

        if isNull (box combine) then
            nullArg (nameof combine)

    let source = Seq.toArray values
    let size = source.Length
    let tree = Array.create (max 4 (4 * size)) identity

    let rec build node left right =
        if left = right then
            tree[node] <- source[left]
        else
            let mid = (left + right) / 2
            build (2 * node) left mid
            build (2 * node + 1) (mid + 1) right
            tree[node] <- combine tree[2 * node] tree[2 * node + 1]

    let rec queryHelper node left right queryLeft queryRight =
        if right < queryLeft || left > queryRight then
            identity
        elif queryLeft <= left && right <= queryRight then
            tree[node]
        else
            let mid = (left + right) / 2
            let leftResult = queryHelper (2 * node) left mid queryLeft queryRight
            let rightResult = queryHelper (2 * node + 1) (mid + 1) right queryLeft queryRight
            combine leftResult rightResult

    let rec updateHelper node left right index value =
        if left = right then
            tree[node] <- value
        else
            let mid = (left + right) / 2

            if index <= mid then
                updateHelper (2 * node) left mid index value
            else
                updateHelper (2 * node + 1) (mid + 1) right index value

            tree[node] <- combine tree[2 * node] tree[2 * node + 1]

    let rec collectLeaves node left right (acc: ResizeArray<'T>) =
        if left = right then
            acc.Add tree[node]
        else
            let mid = (left + right) / 2
            collectLeaves (2 * node) left mid acc
            collectLeaves (2 * node + 1) (mid + 1) right acc

    do
        if size > 0 then
            build 1 0 (size - 1)

    member _.Size = size

    member _.Count = size

    member _.IsEmpty = size = 0

    member _.Query(queryLeft: int, queryRight: int) =
        if queryLeft < 0 || queryRight >= size || queryLeft > queryRight then
            raise (ArgumentOutOfRangeException(nameof queryLeft, $"Invalid query range [{queryLeft}, {queryRight}] for array of size {size}."))

        queryHelper 1 0 (size - 1) queryLeft queryRight

    member _.Update(index: int, value: 'T) =
        if index < 0 || index >= size then
            raise (ArgumentOutOfRangeException(nameof index, $"Index {index} out of range for array of size {size}."))

        updateHelper 1 0 (size - 1) index value

    member _.ToList() =
        let result = ResizeArray<'T>(size)

        if size > 0 then
            collectLeaves 1 0 (size - 1) result

        result |> Seq.toList

    override _.ToString() = $"SegmentTree{{n={size}, identity={identity}}}"

[<RequireQualifiedAccess>]
module SegmentTree =
    [<Literal>]
    let VERSION = "0.1.0"

    let private gcd (left: int) (right: int) =
        let mutable a = Math.Abs left
        let mutable b = Math.Abs right

        while b <> 0 do
            let next = a % b
            a <- b
            b <- next

        a

    let sumTree (values: seq<int>) = SegmentTree<int>(values, (+), 0)

    let minTree (values: seq<int>) = SegmentTree<int>(values, min, Int32.MaxValue)

    let maxTree (values: seq<int>) = SegmentTree<int>(values, max, Int32.MinValue)

    let gcdTree (values: seq<int>) = SegmentTree<int>(values, gcd, 0)
