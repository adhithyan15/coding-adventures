namespace CodingAdventures.NeuralNetwork;

public sealed record NeuralEdge(string Id, string From, string To, double Weight, IReadOnlyDictionary<string, object?> Properties);
public sealed record WeightedInput(string From, double Weight, string? EdgeId = null, IReadOnlyDictionary<string, object?>? Properties = null);

public sealed class NeuralGraph
{
    private readonly List<string> _nodes = [];
    private readonly Dictionary<string, Dictionary<string, object?>> _nodeProperties = [];
    private readonly List<NeuralEdge> _edges = [];
    private int _nextEdgeId;

    public NeuralGraph(string? name = null)
    {
        GraphProperties = new Dictionary<string, object?> { ["nn.version"] = "0" };
        if (!string.IsNullOrWhiteSpace(name)) GraphProperties["nn.name"] = name;
    }

    public Dictionary<string, object?> GraphProperties { get; }
    public IReadOnlyList<string> Nodes => _nodes.ToArray();
    public IReadOnlyList<NeuralEdge> Edges => _edges.ToArray();

    public void AddNode(string node, IReadOnlyDictionary<string, object?>? properties = null)
    {
        if (!_nodeProperties.ContainsKey(node))
        {
            _nodes.Add(node);
            _nodeProperties[node] = [];
        }
        if (properties is not null)
        {
            foreach (var (key, value) in properties) _nodeProperties[node][key] = value;
        }
    }

    public IReadOnlyDictionary<string, object?> NodeProperties(string node) =>
        _nodeProperties.TryGetValue(node, out var properties) ? new Dictionary<string, object?>(properties) : new Dictionary<string, object?>();

    public string AddEdge(string from, string to, double weight = 1.0, IReadOnlyDictionary<string, object?>? properties = null, string? edgeId = null)
    {
        AddNode(from);
        AddNode(to);
        var id = edgeId ?? $"e{_nextEdgeId++}";
        var merged = properties is null ? [] : new Dictionary<string, object?>(properties);
        merged["weight"] = weight;
        _edges.Add(new NeuralEdge(id, from, to, weight, merged));
        return id;
    }

    public IReadOnlyList<NeuralEdge> IncomingEdges(string node) => _edges.Where(edge => edge.To == node).ToArray();

    public IReadOnlyList<string> TopologicalSort()
    {
        var indegree = _nodes.ToDictionary(node => node, _ => 0);
        foreach (var edge in _edges)
        {
            indegree.TryAdd(edge.From, 0);
            indegree[edge.To] = indegree.GetValueOrDefault(edge.To) + 1;
        }
        var ready = new Queue<string>(indegree.Where(item => item.Value == 0).Select(item => item.Key).Order());
        var order = new List<string>();
        while (ready.Count > 0)
        {
            var node = ready.Dequeue();
            order.Add(node);
            foreach (var released in _edges.Where(edge => edge.From == node).Select(edge => edge.To).ToArray())
            {
                indegree[released]--;
                if (indegree[released] == 0) ready.Enqueue(released);
            }
        }
        if (order.Count != indegree.Count) throw new InvalidOperationException("neural graph contains a cycle");
        return order;
    }
}

public sealed class NeuralNetworkModel
{
    public NeuralNetworkModel(string? name = null) => Graph = NeuralNetworkPrimitives.CreateNeuralGraph(name);
    public NeuralGraph Graph { get; }
    public NeuralNetworkModel Input(string node) { NeuralNetworkPrimitives.AddInput(Graph, node); return this; }
    public NeuralNetworkModel Constant(string node, double value, IReadOnlyDictionary<string, object?>? properties = null) { NeuralNetworkPrimitives.AddConstant(Graph, node, value, properties); return this; }
    public NeuralNetworkModel WeightedSum(string node, IReadOnlyList<WeightedInput> inputs, IReadOnlyDictionary<string, object?>? properties = null) { NeuralNetworkPrimitives.AddWeightedSum(Graph, node, inputs, properties); return this; }
    public NeuralNetworkModel Activation(string node, string input, string activation, IReadOnlyDictionary<string, object?>? properties = null, string? edgeId = null) { NeuralNetworkPrimitives.AddActivation(Graph, node, input, activation, properties, edgeId); return this; }
    public NeuralNetworkModel Output(string node, string input, string outputName, IReadOnlyDictionary<string, object?>? properties = null, string? edgeId = null) { NeuralNetworkPrimitives.AddOutput(Graph, node, input, outputName, properties, edgeId); return this; }
}

public static class NeuralNetworkPrimitives
{
    public static NeuralGraph CreateNeuralGraph(string? name = null) => new(name);
    public static NeuralNetworkModel CreateNeuralNetwork(string? name = null) => new(name);
    public static WeightedInput Wi(string from, double weight, string edgeId) => new(from, weight, edgeId, new Dictionary<string, object?>());

    public static void AddInput(NeuralGraph graph, string node, string? inputName = null, IReadOnlyDictionary<string, object?>? properties = null) =>
        graph.AddNode(node, Merge(properties, new Dictionary<string, object?> { ["nn.op"] = "input", ["nn.input"] = inputName ?? node }));

    public static void AddConstant(NeuralGraph graph, string node, double value, IReadOnlyDictionary<string, object?>? properties = null)
    {
        if (!double.IsFinite(value)) throw new ArgumentOutOfRangeException(nameof(value), "Constant value must be finite.");
        graph.AddNode(node, Merge(properties, new Dictionary<string, object?> { ["nn.op"] = "constant", ["nn.value"] = value }));
    }

    public static void AddWeightedSum(NeuralGraph graph, string node, IReadOnlyList<WeightedInput> inputs, IReadOnlyDictionary<string, object?>? properties = null)
    {
        graph.AddNode(node, Merge(properties, new Dictionary<string, object?> { ["nn.op"] = "weighted_sum" }));
        foreach (var input in inputs) graph.AddEdge(input.From, node, input.Weight, input.Properties, input.EdgeId);
    }

    public static string AddActivation(NeuralGraph graph, string node, string input, string activation, IReadOnlyDictionary<string, object?>? properties = null, string? edgeId = null)
    {
        graph.AddNode(node, Merge(properties, new Dictionary<string, object?> { ["nn.op"] = "activation", ["nn.activation"] = activation }));
        return graph.AddEdge(input, node, 1.0, edgeId: edgeId);
    }

    public static string AddOutput(NeuralGraph graph, string node, string input, string? outputName = null, IReadOnlyDictionary<string, object?>? properties = null, string? edgeId = null)
    {
        graph.AddNode(node, Merge(properties, new Dictionary<string, object?> { ["nn.op"] = "output", ["nn.output"] = outputName ?? node }));
        return graph.AddEdge(input, node, 1.0, edgeId: edgeId);
    }

    public static NeuralNetworkModel CreateXorNetwork(string name = "xor") =>
        CreateNeuralNetwork(name)
            .Input("x0").Input("x1").Constant("bias", 1.0, new Dictionary<string, object?> { ["nn.role"] = "bias" })
            .WeightedSum("h_or_sum", [Wi("x0", 20, "x0_to_h_or"), Wi("x1", 20, "x1_to_h_or"), Wi("bias", -10, "bias_to_h_or")], new Dictionary<string, object?> { ["nn.layer"] = "hidden" })
            .Activation("h_or", "h_or_sum", "sigmoid", new Dictionary<string, object?> { ["nn.layer"] = "hidden" }, "h_or_sum_to_h_or")
            .WeightedSum("h_nand_sum", [Wi("x0", -20, "x0_to_h_nand"), Wi("x1", -20, "x1_to_h_nand"), Wi("bias", 30, "bias_to_h_nand")], new Dictionary<string, object?> { ["nn.layer"] = "hidden" })
            .Activation("h_nand", "h_nand_sum", "sigmoid", new Dictionary<string, object?> { ["nn.layer"] = "hidden" }, "h_nand_sum_to_h_nand")
            .WeightedSum("out_sum", [Wi("h_or", 20, "h_or_to_out"), Wi("h_nand", 20, "h_nand_to_out"), Wi("bias", -30, "bias_to_out")], new Dictionary<string, object?> { ["nn.layer"] = "output" })
            .Activation("out_activation", "out_sum", "sigmoid", new Dictionary<string, object?> { ["nn.layer"] = "output" }, "out_sum_to_activation")
            .Output("out", "out_activation", "prediction", new Dictionary<string, object?> { ["nn.layer"] = "output" }, "activation_to_out");

    private static Dictionary<string, object?> Merge(IReadOnlyDictionary<string, object?>? first, Dictionary<string, object?> second)
    {
        var result = first is null ? [] : new Dictionary<string, object?>(first);
        foreach (var (key, value) in second) result[key] = value;
        return result;
    }
}
