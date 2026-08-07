import { useMemo, useState } from "react";
import {
  GRADIENT_ACCUMULATION_SCENARIOS,
  traceGradientAccumulation,
  type GradientAccumulationScenarioId,
  type GradientBufferTrace,
} from "./gradient-accumulation-lab.js";

function formatNumber(value: number, digits = 6): string {
  if (Math.abs(value) < 1e-12) return "0";
  if (Math.abs(value) < 0.0001 || Math.abs(value) >= 1000) return value.toExponential(3);
  return Number(value.toFixed(digits)).toString();
}

function eventTitle(step: GradientBufferTrace): string {
  if (step.kind === "backward") return `backward(${step.sampleId})`;
  if (step.kind === "zero_grad") return "zero_grad()";
  return `step(grad / ${step.divisor})`;
}

export function GradientAccumulationWorkbench() {
  const [scenarioId, setScenarioId] = useState<GradientAccumulationScenarioId>(
    "accumulate_two_calls",
  );
  const [selectedIndex, setSelectedIndex] = useState(0);
  const trace = useMemo(() => traceGradientAccumulation(scenarioId), [scenarioId]);
  const selected = trace.steps[Math.min(selectedIndex, trace.steps.length - 1)]!;

  function chooseScenario(id: GradientAccumulationScenarioId): void {
    setScenarioId(id);
    setSelectedIndex(0);
  }

  return (
    <main className="workspace workspace--gradient-buffer">
      <section className="gradient-buffer-stage" aria-label="Gradient accumulation and zeroing visualizer">
        <section className="gradient-buffer-intro">
          <div>
            <p className="eyebrow">NN28 / tensor and autograd bridge</p>
            <h2>Gradient buffer timeline</h2>
            <p>Backward adds into a persistent buffer. An optimizer reads it, but only an explicit zero clears it.</p>
          </div>
          <div className="gradient-buffer-chip">w.grad += local</div>
        </section>

        <section className="gradient-buffer-state" aria-label="Selected gradient buffer state">
          <div className="panel-heading">
            <div>
              <p className="eyebrow">The two pieces of mutable state</p>
              <h2>Parameter and gradient buffer</h2>
            </div>
            <span>event {selected.index + 1} of {trace.steps.length}</span>
          </div>
          <div className="gradient-buffer-vessels">
            <div>
              <small>parameter w</small>
              <code>{formatNumber(selected.parameterBefore)}</code>
              <span>→</span>
              <strong>{formatNumber(selected.parameterAfter)}</strong>
            </div>
            <div className={selected.bufferAfter === 0 ? "is-empty" : "is-filled"}>
              <small>persistent w.grad</small>
              <code>{formatNumber(selected.bufferBefore)}</code>
              <span>→</span>
              <strong>{formatNumber(selected.bufferAfter)}</strong>
            </div>
          </div>
        </section>

        <section className="gradient-buffer-timeline" aria-label="Gradient schedule timeline">
          <div className="panel-heading">
            <div>
              <p className="eyebrow">Replay the schedule</p>
              <h2>Every API call is a state transition</h2>
            </div>
            <span>{trace.backwardCalls} backward / {trace.optimizerSteps} step / {trace.zeroCalls} zero</span>
          </div>
          <div className="gradient-buffer-event-lane">
            {trace.steps.map((step) => (
              <button
                aria-label={`Open event ${step.index + 1}, ${eventTitle(step)}, buffer ${formatNumber(step.bufferBefore)} to ${formatNumber(step.bufferAfter)}`}
                aria-pressed={step.index === selected.index}
                key={step.index}
                type="button"
                onClick={() => setSelectedIndex(step.index)}
              >
                <small>event {step.index + 1}</small>
                <strong>{eventTitle(step)}</strong>
                <code>grad {formatNumber(step.bufferBefore)} → {formatNumber(step.bufferAfter)}</code>
              </button>
            ))}
          </div>
        </section>

        <section className="gradient-buffer-equation" aria-label="Selected gradient buffer calculation">
          <div className="panel-heading">
            <div>
              <p className="eyebrow">Open the arithmetic</p>
              <h2>{eventTitle(selected)}</h2>
            </div>
            <span>{selected.kind.replace("_", " ")}</span>
          </div>
          {selected.kind === "backward" ? (
            <div className="gradient-buffer-backward-grid">
              <div>
                <small>forward sample {selected.sampleId}</small>
                <code>{formatNumber(selected.parameterBefore)} × {formatNumber(selected.input)} = {formatNumber(selected.prediction)}</code>
                <code>{formatNumber(selected.prediction)} - {formatNumber(selected.target)} = {formatNumber(selected.residual)}</code>
                <strong>½ × {formatNumber(selected.residual)}² = {formatNumber(selected.loss)}</strong>
              </div>
              <div>
                <small>local gradient</small>
                <code>({formatNumber(selected.prediction)} - {formatNumber(selected.target)}) × {formatNumber(selected.input)}</code>
                <strong>dL/dw = {formatNumber(selected.localGradient)}</strong>
              </div>
              <div className="gradient-buffer-addition">
                <small>buffer addition</small>
                <code>{formatNumber(selected.bufferBefore)} + {formatNumber(selected.localGradient)}</code>
                <strong>w.grad = {formatNumber(selected.bufferAfter)}</strong>
              </div>
            </div>
          ) : selected.kind === "zero_grad" ? (
            <div className="gradient-buffer-zero-rule">
              <code>w.grad ← 0</code>
              <p>The parameter stays {formatNumber(selected.parameterAfter)}. Only the buffer is cleared.</p>
            </div>
          ) : (
            <div className="gradient-buffer-step-rule">
              <div>
                <small>choose sum or mean</small>
                <code>{formatNumber(selected.bufferBefore)} / {selected.divisor} = {formatNumber(selected.appliedGradient)}</code>
              </div>
              <div>
                <small>SGD update</small>
                <code>{formatNumber(selected.parameterBefore)} - {formatNumber(trace.scenario.learningRate)} × {formatNumber(selected.appliedGradient)}</code>
                <strong>w = {formatNumber(selected.parameterAfter)}</strong>
              </div>
              <p>The optimizer read {formatNumber(selected.bufferBefore)} but left that buffer unchanged.</p>
            </div>
          )}
        </section>

        <section className="gradient-buffer-audit" aria-label="Gradient buffer numerical audit">
          <div className="panel-heading">
            <div>
              <p className="eyebrow">Independent check</p>
              <h2>Each local gradient gets fresh forward passes</h2>
            </div>
            <span>epsilon 1e-5</span>
          </div>
          <div className="gradient-buffer-audit-grid">
            {trace.steps.filter((step) => step.kind === "backward").map((step) => {
              if (step.kind !== "backward") return null;
              return (
                <div key={step.index}>
                  <strong>event {step.index + 1} / {step.sampleId}</strong>
                  <span>analytical <code>{formatNumber(step.localGradient)}</code></span>
                  <span>numerical <code>{formatNumber(step.numericalGradient)}</code></span>
                  <small>error {formatNumber(step.gradientAbsoluteError)}</small>
                </div>
              );
            })}
            <div className="gradient-buffer-audit-max">
              <strong>maximum error</strong>
              <code>{formatNumber(trace.maxGradientAbsoluteError)}</code>
              <small>must stay below 1e-8</small>
            </div>
          </div>
        </section>
      </section>

      <aside className="controls gradient-buffer-controls" aria-label="Gradient buffer scenarios">
        <p className="eyebrow">Schedule presets</p>
        <h2>Move the zero call</h2>
        <div className="gradient-buffer-scenario-buttons">
          {GRADIENT_ACCUMULATION_SCENARIOS.map((scenario) => (
            <button
              aria-pressed={scenario.id === scenarioId}
              key={scenario.id}
              type="button"
              onClick={() => chooseScenario(scenario.id)}
            >
              <strong>{scenario.title}</strong>
              <span>{scenario.summary}</span>
            </button>
          ))}
        </div>
        <div className="gradient-buffer-summary">
          <p className="eyebrow">Final state</p>
          <code>w = {formatNumber(trace.finalParameter)}</code>
          <code>w.grad = {formatNumber(trace.finalGradientBuffer)}</code>
        </div>
        <div className="gradient-buffer-mental-model">
          <p className="eyebrow">Keep this picture</p>
          <h2>Backward adds. Step reads. Zero clears.</h2>
          <p>Accumulation is useful across micro-batches and dangerous across optimizer steps unless the schedule is deliberate.</p>
        </div>
      </aside>
    </main>
  );
}
