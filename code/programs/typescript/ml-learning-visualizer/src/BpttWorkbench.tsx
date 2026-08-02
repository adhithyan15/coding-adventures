import { useMemo, useState } from "react";
import {
  DEFAULT_RECURRENT_PARAMETERS,
  traceRecurrentBptt,
} from "./recurrent-unroll-lab.js";

interface BpttWorkbenchProps {
  onShowForward: () => void;
}

function formatNumber(value: number): string {
  if (Math.abs(value) < 1e-12) {
    return "0";
  }
  if (Math.abs(value) < 1e-6) {
    return value.toExponential(2);
  }
  return Number(value.toFixed(6)).toString();
}

export function BpttWorkbench({ onShowForward }: BpttWorkbenchProps) {
  const trace = useMemo(() => traceRecurrentBptt(), []);
  const [selectedTime, setSelectedTime] = useState(2);
  const selected = trace.backwardSteps.find((step) => step.time === selectedTime)!;
  const chronologicalSteps = [...trace.backwardSteps].reverse();

  return (
    <main className="workspace workspace--bptt">
      <section className="bptt-stage" aria-label="Backpropagation through time trace">
        <div className="bptt-intro">
          <div>
            <p className="eyebrow">NN10 · sequence gradients</p>
            <h2>Backpropagation-through-time microscope</h2>
            <p>
              Keep the three saved forward states, then reverse every arrow.
              Watch later evidence reach earlier cells and add into one shared gradient.
            </p>
          </div>
          <div className="bptt-loss-chip">
            <small>final-state loss</small>
            <strong>{formatNumber(trace.loss)}</strong>
          </div>
        </div>

        <section className="bptt-panel" aria-label="Forward states and backward gradient lane">
          <div className="bptt-panel-heading">
            <div>
              <p className="eyebrow">Forward saved · backward reversed</p>
              <h2>One chain, two directions</h2>
            </div>
            <code>target = {formatNumber(trace.target)}</code>
          </div>

          <div className="bptt-forward-lane" aria-label="Saved forward states">
            <div><small>initial</small><strong>h[-1] = 0</strong></div>
            {trace.forward.steps.map((step) => (
              <div key={step.time}>
                <small>a[{step.time}] = {formatNumber(step.preactivation)}</small>
                <strong>h[{step.time}] = {formatNumber(step.state)}</strong>
              </div>
            ))}
            <div className="bptt-forward-lane__loss">
              <small>half-squared</small>
              <strong>L = {formatNumber(trace.loss)}</strong>
            </div>
          </div>

          <div className="bptt-direction-label">
            <span aria-hidden="true">←</span>
            backward pass runs from t = 2 to t = 0
          </div>
          <div className="bptt-backward-lane" aria-label="Reverse-time gradient steps">
            {trace.backwardSteps.map((step) => (
              <button
                aria-label={`Select backward step ${step.time}`}
                aria-pressed={selectedTime === step.time}
                className={selectedTime === step.time
                  ? "bptt-step bptt-step--active"
                  : "bptt-step"}
                key={step.time}
                type="button"
                onClick={() => setSelectedTime(step.time)}
              >
                <small>reverse t = {step.time}</small>
                <strong>dL/dh = {formatNumber(step.stateGradient)}</strong>
                <span>dL/da = {formatNumber(step.preactivationGradient)}</span>
              </button>
            ))}
          </div>

          <div className="bptt-arithmetic" aria-label="Selected backward arithmetic">
            <div className="bptt-arithmetic-heading">
              <div>
                <p className="eyebrow">Selected · reverse step {selectedTime}</p>
                <h3>Combine incoming gradient before differentiating</h3>
              </div>
              <code>ReLU&apos; = {formatNumber(selected.reluDerivative)}</code>
            </div>
            <div className="bptt-equation">
              <div><small>direct loss</small><strong>{formatNumber(selected.directStateGradient)}</strong></div>
              <span>+</span>
              <div><small>from future</small><strong>{formatNumber(selected.futureStateGradient)}</strong></div>
              <span>=</span>
              <div><small>dL/dh[{selectedTime}]</small><strong>{formatNumber(selected.stateGradient)}</strong></div>
              <span>×</span>
              <div><small>ReLU derivative</small><strong>{formatNumber(selected.reluDerivative)}</strong></div>
              <span>=</span>
              <div className="bptt-equation__result"><small>dL/da[{selectedTime}]</small><strong>{formatNumber(selected.preactivationGradient)}</strong></div>
            </div>
            <div className="bptt-local-gradients">
              <code>ΔW_x = {formatNumber(selected.parameterContributions.inputWeight)}</code>
              <code>ΔW_h = {formatNumber(selected.parameterContributions.recurrentWeight)}</code>
              <code>Δb = {formatNumber(selected.parameterContributions.bias)}</code>
              <code>to h[{selectedTime - 1}] = {formatNumber(selected.previousStateGradient)}</code>
            </div>
          </div>
        </section>

        <section className="bptt-panel" aria-label="Shared gradient reduction">
          <div className="bptt-panel-heading">
            <div>
              <p className="eyebrow">Three executions · one parameter set</p>
              <h2>Shared gradients add; they do not overwrite</h2>
            </div>
            <strong className="bptt-pass">ACCUMULATE</strong>
          </div>
          <div className="bptt-table-wrap">
            <table className="bptt-table">
              <caption>Per-time-step parameter contributions and their totals</caption>
              <thead><tr><th scope="col">gradient</th>{chronologicalSteps.map((step) => <th scope="col" key={step.time}>t = {step.time}</th>)}<th scope="col">total</th></tr></thead>
              <tbody>
                <tr><th scope="row">dL/dW_x</th>{chronologicalSteps.map((step) => <td key={step.time}>{formatNumber(step.parameterContributions.inputWeight)}</td>)}<td><strong>{formatNumber(trace.gradientTotals.inputWeight)}</strong></td></tr>
                <tr><th scope="row">dL/dW_h</th>{chronologicalSteps.map((step) => <td key={step.time}>{formatNumber(step.parameterContributions.recurrentWeight)}</td>)}<td><strong>{formatNumber(trace.gradientTotals.recurrentWeight)}</strong></td></tr>
                <tr><th scope="row">dL/db</th>{chronologicalSteps.map((step) => <td key={step.time}>{formatNumber(step.parameterContributions.bias)}</td>)}<td><strong>{formatNumber(trace.gradientTotals.bias)}</strong></td></tr>
              </tbody>
            </table>
          </div>
          <p className="bptt-initial-gradient">
            The reverse chain continues into the explicit initial state:
            <strong> dL/dh[-1] = {formatNumber(trace.gradientTotals.initialState)}</strong>
          </p>
        </section>

        <section className="bptt-audit-grid" aria-label="Gradient audit and update preview">
          <div className="bptt-panel">
            <div className="bptt-panel-heading">
              <div><p className="eyebrow">Independent oracle</p><h2>Finite-difference gradient check</h2></div>
              <strong className="bptt-pass">PASS</strong>
            </div>
            <div className="bptt-table-wrap">
              <table className="bptt-table">
                <caption>Analytical and numerical gradient agreement</caption>
                <thead><tr><th scope="col">parameter</th><th scope="col">BPTT</th><th scope="col">numerical</th><th scope="col">error</th></tr></thead>
                <tbody>
                  {(["inputWeight", "recurrentWeight", "bias"] as const).map((parameter) => (
                    <tr key={parameter}>
                      <th scope="row">{parameter === "inputWeight" ? "W_x" : parameter === "recurrentWeight" ? "W_h" : "b"}</th>
                      <td>{formatNumber(trace.gradientTotals[parameter])}</td>
                      <td>{formatNumber(trace.numericalGradients[parameter])}</td>
                      <td>{formatNumber(trace.gradientErrors[parameter])}</td>
                    </tr>
                  ))}
                </tbody>
              </table>
            </div>
          </div>
          <div className="bptt-panel bptt-update-panel">
            <p className="eyebrow">One step · learning rate 0.1</p>
            <h2>Move against the accumulated gradient</h2>
            <div className="bptt-loss-change">
              <div><small>before loss</small><strong>{formatNumber(trace.loss)}</strong></div>
              <span aria-hidden="true">→</span>
              <div><small>after loss</small><strong>{formatNumber(trace.update.loss)}</strong></div>
            </div>
            <div className="bptt-parameter-update">
              <code>W_x: {formatNumber(DEFAULT_RECURRENT_PARAMETERS.inputWeight)} → {formatNumber(trace.update.parameters.inputWeight)}</code>
              <code>W_h: {formatNumber(DEFAULT_RECURRENT_PARAMETERS.recurrentWeight)} → {formatNumber(trace.update.parameters.recurrentWeight)}</code>
              <code>b: {formatNumber(DEFAULT_RECURRENT_PARAMETERS.bias)} → {formatNumber(trace.update.parameters.bias)}</code>
            </div>
            <p>Updated states = [{trace.update.states.map(formatNumber).join(", ")}]</p>
          </div>
        </section>
      </section>

      <aside className="recurrent-controls bptt-controls" aria-label="BPTT microscope controls">
        <p className="eyebrow">Forward and backward belong together</p>
        <h2>Reverse the unroll</h2>
        <p>
          Select a reverse-time cell. Its future gradient was produced by the
          cell immediately to its right in the forward graph.
        </p>
        <button className="bptt-view-button" type="button" onClick={onShowForward}>
          Show forward unroll
        </button>
        <div className="recurrent-selected-summary">
          <small>selected reverse step</small>
          <strong>t = {selectedTime}</strong>
          <span>
            {formatNumber(selected.directStateGradient)} direct + {formatNumber(selected.futureStateGradient)} from the future.
          </span>
        </div>
        <div className="recurrent-note">
          <span>What scales next?</span>
          <p>
            Vectors use the same reverse walk with matrix products. GRUs and
            LSTMs add gates, while truncated BPTT limits how far this lane runs.
          </p>
        </div>
      </aside>
    </main>
  );
}
