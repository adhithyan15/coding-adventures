import { useMemo, useState } from "react";
import {
  probeWebGpuBackendParity,
  traceBackendParity,
  type AcceleratorProbeResult,
  type BackendEvidence,
  type BackendParityLaneId,
} from "./backend-parity-lab.js";

type ProbeState = AcceleratorProbeResult | { readonly status: "not-run" | "running"; readonly message: string };

function formatNumber(value: number): string {
  if (Math.abs(value) < 1e-12) return "0";
  if (Number.isInteger(value)) return String(value);
  return Number(value.toPrecision(9)).toString();
}

function evidenceLabel(evidence: BackendEvidence): string {
  switch (evidence) {
    case "executed-production": return "executed here";
    case "validated-native-fixture": return "native fixture proof";
    case "deterministic-oracle": return "oracle until probed";
  }
}

export function BackendParityWorkbench() {
  const trace = useMemo(() => traceBackendParity(), []);
  const [selectedLaneId, setSelectedLaneId] = useState<BackendParityLaneId>("rust_matrix_cpu");
  const [probe, setProbe] = useState<ProbeState>({
    status: "not-run",
    message: "Run the probe to ask this browser for a real WebGPU adapter.",
  });
  const selectedLane = trace.lanes.find((lane) => lane.id === selectedLaneId)!;

  async function runProbe() {
    setProbe({ status: "running", message: "Requesting a WebGPU adapter and executing the plan…" });
    setProbe(await probeWebGpuBackendParity());
  }

  return (
    <main className="workspace workspace--backend-parity">
      <section className="backend-parity-stage">
        <header className="backend-parity-intro">
          <div>
            <p className="eyebrow">NN31 · one graph, four execution engines</p>
            <h2>Backend parity laboratory</h2>
            <p>{trace.fixture.question}</p>
          </div>
          <span className="backend-parity-chip">
            max error {trace.maxAbsoluteError.toExponential(1)}
          </span>
        </header>

        <section className="backend-parity-paper" aria-label="Dense layer hand calculation">
          <div className="panel-heading">
            <div>
              <p className="eyebrow">1 · calculate</p>
              <h2>Do the middle row on paper</h2>
            </div>
            <code>y = XW + B</code>
          </div>
          <div className="backend-parity-equation-flow">
            <code>x = 2</code><span>×</span><code>w = 2</code><span>=</span>
            <code>4</code><span>+</span><code>b = 1</code><span>=</span><strong>5</strong>
          </div>
          <div className="backend-parity-paper-table" role="table" aria-label="Hand calculated dense layer rows">
            <div className="backend-parity-table-head" role="row">
              <strong role="columnheader">row</strong><strong role="columnheader">x</strong>
              <strong role="columnheader">x × 2</strong><strong role="columnheader">+ 1</strong>
            </div>
            {trace.fixture.scenario.inputs.map((input, index) => (
              <div role="row" key={index}>
                <strong role="cell">{index}</strong><code role="cell">{formatNumber(input)}</code>
                <code role="cell">{formatNumber(trace.products[index]!)}</code>
                <code role="cell">{formatNumber(trace.fixture.scenario.outputs[index]!)}</code>
              </div>
            ))}
          </div>
        </section>

        <section className="backend-parity-lanes" aria-label="Backend execution lanes">
          <div className="panel-heading">
            <div>
              <p className="eyebrow">2 · schedule</p>
              <h2>Same graph, different work plans</h2>
            </div>
            <code>{trace.scalarInstructionCount} scalar · {trace.matrixOperationCount} matrix</code>
          </div>
          <div className="backend-parity-lane-grid">
            {trace.lanes.map((lane) => (
              <button
                aria-label={`Inspect ${lane.title}`}
                aria-pressed={selectedLaneId === lane.id}
                key={lane.id}
                onClick={() => setSelectedLaneId(lane.id)}
                type="button"
              >
                <small>{lane.precision} · {evidenceLabel(lane.evidence)}</small>
                <strong>{lane.title}</strong>
                <span>{lane.runtime}</span>
                <code>[{lane.outputs.map(formatNumber).join(", ")}]</code>
              </button>
            ))}
          </div>
        </section>

        <section className="backend-parity-inspector" aria-label="Selected backend detail">
          <div className="panel-heading">
            <div>
              <p className="eyebrow">3 · inspect {selectedLane.precision}</p>
              <h2>{selectedLane.title}</h2>
            </div>
            <code>{selectedLane.availability}</code>
          </div>
          <div className="backend-parity-detail-grid">
            <div>
              <small>operations</small>
              <ol>{selectedLane.steps.map((step) => <li key={step}>{step}</li>)}</ol>
            </div>
            <div>
              <small>buffer residency</small>
              <ol>{selectedLane.residency.map((place) => <li key={place}><code>{place}</code></li>)}</ol>
            </div>
            <div>
              <small>proof</small>
              <strong>{evidenceLabel(selectedLane.evidence)}</strong>
              <p>maximum absolute error: <code>{selectedLane.maxAbsoluteError.toExponential(1)}</code></p>
            </div>
          </div>
        </section>

        <section className="backend-parity-results" aria-label="Backend output parity">
          <div className="panel-heading">
            <div>
              <p className="eyebrow">4 · compare</p>
              <h2>Every lane meets the same oracle</h2>
            </div>
            <code>tolerance {trace.fixture.absoluteTolerance}</code>
          </div>
          <div className="backend-parity-results-table" role="table" aria-label="CPU Rust and accelerator outputs">
            <div className="backend-parity-table-head" role="row">
              <strong role="columnheader">lane</strong>
              {trace.fixture.scenario.inputs.map((_, index) => <strong role="columnheader" key={index}>row {index}</strong>)}
              <strong role="columnheader">error</strong>
            </div>
            {trace.lanes.map((lane) => (
              <div role="row" key={lane.id}>
                <strong role="cell">{lane.title}</strong>
                {lane.outputs.map((output, index) => <code role="cell" key={index}>{formatNumber(output)}</code>)}
                <code role="cell">{lane.maxAbsoluteError.toExponential(1)}</code>
              </div>
            ))}
          </div>
        </section>

        <section className="backend-parity-probe" aria-label="WebGPU runtime probe">
          <div>
            <p className="eyebrow">5 · prove the hardware claim</p>
            <h2>Browser accelerator probe</h2>
            <p>{probe.message}</p>
            {probe.status === "executed" ? (
              <code>
                [{probe.outputs.map(formatNumber).join(", ")}] · error {probe.maxAbsoluteError.toExponential(1)} · {probe.withinTolerance ? "parity pass" : "parity mismatch"}
              </code>
            ) : null}
          </div>
          <div className={`backend-parity-probe-status backend-parity-probe-status--${probe.status}`}>
            <strong>{probe.status}</strong>
            <button disabled={probe.status === "running"} onClick={runProbe} type="button">
              {probe.status === "running" ? "Running…" : "Run WebGPU probe"}
            </button>
          </div>
        </section>
      </section>

      <aside className="backend-parity-controls">
        <p className="eyebrow">Mental model</p>
        <h2>Meaning above, mechanics below</h2>
        <p>The graph owns the equation. A backend owns scheduling, precision, buffers, and transfers.</p>
        <div className="backend-parity-rule">
          <code>same graph</code><span>+</span><code>same input</code><span>→</span><strong>equal output</strong>
        </div>
        <section>
          <p className="eyebrow">Rust boundary</p>
          <p>MatrixIR JSON and little-endian f32 buffers are shared. The Node-free Rust helper test executes the checked-in bytes through <code>matrix-cpu</code>.</p>
        </section>
        <section>
          <p className="eyebrow">Language direction</p>
          <p>New language ports can replay this oracle natively, then swap in a Rust binding. A stable C ABI remains an explicit future tranche.</p>
        </section>
        <section className="backend-parity-warning">
          <p className="eyebrow">Do not confuse</p>
          <p>Equal answers prove correctness. They do not prove the GPU is faster—or that it ran at all.</p>
        </section>
      </aside>
    </main>
  );
}
