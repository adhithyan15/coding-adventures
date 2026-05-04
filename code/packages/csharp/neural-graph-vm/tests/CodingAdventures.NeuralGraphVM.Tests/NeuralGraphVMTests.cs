using CodingAdventures.NeuralNetwork;

namespace CodingAdventures.NeuralGraphVM.Tests;

public sealed class NeuralGraphVMTests
{
    private static NeuralGraph TinyGraph()
    {
        var graph = NeuralNetworkPrimitives.CreateNeuralGraph("tiny");
        NeuralNetworkPrimitives.AddInput(graph, "x0");
        NeuralNetworkPrimitives.AddInput(graph, "x1");
        NeuralNetworkPrimitives.AddConstant(graph, "bias", 1.0);
        NeuralNetworkPrimitives.AddWeightedSum(graph, "sum", [NeuralNetworkPrimitives.Wi("x0", 0.25, "x0_to_sum"), NeuralNetworkPrimitives.Wi("x1", 0.75, "x1_to_sum"), NeuralNetworkPrimitives.Wi("bias", -1.0, "bias_to_sum")]);
        NeuralNetworkPrimitives.AddActivation(graph, "relu", "sum", "relu", edgeId: "sum_to_relu");
        NeuralNetworkPrimitives.AddOutput(graph, "out", "relu", "prediction", edgeId: "relu_to_out");
        return graph;
    }

    [Fact]
    public void RunsTinyWeightedSum()
    {
        var outputs = NeuralGraphVM.RunNeuralBytecodeForward(NeuralGraphVM.CompileNeuralGraphToBytecode(TinyGraph()), new Dictionary<string, double> { ["x0"] = 4.0, ["x1"] = 8.0 });
        Assert.Equal(6.0, outputs["prediction"], 9);
    }

    [Fact]
    public void RunsXor()
    {
        var bytecode = NeuralGraphVM.CompileNeuralNetworkToBytecode(NeuralNetworkPrimitives.CreateXorNetwork());
        foreach (var (x0, x1, expected) in new[] { (0.0, 0.0, 0.0), (0.0, 1.0, 1.0), (1.0, 0.0, 1.0), (1.0, 1.0, 0.0) })
        {
            var prediction = NeuralGraphVM.RunNeuralBytecodeForward(bytecode, new Dictionary<string, double> { ["x0"] = x0, ["x1"] = x1 })["prediction"];
            Assert.True(expected == 1.0 ? prediction > 0.99 : prediction < 0.01);
        }
    }
}
