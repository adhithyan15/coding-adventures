import { Fragment, useMemo, useState } from "react";
import {
  DEFAULT_RECURRENT_INITIAL_STATE,
  DEFAULT_RECURRENT_INPUTS,
  DEFAULT_RECURRENT_PARAMETERS,
  traceRecurrentUnroll,
} from "./recurrent-unroll-lab.js";

function formatNumber(value: number): string {
  if (Math.abs(value) < 1e-12) {
    return "0";
  }
  return Number(value.toFixed(4)).toString();
}

export function RecurrentWorkbench() {
  const [selectedTime, setSelectedTime] = useState(0);
  const [memoryEnabled, setMemoryEnabled] = useState(true);
  const withMemory = useMemo(() => traceRecurrentUnroll(), []);
  const withoutMemory = useMemo(
    () => traceRecurrentUnroll(
      DEFAULT_RECURRENT_INPUTS,
      DEFAULT_RECURRENT_INITIAL_STATE,
      DEFAULT_RECURRENT_PARAMETERS,
      false,
    ),
    [],
  );
  const displayed = memoryEnabled ? withMemory : withoutMemory;
  const selected = displayed.steps[selectedTime]!;

  return (
    <main className="workspace workspace--recurrent">
      <section className="recurrent-stage" aria-label="Three-step recurrent state trace">
        <div className="recurrent-intro">
          <div>
            <p className="eyebrow">NN09 · sequence networks</p>
            <h2>Recurrent-state unroller</h2>
            <p>
              Run one scalar cell three times. Each result becomes part of the
              next input while one parameter set stays shared across time.
            </p>
          </div>
          <div className="recurrent-sequence-chip">x = [1, 2, 0]</div>
        </div>

        <section className="recurrent-unroll-panel" aria-label="Recurrent cell unroll">
          <div className="recurrent-panel-heading">
            <div>
              <p className="eyebrow">One cell · three executions</p>
              <h2>Follow the state from left to right</h2>
            </div>
            <div className="recurrent-final-state">
              <small>final state</small>
              <strong>{formatNumber(displayed.finalState)}</strong>
            </div>
          </div>

          <div className="shared-parameter-strip" aria-label="Parameters shared by every time step">
            <span>shared at t=0, 1, 2</span>
            <code>Wₓ = {formatNumber(DEFAULT_RECURRENT_PARAMETERS.inputWeight)}</code>
            <code>Wₕ = {formatNumber(DEFAULT_RECURRENT_PARAMETERS.recurrentWeight)}</code>
            <code>b = {formatNumber(DEFAULT_RECURRENT_PARAMETERS.bias)}</code>
          </div>

          <div className="recurrent-chain" aria-label="Unrolled recurrent state chain">
            <div className="recurrent-initial-node">
              <small>initial</small>
              <strong>h[-1]</strong>
              <code>{formatNumber(DEFAULT_RECURRENT_INITIAL_STATE)}</code>
            </div>
            {displayed.steps.map((step) => (
              <Fragment key={step.time}>
                <div className={memoryEnabled
                  ? "recurrent-connector"
                  : "recurrent-connector recurrent-connector--disabled"}
                >
                  <small>{memoryEnabled ? "carry h" : "cut"}</small>
                  <span aria-hidden="true">→</span>
                </div>
                <button
                  aria-label={`Select recurrent step ${step.time}`}
                  aria-pressed={selectedTime === step.time}
                  className={selectedTime === step.time
                    ? "recurrent-cell recurrent-cell--active"
                    : "recurrent-cell"}
                  type="button"
                  onClick={() => setSelectedTime(step.time)}
                >
                  <small>time {step.time}</small>
                  <span>x[{step.time}] = {formatNumber(step.input)}</span>
                  <strong>h[{step.time}] = {formatNumber(step.state)}</strong>
                </button>
              </Fragment>
            ))}
          </div>

          <div className="recurrent-arithmetic" aria-label="Selected recurrent arithmetic">
            <div className="recurrent-arithmetic-heading">
              <div>
                <p className="eyebrow">Selected · time {selectedTime}</p>
                <h3>Open this cell</h3>
              </div>
              <code>h[{selectedTime - 1}] → h[{selectedTime}]</code>
            </div>
            <div className="recurrent-equation">
              <div>
                <small>new input</small>
                <strong>
                  2 × {formatNumber(selected.input)} = {formatNumber(selected.inputProduct)}
                </strong>
              </div>
              <span>+</span>
              <div className={memoryEnabled ? "" : "equation-term--disabled"}>
                <small>carried state</small>
                <strong>
                  0.5 × {formatNumber(selected.previousState)} = {formatNumber(selected.recurrentProduct)}
                </strong>
              </div>
              <span>+</span>
              <div>
                <small>bias</small>
                <strong>{formatNumber(selected.bias)}</strong>
              </div>
              <span>=</span>
              <div>
                <small>preactivation</small>
                <strong>{formatNumber(selected.preactivation)}</strong>
              </div>
              <span>→</span>
              <div className="recurrent-equation__state">
                <small>ReLU state</small>
                <strong>{formatNumber(selected.state)}</strong>
              </div>
            </div>
          </div>
        </section>

        <section className="memory-ablation-panel" aria-label="Recurrent memory ablation">
          <div className="recurrent-panel-heading">
            <div>
              <p className="eyebrow">Same inputs · memory removed</p>
              <h2>What came through the recurrent link?</h2>
            </div>
            <p>The final zero input remembers earlier steps only when the link is present.</p>
          </div>
          <div className="recurrent-table-wrap">
            <table className="recurrent-table">
              <caption>State comparison with and without recurrence</caption>
              <thead>
                <tr>
                  <th scope="col">time</th>
                  <th scope="col">input</th>
                  <th scope="col">with memory</th>
                  <th scope="col">without memory</th>
                  <th scope="col">difference</th>
                </tr>
              </thead>
              <tbody>
                {withMemory.steps.map((step, time) => {
                  const ablatedState = withoutMemory.states[time]!;
                  return (
                    <tr className={selectedTime === time ? "recurrent-table-row--active" : ""} key={time}>
                      <th scope="row">{time}</th>
                      <td>{formatNumber(step.input)}</td>
                      <td>{formatNumber(step.state)}</td>
                      <td>{formatNumber(ablatedState)}</td>
                      <td>{formatNumber(step.state - ablatedState)}</td>
                    </tr>
                  );
                })}
              </tbody>
            </table>
          </div>
        </section>
      </section>

      <aside className="recurrent-controls" aria-label="Recurrent unroll controls">
        <p className="eyebrow">One honest experiment</p>
        <h2>Memory control</h2>
        <p>
          Select a time-step cell, then cut the recurrent link without changing
          its inputs, weights, or bias.
        </p>

        <label className="recurrent-memory-control">
          <input
            type="checkbox"
            checked={memoryEnabled}
            onChange={(event) => setMemoryEnabled(event.target.checked)}
          />
          <span>
            <strong>Carry the previous state</strong>
            <small>Use Wₕ × h[t - 1] at every step.</small>
          </span>
        </label>

        <div className="recurrent-selected-summary">
          <small>selected time</small>
          <strong>t = {selectedTime}</strong>
          <span>
            {memoryEnabled
              ? `${formatNumber(selected.recurrentProduct)} enters through memory.`
              : "The recurrent contribution is forced to zero."}
          </span>
        </div>

        <div className="recurrent-note">
          <span>What scales next?</span>
          <p>
            Vector states repeat this same pattern across several coordinates.
            Backpropagation will reverse the unrolled arrows and add gradient
            contributions into the shared parameters.
          </p>
        </div>
      </aside>
    </main>
  );
}
