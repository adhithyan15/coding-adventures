namespace CodingAdventures.DirectedGraph.FSharp

open System
open System.Collections.Generic

type CycleError(message: string, cycle: IReadOnlyList<string>) =
    inherit Exception(message)

    member _.Cycle = cycle

type NodeNotFoundError(node: string) =
    inherit Exception(sprintf "Node not found: \"%s\"" node)

    member _.Node = node

type EdgeNotFoundError(fromNode: string, toNode: string) =
    inherit Exception(sprintf "Edge not found: \"%s\" -> \"%s\"" fromNode toNode)

    member _.FromNode = fromNode
    member _.ToNode = toNode

type Graph(?allowSelfLoops: bool) =
    let allowSelfLoops = defaultArg allowSelfLoops false
    let forward = Dictionary<string, HashSet<string>>(StringComparer.Ordinal)
    let reverse = Dictionary<string, HashSet<string>>(StringComparer.Ordinal)

    let ensureNode (node: string) =
        if not (forward.ContainsKey(node)) then
            raise (NodeNotFoundError(node))

    let orderedStrings (values: seq<string>) =
        values |> Seq.sortWith (fun left right -> StringComparer.Ordinal.Compare(left, right)) |> Seq.toList

    member _.AllowSelfLoops = allowSelfLoops
    member _.Size = forward.Count

    member _.AddNode(node: string) =
        if not (forward.ContainsKey(node)) then
            forward.[node] <- HashSet<string>(StringComparer.Ordinal)

        if not (reverse.ContainsKey(node)) then
            reverse.[node] <- HashSet<string>(StringComparer.Ordinal)

    member this.RemoveNode(node: string) =
        ensureNode node

        for successor in forward.[node] |> Seq.toArray do
            reverse.[successor].Remove(node) |> ignore

        for predecessor in reverse.[node] |> Seq.toArray do
            forward.[predecessor].Remove(node) |> ignore

        forward.Remove(node) |> ignore
        reverse.Remove(node) |> ignore

    member _.HasNode(node: string) = forward.ContainsKey(node)

    member _.Nodes() = orderedStrings forward.Keys

    member this.AddEdge(fromNode: string, toNode: string) =
        if fromNode = toNode && not allowSelfLoops then
            invalidArg "toNode" (sprintf "Self-loops are not allowed: \"%s\" -> \"%s\"" fromNode toNode)

        this.AddNode(fromNode)
        this.AddNode(toNode)
        forward.[fromNode].Add(toNode) |> ignore
        reverse.[toNode].Add(fromNode) |> ignore

    member _.RemoveEdge(fromNode: string, toNode: string) =
        ensureNode fromNode

        if not (forward.ContainsKey(toNode)) || not (forward.[fromNode].Contains(toNode)) then
            raise (EdgeNotFoundError(fromNode, toNode))

        forward.[fromNode].Remove(toNode) |> ignore
        reverse.[toNode].Remove(fromNode) |> ignore

    member _.HasEdge(fromNode: string, toNode: string) =
        forward.ContainsKey(fromNode) && forward.[fromNode].Contains(toNode)

    member _.Successors(node: string) =
        ensureNode node
        orderedStrings forward.[node]

    member _.Predecessors(node: string) =
        ensureNode node
        orderedStrings reverse.[node]

    member _.Edges() =
        [
            for KeyValue(source, targets) in forward do
                for target in targets do
                    yield source, target
        ]
        |> List.sortWith (fun (leftSource, leftTarget) (rightSource, rightTarget) ->
            let bySource = StringComparer.Ordinal.Compare(leftSource, rightSource)
            if bySource <> 0 then bySource else StringComparer.Ordinal.Compare(leftTarget, rightTarget))

    member this.TransitiveClosure(startNode: string) =
        this.Reach(startNode, this.Successors)

    member this.TransitiveDependents(startNode: string) =
        this.Reach(startNode, this.Predecessors)

    member this.TopologicalSort() =
        let indegree = Dictionary<string, int>(StringComparer.Ordinal)
        for node in forward.Keys do
            indegree.[node] <- reverse.[node].Count

        let available =
            ResizeArray(
                indegree
                |> Seq.choose (fun (KeyValue(node, count)) -> if count = 0 then Some node else None)
                |> Seq.sortWith (fun left right -> StringComparer.Ordinal.Compare(left, right)))
        let order = ResizeArray<string>()

        let popSmallest () =
            let smallest =
                available
                |> Seq.minBy id

            available.Remove(smallest) |> ignore
            smallest

        while available.Count > 0 do
            let node = popSmallest ()
            order.Add(node)

            for successor in this.Successors(node) do
                indegree.[successor] <- indegree.[successor] - 1
                if indegree.[successor] = 0 then
                    available.Add(successor)

        if order.Count <> forward.Count then
            raise (CycleError("Graph contains a cycle", this.FindCycle()))

        order |> Seq.toList

    member this.IndependentGroups() =
        let indegree = Dictionary<string, int>(StringComparer.Ordinal)
        for node in forward.Keys do
            indegree.[node] <- reverse.[node].Count

        let remaining = HashSet<string>(forward.Keys, StringComparer.Ordinal)
        let groups = ResizeArray<IReadOnlyList<string>>()

        while remaining.Count > 0 do
            let layer =
                remaining
                |> Seq.filter (fun node -> indegree.[node] = 0)
                |> Seq.sortWith (fun left right -> StringComparer.Ordinal.Compare(left, right))
                |> Seq.toList

            if List.isEmpty layer then
                raise (CycleError("Graph contains a cycle", this.FindCycle()))

            groups.Add(layer)
            for node in layer do
                remaining.Remove(node) |> ignore
                for successor in this.Successors(node) do
                    indegree.[successor] <- indegree.[successor] - 1

        groups |> Seq.toList

    member this.AffectedNodes(changedNodes: seq<string>) =
        let affected = HashSet<string>(StringComparer.Ordinal)
        for node in changedNodes do
            if this.HasNode(node) then
                affected.Add(node) |> ignore
                affected.UnionWith(this.TransitiveClosure(node))

        this.TopologicalSort() |> List.filter affected.Contains

    member private _.Reach(startNode: string, next: string -> string list) =
        ensureNode startNode

        let visited = HashSet<string>(StringComparer.Ordinal)
        let stack = Stack<string>()
        stack.Push(startNode)

        while stack.Count > 0 do
            let node = stack.Pop()
            for neighbor in next(node) do
                if visited.Add(neighbor) then
                    stack.Push(neighbor)

        visited :> ISet<string>

    member private this.FindCycle() =
        let visited = HashSet<string>(StringComparer.Ordinal)
        let onStack = HashSet<string>(StringComparer.Ordinal)
        let parent = Dictionary<string, string option>(StringComparer.Ordinal)

        let rec findCycleFrom node =
            if onStack.Contains(node) then
                let mutable cycle = [ node ]
                let mutable current =
                    match parent.TryGetValue(node) with
                    | true, value -> value
                    | _ -> None

                while current.IsSome && current.Value <> node do
                    cycle <- current.Value :: cycle
                    current <-
                        match parent.TryGetValue(current.Value) with
                        | true, value -> value
                        | _ -> None

                cycle @ [ node ]

            elif not (visited.Add(node)) then
                []

            else
                onStack.Add(node) |> ignore
                let mutable found = []
                for successor in this.Successors(node) do
                    if List.isEmpty found then
                        parent.[successor] <- Some node
                        let cycle = findCycleFrom successor
                        if not (List.isEmpty cycle) then
                            found <- cycle

                onStack.Remove(node) |> ignore
                found

        forward.Keys
        |> Seq.tryPick (fun node ->
            let cycle = findCycleFrom node
            if List.isEmpty cycle then None else Some cycle)
        |> Option.defaultValue []

type LabeledDirectedGraph(?allowSelfLoops: bool) =
    let graph = Graph(defaultArg allowSelfLoops false)
    let labels = Dictionary<string, Dictionary<string, HashSet<string>>>(StringComparer.Ordinal)

    member _.AddNode(node: string) =
        graph.AddNode(node)
        if not (labels.ContainsKey(node)) then
            labels.[node] <- Dictionary<string, HashSet<string>>(StringComparer.Ordinal)

    member this.AddEdge(fromNode: string, toNode: string, label: string) =
        this.AddNode(fromNode)
        this.AddNode(toNode)
        graph.AddEdge(fromNode, toNode)

        if not (labels.[fromNode].ContainsKey(toNode)) then
            labels.[fromNode].[toNode] <- HashSet<string>(StringComparer.Ordinal)

        labels.[fromNode].[toNode].Add(label) |> ignore

    member _.Labels(fromNode: string, toNode: string) =
        match labels.TryGetValue(fromNode) with
        | true, targets when targets.ContainsKey(toNode) ->
            targets.[toNode] |> Seq.sortWith (fun left right -> StringComparer.Ordinal.Compare(left, right)) |> Seq.toList
        | _ ->
            []

    member _.TopologicalSort() = graph.TopologicalSort()
    member _.TransitiveClosure(startNode: string) = graph.TransitiveClosure(startNode)

module DirectedGraph =
    let create allowSelfLoops = Graph(allowSelfLoops)
    let createDefault () = Graph()

    let addEdge fromNode toNode (graph: Graph) =
        graph.AddEdge(fromNode, toNode)
        graph

    let topologicalSort (graph: Graph) = graph.TopologicalSort()
