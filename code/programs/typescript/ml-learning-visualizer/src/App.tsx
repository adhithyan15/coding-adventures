import { useEffect, useMemo, useState } from "react";
import {
  CELSIUS_DATASET,
  loss,
  meanAbsoluteError,
  predictions,
  trainStep,
  trainSteps,
  type LossKind,
  type ModelState,
  type StepResult,
} from "./training.js";

interface HistoryPoint {
  epoch: number;
  loss: number;
  mae: number;
  weight: number;
  bias: number;
}

const CHART = {
  width: 720,
  height: 460,
  padLeft: 58,
  padRight: 22,
  padTop: 24,
  padBottom: 48,
  xMin: -50,
  xMax: 110,
  yMin: -60,
  yMax: 230,
};

const DEFAULT_MODEL: ModelState = { weight: 0.5, bias: 0.5, epoch: 0 };

function formatNumber(value: number, digits = 3): string {
  return Number.isFinite(value) ? value.toFixed(digits) : "0.000";
}

function clamp(value: number, min: number, max: number): number {
  return Math.min(max, Math.max(min, value));
}

function xScale(celsius: number): number {
  const innerWidth = CHART.width - CHART.padLeft - CHART.padRight;
  return CHART.padLeft + ((celsius - CHART.xMin) / (CHART.xMax - CHART.xMin)) * innerWidth;
}

function yScale(fahrenheit: number): number {
  const innerHeight = CHART.height - CHART.padTop - CHART.padBottom;
  return (
    CHART.padTop +
    (1 - (fahrenheit - CHART.yMin) / (CHART.yMax - CHART.yMin)) * innerHeight
  );
}

function linePath(state: ModelState): string {
  const x1 = CHART.xMin;
  const x2 = CHART.xMax;
  return `M ${xScale(x1)} ${yScale(state.weight * x1 + state.bias)} L ${xScale(x2)} ${yScale(
    state.weight * x2 + state.bias,
  )}`;
}

function historyPath(history: HistoryPoint[]): string {
  if (history.length === 0) {
    return "";
  }

  const width = 250;
  const height = 74;
  const maxLoss = Math.max(...history.map((point) => point.loss), 1);
  const startEpoch = history[0]!.epoch;
  const span = Math.max(history.at(-1)!.epoch - startEpoch, 1);

  return history
    .map((point, index) => {
      const x = ((point.epoch - startEpoch) / span) * width;
      const y = height - (clamp(point.loss / maxLoss, 0, 1) * height);
      return `${index === 0 ? "M" : "L"} ${x.toFixed(2)} ${y.toFixed(2)}`;
    })
    .join(" ");
}

function makeHistoryPoint(state: ModelState, lossKind: LossKind): HistoryPoint {
  return {
    epoch: state.epoch,
    loss: loss(CELSIUS_DATASET, state, lossKind),
    mae: meanAbsoluteError(CELSIUS_DATASET, state),
    weight: state.weight,
    bias: state.bias,
  };
}

export function App() {
  const [lossKind, setLossKind] = useState<LossKind>("mse");
  const [learningRate, setLearningRate] = useState(0.0005);
  const [initialWeight, setInitialWeight] = useState(DEFAULT_MODEL.weight);
  const [initialBias, setInitialBias] = useState(DEFAULT_MODEL.bias);
  const [model, setModel] = useState<ModelState>(DEFAULT_MODEL);
  const [history, setHistory] = useState<HistoryPoint[]>([
    makeHistoryPoint(DEFAULT_MODEL, "mse"),
  ]);
  const [lastStep, setLastStep] = useState<StepResult | null>(null);
  const [isRunning, setIsRunning] = useState(false);

  const yPred = useMemo(() => predictions(CELSIUS_DATASET, model), [model]);
  const currentLoss = useMemo(() => loss(CELSIUS_DATASET, model, lossKind), [lossKind, model]);
  const currentMae = useMemo(() => meanAbsoluteError(CELSIUS_DATASET, model), [model]);
  const samplePrediction = model.weight * 100 + model.bias;
  const idealModel: ModelState = { weight: 1.8, bias: 32, epoch: 0 };

  function recordResult(result: StepResult): void {
    setModel(result.state);
    setLastStep(result);
    setHistory((items) => [...items.slice(-159), {
      epoch: result.state.epoch,
      loss: result.loss,
      mae: result.mae,
      weight: result.state.weight,
      bias: result.state.bias,
    }]);
  }

  function step(count: number): void {
    const results = trainSteps(CELSIUS_DATASET, model, learningRate, lossKind, count);
    const last = results.at(-1);
    if (last !== undefined) {
      recordResult(last);
    }
  }

  function reset(): void {
    const next = { weight: initialWeight, bias: initialBias, epoch: 0 };
    setModel(next);
    setLastStep(null);
    setIsRunning(false);
    setHistory([makeHistoryPoint(next, lossKind)]);
  }

  useEffect(() => {
    if (!isRunning) {
      return undefined;
    }

    const id = window.setInterval(() => {
      setModel((current) => {
        const result = trainStep(CELSIUS_DATASET, current, learningRate, lossKind);
        setLastStep(result);
        setHistory((items) => [...items.slice(-159), {
          epoch: result.state.epoch,
          loss: result.loss,
          mae: result.mae,
          weight: result.state.weight,
          bias: result.state.bias,
        }]);
        return result.state;
      });
    }, 180);

    return () => window.clearInterval(id);
  }, [isRunning, learningRate, lossKind]);

  return (
    <div className="app">
      <header className="app-header">
        <div>
          <p className="eyebrow">Linear Regression</p>
          <h1>ML Learning Visualizer</h1>
        </div>
        <div className="formula">
          F = <strong>{formatNumber(model.weight)}</strong>C + <strong>{formatNumber(model.bias)}</strong>
        </div>
      </header>

      <main className="workspace">
        <section className="controls" aria-label="Training controls">
          <label className="field">
            <span>Loss</span>
            <select value={lossKind} onChange={(event) => setLossKind(event.target.value as LossKind)}>
              <option value="mse">Mean squared error</option>
              <option value="mae">Mean absolute error</option>
            </select>
          </label>

          <label className="field">
            <span>Learning rate</span>
            <input
              type="range"
              min="0.00005"
              max="0.02"
              step="0.00005"
              value={learningRate}
              onChange={(event) => setLearningRate(Number(event.target.value))}
            />
            <input
              type="number"
              min="0.00001"
              max="0.05"
              step="0.00005"
              value={learningRate}
              onChange={(event) => setLearningRate(Number(event.target.value))}
            />
          </label>

          <div className="field-grid">
            <label className="field">
              <span>Initial weight</span>
              <input
                type="number"
                step="0.1"
                value={initialWeight}
                onChange={(event) => setInitialWeight(Number(event.target.value))}
              />
            </label>
            <label className="field">
              <span>Initial bias</span>
              <input
                type="number"
                step="0.5"
                value={initialBias}
                onChange={(event) => setInitialBias(Number(event.target.value))}
              />
            </label>
          </div>

          <div className="button-grid">
            <button type="button" onClick={() => step(1)}>Step</button>
            <button type="button" onClick={() => step(25)}>Step 25</button>
            <button type="button" onClick={() => setIsRunning((value) => !value)}>
              {isRunning ? "Pause" : "Run"}
            </button>
            <button type="button" onClick={reset}>Reset</button>
          </div>
        </section>

        <section className="chart-panel" aria-label="Training chart">
          <svg viewBox={`0 0 ${CHART.width} ${CHART.height}`} role="img" aria-label="Celsius to Fahrenheit fit chart">
            <rect className="plot-bg" x={CHART.padLeft} y={CHART.padTop} width={CHART.width - CHART.padLeft - CHART.padRight} height={CHART.height - CHART.padTop - CHART.padBottom} />

            {[-40, 0, 40, 80].map((tick) => (
              <g key={`x-${tick}`}>
                <line className="grid-line" x1={xScale(tick)} x2={xScale(tick)} y1={CHART.padTop} y2={CHART.height - CHART.padBottom} />
                <text className="axis-label" x={xScale(tick)} y={CHART.height - 18}>{tick}</text>
              </g>
            ))}
            {[-40, 32, 100, 180, 212].map((tick) => (
              <g key={`y-${tick}`}>
                <line className="grid-line" x1={CHART.padLeft} x2={CHART.width - CHART.padRight} y1={yScale(tick)} y2={yScale(tick)} />
                <text className="axis-label axis-label--y" x={CHART.padLeft - 10} y={yScale(tick) + 4}>{tick}</text>
              </g>
            ))}

            <line className="axis" x1={CHART.padLeft} x2={CHART.width - CHART.padRight} y1={yScale(0)} y2={yScale(0)} />
            <line className="axis" x1={xScale(0)} x2={xScale(0)} y1={CHART.padTop} y2={CHART.height - CHART.padBottom} />
            <path className="ideal-line" d={linePath(idealModel)} />
            <path className="model-line" d={linePath(model)} />

            {CELSIUS_DATASET.map((point, index) => {
              const px = xScale(point.celsius);
              const trueY = yScale(point.fahrenheit);
              const predY = yScale(yPred[index]!);
              return (
                <g key={point.celsius}>
                  <line className="error-line" x1={px} x2={px} y1={trueY} y2={predY} />
                  <circle className="truth-point" cx={px} cy={trueY} r="6" />
                  <circle className="prediction-point" cx={px} cy={predY} r="5" />
                </g>
              );
            })}

            <text className="axis-title" x={CHART.width / 2} y={CHART.height - 4}>Celsius</text>
            <text className="axis-title axis-title--y" x="18" y={CHART.height / 2}>Fahrenheit</text>
          </svg>

          <div className="legend" aria-label="Chart legend">
            <span><i className="legend-dot legend-dot--truth" />Actual</span>
            <span><i className="legend-dot legend-dot--prediction" />Prediction</span>
            <span><i className="legend-line legend-line--model" />Current line</span>
            <span><i className="legend-line legend-line--ideal" />Exact formula</span>
          </div>
        </section>

        <aside className="metrics" aria-label="Training metrics">
          <div className="metric">
            <span>Epoch</span>
            <strong>{model.epoch}</strong>
          </div>
          <div className="metric">
            <span>Loss</span>
            <strong>{formatNumber(currentLoss, 4)}</strong>
          </div>
          <div className="metric">
            <span>Average error</span>
            <strong>{formatNumber(currentMae, 3)} F</strong>
          </div>
          <div className="metric">
            <span>100 C prediction</span>
            <strong>{formatNumber(samplePrediction, 2)} F</strong>
          </div>

          <div className="history">
            <div className="history__topline">
              <span>Loss history</span>
              <strong>{history.length} points</strong>
            </div>
            <svg viewBox="0 0 250 74" role="img" aria-label="Loss history sparkline">
              <path className="history-grid" d="M 0 37 L 250 37" />
              <path className="history-line" d={historyPath(history)} />
            </svg>
          </div>

          <div className="gradients">
            <span>Last gradient</span>
            <code>w {lastStep === null ? "0.000" : formatNumber(lastStep.gradientWeight, 3)}</code>
            <code>b {lastStep === null ? "0.000" : formatNumber(lastStep.gradientBias, 3)}</code>
          </div>
        </aside>
      </main>
    </div>
  );
}
