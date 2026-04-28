namespace CodingAdventures.BTree

open System
open System.Collections.Generic

type private Node<'K, 'V when 'K: comparison>(isLeaf: bool) =
    let keys = ResizeArray<'K>()
    let values = ResizeArray<'V>()
    let children = ResizeArray<Node<'K, 'V>>()

    member _.IsLeaf = isLeaf
    member _.Keys = keys
    member _.Values = values
    member _.Children = children

    member _.IsFull(minimumDegree: int) =
        keys.Count = 2 * minimumDegree - 1

    member _.FindKeyIndex(key: 'K) =
        let mutable low = 0
        let mutable high = keys.Count

        while low < high do
            let mid = (low + high) / 2

            if compare keys[mid] key < 0 then
                low <- mid + 1
            else
                high <- mid

        low

type BTree<'K, 'V when 'K: comparison>(minimumDegree: int) =
    let t =
        if minimumDegree < 2 then
            invalidArg (nameof minimumDegree) "Minimum degree must be at least 2."

        minimumDegree

    let mutable root = Node<'K, 'V>(true)
    let mutable count = 0

    let throwIfNull name value =
        if isNull (box value) then
            nullArg name

    let rec minNode (node: Node<'K, 'V>) =
        let mutable current = node

        while not current.IsLeaf do
            current <- current.Children[0]

        current

    let rec maxNode (node: Node<'K, 'V>) =
        let mutable current = node

        while not current.IsLeaf do
            current <- current.Children[current.Children.Count - 1]

        current

    let splitChild (parent: Node<'K, 'V>) childIndex =
        let child = parent.Children[childIndex]
        let right = Node<'K, 'V>(child.IsLeaf)
        let mid = t - 1

        parent.Keys.Insert(childIndex, child.Keys[mid])
        parent.Values.Insert(childIndex, child.Values[mid])
        parent.Children.Insert(childIndex + 1, right)

        right.Keys.AddRange(child.Keys.GetRange(mid + 1, child.Keys.Count - mid - 1))
        right.Values.AddRange(child.Values.GetRange(mid + 1, child.Values.Count - mid - 1))

        if not child.IsLeaf then
            right.Children.AddRange(child.Children.GetRange(t, child.Children.Count - t))
            child.Children.RemoveRange(t, child.Children.Count - t)

        child.Keys.RemoveRange(mid, child.Keys.Count - mid)
        child.Values.RemoveRange(mid, child.Values.Count - mid)

    let rec insertNonFull (node: Node<'K, 'V>) key value =
        let mutable index = node.FindKeyIndex key

        if index < node.Keys.Count && compare node.Keys[index] key = 0 then
            node.Values[index] <- value
            false
        elif node.IsLeaf then
            node.Keys.Insert(index, key)
            node.Values.Insert(index, value)
            true
        else
            if node.Children[index].IsFull t then
                splitChild node index
                let comparison = compare key node.Keys[index]

                if comparison = 0 then
                    node.Values[index] <- value
                    false
                else
                    if comparison > 0 then
                        index <- index + 1

                    insertNonFull node.Children[index] key value
            else
                insertNonFull node.Children[index] key value

    let rec searchRecursive (node: Node<'K, 'V>) key =
        let index = node.FindKeyIndex key

        if index < node.Keys.Count && compare node.Keys[index] key = 0 then
            Some node.Values[index]
        elif node.IsLeaf then
            None
        else
            searchRecursive node.Children[index] key

    let rec containsRecursive (node: Node<'K, 'V>) key =
        let index = node.FindKeyIndex key

        if index < node.Keys.Count && compare node.Keys[index] key = 0 then
            true
        elif node.IsLeaf then
            false
        else
            containsRecursive node.Children[index] key

    let mergeChildren (parent: Node<'K, 'V>) leftIndex =
        let left = parent.Children[leftIndex]
        let right = parent.Children[leftIndex + 1]

        left.Keys.Add(parent.Keys[leftIndex])
        left.Values.Add(parent.Values[leftIndex])
        parent.Keys.RemoveAt leftIndex
        parent.Values.RemoveAt leftIndex
        parent.Children.RemoveAt(leftIndex + 1)

        left.Keys.AddRange right.Keys
        left.Values.AddRange right.Values

        if not left.IsLeaf then
            left.Children.AddRange right.Children

        left

    let rec deleteRecursive (node: Node<'K, 'V>) key =
        let mutable index = node.FindKeyIndex key
        let found = index < node.Keys.Count && compare node.Keys[index] key = 0

        if found then
            if node.IsLeaf then
                node.Keys.RemoveAt index
                node.Values.RemoveAt index
            else
                let leftChild = node.Children[index]
                let rightChild = node.Children[index + 1]

                if leftChild.Keys.Count >= t then
                    let predecessor = maxNode leftChild
                    let predecessorKey = predecessor.Keys[predecessor.Keys.Count - 1]
                    let predecessorValue = predecessor.Values[predecessor.Values.Count - 1]
                    node.Keys[index] <- predecessorKey
                    node.Values[index] <- predecessorValue
                    deleteRecursive leftChild predecessorKey
                elif rightChild.Keys.Count >= t then
                    let successor = minNode rightChild
                    let successorKey = successor.Keys[0]
                    let successorValue = successor.Values[0]
                    node.Keys[index] <- successorKey
                    node.Values[index] <- successorValue
                    deleteRecursive rightChild successorKey
                else
                    let merged = mergeChildren node index
                    deleteRecursive merged key
        elif not node.IsLeaf then
            let ensureMinKeys (parent: Node<'K, 'V>) childIndex =
                let child = parent.Children[childIndex]

                if child.Keys.Count >= t then
                    childIndex
                elif childIndex > 0 && parent.Children[childIndex - 1].Keys.Count >= t then
                    let leftSibling = parent.Children[childIndex - 1]
                    child.Keys.Insert(0, parent.Keys[childIndex - 1])
                    child.Values.Insert(0, parent.Values[childIndex - 1])

                    let last = leftSibling.Keys.Count - 1
                    parent.Keys[childIndex - 1] <- leftSibling.Keys[last]
                    parent.Values[childIndex - 1] <- leftSibling.Values[last]
                    leftSibling.Keys.RemoveAt last
                    leftSibling.Values.RemoveAt last

                    if not leftSibling.IsLeaf then
                        child.Children.Insert(0, leftSibling.Children[leftSibling.Children.Count - 1])
                        leftSibling.Children.RemoveAt(leftSibling.Children.Count - 1)

                    childIndex
                elif childIndex < parent.Children.Count - 1 && parent.Children[childIndex + 1].Keys.Count >= t then
                    let rightSibling = parent.Children[childIndex + 1]
                    child.Keys.Add(parent.Keys[childIndex])
                    child.Values.Add(parent.Values[childIndex])

                    parent.Keys[childIndex] <- rightSibling.Keys[0]
                    parent.Values[childIndex] <- rightSibling.Values[0]
                    rightSibling.Keys.RemoveAt 0
                    rightSibling.Values.RemoveAt 0

                    if not rightSibling.IsLeaf then
                        child.Children.Add(rightSibling.Children[0])
                        rightSibling.Children.RemoveAt 0

                    childIndex
                elif childIndex > 0 then
                    mergeChildren parent (childIndex - 1) |> ignore
                    childIndex - 1
                else
                    mergeChildren parent childIndex |> ignore
                    childIndex

            index <- ensureMinKeys node index
            deleteRecursive node.Children[index] key

    let rec collectInOrder (node: Node<'K, 'V>) (result: ResizeArray<KeyValuePair<'K, 'V>>) =
        if node.IsLeaf then
            for index in 0 .. node.Keys.Count - 1 do
                result.Add(KeyValuePair(node.Keys[index], node.Values[index]))
        else
            for index in 0 .. node.Keys.Count - 1 do
                collectInOrder node.Children[index] result
                result.Add(KeyValuePair(node.Keys[index], node.Values[index]))

            collectInOrder node.Children[node.Children.Count - 1] result

    let rec validate (node: Node<'K, 'V>) minKey maxKey depth (leafDepth: int byref) isRoot =
        let keyCount = node.Keys.Count

        let keyCountValid =
            if isRoot then
                count = 0 || keyCount >= 1
            else
                keyCount >= t - 1 && keyCount <= 2 * t - 1

        if not keyCountValid then
            false
        else
            let mutable valid = true
            let mutable index = 0

            while valid && index < keyCount do
                let key = node.Keys[index]

                match minKey with
                | Some low when compare key low <= 0 -> valid <- false
                | _ -> ()

                match maxKey with
                | Some high when compare key high >= 0 -> valid <- false
                | _ -> ()

                if index > 0 && compare key node.Keys[index - 1] <= 0 then
                    valid <- false

                index <- index + 1

            if not valid then
                false
            elif node.IsLeaf then
                if node.Children.Count <> 0 then
                    false
                else
                    if leafDepth < 0 then
                        leafDepth <- depth

                    leafDepth = depth
            elif node.Children.Count <> keyCount + 1 then
                false
            else
                let mutable childIndex = 0

                while valid && childIndex <= keyCount do
                    let low = if childIndex > 0 then Some node.Keys[childIndex - 1] else minKey
                    let high = if childIndex < keyCount then Some node.Keys[childIndex] else maxKey

                    if not (validate node.Children[childIndex] low high (depth + 1) &leafDepth false) then
                        valid <- false

                    childIndex <- childIndex + 1

                valid

    new() = BTree<'K, 'V>(2)

    member _.MinimumDegree = t
    member _.Count = count
    member _.Size = count
    member _.IsEmpty = count = 0

    member _.Insert(key: 'K, value: 'V) =
        throwIfNull (nameof key) key

        if root.IsFull t then
            let newRoot = Node<'K, 'V>(false)
            newRoot.Children.Add root
            splitChild newRoot 0
            root <- newRoot

        if insertNonFull root key value then
            count <- count + 1

    member _.Delete(key: 'K) =
        throwIfNull (nameof key) key

        if not (containsRecursive root key) then
            raise (KeyNotFoundException($"Key not found: {key}"))

        deleteRecursive root key
        count <- count - 1

        if root.Keys.Count = 0 && root.Children.Count > 0 then
            root <- root.Children[0]

    member _.Search(key: 'K) =
        if isNull (box key) then
            None
        else
            searchRecursive root key

    member _.Contains(key: 'K) =
        if isNull (box key) then
            false
        else
            containsRecursive root key

    member _.MinKey() =
        if count = 0 then
            invalidOp "Tree is empty."

        (minNode root).Keys[0]

    member _.MaxKey() =
        if count = 0 then
            invalidOp "Tree is empty."

        let node = maxNode root
        node.Keys[node.Keys.Count - 1]

    member this.RangeQuery(low: 'K, high: 'K) =
        throwIfNull (nameof low) low
        throwIfNull (nameof high) high

        (this.InOrder(): KeyValuePair<'K, 'V> list)
        |> List.takeWhile (fun (entry: KeyValuePair<'K, 'V>) -> compare entry.Key high <= 0)
        |> List.filter (fun (entry: KeyValuePair<'K, 'V>) -> compare entry.Key low >= 0)

    member _.InOrder() =
        let result = ResizeArray<KeyValuePair<'K, 'V>>(count)
        collectInOrder root result
        List.ofSeq result

    member _.Height() =
        let mutable node = root
        let mutable height = 0

        while not node.IsLeaf do
            node <- node.Children[0]
            height <- height + 1

        height

    member _.IsValid() =
        if count = 0 then
            root.Keys.Count = 0 && root.IsLeaf
        else
            let mutable leafDepth = -1
            validate root None None 0 &leafDepth true

    override this.ToString() =
        $"BTree(t={t}, size={count}, height={this.Height()})"
