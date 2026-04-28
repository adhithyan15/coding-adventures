namespace CodingAdventures.Graph.Tests;

public sealed class GraphTests
{
    private static GraphRepr[] Representations() =>
        [GraphRepr.AdjacencyList, GraphRepr.AdjacencyMatrix];

    private static Graph<string> MakeGraph(GraphRepr repr)
    {
        var graph = new Graph<string>(repr);
        graph.AddEdge("London", "Paris", 300);
        graph.AddEdge("London", "Amsterdam", 520);
        graph.AddEdge("Paris", "Berlin", 878);
        graph.AddEdge("Amsterdam", "Berlin", 655);
        graph.AddEdge("Amsterdam", "Brussels", 180);
        return graph;
    }

    private static Graph<string> MakeTriangle(GraphRepr repr)
    {
        var graph = new Graph<string>(repr);
        graph.AddEdge("A", "B", 1);
        graph.AddEdge("B", "C", 1);
        graph.AddEdge("C", "A", 1);
        return graph;
    }

    private static Graph<string> MakePath(GraphRepr repr)
    {
        var graph = new Graph<string>(repr);
        graph.AddEdge("A", "B", 1);
        graph.AddEdge("B", "C", 1);
        return graph;
    }

    [Fact]
    public void ConstructionAndBasicNodeOperationsWork()
    {
        foreach (var repr in Representations())
        {
            var graph = new Graph<string>(repr);
            Assert.Equal(repr, graph.Representation);
            Assert.Equal(0, graph.Size);

            graph.AddNode("A");
            graph.AddNode("B");
            graph.AddNode("A");

            Assert.True(graph.HasNode("A"));
            Assert.True(new HashSet<string>(graph.Nodes()).SetEquals(["A", "B"]));
            Assert.Equal(2, graph.Size);

            graph.RemoveNode("A");
            Assert.False(graph.HasNode("A"));
            Assert.True(graph.HasNode("B"));
        }
    }

    [Fact]
    public void MissingNodesAndEdgesThrowHelpfulErrors()
    {
        foreach (var repr in Representations())
        {
            var graph = new Graph<string>(repr);
            graph.AddNode("A");
            graph.AddNode("B");

            Assert.Throws<KeyNotFoundException>(() => graph.RemoveNode("missing"));
            Assert.Throws<KeyNotFoundException>(() => graph.RemoveEdge("A", "B"));
            Assert.Throws<KeyNotFoundException>(() => graph.EdgeWeight("A", "B"));
            Assert.Throws<KeyNotFoundException>(() => graph.Neighbors("missing"));
        }
    }

    [Fact]
    public void EdgeOperationsRemainUndirectedAndSupportSelfLoops()
    {
        foreach (var repr in Representations())
        {
            var graph = new Graph<string>(repr);
            graph.AddEdge("A", "B", 2.5);
            graph.AddEdge("A", "A", 0.0);

            Assert.True(graph.HasEdge("A", "B"));
            Assert.True(graph.HasEdge("B", "A"));
            Assert.True(graph.HasEdge("A", "A"));
            Assert.Equal(2.5, graph.EdgeWeight("B", "A"));
            Assert.Equal(0.0, graph.EdgeWeight("A", "A"));

            var neighbors = graph.Neighbors("A");
            Assert.Contains("A", neighbors);
            Assert.Contains("B", neighbors);

            graph.RemoveEdge("A", "B");
            Assert.False(graph.HasEdge("A", "B"));
            Assert.True(graph.HasNode("A"));
            Assert.True(graph.HasNode("B"));
        }
    }

    [Fact]
    public void NeighborhoodQueriesAndEdgesAreDeterministic()
    {
        foreach (var repr in Representations())
        {
            var graph = MakeGraph(repr);
            Assert.Equal(3, graph.Degree("Amsterdam"));
            Assert.Equal(["Berlin", "Brussels", "London"], graph.Neighbors("Amsterdam"));

            var weighted = graph.NeighborsWeighted("Amsterdam");
            Assert.Equal(520, weighted["London"]);
            Assert.Equal(180, weighted["Brussels"]);

            var edges = graph.Edges();
            Assert.Equal(5, edges.Count);
            Assert.Contains(new WeightedEdge<string>("Amsterdam", "Brussels", 180), edges);
            Assert.Contains(new WeightedEdge<string>("London", "Paris", 300), edges);
        }
    }

    [Fact]
    public void TraversalsVisitReachableNodesInStableOrder()
    {
        foreach (var repr in Representations())
        {
            Assert.Equal(["A", "B", "C"], GraphAlgorithms.Bfs(MakePath(repr), "A"));
            Assert.Equal(["A", "B", "C"], GraphAlgorithms.Dfs(MakePath(repr), "A"));

            var graph = new Graph<string>(repr);
            graph.AddEdge("A", "B");
            graph.AddNode("C");

            Assert.Equal(["A", "B"], GraphAlgorithms.Bfs(graph, "A"));
            Assert.Equal(["A", "B"], GraphAlgorithms.Dfs(graph, "A"));
            Assert.Throws<KeyNotFoundException>(() => GraphAlgorithms.Bfs(graph, "missing"));
        }
    }

    [Fact]
    public void ConnectivityAndComponentsMatchExpectations()
    {
        foreach (var repr in Representations())
        {
            Assert.True(GraphAlgorithms.IsConnected(MakeGraph(repr)));

            var graph = new Graph<string>(repr);
            graph.AddEdge("A", "B");
            graph.AddEdge("B", "C");
            graph.AddEdge("D", "E");
            graph.AddNode("F");

            Assert.False(GraphAlgorithms.IsConnected(graph));

            var components = GraphAlgorithms.ConnectedComponents(graph);
            Assert.Equal(3, components.Count);
            Assert.Contains(components, component => component.SetEquals(["A", "B", "C"]));
            Assert.Contains(components, component => component.SetEquals(["D", "E"]));
            Assert.Contains(components, component => component.SetEquals(["F"]));
        }
    }

    [Fact]
    public void CycleDetectionMatchesTriangleAndPathCases()
    {
        foreach (var repr in Representations())
        {
            Assert.True(GraphAlgorithms.HasCycle(MakeTriangle(repr)));
            Assert.False(GraphAlgorithms.HasCycle(MakePath(repr)));
        }
    }

    [Fact]
    public void ShortestPathChoosesTheCorrectStrategy()
    {
        foreach (var repr in Representations())
        {
            Assert.Equal(["A", "B", "C"], GraphAlgorithms.ShortestPath(MakePath(repr), "A", "C"));

            var weighted = new Graph<string>(repr);
            weighted.AddEdge("A", "B", 1);
            weighted.AddEdge("B", "D", 10);
            weighted.AddEdge("A", "C", 3);
            weighted.AddEdge("C", "D", 3);

            Assert.Equal(["A", "C", "D"], GraphAlgorithms.ShortestPath(weighted, "A", "D"));
            Assert.Equal(["London", "Amsterdam", "Berlin"], GraphAlgorithms.ShortestPath(MakeGraph(repr), "London", "Berlin"));

            var disconnected = new Graph<string>(repr);
            disconnected.AddNode("A");
            disconnected.AddNode("B");
            Assert.Empty(GraphAlgorithms.ShortestPath(disconnected, "A", "B"));
            Assert.Empty(GraphAlgorithms.ShortestPath(disconnected, "A", "missing"));
        }
    }

    [Fact]
    public void MinimumSpanningTreeReturnsTheCheapestConnectingEdges()
    {
        foreach (var repr in Representations())
        {
            var mst = GraphAlgorithms.MinimumSpanningTree(MakeGraph(repr));
            Assert.Equal(4, mst.Count);
            Assert.Equal(1655, mst.Sum(static edge => edge.Weight));

            var triangle = GraphAlgorithms.MinimumSpanningTree(MakeTriangle(repr));
            Assert.Equal(2, triangle.Count);
            Assert.Equal(2, triangle.Sum(static edge => edge.Weight));

            var disconnected = new Graph<string>(repr);
            disconnected.AddEdge("A", "B");
            disconnected.AddNode("C");
            Assert.Throws<InvalidOperationException>(() => GraphAlgorithms.MinimumSpanningTree(disconnected));
        }
    }

    [Fact]
    public void PropertyBagsTrackGraphNodeAndEdgeMetadata()
    {
        foreach (var repr in Representations())
        {
            var graph = new Graph<string>(repr);

            graph.SetGraphProperty("name", "city-map");
            graph.SetGraphProperty("version", 1);
            Assert.Equal("city-map", graph.GraphProperties()["name"]);
            Assert.Equal(1, graph.GraphProperties()["version"]);
            graph.RemoveGraphProperty("version");
            Assert.False(graph.GraphProperties().ContainsKey("version"));

            graph.AddNode("A", new Dictionary<string, object?> { ["kind"] = "input" });
            graph.AddNode("A", new Dictionary<string, object?> { ["trainable"] = false });
            graph.SetNodeProperty("A", "slot", 0);
            var nodeProperties = graph.NodeProperties("A");
            Assert.Equal("input", nodeProperties["kind"]);
            Assert.Equal(false, nodeProperties["trainable"]);
            Assert.Equal(0, nodeProperties["slot"]);
            graph.RemoveNodeProperty("A", "slot");
            Assert.False(graph.NodeProperties("A").ContainsKey("slot"));

            graph.AddEdge("A", "B", 2.5, new Dictionary<string, object?> { ["role"] = "distance" });
            Assert.Equal("distance", graph.EdgeProperties("B", "A")["role"]);
            Assert.Equal(2.5, graph.EdgeProperties("B", "A")["weight"]);
            graph.SetEdgeProperty("B", "A", "weight", 7.0);
            Assert.Equal(7.0, graph.EdgeWeight("A", "B"));
            graph.SetEdgeProperty("A", "B", "trainable", true);
            graph.RemoveEdgeProperty("A", "B", "role");
            Assert.Equal(true, graph.EdgeProperties("A", "B")["trainable"]);
            Assert.False(graph.EdgeProperties("A", "B").ContainsKey("role"));

            graph.RemoveEdge("A", "B");
            Assert.Throws<KeyNotFoundException>(() => graph.EdgeProperties("A", "B"));
        }
    }

    [Fact]
    public void NumericNodesAreSupported()
    {
        foreach (var repr in Representations())
        {
            var graph = new Graph<int>(repr);
            graph.AddEdge(1, 2);
            graph.AddEdge(2, 3);

            Assert.Equal([1, 2, 3], GraphAlgorithms.ShortestPath(graph, 1, 3));
            Assert.True(GraphAlgorithms.IsConnected(graph));
        }
    }

    [Fact]
    public void GraphStringRepresentationIncludesState()
    {
        var graph = MakeGraph(GraphRepr.AdjacencyList);
        var text = graph.ToString();
        Assert.Contains("Graph(nodes=5", text);
        Assert.Contains("edges=5", text);
        Assert.Contains("AdjacencyList", text);
    }
}
