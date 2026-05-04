const neuralNetworkVersion = '0.1.0';

typedef PropertyBag = Map<String, Object?>;

class NeuralEdge {
  NeuralEdge({required this.id, required this.from, required this.to, required this.weight, Map<String, Object?>? properties})
      : properties = Map.unmodifiable({...?properties, 'weight': weight});

  final String id;
  final String from;
  final String to;
  final double weight;
  final Map<String, Object?> properties;
}

class WeightedInput {
  const WeightedInput(this.from, this.weight, {this.edgeId, this.properties = const {}});
  final String from;
  final double weight;
  final String? edgeId;
  final Map<String, Object?> properties;
}

class NeuralGraph {
  NeuralGraph([String? name]) {
    graphProperties['nn.version'] = '0';
    if (name != null) graphProperties['nn.name'] = name;
  }

  final Map<String, Object?> graphProperties = {};
  final List<String> _nodes = [];
  final Map<String, Map<String, Object?>> _nodeProperties = {};
  final List<NeuralEdge> _edges = [];
  int _nextEdgeId = 0;

  List<String> get nodes => List.unmodifiable(_nodes);
  List<NeuralEdge> get edges => List.unmodifiable(_edges);

  void addNode(String node, [Map<String, Object?> properties = const {}]) {
    _nodeProperties.putIfAbsent(node, () {
      _nodes.add(node);
      return <String, Object?>{};
    }).addAll(properties);
  }

  Map<String, Object?> nodeProperties(String node) => Map.unmodifiable(_nodeProperties[node] ?? const {});

  String addEdge(String from, String to, double weight, {Map<String, Object?> properties = const {}, String? edgeId}) {
    addNode(from);
    addNode(to);
    final id = edgeId ?? 'e${_nextEdgeId++}';
    _edges.add(NeuralEdge(id: id, from: from, to: to, weight: weight, properties: properties));
    return id;
  }

  List<NeuralEdge> incomingEdges(String node) => _edges.where((edge) => edge.to == node).toList(growable: false);

  List<String> topologicalSort() {
    final indegree = <String, int>{for (final node in _nodes) node: 0};
    for (final edge in _edges) {
      indegree.putIfAbsent(edge.from, () => 0);
      indegree[edge.to] = (indegree[edge.to] ?? 0) + 1;
    }
    final ready = indegree.entries.where((entry) => entry.value == 0).map((entry) => entry.key).toList()..sort();
    final order = <String>[];
    while (ready.isNotEmpty) {
      final node = ready.removeAt(0);
      order.add(node);
      final released = <String>[];
      for (final edge in _edges.where((edge) => edge.from == node)) {
        indegree[edge.to] = indegree[edge.to]! - 1;
        if (indegree[edge.to] == 0) released.add(edge.to);
      }
      ready.addAll(released..sort());
    }
    if (order.length != indegree.length) throw StateError('neural graph contains a cycle');
    return order;
  }
}

class NeuralNetworkModel {
  NeuralNetworkModel([String? name]) : graph = createNeuralGraph(name);
  final NeuralGraph graph;

  NeuralNetworkModel input(String node, {String? inputName, Map<String, Object?> properties = const {}}) {
    addInput(graph, node, inputName: inputName, properties: properties);
    return this;
  }

  NeuralNetworkModel constant(String node, double value, {Map<String, Object?> properties = const {}}) {
    addConstant(graph, node, value, properties: properties);
    return this;
  }

  NeuralNetworkModel weightedSum(String node, List<WeightedInput> inputs, {Map<String, Object?> properties = const {}}) {
    addWeightedSum(graph, node, inputs, properties: properties);
    return this;
  }

  NeuralNetworkModel activation(String node, String input, String activation, {Map<String, Object?> properties = const {}, String? edgeId}) {
    addActivation(graph, node, input, activation, properties: properties, edgeId: edgeId);
    return this;
  }

  NeuralNetworkModel output(String node, String input, {String? outputName, Map<String, Object?> properties = const {}, String? edgeId}) {
    addOutput(graph, node, input, outputName: outputName, properties: properties, edgeId: edgeId);
    return this;
  }
}

NeuralGraph createNeuralGraph([String? name]) => NeuralGraph(name);
NeuralNetworkModel createNeuralNetwork([String? name]) => NeuralNetworkModel(name);

void addInput(NeuralGraph graph, String node, {String? inputName, Map<String, Object?> properties = const {}}) {
  graph.addNode(node, {...properties, 'nn.op': 'input', 'nn.input': inputName ?? node});
}

void addConstant(NeuralGraph graph, String node, double value, {Map<String, Object?> properties = const {}}) {
  if (!value.isFinite) throw ArgumentError('constant value must be finite');
  graph.addNode(node, {...properties, 'nn.op': 'constant', 'nn.value': value});
}

void addWeightedSum(NeuralGraph graph, String node, List<WeightedInput> inputs, {Map<String, Object?> properties = const {}}) {
  graph.addNode(node, {...properties, 'nn.op': 'weighted_sum'});
  for (final input in inputs) {
    graph.addEdge(input.from, node, input.weight, properties: input.properties, edgeId: input.edgeId);
  }
}

String addActivation(NeuralGraph graph, String node, String input, String activation, {Map<String, Object?> properties = const {}, String? edgeId}) {
  graph.addNode(node, {...properties, 'nn.op': 'activation', 'nn.activation': activation});
  return graph.addEdge(input, node, 1.0, edgeId: edgeId);
}

String addOutput(NeuralGraph graph, String node, String input, {String? outputName, Map<String, Object?> properties = const {}, String? edgeId}) {
  graph.addNode(node, {...properties, 'nn.op': 'output', 'nn.output': outputName ?? node});
  return graph.addEdge(input, node, 1.0, edgeId: edgeId);
}

NeuralNetworkModel createXorNetwork([String name = 'xor']) {
  return createNeuralNetwork(name)
    ..input('x0')
    ..input('x1')
    ..constant('bias', 1.0, properties: {'nn.role': 'bias'})
    ..weightedSum('h_or_sum', [const WeightedInput('x0', 20, edgeId: 'x0_to_h_or'), const WeightedInput('x1', 20, edgeId: 'x1_to_h_or'), const WeightedInput('bias', -10, edgeId: 'bias_to_h_or')], properties: {'nn.layer': 'hidden'})
    ..activation('h_or', 'h_or_sum', 'sigmoid', properties: {'nn.layer': 'hidden'}, edgeId: 'h_or_sum_to_h_or')
    ..weightedSum('h_nand_sum', [const WeightedInput('x0', -20, edgeId: 'x0_to_h_nand'), const WeightedInput('x1', -20, edgeId: 'x1_to_h_nand'), const WeightedInput('bias', 30, edgeId: 'bias_to_h_nand')], properties: {'nn.layer': 'hidden'})
    ..activation('h_nand', 'h_nand_sum', 'sigmoid', properties: {'nn.layer': 'hidden'}, edgeId: 'h_nand_sum_to_h_nand')
    ..weightedSum('out_sum', [const WeightedInput('h_or', 20, edgeId: 'h_or_to_out'), const WeightedInput('h_nand', 20, edgeId: 'h_nand_to_out'), const WeightedInput('bias', -30, edgeId: 'bias_to_out')], properties: {'nn.layer': 'output'})
    ..activation('out_activation', 'out_sum', 'sigmoid', properties: {'nn.layer': 'output'}, edgeId: 'out_sum_to_activation')
    ..output('out', 'out_activation', outputName: 'prediction', properties: {'nn.layer': 'output'}, edgeId: 'activation_to_out');
}
