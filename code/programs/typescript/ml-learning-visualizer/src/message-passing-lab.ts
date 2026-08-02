export interface GraphEdge { source: number; target: number }
export interface MessageParameters { messageWeight: number; selfWeight: number; bias: number }
export interface DirectedMessage {
  source: number;
  target: number;
  sourceFeature: number;
  messageWeight: number;
  message: number;
}
export interface NodeMessageUpdate {
  node: number;
  oldFeature: number;
  incoming: DirectedMessage[];
  aggregate: number;
  selfContribution: number;
  bias: number;
  preactivation: number;
  outputFeature: number;
}
export interface MessagePassingTrace {
  nodeFeatures: number[];
  edges: GraphEdge[];
  parameters: MessageParameters;
  directedMessages: DirectedMessage[];
  nodeUpdates: NodeMessageUpdate[];
  outputFeatures: number[];
}

export const DEFAULT_GRAPH_FEATURES = [1, 2, -1];
export const DEFAULT_GRAPH_EDGES: GraphEdge[] = [{ source: 0, target: 1 }, { source: 1, target: 2 }];
export const DEFAULT_MESSAGE_PARAMETERS: MessageParameters = { messageWeight: 0.5, selfWeight: 0.25, bias: -0.5 };

function normalizeZero(value: number): number {
  return Math.abs(value) < 1e-12 ? 0 : value;
}

export function traceTinyMessagePassing(
  nodeFeatures: readonly number[] = DEFAULT_GRAPH_FEATURES,
  edges: readonly GraphEdge[] = DEFAULT_GRAPH_EDGES,
  parameters: MessageParameters = DEFAULT_MESSAGE_PARAMETERS,
): MessagePassingTrace {
  const scalars = [...nodeFeatures, parameters.messageWeight, parameters.selfWeight, parameters.bias];
  if (
    nodeFeatures.length < 2
    || !scalars.every(Number.isFinite)
    || edges.length < 1
    || edges.some((edge) => (
      !Number.isInteger(edge.source)
      || !Number.isInteger(edge.target)
      || edge.source < 0
      || edge.target < 0
      || edge.source >= nodeFeatures.length
      || edge.target >= nodeFeatures.length
      || edge.source === edge.target
    ))
  ) {
    throw new Error("NN21 V1 needs finite node features and valid non-self undirected edges.");
  }
  const edgeKeys = edges.map((edge) => `${Math.min(edge.source, edge.target)}-${Math.max(edge.source, edge.target)}`);
  if (new Set(edgeKeys).size !== edgeKeys.length) {
    throw new Error("NN21 V1 needs unique undirected edges.");
  }

  const directedMessages = edges.flatMap((edge) => ([
    { source: edge.source, target: edge.target },
    { source: edge.target, target: edge.source },
  ])).map(({ source, target }) => {
    const sourceFeature = nodeFeatures[source]!;
    return {
      source,
      target,
      sourceFeature,
      messageWeight: parameters.messageWeight,
      message: normalizeZero(parameters.messageWeight * sourceFeature),
    };
  }).sort((left, right) => left.target - right.target || left.source - right.source);

  const nodeUpdates = nodeFeatures.map((oldFeature, node) => {
    const incoming = directedMessages.filter((message) => message.target === node);
    const aggregate = normalizeZero(incoming.reduce((sum, row) => sum + row.message, 0));
    const selfContribution = normalizeZero(parameters.selfWeight * oldFeature);
    const preactivation = normalizeZero(selfContribution + aggregate + parameters.bias);
    return {
      node,
      oldFeature,
      incoming,
      aggregate,
      selfContribution,
      bias: parameters.bias,
      preactivation,
      outputFeature: Math.max(0, preactivation),
    };
  });

  return {
    nodeFeatures: [...nodeFeatures],
    edges: edges.map((edge) => ({ ...edge })),
    parameters: { ...parameters },
    directedMessages,
    nodeUpdates,
    outputFeatures: nodeUpdates.map((row) => row.outputFeature),
  };
}
