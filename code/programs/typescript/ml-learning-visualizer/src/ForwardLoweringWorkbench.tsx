import { useMemo, useState } from "react";
import {
  FORWARD_LOWERING_SCENARIOS,
  traceForwardLowering,
  type ForwardLoweringScenarioId,
  type NormalizedMatrixOperation,
  type NormalizedNeuralInstruction,
} from "./forward-lowering-lab.js";

type Selection =
  | { readonly lane: "neural"; readonly id: string }
  | { readonly lane: "matrix"; readonly id: string };

function formatNumber(value: number): string {
  if (Math.abs(value) < 1e-12) return "0";
  if (Number.isInteger(value)) return String(value);
  return Number(value.toPrecision(10)).toString();
}

function describeNeural(instruction: NormalizedNeuralInstruction): string {
  switch (instruction.op) {
    case "LOAD_CONST": return `materialize ${instruction.attributes.value}`;
    case "LOAD_INPUT": return `bind ${instruction.attributes.input_name}`;
    case "LOAD_EDGE_WEIGHT": return `load ${instruction.attributes.edge_id}`;
    case "MUL": return `${instruction.inputs.join(" x ")}`;
    case "ADD": return instruction.inputs.join(" + ");
    case "ACTIVATE": return `${instruction.attributes.activation}(${instruction.inputs[0]})`;
    case "STORE_OUTPUT": return `publish ${instruction.attributes.output_name}`;
    default: return instruction.op;
  }
}

function describeMatrix(operation: NormalizedMatrixOperation): string {
  switch (operation.op) {
    case "LOAD_CONST_MATRIX": return `broadcast ${operation.attributes.value}`;
    case "LOAD_INPUT_MATRIX": return `column ${operation.attributes.input_name}`;
    case "WEIGHTED_SUM_MATRIX": return `${operation.inputs.length} fused terms`;
    case "ACTIVATE_MATRIX": return `${operation.attributes.activation} column`;
    case "STORE_OUTPUT_MATRIX": return `publish ${operation.attributes.output_name}`;
    default: return operation.op;
  }
}

function attributeText(
  attributes: Readonly<Record<string, string | number | readonly string[] | readonly number[]>>,
): string {
  const entries = Object.entries(attributes);
  if (entries.length === 0) return "none";
  return entries.map(([key, value]) => (
    `${key}=${Array.isArray(value) ? `[${value.join(", ")}]` : String(value)}`
  )).join("; ");
}

export function ForwardLoweringWorkbench() {
  const [scenarioId, setScenarioId] = useState<ForwardLoweringScenarioId>("single_row");
  const [selection, setSelection] = useState<Selection>({ lane: "matrix", id: "m3" });
  const trace = useMemo(() => traceForwardLowering(scenarioId), [scenarioId]);
  const selectedNeural = selection.lane === "neural"
    ? trace.neuralIr.instructions.find((instruction) => instruction.id === selection.id)
    : undefined;
  const selectedMatrix = selection.lane === "matrix"
    ? trace.matrixIr.operations.find((operation) => operation.id === selection.id)
    : undefined;
  const selectedReading = selectedNeural === undefined
    ? undefined
    : trace.firstRowInstructionReadings.find((reading) => reading.instructionId === selectedNeural.id);

  return (
    <main className="workspace workspace--forward-lowering">
      <section className="forward-lowering-stage">
        <header className="forward-lowering-intro">
          <div>
            <p className="eyebrow">NN29 - graph -&gt; NeuralIR -&gt; MatrixIR</p>
            <h2>Forward graph lowering map</h2>
            <p>
              Keep one prediction fixed while a dependency graph becomes an ordered
              scalar program and then a fused batch plan.
            </p>
          </div>
          <span className="forward-lowering-chip">
            6 nodes -&gt; {trace.neuralIr.instructions.length} instructions -&gt; {trace.matrixIr.operations.length} ops
          </span>
        </header>

        <section className="forward-lowering-graph" aria-label="Canonical forward neural graph">
          <div className="panel-heading">
            <div>
              <p className="eyebrow">1 - meaning</p>
              <h2>The graph says what depends on what</h2>
            </div>
            <code>{trace.graph.topologicalOrder.join(" -> ")}</code>
          </div>
          <div className="forward-lowering-node-flow">
            <div className="forward-lowering-input-stack">
              {trace.graph.nodes.slice(0, 3).map((node) => (
                <article key={node.id}>
                  <strong>{node.id}</strong>
                  <span>{node.detail}</span>
                </article>
              ))}
            </div>
            <span className="forward-lowering-arrow">-&gt;</span>
            {trace.graph.nodes.slice(3).map((node, index) => (
              <div className="forward-lowering-flow-tail" key={node.id}>
                <article>
                  <strong>{node.id}</strong>
                  <span>{node.detail}</span>
                </article>
                {index < 2 ? <span className="forward-lowering-arrow">-&gt;</span> : null}
              </div>
            ))}
          </div>
          <div className="forward-lowering-edge-grid">
            {trace.graph.edges.slice(0, 3).map((edge) => (
              <div key={edge.id}>
                <code>{edge.id}</code>
                <span>{edge.from} -&gt; {edge.to}</span>
                <strong>x {formatNumber(edge.weight)}</strong>
              </div>
            ))}
          </div>
        </section>

        <section className="forward-lowering-ir" aria-label="NeuralIR instruction stream">
          <div className="panel-heading">
            <div>
              <p className="eyebrow">2 - schedule</p>
              <h2>NeuralIR writes each value once</h2>
            </div>
            <code>{trace.neuralIr.magic} v{trace.neuralIr.version}</code>
          </div>
          <div className="forward-lowering-instruction-lane">
            {trace.neuralIr.instructions.map((instruction) => (
              <button
                aria-label={`Open NeuralIR ${instruction.id}, ${instruction.op}`}
                aria-pressed={selection.lane === "neural" && selection.id === instruction.id}
                key={instruction.id}
                onClick={() => setSelection({ lane: "neural", id: instruction.id })}
                type="button"
              >
                <small>{instruction.id}</small>
                <strong>{instruction.op}</strong>
                <code>{instruction.output ?? "output boundary"}</code>
                <span>{describeNeural(instruction)}</span>
              </button>
            ))}
          </div>
        </section>

        <section className="forward-lowering-ir" aria-label="MatrixIR operation stream">
          <div className="panel-heading">
            <div>
              <p className="eyebrow">3 - fuse</p>
              <h2>MatrixIR keeps columns together</h2>
            </div>
            <code>{trace.matrixIr.magic} v{trace.matrixIr.version}</code>
          </div>
          <div className="forward-lowering-matrix-lane">
            {trace.matrixIr.operations.map((operation) => (
              <button
                aria-label={`Open MatrixIR ${operation.id}, ${operation.op}`}
                aria-pressed={selection.lane === "matrix" && selection.id === operation.id}
                key={operation.id}
                onClick={() => setSelection({ lane: "matrix", id: operation.id })}
                type="button"
              >
                <small>{operation.id}</small>
                <strong>{operation.op}</strong>
                <code>{operation.output ?? "output boundary"}</code>
                <span>{describeMatrix(operation)}</span>
              </button>
            ))}
          </div>
        </section>

        <section className="forward-lowering-selection" aria-label="Selected lowering detail">
          <div className="panel-heading">
            <div>
              <p className="eyebrow">selected translation</p>
              <h2>{selectedNeural?.op ?? selectedMatrix?.op}</h2>
            </div>
            <code>{selection.id}</code>
          </div>
          {selectedNeural !== undefined ? (
            <div className="forward-lowering-detail-grid">
              <div>
                <small>reads</small>
                <code>{selectedReading?.reads.map((read) => (
                  `${read.valueId}=${formatNumber(read.value)}`
                )).join(", ") || "none"}</code>
              </div>
              <div>
                <small>writes</small>
                <code>{selectedReading?.write === undefined
                  ? `${selectedReading?.output?.outputName}=${formatNumber(selectedReading?.output?.value ?? 0)}`
                  : `${selectedReading.write.valueId}=${formatNumber(selectedReading.write.value)}`}</code>
              </div>
              <div>
                <small>graph provenance</small>
                <code>{[...selectedNeural.sourceNodes, ...selectedNeural.sourceEdges].join(", ") || "none"}</code>
              </div>
            </div>
          ) : selectedMatrix !== undefined ? (
            <div className="forward-lowering-detail-grid">
              <div>
                <small>fuses NeuralIR</small>
                <code>{selectedMatrix.sourceInstructions.join(", ")}</code>
              </div>
              <div>
                <small>attributes</small>
                <code>{attributeText(selectedMatrix.attributes)}</code>
              </div>
              <div>
                <small>graph provenance</small>
                <code>{[...selectedMatrix.sourceNodes, ...selectedMatrix.sourceEdges].join(", ") || "none"}</code>
              </div>
            </div>
          ) : null}
        </section>

        <section className="forward-lowering-parity" aria-label="Forward lowering execution parity">
          <div className="panel-heading">
            <div>
              <p className="eyebrow">4 - prove equivalence</p>
              <h2>Three paths, the same prediction</h2>
            </div>
            <code>max error {trace.maxParityError.toExponential(1)}</code>
          </div>
          <div className="forward-lowering-parity-table" role="table" aria-label="Direct NeuralIR MatrixIR outputs">
            <div className="forward-lowering-parity-head" role="row">
              <strong role="columnheader">row</strong>
              <strong role="columnheader">x0</strong>
              <strong role="columnheader">x1</strong>
              <strong role="columnheader">direct</strong>
              <strong role="columnheader">NeuralIR</strong>
              <strong role="columnheader">MatrixIR</strong>
            </div>
            {trace.directOutputs.map((direct, index) => (
              <div key={index} role="row">
                <strong role="cell">{index}</strong>
                <code role="cell">{formatNumber(trace.scenario.inputs.x0[index]!)}</code>
                <code role="cell">{formatNumber(trace.scenario.inputs.x1[index]!)}</code>
                <code role="cell">{formatNumber(direct)}</code>
                <code role="cell">{formatNumber(trace.neuralIrOutputs[index]!)}</code>
                <code role="cell">{formatNumber(trace.matrixIrOutputs[index]!)}</code>
              </div>
            ))}
          </div>
        </section>
      </section>

      <aside className="forward-lowering-controls">
        <p className="eyebrow">Run shape</p>
        <h2>Keep the compiler fixed</h2>
        <p>Change only the number of input rows and watch every IR identifier stay stable.</p>
        <div className="forward-lowering-scenario-buttons">
          {FORWARD_LOWERING_SCENARIOS.map((scenario) => (
            <button
              aria-label={scenario.title}
              aria-pressed={scenarioId === scenario.id}
              key={scenario.id}
              onClick={() => setScenarioId(scenario.id as ForwardLoweringScenarioId)}
              type="button"
            >
              <strong>{scenario.title}</strong>
              <span>{scenario.summary}</span>
            </button>
          ))}
        </div>
        <div className="forward-lowering-equation">
          <p className="eyebrow">Paper result</p>
          <code>z = -1 + 0.25x0 + 0.75x1</code>
          <code>prediction = max(0, z)</code>
        </div>
        <div className="forward-lowering-mental-model">
          <p className="eyebrow">Rust boundary</p>
          <h2>Meaning stays above tensors</h2>
          <p>
            The neural compiler retains source IDs and fusion rules. A Rust MX01 bridge
            receives explicit tensors, dtypes, shapes, constants, inputs, and outputs.
          </p>
        </div>
      </aside>
    </main>
  );
}
