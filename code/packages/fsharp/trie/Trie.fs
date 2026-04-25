namespace CodingAdventures.Trie

open System
open System.Collections.Generic
open System.Text

type private Node<'T>() =
    let children = Dictionary<char, Node<'T>>()

    member _.Children = children
    member val IsEnd = false with get, set
    member val Value = Unchecked.defaultof<'T> with get, set

/// Generic trie (prefix tree) mapping string keys to values.
type Trie<'T>() =
    let root = Node<'T>()
    let mutable count = 0

    member _.Count = count
    member _.Size = count
    member _.IsEmpty = count = 0

    member private _.FindNode(key: string) =
        if isNull key then
            None
        else
            let mutable node = root
            let mutable index = 0
            let mutable found = true

            while found && index < key.Length do
                match node.Children.TryGetValue(key[index]) with
                | true, child ->
                    node <- child
                    index <- index + 1
                | false, _ -> found <- false

            if found then Some node else None

    member _.Insert(key: string, value: 'T) =
        if isNull key then
            nullArg (nameof key)

        let mutable node = root

        for ch in key do
            match node.Children.TryGetValue(ch) with
            | true, child -> node <- child
            | false, _ ->
                let child = Node<'T>()
                node.Children.Add(ch, child)
                node <- child

        if not node.IsEnd then
            node.IsEnd <- true
            count <- count + 1

        node.Value <- value

    member this.Get(key: string) =
        match this.FindNode key with
        | Some node when node.IsEnd -> Some node.Value
        | _ -> None

    member this.TryGetValue(key: string, value: byref<'T>) =
        match this.FindNode key with
        | Some node when node.IsEnd ->
            value <- node.Value
            true
        | _ ->
            value <- Unchecked.defaultof<'T>
            false

    member this.Contains(key: string) =
        match this.FindNode key with
        | Some node -> node.IsEnd
        | None -> false

    member this.StartsWith(prefix: string) =
        this.FindNode prefix |> Option.isSome

    member this.Delete(key: string) =
        match this.FindNode key with
        | Some node when node.IsEnd ->
            node.IsEnd <- false
            node.Value <- Unchecked.defaultof<'T>
            count <- count - 1
            true
        | _ -> false

    member private this.CollectKeys(node: Node<'T>, prefix: StringBuilder, results: ResizeArray<string>) =
        if node.IsEnd then
            results.Add(prefix.ToString())

        for KeyValue(ch, child) in node.Children do
            prefix.Append(ch) |> ignore
            this.CollectKeys(child, prefix, results)
            prefix.Length <- prefix.Length - 1

    member this.KeysWithPrefix(prefix: string) =
        if isNull prefix then
            nullArg (nameof prefix)

        let results = ResizeArray<string>()

        match this.FindNode prefix with
        | Some node -> this.CollectKeys(node, StringBuilder(prefix), results)
        | None -> ()

        List.ofSeq results

    member this.Keys() = this.KeysWithPrefix("")
