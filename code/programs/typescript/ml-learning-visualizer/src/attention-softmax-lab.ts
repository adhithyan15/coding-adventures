import {
  traceAttentionQkv,
  type AttentionTokenId,
} from "./attention-qkv-lab.js";

export interface AttentionSoftmaxRow {
  queryId: AttentionTokenId;
  allowed: boolean[];
  scaledScores: number[];
  maskedScores: Array<number | null>;
  rowMax: number;
  shiftedScores: Array<number | null>;
  exponentials: number[];
  denominator: number;
  weights: number[];
  values: number[][];
  valueContributions: number[][];
  context: number[];
}

export interface AttentionSoftmaxTrace {
  causal: boolean;
  tokenIds: AttentionTokenId[];
  rows: AttentionSoftmaxRow[];
  weightMatrix: number[][];
  contextMatrix: number[][];
}

const qkvTrace = traceAttentionQkv();

export const DEFAULT_ATTENTION_TOKEN_IDS = qkvTrace.projections.map(
  (projection) => projection.id,
);
export const DEFAULT_SCALED_SCORE_MATRIX = qkvTrace.scaledScoreMatrix;
export const DEFAULT_ATTENTION_VALUES = qkvTrace.projections.map(
  (projection) => projection.value,
);

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

export function traceAttentionSoftmax(
  causal = true,
  scaledScoreMatrix: readonly (readonly number[])[] = DEFAULT_SCALED_SCORE_MATRIX,
  values: readonly (readonly number[])[] = DEFAULT_ATTENTION_VALUES,
  tokenIds: readonly AttentionTokenId[] = DEFAULT_ATTENTION_TOKEN_IDS,
): AttentionSoftmaxTrace {
  if (
    tokenIds.length !== 3
    || new Set(tokenIds).size !== tokenIds.length
    || !validMatrix(scaledScoreMatrix, 3, 3)
    || !validMatrix(values, 3, 2)
  ) {
    throw new Error(
      "NN13 V1 needs three token IDs, a finite 3 x 3 score matrix, and finite 3 x 2 values.",
    );
  }

  const rows = scaledScoreMatrix.map(
    (scoreRow, queryIndex): AttentionSoftmaxRow => {
      const allowed = scoreRow.map(
        (_, keyIndex) => !causal || keyIndex <= queryIndex,
      );
      const maskedScores = scoreRow.map((score, keyIndex) => (
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
      const denominator = exponentials.reduce(
        (sum, exponential) => sum + exponential,
        0,
      );
      const weights = exponentials.map(
        (exponential) => cleanZero(exponential / denominator),
      );
      const valueContributions = values.map((value, keyIndex) => (
        value.map((coordinate) => cleanZero(weights[keyIndex]! * coordinate))
      ));
      const context = values[0]!.map((_, coordinate) => cleanZero(
        valueContributions.reduce(
          (sum, contribution) => sum + contribution[coordinate]!,
          0,
        ),
      ));

      return {
        queryId: tokenIds[queryIndex]!,
        allowed,
        scaledScores: [...scoreRow],
        maskedScores,
        rowMax,
        shiftedScores,
        exponentials,
        denominator,
        weights,
        values: values.map((value) => [...value]),
        valueContributions,
        context,
      };
    },
  );

  return {
    causal,
    tokenIds: [...tokenIds],
    rows,
    weightMatrix: rows.map((row) => row.weights),
    contextMatrix: rows.map((row) => row.context),
  };
}

export function attentionSoftmaxRow(
  trace: AttentionSoftmaxTrace,
  queryId: AttentionTokenId,
): AttentionSoftmaxRow {
  const row = trace.rows.find((item) => item.queryId === queryId);
  if (row === undefined) {
    throw new Error(`Unknown attention softmax query ${queryId}.`);
  }
  return row;
}
