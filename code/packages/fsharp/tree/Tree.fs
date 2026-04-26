namespace CodingAdventures.Tree

open System
open System.Collections.Generic
open System.Text

type TreeErrorKind =
    | NodeNotFound = 0
    | DuplicateNode = 1
    | RootRemoval = 2

type private TreeMessages =
    static member Create(kind: TreeErrorKind, node: string option) =
        let nodeText = Option.defaultValue "" node

        match kind with
        | TreeErrorKind.NodeNotFound -> "Node not found in tree: " + nodeText
        | TreeErrorKind.DuplicateNode -> "Node already exists in tree: " + nodeText
        | TreeErrorKind.RootRemoval -> "Cannot remove the root node."
        | _ -> "Tree operation failed."

type TreeException(kind: TreeErrorKind, node: string option) =
    inherit InvalidOperationException(TreeMessages.Create(kind, node))

    member _.Kind = kind
    member _.Node = node

/// A rooted tree with unique string node names.
type Tree(root: string) =
    do
        if String.IsNullOrWhiteSpace(root) then
            invalidArg (nameof root) "Root must not be empty."

    let parents = Dictionary<string, string option>(StringComparer.Ordinal)
    let children = Dictionary<string, SortedSet<string>>(StringComparer.Ordinal)

    do
        parents[root] <- None
        children[root] <- SortedSet<string>(StringComparer.Ordinal)

    let hasNode node = parents.ContainsKey node

    let ensureNode node =
        if String.IsNullOrWhiteSpace node then
            invalidArg (nameof node) "Node must not be empty."

        if not (hasNode node) then
            raise (TreeException(TreeErrorKind.NodeNotFound, Some node))

    let rec visitPreorder node (result: ResizeArray<string>) =
        result.Add node

        for child in children[node] do
            visitPreorder child result

    let rec visitPostorder node (result: ResizeArray<string>) =
        for child in children[node] do
            visitPostorder child result

        result.Add node

    let collectSubtreeNodes node =
        let result = ResizeArray<string>()
        let queue = Queue<string>()
        queue.Enqueue node

        while queue.Count > 0 do
            let current = queue.Dequeue()
            result.Add current

            for child in children[current] do
                queue.Enqueue child

        List.ofSeq result

    let rec copyChildrenInto node (target: Tree) =
        for child in children[node] do
            target.AddChild(node, child) |> ignore
            copyChildrenInto child target

    let rec appendAscii (builder: StringBuilder) (node: string) (prefix: string) (isLast: bool) =
        builder.Append(prefix).Append(if isLast then "`-- " else "|-- ").AppendLine(node) |> ignore
        let childArray = children[node] |> Seq.toArray

        for index in 0 .. childArray.Length - 1 do
            appendAscii builder childArray[index] (prefix + (if isLast then "    " else "|   ")) (index = childArray.Length - 1)

    member _.Root = root
    member _.Size = parents.Count

    member this.AddChild(parent: string, child: string) =
        if String.IsNullOrWhiteSpace parent then
            invalidArg (nameof parent) "Parent must not be empty."

        if String.IsNullOrWhiteSpace child then
            invalidArg (nameof child) "Child must not be empty."

        if not (hasNode parent) then
            raise (TreeException(TreeErrorKind.NodeNotFound, Some parent))

        if hasNode child then
            raise (TreeException(TreeErrorKind.DuplicateNode, Some child))

        parents[child] <- Some parent
        children[child] <- SortedSet<string>(StringComparer.Ordinal)
        children[parent].Add child |> ignore
        this

    member this.RemoveSubtree(node: string) =
        ensureNode node

        if node = root then
            raise (TreeException(TreeErrorKind.RootRemoval, Some node))

        let nodes = collectSubtreeNodes node

        for current in nodes |> List.rev do
            for child in children[current] |> Seq.toArray do
                children[current].Remove child |> ignore

            match parents[current] with
            | Some parent -> children[parent].Remove current |> ignore
            | None -> ()

            children.Remove current |> ignore
            parents.Remove current |> ignore

        this

    member _.Parent(node: string) =
        ensureNode node
        parents[node]

    member _.Children(node: string) =
        ensureNode node
        children[node] |> Seq.toList

    member this.Siblings(node: string) =
        ensureNode node

        match parents[node] with
        | None -> []
        | Some parent -> this.Children(parent) |> List.filter ((<>) node)

    member _.IsLeaf(node: string) =
        ensureNode node
        children[node].Count = 0

    member _.IsRoot(node: string) =
        ensureNode node
        node = root

    member _.Depth(node: string) =
        ensureNode node
        let mutable depth = 0
        let mutable current = node
        let mutable keepGoing = true

        while keepGoing do
            match parents[current] with
            | Some parent ->
                depth <- depth + 1
                current <- parent
            | None -> keepGoing <- false

        depth

    member this.Height() =
        parents.Keys |> Seq.map this.Depth |> Seq.max

    member _.Nodes() =
        parents.Keys |> Seq.sort |> Seq.toList

    member _.Leaves() =
        parents.Keys |> Seq.filter (fun node -> children[node].Count = 0) |> Seq.sort |> Seq.toList

    member _.HasNode(node: string) =
        hasNode node

    member _.Preorder() =
        let result = ResizeArray<string>()
        visitPreorder root result
        List.ofSeq result

    member _.Postorder() =
        let result = ResizeArray<string>()
        visitPostorder root result
        List.ofSeq result

    member _.LevelOrder() =
        let result = ResizeArray<string>()
        let queue = Queue<string>()
        queue.Enqueue root

        while queue.Count > 0 do
            let node = queue.Dequeue()
            result.Add node

            for child in children[node] do
                queue.Enqueue child

        List.ofSeq result

    member _.PathTo(node: string) =
        ensureNode node
        let path = ResizeArray<string>()
        let mutable current = node
        let mutable keepGoing = true

        while keepGoing do
            path.Add current

            match parents[current] with
            | Some parent -> current <- parent
            | None -> keepGoing <- false

        path |> Seq.rev |> Seq.toList

    member this.LowestCommonAncestor(left: string, right: string) =
        ensureNode left
        ensureNode right
        let leftPath = this.PathTo left
        let rightPath = this.PathTo right
        let mutable ancestor = root
        let mutable index = 0

        while index < min leftPath.Length rightPath.Length && leftPath[index] = rightPath[index] do
            ancestor <- leftPath[index]
            index <- index + 1

        ancestor

    member this.Lca(left: string, right: string) =
        this.LowestCommonAncestor(left, right)

    member _.Subtree(node: string) =
        ensureNode node
        let tree = Tree(node)
        copyChildrenInto node tree
        tree

    member _.ToAscii() =
        let builder = StringBuilder()
        builder.AppendLine(root) |> ignore
        let childArray = children[root] |> Seq.toArray

        for index in 0 .. childArray.Length - 1 do
            appendAscii builder childArray[index] "" (index = childArray.Length - 1)

        builder.ToString().TrimEnd()

    override this.ToString() =
        $"Tree(root={root}, size={parents.Count}, height={this.Height()})"
