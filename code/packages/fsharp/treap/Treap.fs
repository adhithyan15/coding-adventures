namespace CodingAdventures.Treap

open System

type Node =
    { Key: int
      Priority: double
      Left: Node option
      Right: Node option }

type SplitResult =
    { Left: Node option
      Right: Node option }

type Treap private (root: Node option, random: Random) =
    static let rec splitNode (node: Node option) (key: int) : SplitResult =
        match node with
        | None -> { Left = None; Right = None }
        | Some current when current.Key <= key ->
            let parts = splitNode current.Right key
            { Left = Some { current with Right = parts.Left }
              Right = parts.Right }
        | Some current ->
            let parts = splitNode current.Left key
            { Left = parts.Left
              Right = Some { current with Left = parts.Right } }

    static let rec splitStrict (node: Node option) (key: int) : SplitResult =
        match node with
        | None -> { Left = None; Right = None }
        | Some current when current.Key < key ->
            let parts = splitStrict current.Right key
            { Left = Some { current with Right = parts.Left }
              Right = parts.Right }
        | Some current ->
            let parts = splitStrict current.Left key
            { Left = parts.Left
              Right = Some { current with Left = parts.Right } }

    static let rec mergeNodes (left: Node option) (right: Node option) : Node option =
        match left, right with
        | None, other -> other
        | other, None -> other
        | Some l, Some r when l.Priority > r.Priority ->
            Some { l with Right = mergeNodes l.Right right }
        | Some l, Some r ->
            Some { r with Left = mergeNodes left r.Left }

    static let rec inOrder (node: Node option) : int list =
        match node with
        | None -> []
        | Some current -> inOrder current.Left @ [ current.Key ] @ inOrder current.Right

    static let rec checkNode (node: Node option) (minKey: int option) (maxKey: int option) (maxPriority: double) : bool =
        match node with
        | None -> true
        | Some current ->
            let aboveMin =
                match minKey with
                | None -> true
                | Some low -> current.Key > low

            let belowMax =
                match maxKey with
                | None -> true
                | Some high -> current.Key < high

            aboveMin
            && belowMax
            && current.Priority <= maxPriority
            && checkNode current.Left minKey (Some current.Key) current.Priority
            && checkNode current.Right (Some current.Key) maxKey current.Priority

    static let rec countNodes (node: Node option) : int =
        match node with
        | None -> 0
        | Some current -> 1 + countNodes current.Left + countNodes current.Right

    static let rec heightOf (node: Node option) : int =
        match node with
        | None -> 0
        | Some current -> 1 + max (heightOf current.Left) (heightOf current.Right)

    static member Empty() = Treap(None, Random())
    static member Empty(random: Random) = Treap(None, random)
    static member WithSeed(seed: int) = Treap(None, Random seed)
    static member FromRoot(root: Node option, ?random: Random) = Treap(root, defaultArg random (Random()))

    static member Merge(left: Treap, right: Treap) =
        ArgumentNullException.ThrowIfNull(left)
        ArgumentNullException.ThrowIfNull(right)
        Treap(mergeNodes left.Root right.Root, Random())

    member _.Root = root
    member _.IsEmpty = root.IsNone
    member _.Size = countNodes root
    member _.Height = heightOf root

    member this.Insert(key: int) =
        if this.Contains key then
            this
        else
            this.InsertWithPriority(key, random.NextDouble())

    member this.InsertWithPriority(key: int, priority: double) =
        if this.Contains key then
            this
        else
            let parts = splitStrict root key
            let singleton: Node option =
                Some
                    { Key = key
                      Priority = priority
                      Left = None
                      Right = None }

            Treap(mergeNodes (mergeNodes parts.Left singleton) parts.Right, random)

    member this.Delete(key: int) =
        if not (this.Contains key) then
            this
        else
            let leftPart = splitStrict root key
            let rightPart = splitNode leftPart.Right key
            Treap(mergeNodes leftPart.Left rightPart.Right, random)

    member _.Split(key: int) : SplitResult = splitNode root key

    member _.Contains(key: int) =
        let rec loop (node: Node option) =
            match node with
            | None -> false
            | Some current when key < current.Key -> loop current.Left
            | Some current when key > current.Key -> loop current.Right
            | Some _ -> true

        loop root

    member _.Min() =
        let rec loop (node: Node option) =
            match node with
            | None -> None
            | Some current ->
                match current.Left with
                | None -> Some current.Key
                | child -> loop child

        loop root

    member _.Max() =
        let rec loop (node: Node option) =
            match node with
            | None -> None
            | Some current ->
                match current.Right with
                | None -> Some current.Key
                | child -> loop child

        loop root

    member _.Predecessor(key: int) =
        let rec loop (best: int option) (node: Node option) =
            match node with
            | None -> best
            | Some current when key > current.Key -> loop (Some current.Key) current.Right
            | Some current -> loop best current.Left

        loop None root

    member _.Successor(key: int) =
        let rec loop (best: int option) (node: Node option) =
            match node with
            | None -> best
            | Some current when key < current.Key -> loop (Some current.Key) current.Left
            | Some current -> loop best current.Right

        loop None root

    member this.KthSmallest(k: int) =
        let sorted = this.ToSortedList()

        if k < 1 || k > sorted.Length then
            invalidArg (nameof k) $"k={k} out of range; treap has {sorted.Length} elements."

        sorted[k - 1]

    member _.ToSortedList() : int list = inOrder root

    member _.IsValidTreap() = checkNode root None None Double.MaxValue

    override this.ToString() =
        $"Treap(size={this.Size}, height={this.Height})"
