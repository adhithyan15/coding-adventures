namespace CodingAdventures.BPlusTree

open System
open System.Collections
open System.Collections.Generic

[<AbstractClass>]
type private Node<'K, 'V when 'K: comparison>() =
    member val Keys = ResizeArray<'K>()

type private LeafNode<'K, 'V when 'K: comparison>() =
    inherit Node<'K, 'V>()

    member val Values = ResizeArray<'V>()
    member val Next: LeafNode<'K, 'V> option = None with get, set

type private InternalNode<'K, 'V when 'K: comparison>() =
    inherit Node<'K, 'V>()

    member val Children = ResizeArray<Node<'K, 'V>>()

type BPlusTree<'K, 'V when 'K: comparison>(minimumDegree: int) =
    let t =
        if minimumDegree < 2 then
            invalidArg (nameof minimumDegree) "Minimum degree must be at least 2."

        minimumDegree

    let entries = SortedDictionary<'K, 'V>()
    let mutable firstLeaf = LeafNode<'K, 'V>()
    let mutable root: Node<'K, 'V> = firstLeaf

    let maxKeys = 2 * t - 1
    let maxChildren = 2 * t

    let throwIfNull name value =
        if isNull (box value) then
            nullArg name

    let findKeyIndex (keys: ResizeArray<'K>) key =
        let mutable low = 0
        let mutable high = keys.Count

        while low < high do
            let mid = (low + high) / 2

            if compare keys[mid] key < 0 then
                low <- mid + 1
            else
                high <- mid

        low

    let partitionSizes count minSize maxSize =
        if count <= maxSize then
            [ count ]
        else
            let groups = (count + maxSize - 1) / maxSize
            let baseSize = count / groups
            let remainder = count % groups

            [ for index in 0 .. groups - 1 do
                  let size = baseSize + if index < remainder then 1 else 0

                  if size < minSize || size > maxSize then
                      invalidOp "Unable to partition B+ tree nodes within degree constraints."

                  size ]

    let rec getFirstKey (node: Node<'K, 'V>) =
        match node with
        | :? LeafNode<'K, 'V> as leaf -> leaf.Keys[0]
        | :? InternalNode<'K, 'V> as internalNode -> getFirstKey internalNode.Children[0]
        | _ -> invalidOp "Unknown B+ tree node type."

    let buildInternal (children: Node<'K, 'V> list) =
        let node = InternalNode<'K, 'V>()

        for child in children do
            node.Children.Add child

        for child in children |> List.tail do
            node.Keys.Add(getFirstKey child)

        node :> Node<'K, 'V>

    let rec buildLevel (children: Node<'K, 'V> list) =
        match children with
        | [ single ] -> single
        | _ when children.Length <= maxChildren -> buildInternal children
        | _ ->
            let mutable offset = 0

            let parents =
                [ for size in partitionSizes children.Length t maxChildren do
                      let group = children |> List.skip offset |> List.take size
                      offset <- offset + size
                      buildInternal group ]

            buildLevel parents

    let rebuild () =
        if entries.Count = 0 then
            firstLeaf <- LeafNode<'K, 'V>()
            root <- firstLeaf
        else
            let pairs = entries |> Seq.toArray
            let mutable previous: LeafNode<'K, 'V> option = None
            let mutable offset = 0
            let leaves = ResizeArray<Node<'K, 'V>>()

            for size in partitionSizes pairs.Length (t - 1) maxKeys do
                let leaf = LeafNode<'K, 'V>()

                for index = offset to offset + size - 1 do
                    leaf.Keys.Add pairs[index].Key
                    leaf.Values.Add pairs[index].Value

                match previous with
                | Some previousLeaf -> previousLeaf.Next <- Some leaf
                | None -> firstLeaf <- leaf

                previous <- Some leaf
                leaves.Add(leaf :> Node<'K, 'V>)
                offset <- offset + size

            root <- buildLevel (List.ofSeq leaves)

    let rec findLeaf (node: Node<'K, 'V>) key =
        match node with
        | :? LeafNode<'K, 'V> as leaf -> leaf
        | :? InternalNode<'K, 'V> as internalNode ->
            let mutable index = 0

            while index < internalNode.Keys.Count && compare key internalNode.Keys[index] >= 0 do
                index <- index + 1

            findLeaf internalNode.Children[index] key
        | _ -> invalidOp "Unknown B+ tree node type."

    new() = BPlusTree<'K, 'V>(2)

    member _.MinimumDegree = t
    member _.Count = entries.Count
    member _.Size = entries.Count
    member _.IsEmpty = entries.Count = 0

    member _.Insert(key: 'K, value: 'V) =
        throwIfNull (nameof key) key
        entries[key] <- value
        rebuild ()

    member _.Delete(key: 'K) =
        throwIfNull (nameof key) key

        if entries.Remove key then
            rebuild ()

    member _.Search(key: 'K) =
        throwIfNull (nameof key) key
        let leaf = findLeaf root key
        let index = findKeyIndex leaf.Keys key

        if index < leaf.Keys.Count && compare leaf.Keys[index] key = 0 then
            Some leaf.Values[index]
        else
            None

    member _.Contains(key: 'K) =
        throwIfNull (nameof key) key
        let leaf = findLeaf root key
        let index = findKeyIndex leaf.Keys key
        index < leaf.Keys.Count && compare leaf.Keys[index] key = 0

    member _.MinKey() =
        if entries.Count = 0 then
            invalidOp "Tree is empty."

        firstLeaf.Keys[0]

    member _.MaxKey() =
        if entries.Count = 0 then
            invalidOp "Tree is empty."

        let mutable node = root
        let mutable doneDescending = false

        while not doneDescending do
            match node with
            | :? InternalNode<'K, 'V> as internalNode ->
                node <- internalNode.Children[internalNode.Children.Count - 1]
            | _ -> doneDescending <- true

        let leaf = node :?> LeafNode<'K, 'V>
        leaf.Keys[leaf.Keys.Count - 1]

    member _.RangeScan(low: 'K, high: 'K) =
        throwIfNull (nameof low) low
        throwIfNull (nameof high) high

        if compare low high > 0 then
            invalidArg (nameof low) "Low key must be less than or equal to high key."

        let result = ResizeArray<KeyValuePair<'K, 'V>>()
        let mutable current = Some(findLeaf root low)
        let mutable keepScanning = true

        while keepScanning && current.IsSome do
            let leaf = current.Value

            for index = 0 to leaf.Keys.Count - 1 do
                let key = leaf.Keys[index]

                if compare key high > 0 then
                    keepScanning <- false
                elif compare key low >= 0 then
                    result.Add(KeyValuePair(key, leaf.Values[index]))

            current <- leaf.Next

        List.ofSeq result

    member this.RangeQuery(low: 'K, high: 'K) =
        this.RangeScan(low, high)

    member _.FullScan() =
        let result = ResizeArray<KeyValuePair<'K, 'V>>(entries.Count)
        let mutable current = Some firstLeaf

        while current.IsSome do
            let leaf = current.Value

            for index = 0 to leaf.Keys.Count - 1 do
                result.Add(KeyValuePair(leaf.Keys[index], leaf.Values[index]))

            current <- leaf.Next

        List.ofSeq result

    member this.InOrder() =
        this.FullScan()

    member _.Height() =
        let mutable height = 0
        let mutable node = root
        let mutable doneDescending = false

        while not doneDescending do
            match node with
            | :? InternalNode<'K, 'V> as internalNode ->
                height <- height + 1
                node <- internalNode.Children[0]
            | _ -> doneDescending <- true

        height

    member this.IsValid() =
        if entries.Count = 0 then
            match root with
            | :? LeafNode<'K, 'V> as leaf ->
                obj.ReferenceEquals(leaf, firstLeaf) && leaf.Keys.Count = 0 && leaf.Next.IsNone
            | _ -> false
        else
            let scan = this.FullScan()

            if scan.Length <> entries.Count then
                false
            else
                let expected = entries |> Seq.toArray
                let mutable previous: 'K option = None
                let mutable valid = true
                let mutable index = 0

                while valid && index < scan.Length do
                    let entry = scan[index]

                    match previous with
                    | Some previousKey when compare previousKey entry.Key >= 0 -> valid <- false
                    | _ -> ()

                    if valid then
                        let current = expected[index]

                        let searchMatches =
                            match this.Search entry.Key with
                            | Some value -> EqualityComparer<'V>.Default.Equals(value, entry.Value)
                            | None -> false

                        if current.Key <> entry.Key
                           || not (EqualityComparer<'V>.Default.Equals(current.Value, entry.Value))
                           || not (this.Contains entry.Key)
                           || not searchMatches then
                            valid <- false

                    previous <- Some entry.Key
                    index <- index + 1

                valid

    override this.ToString() =
        $"BPlusTree(t={t}, size={entries.Count}, height={this.Height()})"

    interface IEnumerable<KeyValuePair<'K, 'V>> with
        member this.GetEnumerator() =
            (this.FullScan() :> seq<KeyValuePair<'K, 'V>>).GetEnumerator()

    interface IEnumerable with
        member this.GetEnumerator() =
            ((this :> IEnumerable<KeyValuePair<'K, 'V>>).GetEnumerator() :> IEnumerator)
