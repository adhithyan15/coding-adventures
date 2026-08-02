export interface NeighborhoodRow {
  source: number;
  sourceFeature: number;
  sourceDegree: number;
  targetDegree: number;
  coefficient: number;
  contribution: number;
}
export interface AttentionRow {
  source: number;
  sourceFeature: number;
  score: number;
  shiftedScore: number;
  exponential: number;
  attentionWeight: number;
  contribution: number;
}
export interface GraphNeighborhoodTrace {
  features: number[];
  neighborhoods: number[][];
  degrees: number[];
  gcn: Array<{ target: number; rows: NeighborhoodRow[]; preactivation: number; output: number }>;
  gat: Array<{ target: number; rows: AttentionRow[]; maximumScore: number; denominator: number; preactivation: number; output: number }>;
  gcnOutputs: number[];
  gatOutputs: number[];
}

export const DEFAULT_GRAPH_NEIGHBORHOODS = [[0, 1], [0, 1, 2], [1, 2]];
export const DEFAULT_GRAPH_ATTENTION_FEATURES = [1, 2, -1];

export function traceGraphNeighborhoodComparison(
  features: readonly number[] = DEFAULT_GRAPH_ATTENTION_FEATURES,
  neighborhoods: readonly (readonly number[])[] = DEFAULT_GRAPH_NEIGHBORHOODS,
): GraphNeighborhoodTrace {
  if (features.length < 2 || !features.every(Number.isFinite) || neighborhoods.length !== features.length) {
    throw new Error("NN22 V1 needs finite features and one neighborhood per node.");
  }
  neighborhoods.forEach((sources, target) => {
    if (sources.length < 1 || new Set(sources).size !== sources.length || !sources.includes(target) || sources.some((source) => !Number.isInteger(source) || source < 0 || source >= features.length)) {
      throw new Error("NN22 V1 neighborhoods must be unique valid indices and include self-loops.");
    }
  });
  for (let target = 0; target < neighborhoods.length; target += 1) {
    for (const source of neighborhoods[target]!) {
      if (!neighborhoods[source]!.includes(target)) throw new Error("NN22 V1 neighborhoods must be symmetric.");
    }
  }
  const degrees = neighborhoods.map((sources) => sources.length);
  const gcn = neighborhoods.map((sources, target) => {
    const rows = sources.map((source) => {
      const coefficient = 1 / Math.sqrt(degrees[target]! * degrees[source]!);
      return { source, sourceFeature: features[source]!, sourceDegree: degrees[source]!, targetDegree: degrees[target]!, coefficient, contribution: coefficient * features[source]! };
    });
    const preactivation = rows.reduce((sum, row) => sum + row.contribution, 0);
    return { target, rows, preactivation, output: Math.max(0, preactivation) };
  });
  const gat = neighborhoods.map((sources, target) => {
    const scores = sources.map((source) => features[source]!);
    const maximumScore = Math.max(...scores);
    const exponentials = scores.map((score) => Math.exp(score - maximumScore));
    const denominator = exponentials.reduce((sum, value) => sum + value, 0);
    const rows = sources.map((source, index) => {
      const attentionWeight = exponentials[index]! / denominator;
      return { source, sourceFeature: features[source]!, score: scores[index]!, shiftedScore: scores[index]! - maximumScore, exponential: exponentials[index]!, attentionWeight, contribution: attentionWeight * features[source]! };
    });
    const preactivation = rows.reduce((sum, row) => sum + row.contribution, 0);
    return { target, rows, maximumScore, denominator, preactivation, output: Math.max(0, preactivation) };
  });
  return { features: [...features], neighborhoods: neighborhoods.map((row) => [...row]), degrees, gcn, gat, gcnOutputs: gcn.map((row) => row.output), gatOutputs: gat.map((row) => row.output) };
}
