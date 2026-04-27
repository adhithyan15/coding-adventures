namespace CodingAdventures.BinaryTree

open System
open System.Collections.Generic

type BinaryTreeNode<'T> =
    { Value: 'T
      Left: BinaryTreeNode<'T> option
      Right: BinaryTreeNode<'T> option }

    static member Leaf(value: 'T) =
        { Value = value
          Left = None
          Right = None }

    static member Create(value: 'T, left: BinaryTreeNode<'T> option, right: BinaryTreeNode<'T> option) =
        { Value = value
          Left = left
          Right = right }

module private BinaryTreeHelpers =
    let comparer<'T> = EqualityComparer<'T>.Default

    let rec find (root: BinaryTreeNode<'T> option) (value: 'T) =
        match root with
        | None -> None
        | Some node when comparer.Equals(node.Value, value) -> Some node
        | Some node ->
            match find node.Left value with
            | Some found -> Some found
            | None -> find node.Right value

    let rec isFull root =
        match root with
        | None -> true
        | Some { Left = None; Right = None } -> true
        | Some { Left = Some left; Right = Some right } -> isFull (Some left) && isFull (Some right)
        | Some _ -> false

    let isComplete root =
        let queue = Queue<BinaryTreeNode<'T> option>()
        queue.Enqueue root
        let mutable seenNone = false
        let mutable complete = true

        while complete && queue.Count > 0 do
            match queue.Dequeue() with
            | None -> seenNone <- true
            | Some node ->
                if seenNone then
                    complete <- false
                else
                    queue.Enqueue node.Left
                    queue.Enqueue node.Right

        complete

    let rec height root =
        match root with
        | None -> -1
        | Some node -> 1 + max (height node.Left) (height node.Right)

    let rec size root =
        match root with
        | None -> 0
        | Some node -> 1 + size node.Left + size node.Right

    let isPerfect root =
        let h = height root

        if h < 0 then
            size root = 0
        else
            size root = (1 <<< (h + 1)) - 1

    let rec buildFromLevelOrder (values: 'T option array) index =
        if index >= values.Length then
            None
        else
            match values[index] with
            | None -> None
            | Some value ->
                Some
                    { Value = value
                      Left = buildFromLevelOrder values ((2 * index) + 1)
                      Right = buildFromLevelOrder values ((2 * index) + 2) }

    let rec inorder root =
        match root with
        | None -> []
        | Some node -> inorder node.Left @ [ node.Value ] @ inorder node.Right

    let rec preorder root =
        match root with
        | None -> []
        | Some node -> node.Value :: (preorder node.Left @ preorder node.Right)

    let rec postorder root =
        match root with
        | None -> []
        | Some node -> postorder node.Left @ postorder node.Right @ [ node.Value ]

    let levelOrder root =
        match root with
        | None -> []
        | Some start ->
            let queue = Queue<BinaryTreeNode<'T>>()
            let output = ResizeArray<'T>()
            queue.Enqueue start

            while queue.Count > 0 do
                let node = queue.Dequeue()
                output.Add node.Value

                node.Left |> Option.iter queue.Enqueue
                node.Right |> Option.iter queue.Enqueue

            output |> Seq.toList

    let toArray root =
        let h = height root

        if h < 0 then
            []
        else
            let output: 'T option array = Array.create ((1 <<< (h + 1)) - 1) None

            let rec fill node index =
                match node with
                | None -> ()
                | Some current when index >= output.Length -> ()
                | Some current ->
                    output[index] <- Some current.Value
                    fill current.Left ((2 * index) + 1)
                    fill current.Right ((2 * index) + 2)

            fill root 0
            output |> Array.toList

    let toAscii root =
        let lines = ResizeArray<string>()

        let rec render (node: BinaryTreeNode<'T>) prefix isTail =
            let connector = if isTail then "`-- " else "|-- "
            lines.Add(sprintf "%s%s%A" prefix connector node.Value)

            let children =
                [ node.Left; node.Right ]
                |> List.choose id

            let nextPrefix = prefix + if isTail then "    " else "|   "

            children
            |> List.iteri (fun index child -> render child nextPrefix (index + 1 = children.Length))

        root |> Option.iter (fun node -> render node "" true)
        String.Join(Environment.NewLine, lines)

type BinaryTree<'T>(root: BinaryTreeNode<'T> option) =
    new() = BinaryTree<'T>(None)

    member _.Root = root

    static member Empty() = BinaryTree<'T>(None)

    static member Singleton(value: 'T) =
        BinaryTree<'T>(Some(BinaryTreeNode.Leaf value))

    static member WithRoot(root: BinaryTreeNode<'T> option) = BinaryTree<'T>(root)

    static member FromLevelOrder(values: seq<'T option>) =
        if isNull (box values) then
            nullArg "values"

        values
        |> Seq.toArray
        |> fun items -> BinaryTree<'T>(BinaryTreeHelpers.buildFromLevelOrder items 0)

    member _.Find(value: 'T) = BinaryTreeHelpers.find root value

    member this.LeftChild(value: 'T) = this.Find(value) |> Option.bind _.Left

    member this.RightChild(value: 'T) = this.Find(value) |> Option.bind _.Right

    member _.IsFull() = BinaryTreeHelpers.isFull root

    member _.IsComplete() = BinaryTreeHelpers.isComplete root

    member _.IsPerfect() = BinaryTreeHelpers.isPerfect root

    member _.Height() = BinaryTreeHelpers.height root

    member _.Size() = BinaryTreeHelpers.size root

    member _.InOrder() = BinaryTreeHelpers.inorder root

    member _.PreOrder() = BinaryTreeHelpers.preorder root

    member _.PostOrder() = BinaryTreeHelpers.postorder root

    member _.LevelOrder() = BinaryTreeHelpers.levelOrder root

    member _.ToArray() = BinaryTreeHelpers.toArray root

    member _.ToAscii() = BinaryTreeHelpers.toAscii root

    override _.ToString() =
        let rootLabel =
            match root with
            | None -> "null"
            | Some node -> sprintf "%A" node.Value

        sprintf "BinaryTree(root=%s, size=%d)" rootLabel (BinaryTreeHelpers.size root)

module BinaryTreeAlgorithms =
    let find value (root: BinaryTreeNode<'T> option) = BinaryTreeHelpers.find root value

    let leftChild value (root: BinaryTreeNode<'T> option) = find value root |> Option.bind _.Left

    let rightChild value (root: BinaryTreeNode<'T> option) = find value root |> Option.bind _.Right

    let isFull root = BinaryTreeHelpers.isFull root

    let isComplete root = BinaryTreeHelpers.isComplete root

    let isPerfect root = BinaryTreeHelpers.isPerfect root

    let height root = BinaryTreeHelpers.height root

    let size root = BinaryTreeHelpers.size root

    let inOrder root = BinaryTreeHelpers.inorder root

    let preOrder root = BinaryTreeHelpers.preorder root

    let postOrder root = BinaryTreeHelpers.postorder root

    let levelOrder root = BinaryTreeHelpers.levelOrder root

    let toArray root = BinaryTreeHelpers.toArray root

    let toAscii root = BinaryTreeHelpers.toAscii root
