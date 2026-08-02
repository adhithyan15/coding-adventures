export interface RecurrentParameters {
  inputWeight: number;
  recurrentWeight: number;
  bias: number;
}

export interface RecurrentStepTrace {
  time: number;
  input: number;
  previousState: number;
  inputProduct: number;
  recurrentProduct: number;
  bias: number;
  preactivation: number;
  state: number;
}

export interface RecurrentUnrollTrace {
  steps: RecurrentStepTrace[];
  states: number[];
  finalState: number;
}

export const DEFAULT_RECURRENT_INPUTS = [1, 2, 0] as const;
export const DEFAULT_RECURRENT_INITIAL_STATE = 0;
export const DEFAULT_RECURRENT_PARAMETERS: Readonly<RecurrentParameters> = {
  inputWeight: 2,
  recurrentWeight: 0.5,
  bias: -1,
};

function cleanZero(value: number): number {
  return Math.abs(value) < 1e-12 ? 0 : value;
}

export function traceRecurrentUnroll(
  inputs: readonly number[] = DEFAULT_RECURRENT_INPUTS,
  initialState = DEFAULT_RECURRENT_INITIAL_STATE,
  parameters: Readonly<RecurrentParameters> = DEFAULT_RECURRENT_PARAMETERS,
  recurrentEnabled = true,
): RecurrentUnrollTrace {
  if (
    inputs.length !== 3
    || ![
      ...inputs,
      initialState,
      parameters.inputWeight,
      parameters.recurrentWeight,
      parameters.bias,
    ].every(Number.isFinite)
  ) {
    throw new Error("NN09 V1 needs three finite inputs, state, and parameters.");
  }

  let previousState = initialState;
  const steps = inputs.map((input, time): RecurrentStepTrace => {
    const inputProduct = cleanZero(parameters.inputWeight * input);
    const recurrentProduct = recurrentEnabled
      ? cleanZero(parameters.recurrentWeight * previousState)
      : 0;
    const preactivation = cleanZero(
      inputProduct + recurrentProduct + parameters.bias,
    );
    const state = cleanZero(Math.max(0, preactivation));
    const step = {
      time,
      input,
      previousState,
      inputProduct,
      recurrentProduct,
      bias: parameters.bias,
      preactivation,
      state,
    };
    previousState = state;
    return step;
  });

  return {
    steps,
    states: steps.map((step) => step.state),
    finalState: steps[steps.length - 1]!.state,
  };
}
