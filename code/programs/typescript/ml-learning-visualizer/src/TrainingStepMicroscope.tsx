import { useMemo, useState } from "react";
import {
  DEFAULT_MICROSCOPE_STATE,
  traceTrainingStep,
  type MicroscopeActivation,
  type MicroscopeState,
  type MicroscopeTrace,
} from "./training-microscope.js";

interface PhaseDefinition {
  id: string;
  shortLabel: string;
  title: string;
  question: string;
  formula: (trace: MicroscopeTrace) => string;
  value: (trace: MicroscopeTrace) => string;
  explanation: (trace: MicroscopeTrace) => string;
}

function formatNumber(value: number, digits = 5): string {
  if (!Number.isFinite(value)) {
    return String(value);
  }
  if (Math.abs(value) < 1e-12) {
    return "0";
  }
  if (Math.abs(value) >= 1000 || (Math.abs(value) > 0 && Math.abs(value) < 0.0001)) {
    return value.toExponential(3);
  }
  return Number(value.toFixed(digits)).toString();
}

const PHASES: readonly PhaseDefinition[] = [
  {
    id: "example",
    shortLabel: "Example",
    title: "Choose one training example",
    question: "What information is the neuron trying to connect?",
    formula: (trace) => `x = ${formatNumber(trace.input)}, target = ${formatNumber(trace.target)}`,
    value: (trace) => `x ${formatNumber(trace.input)} / target ${formatNumber(trace.target)}`,
    explanation: () => "The input is evidence. The target is the answer we want this one neuron to approach.",
  },
  {
    id: "multiply",
    shortLabel: "Multiply",
    title: "Scale the input by its weight",
    question: "How strongly does this input contribute?",
    formula: (trace) => `${formatNumber(trace.input)} x ${formatNumber(trace.weight)} = ${formatNumber(trace.weightedInput)}`,
    value: (trace) => formatNumber(trace.weightedInput),
    explanation: (trace) => `The current weight ${formatNumber(trace.weight)} turns the input into one weighted contribution.`,
  },
  {
    id: "bias",
    shortLabel: "Add bias",
    title: "Shift the weighted contribution",
    question: "What should the neuron predict when its input contribution is zero?",
    formula: (trace) => `${formatNumber(trace.weightedInput)} + ${formatNumber(trace.bias)} = ${formatNumber(trace.preActivation)}`,
    value: (trace) => `z = ${formatNumber(trace.preActivation)}`,
    explanation: (trace) => `The bias ${formatNumber(trace.bias)} shifts the neuron before any activation is applied.`,
  },
  {
    id: "activation",
    shortLabel: "Activate",
    title: "Transform the raw sum",
    question: "What range or shape should the output have?",
    formula: (trace) => `${trace.activation}(${formatNumber(trace.preActivation)}) = ${formatNumber(trace.prediction)}`,
    value: (trace) => `prediction ${formatNumber(trace.prediction)}`,
    explanation: (trace) => `The ${trace.activation} activation transforms z into the value compared with the target.`,
  },
  {
    id: "loss",
    shortLabel: "Measure loss",
    title: "Turn the mistake into one score",
    question: "How wrong is the current prediction?",
    formula: (trace) => `(${formatNumber(trace.prediction)} - ${formatNumber(trace.target)})^2 = ${formatNumber(trace.loss)}`,
    value: (trace) => `loss ${formatNumber(trace.loss)}`,
    explanation: (trace) => `The signed error is ${formatNumber(trace.error)}. Squaring it makes the score positive and magnifies larger mistakes.`,
  },
  {
    id: "backprop",
    shortLabel: "Backprop",
    title: "Assign responsibility with the chain rule",
    question: "How much did each parameter contribute to the loss?",
    formula: (trace) => `dL/dw = ${formatNumber(trace.lossPredictionDerivative)} x ${formatNumber(trace.activationDerivative)} x ${formatNumber(trace.input)} = ${formatNumber(trace.gradientWeight)}`,
    value: (trace) => `dw ${formatNumber(trace.gradientWeight)} / db ${formatNumber(trace.gradientBias)}`,
    explanation: () => "Backpropagation multiplies local derivatives along each path from the loss to a parameter.",
  },
  {
    id: "update",
    shortLabel: "Update",
    title: "Move the parameters against the gradient",
    question: "What small change should reduce the loss?",
    formula: (trace) => `w' = ${formatNumber(trace.weight)} - ${formatNumber(trace.learningRate)} x ${formatNumber(trace.gradientWeight)} = ${formatNumber(trace.nextWeight)}`,
    value: (trace) => `w' ${formatNumber(trace.nextWeight)} / b' ${formatNumber(trace.nextBias)}`,
    explanation: (trace) => `With the proposed parameters, the loss changes from ${formatNumber(trace.loss)} to ${formatNumber(trace.nextLoss)}.`,
  },
];

function numberValue(value: string, fallback: number): number {
  const parsed = Number(value);
  return Number.isFinite(parsed) ? parsed : fallback;
}

export function TrainingStepMicroscope() {
  const [state, setState] = useState<MicroscopeState>(DEFAULT_MICROSCOPE_STATE);
  const [phaseIndex, setPhaseIndex] = useState(0);
  const [updateCount, setUpdateCount] = useState(0);
  const trace = useMemo(() => traceTrainingStep(state), [state]);
  const phase = PHASES[phaseIndex]!;

  function setNumber(field: keyof Omit<MicroscopeState, "activation">, value: string): void {
    setState((current) => ({ ...current, [field]: numberValue(value, current[field]) }));
    setPhaseIndex(0);
  }

  function setActivation(value: MicroscopeActivation): void {
    setState((current) => ({ ...current, activation: value }));
    setPhaseIndex(0);
  }

  function applyUpdate(): void {
    setState((current) => {
      const currentTrace = traceTrainingStep(current);
      return {
        ...current,
        weight: Number(currentTrace.nextWeight.toPrecision(12)),
        bias: Number(currentTrace.nextBias.toPrecision(12)),
      };
    });
    setUpdateCount((count) => count + 1);
    setPhaseIndex(0);
  }

  function reset(): void {
    setState(DEFAULT_MICROSCOPE_STATE);
    setPhaseIndex(0);
    setUpdateCount(0);
  }

  return (
    <main className="workspace workspace--microscope">
      <section className="microscope-stage" aria-label="Training step microscope">
        <div className="lab-intro">
          <div>
            <p className="eyebrow">One neuron / one example / one update</p>
            <h2>Training-step microscope</h2>
            <p>Reveal the arithmetic in order. Future phases stay hidden until you reach them.</p>
          </div>
          <div className="lab-chip">update {updateCount}</div>
        </div>

        <ol className="phase-strip" aria-label="Training phases">
          {PHASES.map((item, index) => (
            <li key={item.id}>
              <button
                className={`phase-button${index === phaseIndex ? " phase-button--active" : ""}${index < phaseIndex ? " phase-button--complete" : ""}`}
                type="button"
                onClick={() => setPhaseIndex(index)}
                aria-current={index === phaseIndex ? "step" : undefined}
              >
                <span>{index + 1}</span>
                {item.shortLabel}
              </button>
            </li>
          ))}
        </ol>

        <section className="microscope-focus" aria-live="polite">
          <div>
            <p className="eyebrow">Phase {phaseIndex + 1} of {PHASES.length}</p>
            <h2>{phase.title}</h2>
            <p className="focus-question">{phase.question}</p>
          </div>
          <code>{phase.formula(trace)}</code>
          <p>{phase.explanation(trace)}</p>
        </section>

        <section className="signal-pipeline" aria-label="Neuron signal pipeline">
          {PHASES.map((item, index) => (
            <button
              key={item.id}
              className={`signal-node${index === phaseIndex ? " signal-node--active" : ""}${index > phaseIndex ? " signal-node--locked" : ""}`}
              type="button"
              onClick={() => setPhaseIndex(index)}
            >
              <span>{item.shortLabel}</span>
              <strong>{index <= phaseIndex ? item.value(trace) : "?"}</strong>
            </button>
          ))}
        </section>

        {phase.id === "backprop" && (
          <section className="derivative-panel" aria-label="Chain rule factors">
            <div className="derivative-factor">
              <span>Loss slope</span>
              <code>dL/dy = {formatNumber(trace.lossPredictionDerivative)}</code>
            </div>
            <div className="derivative-times" aria-hidden="true">x</div>
            <div className="derivative-factor">
              <span>Activation slope</span>
              <code>dy/dz = {formatNumber(trace.activationDerivative)}</code>
            </div>
            <div className="derivative-times" aria-hidden="true">x</div>
            <div className="derivative-factor">
              <span>Weight path</span>
              <code>dz/dw = {formatNumber(trace.preActivationWeightDerivative)}</code>
            </div>
            <div className="derivative-equals" aria-hidden="true">=</div>
            <div className="derivative-factor derivative-factor--result">
              <span>Weight gradient</span>
              <code>dL/dw = {formatNumber(trace.gradientWeight)}</code>
            </div>
          </section>
        )}

        {phase.id === "update" && (
          <section className="before-after" aria-label="Parameter update result">
            <div>
              <span>Before</span>
              <strong>w {formatNumber(trace.weight)} / b {formatNumber(trace.bias)}</strong>
              <small>prediction {formatNumber(trace.prediction)} / loss {formatNumber(trace.loss)}</small>
            </div>
            <div className="update-arrow" aria-hidden="true">-&gt;</div>
            <div>
              <span>After proposed update</span>
              <strong>w {formatNumber(trace.nextWeight)} / b {formatNumber(trace.nextBias)}</strong>
              <small>prediction {formatNumber(trace.nextPrediction)} / loss {formatNumber(trace.nextLoss)}</small>
            </div>
          </section>
        )}

        <div className="microscope-actions">
          <button type="button" disabled={phaseIndex === 0} onClick={() => setPhaseIndex((index) => Math.max(0, index - 1))}>Previous phase</button>
          {phaseIndex < PHASES.length - 1 ? (
            <button className="primary-action" type="button" onClick={() => setPhaseIndex((index) => Math.min(PHASES.length - 1, index + 1))}>Next phase</button>
          ) : (
            <button className="primary-action" type="button" onClick={applyUpdate}>Apply this update</button>
          )}
          <button type="button" onClick={reset}>Reset example</button>
        </div>
      </section>

      <aside className="controls microscope-controls" aria-label="Microscope values">
        <div className="lesson">
          <span>Change one thing</span>
          <p>Adjust a value, then step forward again and watch where its effect first appears.</p>
        </div>

        <label className="field">
          <span>Input x</span>
          <input aria-label="Input x" type="number" step="0.1" value={state.input} onChange={(event) => setNumber("input", event.target.value)} />
        </label>
        <label className="field">
          <span>Target</span>
          <input aria-label="Target" type="number" step="0.1" value={state.target} onChange={(event) => setNumber("target", event.target.value)} />
        </label>
        <div className="field-grid">
          <label className="field">
            <span>Weight w</span>
            <input aria-label="Weight w" type="number" step="0.05" value={state.weight} onChange={(event) => setNumber("weight", event.target.value)} />
          </label>
          <label className="field">
            <span>Bias b</span>
            <input aria-label="Bias b" type="number" step="0.05" value={state.bias} onChange={(event) => setNumber("bias", event.target.value)} />
          </label>
        </div>
        <label className="field">
          <span>Activation</span>
          <select aria-label="Activation" value={state.activation} onChange={(event) => setActivation(event.target.value as MicroscopeActivation)}>
            <option value="linear">Identity / linear</option>
            <option value="sigmoid">Sigmoid</option>
            <option value="tanh">Tanh</option>
            <option value="relu">ReLU</option>
          </select>
        </label>
        <label className="field">
          <span>Learning rate</span>
          <input aria-label="Learning rate" type="number" min="0.0001" step="0.01" value={state.learningRate} onChange={(event) => setNumber("learningRate", event.target.value)} />
        </label>

        <div className="metric">
          <span>Current prediction</span>
          <strong>{formatNumber(trace.prediction)}</strong>
        </div>
        <div className="metric">
          <span>Current loss</span>
          <strong>{formatNumber(trace.loss)}</strong>
        </div>
        <div className="gradients">
          <span>Proposed gradients</span>
          <code>dL/dw = {formatNumber(trace.gradientWeight)}</code>
          <code>dL/db = {formatNumber(trace.gradientBias)}</code>
        </div>
      </aside>
    </main>
  );
}
