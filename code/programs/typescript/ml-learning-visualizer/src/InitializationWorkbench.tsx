import { useMemo, useState } from "react";
import {
  INITIALIZATION_KINDS,
  traceInitializationDistributions,
  type DistributionActivation,
  type InitializationKind,
} from "./initialization-distribution-lab.js";

const INITIALIZERS: readonly { kind: InitializationKind; label: string; summary: string }[] = [
  { kind: "tiny", label: "Tiny", summary: "fixed scale 0.1" },
  { kind: "xavier", label: "Xavier", summary: "sqrt(1 / fan-in)" },
  { kind: "he", label: "He", summary: "sqrt(2 / fan-in)" },
  { kind: "large", label: "Large", summary: "fixed scale 2" },
];

function formatNumber(value: number, digits = 6): string {
  if (Math.abs(value) < 1e-12) return "0";
  if (Math.abs(value) < 0.0001 || Math.abs(value) >= 1000) return value.toExponential(3);
  return Number(value.toFixed(digits)).toString();
}

function distributionPosition(value: number, minimum: number, maximum: number): string {
  const span = Math.max(maximum - minimum, 1e-12);
  return `${((value - minimum) / span) * 100}%`;
}

export function InitializationWorkbench() {
  const [initializer, setInitializer] = useState<InitializationKind>("xavier");
  const [activation, setActivation] = useState<DistributionActivation>("tanh");
  const [selectedLayer, setSelectedLayer] = useState(0);
  const trace = useMemo(
    () => traceInitializationDistributions(initializer, activation),
    [activation, initializer],
  );
  const comparison = useMemo(
    () => INITIALIZATION_KINDS.map((kind) => traceInitializationDistributions(kind, activation)),
    [activation],
  );
  const layer = trace.layers[selectedLayer]!;
  const sampleInput = layer.inputs[0]!;
  const terms = sampleInput.map((value, index) => value * layer.weights[index]![0]!);
  const range = Math.max(
    ...trace.layers.flatMap((item) => item.activations.flat().map(Math.abs)),
    1,
  );
  const comparisonMax = Math.max(
    ...comparison.flatMap((item) => item.layers.map((entry) => entry.summary.standardDeviation)),
    1e-12,
  );

  return (
    <main className="workspace workspace--initialization">
      <section className="initialization-stage" aria-label="Initialization distribution explorer">
        <div className="lab-intro initialization-intro">
          <div>
            <p className="eyebrow">NN23 / same signs, different scale</p>
            <h2>Initialization and activation distributions</h2>
            <p>Follow four tiny inputs through three layers and see when signals shrink, spread, saturate, or explode.</p>
          </div>
          <div className="initialization-chip">{initializer} + {activation}</div>
        </div>

        <section className="initialization-flow" aria-label="Layer activation distributions">
          <div className="distribution-card distribution-card--input">
            <p className="eyebrow">Input batch</p>
            <strong>4 rows x 2 values</strong>
            <code>std {formatNumber(trace.inputSummary.standardDeviation)}</code>
          </div>
          {trace.layers.map((item, index) => {
            const values = item.activations.flat();
            return (
              <button
                aria-pressed={selectedLayer === index}
                className="distribution-card"
                key={item.layer}
                type="button"
                onClick={() => setSelectedLayer(index)}
              >
                <span className="eyebrow">Layer {item.layer}</span>
                <strong>std {formatNumber(item.summary.standardDeviation)}</strong>
                <span className="distribution-dot-plot" aria-hidden="true">
                  {values.map((value, valueIndex) => (
                    <i
                      key={valueIndex}
                      style={{ left: distributionPosition(value, -range, range) }}
                    />
                  ))}
                </span>
                <span>{formatNumber(item.summary.minimum)} to {formatNumber(item.summary.maximum)}</span>
              </button>
            );
          })}
        </section>

        <section className="distribution-summary-panel" aria-label="Selected activation distribution">
          <div className="panel-heading">
            <div>
              <p className="eyebrow">All eight activations</p>
              <h2>Layer {layer.layer} distribution</h2>
            </div>
            <span>scale {formatNumber(layer.scale)}</span>
          </div>
          <div className="distribution-stat-grid">
            <div><span>mean</span><strong>{formatNumber(layer.summary.mean)}</strong></div>
            <div><span>variance</span><strong>{formatNumber(layer.summary.variance)}</strong></div>
            <div><span>standard deviation</span><strong>{formatNumber(layer.summary.standardDeviation)}</strong></div>
            <div><span>{activation === "tanh" ? "saturated" : "exact zeros"}</span><strong>{formatNumber((activation === "tanh" ? layer.summary.saturatedFraction : layer.summary.zeroFraction) * 100, 3)}%</strong></div>
          </div>
          <div className="activation-value-grid">
            {layer.activations.flat().map((value, index) => (
              <code key={index}>{formatNumber(value)}</code>
            ))}
          </div>
        </section>

        <section className="initialization-arithmetic" aria-label="Selected layer hand calculation">
          <div className="panel-heading">
            <div>
              <p className="eyebrow">Sample 0 / neuron 0</p>
              <h2>Open one activation</h2>
            </div>
            <span>no bias in this controlled experiment</span>
          </div>
          <div className="initialization-equation">
            {terms.map((term, index) => (
              <code key={index}>{formatNumber(sampleInput[index]!)} x {formatNumber(layer.weights[index]![0]!)} = {formatNumber(term)}</code>
            ))}
            <strong>sum = {formatNumber(layer.preactivations[0]![0]!)}</strong>
            <strong>{activation} = {formatNumber(layer.activations[0]![0]!)}</strong>
          </div>
        </section>

        <section className="initializer-comparison" aria-label="Initializer comparison">
          <div className="panel-heading">
            <div>
              <p className="eyebrow">Same inputs / same signs / same activation</p>
              <h2>Compare signal spread</h2>
            </div>
            <span>bar length = layer standard deviation</span>
          </div>
          <div className="initializer-comparison-grid">
            {comparison.map((item) => (
              <article className={item.initializer === initializer ? "is-selected" : ""} key={item.initializer}>
                <strong>{item.initializer}</strong>
                {item.layers.map((entry) => (
                  <div className="spread-row" key={entry.layer}>
                    <span>L{entry.layer}</span>
                    <i style={{ width: `${(entry.summary.standardDeviation / comparisonMax) * 100}%` }} />
                    <code>{formatNumber(entry.summary.standardDeviation)}</code>
                  </div>
                ))}
              </article>
            ))}
          </div>
        </section>
      </section>

      <aside className="controls initialization-controls">
        <section>
          <p className="eyebrow">Weight scale</p>
          <h2>Choose an initializer</h2>
          <p>The sign template stays fixed so only the scaling rule changes.</p>
          <div className="initializer-buttons">
            {INITIALIZERS.map((item) => (
              <button
                aria-pressed={initializer === item.kind}
                key={item.kind}
                type="button"
                onClick={() => setInitializer(item.kind)}
              >
                <span>{item.label}</span>
                <small>{item.summary}</small>
              </button>
            ))}
          </div>
        </section>
        <section>
          <p className="eyebrow">Nonlinearity</p>
          <h2>Switch the activation</h2>
          <div className="activation-choice-grid">
            {(["tanh", "relu"] as const).map((kind) => (
              <button
                aria-pressed={activation === kind}
                key={kind}
                type="button"
                onClick={() => setActivation(kind)}
              >
                {kind === "tanh" ? "tanh" : "ReLU"}
              </button>
            ))}
          </div>
        </section>
        <section className="initialization-reading">
          <p className="eyebrow">What to notice</p>
          <h2>{initializer === "tiny" ? "Signal is fading" : initializer === "large" ? (activation === "tanh" ? "tanh is pinned near its limits" : "Signal is growing") : "Scale and activation cooperate"}</h2>
          <p>Real initializers draw random weights. NN23 fixes the signs so every language can reproduce the arithmetic exactly.</p>
        </section>
      </aside>
    </main>
  );
}
