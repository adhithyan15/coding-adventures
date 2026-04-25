namespace CodingAdventures.RadixTree

open System
open System.Collections
open System.Collections.Generic

type private Edge<'V>(label: string, child: Node<'V>) =
    member _.Label = label
    member _.Child = child

and private Node<'V>() =
    let children = SortedDictionary<char, Edge<'V>>()

    member val IsEnd = false with get, set
    member val Value = Unchecked.defaultof<'V> with get, set
    member _.Children = children

    static member Leaf(value: 'V) =
        let node = Node<'V>()
        node.IsEnd <- true
        node.Value <- value
        node

type private DeleteResult =
    { Deleted: bool
      ChildMergeable: bool }

type RadixTree<'V>(entries: seq<string * 'V>) =
    let root = Node<'V>()
    let mutable size = 0

    let firstChar (value: string) = value[0]

    let commonPrefixLength (left: string) (right: string) =
        let limit = min left.Length right.Length
        let mutable index = 0

        while index < limit && left[index] = right[index] do
            index <- index + 1

        index

    let rec insertRecursive (node: Node<'V>) (key: string) (value: 'V) =
        if key.Length = 0 then
            let added = not node.IsEnd
            node.IsEnd <- true
            node.Value <- value
            added
        else
            let first = firstChar key

            match node.Children.TryGetValue(first) with
            | false, _ ->
                node.Children[first] <- Edge(key, Node.Leaf value)
                true
            | true, edge ->
                let commonLength = commonPrefixLength key edge.Label

                if commonLength = edge.Label.Length then
                    insertRecursive edge.Child (key.Substring commonLength) value
                else
                    let common = edge.Label.Substring(0, commonLength)
                    let labelRest = edge.Label.Substring commonLength
                    let keyRest = key.Substring commonLength
                    let splitNode = Node<'V>()
                    splitNode.Children[firstChar labelRest] <- Edge(labelRest, edge.Child)

                    if keyRest.Length = 0 then
                        splitNode.IsEnd <- true
                        splitNode.Value <- value
                    else
                        splitNode.Children[firstChar keyRest] <- Edge(keyRest, Node.Leaf value)

                    node.Children[first] <- Edge(common, splitNode)
                    true

    let rec deleteRecursive (node: Node<'V>) (key: string) =
        if key.Length = 0 then
            if not node.IsEnd then
                { Deleted = false; ChildMergeable = false }
            else
                node.IsEnd <- false
                node.Value <- Unchecked.defaultof<'V>
                { Deleted = true; ChildMergeable = node.Children.Count = 1 }
        else
            let first = firstChar key

            match node.Children.TryGetValue(first) with
            | false, _ -> { Deleted = false; ChildMergeable = false }
            | true, edge ->
                let commonLength = commonPrefixLength key edge.Label

                if commonLength < edge.Label.Length then
                    { Deleted = false; ChildMergeable = false }
                else
                    let result = deleteRecursive edge.Child (key.Substring commonLength)

                    if not result.Deleted then
                        result
                    else
                        if result.ChildMergeable then
                            let grandchild = edge.Child.Children.Values |> Seq.head
                            node.Children[first] <- Edge(edge.Label + grandchild.Label, grandchild.Child)
                        elif not edge.Child.IsEnd && edge.Child.Children.Count = 0 then
                            node.Children.Remove(first) |> ignore

                        { Deleted = true
                          ChildMergeable = (not node.IsEnd) && node.Children.Count = 1 }

    let rec collectKeys (node: Node<'V>) (current: string) (results: ResizeArray<string>) =
        if node.IsEnd then
            results.Add current

        for edge in node.Children.Values do
            collectKeys edge.Child (current + edge.Label) results

    let rec collectValues (node: Node<'V>) current (results: ResizeArray<string * 'V>) =
        if node.IsEnd then
            results.Add(current, node.Value)

        for edge in node.Children.Values do
            collectValues edge.Child (current + edge.Label) results

    let rec countNodes (node: Node<'V>) =
        let mutable count = 1

        for edge in node.Children.Values do
            count <- count + countNodes edge.Child

        count

    let keyExists (key: string) =
        let mutable node = root
        let mutable remaining = key
        let mutable matched = true

        while matched && remaining.Length > 0 do
            match node.Children.TryGetValue(firstChar remaining) with
            | false, _ -> matched <- false
            | true, edge ->
                let commonLength = commonPrefixLength remaining edge.Label

                if commonLength < edge.Label.Length then
                    matched <- false
                else
                    remaining <- remaining.Substring commonLength
                    node <- edge.Child

        matched && node.IsEnd

    do
        if isNull (box entries) then
            nullArg (nameof entries)

        for key, value in entries do
            if insertRecursive root key value then
                size <- size + 1

    new() = RadixTree<'V>(Seq.empty)

    member _.Count = size
    member _.Size = size
    member _.IsEmpty = size = 0

    member _.Insert(key: string, value: 'V) =
        if isNull key then
            nullArg (nameof key)

        if insertRecursive root key value then
            size <- size + 1

    member this.Put(key: string, value: 'V) = this.Insert(key, value)

    member _.Search(key: string) =
        if isNull key then
            nullArg (nameof key)

        let mutable node = root
        let mutable remaining = key
        let mutable matched = true

        while matched && remaining.Length > 0 do
            match node.Children.TryGetValue(firstChar remaining) with
            | false, _ -> matched <- false
            | true, edge ->
                let commonLength = commonPrefixLength remaining edge.Label

                if commonLength < edge.Label.Length then
                    matched <- false
                else
                    remaining <- remaining.Substring commonLength
                    node <- edge.Child

        if matched && node.IsEnd then
            Some node.Value
        else
            None

    member this.Get(key: string) = this.Search key

    member _.ContainsKey(key: string) =
        if isNull key then
            nullArg (nameof key)

        keyExists key

    member _.Delete(key: string) =
        if isNull key then
            nullArg (nameof key)

        let result = deleteRecursive root key

        if result.Deleted then
            size <- size - 1

        result.Deleted

    member _.StartsWith(prefix: string) =
        if isNull prefix then
            nullArg (nameof prefix)

        if prefix.Length = 0 then
            size > 0
        else
            let mutable node = root
            let mutable remaining = prefix
            let mutable matched = true
            let mutable answer = false

            while matched && not answer && remaining.Length > 0 do
                match node.Children.TryGetValue(firstChar remaining) with
                | false, _ -> matched <- false
                | true, edge ->
                    let commonLength = commonPrefixLength remaining edge.Label

                    if commonLength = remaining.Length then
                        answer <- true
                    elif commonLength < edge.Label.Length then
                        matched <- false
                    else
                        remaining <- remaining.Substring commonLength
                        node <- edge.Child

            answer || (matched && (node.IsEnd || node.Children.Count > 0))

    member this.WordsWithPrefix(prefix: string) =
        if isNull prefix then
            nullArg (nameof prefix)

        if prefix.Length = 0 then
            this.Keys()
        else
            let mutable node = root
            let mutable remaining = prefix
            let mutable path = ""
            let mutable matched = true
            let mutable finished = false
            let results = ResizeArray<string>()

            while matched && not finished && remaining.Length > 0 do
                match node.Children.TryGetValue(firstChar remaining) with
                | false, _ -> matched <- false
                | true, edge ->
                    let commonLength = commonPrefixLength remaining edge.Label

                    if commonLength = remaining.Length then
                        if commonLength = edge.Label.Length then
                            path <- path + edge.Label
                            node <- edge.Child
                            remaining <- ""
                        else
                            collectKeys edge.Child (path + edge.Label) results
                            finished <- true
                    elif commonLength < edge.Label.Length then
                        matched <- false
                    else
                        path <- path + edge.Label
                        remaining <- remaining.Substring commonLength
                        node <- edge.Child

            if matched && not finished then
                collectKeys node path results

            List.ofSeq results

    member _.LongestPrefixMatch(key: string) =
        if isNull key then
            nullArg (nameof key)

        let mutable node = root
        let mutable remaining = key
        let mutable consumed = 0
        let mutable best = if node.IsEnd then Some "" else None
        let mutable keepGoing = true

        while keepGoing && remaining.Length > 0 do
            match node.Children.TryGetValue(firstChar remaining) with
            | false, _ -> keepGoing <- false
            | true, edge ->
                let commonLength = commonPrefixLength remaining edge.Label

                if commonLength < edge.Label.Length then
                    keepGoing <- false
                else
                    consumed <- consumed + commonLength
                    remaining <- remaining.Substring commonLength
                    node <- edge.Child

                    if node.IsEnd then
                        best <- Some(key.Substring(0, consumed))

        best

    member _.Keys() =
        let results = ResizeArray<string>()
        collectKeys root "" results
        List.ofSeq results

    member this.Values() =
        this.ToMap() |> Map.toList |> List.map snd

    member _.ToMap() =
        let results = ResizeArray<string * 'V>()
        collectValues root "" results
        results |> Seq.sortBy fst |> Map.ofSeq

    member _.NodeCount() = countNodes root

    interface IEnumerable<string> with
        member this.GetEnumerator() = (this.Keys() :> seq<string>).GetEnumerator()

    interface IEnumerable with
        member this.GetEnumerator() = (this.Keys() :> IEnumerable).GetEnumerator()
