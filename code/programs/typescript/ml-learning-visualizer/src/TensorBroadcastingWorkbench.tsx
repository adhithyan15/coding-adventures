import { useMemo, useState, type CSSProperties } from "react";
import {
  TENSOR_BROADCAST_SCENARIOS,
  traceTensorBroadcasting,
  type CompatibleBroadcastTrace,
  type TensorBroadcastScenarioId,
} from "./tensor-broadcasting-lab.js";

function formatNumber(value: number, digits = 6): string {
  if (Math.abs(value) < 1e-12) return "0";
  if (Math.abs(value) < 0.0001 || Math.abs(value) >= 1000) return value.toExponential(3);
  return Number(value.toFixed(digits)).toString();
}

function formatShape(shape: readonly number[]): string {
  return shape.length === 0 ? "[] scalar" : `[${shape.join(", ")}]`;
}

function formatIndex(index: readonly number[]): string {
  return index.length === 0 ? "[]" : `[${index.join(", ")}]`;
}

function formatVector(values: readonly number[]): string {
  return `[${values.map((value) => formatNumber(value)).join(", ")}]`;
}

function ShapeAlignment({ trace }: { trace: ReturnType<typeof traceTensorBroadcasting> }) {
  const mismatchAxis = trace.compatible ? -1 : trace.mismatchAxis;
  return (
    <section className="tensor-shape-panel" aria-label="Right aligned tensor shapes">
      <div className="panel-heading">
        <div>
          <p className="eyebrow">Step 1 / line up the tail</p>
          <h2>Compare dimensions from the right</h2>
        </div>
        <span>equal or one</span>
      </div>
      <div className="tensor-shape-equation">
        <code>{formatShape(trace.left.shape)}</code>
        <span>+</span>
        <code>{formatShape(trace.right.shape)}</code>
        <span>→</span>
        <strong>{trace.compatible ? formatShape(trace.outputShape) : "shape error"}</strong>
      </div>
      <div className="tensor-axis-grid">
        {trace.paddedLeftShape.map((leftDimension, axis) => {
          const rightDimension = trace.paddedRightShape[axis]!;
          const compatible = leftDimension === rightDimension || leftDimension === 1 || rightDimension === 1;
          return (
            <div className={axis === mismatchAxis ? "is-mismatch" : ""} key={axis}>
              <small>axis {axis}</small>
              <code>{leftDimension} ↔ {rightDimension}</code>
              <strong>{compatible ? Math.max(leftDimension, rightDimension) : "stop"}</strong>
              <span>{leftDimension === rightDimension ? "same" : compatible ? "expand the 1" : "neither is 1"}</span>
            </div>
          );
        })}
      </div>
    </section>
  );
}

function GradientReduction({ trace }: { trace: CompatibleBroadcastTrace }) {
  return (
    <section className="tensor-gradient-panel" aria-label="Broadcast gradient reduction">
      <div className="panel-heading">
        <div>
          <p className="eyebrow">Step 4 / reverse the reuse</p>
          <h2>Copied routes add back together</h2>
        </div>
        <span>sum expanded axes</span>
      </div>
      <div className="tensor-gradient-grid">
        <div>
          <small>upstream / output shape {formatShape(trace.outputShape)}</small>
          <code>{formatVector(trace.upstream.values)}</code>
        </div>
        <div>
          <small>left gradient / original shape {formatShape(trace.left.shape)}</small>
          <code>{formatVector(trace.leftGradient)}</code>
          <span>reduce axes {trace.leftExpandedAxes.length ? trace.leftExpandedAxes.join(", ") : "none"}</span>
        </div>
        <div>
          <small>right gradient / original shape {formatShape(trace.right.shape)}</small>
          <code>{formatVector(trace.rightGradient)}</code>
          <span>reduce axes {trace.rightExpandedAxes.length ? trace.rightExpandedAxes.join(", ") : "none"}</span>
        </div>
      </div>
      <div className="tensor-gradient-audit">
        <div><small>finite-difference epsilon</small><code>1e-5</code></div>
        <div><small>left numerical</small><code>{formatVector(trace.finiteDifferenceLeftGradient)}</code></div>
        <div><small>right numerical</small><code>{formatVector(trace.finiteDifferenceRightGradient)}</code></div>
        <div><small>maximum absolute error</small><code>{formatNumber(trace.maxGradientAbsoluteError)}</code></div>
      </div>
    </section>
  );
}

export function TensorBroadcastingWorkbench() {
  const [scenarioId, setScenarioId] = useState<TensorBroadcastScenarioId>("outer-grid");
  const [selectedCell, setSelectedCell] = useState(0);
  const trace = useMemo(() => traceTensorBroadcasting(scenarioId), [scenarioId]);
  const mapping = trace.compatible ? trace.mappings[Math.min(selectedCell, trace.mappings.length - 1)]! : null;
  const columns = trace.compatible ? trace.outputShape.at(-1) ?? 1 : 1;

  function chooseScenario(id: TensorBroadcastScenarioId): void {
    setScenarioId(id);
    setSelectedCell(0);
  }

  return (
    <main className="workspace workspace--tensor-broadcasting">
      <section className="tensor-broadcast-stage" aria-label="Tensor shape and broadcasting visualizer">
        <div className="tensor-broadcast-intro">
          <div>
            <p className="eyebrow">NN26 / tensor and autograd bridge</p>
            <h2>Shape and broadcasting microscope</h2>
            <p>A broadcast does not invent new parameters. It reuses an input coordinate wherever an aligned dimension is one.</p>
          </div>
          <div className="tensor-broadcast-chip">row-major</div>
        </div>

        <ShapeAlignment trace={trace} />

        {trace.compatible ? (
          <>
            <section className="tensor-output-panel" aria-label="Broadcast output coordinate map">
              <div className="panel-heading">
                <div>
                  <p className="eyebrow">Step 2 / reuse coordinates</p>
                  <h2>Open any output cell</h2>
                </div>
                <span>{trace.outputValues.length} row-major cells</span>
              </div>
              <div
                className="tensor-output-grid"
                style={{ "--tensor-columns": columns } as CSSProperties}
              >
                {trace.mappings.map((item) => (
                  <button
                    aria-label={`Open output ${formatIndex(item.outputIndex)} value ${formatNumber(item.outputValue)}`}
                    aria-pressed={item.outputFlatIndex === selectedCell}
                    key={item.outputFlatIndex}
                    type="button"
                    onClick={() => setSelectedCell(item.outputFlatIndex)}
                  >
                    <small>{formatIndex(item.outputIndex)}</small>
                    <strong>{formatNumber(item.outputValue)}</strong>
                  </button>
                ))}
              </div>
            </section>

            <section className="tensor-mapping-panel" aria-label="Selected broadcast index calculation">
              <div className="panel-heading">
                <div>
                  <p className="eyebrow">Step 3 / one hand calculation</p>
                  <h2>Output {formatIndex(mapping!.outputIndex)}</h2>
                </div>
                <span>flat slot {mapping!.outputFlatIndex}</span>
              </div>
              <div className="tensor-mapping-equation">
                <div>
                  <small>left source</small>
                  <code>{formatIndex(mapping!.leftIndex)} → {formatNumber(mapping!.leftValue)}</code>
                  <span>{trace.leftExpandedAxes.length ? `axis ${trace.leftExpandedAxes.join(", ")} reuses this slot` : "no left expansion"}</span>
                </div>
                <strong>+</strong>
                <div>
                  <small>right source</small>
                  <code>{formatIndex(mapping!.rightIndex)} → {formatNumber(mapping!.rightValue)}</code>
                  <span>{trace.rightExpandedAxes.length ? `axis ${trace.rightExpandedAxes.join(", ")} reuses this slot` : "no right expansion"}</span>
                </div>
                <strong>=</strong>
                <div>
                  <small>output</small>
                  <code>{formatIndex(mapping!.outputIndex)} → {formatNumber(mapping!.outputValue)}</code>
                  <span>upstream gradient {formatNumber(mapping!.upstream)}</span>
                </div>
              </div>
            </section>

            <GradientReduction trace={trace} />
          </>
        ) : (
          <section className="tensor-mismatch-panel" aria-label="Broadcast shape mismatch">
            <p className="eyebrow">Stop before touching the buffers</p>
            <h2>Axis {trace.mismatchAxis} cannot broadcast</h2>
            <code>{trace.leftDimension} is not {trace.rightDimension}, and neither dimension is 1</code>
            <p>{trace.error}. A tensor library should reject this deterministically instead of recycling values or reading beyond a buffer.</p>
          </section>
        )}
      </section>

      <aside className="controls tensor-broadcast-controls" aria-label="Tensor broadcasting scenarios">
        <p className="eyebrow">Shape presets</p>
        <h2>Change one alignment rule</h2>
        <div className="tensor-scenario-buttons">
          {TENSOR_BROADCAST_SCENARIOS.map((scenario) => (
            <button
              aria-pressed={scenario.id === scenarioId}
              key={scenario.id}
              type="button"
              onClick={() => chooseScenario(scenario.id)}
            >
              <strong>{scenario.title}</strong>
              <code>{formatShape(scenario.left.shape)} + {formatShape(scenario.right.shape)}</code>
              <span>{scenario.summary}</span>
            </button>
          ))}
        </div>
        <div className="tensor-mental-model">
          <p className="eyebrow">Keep this picture</p>
          <h2>Forward reuses. Backward sums.</h2>
          <p>First align the tail. Then replace each compatible one with the other dimension. Every reused route contributes when gradients return.</p>
        </div>
      </aside>
    </main>
  );
}
