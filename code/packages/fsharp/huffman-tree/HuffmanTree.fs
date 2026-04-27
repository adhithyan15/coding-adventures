namespace CodingAdventures.HuffmanTree.FSharp

open System
open System.Collections.Generic
open CodingAdventures.Heap.FSharp

type HuffmanNode =
    | Leaf of symbol: int * weight: int
    | Internal of weight: int * left: HuffmanNode * right: HuffmanNode * creationOrder: int

[<RequireQualifiedAccess>]
module private Node =
    let weight =
        function
        | Leaf (_, weight) -> weight
        | Internal (weight, _, _, _) -> weight

    let priority =
        function
        | Leaf (symbol, weight) -> (weight, 0, symbol, Int32.MaxValue)
        | Internal (weight, _, _, creationOrder) -> (weight, 1, Int32.MaxValue, creationOrder)

type HuffmanTree private (root: HuffmanNode, symbolCount: int) =
    static member Build(weights: seq<int * int>) =
        if isNull (box weights) then nullArg "weights"

        let items = weights |> Seq.toList
        if List.isEmpty items then
            invalidArg "weights" "weights must not be empty"

        for (symbol, frequency) in items do
            if symbol < 0 then
                invalidArg "weights" $"symbol must be non-negative; got symbol={symbol}"

            if frequency <= 0 then
                invalidArg "weights" $"frequency must be positive; got symbol={symbol}, freq={frequency}"

        let heap =
            MinHeap<((int * int * int * int) * HuffmanNode)>(
                comparator =
                    fun (leftPriority, _) (rightPriority, _) -> compare leftPriority rightPriority)

        for (symbol, frequency) in items do
            let leaf = Leaf(symbol, frequency)
            heap.Push(Node.priority leaf, leaf)

        let mutable creationOrder = 0
        while heap.Size > 1 do
            let (_, left) = heap.Pop()
            let (_, right) = heap.Pop()
            let internalNode = Internal(Node.weight left + Node.weight right, left, right, creationOrder)
            creationOrder <- creationOrder + 1
            heap.Push(Node.priority internalNode, internalNode)

        let (_, root) = heap.Pop()
        HuffmanTree(root, List.length items)

    member _.CodeTable() : IReadOnlyDictionary<int, string> =
        let table = Dictionary<int, string>()

        let rec walk node prefix =
            match node with
            | Leaf (symbol, _) -> table.[symbol] <- if prefix = "" then "0" else prefix
            | Internal (_, left, right, _) ->
                walk left (prefix + "0")
                walk right (prefix + "1")

        walk root ""
        table :> IReadOnlyDictionary<int, string>

    member _.CodeFor(symbol: int) =
        let rec find node prefix =
            match node with
            | Leaf (leafSymbol, _) when leafSymbol = symbol -> Some(if prefix = "" then "0" else prefix)
            | Leaf _ -> None
            | Internal (_, left, right, _) ->
                match find left (prefix + "0") with
                | Some code -> Some code
                | None -> find right (prefix + "1")

        find root ""

    member _.CanonicalCodeTable() : IReadOnlyDictionary<int, string> =
        let lengths = Dictionary<int, int>()

        let rec collect node depth =
            match node with
            | Leaf (symbol, _) -> lengths.[symbol] <- if depth = 0 then 1 else depth
            | Internal (_, left, right, _) ->
                collect left (depth + 1)
                collect right (depth + 1)

        collect root 0

        let codes = Dictionary<int, string>()
        if lengths.Count = 1 then
            let onlySymbol = lengths.Keys |> Seq.head
            codes.[onlySymbol] <- "0"
            codes :> IReadOnlyDictionary<int, string>
        else
            let ordered =
                lengths
                |> Seq.map (fun pair -> pair.Key, pair.Value)
                |> Seq.sortBy (fun (symbol, length) -> length, symbol)
                |> Seq.toList

            let mutable codeValue = 0
            let mutable previousLength = ordered |> List.head |> snd
            for (symbol, length) in ordered do
                if length > previousLength then
                    codeValue <- codeValue <<< (length - previousLength)

                codes.[symbol] <- Convert.ToString(codeValue, 2).PadLeft(length, '0')
                codeValue <- codeValue + 1
                previousLength <- length

            codes :> IReadOnlyDictionary<int, string>

    member _.DecodeAll(bits: string, count: int) =
        if isNull bits then nullArg "bits"
        if count < 0 then invalidArg "count" "count must be non-negative"

        let result = ResizeArray<int>()
        let mutable index = 0
        let mutable node = root
        let singleLeaf =
            match root with
            | Leaf _ -> true
            | _ -> false

        while result.Count < count do
            match node with
            | Leaf (symbol, _) ->
                result.Add symbol
                node <- root
                if singleLeaf && index < bits.Length then
                    index <- index + 1
            | Internal (_, left, right, _) ->
                if index >= bits.Length then
                    invalidOp $"Bit stream exhausted after {result.Count} symbols; expected {count}"

                let bit = bits[index]
                index <- index + 1
                node <-
                    match bit with
                    | '0' -> left
                    | '1' -> right
                    | _ -> invalidOp "Bit stream must contain only '0' and '1'"

        List.ofSeq result

    member _.Weight() = Node.weight root

    member _.Depth() =
        let rec maxDepth node depth =
            match node with
            | Leaf _ -> depth
            | Internal (_, left, right, _) -> max (maxDepth left (depth + 1)) (maxDepth right (depth + 1))

        maxDepth root 0

    member _.SymbolCount() = symbolCount

    member this.Leaves() =
        let table = this.CodeTable()
        let leaves = ResizeArray<int * string>()

        let rec collect node =
            match node with
            | Leaf (symbol, _) -> leaves.Add(symbol, table.[symbol])
            | Internal (_, left, right, _) ->
                collect left
                collect right

        collect root
        List.ofSeq leaves

    member _.IsValid() =
        let seen = HashSet<int>()

        let rec validate node =
            match node with
            | Leaf (symbol, _) -> seen.Add symbol
            | Internal (weight, left, right, _) ->
                weight = Node.weight left + Node.weight right
                && validate left
                && validate right

        validate root
