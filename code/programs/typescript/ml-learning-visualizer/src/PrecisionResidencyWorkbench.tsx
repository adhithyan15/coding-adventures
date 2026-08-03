import { useMemo, useState } from "react";
import {
  tracePrecisionResidency,
  type PrecisionFormatId,
  type ResidencyStrategyId,
} from "./precision-residency-lab.js";

function formatNumber(value: number): string {
  if (Math.abs(value) < 1e-12) return "0";
  if (Number.isInteger(value)) return String(value);
  return Number(value.toPrecision(10)).toString();
}

function storageLabel(id: PrecisionFormatId, bytes: number, accumulatorBytes?: number): string {
  if (id === "symmetric_int8") return `${bytes}-byte operands · ${accumulatorBytes}-byte accumulator`;
  return `${bytes} byte${bytes === 1 ? "" : "s"} / value`;
}

export function PrecisionResidencyWorkbench() {
  const [formatId, setFormatId] = useState<PrecisionFormatId>("binary16");
  const [strategyId, setStrategyId] = useState<ResidencyStrategyId>("resident");
  const [repeatCount, setRepeatCount] = useState(3);
  const trace = useMemo(
    () => tracePrecisionResidency(formatId, strategyId, repeatCount),
    [formatId, strategyId, repeatCount],
  );

  return (
    <main className="workspace workspace--precision-residency">
      <section className="precision-stage">
        <header className="precision-intro">
          <div>
            <p className="eyebrow">NN32 - smaller numbers, fewer journeys</p>
            <h2>Precision and residency laboratory</h2>
            <p>{trace.fixture.question}</p>
          </div>
          <span className="precision-chip">{storageLabel(trace.format.id, trace.format.storageBytesPerValue, trace.format.accumulatorStorageBytes)}</span>
        </header>

        <section className="precision-paper" aria-label="Reference affine calculation">
          <div className="panel-heading">
            <div><p className="eyebrow">1 - calculate</p><h2>Start with the paper answer</h2></div>
            <code>y = x * 2 + 0</code>
          </div>
          <div className="precision-equation-row">
            <code>1.0004 * 2 = <strong>2.0008</strong></code>
            <code>1.0006 * 2 = <strong>2.0012</strong></code>
          </div>
        </section>

        <section className="precision-formats" aria-label="Precision and quantization formats">
          <div className="panel-heading">
            <div><p className="eyebrow">2 - encode</p><h2>Move onto a smaller number grid</h2></div>
            <code>max error {trace.format.maximumAbsoluteError.toExponential(3)}</code>
          </div>
          <div className="precision-format-buttons">
            {trace.fixture.formats.map((format) => (
              <button
                aria-label={`Use ${format.title}`}
                aria-pressed={format.id === formatId}
                key={format.id}
                onClick={() => setFormatId(format.id)}
                type="button"
              >
                <small>{storageLabel(format.id, format.storageBytesPerValue, format.accumulatorStorageBytes)}</small>
                <strong>{format.title}</strong>
                <code>[{format.outputs.map(formatNumber).join(", ")}]</code>
              </button>
            ))}
          </div>
          {trace.format.id === "symmetric_int8" ? (
            <p className="precision-scale-note">
              <code>input scale {trace.format.inputScale}</code>
              <code>weight scale {trace.format.weightScale}</code>
              <span>Both close inputs become integer 100.</span>
            </p>
          ) : null}
        </section>

        <section className="precision-trace" aria-label="Selected precision arithmetic trace">
          <div className="panel-heading">
            <div><p className="eyebrow">3 - inspect {trace.format.id}</p><h2>Every rounding step stays visible</h2></div>
          </div>
          <div className="precision-table" role="table" aria-label="Precision output and error rows">
            <div className="precision-table-head" role="row">
              <strong role="columnheader">paper x</strong><strong role="columnheader">encoded x</strong>
              <strong role="columnheader">encoded w</strong><strong role="columnheader">accumulator</strong>
              <strong role="columnheader">output</strong><strong role="columnheader">absolute error</strong>
            </div>
            {trace.rows.map((row, index) => (
              <div role="row" key={index}>
                <code role="cell">{formatNumber(row.input)}</code><code role="cell">{formatNumber(row.encodedInput)}</code>
                <code role="cell">{formatNumber(row.encodedWeight)}</code><code role="cell">{formatNumber(row.accumulator)}</code>
                <code role="cell">{formatNumber(row.output)}</code><code role="cell">{row.absoluteError.toExponential(3)}</code>
              </div>
            ))}
          </div>
        </section>

        <section className="precision-residency" aria-label="Buffer residency transfer trace">
          <div className="panel-heading">
            <div><p className="eyebrow">4 - place buffers</p><h2>Same answer, fewer boundary crossings</h2></div>
            <code>{trace.fixture.residency.dtype} baseline · {trace.transferBytes} transfer bytes</code>
          </div>
          <div className="precision-transfer-flow">
            <div><small>host to device</small><strong>{trace.uploadCount} upload{trace.uploadCount === 1 ? "" : "s"}</strong><code>{trace.fixture.residency.uploadBytesPerCopy} bytes each</code></div>
            <span aria-hidden="true">-&gt;</span>
            <div><small>device work</small><strong>{trace.repeatCount} forward pass{trace.repeatCount === 1 ? "" : "es"}</strong><code>x, w, b, y</code></div>
            <span aria-hidden="true">-&gt;</span>
            <div><small>device to host</small><strong>{trace.downloadCount} download{trace.downloadCount === 1 ? "" : "s"}</strong><code>{trace.fixture.residency.downloadBytesPerCopy} bytes each</code></div>
          </div>
          <div className="precision-transfer-equation">
            <code>{strategyId === "eager" ? `(${trace.fixture.residency.uploadBytesPerCopy} + ${trace.fixture.residency.downloadBytesPerCopy}) * ${trace.repeatCount}` : `${trace.fixture.residency.uploadBytesPerCopy} + ${trace.fixture.residency.downloadBytesPerCopy}`}</code>
            <span>=</span><strong>{trace.transferBytes} bytes</strong>
            <span>-</span><code>{trace.bytesSavedAgainstEager} bytes saved vs eager</code>
          </div>
        </section>
      </section>

      <aside className="precision-controls">
        <p className="eyebrow">Experiment controls</p>
        <h2>Separate representation from travel</h2>
        <label>
          repeat forward pass
          <input aria-label="Forward pass repeats" max="8" min="1" onInput={(event) => setRepeatCount(Number(event.currentTarget.value))} type="range" value={repeatCount} />
          <code>{repeatCount}</code>
        </label>
        <div className="precision-strategy-buttons">
          {trace.fixture.residency.strategies.map((strategy) => (
            <button aria-label={strategy.title} aria-pressed={strategy.id === strategyId} key={strategy.id} onClick={() => setStrategyId(strategy.id)} type="button">
              <strong>{strategy.title}</strong><span>{strategy.steps[0]}</span>
            </button>
          ))}
        </div>
        <section>
          <p className="eyebrow">Transfer accounting</p>
          <p>The copy experiment stays on a binary32 byte baseline so number representation and buffer travel can be changed independently.</p>
        </section>
        <section>
          <p className="eyebrow">Selected schedule</p>
          <ol>{trace.strategy.steps.map((step) => <li key={step}>{step}</li>)}</ol>
        </section>
        <section>
          <p className="eyebrow">Rust-core direction</p>
          <p>Keep byte order, scales, ownership, and explicit downloads in a future C ABI. The fixture is ready before that ABI exists.</p>
        </section>
        <section className="precision-warning">
          <p className="eyebrow">Measure, do not assume</p>
          <p>Smaller values and fewer copies are performance hypotheses. Accuracy and timing still need workload-specific tests.</p>
        </section>
      </aside>
    </main>
  );
}
