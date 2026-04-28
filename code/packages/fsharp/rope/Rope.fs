namespace CodingAdventures.Rope

open System
open System.Text

type private RopeNode =
    | Leaf of string
    | Internal of int * RopeNode * RopeNode

type Rope private (root: RopeNode option, length: int) =
    let rec appendTo (builder: StringBuilder) node =
        match node with
        | Leaf chunk -> builder.Append(chunk) |> ignore
        | Internal(_, left, right) ->
            appendTo builder left
            appendTo builder right

    let rec depth node =
        match node with
        | Leaf _ -> 0
        | Internal(_, left, right) -> 1 + max (depth left) (depth right)

    let rec isBalanced node =
        match node with
        | Leaf _ -> true
        | Internal(_, left, right) ->
            abs (depth left - depth right) <= 1 && isBalanced left && isBalanced right

    let text () =
        let builder = StringBuilder(length)

        match root with
        | Some node -> appendTo builder node
        | None -> ()

        builder.ToString()

    static member Empty() = Rope(None, 0)

    static member FromString(value: string) =
        if isNull value then
            nullArg (nameof value)

        if value.Length = 0 then
            Rope.Empty()
        else
            Rope(Some(Leaf value), value.Length)

    static member RopeFromString(value: string) =
        Rope.FromString(value)

    static member Concat(left: Rope, right: Rope) =
        if isNull (box left) then
            nullArg (nameof left)

        if isNull (box right) then
            nullArg (nameof right)

        match left.Root, right.Root with
        | None, _ -> right
        | _, None -> left
        | Some leftRoot, Some rightRoot ->
            Rope(Some(Internal(left.Length, leftRoot, rightRoot)), left.Length + right.Length)

    member private _.Root = root
    member _.Length = length
    member _.Count = length
    member _.IsEmpty = length = 0

    member this.Concat(right: Rope) =
        Rope.Concat(this, right)

    member this.Split(index: int) =
        let chars = this.ToString().ToCharArray()
        let splitAt = Math.Clamp(index, 0, chars.Length)
        let left = String(chars[0.. splitAt - 1])
        let right = String(chars[splitAt ..])
        Rope.FromString(left), Rope.FromString(right)

    member this.Insert(index: int, value: string) =
        if isNull value then
            nullArg (nameof value)

        let left, right = this.Split index
        Rope.Concat(Rope.Concat(left, Rope.FromString(value)), right)

    member this.Delete(start: int, deleteLength: int) =
        if deleteLength < 0 then
            raise (ArgumentOutOfRangeException(nameof deleteLength, "Length must be non-negative."))

        let chars = this.ToString().ToCharArray()
        let safeStart = Math.Clamp(start, 0, chars.Length)
        let finish = Math.Min(safeStart + deleteLength, chars.Length)
        let left = String(chars[0.. safeStart - 1])
        let right = String(chars[finish ..])
        Rope.Concat(Rope.FromString(left), Rope.FromString(right))

    member this.Index(index: int) =
        let value = this.ToString()

        if index >= 0 && index < value.Length then
            Some value[index]
        else
            None

    member this.Substring(start: int, finish: int) =
        let chars = this.ToString().ToCharArray()
        let safeStart = Math.Clamp(start, 0, chars.Length)
        let safeEnd = Math.Clamp(finish, 0, chars.Length)

        if safeStart >= safeEnd then
            String.Empty
        else
            String(chars[safeStart .. safeEnd - 1])

    member _.Depth() =
        root |> Option.map depth |> Option.defaultValue 0

    member _.IsBalanced() =
        root |> Option.map isBalanced |> Option.defaultValue true

    member this.Rebalance() =
        let rec buildBalanced (value: string) =
            if value.Length = 0 then
                Rope.Empty()
            elif value.Length = 1 then
                Rope.FromString(value)
            else
                let midpoint = value.Length / 2
                Rope.Concat(buildBalanced value[.. midpoint - 1], buildBalanced value[midpoint ..])

        buildBalanced (this.ToString())

    override _.ToString() =
        text ()
