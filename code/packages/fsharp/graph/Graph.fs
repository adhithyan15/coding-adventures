namespace CodingAdventures.Graph

open System
open System.Collections.Generic
open System.Text.Json

// Graph.fs -- One graph interface, two internal storage stories
// =============================================================
//
// This package is intentionally educational. We keep the public graph API
// stable while allowing the internal representation to vary:
//
//   AdjacencyList   -- compact for sparse graphs
//   AdjacencyMatrix -- direct edge lookup for dense graphs
//
// The higher-level algorithms later in the file are written against the public
// members of Graph<'T>, so BFS, DFS, shortest path, and minimum spanning tree
// all work regardless of which representation the caller chooses.

/// Select which internal storage model backs the graph.
type GraphRepr =
    | AdjacencyList = 0
    | AdjacencyMatrix = 1

[<Struct>]
/// A single undirected weighted edge in canonical endpoint order.
type WeightedEdge<'T when 'T : equality> =
    {
        Left: 'T
        Right: 'T
        Weight: float
    }

module internal NodeOrdering =
    // Hash-based containers do not preserve insertion order. We sort by a
    // canonical textual key so examples and tests stay deterministic.
    let canonicalKey<'T> (node: 'T) =
        try
            sprintf "%s:%s" typeof<'T>.FullName (JsonSerializer.Serialize(node))
        with _ ->
            sprintf "%s:%O" typeof<'T>.FullName node

    let compareNodes<'T> (left: 'T) (right: 'T) =
        StringComparer.Ordinal.Compare(canonicalKey left, canonicalKey right)

    let orderNodes<'T> (nodes: seq<'T>) =
        nodes |> Seq.sortWith compareNodes |> Seq.toList

    let canonicalEdge<'T> (left: 'T) (right: 'T) =
        if compareNodes left right <= 0 then
            left, right
        else
            right, left

    let edgeKey<'T> (left: 'T) (right: 'T) =
        let leftKey = canonicalKey left
        let rightKey = canonicalKey right
        if StringComparer.Ordinal.Compare(leftKey, rightKey) <= 0 then
            leftKey + "\u0000" + rightKey
        else
            rightKey + "\u0000" + leftKey

type Graph<'T when 'T : equality>(?repr: GraphRepr) =
    let representation = defaultArg repr GraphRepr.AdjacencyList

    // Adjacency-list storage:
    //   node -> (neighbor -> weight)
    let adjacency = Dictionary<'T, Dictionary<'T, float>>()

    // Adjacency-matrix storage:
    //   nodeList/nodeIndex translate values to integer positions
    //   matrix[row][col] stores Some(weight) / None
    let nodeList = ResizeArray<'T>()
    let nodeIndex = Dictionary<'T, int>()
    let matrix = ResizeArray<ResizeArray<float option>>()

    member _.Representation = representation

    member _.Size =
        if representation = GraphRepr.AdjacencyList then
            adjacency.Count
        else
            nodeList.Count

    member _.AddNode(node: 'T) =
        if representation = GraphRepr.AdjacencyList then
            // In list mode, a node exists once it has an empty neighbor map.
            if not (adjacency.ContainsKey(node)) then
                adjacency.[node] <- Dictionary<'T, float>()
        else if not (nodeIndex.ContainsKey(node)) then
            // In matrix mode we must grow the matrix in both dimensions.
            let index = nodeList.Count
            nodeList.Add(node)
            nodeIndex.[node] <- index

            for row in matrix do
                row.Add(None)

            let newRow = ResizeArray<float option>()
            for _ in 0 .. index do
                newRow.Add(None)
            matrix.Add(newRow)

    member _.RemoveNode(node: 'T) =
        if representation = GraphRepr.AdjacencyList then
            match adjacency.TryGetValue(node) with
            | true, neighbors ->
                for KeyValue(neighbor, _) in neighbors do
                    adjacency.[neighbor].Remove(node) |> ignore
                adjacency.Remove(node) |> ignore
            | _ ->
                raise (KeyNotFoundException(sprintf "Node not found: %A" node))
        else
            match nodeIndex.TryGetValue(node) with
            | true, index ->
                nodeIndex.Remove(node) |> ignore
                nodeList.RemoveAt(index)
                matrix.RemoveAt(index)
                for row in matrix do
                    row.RemoveAt(index)
                for i in index .. nodeList.Count - 1 do
                    nodeIndex.[nodeList.[i]] <- i
            | _ ->
                raise (KeyNotFoundException(sprintf "Node not found: %A" node))

    member _.HasNode(node: 'T) =
        if representation = GraphRepr.AdjacencyList then
            adjacency.ContainsKey(node)
        else
            nodeIndex.ContainsKey(node)

    member _.Nodes() =
        if representation = GraphRepr.AdjacencyList then
            adjacency.Keys |> Seq.toList |> NodeOrdering.orderNodes
        else
            nodeList |> Seq.toList |> NodeOrdering.orderNodes

    member this.AddEdge(left: 'T, right: 'T, ?weight: float) =
        let edgeWeight = defaultArg weight 1.0
        this.AddNode(left)
        this.AddNode(right)

        if representation = GraphRepr.AdjacencyList then
            adjacency.[left].[right] <- edgeWeight
            adjacency.[right].[left] <- edgeWeight
        else
            let leftIndex = nodeIndex.[left]
            let rightIndex = nodeIndex.[right]
            matrix.[leftIndex].[rightIndex] <- Some edgeWeight
            matrix.[rightIndex].[leftIndex] <- Some edgeWeight

    member _.RemoveEdge(left: 'T, right: 'T) =
        if representation = GraphRepr.AdjacencyList then
            match adjacency.TryGetValue(left), adjacency.TryGetValue(right) with
            | (true, leftNeighbors), (true, rightNeighbors) when leftNeighbors.ContainsKey(right) ->
                leftNeighbors.Remove(right) |> ignore
                rightNeighbors.Remove(left) |> ignore
            | _ ->
                raise (KeyNotFoundException(sprintf "Edge not found: %A -- %A" left right))
        else
            match nodeIndex.TryGetValue(left), nodeIndex.TryGetValue(right) with
            | (true, leftIndex), (true, rightIndex) when matrix.[leftIndex].[rightIndex].IsSome ->
                matrix.[leftIndex].[rightIndex] <- None
                matrix.[rightIndex].[leftIndex] <- None
            | _ ->
                raise (KeyNotFoundException(sprintf "Edge not found: %A -- %A" left right))

    member _.HasEdge(left: 'T, right: 'T) =
        if representation = GraphRepr.AdjacencyList then
            match adjacency.TryGetValue(left) with
            | true, neighbors -> neighbors.ContainsKey(right)
            | _ -> false
        else
            match nodeIndex.TryGetValue(left), nodeIndex.TryGetValue(right) with
            | (true, leftIndex), (true, rightIndex) -> matrix.[leftIndex].[rightIndex].IsSome
            | _ -> false

    member _.EdgeWeight(left: 'T, right: 'T) =
        if representation = GraphRepr.AdjacencyList then
            match adjacency.TryGetValue(left) with
            | true, neighbors when neighbors.ContainsKey(right) -> neighbors.[right]
            | _ -> raise (KeyNotFoundException(sprintf "Edge not found: %A -- %A" left right))
        else
            match nodeIndex.TryGetValue(left), nodeIndex.TryGetValue(right) with
            | (true, leftIndex), (true, rightIndex) ->
                match matrix.[leftIndex].[rightIndex] with
                | Some weight -> weight
                | None -> raise (KeyNotFoundException(sprintf "Edge not found: %A -- %A" left right))
            | _ ->
                raise (KeyNotFoundException(sprintf "Edge not found: %A -- %A" left right))

    member _.Edges() =
        let result = ResizeArray<WeightedEdge<'T>>()

        if representation = GraphRepr.AdjacencyList then
            // Each undirected edge is stored twice internally, so we deduplicate
            // using a canonical key before exposing the public edge list.
            let seen = HashSet<string>(StringComparer.Ordinal)
            for KeyValue(left, neighbors) in adjacency do
                for KeyValue(right, weight) in neighbors do
                    if seen.Add(NodeOrdering.edgeKey left right) then
                        let first, second = NodeOrdering.canonicalEdge left right
                        result.Add({ Left = first; Right = second; Weight = weight })
        else
            for row in 0 .. nodeList.Count - 1 do
                for col in row .. nodeList.Count - 1 do
                    match matrix.[row].[col] with
                    | Some weight ->
                        result.Add({ Left = nodeList.[row]; Right = nodeList.[col]; Weight = weight })
                    | None -> ()

        result
        |> Seq.toList
        |> List.sortWith (fun left right ->
            let byWeight = compare left.Weight right.Weight
            if byWeight <> 0 then
                byWeight
            else
                let byLeft = NodeOrdering.compareNodes left.Left right.Left
                if byLeft <> 0 then
                    byLeft
                else
                    NodeOrdering.compareNodes left.Right right.Right)

    member _.Neighbors(node: 'T) =
        if representation = GraphRepr.AdjacencyList then
            match adjacency.TryGetValue(node) with
            | true, neighbors -> neighbors.Keys |> Seq.toList |> NodeOrdering.orderNodes
            | _ -> raise (KeyNotFoundException(sprintf "Node not found: %A" node))
        else
            match nodeIndex.TryGetValue(node) with
            | true, index ->
                [
                    for col in 0 .. nodeList.Count - 1 do
                        if matrix.[index].[col].IsSome then
                            yield nodeList.[col]
                ]
                |> NodeOrdering.orderNodes
            | _ ->
                raise (KeyNotFoundException(sprintf "Node not found: %A" node))

    member _.NeighborsWeighted(node: 'T) =
        let result = Dictionary<'T, float>()

        if representation = GraphRepr.AdjacencyList then
            match adjacency.TryGetValue(node) with
            | true, neighbors ->
                for KeyValue(neighbor, weight) in neighbors do
                    result.[neighbor] <- weight
            | _ ->
                raise (KeyNotFoundException(sprintf "Node not found: %A" node))
        else
            match nodeIndex.TryGetValue(node) with
            | true, index ->
                for col in 0 .. nodeList.Count - 1 do
                    match matrix.[index].[col] with
                    | Some weight -> result.[nodeList.[col]] <- weight
                    | None -> ()
            | _ ->
                raise (KeyNotFoundException(sprintf "Node not found: %A" node))

        result

    member this.Degree(node: 'T) = this.Neighbors(node).Length

    override this.ToString() =
        sprintf "Graph(nodes=%d, edges=%d, repr=%O)" this.Size (this.Edges().Length) this.Representation

type private UnionFind<'T when 'T : equality>(nodes: seq<'T>) =
    let parent = Dictionary<'T, 'T>()
    let rank = Dictionary<'T, int>()
    let comparer = EqualityComparer<'T>.Default

    do
        for node in nodes do
            parent.[node] <- node
            rank.[node] <- 0

    let rec find (node: 'T) =
        let parentNode = parent.[node]
        if comparer.Equals(parentNode, node) then
            parentNode
        else
            let root = find parentNode
            parent.[node] <- root
            root

    member _.Find(node: 'T) = find node

    member _.Union(left: 'T, right: 'T) =
        let mutable leftRoot = find left
        let mutable rightRoot = find right

        if not (comparer.Equals(leftRoot, rightRoot)) then
            let mutable leftRank = rank.[leftRoot]
            let mutable rightRank = rank.[rightRoot]

            if leftRank < rightRank then
                let tempRoot = leftRoot
                let tempRank = leftRank
                leftRoot <- rightRoot
                rightRoot <- tempRoot
                leftRank <- rightRank
                rightRank <- tempRank

            parent.[rightRoot] <- leftRoot
            if leftRank = rightRank then
                rank.[leftRoot] <- leftRank + 1

module GraphAlgorithms =
    let private ensureNode (graph: Graph<'T>) (start: 'T) =
        if not (graph.HasNode(start)) then
            raise (KeyNotFoundException(sprintf "Node not found: %A" start))

    /// Breadth-first search visits nodes in increasing hop distance.
    let bfs (graph: Graph<'T>) (start: 'T) =
        ensureNode graph start

        let visited = HashSet<'T>()
        visited.Add(start) |> ignore

        let queue = Queue<'T>()
        queue.Enqueue(start)

        let result = ResizeArray<'T>()
        while queue.Count > 0 do
            let node = queue.Dequeue()
            result.Add(node)

            for neighbor in graph.Neighbors(node) do
                if visited.Add(neighbor) then
                    queue.Enqueue(neighbor)

        result |> Seq.toList

    /// Depth-first search explores one branch as far as it can before backtracking.
    let dfs (graph: Graph<'T>) (start: 'T) =
        ensureNode graph start

        let visited = HashSet<'T>()
        let stack = Stack<'T>()
        stack.Push(start)

        let result = ResizeArray<'T>()
        while stack.Count > 0 do
            let node = stack.Pop()
            if visited.Add(node) then
                result.Add(node)
                for neighbor in graph.Neighbors(node) |> List.rev do
                    if not (visited.Contains(neighbor)) then
                        stack.Push(neighbor)

        result |> Seq.toList

    let isConnected (graph: Graph<'T>) =
        if graph.Size = 0 then
            true
        else
            let start = graph.Nodes() |> List.head
            bfs graph start |> List.length = graph.Size

    let connectedComponents (graph: Graph<'T>) =
        let remaining = HashSet<'T>(graph.Nodes())
        let result = ResizeArray<HashSet<'T>>()

        while remaining.Count > 0 do
            let start = remaining |> Seq.toList |> NodeOrdering.orderNodes |> List.head
            let componentNodes = HashSet<'T>(bfs graph start)
            result.Add(componentNodes)
            remaining.ExceptWith(componentNodes)

        result |> Seq.toList

    let hasCycle (graph: Graph<'T>) =
        let visited = HashSet<'T>()
        let comparer = EqualityComparer<'T>.Default
        let mutable found = false

        for start in graph.Nodes() |> NodeOrdering.orderNodes do
            if not found && not (visited.Contains(start)) then
                let stack = Stack<'T * bool * 'T>()
                stack.Push((start, false, Unchecked.defaultof<'T>))

                while not found && stack.Count > 0 do
                    let node, hasParent, parent = stack.Pop()
                    if not (visited.Contains(node)) then
                        visited.Add(node) |> ignore
                        for neighbor in graph.Neighbors(node) do
                            if not (visited.Contains(neighbor)) then
                                stack.Push((neighbor, true, node))
                            elif (not hasParent) || not (comparer.Equals(neighbor, parent)) then
                                found <- true

        found

    let private buildPath (parents: Dictionary<'T, 'T option>) (start: 'T) (finish: 'T) =
        let comparer = EqualityComparer<'T>.Default
        let path = ResizeArray<'T>()
        let mutable current = Some finish

        while current.IsSome do
            let node = current.Value
            path.Add(node)

            if comparer.Equals(node, start) then
                current <- None
            else
                match parents.TryGetValue(node) with
                | true, parent ->
                    current <- parent
                | _ ->
                    current <- None
                    path.Clear()

        path |> Seq.rev |> Seq.toList

    let private bfsShortestPath (graph: Graph<'T>) (start: 'T) (finish: 'T) =
        let parents = Dictionary<'T, 'T option>()
        let seen = HashSet<'T>()
        seen.Add(start) |> ignore
        parents.[start] <- None

        let queue = Queue<'T>()
        queue.Enqueue(start)

        let comparer = EqualityComparer<'T>.Default
        let mutable doneSearching = false

        while not doneSearching && queue.Count > 0 do
            let node = queue.Dequeue()
            if comparer.Equals(node, finish) then
                doneSearching <- true
            else
                for neighbor in graph.Neighbors(node) do
                    if seen.Add(neighbor) then
                        parents.[neighbor] <- Some node
                        queue.Enqueue(neighbor)

        if parents.ContainsKey(finish) then
            buildPath parents start finish
        else
            []

    let private dijkstraShortestPath (graph: Graph<'T>) (start: 'T) (finish: 'T) =
        let distances = Dictionary<'T, float>()
        let parents = Dictionary<'T, 'T option>()
        let queue = ResizeArray<float * int * 'T>()

        for node in graph.Nodes() do
            distances.[node] <- Double.PositiveInfinity

        distances.[start] <- 0.0
        queue.Add((0.0, 0, start))

        let mutable sequence = 0
        let comparer = EqualityComparer<'T>.Default

        while queue.Count > 0 do
            let sorted =
                queue
                |> Seq.sortBy (fun (distance, seqNo, _) -> distance, seqNo)
                |> Seq.toList

            queue.Clear()
            for item in sorted.Tail do
                queue.Add(item)

            let distance, _, node = sorted.Head
            if distance <= distances.[node] then
                if not (comparer.Equals(node, finish)) then
                    for KeyValue(neighbor, weight) in graph.NeighborsWeighted(node) |> Seq.sortWith (fun (KeyValue(left, _)) (KeyValue(right, _)) -> NodeOrdering.compareNodes left right) do
                        let nextDistance = distance + weight
                        let knownDistance =
                            match distances.TryGetValue(neighbor) with
                            | true, value -> value
                            | _ -> Double.PositiveInfinity

                        if nextDistance < knownDistance then
                            distances.[neighbor] <- nextDistance
                            parents.[neighbor] <- Some node
                            sequence <- sequence + 1
                            queue.Add((nextDistance, sequence, neighbor))

        match distances.TryGetValue(finish) with
        | true, value when not (Double.IsPositiveInfinity(value)) -> buildPath parents start finish
        | _ -> []

    let shortestPath (graph: Graph<'T>) (start: 'T) (finish: 'T) =
        // If every edge has weight 1.0, BFS already gives the optimal route.
        // Once weights vary, we switch to Dijkstra.
        if not (graph.HasNode(start)) || not (graph.HasNode(finish)) then
            []
        elif EqualityComparer<'T>.Default.Equals(start, finish) then
            [ start ]
        elif graph.Edges() |> List.forall (fun edge -> edge.Weight = 1.0) then
            bfsShortestPath graph start finish
        else
            dijkstraShortestPath graph start finish

    let minimumSpanningTree (graph: Graph<'T>) =
        // Kruskal's algorithm:
        //   1. sort edges from cheapest to most expensive
        //   2. keep an edge only if it connects two previously separate trees
        if graph.Size <= 1 then
            []
        elif not (isConnected graph) then
            invalidOp "minimumSpanningTree: graph is not connected"
        else
            let edges = graph.Edges()
            let unionFind = UnionFind(graph.Nodes())
            let result = ResizeArray<WeightedEdge<'T>>()

            for edge in edges do
                if unionFind.Find(edge.Left) <> unionFind.Find(edge.Right) then
                    unionFind.Union(edge.Left, edge.Right)
                    result.Add(edge)

            result |> Seq.toList
