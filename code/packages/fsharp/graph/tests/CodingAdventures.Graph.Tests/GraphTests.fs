namespace CodingAdventures.Graph.Tests

open System
open System.Collections.Generic
open Xunit
open CodingAdventures.Graph

type GraphTests() =
    static member private Representations =
        [ GraphRepr.AdjacencyList; GraphRepr.AdjacencyMatrix ]

    static member private MakeGraph(repr: GraphRepr) =
        let graph = Graph<string>(repr)
        graph.AddEdge("London", "Paris", 300.0)
        graph.AddEdge("London", "Amsterdam", 520.0)
        graph.AddEdge("Paris", "Berlin", 878.0)
        graph.AddEdge("Amsterdam", "Berlin", 655.0)
        graph.AddEdge("Amsterdam", "Brussels", 180.0)
        graph

    static member private MakeTriangle(repr: GraphRepr) =
        let graph = Graph<string>(repr)
        graph.AddEdge("A", "B", 1.0)
        graph.AddEdge("B", "C", 1.0)
        graph.AddEdge("C", "A", 1.0)
        graph

    static member private MakePath(repr: GraphRepr) =
        let graph = Graph<string>(repr)
        graph.AddEdge("A", "B", 1.0)
        graph.AddEdge("B", "C", 1.0)
        graph

    static member private SetEquals<'T when 'T : equality>(actual: seq<'T>) (expected: seq<'T>) =
        HashSet<'T>(actual).SetEquals(expected)

    [<Fact>]
    member _.``Construction and node operations work``() =
        for repr in GraphTests.Representations do
            let graph = Graph<string>(repr)
            Assert.Equal(repr, graph.Representation)
            Assert.Equal(0, graph.Size)

            graph.AddNode("A")
            graph.AddNode("B")
            graph.AddNode("A")

            Assert.True(graph.HasNode("A"))
            Assert.True(GraphTests.SetEquals (graph.Nodes()) [ "A"; "B" ])
            Assert.Equal(2, graph.Size)

            graph.RemoveNode("A")
            Assert.False(graph.HasNode("A"))
            Assert.True(graph.HasNode("B"))

    [<Fact>]
    member _.``Missing nodes and edges throw``() =
        for repr in GraphTests.Representations do
            let graph = Graph<string>(repr)
            graph.AddNode("A")
            graph.AddNode("B")

            Assert.Throws<KeyNotFoundException>(fun () -> graph.RemoveNode("missing")) |> ignore
            Assert.Throws<KeyNotFoundException>(fun () -> graph.RemoveEdge("A", "B")) |> ignore
            Assert.Throws<KeyNotFoundException>(fun () -> graph.EdgeWeight("A", "B") |> ignore) |> ignore
            Assert.Throws<KeyNotFoundException>(fun () -> graph.Neighbors("missing") |> ignore) |> ignore

    [<Fact>]
    member _.``Edge operations are undirected and support self loops``() =
        for repr in GraphTests.Representations do
            let graph = Graph<string>(repr)
            graph.AddEdge("A", "B", 2.5)
            graph.AddEdge("A", "A", 0.0)

            Assert.True(graph.HasEdge("A", "B"))
            Assert.True(graph.HasEdge("B", "A"))
            Assert.True(graph.HasEdge("A", "A"))
            Assert.Equal(2.5, graph.EdgeWeight("B", "A"))
            Assert.Equal(0.0, graph.EdgeWeight("A", "A"))
            Assert.Contains("A", graph.Neighbors("A"))
            Assert.Contains("B", graph.Neighbors("A"))

            graph.RemoveEdge("A", "B")
            Assert.False(graph.HasEdge("A", "B"))
            Assert.True(graph.HasNode("A"))
            Assert.True(graph.HasNode("B"))

    [<Fact>]
    member _.``Neighborhood queries and edges are deterministic``() =
        for repr in GraphTests.Representations do
            let graph = GraphTests.MakeGraph(repr)
            Assert.Equal(3, graph.Degree("Amsterdam"))
            Assert.Equal<string list>([ "Berlin"; "Brussels"; "London" ], graph.Neighbors("Amsterdam"))

            let weighted = graph.NeighborsWeighted("Amsterdam")
            Assert.Equal(520.0, weighted.["London"])
            Assert.Equal(180.0, weighted.["Brussels"])

            let edges = graph.Edges()
            Assert.Equal(5, edges.Length)
            Assert.Contains({ Left = "Amsterdam"; Right = "Brussels"; Weight = 180.0 }, edges)
            Assert.Contains({ Left = "London"; Right = "Paris"; Weight = 300.0 }, edges)

    [<Fact>]
    member _.``Traversals visit reachable nodes in stable order``() =
        for repr in GraphTests.Representations do
            Assert.Equal<string list>([ "A"; "B"; "C" ], GraphAlgorithms.bfs (GraphTests.MakePath(repr)) "A")
            Assert.Equal<string list>([ "A"; "B"; "C" ], GraphAlgorithms.dfs (GraphTests.MakePath(repr)) "A")

            let graph = Graph<string>(repr)
            graph.AddEdge("A", "B")
            graph.AddNode("C")

            Assert.Equal<string list>([ "A"; "B" ], GraphAlgorithms.bfs graph "A")
            Assert.Equal<string list>([ "A"; "B" ], GraphAlgorithms.dfs graph "A")
            Assert.Throws<KeyNotFoundException>(fun () -> GraphAlgorithms.bfs graph "missing" |> ignore) |> ignore

    [<Fact>]
    member _.``Connectivity and components match expectations``() =
        for repr in GraphTests.Representations do
            Assert.True(GraphAlgorithms.isConnected (GraphTests.MakeGraph(repr)))

            let graph = Graph<string>(repr)
            graph.AddEdge("A", "B")
            graph.AddEdge("B", "C")
            graph.AddEdge("D", "E")
            graph.AddNode("F")

            Assert.False(GraphAlgorithms.isConnected graph)

            let components = GraphAlgorithms.connectedComponents graph
            Assert.Equal(3, components.Length)
            Assert.Contains(components, fun componentNodes -> componentNodes.SetEquals([ "A"; "B"; "C" ]))
            Assert.Contains(components, fun componentNodes -> componentNodes.SetEquals([ "D"; "E" ]))
            Assert.Contains(components, fun componentNodes -> componentNodes.SetEquals([ "F" ]))

    [<Fact>]
    member _.``Cycle detection matches triangle and path cases``() =
        for repr in GraphTests.Representations do
            Assert.True(GraphAlgorithms.hasCycle (GraphTests.MakeTriangle(repr)))
            Assert.False(GraphAlgorithms.hasCycle (GraphTests.MakePath(repr)))

    [<Fact>]
    member _.``Shortest path chooses the correct strategy``() =
        for repr in GraphTests.Representations do
            Assert.Equal<string list>([ "A"; "B"; "C" ], GraphAlgorithms.shortestPath (GraphTests.MakePath(repr)) "A" "C")

            let weighted = Graph<string>(repr)
            weighted.AddEdge("A", "B", 1.0)
            weighted.AddEdge("B", "D", 10.0)
            weighted.AddEdge("A", "C", 3.0)
            weighted.AddEdge("C", "D", 3.0)

            Assert.Equal<string list>([ "A"; "C"; "D" ], GraphAlgorithms.shortestPath weighted "A" "D")
            Assert.Equal<string list>([ "London"; "Amsterdam"; "Berlin" ], GraphAlgorithms.shortestPath (GraphTests.MakeGraph(repr)) "London" "Berlin")

            let disconnected = Graph<string>(repr)
            disconnected.AddNode("A")
            disconnected.AddNode("B")
            Assert.Empty(GraphAlgorithms.shortestPath disconnected "A" "B")
            Assert.Empty(GraphAlgorithms.shortestPath disconnected "A" "missing")

    [<Fact>]
    member _.``Minimum spanning tree returns the cheapest connecting edges``() =
        for repr in GraphTests.Representations do
            let mst = GraphAlgorithms.minimumSpanningTree (GraphTests.MakeGraph(repr))
            Assert.Equal(4, mst.Length)
            Assert.Equal(1655.0, mst |> List.sumBy (fun edge -> edge.Weight))

            let triangle = GraphAlgorithms.minimumSpanningTree (GraphTests.MakeTriangle(repr))
            Assert.Equal(2, triangle.Length)
            Assert.Equal(2.0, triangle |> List.sumBy (fun edge -> edge.Weight))

            let disconnected = Graph<string>(repr)
            disconnected.AddEdge("A", "B", 1.0)
            disconnected.AddNode("C")
            Assert.Throws<InvalidOperationException>(fun () -> GraphAlgorithms.minimumSpanningTree disconnected |> ignore) |> ignore

    [<Fact>]
    member _.``Property bags track graph node and edge metadata``() =
        for repr in GraphTests.Representations do
            let graph = Graph<string>(repr)

            graph.SetGraphProperty("name", box "city-map")
            graph.SetGraphProperty("version", box 1)
            Assert.Equal(box "city-map", graph.GraphProperties().["name"])
            Assert.Equal(box 1, graph.GraphProperties().["version"])
            graph.RemoveGraphProperty("version")
            Assert.False(graph.GraphProperties().ContainsKey("version"))

            let nodeProperties = Dictionary<string, obj>(StringComparer.Ordinal)
            nodeProperties.["kind"] <- box "input"
            graph.AddNode("A", nodeProperties)

            let nextNodeProperties = Dictionary<string, obj>(StringComparer.Ordinal)
            nextNodeProperties.["trainable"] <- box false
            graph.AddNode("A", nextNodeProperties)

            graph.SetNodeProperty("A", "slot", box 0)
            Assert.Equal(box "input", graph.NodeProperties("A").["kind"])
            Assert.Equal(box false, graph.NodeProperties("A").["trainable"])
            Assert.Equal(box 0, graph.NodeProperties("A").["slot"])
            graph.RemoveNodeProperty("A", "slot")
            Assert.False(graph.NodeProperties("A").ContainsKey("slot"))

            let edgeProperties = Dictionary<string, obj>(StringComparer.Ordinal)
            edgeProperties.["role"] <- box "distance"
            graph.AddEdge("A", "B", 2.5, edgeProperties)
            Assert.Equal(box "distance", graph.EdgeProperties("B", "A").["role"])
            Assert.Equal(box 2.5, graph.EdgeProperties("B", "A").["weight"])
            graph.SetEdgeProperty("B", "A", "weight", box 7.0)
            Assert.Equal(7.0, graph.EdgeWeight("A", "B"))
            graph.SetEdgeProperty("A", "B", "trainable", box true)
            graph.RemoveEdgeProperty("A", "B", "role")
            Assert.Equal(box true, graph.EdgeProperties("A", "B").["trainable"])
            Assert.False(graph.EdgeProperties("A", "B").ContainsKey("role"))

            graph.RemoveEdge("A", "B")
            Assert.Throws<KeyNotFoundException>(fun () -> graph.EdgeProperties("A", "B") |> ignore) |> ignore

    [<Fact>]
    member _.``Numeric nodes are supported``() =
        for repr in GraphTests.Representations do
            let graph = Graph<int>(repr)
            graph.AddEdge(1, 2, 1.0)
            graph.AddEdge(2, 3, 1.0)

            Assert.Equal<int list>([ 1; 2; 3 ], GraphAlgorithms.shortestPath graph 1 3)
            Assert.True(GraphAlgorithms.isConnected graph)

    [<Fact>]
    member _.``Graph string representation includes state``() =
        let graph = GraphTests.MakeGraph(GraphRepr.AdjacencyList)
        let text = graph.ToString()
        Assert.Contains("Graph(nodes=5", text)
        Assert.Contains("edges=5", text)
        Assert.Contains("AdjacencyList", text)
