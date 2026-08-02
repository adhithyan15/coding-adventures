export type BipolarState = -1 | 1;

export interface HopfieldContribution {
  sourceIndex: number;
  weight: number;
  sourceState: BipolarState;
  contribution: number;
}

export interface HopfieldUpdateStep {
  step: number;
  neuronIndex: number;
  stateBefore: BipolarState[];
  incoming: HopfieldContribution[];
  localField: number;
  previousState: BipolarState;
  nextState: BipolarState;
  changed: boolean;
  stateAfter: BipolarState[];
  energyBefore: number;
  energyAfter: number;
  overlapBefore: number;
  overlapAfter: number;
}

export interface HopfieldMemoryTrace {
  storedPattern: BipolarState[];
  normalization: number;
  weights: number[][];
  corruptedState: BipolarState[];
  updateOrder: number[];
  initialEnergy: number;
  initialOverlap: number;
  initialHammingDistance: number;
  updates: HopfieldUpdateStep[];
  finalState: BipolarState[];
  finalEnergy: number;
  finalOverlap: number;
  finalHammingDistance: number;
  converged: boolean;
}

export const DEFAULT_HOPFIELD_PATTERN: BipolarState[] = [1, -1, 1, -1];
export const DEFAULT_HOPFIELD_CORRUPTED_STATE: BipolarState[] = [-1, -1, 1, -1];
export const DEFAULT_HOPFIELD_UPDATE_ORDER = [0, 1, 2, 3];

function normalizeZero(value: number): number {
  return Math.abs(value) < 1e-12 ? 0 : value;
}

function assertBipolar(values: readonly number[], name: string): asserts values is readonly BipolarState[] {
  if (values.length < 2 || values.some((value) => value !== -1 && value !== 1)) {
    throw new Error(`${name} must contain at least two bipolar values (-1 or +1).`);
  }
}

export function hopfieldWeights(pattern: readonly BipolarState[]): number[][] {
  const normalization = pattern.length;
  return pattern.map((target, row) => pattern.map((source, column) => (
    row === column ? 0 : (target * source) / normalization
  )));
}

export function hopfieldEnergy(
  state: readonly BipolarState[],
  weights: readonly (readonly number[])[],
): number {
  let directedSum = 0;
  for (let row = 0; row < state.length; row += 1) {
    for (let column = 0; column < state.length; column += 1) {
      directedSum += weights[row]![column]! * state[row]! * state[column]!;
    }
  }
  return normalizeZero(-0.5 * directedSum);
}

export function normalizedOverlap(
  pattern: readonly BipolarState[],
  state: readonly BipolarState[],
): number {
  return pattern.reduce((sum, value, index) => sum + value * state[index]!, 0)
    / pattern.length;
}

export function hammingDistance(
  pattern: readonly BipolarState[],
  state: readonly BipolarState[],
): number {
  return pattern.filter((value, index) => value !== state[index]).length;
}

export function traceHopfieldRecall(
  storedPattern: readonly number[] = DEFAULT_HOPFIELD_PATTERN,
  corruptedState: readonly number[] = DEFAULT_HOPFIELD_CORRUPTED_STATE,
  updateOrder: readonly number[] = DEFAULT_HOPFIELD_UPDATE_ORDER,
): HopfieldMemoryTrace {
  assertBipolar(storedPattern, "storedPattern");
  assertBipolar(corruptedState, "corruptedState");
  if (
    storedPattern.length !== corruptedState.length
    || updateOrder.length !== storedPattern.length
    || new Set(updateOrder).size !== updateOrder.length
    || updateOrder.some((index) => !Number.isInteger(index) || index < 0 || index >= storedPattern.length)
  ) {
    throw new Error("NN20 V1 needs equal-sized states and one permutation of every neuron index.");
  }

  const pattern = [...storedPattern];
  const initialState = [...corruptedState];
  const weights = hopfieldWeights(pattern);
  const initialEnergy = hopfieldEnergy(initialState, weights);
  const initialOverlap = normalizedOverlap(pattern, initialState);
  const updates: HopfieldUpdateStep[] = [];
  let state = [...initialState];

  updateOrder.forEach((neuronIndex, step) => {
    const stateBefore = [...state];
    const incoming = stateBefore.map((sourceState, sourceIndex) => {
      const weight = weights[neuronIndex]![sourceIndex]!;
      return {
        sourceIndex,
        weight,
        sourceState,
        contribution: normalizeZero(weight * sourceState),
      };
    });
    const localField = normalizeZero(
      incoming.reduce((sum, row) => sum + row.contribution, 0),
    );
    const previousState = stateBefore[neuronIndex]!;
    const nextState: BipolarState = localField > 0
      ? 1
      : localField < 0
        ? -1
        : previousState;
    state = [...stateBefore];
    state[neuronIndex] = nextState;
    updates.push({
      step,
      neuronIndex,
      stateBefore,
      incoming,
      localField,
      previousState,
      nextState,
      changed: nextState !== previousState,
      stateAfter: [...state],
      energyBefore: hopfieldEnergy(stateBefore, weights),
      energyAfter: hopfieldEnergy(state, weights),
      overlapBefore: normalizedOverlap(pattern, stateBefore),
      overlapAfter: normalizedOverlap(pattern, state),
    });
  });

  const finalEnergy = hopfieldEnergy(state, weights);
  const finalOverlap = normalizedOverlap(pattern, state);
  const finalHammingDistance = hammingDistance(pattern, state);
  return {
    storedPattern: pattern,
    normalization: pattern.length,
    weights,
    corruptedState: initialState,
    updateOrder: [...updateOrder],
    initialEnergy,
    initialOverlap,
    initialHammingDistance: hammingDistance(pattern, initialState),
    updates,
    finalState: [...state],
    finalEnergy,
    finalOverlap,
    finalHammingDistance,
    converged: finalHammingDistance === 0
      && updates.every((row) => row.energyAfter <= row.energyBefore + 1e-12),
  };
}
