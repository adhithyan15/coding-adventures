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

export interface ParameterGradients {
  inputWeight: number;
  recurrentWeight: number;
  bias: number;
}

export interface BpttBackwardStep {
  time: number;
  directStateGradient: number;
  futureStateGradient: number;
  stateGradient: number;
  reluDerivative: number;
  preactivationGradient: number;
  parameterContributions: ParameterGradients;
  previousStateGradient: number;
}

export interface RecurrentBpttTrace {
  forward: RecurrentUnrollTrace;
  target: number;
  loss: number;
  backwardSteps: BpttBackwardStep[];
  gradientTotals: ParameterGradients & { initialState: number };
  numericalGradients: ParameterGradients;
  gradientErrors: ParameterGradients;
  maxGradientError: number;
  update: {
    learningRate: number;
    parameters: RecurrentParameters;
    preactivations: number[];
    states: number[];
    loss: number;
  };
}

export const DEFAULT_RECURRENT_INPUTS = [1, 2, 0] as const;
export const DEFAULT_RECURRENT_INITIAL_STATE = 0;
export const DEFAULT_RECURRENT_PARAMETERS: Readonly<RecurrentParameters> = {
  inputWeight: 2,
  recurrentWeight: 0.5,
  bias: -1,
};
export const DEFAULT_RECURRENT_TARGET = 0;
export const DEFAULT_RECURRENT_LEARNING_RATE = 0.1;
export const DEFAULT_RECURRENT_FINITE_DIFFERENCE_EPSILON = 1e-6;

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

function finalStateLoss(
  inputs: readonly number[],
  initialState: number,
  parameters: Readonly<RecurrentParameters>,
  target: number,
): number {
  const prediction = traceRecurrentUnroll(inputs, initialState, parameters).finalState;
  return 0.5 * (prediction - target) ** 2;
}

function numericalParameterGradient(
  parameter: keyof RecurrentParameters,
  inputs: readonly number[],
  initialState: number,
  parameters: Readonly<RecurrentParameters>,
  target: number,
  epsilon: number,
): number {
  const plus = { ...parameters, [parameter]: parameters[parameter] + epsilon };
  const minus = { ...parameters, [parameter]: parameters[parameter] - epsilon };
  return (
    finalStateLoss(inputs, initialState, plus, target)
    - finalStateLoss(inputs, initialState, minus, target)
  ) / (2 * epsilon);
}

export function traceRecurrentBptt(
  inputs: readonly number[] = DEFAULT_RECURRENT_INPUTS,
  initialState = DEFAULT_RECURRENT_INITIAL_STATE,
  parameters: Readonly<RecurrentParameters> = DEFAULT_RECURRENT_PARAMETERS,
  target = DEFAULT_RECURRENT_TARGET,
  learningRate = DEFAULT_RECURRENT_LEARNING_RATE,
  epsilon = DEFAULT_RECURRENT_FINITE_DIFFERENCE_EPSILON,
): RecurrentBpttTrace {
  if (![target, learningRate, epsilon].every(Number.isFinite) || epsilon <= 0) {
    throw new Error("NN10 V1 needs a finite target and learning rate, plus a positive epsilon.");
  }

  const forward = traceRecurrentUnroll(inputs, initialState, parameters);
  const loss = 0.5 * (forward.finalState - target) ** 2;
  let futureStateGradient = 0;
  const backwardSteps: BpttBackwardStep[] = [];

  for (let time = forward.steps.length - 1; time >= 0; time -= 1) {
    const forwardStep = forward.steps[time]!;
    const directStateGradient = time === forward.steps.length - 1
      ? forward.finalState - target
      : 0;
    const stateGradient = cleanZero(directStateGradient + futureStateGradient);
    const reluDerivative = forwardStep.preactivation > 0 ? 1 : 0;
    const preactivationGradient = cleanZero(stateGradient * reluDerivative);
    const parameterContributions = {
      inputWeight: cleanZero(preactivationGradient * forwardStep.input),
      recurrentWeight: cleanZero(preactivationGradient * forwardStep.previousState),
      bias: preactivationGradient,
    };
    const previousStateGradient = cleanZero(
      preactivationGradient * parameters.recurrentWeight,
    );

    backwardSteps.push({
      time,
      directStateGradient,
      futureStateGradient,
      stateGradient,
      reluDerivative,
      preactivationGradient,
      parameterContributions,
      previousStateGradient,
    });
    futureStateGradient = previousStateGradient;
  }

  const gradientTotals = backwardSteps.reduce(
    (total, step) => ({
      inputWeight: cleanZero(total.inputWeight + step.parameterContributions.inputWeight),
      recurrentWeight: cleanZero(
        total.recurrentWeight + step.parameterContributions.recurrentWeight,
      ),
      bias: cleanZero(total.bias + step.parameterContributions.bias),
      initialState: step.time === 0 ? step.previousStateGradient : total.initialState,
    }),
    { inputWeight: 0, recurrentWeight: 0, bias: 0, initialState: 0 },
  );

  const numericalGradients = {
    inputWeight: numericalParameterGradient(
      "inputWeight", inputs, initialState, parameters, target, epsilon,
    ),
    recurrentWeight: numericalParameterGradient(
      "recurrentWeight", inputs, initialState, parameters, target, epsilon,
    ),
    bias: numericalParameterGradient(
      "bias", inputs, initialState, parameters, target, epsilon,
    ),
  };
  const gradientErrors = {
    inputWeight: Math.abs(gradientTotals.inputWeight - numericalGradients.inputWeight),
    recurrentWeight: Math.abs(
      gradientTotals.recurrentWeight - numericalGradients.recurrentWeight,
    ),
    bias: Math.abs(gradientTotals.bias - numericalGradients.bias),
  };
  const updatedParameters = {
    inputWeight: cleanZero(
      parameters.inputWeight - learningRate * gradientTotals.inputWeight,
    ),
    recurrentWeight: cleanZero(
      parameters.recurrentWeight - learningRate * gradientTotals.recurrentWeight,
    ),
    bias: cleanZero(parameters.bias - learningRate * gradientTotals.bias),
  };
  const updatedForward = traceRecurrentUnroll(inputs, initialState, updatedParameters);

  return {
    forward,
    target,
    loss,
    backwardSteps,
    gradientTotals,
    numericalGradients,
    gradientErrors,
    maxGradientError: Math.max(...Object.values(gradientErrors)),
    update: {
      learningRate,
      parameters: updatedParameters,
      preactivations: updatedForward.steps.map((step) => step.preactivation),
      states: updatedForward.states,
      loss: 0.5 * (updatedForward.finalState - target) ** 2,
    },
  };
}
