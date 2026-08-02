import { useEffect, useMemo, useState } from "react";
import {
  DEFAULT_CONVOLUTION_KERNEL,
  DEFAULT_CONVOLUTION_LEARNING_RATE,
  DEFAULT_CONVOLUTION_SIGNAL,
  DEFAULT_CONVOLUTION_TARGETS,
  numericalKernelGradient,
  parseNumberList,
  proposeConvolutionStep,
  traceConvolutionTraining,
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
  const [targetText, setTargetText] = useState(listText(DEFAULT_CONVOLUTION_TARGETS));
  const [learningRate, setLearningRate] = useState(DEFAULT_CONVOLUTION_LEARNING_RATE);
  const [trainingStep, setTrainingStep] = useState(0);
  const [selectedIndex, setSelectedIndex] = useState(0);
  const signal = useMemo(() => parseNumberList(signalText), [signalText]);
  const kernel = useMemo(() => parseNumberList(kernelText), [kernelText]);
  const targets = useMemo(() => parseNumberList(targetText), [targetText]);
  const error = signal === null || kernel === null
    ? "Use comma-separated finite numbers."
    : kernel.length > signal.length
      ? "The kernel must fit entirely inside the signal in valid mode."
      : null;
  const traces = useMemo(
    () => error === null ? traceValidCorrelation(signal!, kernel!) : [],
    [error, kernel, signal],
  );
  const trainingError = error !== null
    ? error
    : targets === null
      ? "Use comma-separated finite training targets."
      : targets.length !== traces.length
        ? `Valid mode produces ${traces.length} outputs, so enter ${traces.length} targets.`
        : !Number.isFinite(learningRate) || learningRate <= 0
          ? "The learning rate must be a positive number."
          : null;
  const trainingTrace = useMemo(
    () => trainingError === null
      ? traceConvolutionTraining(signal!, kernel!, targets!)
      : null,
    [kernel, signal, targets, trainingError],
  );
  const numericalGradient = useMemo(
    () => trainingError === null
      ? numericalKernelGradient(signal!, kernel!, targets!)
      : [],
    [kernel, signal, targets, trainingError],
  );
  const proposal = useMemo(
    () => trainingError === null
      ? proposeConvolutionStep(signal!, kernel!, targets!, learningRate)
      : null,
    [kernel, learningRate, signal, targets, trainingError],
  );
  const gradientCheckPasses = trainingTrace !== null
    && trainingTrace.kernelGradient.every(
      (gradient, index) => Math.abs(gradient - numericalGradient[index]!) <= 1e-7,
    );

  useEffect(() => {
    setSelectedIndex((current) => Math.min(current, Math.max(traces.length - 1, 0)));
  }, [traces.length]);

  const trace = traces[selectedIndex];
  const selectedContribution = trainingTrace?.contributions[selectedIndex];
  const activeEnd = trace === undefined ? -1 : trace.startIndex + trace.window.length;

  function reset(): void {
    setSignalText(listText(DEFAULT_CONVOLUTION_SIGNAL));
    setKernelText(listText(DEFAULT_CONVOLUTION_KERNEL));
    setTargetText(listText(DEFAULT_CONVOLUTION_TARGETS));
    setLearningRate(DEFAULT_CONVOLUTION_LEARNING_RATE);
    setTrainingStep(0);
    setSelectedIndex(0);
  }

  function applyGradientStep(): void {
    if (proposal === null) {
      return;
    }
    setKernelText(listText(proposal.nextKernel));
    setTrainingStep((step) => step + 1);
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

            <section className="training-panel" aria-label="Shared kernel gradient trace">
              <div className="training-heading">
                <div>
                  <p className="eyebrow">NN06 · backward pass</p>
                  <h2>Shared weights collect gradients</h2>
                  <p>
                    Every output sends a contribution back to each kernel weight.
                    Columns add because the same weight was reused in every window.
                  </p>
                </div>
                <div className={gradientCheckPasses
                  ? "gradient-check-badge gradient-check-badge--pass"
                  : "gradient-check-badge"}
                >
                  <small>finite difference</small>
                  <strong>{gradientCheckPasses ? "PASS" : "CHECK"}</strong>
                </div>
              </div>

              {trainingTrace === null || proposal === null || selectedContribution === undefined ? (
                <div className="convolution-error" role="alert">{trainingError}</div>
              ) : (
                <>
                  <div className="loss-flow" aria-label="Loss before and after proposed step">
                    <div>
                      <small>current MSE</small>
                      <strong>{formatNumber(trainingTrace.loss)}</strong>
                    </div>
                    <span aria-hidden="true">− η∇</span>
                    <div>
                      <small>after proposed step</small>
                      <strong>{formatNumber(proposal.nextLoss)}</strong>
                    </div>
                  </div>

                  <section className="selected-gradient-path" aria-label="Selected output gradient path">
                    <div className="mac-heading">
                      <div>
                        <p className="eyebrow">Selected path · y[{selectedIndex}]</p>
                        <h3>One output sends three contributions</h3>
                      </div>
                      <code>
                        dL/dy = 2/{trainingTrace.outputs.length} × {formatNumber(trainingTrace.errors[selectedIndex]!)}
                        {" = "}{formatNumber(selectedContribution.outputGradient)}
                      </code>
                    </div>
                    <div className="product-grid">
                      {selectedContribution.kernelGradient.map((gradient, kernelIndex) => (
                        <div className="product-card" key={kernelIndex}>
                          <small>toward k[{kernelIndex}]</small>
                          <code>
                            {formatNumber(selectedContribution.outputGradient)} × {formatNumber(selectedContribution.window[kernelIndex]!)}
                          </code>
                          <strong>{formatNumber(gradient)}</strong>
                        </div>
                      ))}
                    </div>
                  </section>

                  <div className="gradient-table-wrap">
                    <table className="gradient-table">
                      <caption>Gradient contributions from every reused position</caption>
                      <thead>
                        <tr>
                          <th scope="col">weight</th>
                          {trainingTrace.contributions.map((contribution) => (
                            <th scope="col" key={contribution.outputIndex}>
                              y[{contribution.outputIndex}]
                            </th>
                          ))}
                          <th scope="col">sum</th>
                          <th scope="col">numeric</th>
                        </tr>
                      </thead>
                      <tbody>
                        {kernel.map((_, kernelIndex) => (
                          <tr key={kernelIndex}>
                            <th scope="row">dL/dk[{kernelIndex}]</th>
                            {trainingTrace.contributions.map((contribution) => (
                              <td key={contribution.outputIndex}>
                                {formatNumber(contribution.kernelGradient[kernelIndex]!)}
                              </td>
                            ))}
                            <td className="gradient-sum">
                              {formatNumber(trainingTrace.kernelGradient[kernelIndex]!)}
                            </td>
                            <td>{formatNumber(numericalGradient[kernelIndex]!)}</td>
                          </tr>
                        ))}
                      </tbody>
                    </table>
                  </div>

                  <div className="kernel-update-grid" aria-label="Proposed kernel update">
                    {kernel.map((value, kernelIndex) => (
                      <div className="kernel-update" key={kernelIndex}>
                        <small>update k[{kernelIndex}]</small>
                        <code>
                          {formatNumber(value)} − {formatNumber(learningRate)} × {formatNumber(trainingTrace.kernelGradient[kernelIndex]!)}
                        </code>
                        <strong>{formatNumber(proposal.nextKernel[kernelIndex]!)}</strong>
                      </div>
                    ))}
                  </div>
                </>
              )}
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
        <div className="convolution-training-controls">
          <div className="history__topline">
            <span>Train shared weights</span>
            <strong>step {trainingStep}</strong>
          </div>
          <label className="field">
            <span>Training targets</span>
            <input
              aria-label="Training targets"
              value={targetText}
              onChange={(event) => setTargetText(event.target.value)}
            />
          </label>
          <label className="field">
            <span>Learning rate</span>
            <input
              aria-label="Convolution learning rate"
              min="0.0001"
              step="0.001"
              type="number"
              value={learningRate}
              onChange={(event) => setLearningRate(Number(event.target.value))}
            />
          </label>
          <button
            className="training-step-button"
            disabled={proposal === null}
            type="button"
            onClick={applyGradientStep}
          >
            Apply gradient step
          </button>
        </div>
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
            Images add a second spatial direction; channels and batches add more
            indexed loops. The same shared-gradient reduction still happens for every
            trainable filter.
          </p>
        </div>
      </aside>
    </main>
  );
}
