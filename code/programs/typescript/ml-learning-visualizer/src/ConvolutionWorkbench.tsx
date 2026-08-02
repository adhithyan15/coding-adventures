import { useEffect, useMemo, useState } from "react";
import {
  DEFAULT_CONVOLUTION_KERNEL,
  DEFAULT_CONVOLUTION_SIGNAL,
  parseNumberList,
  traceValidCorrelation,
} from "./convolution-lab.js";

function formatNumber(value: number): string {
  if (Math.abs(value) < 1e-12) {
    return "0";
  }
  return Number(value.toFixed(4)).toString();
}

function listText(values: readonly number[]): string {
  return values.join(", ");
}

export function ConvolutionWorkbench() {
  const [signalText, setSignalText] = useState(listText(DEFAULT_CONVOLUTION_SIGNAL));
  const [kernelText, setKernelText] = useState(listText(DEFAULT_CONVOLUTION_KERNEL));
  const [selectedIndex, setSelectedIndex] = useState(0);
  const signal = useMemo(() => parseNumberList(signalText), [signalText]);
  const kernel = useMemo(() => parseNumberList(kernelText), [kernelText]);
  const error = signal === null || kernel === null
    ? "Use comma-separated finite numbers."
    : kernel.length > signal.length
      ? "The kernel must fit entirely inside the signal in valid mode."
      : null;
  const traces = useMemo(
    () => error === null ? traceValidCorrelation(signal!, kernel!) : [],
    [error, kernel, signal],
  );

  useEffect(() => {
    setSelectedIndex((current) => Math.min(current, Math.max(traces.length - 1, 0)));
  }, [traces.length]);

  const trace = traces[selectedIndex];
  const activeEnd = trace === undefined ? -1 : trace.startIndex + trace.window.length;

  function reset(): void {
    setSignalText(listText(DEFAULT_CONVOLUTION_SIGNAL));
    setKernelText(listText(DEFAULT_CONVOLUTION_KERNEL));
    setSelectedIndex(0);
  }

  return (
    <main className="workspace workspace--convolution">
      <section className="convolution-stage" aria-label="Sliding kernel trace">
        <div className="convolution-intro">
          <div>
            <p className="eyebrow">NN05 · spatial networks</p>
            <h2>Sliding-kernel microscope</h2>
            <p>
              One small detector reuses the same weights at every position. Select an
              output to expose the exact window, products, and running sum that made it.
            </p>
          </div>
          <div className="convolution-mode-chip">valid · stride 1 · no flip</div>
        </div>

        {trace === undefined || signal === null || kernel === null ? (
          <div className="convolution-error" role="alert">{error}</div>
        ) : (
          <>
            <section className="kernel-slide" aria-label="Kernel over signal">
              <div className="array-label">
                <span>signal</span>
                <code>{signal.length} values</code>
              </div>
              <div
                className="signal-array"
                style={{ gridTemplateColumns: `repeat(${signal.length}, minmax(48px, 1fr))` }}
              >
                {signal.map((value, index) => (
                  <div
                    className={index >= trace.startIndex && index < activeEnd
                      ? "signal-cell signal-cell--active"
                      : "signal-cell"}
                    key={`${index}-${value}`}
                  >
                    <small>x[{index}]</small>
                    <strong>{formatNumber(value)}</strong>
                  </div>
                ))}
              </div>

              <div className="array-label array-label--kernel">
                <span>shared kernel</span>
                <code>starts at x[{trace.startIndex}]</code>
              </div>
              <div
                className="kernel-track"
                style={{ gridTemplateColumns: `repeat(${signal.length}, minmax(48px, 1fr))` }}
              >
                <div
                  className="kernel-window"
                  style={{
                    gridColumn: `${trace.startIndex + 1} / span ${kernel.length}`,
                    gridTemplateColumns: `repeat(${kernel.length}, minmax(48px, 1fr))`,
                  }}
                >
                  {kernel.map((value, index) => (
                    <div className="kernel-cell" key={`${index}-${value}`}>
                      <small>k[{index}]</small>
                      <strong>{formatNumber(value)}</strong>
                    </div>
                  ))}
                </div>
              </div>
            </section>

            <section className="mac-panel" aria-label="Multiply accumulate trace">
              <div className="mac-heading">
                <div>
                  <p className="eyebrow">Output y[{trace.outputIndex}]</p>
                  <h2>Multiply, then accumulate</h2>
                </div>
                <strong className="mac-result">{formatNumber(trace.output)}</strong>
              </div>
              <div className="product-grid">
                {trace.products.map((product, index) => (
                  <div className="product-card" key={index}>
                    <small>term {index + 1}</small>
                    <code>
                      {formatNumber(trace.window[index]!)} × {formatNumber(kernel[index]!)}
                    </code>
                    <strong>{formatNumber(product)}</strong>
                  </div>
                ))}
              </div>
              <div className="accumulator-strip" aria-label="Running accumulator">
                {trace.accumulator.map((value, index) => (
                  <div className="accumulator-step" key={index}>
                    <small>{index === 0 ? "start" : `after term ${index}`}</small>
                    <strong>{formatNumber(value)}</strong>
                  </div>
                ))}
              </div>
              <code className="expanded-equation">
                {trace.window.map((value, index) => (
                  `${formatNumber(value)}×${formatNumber(kernel[index]!)}`
                )).join(" + ")} = {formatNumber(trace.output)}
              </code>
            </section>

            <section className="output-strip" aria-label="Feature map outputs">
              <div className="array-label">
                <span>feature map</span>
                <code>{signal.length} - {kernel.length} + 1 = {traces.length}</code>
              </div>
              <div className="output-buttons">
                {traces.map((position) => (
                  <button
                    aria-label={`Select output ${position.outputIndex}`}
                    className={position.outputIndex === selectedIndex
                      ? "output-button output-button--active"
                      : "output-button"}
                    key={position.outputIndex}
                    type="button"
                    onClick={() => setSelectedIndex(position.outputIndex)}
                  >
                    <small>y[{position.outputIndex}]</small>
                    <strong>{formatNumber(position.output)}</strong>
                  </button>
                ))}
              </div>
            </section>
          </>
        )}
      </section>

      <aside className="convolution-controls" aria-label="Convolution controls">
        <div>
          <p className="eyebrow">Change the arithmetic</p>
          <h2>Signal and detector</h2>
          <p>Use an asymmetric kernel: reversing it should change the outputs.</p>
        </div>
        <label className="field">
          <span>Input signal</span>
          <input
            aria-label="Input signal"
            value={signalText}
            onChange={(event) => setSignalText(event.target.value)}
          />
        </label>
        <label className="field">
          <span>Kernel weights</span>
          <input
            aria-label="Kernel weights"
            value={kernelText}
            onChange={(event) => setKernelText(event.target.value)}
          />
        </label>
        <div className="button-grid">
          <button
            type="button"
            disabled={selectedIndex === 0}
            onClick={() => setSelectedIndex((index) => Math.max(index - 1, 0))}
          >
            Previous
          </button>
          <button
            type="button"
            disabled={selectedIndex >= traces.length - 1}
            onClick={() => setSelectedIndex((index) => Math.min(index + 1, traces.length - 1))}
          >
            Next
          </button>
          <button type="button" onClick={reset}>Reset fixture</button>
        </div>
        <div className="convolution-note">
          <span>Why “no flip”?</span>
          <p>
            Neural libraries usually say convolution while computing
            cross-correlation. Kernel k[0] multiplies the leftmost value in every
            window. The NN05 fixture makes this convention testable across languages.
          </p>
        </div>
        <div className="convolution-note">
          <span>What scales next?</span>
          <p>
            A trainable kernel learns these shared weights. Images add a second spatial
            direction; channels and batches add more indexed loops, not new magic.
          </p>
        </div>
      </aside>
    </main>
  );
}
