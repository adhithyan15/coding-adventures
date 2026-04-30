using CodingAdventures.NeuralNetwork;

namespace CodingAdventures.NeuralNetwork.Tests;

public sealed class NeuralNetworkTests
{
    [Fact]
    public void BuildsTinyWeightedGraph()
    {
        var graph = NeuralNetworkPrimitives.CreateNeuralGraph("tiny");
        NeuralNetworkPrimitives.AddInput(graph, "x0");
        NeuralNetworkPrimitives.AddInput(graph, "x1");
        NeuralNetworkPrimitives.AddConstant(graph, "bias", 1.0);
        NeuralNetworkPrimitives.AddWeightedSum(graph, "sum", [NeuralNetworkPrimitives.Wi("x0", 0.25, "x0_to_sum"), NeuralNetworkPrimitives.Wi("x1", 0.75, "x1_to_sum"), NeuralNetworkPrimitives.Wi("bias", -1.0, "bias_to_sum")]);
        NeuralNetworkPrimitives.AddActivation(graph, "relu", "sum", "relu", edgeId: "sum_to_relu");
        NeuralNetworkPrimitives.AddOutput(graph, "out", "relu", "prediction", edgeId: "relu_to_out");
        Assert.Equal(3, graph.IncomingEdges("sum").Count);
        Assert.Equal("out", graph.TopologicalSort().Last());
    }

    [Fact]
    public void XorNetworkHasHiddenOutputEdge() => Assert.Contains(NeuralNetworkPrimitives.CreateXorNetwork().Graph.Edges, edge => edge.Id == "h_or_to_out");
}
