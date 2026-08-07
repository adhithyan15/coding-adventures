import type { AttentionTokenId } from "./attention-qkv-lab.js";

export type AttentionHeadId = "horizontal" | "vertical";

export interface AttentionHeadParameters {
  id: AttentionHeadId;
  queryProjection: number[];
  keyProjection: number[];
  valueProjection: number[];
}

export interface MultiHeadTrace {
  id: AttentionHeadId;
  queryProducts: number[];
  query: number;
  keyProducts: number[][];
  keys: number[];
  valueProducts: number[][];
  values: number[];
  scaleDivisor: number;
  scaledScores: number[];
  allowed: boolean[];
  maskedScores: Array<number | null>;
  rowMax: number;
  shiftedScores: Array<number | null>;
  exponentials: number[];
  denominator: number;
  weights: number[];
  valueContributions: number[];
  context: number;
}

export interface LayerNormTrace {
  mean: number;
  centered: number[];
  squaredDeviations: number[];
  variance: number;
  denominator: number;
  normalized: number[];
  affineProducts: number[];
  output: number[];
}

export interface MultiHeadAttentionRow {
  tokenId: AttentionTokenId;
  input: number[];
  heads: MultiHeadTrace[];
  concatenated: number[];
  outputProjectionProducts: number[][];
  projectedAttention: number[];
  residualSum: number[];
  layerNorm: LayerNormTrace;
  output: number[];
}

export interface MultiHeadAttentionTrace {
  includeResidual: boolean;
  applyLayerNorm: boolean;
  tokenIds: AttentionTokenId[];
  rows: MultiHeadAttentionRow[];
}

export const DEFAULT_MULTI_HEAD_TOKEN_IDS: AttentionTokenId[] = [
  "red",
  "blue",
  "purple",
];

export const DEFAULT_MULTI_HEAD_EMBEDDINGS = [
  [2, 0],
  [0, 1],
  [2, 1],
];

export const DEFAULT_ATTENTION_HEADS: AttentionHeadParameters[] = [
  {
    id: "horizontal",
    queryProjection: [0.5, 0],
    keyProjection: [0.5, 0],
    valueProjection: [1, 0],
  },
  {
    id: "vertical",
    queryProjection: [0, 1],
    keyProjection: [0, 1],
    valueProjection: [0, 1],
  },
];

export const DEFAULT_OUTPUT_PROJECTION = [
  [1, 0],
  [0, 1],
];

export const DEFAULT_LAYER_NORM = {
  epsilon: 0.00001,
  gamma: [1, 1],
  beta: [0, 0],
};

function cleanZero(value: number): number {
  return Math.abs(value) < 1e-12 ? 0 : value;
}

function validMatrix(
  matrix: readonly (readonly number[])[],
  rows: number,
  columns: number,
): boolean {
  return matrix.length === rows
    && matrix.every(
      (row) => row.length === columns && row.every(Number.isFinite),
    );
}

function products(
  vector: readonly number[],
  projection: readonly number[],
): number[] {
  return vector.map((value, index) => cleanZero(value * projection[index]!));
}

function executeHead(
  embeddings: readonly (readonly number[])[],
  queryIndex: number,
  parameters: AttentionHeadParameters,
): MultiHeadTrace {
  const queryProducts = products(
    embeddings[queryIndex]!,
    parameters.queryProjection,
  );
  const query = cleanZero(queryProducts.reduce((sum, value) => sum + value, 0));
  const keyProducts = embeddings.map((embedding) => (
    products(embedding, parameters.keyProjection)
  ));
  const keys = keyProducts.map((items) => cleanZero(
    items.reduce((sum, value) => sum + value, 0),
  ));
  const valueProducts = embeddings.map((embedding) => (
    products(embedding, parameters.valueProjection)
  ));
  const values = valueProducts.map((items) => cleanZero(
    items.reduce((sum, value) => sum + value, 0),
  ));
  const scaleDivisor = 1;
  const scaledScores = keys.map((key) => cleanZero(query * key / scaleDivisor));
  const allowed = scaledScores.map((_, keyIndex) => keyIndex <= queryIndex);
  const maskedScores = scaledScores.map((score, keyIndex) => (
    allowed[keyIndex] ? score : null
  ));
  const rowMax = Math.max(
    ...maskedScores.filter((score): score is number => score !== null),
  );
  const shiftedScores = maskedScores.map((score) => (
    score === null ? null : cleanZero(score - rowMax)
  ));
  const exponentials = shiftedScores.map((score) => (
    score === null ? 0 : Math.exp(score)
  ));
  const denominator = exponentials.reduce((sum, value) => sum + value, 0);
  const weights = exponentials.map((value) => cleanZero(value / denominator));
  const valueContributions = weights.map((weight, index) => (
    cleanZero(weight * values[index]!)
  ));

  return {
    id: parameters.id,
    queryProducts,
    query,
    keyProducts,
    keys,
    valueProducts,
    values,
    scaleDivisor,
    scaledScores,
    allowed,
    maskedScores,
    rowMax,
    shiftedScores,
    exponentials,
    denominator,
    weights,
    valueContributions,
    context: cleanZero(valueContributions.reduce((sum, value) => sum + value, 0)),
  };
}

export function traceMultiHeadAttention(
  includeResidual = true,
  applyLayerNorm = true,
  embeddings: readonly (readonly number[])[] = DEFAULT_MULTI_HEAD_EMBEDDINGS,
  tokenIds: readonly AttentionTokenId[] = DEFAULT_MULTI_HEAD_TOKEN_IDS,
  heads: readonly AttentionHeadParameters[] = DEFAULT_ATTENTION_HEADS,
  outputProjection: readonly (readonly number[])[] = DEFAULT_OUTPUT_PROJECTION,
  epsilon = DEFAULT_LAYER_NORM.epsilon,
  gamma: readonly number[] = DEFAULT_LAYER_NORM.gamma,
  beta: readonly number[] = DEFAULT_LAYER_NORM.beta,
): MultiHeadAttentionTrace {
  if (
    tokenIds.length !== 3
    || new Set(tokenIds).size !== 3
    || !validMatrix(embeddings, 3, 2)
    || heads.length !== 2
    || new Set(heads.map((head) => head.id)).size !== 2
    || heads.some((head) => (
      !validMatrix([
        head.queryProjection,
        head.keyProjection,
        head.valueProjection,
      ], 3, 2)
    ))
    || !validMatrix(outputProjection, 2, 2)
    || !Number.isFinite(epsilon)
    || epsilon <= 0
    || !validMatrix([gamma, beta], 2, 2)
  ) {
    throw new Error(
      "NN14 V1 needs three 2D tokens, two scalar heads, a 2 x 2 output projection, and finite layer-norm parameters.",
    );
  }

  const rows = embeddings.map((embedding, queryIndex): MultiHeadAttentionRow => {
    const headTraces = heads.map((head) => executeHead(
      embeddings,
      queryIndex,
      head,
    ));
    const concatenated = headTraces.map((head) => head.context);
    const outputProjectionProducts = outputProjection[0]!.map(
      (_, outputIndex) => concatenated.map((context, headIndex) => (
        cleanZero(context * outputProjection[headIndex]![outputIndex]!)
      )),
    );
    const projectedAttention = outputProjectionProducts.map((items) => (
      cleanZero(items.reduce((sum, value) => sum + value, 0))
    ));
    const residualSum = projectedAttention.map((value, index) => cleanZero(
      value + (includeResidual ? embedding[index]! : 0),
    ));
    const mean = residualSum.reduce((sum, value) => sum + value, 0) / 2;
    const centered = residualSum.map((value) => cleanZero(value - mean));
    const squaredDeviations = centered.map((value) => value * value);
    const variance = squaredDeviations.reduce((sum, value) => sum + value, 0) / 2;
    const denominator = Math.sqrt(variance + epsilon);
    const normalized = centered.map((value) => cleanZero(value / denominator));
    const affineProducts = normalized.map((value, index) => (
      cleanZero(value * gamma[index]!)
    ));
    const normalizedOutput = affineProducts.map((value, index) => (
      cleanZero(value + beta[index]!)
    ));

    return {
      tokenId: tokenIds[queryIndex]!,
      input: [...embedding],
      heads: headTraces,
      concatenated,
      outputProjectionProducts,
      projectedAttention,
      residualSum,
      layerNorm: {
        mean,
        centered,
        squaredDeviations,
        variance,
        denominator,
        normalized,
        affineProducts,
        output: normalizedOutput,
      },
      output: applyLayerNorm ? normalizedOutput : residualSum,
    };
  });

  return {
    includeResidual,
    applyLayerNorm,
    tokenIds: [...tokenIds],
    rows,
  };
}

export function multiHeadAttentionRow(
  trace: MultiHeadAttentionTrace,
  tokenId: AttentionTokenId,
): MultiHeadAttentionRow {
  const row = trace.rows.find((item) => item.tokenId === tokenId);
  if (row === undefined) {
    throw new Error(`Unknown multi-head attention token ${tokenId}.`);
  }
  return row;
}
