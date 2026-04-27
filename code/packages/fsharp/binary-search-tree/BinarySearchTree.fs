namespace CodingAdventures.BinarySearchTree

open System.Collections.Generic

type BstNode<'T> =
    { Value: 'T
      Left: BstNode<'T> option
      Right: BstNode<'T> option
      Size: int }

    static member Leaf(value: 'T) =
        { Value = value
          Left = None
          Right = None
          Size = 1 }

    static member Create(value: 'T, left: BstNode<'T> option, right: BstNode<'T> option) =
        let nodeSize node =
            match node with
            | None -> 0
            | Some current -> current.Size

        { Value = value
          Left = left
          Right = right
          Size = 1 + nodeSize left + nodeSize right }

module private BstHelpers =
    let comparer<'T> = Comparer<'T>.Default

    let compareValues left right = comparer.Compare(left, right)

    let nodeSize root =
        match root with
        | None -> 0
        | Some node -> node.Size

    let rec search root value =
        match root with
        | None -> None
        | Some node ->
            match compareValues value node.Value with
            | comparison when comparison < 0 -> search node.Left value
            | comparison when comparison > 0 -> search node.Right value
            | _ -> Some node

    let rec insert root value =
        match root with
        | None -> Some(BstNode.Leaf value)
        | Some node ->
            match compareValues value node.Value with
            | comparison when comparison < 0 ->
                Some(BstNode.Create(node.Value, insert node.Left value, node.Right))
            | comparison when comparison > 0 ->
                Some(BstNode.Create(node.Value, node.Left, insert node.Right value))
            | _ -> root

    let rec private extractMin root =
        match root.Left with
        | None -> root.Right, root.Value
        | Some left ->
            let newLeft, minimum = extractMin left
            Some(BstNode.Create(root.Value, newLeft, root.Right)), minimum

    let rec delete root value =
        match root with
        | None -> None
        | Some node ->
            match compareValues value node.Value with
            | comparison when comparison < 0 ->
                Some(BstNode.Create(node.Value, delete node.Left value, node.Right))
            | comparison when comparison > 0 ->
                Some(BstNode.Create(node.Value, node.Left, delete node.Right value))
            | _ ->
                match node.Left, node.Right with
                | None, None -> None
                | Some left, None -> Some left
                | None, Some right -> Some right
                | Some left, Some right ->
                    let newRight, successor = extractMin right
                    Some(BstNode.Create(successor, Some left, newRight))

    let minValue root =
        let rec walk current =
            match current with
            | None -> None
            | Some node ->
                match node.Left with
                | None -> Some node.Value
                | Some _ -> walk node.Left

        walk root

    let maxValue root =
        let rec walk current =
            match current with
            | None -> None
            | Some node ->
                match node.Right with
                | None -> Some node.Value
                | Some _ -> walk node.Right

        walk root

    let predecessor root value =
        let rec walk current best =
            match current with
            | None -> best
            | Some node ->
                if compareValues value node.Value <= 0 then
                    walk node.Left best
                else
                    walk node.Right (Some node.Value)

        walk root None

    let successor root value =
        let rec walk current best =
            match current with
            | None -> best
            | Some node ->
                if compareValues value node.Value >= 0 then
                    walk node.Right best
                else
                    walk node.Left (Some node.Value)

        walk root None

    let rec kthSmallest root k =
        match root with
        | None -> None
        | Some _ when k <= 0 -> None
        | Some node ->
            let leftSize = nodeSize node.Left

            if k = leftSize + 1 then
                Some node.Value
            elif k <= leftSize then
                kthSmallest node.Left k
            else
                kthSmallest node.Right (k - leftSize - 1)

    let rec rank root value =
        match root with
        | None -> 0
        | Some node ->
            match compareValues value node.Value with
            | comparison when comparison < 0 -> rank node.Left value
            | comparison when comparison > 0 -> nodeSize node.Left + 1 + rank node.Right value
            | _ -> nodeSize node.Left

    let rec inOrder root =
        match root with
        | None -> []
        | Some node -> inOrder node.Left @ [ node.Value ] @ inOrder node.Right

    let rec height root =
        match root with
        | None -> -1
        | Some node -> 1 + max (height node.Left) (height node.Right)

    let rec buildBalanced (values: 'T array) start finish =
        if start >= finish then
            None
        else
            let mid = start + ((finish - start) / 2)

            Some(
                BstNode.Create(
                    values[mid],
                    buildBalanced values start mid,
                    buildBalanced values (mid + 1) finish
                )
            )

    let rec validate root minimum maximum =
        match root with
        | None -> Some(-1, 0)
        | Some node ->
            match minimum, maximum with
            | Some lower, _ when compareValues node.Value lower <= 0 -> None
            | _, Some upper when compareValues node.Value upper >= 0 -> None
            | _ ->
                match validate node.Left minimum (Some node.Value), validate node.Right (Some node.Value) maximum with
                | Some(leftHeight, leftSize), Some(rightHeight, rightSize) ->
                    let expectedSize = 1 + leftSize + rightSize

                    if node.Size = expectedSize then
                        Some(1 + max leftHeight rightHeight, expectedSize)
                    else
                        None
                | _ -> None

type BinarySearchTree<'T>(root: BstNode<'T> option) =
    new() = BinarySearchTree<'T>(None)

    member _.Root = root

    static member Empty() = BinarySearchTree<'T>(None)

    static member FromSortedArray(values: seq<'T>) =
        if isNull (box values) then
            nullArg "values"

        values
        |> Seq.toArray
        |> fun items -> BinarySearchTree<'T>(BstHelpers.buildBalanced items 0 items.Length)

    member _.Insert(value: 'T) = BinarySearchTree<'T>(BstHelpers.insert root value)

    member _.Delete(value: 'T) = BinarySearchTree<'T>(BstHelpers.delete root value)

    member _.Search(value: 'T) = BstHelpers.search root value

    member this.Contains(value: 'T) = this.Search(value).IsSome

    member _.MinValue() = BstHelpers.minValue root

    member _.MaxValue() = BstHelpers.maxValue root

    member _.Predecessor(value: 'T) = BstHelpers.predecessor root value

    member _.Successor(value: 'T) = BstHelpers.successor root value

    member _.KthSmallest(k: int) = BstHelpers.kthSmallest root k

    member _.Rank(value: 'T) = BstHelpers.rank root value

    member _.ToSortedArray() = BstHelpers.inOrder root

    member _.IsValid() = BstHelpers.validate root None None |> Option.isSome

    member _.Height() = BstHelpers.height root

    member _.Size() = BstHelpers.nodeSize root

    override _.ToString() =
        let rootLabel =
            match root with
            | None -> "null"
            | Some node -> sprintf "%A" node.Value

        sprintf "BinarySearchTree(root=%s, size=%d)" rootLabel (BstHelpers.nodeSize root)

module BinarySearchTreeAlgorithms =
    let search value (root: BstNode<'T> option) = BstHelpers.search root value

    let insert value (root: BstNode<'T> option) = BstHelpers.insert root value

    let delete value (root: BstNode<'T> option) = BstHelpers.delete root value

    let minValue root = BstHelpers.minValue root

    let maxValue root = BstHelpers.maxValue root

    let predecessor value root = BstHelpers.predecessor root value

    let successor value root = BstHelpers.successor root value

    let kthSmallest k root = BstHelpers.kthSmallest root k

    let rank value root = BstHelpers.rank root value

    let toSortedArray root = BstHelpers.inOrder root

    let isValid root = BstHelpers.validate root None None |> Option.isSome

    let height root = BstHelpers.height root

    let size root = BstHelpers.nodeSize root
