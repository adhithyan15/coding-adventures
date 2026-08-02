export type AttentionTokenId = "red" | "blue" | "purple";

export interface AttentionToken {
  id: AttentionTokenId;
  label: string;
  embedding: readonly number[];
}

export interface AttentionProjection {
  id: AttentionTokenId;
  label: string;
  embedding: number[];
  query: number[];
  key: number[];
  value: number[];
}

export interface AttentionDotProduct {
  queryId: AttentionTokenId;
  keyId: AttentionTokenId;
  products: number[];
  rawScore: number;
  scaledScore: number;
}

export interface AttentionQkvTrace {
  projections: AttentionProjection[];
  dotProducts: AttentionDotProduct[];
  rawScoreMatrix: number[][];
  scaledScoreMatrix: number[][];
  scaleDivisor: number;
}

export const DEFAULT_ATTENTION_TOKENS: readonly AttentionToken[] = [
  { id: "red", label: "red", embedding: [1, 0] },
  { id: "blue", label: "blue", embedding: [0, 1] },
  { id: "purple", label: "purple", embedding: [1, 1] },
];

export const DEFAULT_QUERY_MATRIX: readonly (readonly number[])[] = [
  [1, 0],
  [0, 1],
];

export const DEFAULT_KEY_MATRIX: readonly (readonly number[])[] = [
  [1, 1],
  [-1, 1],
];

export const DEFAULT_VALUE_MATRIX: readonly (readonly number[])[] = [
  [2, 0],
  [0, 1],
];

function cleanZero(value: number): number {
  return Math.abs(value) < 1e-12 ? 0 : value;
}

function project(
  row: readonly number[],
  matrix: readonly (readonly number[])[],
): number[] {
  if (
    row.length === 0
    || matrix.length !== row.length
    || matrix.some((matrixRow) => matrixRow.length !== matrix[0]!.length)
    || ![...row, ...matrix.flat()].every(Number.isFinite)
  ) {
    throw new Error("NN12 V1 needs finite row vectors and compatible rectangular matrices.");
  }

  return matrix[0]!.map((_, column) => cleanZero(
    row.reduce((sum, value, index) => sum + value * matrix[index]![column]!, 0),
  ));
}

export function traceAttentionQkv(
  tokens: readonly AttentionToken[] = DEFAULT_ATTENTION_TOKENS,
  queryMatrix: readonly (readonly number[])[] = DEFAULT_QUERY_MATRIX,
  keyMatrix: readonly (readonly number[])[] = DEFAULT_KEY_MATRIX,
  valueMatrix: readonly (readonly number[])[] = DEFAULT_VALUE_MATRIX,
): AttentionQkvTrace {
  if (
    tokens.length !== 3
    || new Set(tokens.map((token) => token.id)).size !== tokens.length
    || tokens.some((token) => token.label.length === 0 || token.embedding.length !== 2)
    || [queryMatrix, keyMatrix, valueMatrix].some(
      (matrix) => matrix.length !== 2 || matrix.some((row) => row.length !== 2),
    )
  ) {
    throw new Error("NN12 V1 needs three unique two-number tokens and three 2 x 2 matrices.");
  }

  const projections = tokens.map((token): AttentionProjection => ({
    id: token.id,
    label: token.label,
    embedding: [...token.embedding],
    query: project(token.embedding, queryMatrix),
    key: project(token.embedding, keyMatrix),
    value: project(token.embedding, valueMatrix),
  }));
  const keyDimension = projections[0]!.key.length;
  const scaleDivisor = Math.sqrt(keyDimension);
  const dotProducts = projections.flatMap((query) => projections.map((key) => {
    const products = query.query.map((value, index) => cleanZero(value * key.key[index]!));
    const rawScore = cleanZero(products.reduce((sum, value) => sum + value, 0));
    return {
      queryId: query.id,
      keyId: key.id,
      products,
      rawScore,
      scaledScore: cleanZero(rawScore / scaleDivisor),
    };
  }));

  return {
    projections,
    dotProducts,
    rawScoreMatrix: projections.map((query) => projections.map((key) => (
      dotProducts.find((item) => item.queryId === query.id && item.keyId === key.id)!.rawScore
    ))),
    scaledScoreMatrix: projections.map((query) => projections.map((key) => (
      dotProducts.find((item) => item.queryId === query.id && item.keyId === key.id)!.scaledScore
    ))),
    scaleDivisor,
  };
}

export function attentionCell(
  trace: AttentionQkvTrace,
  queryId: AttentionTokenId,
  keyId: AttentionTokenId,
): AttentionDotProduct {
  const cell = trace.dotProducts.find(
    (item) => item.queryId === queryId && item.keyId === keyId,
  );
  if (cell === undefined) {
    throw new Error(`Unknown attention cell ${queryId} -> ${keyId}.`);
  }
  return cell;
}
