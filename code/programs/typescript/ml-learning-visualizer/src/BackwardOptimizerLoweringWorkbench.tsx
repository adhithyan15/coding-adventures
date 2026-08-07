import { useMemo, useState } from "react";
import {
  BACKWARD_OPTIMIZER_LOWERING_SCENARIOS,
  traceBackwardOptimizerLowering,
  type BackwardOptimizerLoweringScenarioId,
  type BackwardOptimizerLoweringTrace,
  type TrainingLoweringInstruction,
  type TrainingLoweringStream,
} from "./backward-optimizer-lowering-lab.js";

type Lane = "backward" | "optimizer" | "matrix";

interface Selection {
  readonly lane: Lane;
  readonly id: string;
}

function formatNumber(value: number): string {
  if (Math.abs(value) < 1e-12) return "0";
  if (Number.isInteger(value)) return String(value);
  return Number(value.toPrecision(10)).toString();
}

function describeInstruction(item: TrainingLoweringInstruction): string {
  switch (item.op) {
    case "SEED_LOSS_GRAD": return "start reverse mode at 1";
    case "HALF_SQUARED_ERROR_GRAD": return "residual x loss seed";
    case "PROPAGATE_GRAD": return "pass through subtraction";
    case "PARAMETER_LOCAL_GRAD": return "x x d_prediction";
    case "ACCUMULATE_GRAD": return "add rows in stable order";
    case "INPUT_GRAD": return "w x d_prediction";
    case "READ_GRAD_BUFFER": return "read persistent grad_w";
    case "DIVIDE_GRAD": return "apply explicit divisor";
    case "SGD_UPDATE":
    case "SGD_UPDATE_SCALAR": return "w - rate x gradient";
    case "KEEP_GRAD_BUFFER": return "step does not clear";
    case "LOAD_SAVED_COLUMN": return `load ${item.inputs[0]} rows`;
    case "LOSS_GRAD_COLUMN": return "reverse loss as a column";
    case "PARAMETER_LOCAL_GRAD_COLUMN": return "one d_w per row";
    case "INPUT_GRAD_COLUMN": return "one d_x per row";
    case "REDUCE_SUM_GRAD": return "row-ascending reduction";
    case "ACCUMULATE_GRAD_BUFFER": return "add batch sum to persistent grad_w";
    default: return item.inputs.join(", ");
  }
}

function attributeText(item: TrainingLoweringInstruction): string {
  const entries = Object.entries(item.attributes);
  if (entries.length === 0) return "none";
  return entries.map(([key, value]) => (
    `${key}=${Array.isArray(value) ? `[${value.join(", ")}]` : String(value)}`
  )).join("; ");
}

function streamFor(trace: BackwardOptimizerLoweringTrace, lane: Lane): TrainingLoweringStream {
  if (lane === "backward") return trace.backwardIr;
  if (lane === "optimizer") return trace.optimizerIr;
  return trace.matrixTrainingIr;
}

function selectedValue(
  trace: BackwardOptimizerLoweringTrace,
  selection: Selection,
): string {
  const backwardValues: Record<string, readonly number[] | number> = {
    b0: trace.backward.dLoss,
    b1: trace.backward.dResidual,
    b2: trace.backward.dPrediction,
    b3: trace.backward.localDW,
    b4: trace.backward.gradW,
    b5: trace.backward.dX,
  };
  const optimizerValues: Record<string, number> = {
    o0: trace.backward.gradW,
    o1: trace.optimizer.appliedGradient,
    o2: trace.optimizer.parameterAfter,
    o3: trace.optimizer.gradientBufferAfterStep,
  };
  const matrixValues: Record<string, readonly number[] | number> = {
    t0: trace.matrixTraining.columns.x,
    t1: trace.matrixTraining.columns.residual,
    t2: trace.matrixTraining.columns.dPrediction,
    t3: trace.matrixTraining.columns.localDW,
    t4: trace.matrixTraining.columns.dX,
    t5: trace.matrixTraining.batchGradient,
    t6: trace.matrixTraining.gradW,
    t7: trace.matrixTraining.appliedGradient,
    t8: trace.matrixTraining.parameterAfter,
    t9: trace.matrixTraining.gradientBufferAfterStep,
  };
  const value = selection.lane === "backward"
    ? backwardValues[selection.id]
    : selection.lane === "optimizer"
      ? optimizerValues[selection.id]
      : matrixValues[selection.id];
  if (typeof value === "number") return formatNumber(value);
  return `[${(value ?? []).map(formatNumber).join(", ")}]`;
}

function IrLane({
  lane,
  selection,
  setSelection,
  stream,
}: {
  readonly lane: Lane;
  readonly selection: Selection;
  readonly setSelection: (selection: Selection) => void;
  readonly stream: TrainingLoweringStream;
}) {
  const className = lane === "matrix"
    ? "forward-lowering-matrix-lane"
    : "forward-lowering-instruction-lane";
  const laneName = lane === "matrix" ? "Matrix training IR" : lane === "backward" ? "Backward IR" : "Optimizer IR";
  return (
    <div className={className}>
      {stream.instructions.map((item) => (
        <button
          aria-label={`Open ${laneName} ${item.id}, ${item.op}`}
          aria-pressed={selection.lane === lane && selection.id === item.id}
          key={item.id}
          onClick={() => setSelection({ lane, id: item.id })}
          type="button"
        >
          <small>{item.id}</small>
          <strong>{item.op}</strong>
          <code>{item.output}</code>
          <span>{describeInstruction(item)}</span>
        </button>
      ))}
    </div>
  );
}

export function BackwardOptimizerLoweringWorkbench() {
  const [scenarioId, setScenarioId] = useState<BackwardOptimizerLoweringScenarioId>("one_row_by_hand");
  const [selection, setSelection] = useState<Selection>({ lane: "backward", id: "b3" });
  const trace = useMemo(() => traceBackwardOptimizerLowering(scenarioId), [scenarioId]);
  const selectedStream = streamFor(trace, selection.lane);
  const selected = selectedStream.instructions.find((item) => item.id === selection.id);

  return (
    <main className="workspace workspace--forward-lowering">
      <section className="forward-lowering-stage">
        <header className="forward-lowering-intro">
          <div>
            <p className="eyebrow">NN30 - saved values -&gt; backward -&gt; optimizer</p>
            <h2>Backward and optimizer lowering map</h2>
            <p>
              Keep one trainable multiplication fixed while reverse mode becomes an
              executable schedule and SGD remains a separate state transition.
            </p>
          </div>
          <span className="forward-lowering-chip">
            {trace.backwardIr.instructions.length} backward -&gt; {trace.optimizerIr.instructions.length} optimizer -&gt; {trace.matrixTrainingIr.instructions.length} matrix ops
          </span>
        </header>

        <section className="forward-lowering-graph" aria-label="Production forward saved values">
          <div className="panel-heading">
            <div>
              <p className="eyebrow">1 - save</p>
              <h2>The production forward pass leaves evidence</h2>
            </div>
            <code>max forward error {trace.forward.maxError.toExponential(1)}</code>
          </div>
          <div className="forward-lowering-edge-grid">
            <div>
              <code>NeuralIR</code>
              <span>{trace.forward.neuralOps.join(" -> ")}</span>
              <strong>{trace.forward.neuralIrOutputs.map(formatNumber).join(", ")}</strong>
            </div>
            <div>
              <code>MatrixIR</code>
              <span>{trace.forward.matrixOps.join(" -> ")}</span>
              <strong>{trace.forward.matrixIrOutputs.map(formatNumber).join(", ")}</strong>
            </div>
            <div>
              <code>saved contract</code>
              <span>x, w, prediction, residual</span>
              <strong>backward may read them</strong>
            </div>
          </div>
          <div className="forward-lowering-parity-table" role="table" aria-label="Saved forward row values">
            <div className="forward-lowering-parity-head" role="row">
              <strong role="columnheader">row</strong>
              <strong role="columnheader">x</strong>
              <strong role="columnheader">target</strong>
              <strong role="columnheader">prediction</strong>
              <strong role="columnheader">residual</strong>
              <strong role="columnheader">loss</strong>
            </div>
            {trace.savedValues.x.map((x, index) => (
              <div key={index} role="row">
                <strong role="cell">{index}</strong>
                <code role="cell">{formatNumber(x)}</code>
                <code role="cell">{formatNumber(trace.savedValues.target[index]!)}</code>
                <code role="cell">{formatNumber(trace.savedValues.prediction[index]!)}</code>
                <code role="cell">{formatNumber(trace.savedValues.residual[index]!)}</code>
                <code role="cell">{formatNumber(trace.savedValues.loss[index]!)}</code>
              </div>
            ))}
          </div>
        </section>

        <section className="forward-lowering-ir" aria-label="Backward instruction stream">
          <div className="panel-heading">
            <div><p className="eyebrow">2 - reverse</p><h2>Backward produces gradients</h2></div>
            <code>{trace.backwardIr.magic} v{trace.backwardIr.version}</code>
          </div>
          <IrLane lane="backward" selection={selection} setSelection={setSelection} stream={trace.backwardIr} />
        </section>

        <section className="forward-lowering-ir" aria-label="Optimizer instruction stream">
          <div className="panel-heading">
            <div><p className="eyebrow">3 - update policy</p><h2>The optimizer consumes the buffer</h2></div>
            <code>{trace.optimizerIr.magic} v{trace.optimizerIr.version}</code>
          </div>
          <IrLane lane="optimizer" selection={selection} setSelection={setSelection} stream={trace.optimizerIr} />
        </section>

        <section className="forward-lowering-ir" aria-label="Matrix training operation stream">
          <div className="panel-heading">
            <div><p className="eyebrow">4 - batch</p><h2>Columns reduce into shared parameter state</h2></div>
            <code>{trace.matrixTrainingIr.magic} v{trace.matrixTrainingIr.version}</code>
          </div>
          <IrLane lane="matrix" selection={selection} setSelection={setSelection} stream={trace.matrixTrainingIr} />
        </section>

        <section className="forward-lowering-selection" aria-label="Selected training lowering detail">
          <div className="panel-heading">
            <div><p className="eyebrow">selected translation</p><h2>{selected?.op}</h2></div>
            <code>{selection.id}</code>
          </div>
          {selected === undefined ? null : (
            <div className="forward-lowering-detail-grid">
              <div><small>reads -&gt; writes</small><code>{selected.inputs.join(", ") || "none"} -&gt; {selected.output}</code></div>
              <div><small>observed value</small><code>{selectedValue(trace, selection)}</code></div>
              <div><small>attributes</small><code>{attributeText(selected)}</code></div>
              <div><small>graph provenance</small><code>{[...selected.sourceNodes, ...selected.sourceEdges].join(", ") || "none"}</code></div>
              <div><small>lowered from</small><code>{selected.sourceInstructions.join(", ") || "direct semantic rule"}</code></div>
            </div>
          )}
        </section>

        <section className="forward-lowering-parity" aria-label="Backward optimizer execution parity">
          <div className="panel-heading">
            <div><p className="eyebrow">5 - prove equivalence</p><h2>Scalar and matrix training agree</h2></div>
            <code>max error {trace.maxPathError.toExponential(1)}</code>
          </div>
          <div className="forward-lowering-parity-table" role="table" aria-label="Backward row gradient values">
            <div className="forward-lowering-parity-head" role="row">
              <strong role="columnheader">row</strong>
              <strong role="columnheader">x</strong>
              <strong role="columnheader">target</strong>
              <strong role="columnheader">d prediction</strong>
              <strong role="columnheader">local d w</strong>
              <strong role="columnheader">d x</strong>
            </div>
            {trace.backward.dPrediction.map((gradient, index) => (
              <div key={index} role="row">
                <strong role="cell">{index}</strong>
                <code role="cell">{formatNumber(trace.scenario.inputs[index]!)}</code>
                <code role="cell">{formatNumber(trace.scenario.targets[index]!)}</code>
                <code role="cell">{formatNumber(gradient)}</code>
                <code role="cell">{formatNumber(trace.backward.localDW[index]!)}</code>
                <code role="cell">{formatNumber(trace.backward.dX[index]!)}</code>
              </div>
            ))}
          </div>
          <div className="forward-lowering-edge-grid">
            <div><code>persistent accumulation</code><span>{formatNumber(trace.backward.gradientBufferBefore)} before + {trace.backward.localDW.map(formatNumber).join(" + ")}</span><strong>grad_w = {formatNumber(trace.backward.gradW)}</strong></div>
            <div><code>explicit divisor</code><span>{formatNumber(trace.backward.gradW)} / {trace.scenario.divisor}</span><strong>applied = {formatNumber(trace.optimizer.appliedGradient)}</strong></div>
            <div><code>SGD update</code><span>{formatNumber(trace.optimizer.parameterBefore)} - {formatNumber(trace.scenario.learningRate)} x {formatNumber(trace.optimizer.appliedGradient)}</span><strong>w_next = {formatNumber(trace.optimizer.parameterAfter)}</strong></div>
          </div>
        </section>
      </section>

      <aside className="forward-lowering-controls">
        <p className="eyebrow">Run shape</p>
        <h2>Keep the programs fixed</h2>
        <p>Change the number of rows or enter with a nonzero buffer while the programs stay fixed.</p>
        <div className="forward-lowering-scenario-buttons">
          {BACKWARD_OPTIMIZER_LOWERING_SCENARIOS.map((scenario) => (
            <button
              aria-label={scenario.title}
              aria-pressed={scenarioId === scenario.id}
              key={scenario.id}
              onClick={() => setScenarioId(scenario.id as BackwardOptimizerLoweringScenarioId)}
              type="button"
            >
              <strong>{scenario.title}</strong><span>{scenario.summary}</span>
            </button>
          ))}
        </div>
        <div className="forward-lowering-equation">
          <p className="eyebrow">Paper result</p>
          <code>loss = 0.5(w x x - target)^2</code>
          <code>d_w = (prediction - target) x x</code>
          <code>grad_w = grad_w_before + sum(d_w)</code>
          <code>w_next = w - rate x (grad_w / divisor)</code>
        </div>
        <div className="forward-lowering-mental-model">
          <p className="eyebrow">Gradient audit</p>
          <h2>Different route, same slope</h2>
          <p>Finite difference {formatNumber(trace.gradientAudit.numerical)} vs backward {formatNumber(trace.gradientAudit.analytical)}; error {trace.gradientAudit.absoluteError.toExponential(1)}.</p>
        </div>
        <div className="forward-lowering-mental-model">
          <p className="eyebrow">Rust boundary</p>
          <h2>Tensor math, explicit policy</h2>
          <p>Rust may accelerate multiply, add, and ReduceSum. The host still owns saved values, divisor, update timing, and zeroing.</p>
        </div>
      </aside>
    </main>
  );
}
