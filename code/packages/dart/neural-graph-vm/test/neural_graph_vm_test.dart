import 'package:coding_adventures_neural_graph_vm/coding_adventures_neural_graph_vm.dart';
import 'package:coding_adventures_neural_network/coding_adventures_neural_network.dart';
import 'package:test/test.dart';

NeuralGraph tinyGraph() {
  final graph = createNeuralGraph('tiny');
  addInput(graph, 'x0');
  addInput(graph, 'x1');
  addConstant(graph, 'bias', 1.0);
  addWeightedSum(graph, 'sum', const [WeightedInput('x0', 0.25, edgeId: 'x0_to_sum'), WeightedInput('x1', 0.75, edgeId: 'x1_to_sum'), WeightedInput('bias', -1.0, edgeId: 'bias_to_sum')]);
  addActivation(graph, 'relu', 'sum', 'relu', edgeId: 'sum_to_relu');
  addOutput(graph, 'out', 'relu', outputName: 'prediction', edgeId: 'relu_to_out');
  return graph;
}

void main() {
  test('runs the tiny weighted sum', () {
    final bytecode = compileNeuralGraphToBytecode(tinyGraph());
    final outputs = runNeuralBytecodeForward(bytecode, {'x0': 4.0, 'x1': 8.0});
    expect(outputs['prediction'], closeTo(6.0, 1e-9));
  });

  test('runs xor', () {
    final bytecode = compileNeuralNetworkToBytecode(createXorNetwork());
    for (final c in const [[0.0, 0.0, 0.0], [0.0, 1.0, 1.0], [1.0, 0.0, 1.0], [1.0, 1.0, 0.0]]) {
      final prediction = runNeuralBytecodeForward(bytecode, {'x0': c[0], 'x1': c[1]})['prediction']!;
      expect(c[2] == 1.0 ? prediction > 0.99 : prediction < 0.01, isTrue);
    }
  });
}
