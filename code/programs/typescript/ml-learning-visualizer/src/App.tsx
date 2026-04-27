import { useEffect, useMemo, useState } from "react";
import { ACTIVATIONS, activationByKind, activate, type ActivationKind } from "./activation.js";
import { LAB_CATEGORIES, LABS, type LabDefinition } from "./labs.js";
import {
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

interface ChartFrame {
  width: number;
  height: number;
  padLeft: number;
  padRight: number;
  padTop: number;
  padBottom: number;
  xMin: number;
  xMax: number;
  yMin: number;
  yMax: number;
}

const CHART_BASE = {
  width: 720,
  height: 460,
  padLeft: 64,
  padRight: 24,
  padTop: 26,
  padBottom: 52,
};

const GROUP_COLORS = ["#237a57", "#2563eb", "#c2413b", "#b7791f", "#6d5bd0"];

function formatNumber(value: number, digits = 3): string {
  if (!Number.isFinite(value)) {
    return "0";
  }

  if (Math.abs(value) >= 1000) {
    return value.toFixed(0);
  }

  if (Math.abs(value) < 0.01 && value !== 0) {
    return value.toExponential(2);
  }

  return value.toFixed(digits);
}

function clamp(value: number, min: number, max: number): number {
  return Math.min(max, Math.max(min, value));
}

function makeChart(lab: LabDefinition, model: ModelState): ChartFrame {
  const xValues = lab.points.map((point) => point.x);
  const yValues = [
    ...lab.points.map((point) => point.y),
    ...predictions(lab.points, model),
    lab.idealModel.weight * Math.min(...xValues) + lab.idealModel.bias,
    lab.idealModel.weight * Math.max(...xValues) + lab.idealModel.bias,
  ];
  const xMinRaw = Math.min(...xValues);
  const xMaxRaw = Math.max(...xValues);
  const yMinRaw = Math.min(...yValues);
  const yMaxRaw = Math.max(...yValues);
  const xPad = Math.max((xMaxRaw - xMinRaw) * 0.12, 1);
  const yPad = Math.max((yMaxRaw - yMinRaw) * 0.16, 1);

  return {
    ...CHART_BASE,
    xMin: xMinRaw - xPad,
    xMax: xMaxRaw + xPad,
    yMin: yMinRaw - yPad,
    yMax: yMaxRaw + yPad,
  };
}

function xScale(value: number, chart: ChartFrame): number {
  const innerWidth = chart.width - chart.padLeft - chart.padRight;
  return chart.padLeft + ((value - chart.xMin) / (chart.xMax - chart.xMin)) * innerWidth;
}

function yScale(value: number, chart: ChartFrame): number {
  const innerHeight = chart.height - chart.padTop - chart.padBottom;
  return chart.padTop + (1 - (value - chart.yMin) / (chart.yMax - chart.yMin)) * innerHeight;
}

function linePath(state: ModelState, chart: ChartFrame): string {
  const x1 = chart.xMin;
  const x2 = chart.xMax;
  return `M ${xScale(x1, chart)} ${yScale(state.weight * x1 + state.bias, chart)} L ${xScale(
    x2,
    chart,
  )} ${yScale(state.weight * x2 + state.bias, chart)}`;
}

function historyPath(history: HistoryPoint[]): string {
  if (history.length === 0) {
    return "";
  }

  const width = 250;
  const height = 74;
  const maxLoss = Math.max(...history.map((point) => point.loss), 1);
  const startEpoch = history[0]!.epoch;
  const span = Math.max(history[history.length - 1]!.epoch - startEpoch, 1);

  return history
    .map((point, index) => {
      const x = ((point.epoch - startEpoch) / span) * width;
      const y = height - clamp(point.loss / maxLoss, 0, 1) * height;
      return `${index === 0 ? "M" : "L"} ${x.toFixed(2)} ${y.toFixed(2)}`;
    })
    .join(" ");
}

function activationPath(kind: ActivationKind): string {
  const width = 250;
  const height = 82;
  const samples = Array.from({ length: 81 }, (_, index) => -4 + index * 0.1);
  const outputs = samples.map((value) => activate(value, kind));
  const minY = Math.min(...outputs, -1);
  const maxY = Math.max(...outputs, 1);

  return samples
    .map((input, index) => {
      const x = ((input + 4) / 8) * width;
      const y = height - ((outputs[index]! - minY) / (maxY - minY)) * height;
      return `${index === 0 ? "M" : "L"} ${x.toFixed(2)} ${y.toFixed(2)}`;
    })
    .join(" ");
}

function makeHistoryPoint(lab: LabDefinition, state: ModelState, lossKind: LossKind): HistoryPoint {
  return {
    epoch: state.epoch,
    loss: loss(lab.points, state, lossKind),
    mae: meanAbsoluteError(lab.points, state),
    weight: state.weight,
    bias: state.bias,
  };
}

function groupColor(group: string | undefined, groups: string[]): string {
  if (group === undefined) {
    return GROUP_COLORS[0]!;
  }

  return GROUP_COLORS[Math.max(groups.indexOf(group), 0) % GROUP_COLORS.length]!;
}

export function App() {
  const [selectedLabId, setSelectedLabId] = useState(LABS[0]!.id);
  const selectedLab = LABS.find((lab) => lab.id === selectedLabId) ?? LABS[0]!;
  const [activationKind, setActivationKind] = useState<ActivationKind>("linear");
  const [lossKind, setLossKind] = useState<LossKind>(selectedLab.defaultLoss);
  const [learningRate, setLearningRate] = useState(selectedLab.defaultLearningRate);
  const [initialWeight, setInitialWeight] = useState(selectedLab.initialModel.weight);
  const [initialBias, setInitialBias] = useState(selectedLab.initialModel.bias);
  const [model, setModel] = useState<ModelState>(selectedLab.initialModel);
  const [history, setHistory] = useState<HistoryPoint[]>([
    makeHistoryPoint(selectedLab, selectedLab.initialModel, selectedLab.defaultLoss),
  ]);
  const [lastStep, setLastStep] = useState<StepResult | null>(null);
  const [isRunning, setIsRunning] = useState(false);

  useEffect(() => {
    setLossKind(selectedLab.defaultLoss);
    setLearningRate(selectedLab.defaultLearningRate);
    setInitialWeight(selectedLab.initialModel.weight);
    setInitialBias(selectedLab.initialModel.bias);
    setModel(selectedLab.initialModel);
    setLastStep(null);
    setIsRunning(false);
    setHistory([makeHistoryPoint(selectedLab, selectedLab.initialModel, selectedLab.defaultLoss)]);
  }, [selectedLab]);

  const yPred = useMemo(() => predictions(selectedLab.points, model), [model, selectedLab.points]);
  const chart = useMemo(() => makeChart(selectedLab, model), [model, selectedLab]);
  const currentLoss = useMemo(
    () => loss(selectedLab.points, model, lossKind),
    [lossKind, model, selectedLab.points],
  );
  const currentMae = useMemo(() => meanAbsoluteError(selectedLab.points, model), [model, selectedLab.points]);
  const selectedActivation = useMemo(() => activationByKind(activationKind), [activationKind]);
  const groups = useMemo(
    () => Array.from(new Set(selectedLab.points.map((point) => point.group).filter((group): group is string => group !== undefined))),
    [selectedLab.points],
  );
  const categories = useMemo(
    () => LAB_CATEGORIES.map((category) => ({
      category,
      labs: LABS.filter((lab) => lab.category === category),
    })),
    [],
  );

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
    const results = trainSteps(selectedLab.points, model, learningRate, lossKind, count);
    const last = results[results.length - 1];
    if (last !== undefined) {
      recordResult(last);
    }
  }

  function reset(): void {
    const next = { weight: initialWeight, bias: initialBias, epoch: 0 };
    setModel(next);
    setLastStep(null);
    setIsRunning(false);
    setHistory([makeHistoryPoint(selectedLab, next, lossKind)]);
  }

  useEffect(() => {
    if (!isRunning) {
      return undefined;
    }

    const id = window.setInterval(() => {
      setModel((current) => {
        const result = trainStep(selectedLab.points, current, learningRate, lossKind);
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
  }, [isRunning, learningRate, lossKind, selectedLab.points]);

  return (
    <div className="app">
      <header className="app-header">
        <div>
          <p className="eyebrow">100-lab foundation</p>
          <h1>ML Learning Lab</h1>
        </div>
        <div className="formula">
          y = <strong>{formatNumber(model.weight)}</strong>x + <strong>{formatNumber(model.bias)}</strong>
        </div>
      </header>

      <main className="workspace workspace--lab">
        <nav className="lab-rail" aria-label="ML lab examples">
          <div className="rail-summary">
            <strong>{LABS.length}</strong>
            <span>examples</span>
          </div>
          {categories.map(({ category, labs }) => (
            <section className="lab-group" key={category}>
              <h2>{category}</h2>
              <div className="lab-list">
                {labs.map((lab) => (
                  <button
                    className={lab.id === selectedLab.id ? "lab-button lab-button--active" : "lab-button"}
                    key={lab.id}
                    type="button"
                    onClick={() => setSelectedLabId(lab.id)}
                  >
                    <span>{lab.title}</span>
                    <small>{lab.source.kind === "local-csv" ? "CSV" : "Synthetic"}</small>
                  </button>
                ))}
              </div>
            </section>
          ))}
        </nav>

        <section className="lab-stage" aria-label="Selected lab">
          <div className="lab-intro">
            <div>
              <p className="eyebrow">{selectedLab.category}</p>
              <h2>{selectedLab.title}</h2>
              <p>{selectedLab.summary}</p>
            </div>
            <div className="lab-chip">{selectedLab.points.length} points</div>
          </div>

          <section className="chart-panel" aria-label="Training chart">
            <svg viewBox={`0 0 ${chart.width} ${chart.height}`} role="img" aria-label={`${selectedLab.title} fit chart`}>
              <rect
                className="plot-bg"
                x={chart.padLeft}
                y={chart.padTop}
                width={chart.width - chart.padLeft - chart.padRight}
                height={chart.height - chart.padTop - chart.padBottom}
              />

              {[0, 0.25, 0.5, 0.75, 1].map((ratio) => {
                const xTick = chart.xMin + (chart.xMax - chart.xMin) * ratio;
                const yTick = chart.yMin + (chart.yMax - chart.yMin) * ratio;
                return (
                  <g key={ratio}>
                    <line className="grid-line" x1={xScale(xTick, chart)} x2={xScale(xTick, chart)} y1={chart.padTop} y2={chart.height - chart.padBottom} />
                    <text className="axis-label" x={xScale(xTick, chart)} y={chart.height - 20}>{formatNumber(xTick, 1)}</text>
                    <line className="grid-line" x1={chart.padLeft} x2={chart.width - chart.padRight} y1={yScale(yTick, chart)} y2={yScale(yTick, chart)} />
                    <text className="axis-label axis-label--y" x={chart.padLeft - 10} y={yScale(yTick, chart) + 4}>{formatNumber(yTick, 1)}</text>
                  </g>
                );
              })}

              <path className="ideal-line" d={linePath(selectedLab.idealModel, chart)} />
              <path className="model-line" d={linePath(model, chart)} />

              {selectedLab.points.map((point, index) => {
                const px = xScale(point.x, chart);
                const trueY = yScale(point.y, chart);
                const predY = yScale(yPred[index]!, chart);
                const color = groupColor(point.group, groups);
                return (
                  <g key={`${point.x}-${point.y}-${index}`}>
                    <line className="error-line" x1={px} x2={px} y1={trueY} y2={predY} />
                    <circle className="truth-point" cx={px} cy={trueY} r="6" style={{ fill: color }} />
                    <circle className="prediction-point" cx={px} cy={predY} r="5" />
                  </g>
                );
              })}

              <text className="axis-title" x={chart.width / 2} y={chart.height - 5}>{selectedLab.xLabel}</text>
              <text className="axis-title axis-title--y" x="20" y={chart.height / 2}>{selectedLab.yLabel}</text>
            </svg>

            <div className="legend" aria-label="Chart legend">
              <span><i className="legend-dot legend-dot--truth" />Actual</span>
              <span><i className="legend-dot legend-dot--prediction" />Prediction</span>
              <span><i className="legend-line legend-line--model" />Current line</span>
              <span><i className="legend-line legend-line--ideal" />Best fit</span>
            </div>
          </section>
        </section>

        <aside className="controls metrics" aria-label="Training controls and metrics">
          <label className="field">
            <span>Loss</span>
            <select value={lossKind} onChange={(event) => setLossKind(event.target.value as LossKind)}>
              <option value="mse">Mean squared error</option>
              <option value="mae">Mean absolute error</option>
            </select>
          </label>

          <label className="field">
            <span>Activation preview</span>
            <select value={activationKind} onChange={(event) => setActivationKind(event.target.value as ActivationKind)}>
              {ACTIVATIONS.map((activation) => (
                <option key={activation.kind} value={activation.kind}>{activation.label}</option>
              ))}
            </select>
          </label>

          <label className="field">
            <span>Learning rate</span>
            <input
              type="range"
              min={selectedLab.learningRateMin}
              max={selectedLab.learningRateMax}
              step={selectedLab.learningRateStep}
              value={learningRate}
              onChange={(event) => setLearningRate(Number(event.target.value))}
            />
            <input
              type="number"
              min={selectedLab.learningRateMin}
              max={selectedLab.learningRateMax}
              step={selectedLab.learningRateStep}
              value={learningRate}
              onChange={(event) => setLearningRate(Number(event.target.value))}
            />
          </label>

          <div className="field-grid">
            <label className="field">
              <span>Initial weight</span>
              <input type="number" step="0.1" value={initialWeight} onChange={(event) => setInitialWeight(Number(event.target.value))} />
            </label>
            <label className="field">
              <span>Initial bias</span>
              <input type="number" step="0.5" value={initialBias} onChange={(event) => setInitialBias(Number(event.target.value))} />
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
            <strong>{formatNumber(currentMae, 3)}</strong>
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

          <div className="lesson">
            <span>Learning note</span>
            <p>{selectedLab.lesson}</p>
          </div>

          <div className="activation-panel">
            <div className="history__topline">
              <span>{selectedActivation.label}</span>
              <strong>f(x)</strong>
            </div>
            <svg viewBox="0 0 250 82" role="img" aria-label={`${selectedActivation.label} activation curve`}>
              <path className="history-grid" d="M 0 41 L 250 41" />
              <path className="activation-line" d={activationPath(activationKind)} />
            </svg>
            <p>{selectedActivation.summary}</p>
          </div>

          <div className="source-panel">
            <span>Data source</span>
            <p>{selectedLab.source.name}</p>
            <code>{selectedLab.source.license}</code>
          </div>
        </aside>
      </main>
    </div>
  );
}
