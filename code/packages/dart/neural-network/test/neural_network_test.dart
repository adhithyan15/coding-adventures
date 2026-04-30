import 'package:coding_adventures_neural_network/coding_adventures_neural_network.dart';
import 'package:test/test.dart';

void main() {
  test('builds a tiny weighted graph', () {
    final graph = createNeuralGraph('tiny');
    addInput(graph, 'x0');
    addInput(graph, 'x1');
    addConstant(graph, 'bias', 1.0);
    addWeightedSum(graph, 'sum', const [WeightedInput('x0', 0.25, edgeId: 'x0_to_sum'), WeightedInput('x1', 0.75, edgeId: 'x1_to_sum'), WeightedInput('bias', -1.0, edgeId: 'bias_to_sum')]);
    addActivation(graph, 'relu', 'sum', 'relu', edgeId: 'sum_to_relu');
    addOutput(graph, 'out', 'relu', outputName: 'prediction', edgeId: 'relu_to_out');
    expect(graph.incomingEdges('sum'), hasLength(3));
    expect(graph.topologicalSort().last, equals('out'));
  });

  test('xor network has hidden output edge', () {
    final network = createXorNetwork();
    expect(network.graph.edges.any((edge) => edge.id == 'h_or_to_out'), isTrue);
  });
}
