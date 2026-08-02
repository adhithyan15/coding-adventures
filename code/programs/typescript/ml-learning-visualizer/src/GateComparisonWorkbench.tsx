import { useMemo, useState } from "react";
import {
  type GatedCellGate,
  type GatedModel,
  traceGateCounterfactual,
  traceGatedRecurrent,
} from "./gated-recurrent-lab.js";

interface GateComparisonWorkbenchProps {
  onShowBackward: () => void;
}

type Intervention = "canonical" | 0 | 1;

function formatNumber(value: number): string {
  if (Math.abs(value) < 1e-12) {
    return "0";
  }
  return Number(value.toFixed(6)).toString();
}

function canonicalGateValue(model: GatedModel, selectedGate: GatedCellGate): number {
  if (model === "gru") {
    return selectedGate === "reset" ? 0.5 : 0.25;
  }
  if (selectedGate === "forget") {
    return 0.5;
  }
  return selectedGate === "input" ? 0.25 : 0.75;
}

export function GateComparisonWorkbench({
  onShowBackward,
}: GateComparisonWorkbenchProps) {
  const trace = useMemo(() => traceGatedRecurrent(), []);
  const [model, setModel] = useState<GatedModel>("gru");
  const [selectedGate, setSelectedGate] = useState<GatedCellGate>("update");
  const [intervention, setIntervention] = useState<Intervention>("canonical");
  const gateValue = intervention === "canonical"
    ? canonicalGateValue(model, selectedGate)
    : intervention;
  const counterfactual = traceGateCounterfactual(
    model,
    selectedGate,
    gateValue,
    trace,
  );

  const selectGate = (nextModel: GatedModel, gate: GatedCellGate) => {
    setModel(nextModel);
    setSelectedGate(gate);
    setIntervention("canonical");
  };

  const gruReset = model === "gru" && selectedGate === "reset"
    ? gateValue
    : trace.gru.resetGate.value;
  const gruUpdate = model === "gru" && selectedGate === "update"
    ? gateValue
    : trace.gru.updateGate.value;
  const gruCandidate = model === "gru" ? counterfactual.candidate : trace.gru.candidate.value;
  const gruRetained = (1 - gruUpdate) * trace.previousHidden;
  const gruWrite = gruUpdate * gruCandidate;
  const gruHidden = gruRetained + gruWrite;

  const lstmForget = model === "lstm" && selectedGate === "forget"
    ? gateValue
    : trace.lstm.forgetGate.value;
  const lstmInput = model === "lstm" && selectedGate === "input"
    ? gateValue
    : trace.lstm.inputGate.value;
  const lstmOutput = model === "lstm" && selectedGate === "output"
    ? gateValue
    : trace.lstm.outputGate.value;
  const lstmRetained = lstmForget * trace.previousCell;
  const lstmWrite = lstmInput * trace.lstm.candidate.value;
  const lstmCell = lstmRetained + lstmWrite;
  const lstmHidden = lstmOutput * Math.tanh(lstmCell);

  return (
    <main className="workspace workspace--gates">
      <section className="gate-stage" aria-label="GRU and LSTM gate comparison">
        <div className="gate-intro">
          <div>
            <p className="eyebrow">NN11 · gated sequence memory</p>
            <h2>GRU and LSTM gate comparator</h2>
            <p>
              Route the same previous memory and candidate through both cells.
              Change one gate while every other signal stays fixed.
            </p>
          </div>
          <div className="gate-input-chip">
            <small>shared input</small>
            <strong>x = 1 · h = 0.8</strong>
          </div>
        </div>

        <section className="gate-comparison-panel" aria-label="Aligned gated memory lanes">
          <div className="gate-panel-heading">
            <div>
              <p className="eyebrow">Same evidence · different state design</p>
              <h2>Follow what each gate lets through</h2>
            </div>
            <code>candidate = 0.6</code>
          </div>

          <article className="gate-model-lane gate-model-lane--gru" aria-label="GRU memory lane">
            <div className="gate-model-label">
              <span>GRU</span>
              <strong>one stored and exposed state</strong>
            </div>
            <div className="gate-flow">
              <div className="gate-state-node">
                <small>previous state</small>
                <strong>h = {formatNumber(trace.previousHidden)}</strong>
              </div>
              <button
                aria-label="Select GRU reset gate"
                aria-pressed={model === "gru" && selectedGate === "reset"}
                className={model === "gru" && selectedGate === "reset"
                  ? "gate-node gate-node--active"
                  : "gate-node"}
                type="button"
                onClick={() => selectGate("gru", "reset")}
              >
                <small>reset r</small>
                <strong>{formatNumber(gruReset)}</strong>
                <span>candidate sees {formatNumber(gruReset * trace.previousHidden)}</span>
              </button>
              <div className="gate-candidate-node">
                <small>candidate n</small>
                <strong>{formatNumber(gruCandidate)}</strong>
              </div>
              <button
                aria-label="Select GRU update gate"
                aria-pressed={model === "gru" && selectedGate === "update"}
                className={model === "gru" && selectedGate === "update"
                  ? "gate-node gate-node--active"
                  : "gate-node"}
                type="button"
                onClick={() => selectGate("gru", "update")}
              >
                <small>update z</small>
                <strong>{formatNumber(gruUpdate)}</strong>
                <span>new share</span>
              </button>
              <div className="gate-result-node">
                <small>next hidden</small>
                <strong>h = {formatNumber(gruHidden)}</strong>
                <span>{formatNumber(gruRetained)} old + {formatNumber(gruWrite)} new</span>
              </div>
            </div>
          </article>

          <article className="gate-model-lane gate-model-lane--lstm" aria-label="LSTM memory lane">
            <div className="gate-model-label">
              <span>LSTM</span>
              <strong>private cell plus exposed hidden state</strong>
            </div>
            <div className="gate-flow gate-flow--lstm">
              <div className="gate-state-node">
                <small>previous cell</small>
                <strong>c = {formatNumber(trace.previousCell)}</strong>
              </div>
              <button
                aria-label="Select LSTM forget gate"
                aria-pressed={model === "lstm" && selectedGate === "forget"}
                className={model === "lstm" && selectedGate === "forget"
                  ? "gate-node gate-node--active"
                  : "gate-node"}
                type="button"
                onClick={() => selectGate("lstm", "forget")}
              >
                <small>forget f</small>
                <strong>{formatNumber(lstmForget)}</strong>
                <span>old share</span>
              </button>
              <button
                aria-label="Select LSTM input gate"
                aria-pressed={model === "lstm" && selectedGate === "input"}
                className={model === "lstm" && selectedGate === "input"
                  ? "gate-node gate-node--active"
                  : "gate-node"}
                type="button"
                onClick={() => selectGate("lstm", "input")}
              >
                <small>input i</small>
                <strong>{formatNumber(lstmInput)}</strong>
                <span>candidate share</span>
              </button>
              <div className="gate-cell-node">
                <small>private cell</small>
                <strong>c = {formatNumber(lstmCell)}</strong>
                <span>{formatNumber(lstmRetained)} old + {formatNumber(lstmWrite)} new</span>
              </div>
              <button
                aria-label="Select LSTM output gate"
                aria-pressed={model === "lstm" && selectedGate === "output"}
                className={model === "lstm" && selectedGate === "output"
                  ? "gate-node gate-node--active"
                  : "gate-node"}
                type="button"
                onClick={() => selectGate("lstm", "output")}
              >
                <small>output o</small>
                <strong>{formatNumber(lstmOutput)}</strong>
                <span>visible share</span>
              </button>
              <div className="gate-result-node">
                <small>next hidden</small>
                <strong>h = {formatNumber(lstmHidden)}</strong>
                <span>o × tanh(c)</span>
              </div>
            </div>
          </article>
        </section>

        <section className="gate-comparison-panel" aria-label="Gate responsibility comparison">
          <div className="gate-panel-heading">
            <div>
              <p className="eyebrow">Architecture, not acronym memorization</p>
              <h2>Which signal does each gate control?</h2>
            </div>
          </div>
          <div className="gate-table-wrap">
            <table className="gate-table">
              <caption>GRU and LSTM state-routing responsibilities</caption>
              <thead><tr><th scope="col">Responsibility</th><th scope="col">GRU</th><th scope="col">LSTM</th></tr></thead>
              <tbody>
                <tr><th scope="row">Build candidate</th><td>reset gate</td><td>candidate tanh path</td></tr>
                <tr><th scope="row">Retain old memory</th><td rowSpan={2}>update gate mixes both</td><td>forget gate</td></tr>
                <tr><th scope="row">Write new memory</th><td>input gate</td></tr>
                <tr><th scope="row">Expose memory</th><td>same hidden state</td><td>output gate</td></tr>
                <tr><th scope="row">State buffers</th><td>h = {formatNumber(gruHidden)}</td><td>c = {formatNumber(lstmCell)}, h = {formatNumber(lstmHidden)}</td></tr>
              </tbody>
            </table>
          </div>
        </section>
      </section>

      <aside className="gate-controls" aria-label="Gate intervention controls">
        <p className="eyebrow">One controlled intervention</p>
        <h2>{model.toUpperCase()} {selectedGate} gate</h2>
        <p>
          Keep every other gate fixed. Use the learned canonical value or force
          this one valve fully closed or open.
        </p>
        <div className="gate-intervention-buttons" aria-label="Selected gate value">
          <button aria-pressed={intervention === "canonical"} type="button" onClick={() => setIntervention("canonical")}>Canonical</button>
          <button aria-pressed={intervention === 0} type="button" onClick={() => setIntervention(0)}>Force 0</button>
          <button aria-pressed={intervention === 1} type="button" onClick={() => setIntervention(1)}>Force 1</button>
        </div>
        <div className="gate-selected-summary" aria-label="Selected gate effect">
          <small>selected value</small>
          <strong>{formatNumber(gateValue)}</strong>
          <span>
            {model === "gru"
              ? `candidate ${formatNumber(gruCandidate)} · next h ${formatNumber(gruHidden)}`
              : `next c ${formatNumber(lstmCell)} · visible h ${formatNumber(lstmHidden)}`}
          </span>
        </div>
        <button className="bptt-view-button" type="button" onClick={onShowBackward}>
          Return to BPTT gradients
        </button>
        <div className="recurrent-note">
          <span>What scales next?</span>
          <p>
            Vector cells pack each gate&apos;s affine projection into matrices.
            The scalar routing stays identical at every coordinate.
          </p>
        </div>
      </aside>
    </main>
  );
}
