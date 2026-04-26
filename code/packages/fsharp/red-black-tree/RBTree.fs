namespace CodingAdventures.RedBlackTree

open System

type Color =
    | Red
    | Black

type Node =
    { Value: int
      Color: Color
      Left: Node option
      Right: Node option }

    member this.IsRed = this.Color = Red

module private Core =
    let isRed =
        function
        | Some node -> node.Color = Red
        | None -> false

    let toggle =
        function
        | Red -> Black
        | Black -> Red

    let withColor color node =
        if color = node.Color then
            node
        else
            { node with Color = color }

    let rotateLeft node =
        let right = Option.get node.Right

        { Value = right.Value
          Color = node.Color
          Left =
            Some
                { Value = node.Value
                  Color = Red
                  Left = node.Left
                  Right = right.Left }
          Right = right.Right }

    let rotateRight node =
        let left = Option.get node.Left

        { Value = left.Value
          Color = node.Color
          Left = left.Left
          Right =
            Some
                { Value = node.Value
                  Color = Red
                  Left = left.Right
                  Right = node.Right } }

    let flipOption =
        function
        | Some node -> Some(withColor (toggle node.Color) node)
        | None -> None

    let flipColors node =
        { Value = node.Value
          Color = toggle node.Color
          Left = flipOption node.Left
          Right = flipOption node.Right }

    let fixUp node =
        let mutable current = node

        if isRed current.Right && not (isRed current.Left) then
            current <- rotateLeft current

        let leftLeft =
            current.Left
            |> Option.bind _.Left

        if isRed current.Left && isRed leftLeft then
            current <- rotateRight current

        if isRed current.Left && isRed current.Right then
            current <- flipColors current

        current

    let moveRedLeft node =
        let mutable current = flipColors node
        let rightLeft = current.Right |> Option.bind _.Left

        if isRed rightLeft then
            current <- { current with Right = current.Right |> Option.map rotateRight }
            current <- rotateLeft current
            current <- flipColors current

        current

    let moveRedRight node =
        let mutable current = flipColors node
        let leftLeft = current.Left |> Option.bind _.Left

        if isRed leftLeft then
            current <- rotateRight current
            current <- flipColors current

        current

    let rec insertHelper node value =
        match node with
        | None ->
            { Value = value
              Color = Red
              Left = None
              Right = None }
        | Some h when value < h.Value -> fixUp { h with Left = Some(insertHelper h.Left value) }
        | Some h when value > h.Value -> fixUp { h with Right = Some(insertHelper h.Right value) }
        | Some h -> h

    let minValue node =
        let mutable current = node

        while current.Left.IsSome do
            current <- Option.get current.Left

        current.Value

    let rec deleteMin node =
        if node.Left.IsNone then
            None
        else
            let mutable current = node
            let leftLeft = current.Left |> Option.bind _.Left

            if not (isRed current.Left) && not (isRed leftLeft) then
                current <- moveRedLeft current

            Some(fixUp { current with Left = current.Left |> Option.bind deleteMin })

    let rec deleteHelper node value =
        if value < node.Value then
            let mutable current = node
            let leftLeft = current.Left |> Option.bind _.Left

            if not (isRed current.Left) && not (isRed leftLeft) then
                current <- moveRedLeft current

            Some(fixUp { current with Left = current.Left |> Option.bind (fun left -> deleteHelper left value) })
        else
            let mutable current = node

            if isRed current.Left then
                current <- rotateRight current

            if value = current.Value && current.Right.IsNone then
                None
            else
                let rightLeft = current.Right |> Option.bind _.Left

                if not (isRed current.Right) && not (isRed rightLeft) then
                    current <- moveRedRight current

                if value = current.Value then
                    let right = Option.get current.Right
                    let successor = minValue right
                    let newRight = deleteMin right

                    Some(
                        fixUp
                            { Value = successor
                              Color = current.Color
                              Left = current.Left
                              Right = newRight }
                    )
                else
                    Some(fixUp { current with Right = current.Right |> Option.bind (fun right -> deleteHelper right value) })

    let rec inOrder node acc =
        match node with
        | None -> acc
        | Some n ->
            let withLeft = inOrder n.Left acc
            let withCurrent = n.Value :: withLeft
            inOrder n.Right withCurrent

    let toSortedList root =
        inOrder root [] |> List.rev

    let rec checkNode node =
        match node with
        | None -> 1
        | Some n ->
            if n.Color = Red && (isRed n.Left || isRed n.Right) then
                -1
            else
                let leftBlackHeight = checkNode n.Left
                let rightBlackHeight = checkNode n.Right

                if leftBlackHeight = -1 || rightBlackHeight = -1 || leftBlackHeight <> rightBlackHeight then
                    -1
                else
                    leftBlackHeight + if n.Color = Black then 1 else 0

    let rec blackHeight node =
        match node with
        | None -> 0
        | Some n -> blackHeight n.Left + if n.Color = Black then 1 else 0

    let rec size node =
        match node with
        | None -> 0
        | Some n -> 1 + size n.Left + size n.Right

    let rec height node =
        match node with
        | None -> 0
        | Some n -> 1 + max (height n.Left) (height n.Right)

type RBTree private (root: Node option) =
    member _.Root = root

    static member Empty() = RBTree None

    member _.Insert(value: int) =
        Core.insertHelper root value
        |> Core.withColor Black
        |> Some
        |> RBTree

    member this.Delete(value: int) =
        if not (this.Contains value) then
            this
        else
            let newRoot = root |> Option.bind (fun node -> Core.deleteHelper node value)
            RBTree(newRoot |> Option.map (Core.withColor Black))

    member _.Contains(value: int) =
        let mutable node = root
        let mutable found = false

        while node.IsSome && not found do
            let current = Option.get node

            if value < current.Value then
                node <- current.Left
            elif value > current.Value then
                node <- current.Right
            else
                found <- true

        found

    member _.Min() =
        match root with
        | None -> None
        | Some node -> Some(Core.minValue node)

    member _.Max() =
        let mutable node = root

        match node with
        | None -> None
        | Some _ ->
            while (Option.get node).Right.IsSome do
                node <- (Option.get node).Right

            Some((Option.get node).Value)

    member _.Predecessor(value: int) =
        let mutable best = None
        let mutable node = root

        while node.IsSome do
            let current = Option.get node

            if value > current.Value then
                best <- Some current.Value
                node <- current.Right
            else
                node <- current.Left

        best

    member _.Successor(value: int) =
        let mutable best = None
        let mutable node = root

        while node.IsSome do
            let current = Option.get node

            if value < current.Value then
                best <- Some current.Value
                node <- current.Left
            else
                node <- current.Right

        best

    member _.ToSortedList() = Core.toSortedList root

    member this.KthSmallest(k: int) =
        let sorted: int list = this.ToSortedList()

        if k < 1 || k > sorted.Length then
            invalidOp $"k={k} out of range; tree has {sorted.Length} elements"

        sorted[k - 1]

    member _.IsValidRB() =
        match root with
        | None -> true
        | Some node -> node.Color = Black && Core.checkNode root <> -1

    member _.BlackHeight() = Core.blackHeight root

    member _.Size() = Core.size root

    member _.Height() = Core.height root

    member _.IsEmpty = root.IsNone

    override this.ToString() =
        $"RBTree{{size={this.Size()}, height={this.Height()}, blackHeight={this.BlackHeight()}}}"
