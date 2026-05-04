import { useEffect, useMemo, useState } from "react";
import {
  HIDDEN_LAYER_EXAMPLES,
  createInitialHiddenState,
  exampleInputs,
  hiddenHistoryPoint,
  hiddenLoss,
  hiddenMeanAbsoluteError,
  predictHidden,
  traceHiddenExample,
  trainHiddenSteps,
  type HiddenLayerExample,
  type HiddenLayerHistoryPoint,
  type HiddenLayerModelState,
  type HiddenLayerStepResult,
} from "./hidden-layer-examples.js";
import { gradientShape } from "./layered-network.js";
import {
  canUseWebGpuMatrixBackend,
  predictLayeredWithBestMatrixBackend,
  predictLayeredWithVm,
  type MatrixExecutionBackend,
} from "./neural-vm.js";
import { HiddenNetworkDiagram } from "./NetworkDiagram.js";

interface HiddenChartFrame {
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

const CURVE_CHART: HiddenChartFrame = {
  width: 720,
  height: 410,
  padLeft: 58,
  padRight: 24,
  padTop: 24,
  padBottom: 48,
  xMin: -1,
  xMax: 1,
  yMin: -0.08,
  yMax: 1.08,
};

const SURFACE_SIZE = 460;

interface SurfaceCell {
  readonly x: number;
  readonly y: number;
  readonly value: number;
}

interface SurfaceGrid {
  readonly cells: readonly SurfaceCell[];
  readonly inputs: number[][];
}

interface HiddenPredictionBundle {
  readonly rowPredictions: number[];
  readonly curvePath: string;
  readonly surfaceCells: readonly SurfaceCell[];
  readonly backend: MatrixExecutionBackend;
  readonly fallbackReason?: string;
}

function formatNumber(value: number, digits = 3): string {
  if (!Number.isFinite(value)) {
    return "0";
  }
  if (Math.abs(value) < 0.01 && value !== 0) {
    return value.toExponential(2);
  }
  return value.toFixed(digits);
}

function clamp(value: number, min: number, max: number): number {
  return Math.min(max, Math.max(min, value));
}

function hiddenLayerLabel(count: number): string {
  return `${count} hidden layer${count === 1 ? "" : "s"}`;
}

function xScale(value: number, chart: HiddenChartFrame): number {
  const width = chart.width - chart.padLeft - chart.padRight;
  return chart.padLeft + ((value - chart.xMin) / (chart.xMax - chart.xMin)) * width;
}

function yScale(value: number, chart: HiddenChartFrame): number {
  const height = chart.height - chart.padTop - chart.padBottom;
  return chart.padTop + (1 - (value - chart.yMin) / (chart.yMax - chart.yMin)) * height;
}

function hiddenHistoryPath(history: HiddenLayerHistoryPoint[]): string {
  if (history.length === 0) {
    return "";
  }
  const width = 250;
  const height = 74;
  const maxLoss = Math.max(...history.map((point) => point.loss), 1e-6);
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

function makeCurveSamples(): number[] {
  return Array.from({ length: 121 }, (_, index) => {
    const x = CURVE_CHART.xMin + (index / 120) * (CURVE_CHART.xMax - CURVE_CHART.xMin);
    return x;
  });
}

function makeCurvePathFromPredictions(samples: readonly number[], predictions: readonly number[]): string {
  const points = samples.map((x, index) => {
    return [x, predictions[index] ?? 0] as const;
  });

  return points
    .map(([x, y], index) => {
      const command = index === 0 ? "M" : "L";
      return `${command} ${xScale(x, CURVE_CHART).toFixed(2)} ${yScale(y, CURVE_CHART).toFixed(2)}`;
    })
    .join(" ");
}

function makeCurvePath(example: HiddenLayerExample, state: HiddenLayerModelState): string {
  const samples = makeCurveSamples();
  const predictions = predictLayeredWithVm(
    samples.map((x) => [x]),
    state.parameters,
    {
      inputNames: example.inputLabels,
      outputNames: [example.outputLabel],
    },
  ).predictions.map((row) => row[0] ?? 0);

  return makeCurvePathFromPredictions(samples, predictions);
}

function makeSurfaceGrid(example: HiddenLayerExample): SurfaceGrid {
  const cells: SurfaceCell[] = [];
  const inputs: number[][] = [];
  const divisions = 26;
  const xValues = example.rows.map((row) => row.input[0]!);
  const yValues = example.rows.map((row) => row.input[1]!);
  const xMin = Math.min(...xValues, -1);
  const xMax = Math.max(...xValues, 1);
  const yMin = Math.min(...yValues, -1);
  const yMax = Math.max(...yValues, 1);
  const xPad = Math.max((xMax - xMin) * 0.08, 0.15);
  const yPad = Math.max((yMax - yMin) * 0.08, 0.15);

  for (let y = 0; y < divisions; y += 1) {
    for (let x = 0; x < divisions; x += 1) {
      const inputX = xMin - xPad + (x / (divisions - 1)) * (xMax - xMin + xPad * 2);
      const inputY = yMin - yPad + (y / (divisions - 1)) * (yMax - yMin + yPad * 2);
      inputs.push([inputX, inputY]);
      const value = 0;
      cells.push({ x, y, value });
    }
  }

  return { cells, inputs };
}

function makeSurfaceCells(example: HiddenLayerExample, state: HiddenLayerModelState): readonly SurfaceCell[] {
  const grid = makeSurfaceGrid(example);
  const predictions = predictLayeredWithVm(grid.inputs, state.parameters, {
    inputNames: example.inputLabels,
    outputNames: [example.outputLabel],
  }).predictions;

  return grid.cells.map((cell, index) => ({
    ...cell,
    value: predictions[index]?.[0] ?? 0,
  }));
}

function makePredictionBundleSync(example: HiddenLayerExample, state: HiddenLayerModelState): HiddenPredictionBundle {
  return {
    rowPredictions: predictHidden(example, state),
    curvePath: example.chartKind === "curve" ? makeCurvePath(example, state) : "",
    surfaceCells: example.chartKind === "surface" ? makeSurfaceCells(example, state) : [],
    backend: "cpu",
  };
}

async function makePredictionBundleAsync(
  example: HiddenLayerExample,
  state: HiddenLayerModelState,
): Promise<HiddenPredictionBundle> {
  const rowInputs = exampleInputs(example);
  const curveSamples = example.chartKind === "curve" ? makeCurveSamples() : [];
  const curveInputs = curveSamples.map((x) => [x]);
  const surfaceGrid = example.chartKind === "surface" ? makeSurfaceGrid(example) : { cells: [], inputs: [] };
  const allInputs = [
    ...rowInputs,
    ...curveInputs,
    ...surfaceGrid.inputs,
  ];
  const run = await predictLayeredWithBestMatrixBackend(allInputs, state.parameters, {
    inputNames: example.inputLabels,
    outputNames: [example.outputLabel],
  });
  const values = run.predictions.map((row) => row[0] ?? 0);
  const rowPredictions = values.slice(0, rowInputs.length);
  const curvePredictions = values.slice(rowInputs.length, rowInputs.length + curveInputs.length);
  const surfacePredictions = values.slice(rowInputs.length + curveInputs.length);

  return {
    rowPredictions,
    curvePath: curveSamples.length > 0
      ? makeCurvePathFromPredictions(curveSamples, curvePredictions)
      : "",
    surfaceCells: surfaceGrid.cells.map((cell, index) => ({
      ...cell,
      value: surfacePredictions[index] ?? 0,
    })),
    backend: run.backend,
    fallbackReason: run.fallbackReason,
  };
}

function CurveChart({ example, curvePath, predictions }: { example: HiddenLayerExample; curvePath: string; predictions: number[] }) {
  return (
    <svg viewBox={`0 0 ${CURVE_CHART.width} ${CURVE_CHART.height}`} role="img" aria-label={`${example.title} hidden-layer curve`}>
      <rect
        className="plot-bg"
        x={CURVE_CHART.padLeft}
        y={CURVE_CHART.padTop}
        width={CURVE_CHART.width - CURVE_CHART.padLeft - CURVE_CHART.padRight}
        height={CURVE_CHART.height - CURVE_CHART.padTop - CURVE_CHART.padBottom}
      />
      {[0, 0.25, 0.5, 0.75, 1].map((ratio) => {
        const x = CURVE_CHART.xMin + (CURVE_CHART.xMax - CURVE_CHART.xMin) * ratio;
        const y = CURVE_CHART.yMin + (CURVE_CHART.yMax - CURVE_CHART.yMin) * ratio;
        return (
          <g key={ratio}>
            <line className="grid-line" x1={xScale(x, CURVE_CHART)} x2={xScale(x, CURVE_CHART)} y1={CURVE_CHART.padTop} y2={CURVE_CHART.height - CURVE_CHART.padBottom} />
            <text className="axis-label" x={xScale(x, CURVE_CHART)} y={CURVE_CHART.height - 18}>{formatNumber(x, 1)}</text>
            <line className="grid-line" x1={CURVE_CHART.padLeft} x2={CURVE_CHART.width - CURVE_CHART.padRight} y1={yScale(y, CURVE_CHART)} y2={yScale(y, CURVE_CHART)} />
            <text className="axis-label axis-label--y" x={CURVE_CHART.padLeft - 10} y={yScale(y, CURVE_CHART) + 4}>{formatNumber(y, 1)}</text>
          </g>
        );
      })}
      <path className="hidden-curve" d={curvePath} />
      {example.rows.map((row, index) => {
        const px = xScale(row.input[0]!, CURVE_CHART);
        const targetY = yScale(row.target, CURVE_CHART);
        const predY = yScale(predictions[index]!, CURVE_CHART);
        return (
          <g key={row.label}>
            <line className="error-line" x1={px} x2={px} y1={targetY} y2={predY} />
            <circle className="truth-point" cx={px} cy={targetY} r="6" />
            <circle className="prediction-point" cx={px} cy={predY} r="5" />
          </g>
        );
      })}
      <text className="axis-title" x={CURVE_CHART.width / 2} y={CURVE_CHART.height - 5}>{example.inputLabels[0]}</text>
      <text className="axis-title axis-title--y" x="20" y={CURVE_CHART.height / 2}>{example.outputLabel}</text>
    </svg>
  );
}

function SurfaceChart({ example, cells, predictions, selectedIndex, onSelect }: {
  example: HiddenLayerExample;
  cells: readonly SurfaceCell[];
  predictions: number[];
  selectedIndex: number;
  onSelect: (index: number) => void;
}) {
  const divisions = Math.sqrt(cells.length);
  const cellSize = SURFACE_SIZE / divisions;
  const xValues = example.rows.map((row) => row.input[0]!);
  const yValues = example.rows.map((row) => row.input[1]!);
  const xMin = Math.min(...xValues, -1);
  const xMax = Math.max(...xValues, 1);
  const yMin = Math.min(...yValues, -1);
  const yMax = Math.max(...yValues, 1);
  const xPad = Math.max((xMax - xMin) * 0.08, 0.15);
  const yPad = Math.max((yMax - yMin) * 0.08, 0.15);
  const sx = (value: number) => ((value - (xMin - xPad)) / (xMax - xMin + xPad * 2)) * SURFACE_SIZE;
  const sy = (value: number) => SURFACE_SIZE - ((value - (yMin - yPad)) / (yMax - yMin + yPad * 2)) * SURFACE_SIZE;

  return (
    <svg className="surface-chart" viewBox={`0 0 ${SURFACE_SIZE} ${SURFACE_SIZE}`} role="img" aria-label={`${example.title} decision surface`}>
      {cells.map((cell) => (
        <rect
          key={`${cell.x}-${cell.y}`}
          x={cell.x * cellSize}
          y={cell.y * cellSize}
          width={cellSize + 0.5}
          height={cellSize + 0.5}
          style={{
            fill: `rgba(${Math.round(194 - cell.value * 150)}, ${Math.round(65 + cell.value * 90)}, ${Math.round(59 + cell.value * 120)}, 0.72)`,
          }}
        />
      ))}
      {example.rows.map((row, index) => (
        <g
          aria-label={`Select ${row.label}`}
          className="svg-button"
          key={row.label}
          role="button"
          tabIndex={0}
          onClick={() => onSelect(index)}
          onKeyDown={(event) => {
            if (event.key === "Enter" || event.key === " ") {
              onSelect(index);
            }
          }}
        >
          <circle
            className={index === selectedIndex ? "surface-point surface-point--selected" : "surface-point"}
            cx={sx(row.input[0]!)}
            cy={sy(row.input[1]!)}
            r={index === selectedIndex ? 9 : 7}
            style={{ fill: row.target >= 0.5 ? "#237a57" : "#f7f8f3" }}
          />
          <text className="surface-label" x={sx(row.input[0]!) + 10} y={sy(row.input[1]!) - 8}>
            {formatNumber(predictions[index]!, 2)}
          </text>
        </g>
      ))}
    </svg>
  );
}

function TableChart({ example, predictions, selectedIndex, onSelect }: {
  example: HiddenLayerExample;
  predictions: number[];
  selectedIndex: number;
  onSelect: (index: number) => void;
}) {
  return (
    <div className="hidden-table-chart">
      {example.rows.map((row, index) => {
        const error = predictions[index]! - row.target;
        return (
          <button
            className={index === selectedIndex ? "table-row table-row--selected" : "table-row"}
            key={row.label}
            type="button"
            onClick={() => onSelect(index)}
          >
            <span>{row.label}</span>
            <span className="bar-pair">
              <i className="bar-target" style={{ width: `${row.target * 100}%` }} />
              <i className="bar-prediction" style={{ width: `${predictions[index]! * 100}%` }} />
            </span>
            <code>{formatNumber(error, 3)}</code>
          </button>
        );
      })}
    </div>
  );
}

export function HiddenLayerWorkbench() {
  const [selectedId, setSelectedId] = useState(HIDDEN_LAYER_EXAMPLES[0]!.id);
  const example = HIDDEN_LAYER_EXAMPLES.find((item) => item.id === selectedId) ?? HIDDEN_LAYER_EXAMPLES[0]!;
  const [learningRate, setLearningRate] = useState(example.defaultLearningRate);
  const [state, setState] = useState<HiddenLayerModelState>(() => createInitialHiddenState(example));
  const [history, setHistory] = useState<HiddenLayerHistoryPoint[]>(() => [hiddenHistoryPoint(example, createInitialHiddenState(example))]);
  const [lastStep, setLastStep] = useState<HiddenLayerStepResult | null>(null);
  const [selectedIndex, setSelectedIndex] = useState(0);
  const [isRunning, setIsRunning] = useState(false);
  const hiddenLayerCount = state.hiddenLayerCount;

  useEffect(() => {
    const initial = createInitialHiddenState(example);
    setLearningRate(example.defaultLearningRate);
    setState(initial);
    setHistory([hiddenHistoryPoint(example, initial)]);
    setLastStep(null);
    setSelectedIndex(0);
    setIsRunning(false);
  }, [example]);

  const fallbackBundle = useMemo(() => makePredictionBundleSync(example, state), [example, state]);
  const [acceleratedBundle, setAcceleratedBundle] = useState<HiddenPredictionBundle | null>(null);
  const predictionBundle = acceleratedBundle ?? fallbackBundle;
  const predictions = predictionBundle.rowPredictions;
  const loss = useMemo(() => hiddenLoss(example, state), [example, state]);
  const mae = useMemo(() => hiddenMeanAbsoluteError(example, state), [example, state]);
  const trace = useMemo(() => traceHiddenExample(example, state, selectedIndex), [example, selectedIndex, state]);
  const outputGradient = lastStep?.step.weightGradients[lastStep.step.weightGradients.length - 1];
  const backendLabel = predictionBundle.backend === "webgpu" ? "WebGPU" : "CPU";

  useEffect(() => {
    let cancelled = false;
    setAcceleratedBundle(null);
    if (!canUseWebGpuMatrixBackend()) {
      return () => {
        cancelled = true;
      };
    }

    makePredictionBundleAsync(example, state)
      .then((bundle) => {
        if (!cancelled) {
          setAcceleratedBundle(bundle);
        }
      })
      .catch((error: unknown) => {
        if (!cancelled) {
          setAcceleratedBundle({
            ...fallbackBundle,
            fallbackReason: error instanceof Error ? error.message : "Matrix backend failed",
          });
        }
      });

    return () => {
      cancelled = true;
    };
  }, [example, fallbackBundle, state]);

  function record(result: HiddenLayerStepResult): void {
    setState(result.state);
    setLastStep(result);
    setHistory((items) => [...items.slice(-159), {
      epoch: result.state.epoch,
      loss: result.loss,
      mae: result.mae,
    }]);
  }

  function step(count: number): void {
    const results = trainHiddenSteps(example, state, learningRate, count);
    const result = results[results.length - 1];
    if (result !== undefined) {
      record(result);
    }
  }

  function reset(): void {
    const initial = createInitialHiddenState(example, hiddenLayerCount);
    setState(initial);
    setHistory([hiddenHistoryPoint(example, initial)]);
    setLastStep(null);
    setIsRunning(false);
  }

  function selectExample(nextExample: HiddenLayerExample): void {
    const initial = createInitialHiddenState(nextExample);
    setSelectedId(nextExample.id);
    setLearningRate(nextExample.defaultLearningRate);
    setState(initial);
    setHistory([hiddenHistoryPoint(nextExample, initial)]);
    setLastStep(null);
    setSelectedIndex(0);
    setIsRunning(false);
  }

  function changeHiddenLayerCount(value: number): void {
    const nextCount = Math.max(
      example.hiddenLayerMin,
      Math.min(example.hiddenLayerMax, Math.round(value)),
    );
    const initial = createInitialHiddenState(example, nextCount);
    setState(initial);
    setHistory([hiddenHistoryPoint(example, initial)]);
    setLastStep(null);
    setSelectedIndex(0);
    setIsRunning(false);
  }

  useEffect(() => {
    if (!isRunning) {
      return undefined;
    }

    const id = window.setInterval(() => {
      setState((current) => {
        const results = trainHiddenSteps(example, current, learningRate, 5);
        const result = results[results.length - 1]!;
        setLastStep(result);
        setHistory((items) => [...items.slice(-159), {
          epoch: result.state.epoch,
          loss: result.loss,
          mae: result.mae,
        }]);
        return result.state;
      });
    }, 160);

    return () => window.clearInterval(id);
  }, [example, isRunning, learningRate]);

  return (
    <main className="workspace workspace--hidden">
      <nav className="lab-rail" aria-label="Hidden-layer examples">
        <div className="rail-summary">
          <strong>{HIDDEN_LAYER_EXAMPLES.length}</strong>
          <span>hidden examples</span>
        </div>
        <div className="lab-list">
          {HIDDEN_LAYER_EXAMPLES.map((item) => (
            <button
              className={item.id === example.id ? "lab-button lab-button--active" : "lab-button"}
              key={item.id}
              type="button"
              onClick={() => selectExample(item)}
            >
              <span>{item.title}</span>
              <small>{item.category}</small>
            </button>
          ))}
        </div>
      </nav>

      <section className="lab-stage" aria-label="Hidden-layer training stage">
        <div className="lab-intro">
          <div>
            <p className="eyebrow">{example.category}</p>
            <h2>{example.title}</h2>
            <p>{example.summary}</p>
          </div>
          <div className="lab-chip">{hiddenLayerCount} layers / {example.hiddenCount} neurons</div>
        </div>

        <section className="chart-panel chart-panel--hidden" aria-label="Hidden-layer chart">
          {example.chartKind === "curve" && (
            <CurveChart
              example={example}
              curvePath={predictionBundle.curvePath}
              predictions={predictions}
            />
          )}
          {example.chartKind === "surface" && (
            <SurfaceChart
              example={example}
              cells={predictionBundle.surfaceCells}
              predictions={predictions}
              selectedIndex={selectedIndex}
              onSelect={setSelectedIndex}
            />
          )}
          {example.chartKind === "table" && (
            <TableChart example={example} predictions={predictions} selectedIndex={selectedIndex} onSelect={setSelectedIndex} />
          )}

          <div className="legend" aria-label="Hidden chart legend">
            <span><i className="legend-dot legend-dot--truth" />Target</span>
            <span><i className="legend-dot legend-dot--prediction" />Prediction</span>
            <span><i className="legend-line legend-line--model" />Current model</span>
          </div>
        </section>

        <section className="trace-panel" aria-label="Neuron trace">
          <div className="history__topline">
            <span>{example.rows[selectedIndex]!.label}</span>
            <strong>{formatNumber(predictions[selectedIndex]!, 3)} / {formatNumber(example.rows[selectedIndex]!.target, 3)}</strong>
          </div>
          <div className="hidden-neuron-grid">
            {trace.layers
              .filter((layer) => layer.layer.startsWith("hidden"))
              .flatMap((layer, layerIndex) => layer.neurons.map((neuron, neuronIndex) => (
                <div className="neuron-tile" key={neuron.neuron}>
                  <span>h{layerIndex + 1}.{neuronIndex + 1}</span>
                  <strong>{formatNumber(neuron.output, 3)}</strong>
                  <i style={{ width: `${clamp(neuron.output, 0, 1) * 100}%` }} />
                </div>
              )))}
          </div>
          <div className="trace-equation">
            <code>{example.inputLabels.join(", ")} {"->"} {hiddenLayerCount} x hidden[{example.hiddenCount}] {"->"} {example.outputLabel}</code>
          </div>
        </section>

        <HiddenNetworkDiagram
          example={example}
          state={state}
          selectedRow={example.rows[selectedIndex]!}
          selectedIndex={selectedIndex}
          prediction={predictions[selectedIndex]!}
          lastStep={lastStep}
          learningRate={learningRate}
        />
      </section>

      <aside className="controls metrics" aria-label="Hidden-layer controls">
        <label className="field">
          <span>Hidden layers</span>
          <input
            type="range"
            min={example.hiddenLayerMin}
            max={example.hiddenLayerMax}
            step="1"
            value={hiddenLayerCount}
            onChange={(event) => changeHiddenLayerCount(Number(event.target.value))}
          />
          <input
            type="number"
            min={example.hiddenLayerMin}
            max={example.hiddenLayerMax}
            step="1"
            value={hiddenLayerCount}
            onChange={(event) => changeHiddenLayerCount(Number(event.target.value))}
          />
        </label>

        <label className="field">
          <span>Learning rate</span>
          <input
            type="range"
            min={example.learningRateMin}
            max={example.learningRateMax}
            step={example.learningRateStep}
            value={learningRate}
            onChange={(event) => setLearningRate(Number(event.target.value))}
          />
          <input
            type="number"
            min={example.learningRateMin}
            max={example.learningRateMax}
            step={example.learningRateStep}
            value={learningRate}
            onChange={(event) => setLearningRate(Number(event.target.value))}
          />
        </label>

        <div className="button-grid">
          <button type="button" onClick={() => step(1)}>Step</button>
          <button type="button" onClick={() => step(25)}>Step 25</button>
          <button type="button" onClick={() => setIsRunning((value) => !value)}>{isRunning ? "Pause" : "Run"}</button>
          <button type="button" onClick={reset}>Reset</button>
        </div>

        <div className="metric">
          <span>Epoch</span>
          <strong>{state.epoch}</strong>
        </div>
        <div className="metric">
          <span>Loss</span>
          <strong>{formatNumber(loss, 4)}</strong>
        </div>
        <div className="metric">
          <span>Average error</span>
          <strong>{formatNumber(mae, 3)}</strong>
        </div>
        <div className="metric" title={predictionBundle.fallbackReason}>
          <span>Matrix backend</span>
          <strong>{backendLabel}</strong>
        </div>

        <div className="history">
          <div className="history__topline">
            <span>Loss history</span>
            <strong>{history.length} points</strong>
          </div>
          <svg viewBox="0 0 250 74" role="img" aria-label="Hidden-layer loss history">
            <path className="history-grid" d="M 0 37 L 250 37" />
            <path className="history-line" d={hiddenHistoryPath(history)} />
          </svg>
        </div>

        <label className="field">
          <span>Trace row</span>
          <select value={selectedIndex} onChange={(event) => setSelectedIndex(Number(event.target.value))}>
            {example.rows.map((row, index) => (
              <option key={row.label} value={index}>{row.label}</option>
            ))}
          </select>
        </label>

        <div className="gradients">
          <span>Last gradient shape</span>
          <code>{hiddenLayerLabel(hiddenLayerCount)}</code>
          <code>input-hidden {gradientShape(lastStep?.step.weightGradients[0])}</code>
          <code>hidden-output {gradientShape(outputGradient)}</code>
        </div>

        <div className="lesson">
          <span>Learning note</span>
          <p>{example.lesson}</p>
        </div>
      </aside>
    </main>
  );
}
