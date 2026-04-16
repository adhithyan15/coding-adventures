using CodingAdventures.DirectedGraph;

namespace CodingAdventures.DirectedGraph.Tests;

public class DirectedGraphTests
{
    [Fact]
    public void TopologicalSort_OrdersDependencies()
    {
        var graph = new Graph();
        graph.AddEdge("A", "B");
        graph.AddEdge("B", "C");

        Assert.Equal(["A", "B", "C"], graph.TopologicalSort());
    }

    [Fact]
    public void IndependentGroups_LayersDiamond()
    {
        var graph = new Graph();
        graph.AddEdge("A", "B");
        graph.AddEdge("A", "C");
        graph.AddEdge("B", "D");
        graph.AddEdge("C", "D");

        var groups = graph.IndependentGroups();
        Assert.Equal(["A"], groups[0]);
        Assert.Equal(["B", "C"], groups[1]);
        Assert.Equal(["D"], groups[2]);
    }
}
