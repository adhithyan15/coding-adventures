class GraphException implements Exception {
  const GraphException(this.message);

  final String message;

  @override
  String toString() => message;
}

class NodeNotFoundError<T> extends GraphException {
  NodeNotFoundError(this.node) : super('Node not found: ${_describeNode(node)}');

  final T node;
}

class EdgeNotFoundError<T> extends GraphException {
  EdgeNotFoundError(this.left, this.right)
      : super(
          'Edge not found: ${_describeNode(left)} -- ${_describeNode(right)}',
        );

  final T left;
  final T right;
}

class GraphNotConnectedError extends GraphException {
  const GraphNotConnectedError()
      : super(
          'minimumSpanningTree: graph is not connected and has no spanning tree',
        );
}

String _describeNode(Object? node) => '$node';
