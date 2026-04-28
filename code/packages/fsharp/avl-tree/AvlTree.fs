namespace CodingAdventures.AvlTree

open System.Collections.Generic

type AvlNode<'T> =
    { Value: 'T
      Left: AvlNode<'T> option
      Right: AvlNode<'T> option
      Height: int
      Size: int }

    static member Leaf(value: 'T) =
        { Value = value
          Left = None
          Right = None
          Height = 0
          Size = 1 }

    static member Create(value: 'T, left: AvlNode<'T> option, right: AvlNode<'T> option) =
        let nodeHeight node =
            match node with
            | None -> -1
            | Some current -> current.Height

        let nodeSize node =
            match node with
            | None -> 0
            | Some current -> current.Size

        { Value = value
          Left = left
          Right = right
          Height = 1 + max (nodeHeight left) (nodeHeight right)
          Size = 1 + nodeSize left + nodeSize right }

module private AvlHelpers =
    let comparer<'T> = Comparer<'T>.Default

    let compareValues left right = comparer.Compare(left, right)

    let nodeHeight root =
        match root with
        | None -> -1
        | Some node -> node.Height

    let nodeSize root =
        match root with
        | None -> 0
        | Some node -> node.Size

    let balanceFactor root =
        match root with
        | None -> 0
        | Some node -> nodeHeight node.Left - nodeHeight node.Right

    let rotateLeft root =
        match root.Right with
        | None -> root
        | Some right ->
            let newLeft = AvlNode.Create(root.Value, root.Left, right.Left)
            AvlNode.Create(right.Value, Some newLeft, right.Right)

    let rotateRight root =
        match root.Left with
        | None -> root
        | Some left ->
            let newRight = AvlNode.Create(root.Value, left.Right, root.Right)
            AvlNode.Create(left.Value, left.Left, Some newRight)

    let rebalance node =
        let bf = balanceFactor (Some node)

        if bf > 1 then
            let left =
                match node.Left with
                | Some child when balanceFactor (Some child) < 0 -> Some(rotateLeft child)
                | other -> other

            rotateRight (AvlNode.Create(node.Value, left, node.Right))
        elif bf < -1 then
            let right =
                match node.Right with
                | Some child when balanceFactor (Some child) > 0 -> Some(rotateRight child)
                | other -> other

            rotateLeft (AvlNode.Create(node.Value, node.Left, right))
        else
            node

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
        | None -> Some(AvlNode.Leaf value)
        | Some node ->
            match compareValues value node.Value with
            | comparison when comparison < 0 ->
                Some(rebalance (AvlNode.Create(node.Value, insert node.Left value, node.Right)))
            | comparison when comparison > 0 ->
                Some(rebalance (AvlNode.Create(node.Value, node.Left, insert node.Right value)))
            | _ -> root

    let rec private extractMin root =
        match root.Left with
        | None -> root.Right, root.Value
        | Some left ->
            let newLeft, minimum = extractMin left
            Some(rebalance (AvlNode.Create(root.Value, newLeft, root.Right))), minimum

    let rec delete root value =
        match root with
        | None -> None
        | Some node ->
            match compareValues value node.Value with
            | comparison when comparison < 0 ->
                Some(rebalance (AvlNode.Create(node.Value, delete node.Left value, node.Right)))
            | comparison when comparison > 0 ->
                Some(rebalance (AvlNode.Create(node.Value, node.Left, delete node.Right value)))
            | _ ->
                match node.Left, node.Right with
                | None, None -> None
                | Some left, None -> Some left
                | None, Some right -> Some right
                | Some left, Some right ->
                    let newRight, successor = extractMin right
                    Some(rebalance (AvlNode.Create(successor, Some left, newRight)))

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

    let rec validateBst root minimum maximum =
        match root with
        | None -> true
        | Some node ->
            match minimum, maximum with
            | Some lower, _ when compareValues node.Value lower <= 0 -> false
            | _, Some upper when compareValues node.Value upper >= 0 -> false
            | _ -> validateBst node.Left minimum (Some node.Value) && validateBst node.Right (Some node.Value) maximum

    let rec validateAvl root minimum maximum =
        match root with
        | None -> Some(-1, 0)
        | Some node ->
            match minimum, maximum with
            | Some lower, _ when compareValues node.Value lower <= 0 -> None
            | _, Some upper when compareValues node.Value upper >= 0 -> None
            | _ ->
                match validateAvl node.Left minimum (Some node.Value), validateAvl node.Right (Some node.Value) maximum with
                | Some(leftHeight, leftSize), Some(rightHeight, rightSize) ->
                    let expectedHeight = 1 + max leftHeight rightHeight
                    let expectedSize = 1 + leftSize + rightSize

                    if node.Height = expectedHeight
                       && node.Size = expectedSize
                       && abs (leftHeight - rightHeight) <= 1 then
                        Some(expectedHeight, expectedSize)
                    else
                        None
                | _ -> None

type AvlTree<'T>(root: AvlNode<'T> option) =
    new() = AvlTree<'T>(None)

    member _.Root = root

    static member Empty() = AvlTree<'T>(None)

    static member FromValues(values: seq<'T>) =
        if isNull (box values) then
            nullArg "values"

        values
        |> Seq.fold (fun (tree: AvlTree<'T>) value -> tree.Insert(value)) (AvlTree<'T>.Empty())

    member _.Insert(value: 'T) = AvlTree<'T>(AvlHelpers.insert root value)

    member _.Delete(value: 'T) = AvlTree<'T>(AvlHelpers.delete root value)

    member _.Search(value: 'T) = AvlHelpers.search root value

    member this.Contains(value: 'T) = this.Search(value).IsSome

    member _.MinValue() = AvlHelpers.minValue root

    member _.MaxValue() = AvlHelpers.maxValue root

    member _.Predecessor(value: 'T) = AvlHelpers.predecessor root value

    member _.Successor(value: 'T) = AvlHelpers.successor root value

    member _.KthSmallest(k: int) = AvlHelpers.kthSmallest root k

    member _.Rank(value: 'T) = AvlHelpers.rank root value

    member _.ToSortedArray() = AvlHelpers.inOrder root

    member _.IsValidBst() = AvlHelpers.validateBst root None None

    member _.IsValidAvl() = AvlHelpers.validateAvl root None None |> Option.isSome

    member _.BalanceFactor(node: AvlNode<'T> option) = AvlHelpers.balanceFactor node

    member _.Height() = AvlHelpers.nodeHeight root

    member _.Size() = AvlHelpers.nodeSize root

    override _.ToString() =
        let rootLabel =
            match root with
            | None -> "null"
            | Some node -> sprintf "%A" node.Value

        sprintf "AvlTree(root=%s, size=%d, height=%d)" rootLabel (AvlHelpers.nodeSize root) (AvlHelpers.nodeHeight root)

module AvlTreeAlgorithms =
    let search value (root: AvlNode<'T> option) = AvlHelpers.search root value

    let insert value (root: AvlNode<'T> option) = AvlHelpers.insert root value

    let delete value (root: AvlNode<'T> option) = AvlHelpers.delete root value

    let minValue root = AvlHelpers.minValue root

    let maxValue root = AvlHelpers.maxValue root

    let predecessor value root = AvlHelpers.predecessor root value

    let successor value root = AvlHelpers.successor root value

    let kthSmallest k root = AvlHelpers.kthSmallest root k

    let rank value root = AvlHelpers.rank root value

    let toSortedArray root = AvlHelpers.inOrder root

    let isValidBst root = AvlHelpers.validateBst root None None

    let isValidAvl root = AvlHelpers.validateAvl root None None |> Option.isSome

    let balanceFactor root = AvlHelpers.balanceFactor root

    let height root = AvlHelpers.nodeHeight root

    let size root = AvlHelpers.nodeSize root
