class DirectedGraphException implements Exception {
  const DirectedGraphException(this.message);

  final String message;

  @override
  String toString() => message;
}

class CycleError extends DirectedGraphException {
  CycleError(this.cycle)
      : super(
          'Graph contains a cycle: ${cycle.join(' -> ')}',
        );

  final List<String> cycle;
}

class NodeNotFoundError extends DirectedGraphException {
  NodeNotFoundError(this.node) : super('Node not found: "$node"');

  final String node;
}

class EdgeNotFoundError extends DirectedGraphException {
  EdgeNotFoundError(this.fromNode, this.toNode)
      : super('Edge not found: "$fromNode" -> "$toNode"');

  final String fromNode;
  final String toNode;
}
