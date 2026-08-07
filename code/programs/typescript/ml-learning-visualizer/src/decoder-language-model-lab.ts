export type DecoderTokenId = "red" | "blue" | "purple";

export interface DecoderTrainingRow {
  position: number;
  inputToken: DecoderTokenId;
  targetToken: DecoderTokenId;
  targetIndex: number;
  causalPrefix: DecoderTokenId[];
  decoderState: number[];
  logitProducts: number[][];
  logits: number[];
  rowMax: number;
  shiftedLogits: number[];
  exponentials: number[];
  denominator: number;
  probabilities: number[];
  targetProbability: number;
  loss: number;
  logitGradients: number[];
  unembeddingGradientContribution: number[][];
  biasGradientContribution: number[];
  stateGradient: number[];
}

export interface DecoderPostUpdateRow {
  position: number;
  logits: number[];
  probabilities: number[];
  targetProbability: number;
  loss: number;
}

export interface DecoderTrainingTrace {
  vocabulary: DecoderTokenId[];
  sequence: DecoderTokenId[];
  learningRate: number;
  rows: DecoderTrainingRow[];
  meanLoss: number;
  unembeddingGradient: number[][];
  biasGradient: number[];
  gradientCheck: {
    epsilon: number;
    numericalUnembeddingGradient: number[][];
    numericalBiasGradient: number[];
    maxAbsoluteError: number;
  };
  updatedUnembedding: number[][];
  updatedBias: number[];
  postUpdateRows: DecoderPostUpdateRow[];
  postUpdateMeanLoss: number;
}

export const DEFAULT_DECODER_VOCABULARY: DecoderTokenId[] = [
  "red",
  "blue",
  "purple",
];
export const DEFAULT_DECODER_SEQUENCE: DecoderTokenId[] = [
  "red",
  "blue",
  "purple",
];
export const DEFAULT_DECODER_STATES = [[1, 0], [0, 1]];
export const DEFAULT_DECODER_UNEMBEDDING = [[1, 0, -1], [0, 1, -1]];
export const DEFAULT_DECODER_BIAS = [0, 0, 0];
export const DEFAULT_DECODER_LEARNING_RATE = 0.5;

function cleanZero(value: number): number {
  return Math.abs(value) < 1e-12 ? 0 : value;
}

function validMatrix(
  matrix: readonly (readonly number[])[],
  rows: number,
  columns: number,
): boolean {
  return matrix.length === rows
    && matrix.every((row) => (
      row.length === columns && row.every(Number.isFinite)
    ));
}

function forwardRow(
  state: readonly number[],
  targetIndex: number,
  unembedding: readonly (readonly number[])[],
  bias: readonly number[],
) {
  const logitProducts = DEFAULT_DECODER_VOCABULARY.map((_, vocabularyIndex) => (
    state.map((value, dimension) => cleanZero(
      value * unembedding[dimension]![vocabularyIndex]!,
    ))
  ));
  const logits = logitProducts.map((products, vocabularyIndex) => cleanZero(
    products.reduce((sum, value) => sum + value, 0) + bias[vocabularyIndex]!,
  ));
  const rowMax = Math.max(...logits);
  const shiftedLogits = logits.map((logit) => cleanZero(logit - rowMax));
  const exponentials = shiftedLogits.map(Math.exp);
  const denominator = exponentials.reduce((sum, value) => sum + value, 0);
  const probabilities = exponentials.map((value) => value / denominator);
  const targetProbability = probabilities[targetIndex]!;
  return {
    logitProducts,
    logits,
    rowMax,
    shiftedLogits,
    exponentials,
    denominator,
    probabilities,
    targetProbability,
    loss: -Math.log(targetProbability),
  };
}

function meanLoss(
  states: readonly (readonly number[])[],
  targetIndices: readonly number[],
  unembedding: readonly (readonly number[])[],
  bias: readonly number[],
): number {
  return states.reduce((sum, state, position) => (
    sum + forwardRow(
      state,
      targetIndices[position]!,
      unembedding,
      bias,
    ).loss
  ), 0) / states.length;
}

export function traceTinyDecoderTraining(
  learningRate = DEFAULT_DECODER_LEARNING_RATE,
  states: readonly (readonly number[])[] = DEFAULT_DECODER_STATES,
  unembedding: readonly (readonly number[])[] = DEFAULT_DECODER_UNEMBEDDING,
  bias: readonly number[] = DEFAULT_DECODER_BIAS,
): DecoderTrainingTrace {
  if (
    !Number.isFinite(learningRate)
    || learningRate <= 0
    || !validMatrix(states, 2, 2)
    || !validMatrix(unembedding, 2, 3)
    || bias.length !== 3
    || !bias.every(Number.isFinite)
  ) {
    throw new Error(
      "NN15 V1 needs two 2D decoder states, a 2 x 3 unembedding, three finite biases, and a positive learning rate.",
    );
  }

  const inputTokens = DEFAULT_DECODER_SEQUENCE.slice(0, -1);
  const targetTokens = DEFAULT_DECODER_SEQUENCE.slice(1);
  const unembeddingGradient = Array.from({ length: 2 }, () => [0, 0, 0]);
  const biasGradient = [0, 0, 0];
  const rows = states.map((state, position): DecoderTrainingRow => {
    const inputToken = inputTokens[position]!;
    const targetToken = targetTokens[position]!;
    const targetIndex = DEFAULT_DECODER_VOCABULARY.indexOf(targetToken);
    const forward = forwardRow(state, targetIndex, unembedding, bias);
    const logitGradients = forward.probabilities.map((probability, index) => (
      (probability - (index === targetIndex ? 1 : 0)) / states.length
    ));
    const unembeddingGradientContribution = state.map((value) => (
      logitGradients.map((gradient) => cleanZero(value * gradient))
    ));
    for (let dimension = 0; dimension < 2; dimension += 1) {
      for (let vocabularyIndex = 0; vocabularyIndex < 3; vocabularyIndex += 1) {
        unembeddingGradient[dimension]![vocabularyIndex]! += (
          unembeddingGradientContribution[dimension]![vocabularyIndex]!
        );
      }
    }
    for (let vocabularyIndex = 0; vocabularyIndex < 3; vocabularyIndex += 1) {
      biasGradient[vocabularyIndex]! += logitGradients[vocabularyIndex]!;
    }
    const stateGradient = state.map((_, dimension) => cleanZero(
      logitGradients.reduce((sum, gradient, vocabularyIndex) => (
        sum + gradient * unembedding[dimension]![vocabularyIndex]!
      ), 0),
    ));

    return {
      position,
      inputToken,
      targetToken,
      targetIndex,
      causalPrefix: DEFAULT_DECODER_SEQUENCE.slice(0, position + 1),
      decoderState: [...state],
      ...forward,
      logitGradients,
      unembeddingGradientContribution,
      biasGradientContribution: [...logitGradients],
      stateGradient,
    };
  });

  const updatedUnembedding = unembedding.map((row, dimension) => (
    row.map((value, vocabularyIndex) => (
      value - learningRate * unembeddingGradient[dimension]![vocabularyIndex]!
    ))
  ));
  const updatedBias = bias.map((value, vocabularyIndex) => (
    value - learningRate * biasGradient[vocabularyIndex]!
  ));
  const gradientEpsilon = 1e-6;
  const targetIndices = rows.map((row) => row.targetIndex);
  const numericalUnembeddingGradient = Array.from(
    { length: 2 },
    () => [0, 0, 0],
  );
  for (let dimension = 0; dimension < 2; dimension += 1) {
    for (let vocabularyIndex = 0; vocabularyIndex < 3; vocabularyIndex += 1) {
      const plus = unembedding.map((row) => [...row]);
      const minus = unembedding.map((row) => [...row]);
      plus[dimension]![vocabularyIndex]! += gradientEpsilon;
      minus[dimension]![vocabularyIndex]! -= gradientEpsilon;
      numericalUnembeddingGradient[dimension]![vocabularyIndex] = (
        meanLoss(states, targetIndices, plus, bias)
        - meanLoss(states, targetIndices, minus, bias)
      ) / (2 * gradientEpsilon);
    }
  }
  const numericalBiasGradient = bias.map((_, vocabularyIndex) => {
    const plus = [...bias];
    const minus = [...bias];
    plus[vocabularyIndex]! += gradientEpsilon;
    minus[vocabularyIndex]! -= gradientEpsilon;
    return (
      meanLoss(states, targetIndices, unembedding, plus)
      - meanLoss(states, targetIndices, unembedding, minus)
    ) / (2 * gradientEpsilon);
  });
  const gradientErrors = [
    ...unembeddingGradient.flatMap((row, dimension) => (
      row.map((value, vocabularyIndex) => Math.abs(
        value - numericalUnembeddingGradient[dimension]![vocabularyIndex]!,
      ))
    )),
    ...biasGradient.map((value, index) => Math.abs(
      value - numericalBiasGradient[index]!,
    )),
  ];
  const postUpdateRows = rows.map((row): DecoderPostUpdateRow => {
    const forward = forwardRow(
      row.decoderState,
      row.targetIndex,
      updatedUnembedding,
      updatedBias,
    );
    return {
      position: row.position,
      logits: forward.logits,
      probabilities: forward.probabilities,
      targetProbability: forward.targetProbability,
      loss: forward.loss,
    };
  });

  return {
    vocabulary: [...DEFAULT_DECODER_VOCABULARY],
    sequence: [...DEFAULT_DECODER_SEQUENCE],
    learningRate,
    rows,
    meanLoss: rows.reduce((sum, row) => sum + row.loss, 0) / rows.length,
    unembeddingGradient,
    biasGradient,
    gradientCheck: {
      epsilon: gradientEpsilon,
      numericalUnembeddingGradient,
      numericalBiasGradient,
      maxAbsoluteError: Math.max(...gradientErrors),
    },
    updatedUnembedding,
    updatedBias,
    postUpdateRows,
    postUpdateMeanLoss: postUpdateRows.reduce(
      (sum, row) => sum + row.loss,
      0,
    ) / postUpdateRows.length,
  };
}

export function decoderTrainingRow(
  trace: DecoderTrainingTrace,
  position: number,
): DecoderTrainingRow {
  const row = trace.rows[position];
  if (row === undefined) {
    throw new Error(`Unknown decoder training position ${position}.`);
  }
  return row;
}
