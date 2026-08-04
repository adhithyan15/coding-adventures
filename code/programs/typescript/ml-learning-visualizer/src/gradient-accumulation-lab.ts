export type GradientAccumulationScenarioId =
  | "accumulate_two_calls"
  | "zero_between_calls"
  | "mean_then_zero"
  | "stale_next_batch";

export interface GradientSample {
  id: string;
  input: number;
  target: number;
}

export type GradientScheduleEvent =
  | { kind: "backward"; sampleId: string }
  | { kind: "zero_grad" }
  | { kind: "optimizer_step"; divisor: number };

export interface GradientAccumulationScenario {
  id: GradientAccumulationScenarioId;
  title: string;
  summary: string;
  initialParameter: number;
  learningRate: number;
  samples: readonly GradientSample[];
  events: readonly GradientScheduleEvent[];
}

interface GradientTraceBase {
  index: number;
  kind: GradientScheduleEvent["kind"];
  parameterBefore: number;
  parameterAfter: number;
  bufferBefore: number;
  bufferAfter: number;
}

export interface BackwardBufferTrace extends GradientTraceBase {
  kind: "backward";
  sampleId: string;
  input: number;
  target: number;
  prediction: number;
  residual: number;
  loss: number;
  localGradient: number;
  numericalGradient: number;
  gradientAbsoluteError: number;
}

export interface ZeroBufferTrace extends GradientTraceBase {
  kind: "zero_grad";
}

export interface OptimizerBufferTrace extends GradientTraceBase {
  kind: "optimizer_step";
  divisor: number;
  appliedGradient: number;
  parameterDelta: number;
}

export type GradientBufferTrace =
  | BackwardBufferTrace
  | ZeroBufferTrace
  | OptimizerBufferTrace;

export interface GradientAccumulationTrace {
  scenario: GradientAccumulationScenario;
  steps: GradientBufferTrace[];
  finalParameter: number;
  finalGradientBuffer: number;
  backwardCalls: number;
  optimizerSteps: number;
  zeroCalls: number;
  maxGradientAbsoluteError: number;
}

const IDENTIFIER = /^[a-z][a-z0-9_]{0,31}$/;
const MAX_SAMPLES = 4;
const MAX_EVENTS = 12;
const MAX_ABSOLUTE_INPUT = 1e3;
const MAX_ABSOLUTE_DERIVED = 1e12;
const DEFAULT_EPSILON = 1e-5;

const SHARED_SAMPLES = [
  { id: "a", input: 2, target: 1 },
  { id: "b", input: -1, target: 1 },
] as const;

export const GRADIENT_ACCUMULATION_SCENARIOS: readonly GradientAccumulationScenario[] = [
  {
    id: "accumulate_two_calls",
    title: "Two backward calls",
    summary: "The second backward adds 2 to the 2 already in w.grad.",
    initialParameter: 1,
    learningRate: 0.1,
    samples: SHARED_SAMPLES,
    events: [
      { kind: "backward", sampleId: "a" },
      { kind: "backward", sampleId: "b" },
    ],
  },
  {
    id: "zero_between_calls",
    title: "Zero between calls",
    summary: "Clearing the buffer makes the second gradient stand alone.",
    initialParameter: 1,
    learningRate: 0.1,
    samples: SHARED_SAMPLES,
    events: [
      { kind: "backward", sampleId: "a" },
      { kind: "zero_grad" },
      { kind: "backward", sampleId: "b" },
    ],
  },
  {
    id: "mean_then_zero",
    title: "Mean, step, zero",
    summary: "Two micro-batches become one mean gradient, then zero starts the next batch clean.",
    initialParameter: 1,
    learningRate: 0.1,
    samples: SHARED_SAMPLES,
    events: [
      { kind: "backward", sampleId: "a" },
      { kind: "backward", sampleId: "b" },
      { kind: "optimizer_step", divisor: 2 },
      { kind: "zero_grad" },
    ],
  },
  {
    id: "stale_next_batch",
    title: "Forgotten zero",
    summary: "A new 0.8 gradient lands on a stale buffer of 4 and drives the wrong update.",
    initialParameter: 1,
    learningRate: 0.1,
    samples: [...SHARED_SAMPLES, { id: "c", input: 1, target: 0 }],
    events: [
      { kind: "backward", sampleId: "a" },
      { kind: "backward", sampleId: "b" },
      { kind: "optimizer_step", divisor: 2 },
      { kind: "backward", sampleId: "c" },
      { kind: "optimizer_step", divisor: 1 },
    ],
  },
] as const;

function finite(value: number, context: string): number {
  if (!Number.isFinite(value) || Math.abs(value) > MAX_ABSOLUTE_DERIVED) {
    throw new Error(`${context} must remain finite and bounded`);
  }
  return value;
}

function boundedNumber(value: unknown, context: string): asserts value is number {
  if (typeof value !== "number" || !Number.isFinite(value)
    || Math.abs(value) > MAX_ABSOLUTE_INPUT) {
    throw new Error(`${context} must be finite and bounded`);
  }
}

function validateScenario(scenario: GradientAccumulationScenario): void {
  if (typeof scenario !== "object" || scenario === null
    || !Array.isArray(scenario.samples) || !Array.isArray(scenario.events)) {
    throw new Error("gradient schedule must contain bounded sample and event arrays");
  }
  if (typeof scenario.id !== "string" || !IDENTIFIER.test(scenario.id)
    || typeof scenario.title !== "string" || scenario.title.length < 1
    || scenario.title.length > 256
    || typeof scenario.summary !== "string" || scenario.summary.length < 1
    || scenario.summary.length > 512) {
    throw new Error("gradient schedule metadata must contain bounded strings");
  }
  if (scenario.samples.length < 1 || scenario.samples.length > MAX_SAMPLES
    || scenario.events.length < 1 || scenario.events.length > MAX_EVENTS) {
    throw new Error("gradient schedule exceeds bounded sizes");
  }
  boundedNumber(scenario.initialParameter, "initial parameter");
  boundedNumber(scenario.learningRate, "learning rate");
  if (scenario.learningRate <= 0 || scenario.learningRate > 1) {
    throw new Error("learning rate must be in (0, 1]");
  }
  const sampleIds = new Set<string>();
  scenario.samples.forEach((sample) => {
    if (typeof sample !== "object" || sample === null
      || typeof sample.id !== "string" || !IDENTIFIER.test(sample.id)) {
      throw new Error("sample must have a bounded identifier");
    }
    if (sampleIds.has(sample.id)) throw new Error(`duplicate sample id ${sample.id}`);
    boundedNumber(sample.input, `sample ${sample.id} input`);
    boundedNumber(sample.target, `sample ${sample.id} target`);
    sampleIds.add(sample.id);
  });
  let backwardCalls = 0;
  scenario.events.forEach((event) => {
    if (typeof event !== "object" || event === null) throw new Error("event must be an object");
    if (event.kind === "backward") {
      if (typeof event.sampleId !== "string" || !IDENTIFIER.test(event.sampleId)) {
        throw new Error("backward sample id must be a bounded identifier");
      }
      if (!sampleIds.has(event.sampleId)) {
        throw new Error(`backward references unknown sample ${event.sampleId}`);
      }
      backwardCalls += 1;
    } else if (event.kind === "optimizer_step") {
      if (!Number.isInteger(event.divisor) || event.divisor < 1
        || event.divisor > MAX_SAMPLES) {
        throw new Error("optimizer divisor must be a bounded positive integer");
      }
    } else if (event.kind !== "zero_grad") {
      throw new Error("unsupported gradient schedule event");
    }
  });
  if (backwardCalls === 0) throw new Error("gradient schedule needs a backward call");
}

function snapshotScenario(
  scenario: GradientAccumulationScenario,
): GradientAccumulationScenario {
  const samples = scenario.samples.map((sample) => Object.freeze({ ...sample }));
  const events = scenario.events.map((event) => Object.freeze({ ...event }));
  return Object.freeze({
    id: scenario.id,
    title: scenario.title,
    summary: scenario.summary,
    initialParameter: scenario.initialParameter,
    learningRate: scenario.learningRate,
    samples: Object.freeze(samples),
    events: Object.freeze(events),
  });
}

function sampleLoss(parameter: number, sample: GradientSample): number {
  const prediction = finite(parameter * sample.input, "finite-difference prediction");
  const residual = finite(prediction - sample.target, "finite-difference residual");
  return finite(0.5 * residual * residual, "finite-difference loss");
}

export function traceGradientAccumulationProgram(
  scenario: GradientAccumulationScenario,
  epsilon = DEFAULT_EPSILON,
): GradientAccumulationTrace {
  validateScenario(scenario);
  if (!Number.isFinite(epsilon) || epsilon < 1e-12 || epsilon > 1) {
    throw new Error("finite-difference epsilon must be in [1e-12, 1]");
  }
  const snapshot = snapshotScenario(scenario);
  const samples = new Map(snapshot.samples.map((sample) => [sample.id, sample]));
  const steps: GradientBufferTrace[] = [];
  let parameter = snapshot.initialParameter;
  let gradientBuffer = 0;
  let backwardCalls = 0;
  let optimizerSteps = 0;
  let zeroCalls = 0;
  let maxGradientAbsoluteError = 0;

  snapshot.events.forEach((event, index) => {
    const parameterBefore = parameter;
    const bufferBefore = gradientBuffer;
    if (event.kind === "backward") {
      const sample = samples.get(event.sampleId)!;
      const prediction = finite(parameter * sample.input, `event ${index} prediction`);
      const residual = finite(prediction - sample.target, `event ${index} residual`);
      const loss = finite(0.5 * residual * residual, `event ${index} loss`);
      const localGradient = finite(residual * sample.input, `event ${index} gradient`);
      gradientBuffer = finite(gradientBuffer + localGradient, `event ${index} buffer`);
      const numericalGradient = finite(
        (sampleLoss(parameter + epsilon, sample) - sampleLoss(parameter - epsilon, sample))
          / (2 * epsilon),
        `event ${index} numerical gradient`,
      );
      const gradientAbsoluteError = Math.abs(localGradient - numericalGradient);
      maxGradientAbsoluteError = Math.max(maxGradientAbsoluteError, gradientAbsoluteError);
      backwardCalls += 1;
      steps.push({
        index,
        kind: "backward",
        sampleId: sample.id,
        input: sample.input,
        target: sample.target,
        parameterBefore,
        parameterAfter: parameter,
        bufferBefore,
        bufferAfter: gradientBuffer,
        prediction,
        residual,
        loss,
        localGradient,
        numericalGradient,
        gradientAbsoluteError,
      });
    } else if (event.kind === "zero_grad") {
      gradientBuffer = 0;
      zeroCalls += 1;
      steps.push({
        index,
        kind: "zero_grad",
        parameterBefore,
        parameterAfter: parameter,
        bufferBefore,
        bufferAfter: gradientBuffer,
      });
    } else {
      const appliedGradient = finite(
        gradientBuffer / event.divisor,
        `event ${index} applied gradient`,
      );
      const parameterDelta = finite(
        -snapshot.learningRate * appliedGradient,
        `event ${index} parameter delta`,
      );
      parameter = finite(parameter + parameterDelta, `event ${index} parameter`);
      optimizerSteps += 1;
      steps.push({
        index,
        kind: "optimizer_step",
        parameterBefore,
        parameterAfter: parameter,
        bufferBefore,
        bufferAfter: gradientBuffer,
        divisor: event.divisor,
        appliedGradient,
        parameterDelta,
      });
    }
  });

  return {
    scenario: snapshot,
    steps,
    finalParameter: parameter,
    finalGradientBuffer: gradientBuffer,
    backwardCalls,
    optimizerSteps,
    zeroCalls,
    maxGradientAbsoluteError,
  };
}

export function traceGradientAccumulation(
  scenarioId: GradientAccumulationScenarioId,
): GradientAccumulationTrace {
  const scenario = GRADIENT_ACCUMULATION_SCENARIOS.find((item) => item.id === scenarioId);
  if (!scenario) throw new Error(`unknown gradient accumulation scenario: ${scenarioId}`);
  return traceGradientAccumulationProgram(scenario);
}
