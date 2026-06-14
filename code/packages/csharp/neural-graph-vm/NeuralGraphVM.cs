using CodingAdventures.NeuralNetwork;

namespace CodingAdventures.NeuralGraphVM;

public sealed record NeuralBytecodeInstruction(string Op, string? Dst = null, string? InputName = null, string? OutputName = null, string? EdgeId = null, double? Value = null, string? Left = null, string? Right = null, IReadOnlyList<string>? Inputs = null, string? Input = null, string? Activation = null, string? SourceNode = null, string? SourceEdge = null);
public sealed record NeuralBytecodeFunction(string Id, string Kind, IReadOnlyList<NeuralBytecodeInstruction> Instructions);
public sealed record NeuralBytecodeGraphEdge(string Id, string From, string To, double Weight);
public sealed record NeuralBytecodeModule(string Magic, int Version, IReadOnlyList<string> Nodes, IReadOnlyList<NeuralBytecodeGraphEdge> Edges, IReadOnlyList<NeuralBytecodeFunction> Functions);

public static class NeuralGraphVM
{
    public static NeuralBytecodeModule CompileNeuralNetworkToBytecode(NeuralNetworkModel network) => CompileNeuralGraphToBytecode(network.Graph);

    public static NeuralBytecodeModule CompileNeuralGraphToBytecode(NeuralGraph graph)
    {
        var values = new Dictionary<string, string>();
        var instructions = new List<NeuralBytecodeInstruction>();
        var nextValueId = 0;
        string Alloc() => $"v{nextValueId++}";

        foreach (var node in graph.TopologicalSort())
        {
            var props = graph.NodeProperties(node);
            var op = props.TryGetValue("nn.op", out var opValue) ? (string)opValue! : "weighted_sum";
            switch (op)
            {
                case "input":
                    {
                        var dst = Alloc();
                        values[node] = dst;
                        instructions.Add(new NeuralBytecodeInstruction("LOAD_INPUT", Dst: dst, InputName: props.GetValueOrDefault("nn.input") as string ?? node, SourceNode: node));
                        break;
                    }
                case "constant":
                    {
                        var dst = Alloc();
                        values[node] = dst;
                        instructions.Add(new NeuralBytecodeInstruction("LOAD_CONST", Dst: dst, Value: Convert.ToDouble(props["nn.value"]), SourceNode: node));
                        break;
                    }
                case "weighted_sum":
                    {
                        var terms = new List<string>();
                        foreach (var edge in graph.IncomingEdges(node).OrderBy(edge => edge.Id))
                        {
                            var weightValue = Alloc();
                            var termValue = Alloc();
                            instructions.Add(new NeuralBytecodeInstruction("LOAD_EDGE_WEIGHT", Dst: weightValue, EdgeId: edge.Id, SourceEdge: edge.Id));
                            instructions.Add(new NeuralBytecodeInstruction("MUL", Dst: termValue, Left: values[edge.From], Right: weightValue, SourceEdge: edge.Id));
                            terms.Add(termValue);
                        }
                        var dst = Alloc();
                        values[node] = dst;
                        instructions.Add(terms.Count == 0 ? new NeuralBytecodeInstruction("LOAD_CONST", Dst: dst, Value: 0.0, SourceNode: node) : new NeuralBytecodeInstruction("ADD", Dst: dst, Inputs: terms, SourceNode: node));
                        break;
                    }
                case "activation":
                    {
                        var dst = Alloc();
                        values[node] = dst;
                        instructions.Add(new NeuralBytecodeInstruction("ACTIVATE", Dst: dst, Input: SingleInputValue(graph, values, node), Activation: props.GetValueOrDefault("nn.activation") as string ?? "relu", SourceNode: node));
                        break;
                    }
                case "output":
                    {
                        var input = SingleInputValue(graph, values, node);
                        values[node] = input;
                        instructions.Add(new NeuralBytecodeInstruction("STORE_OUTPUT", OutputName: props.GetValueOrDefault("nn.output") as string ?? node, Input: input, SourceNode: node));
                        break;
                    }
                default:
                    throw new InvalidOperationException($"Unsupported neural graph op: {op}");
            }
        }

        return new NeuralBytecodeModule("CANN", 0, graph.Nodes, graph.Edges.Select(edge => new NeuralBytecodeGraphEdge(edge.Id, edge.From, edge.To, edge.Weight)).ToArray(), [new NeuralBytecodeFunction("forward", "forward", instructions)]);
    }

    public static Dictionary<string, double> RunNeuralBytecodeForward(NeuralBytecodeModule module, IReadOnlyDictionary<string, double> inputs)
    {
        var values = new Dictionary<string, double>();
        var edgeWeights = module.Edges.ToDictionary(edge => edge.Id, edge => edge.Weight);
        var outputs = new Dictionary<string, double>();
        var forward = module.Functions.Single(fn => fn.Kind == "forward");
        foreach (var instruction in forward.Instructions)
        {
            switch (instruction.Op)
            {
                case "LOAD_INPUT": values[instruction.Dst!] = inputs[instruction.InputName!]; break;
                case "LOAD_CONST": values[instruction.Dst!] = instruction.Value ?? 0.0; break;
                case "LOAD_EDGE_WEIGHT": values[instruction.Dst!] = edgeWeights.GetValueOrDefault(instruction.EdgeId!, 1.0); break;
                case "MUL": values[instruction.Dst!] = values[instruction.Left!] * values[instruction.Right!]; break;
                case "ADD": values[instruction.Dst!] = instruction.Inputs!.Sum(id => values[id]); break;
                case "ACTIVATE": values[instruction.Dst!] = ApplyNeuralActivation(values[instruction.Input!], instruction.Activation ?? "relu"); break;
                case "STORE_OUTPUT": outputs[instruction.OutputName ?? "output"] = values[instruction.Input!]; break;
                default: throw new InvalidOperationException($"Unsupported opcode: {instruction.Op}");
            }
        }
        return outputs;
    }

    public static double ApplyNeuralActivation(double value, string activation) => activation switch
    {
        "relu" => value > 0 ? value : 0.0,
        "sigmoid" => 1.0 / (1.0 + Math.Exp(-value)),
        "tanh" => Math.Tanh(value),
        _ => value,
    };

    private static string SingleInputValue(NeuralGraph graph, Dictionary<string, string> values, string node)
    {
        var incoming = graph.IncomingEdges(node);
        if (incoming.Count != 1) throw new InvalidOperationException($"Node {node} expects exactly one input.");
        return values[incoming[0].From];
    }
}
