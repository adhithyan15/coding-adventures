import { useMemo, useState } from "react";
import {
  DEFAULT_OPTIMIZATION_STATE,
  OPTIMIZATION_DATASET,
  OPTIMUM_PARAMETERS,
  checkGradient,
  meanSquaredError,
  optimizationStep,
  runOptimization,
  sampleLossLandscape,
  type BatchStrategy,
  type OptimizationState,
  type OptimizationTracePoint,
} from "./optimization-lab.js";

const STRATEGIES: readonly { kind: BatchStrategy; label: string; summary: string }[] = [
  { kind: "stochastic", label: "SGD / 1 row", summary: "Noisy, frequent updates" },
  { kind: "mini-batch", label: "Mini-batch / 2 rows", summary: "A compromise between noise and stability" },
  { kind: "full-batch", label: "Full batch / 4 rows", summary: "Stable average gradient" },
];

const LANDSCAPE = {
  width: 720,
  height: 430,
  left: 68,
  right: 28,
  top: 24,
  bottom: 58,
  weightRange: [-1, 3.5] as const,
  biasRange: [-1, 3] as const,
  resolution: 25,
};

function formatNumber(value: number, digits = 5): string {
  if (Math.abs(value) < 1e-12) {
    return "0";
  }
  if (Math.abs(value) >= 1000 || Math.abs(value) < 0.0001) {
    return value.toExponential(3);
  }
  return Number(value.toFixed(digits)).toString();
}

function scaleX(weight: number): number {
  const innerWidth = LANDSCAPE.width - LANDSCAPE.left - LANDSCAPE.right;
  return LANDSCAPE.left
    + ((weight - LANDSCAPE.weightRange[0])
      / (LANDSCAPE.weightRange[1] - LANDSCAPE.weightRange[0])) * innerWidth;
}

function scaleY(bias: number): number {
  const innerHeight = LANDSCAPE.height - LANDSCAPE.top - LANDSCAPE.bottom;
  return LANDSCAPE.top
    + (1 - (bias - LANDSCAPE.biasRange[0])
      / (LANDSCAPE.biasRange[1] - LANDSCAPE.biasRange[0])) * innerHeight;
}

function tracePath(trace: readonly OptimizationTracePoint[], maxLogLoss: number): string {
  if (trace.length === 0) {
    return "";
  }
  const width = 590;
  const height = 138;
  const stepSpan = Math.max(trace.length - 1, 1);
  return trace.map((point, index) => {
    const x = (index / stepSpan) * width;
    const y = height - (Math.log1p(point.loss) / maxLogLoss) * height;
    return `${index === 0 ? "M" : "L"} ${x.toFixed(2)} ${y.toFixed(2)}`;
  }).join(" ");
}

function numberValue(value: string, fallback: number): number {
  const parsed = Number(value);
  return Number.isFinite(parsed) ? parsed : fallback;
}

export function OptimizationWorkbench() {
  const [state, setState] = useState<OptimizationState>(DEFAULT_OPTIMIZATION_STATE);
  const [epsilon, setEpsilon] = useState(0.00001);
  const [learningRate, setLearningRate] = useState(0.05);
  const [stepCount, setStepCount] = useState(20);
  const gradientCheck = useMemo(
    () => checkGradient(OPTIMIZATION_DATASET, state, epsilon),
    [epsilon, state],
  );
  const landscape = useMemo(
    () => sampleLossLandscape(
      OPTIMIZATION_DATASET,
      LANDSCAPE.weightRange,
      LANDSCAPE.biasRange,
      LANDSCAPE.resolution,
    ),
    [],
  );
  const maxLogLandscapeLoss = useMemo(
    () => Math.max(...landscape.map((point) => Math.log1p(point.loss)), 1),
    [landscape],
  );
  const trajectories = useMemo(
    () => STRATEGIES.map((strategy) => ({
      ...strategy,
      trace: runOptimization(strategy.kind, stepCount, learningRate, state),
    })),
    [learningRate, state, stepCount],
  );
  const maxLogTraceLoss = Math.max(
    ...trajectories.flatMap((trajectory) => trajectory.trace.map((point) => Math.log1p(point.loss))),
    1,
  );
  const oneStep = optimizationStep(OPTIMIZATION_DATASET, state, learningRate, "full-batch");
  const cellWidth = (LANDSCAPE.width - LANDSCAPE.left - LANDSCAPE.right) / LANDSCAPE.resolution;
  const cellHeight = (LANDSCAPE.height - LANDSCAPE.top - LANDSCAPE.bottom) / LANDSCAPE.resolution;

  function updateState(field: "weight" | "bias", value: string): void {
    setState((current) => ({ ...current, [field]: numberValue(value, current[field]), step: 0 }));
  }

  function reset(): void {
    setState(DEFAULT_OPTIMIZATION_STATE);
    setEpsilon(0.00001);
    setLearningRate(0.05);
    setStepCount(20);
  }

  return (
    <main className="workspace workspace--optimization">
      <section className="optimization-stage" aria-label="Optimization microscope">
        <div className="lab-intro">
          <div>
            <p className="eyebrow">Slope / check / step size / batch noise</p>
            <h2>Optimization microscope</h2>
            <p>See the loss surface, verify the gradient independently, and compare three ways to choose training rows.</p>
          </div>
          <div className="lab-chip">MSE {formatNumber(meanSquaredError(OPTIMIZATION_DATASET, state), 4)}</div>
        </div>

        <section className="landscape-panel" aria-label="Loss landscape">
          <div className="panel-heading">
            <div>
              <p className="eyebrow">Every location is one model</p>
              <h2>Loss landscape</h2>
            </div>
            <span>Darker = larger loss</span>
          </div>
          <svg
            className="landscape-chart"
            viewBox={`0 0 ${LANDSCAPE.width} ${LANDSCAPE.height}`}
            role="img"
            aria-label={`Mean squared error by weight and bias. Current weight ${formatNumber(state.weight)} and bias ${formatNumber(state.bias)}.`}
          >
            <title>Loss landscape for a four-point linear regression problem</title>
            {landscape.map((point) => (
              <rect
                className="landscape-cell"
                key={`${point.row}-${point.column}`}
                x={LANDSCAPE.left + point.column * cellWidth}
                y={LANDSCAPE.top + (LANDSCAPE.resolution - 1 - point.row) * cellHeight}
                width={cellWidth + 0.4}
                height={cellHeight + 0.4}
                style={{ opacity: 0.08 + 0.78 * (Math.log1p(point.loss) / maxLogLandscapeLoss) }}
              />
            ))}
            <line
              className="gradient-arrow"
              x1={scaleX(state.weight)}
              y1={scaleY(state.bias)}
              x2={scaleX(oneStep.weight)}
              y2={scaleY(oneStep.bias)}
              markerEnd="url(#gradient-arrow-head)"
            />
            <defs>
              <marker id="gradient-arrow-head" markerWidth="8" markerHeight="8" refX="5" refY="3" orient="auto">
                <path d="M 0 0 L 6 3 L 0 6 z" className="gradient-arrow-head" />
              </marker>
            </defs>
            <circle className="optimum-point" cx={scaleX(OPTIMUM_PARAMETERS.weight)} cy={scaleY(OPTIMUM_PARAMETERS.bias)} r="8" />
            <text className="landscape-label" x={scaleX(OPTIMUM_PARAMETERS.weight) + 12} y={scaleY(OPTIMUM_PARAMETERS.bias) - 10}>minimum (2, 1)</text>
            <circle className="current-parameter-point" cx={scaleX(state.weight)} cy={scaleY(state.bias)} r="9" />
            <text className="landscape-label" x={scaleX(state.weight) + 12} y={scaleY(state.bias) + 22}>current model</text>
            <text className="axis-title" x={LANDSCAPE.width / 2} y={LANDSCAPE.height - 10}>weight w</text>
            <text className="axis-title axis-title--optimization-y" x="18" y={LANDSCAPE.height / 2}>bias b</text>
          </svg>
          <div className="landscape-equation">
            <code>w' = {formatNumber(state.weight)} - {formatNumber(learningRate)} x ({formatNumber(gradientCheck.analytical.weight)}) = {formatNumber(oneStep.weight)}</code>
            <code>b' = {formatNumber(state.bias)} - {formatNumber(learningRate)} x ({formatNumber(gradientCheck.analytical.bias)}) = {formatNumber(oneStep.bias)}</code>
          </div>
        </section>

        <section className="gradient-check-panel" aria-label="Finite-difference gradient check">
          <div className="panel-heading">
            <div>
              <p className="eyebrow">Backpropagation gets an independent audit</p>
              <h2>Finite-difference gradient check</h2>
            </div>
            <span className={gradientCheck.passes ? "check-status check-status--pass" : "check-status check-status--fail"}>
              {gradientCheck.passes ? "PASS" : "CHECK EPSILON"}
            </span>
          </div>
          <div className="gradient-check-grid" role="table" aria-label="Gradient comparison">
            <span role="columnheader">Parameter</span>
            <span role="columnheader">Backprop</span>
            <span role="columnheader">Finite difference</span>
            <span role="columnheader">Absolute error</span>
            <strong role="cell">weight</strong>
            <code role="cell">{formatNumber(gradientCheck.analytical.weight)}</code>
            <code role="cell">{formatNumber(gradientCheck.numerical.weight)}</code>
            <code role="cell">{formatNumber(gradientCheck.absoluteError.weight)}</code>
            <strong role="cell">bias</strong>
            <code role="cell">{formatNumber(gradientCheck.analytical.bias)}</code>
            <code role="cell">{formatNumber(gradientCheck.numerical.bias)}</code>
            <code role="cell">{formatNumber(gradientCheck.absoluteError.bias)}</code>
          </div>
          <p>Finite differences nudge one parameter by +/- epsilon and estimate the slope without using backpropagation.</p>
        </section>

        <section className="batch-comparison-panel" aria-label="Batch strategy comparison">
          <div className="panel-heading">
            <div>
              <p className="eyebrow">Same model / same data / different row selection</p>
              <h2>Batch versus stochastic updates</h2>
            </div>
            <span>{stepCount} updates</span>
          </div>
          <svg className="batch-chart" viewBox="0 0 650 175" role="img" aria-label="Loss trajectories for stochastic, mini-batch, and full-batch gradient descent">
            <line className="batch-grid" x1="42" x2="632" y1="148" y2="148" />
            <line className="batch-grid" x1="42" x2="42" y1="10" y2="148" />
            <g transform="translate(42 10)">
              {trajectories.map((trajectory) => (
                <path
                  key={trajectory.kind}
                  className={`batch-line batch-line--${trajectory.kind}`}
                  d={tracePath(trajectory.trace, maxLogTraceLoss)}
                />
              ))}
            </g>
            <text className="batch-axis-label" x="337" y="172">update</text>
            <text className="batch-axis-label batch-axis-label--y" x="12" y="82">log loss</text>
          </svg>
          <div className="strategy-grid">
            {trajectories.map((trajectory) => {
              const final = trajectory.trace[trajectory.trace.length - 1]!;
              return (
                <div className={`strategy-summary strategy-summary--${trajectory.kind}`} key={trajectory.kind}>
                  <strong>{trajectory.label}</strong>
                  <span>{trajectory.summary}</span>
                  <code>loss {formatNumber(final.loss, 4)}</code>
                  <small>w {formatNumber(final.weight, 3)} / b {formatNumber(final.bias, 3)}</small>
                </div>
              );
            })}
          </div>
        </section>
      </section>

      <aside className="controls optimization-controls" aria-label="Optimization controls">
        <div className="lesson">
          <span>Try this</span>
          <p>Move the model away from the minimum, then increase the learning rate until one or more trajectories overshoot.</p>
        </div>
        <div className="field-grid">
          <label className="field">
            <span>Weight w</span>
            <input aria-label="Optimization weight" type="number" step="0.1" value={state.weight} onChange={(event) => updateState("weight", event.target.value)} />
          </label>
          <label className="field">
            <span>Bias b</span>
            <input aria-label="Optimization bias" type="number" step="0.1" value={state.bias} onChange={(event) => updateState("bias", event.target.value)} />
          </label>
        </div>
        <label className="field">
          <span>Learning rate</span>
          <input aria-label="Optimization learning rate" type="range" min="0.005" max="0.3" step="0.005" value={learningRate} onChange={(event) => setLearningRate(Number(event.target.value))} />
          <input type="number" min="0.005" max="0.3" step="0.005" value={learningRate} onChange={(event) => setLearningRate(numberValue(event.target.value, learningRate))} />
        </label>
        <label className="field">
          <span>Comparison updates</span>
          <input aria-label="Comparison updates" type="range" min="1" max="80" step="1" value={stepCount} onChange={(event) => setStepCount(Number(event.target.value))} />
          <strong>{stepCount}</strong>
        </label>
        <label className="field">
          <span>Finite-difference epsilon</span>
          <select aria-label="Finite-difference epsilon" value={epsilon} onChange={(event) => setEpsilon(Number(event.target.value))}>
            <option value="0.01">1e-2</option>
            <option value="0.001">1e-3</option>
            <option value="0.0001">1e-4</option>
            <option value="0.00001">1e-5</option>
            <option value="0.000001">1e-6</option>
            <option value="1e-8">1e-8</option>
          </select>
        </label>
        <div className="metric">
          <span>Maximum relative gradient error</span>
          <strong>{formatNumber(gradientCheck.maximumRelativeError)}</strong>
        </div>
        <button type="button" onClick={reset}>Reset optimization lab</button>
      </aside>
    </main>
  );
}
