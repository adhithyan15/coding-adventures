import { useMemo, useState } from "react";
import {
  DEFAULT_RESIDUAL_INPUT,
  traceResidualBlock,
} from "./residual-field-lab.js";

function formatNumber(value: number): string {
  if (Math.abs(value) < 1e-12) {
    return "0";
  }
  return Number(value.toFixed(4)).toString();
}

function SignalRow({
  label,
  values,
  selectedIndex,
  activeIndices = [],
  annotation,
}: {
  label: string;
  values: readonly number[];
  selectedIndex?: number;
  activeIndices?: readonly number[];
  annotation?: string;
}) {
  return (
    <div className="residual-signal-block">
      <div className="residual-row-label">
        <span>{label}</span>
        {annotation === undefined ? null : <code>{annotation}</code>}
      </div>
      <div
        className="residual-signal-row"
        style={{ gridTemplateColumns: `repeat(${values.length}, minmax(52px, 1fr))` }}
        aria-label={label}
      >
        {values.map((value, index) => {
          const className = index === selectedIndex
            ? "residual-cell residual-cell--selected"
            : activeIndices.includes(index)
              ? "residual-cell residual-cell--active"
              : "residual-cell";
          return (
            <div className={className} key={index}>
              <small>[{index}]</small>
              <strong>{formatNumber(value)}</strong>
            </div>
          );
        })}
      </div>
    </div>
  );
}

export function ResidualWorkbench() {
  const [selectedIndex, setSelectedIndex] = useState(2);
  const [includeSkip, setIncludeSkip] = useState(true);
  const block = useMemo(() => traceResidualBlock(), []);
  const trace = block.traces[selectedIndex]!;
  const displayedSum = trace.mainOutput + (includeSkip ? trace.skipContribution : 0);
  const displayedOutput = Math.max(0, displayedSum);
  const displayedOutputs = block.main.map((mainValue, index) => (
    Math.max(0, mainValue + (includeSkip ? block.skip[index]! : 0))
  ));

  function reset(): void {
    setSelectedIndex(2);
    setIncludeSkip(true);
  }

  return (
    <main className="workspace workspace--residual">
      <section className="residual-stage" aria-label="Residual path and receptive field trace">
        <div className="residual-intro">
          <div>
            <p className="eyebrow">NN08 · spatial networks</p>
            <h2>Residual-path microscope</h2>
            <p>
              Open one output into its deep local path and short identity path,
              then trace every dependency back to the original input.
            </p>
          </div>
          <div className="residual-shape-chip">5 → 5 → 5 + identity</div>
        </div>

        <section className="residual-block-panel" aria-label="Residual block forward trace">
          <div className="residual-panel-heading">
            <div>
              <p className="eyebrow">Selected output · y[{selectedIndex}]</p>
              <h2>Two routes meet at one addition</h2>
            </div>
            <strong className="residual-result">{formatNumber(displayedOutput)}</strong>
          </div>

          <div className="residual-main-path">
            <span className="residual-lane-label">main path · two local layers</span>
            <SignalRow
              label="input x"
              values={DEFAULT_RESIDUAL_INPUT}
              selectedIndex={includeSkip ? selectedIndex : undefined}
              activeIndices={trace.receptiveFieldIndices}
              annotation="receptive field highlighted"
            />
            <span className="residual-down-arrow" aria-hidden="true">↓ [1, 1, 1] · same zero pad</span>
            <SignalRow
              label="hidden h"
              values={block.hidden}
              activeIndices={trace.hiddenIndices}
              annotation={`${trace.hiddenIndices.length} values feed main[${selectedIndex}]`}
            />
            <span className="residual-down-arrow" aria-hidden="true">↓ [1, 1, 1] · same zero pad</span>
            <SignalRow
              label="main transform F(x)"
              values={block.main}
              selectedIndex={selectedIndex}
              annotation={`main[${selectedIndex}] = ${formatNumber(trace.mainOutput)}`}
            />
          </div>

          <div className={includeSkip ? "residual-skip-lane" : "residual-skip-lane residual-skip-lane--disabled"}>
            <div>
              <small>identity skip</small>
              <strong>x[{selectedIndex}] = {formatNumber(trace.skipContribution)}</strong>
            </div>
            <span aria-hidden="true">────────────→</span>
            <code>{includeSkip ? "included" : "disabled"}</code>
          </div>

          <div className="residual-addition" aria-label="Selected residual addition">
            <div>
              <small>main path</small>
              <strong>{formatNumber(trace.mainOutput)}</strong>
            </div>
            <span>+</span>
            <div>
              <small>skip path</small>
              <strong>{includeSkip ? formatNumber(trace.skipContribution) : "0"}</strong>
            </div>
            <span>=</span>
            <div>
              <small>before ReLU</small>
              <strong>{formatNumber(displayedSum)}</strong>
            </div>
            <span>→</span>
            <div className="residual-addition__output">
              <small>output</small>
              <strong>{formatNumber(displayedOutput)}</strong>
            </div>
          </div>

          <SignalRow
            label={includeSkip ? "block output ReLU(F(x) + x)" : "block output ReLU(F(x))"}
            values={displayedOutputs}
            selectedIndex={selectedIndex}
          />
        </section>

        <section className="receptive-panel" aria-label="Receptive field explorer">
          <div className="residual-panel-heading">
            <div>
              <p className="eyebrow">Receptive field · output {selectedIndex}</p>
              <h2>One output, every path back</h2>
            </div>
            <div className="field-width-badge">
              <small>in-range width</small>
              <strong>{trace.receptiveFieldIndices.length}</strong>
            </div>
          </div>

          <div className="hidden-path-grid">
            {trace.hiddenPaths.map((path) => (
              <article className="hidden-path-card" key={path.hiddenIndex}>
                <div>
                  <small>layer 2 reads</small>
                  <strong>h[{path.hiddenIndex}] = {formatNumber(path.subtotal)}</strong>
                </div>
                <code>
                  {path.inputIndices.map((inputIndex) => `x[${inputIndex}]`).join(" + ")}
                </code>
                <span>
                  {path.inputValues.map(formatNumber).join(" + ")} = {formatNumber(path.subtotal)}
                </span>
              </article>
            ))}
          </div>

          <div className="path-count-table-wrap">
            <table className="path-count-table">
              <caption>Original inputs after expanding both layers</caption>
              <thead>
                <tr>
                  <th scope="col">input</th>
                  {DEFAULT_RESIDUAL_INPUT.map((_, index) => <th scope="col" key={index}>x[{index}]</th>)}
                  <th scope="col">sum</th>
                </tr>
              </thead>
              <tbody>
                <tr>
                  <th scope="row">paths</th>
                  {trace.inputPathCounts.map((count, index) => <td key={index}>{count}</td>)}
                  <td>—</td>
                </tr>
                <tr>
                  <th scope="row">value × paths</th>
                  {trace.inputContributions.map((value, index) => <td key={index}>{formatNumber(value)}</td>)}
                  <td className="path-count-total">{formatNumber(trace.mainOutput)}</td>
                </tr>
              </tbody>
            </table>
          </div>

          <div className="receptive-summary">
            <code>
              receptive input indices = [{trace.receptiveFieldIndices.join(", ")}]
            </code>
            <span>
              Zero-valued inputs still belong to the structural field: changing
              them can change this output.
            </span>
          </div>
        </section>
      </section>

      <aside className="residual-controls" aria-label="Residual explorer controls">
        <p className="eyebrow">Choose one output</p>
        <h2>Trace controls</h2>
        <p>Move from a clipped boundary field to the five-position center field.</p>

        <div className="residual-output-buttons">
          {block.output.map((value, index) => (
            <button
              aria-label={`Select residual output ${index}`}
              className={index === selectedIndex
                ? "residual-output-button residual-output-button--active"
                : "residual-output-button"}
              key={index}
              type="button"
              onClick={() => setSelectedIndex(index)}
            >
              <small>y[{index}]</small>
              <strong>{formatNumber(value)}</strong>
            </button>
          ))}
        </div>

        <label className="residual-skip-control">
          <input
            type="checkbox"
            checked={includeSkip}
            onChange={(event) => setIncludeSkip(event.target.checked)}
          />
          <span>
            <strong>Include identity skip</strong>
            <small>Add x[i] directly to the main path.</small>
          </span>
        </label>

        <div className="button-grid">
          <button
            type="button"
            disabled={selectedIndex === 0}
            onClick={() => setSelectedIndex((index) => Math.max(0, index - 1))}
          >
            Previous output
          </button>
          <button
            type="button"
            disabled={selectedIndex === block.output.length - 1}
            onClick={() => setSelectedIndex((index) => Math.min(block.output.length - 1, index + 1))}
          >
            Next output
          </button>
          <button type="button" onClick={reset}>Reset trace</button>
        </div>

        <div className="residual-note">
          <span>What scales next?</span>
          <p>
            More layers widen the main path's field. Projection skips handle
            shape changes, but must still land on a tensor compatible with the
            addition.
          </p>
        </div>
      </aside>
    </main>
  );
}
