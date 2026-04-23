namespace CodingAdventures.Heap.FSharp

open System
open System.Collections.Generic

type Comparator<'T> = 'T -> 'T -> int

type Heap<'T>(higherPriority: 'T -> 'T -> bool) =
    let data = ResizeArray<'T>()

    member _.Size = data.Count
    member _.IsEmpty() = data.Count = 0
    member _.ToArray() = data |> Seq.toList

    member _.Push(value: 'T) =
        data.Add value
        let mutable index = data.Count - 1
        while index > 0 do
            let parent = (index - 1) / 2
            if higherPriority data.[index] data.[parent] then
                let current = data.[index]
                data.[index] <- data.[parent]
                data.[parent] <- current
                index <- parent
            else
                index <- 0

    member _.Peek() =
        if data.Count = 0 then invalidOp "peek at an empty heap"
        data.[0]

    member _.Pop() =
        if data.Count = 0 then invalidOp "pop from an empty heap"
        let root = data.[0]
        let last = data.[data.Count - 1]
        data.RemoveAt(data.Count - 1)
        if data.Count > 0 then
            data.[0] <- last
            let mutable index = 0
            let mutable running = true
            while running do
                let left = (index * 2) + 1
                let right = left + 1
                let mutable best = index
                if left < data.Count && higherPriority data.[left] data.[best] then
                    best <- left
                if right < data.Count && higherPriority data.[right] data.[best] then
                    best <- right
                if best = index then
                    running <- false
                else
                    let current = data.[index]
                    data.[index] <- data.[best]
                    data.[best] <- current
                    index <- best
        root

type MinHeap<'T when 'T : comparison>(?comparator: Comparator<'T>) =
    inherit Heap<'T>(fun left right ->
        let cmp = defaultArg comparator compare
        cmp left right < 0)

    static member FromEnumerable(items: seq<'T>, ?comparator: Comparator<'T>) =
        let heap = MinHeap<'T>(?comparator = comparator)
        items |> Seq.iter heap.Push
        heap

type MaxHeap<'T when 'T : comparison>(?comparator: Comparator<'T>) =
    inherit Heap<'T>(fun left right ->
        let cmp = defaultArg comparator compare
        cmp left right > 0)

module HeapFunctions =
    let heapSort items =
        let heap = MinHeap<_>.FromEnumerable(items)
        let mutable output = []
        while not (heap.IsEmpty()) do
            output <- heap.Pop() :: output
        List.rev output
