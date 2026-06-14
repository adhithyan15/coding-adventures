import 'dart:math' as math;

import 'package:coding_adventures_neural_network/coding_adventures_neural_network.dart';

class NeuralBytecodeInstruction {
  NeuralBytecodeInstruction({required this.op, this.dst, this.inputName, this.outputName, this.edgeId, this.value, this.left, this.right, this.inputs = const [], this.input, this.activation, this.sourceNode, this.sourceEdge});
  final String op;
  final String? dst;
  final String? inputName;
  final String? outputName;
  final String? edgeId;
  final double? value;
  final String? left;
  final String? right;
  final List<String> inputs;
  final String? input;
  final String? activation;
  final String? sourceNode;
  final String? sourceEdge;
}

class NeuralBytecodeFunction {
  NeuralBytecodeFunction({required this.id, required this.kind, required this.instructions});
  final String id;
  final String kind;
  final List<NeuralBytecodeInstruction> instructions;
}

class NeuralBytecodeGraphEdge {
  NeuralBytecodeGraphEdge({required this.id, required this.from, required this.to, required this.weight});
  final String id;
  final String from;
  final String to;
  final double weight;
}

class NeuralBytecodeModule {
  NeuralBytecodeModule({required this.nodes, required this.edges, required this.functions});
  final String magic = 'CANN';
  final int version = 0;
  final List<String> nodes;
  final List<NeuralBytecodeGraphEdge> edges;
  final List<NeuralBytecodeFunction> functions;
}

NeuralBytecodeModule compileNeuralNetworkToBytecode(NeuralNetworkModel network) => compileNeuralGraphToBytecode(network.graph);

NeuralBytecodeModule compileNeuralGraphToBytecode(NeuralGraph graph) {
  final values = <String, String>{};
  var nextValueId = 0;
  String alloc() => 'v${nextValueId++}';
  final instructions = <NeuralBytecodeInstruction>[];

  for (final node in graph.topologicalSort()) {
    final props = graph.nodeProperties(node);
    switch ((props['nn.op'] as String?) ?? 'weighted_sum') {
      case 'input':
        final dst = alloc();
        values[node] = dst;
        instructions.add(NeuralBytecodeInstruction(op: 'LOAD_INPUT', dst: dst, inputName: (props['nn.input'] as String?) ?? node, sourceNode: node));
      case 'constant':
        final dst = alloc();
        values[node] = dst;
        instructions.add(NeuralBytecodeInstruction(op: 'LOAD_CONST', dst: dst, value: (props['nn.value'] as num).toDouble(), sourceNode: node));
      case 'weighted_sum':
        final incoming = graph.incomingEdges(node)..sort((a, b) => a.id.compareTo(b.id));
        final terms = <String>[];
        for (final edge in incoming) {
          final weightValue = alloc();
          final termValue = alloc();
          instructions.add(NeuralBytecodeInstruction(op: 'LOAD_EDGE_WEIGHT', dst: weightValue, edgeId: edge.id, sourceEdge: edge.id));
          instructions.add(NeuralBytecodeInstruction(op: 'MUL', dst: termValue, left: values[edge.from], right: weightValue, sourceEdge: edge.id));
          terms.add(termValue);
        }
        final dst = alloc();
        values[node] = dst;
        instructions.add(terms.isEmpty ? NeuralBytecodeInstruction(op: 'LOAD_CONST', dst: dst, value: 0.0, sourceNode: node) : NeuralBytecodeInstruction(op: 'ADD', dst: dst, inputs: terms, sourceNode: node));
      case 'activation':
        final dst = alloc();
        values[node] = dst;
        instructions.add(NeuralBytecodeInstruction(op: 'ACTIVATE', dst: dst, input: _singleInputValue(graph, values, node), activation: (props['nn.activation'] as String?) ?? 'relu', sourceNode: node));
      case 'output':
        final input = _singleInputValue(graph, values, node);
        values[node] = input;
        instructions.add(NeuralBytecodeInstruction(op: 'STORE_OUTPUT', outputName: (props['nn.output'] as String?) ?? node, input: input, sourceNode: node));
      default:
        throw StateError('unsupported neural graph op: ${props['nn.op']}');
    }
  }

  return NeuralBytecodeModule(
    nodes: graph.nodes,
    edges: graph.edges.map((edge) => NeuralBytecodeGraphEdge(id: edge.id, from: edge.from, to: edge.to, weight: edge.weight)).toList(),
    functions: [NeuralBytecodeFunction(id: 'forward', kind: 'forward', instructions: instructions)],
  );
}

Map<String, double> runNeuralBytecodeForward(NeuralBytecodeModule module, Map<String, double> inputs) {
  final values = <String, double>{};
  final edgeWeights = {for (final edge in module.edges) edge.id: edge.weight};
  final outputs = <String, double>{};
  final forward = module.functions.firstWhere((fn) => fn.kind == 'forward');
  for (final instruction in forward.instructions) {
    switch (instruction.op) {
      case 'LOAD_INPUT':
        values[instruction.dst!] = inputs[instruction.inputName!]!;
      case 'LOAD_CONST':
        values[instruction.dst!] = instruction.value ?? 0.0;
      case 'LOAD_EDGE_WEIGHT':
        values[instruction.dst!] = edgeWeights[instruction.edgeId!] ?? 1.0;
      case 'MUL':
        values[instruction.dst!] = values[instruction.left!]! * values[instruction.right!]!;
      case 'ADD':
        values[instruction.dst!] = instruction.inputs.fold(0.0, (sum, id) => sum + values[id]!);
      case 'ACTIVATE':
        values[instruction.dst!] = applyNeuralActivation(values[instruction.input!]!, instruction.activation ?? 'relu');
      case 'STORE_OUTPUT':
        outputs[instruction.outputName ?? 'output'] = values[instruction.input!]!;
      default:
        throw StateError('unsupported opcode: ${instruction.op}');
    }
  }
  return outputs;
}

double applyNeuralActivation(double value, String activation) {
  switch (activation) {
    case 'relu':
      return value > 0 ? value : 0.0;
    case 'sigmoid':
      return 1.0 / (1.0 + math.exp(-value));
    case 'tanh':
      final raised = math.exp(2.0 * value);
      return (raised - 1.0) / (raised + 1.0);
    default:
      return value;
  }
}

String _singleInputValue(NeuralGraph graph, Map<String, String> values, String node) {
  final incoming = graph.incomingEdges(node);
  if (incoming.length != 1) throw StateError('node $node expects exactly one input');
  return values[incoming.first.from]!;
}
