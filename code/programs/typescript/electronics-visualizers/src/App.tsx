import { useEffect, useMemo, useState } from "react";
import { IdealDcSupply, IdealSineSupply } from "@coding-adventures/power-supply";
import { IdealResistor, analyzeDcResistor } from "@coding-adventures/electronics";
import {
  SliderControl,
  TabList,
  useAnimatedNumber,
  useAnimationFrame,
} from "@coding-adventures/ui-components";

interface PlotPoint {
  x: number;
  y: number;
}

const PLOT_LEFT = 60;
const PLOT_TOP = 26;
const PLOT_WIDTH = 740;
const PLOT_HEIGHT = 210;

type SourceMode = "dc" | "ac";
type VisualizerTab = "resistors";
const AC_TIMEBASE_SCALE = 0.2;
const VOLTAGE_SCOPE_MAX = 12;
const CURRENT_SCOPE_MAX = 1.2;

function formatNumber(value: number): string {
  return value.toFixed(3);
}

function toPolyline(points: PlotPoint[], yScaleMax: number): string {
  const maxAbsY = Math.max(yScaleMax, 1e-9);
  return points
    .map((point) => {
      const px = PLOT_LEFT + point.x * PLOT_WIDTH;
      const py =
        PLOT_TOP + PLOT_HEIGHT / 2 - (point.y / maxAbsY) * (PLOT_HEIGHT * 0.42);
      return `${px},${py}`;
    })
    .join(" ");
}

function buildContinuousPoints(
  fn: (timeSeconds: number) => number,
  durationSeconds: number,
  pointCount: number
): PlotPoint[] {
  return Array.from({ length: pointCount }, (_, index) => {
    const x = index / (pointCount - 1);
    const timeSeconds = x * durationSeconds;
    return { x, y: fn(timeSeconds) };
  });
}

function useAnimatedTime(active: boolean): number {
  const [timeSeconds, setTimeSeconds] = useState(0);

  useEffect(() => {
    if (!active) {
      setTimeSeconds(0);
    }
  }, [active]);

  useAnimationFrame(
    (deltaMs) => {
      setTimeSeconds((current) => current + (deltaMs / 1000) * AC_TIMEBASE_SCALE);
    },
    active,
  );

  return timeSeconds;
}

function ScopePlot({
  title,
  points,
  yScaleMax,
  yAxisLabel,
  colorClass,
}: {
  title: string;
  points: PlotPoint[];
  yScaleMax: number;
  yAxisLabel: string;
  colorClass: string;
}) {
  const animatedScale = useAnimatedNumber(yScaleMax, 280);
  const [isScalePulsing, setIsScalePulsing] = useState(false);
  const tickValues = [
    animatedScale,
    animatedScale / 2,
    0,
    -animatedScale / 2,
    -animatedScale,
  ];
  const centerY = PLOT_TOP + PLOT_HEIGHT / 2;

  useEffect(() => {
    setIsScalePulsing(true);
    const timeoutId = window.setTimeout(() => setIsScalePulsing(false), 420);
    return () => window.clearTimeout(timeoutId);
  }, [yScaleMax]);

  return (
    <section className="plot-card scope">
      <div className="plot-card__header">
        <h3>{title}</h3>
        <div
          className={
            isScalePulsing
              ? "scope-readout scope-readout--pulse"
              : "scope-readout"
          }
        >
          <span className="scope-readout__label">Scale</span>
          <strong>{formatNumber(animatedScale)}</strong>
          <span className="scope-readout__units">{yAxisLabel}</span>
        </div>
      </div>
      <svg viewBox="0 0 800 260" className="plot-card__svg" aria-label={title}>
        {tickValues.map((tickValue) => {
          const y =
            PLOT_TOP +
            PLOT_HEIGHT / 2 -
            (tickValue / Math.max(animatedScale, 1e-9)) * (PLOT_HEIGHT * 0.42);

          return (
            <g key={tickValue}>
              <line
                x1={PLOT_LEFT}
                y1={y}
                x2={PLOT_LEFT + PLOT_WIDTH}
                y2={y}
                className="plot-card__grid"
              />
              <text x="8" y={y + 4} className="plot-card__tick">
                {formatNumber(tickValue)}
              </text>
            </g>
          );
        })}
        <text x="14" y="20" className="plot-card__axis-label">
          {yAxisLabel}
        </text>
        <line
          x1={PLOT_LEFT}
          y1={PLOT_TOP}
          x2={PLOT_LEFT}
          y2={PLOT_TOP + PLOT_HEIGHT}
          className="plot-card__axis"
        />
        <line
          x1={PLOT_LEFT}
          y1={centerY}
          x2={PLOT_LEFT + PLOT_WIDTH}
          y2={centerY}
          className="plot-card__axis"
        />
        <polyline
          fill="none"
          strokeWidth="4"
          className={colorClass}
          points={toPolyline(points, animatedScale)}
        />
        <text x="738" y="248" className="plot-card__axis-label">
          time
        </text>
      </svg>
    </section>
  );
}

function BenchDiagram({
  mode,
  sourceLabel,
  resistorOhms,
  voltageText,
  currentText,
}: {
  mode: SourceMode;
  sourceLabel: string;
  resistorOhms: number;
  voltageText: string;
  currentText: string;
}) {
  return (
    <section className="bench-card">
      <div className="bench-header">
        <h2>Bench Setup</h2>
        <span className="bench-badge">{mode === "dc" ? "DC supply" : "AC supply"}</span>
      </div>

      <div className="bench-visual">
        <div className="supply">
          <div className="supply__screen">{sourceLabel}</div>
          <div className="supply__knob" />
          <div className="supply__label">Power Supply</div>
        </div>

        <div className="board">
          <div className="board__trace board__trace--top" />
          <div className="board__trace board__trace--bottom" />
          <div className="board__resistor">
            <div className="board__resistor-body" />
            <div className="board__resistor-text">{formatNumber(resistorOhms)} ohms</div>
          </div>
          <div className="probe probe--voltage">
            <span>Voltage probe</span>
            <strong>{voltageText}</strong>
          </div>
          <div className="probe probe--current">
            <span>Current probe</span>
            <strong>{currentText}</strong>
          </div>
        </div>
      </div>
    </section>
  );
}

export function App() {
  const [activeTab, setActiveTab] = useState<VisualizerTab>("resistors");
  const [mode, setMode] = useState<SourceMode>("dc");
  const [dcVoltage, setDcVoltage] = useState(5);
  const [acAmplitude, setAcAmplitude] = useState(5);
  const [frequency, setFrequency] = useState(2);
  const [resistance, setResistance] = useState(100);

  const resistor = useMemo(() => new IdealResistor(resistance), [resistance]);
  const animatedTime = useAnimatedTime(mode === "ac");

  const durationSeconds = mode === "dc" ? 1 : 1 / frequency;
  const pointCount = 240;

  const voltagePoints = useMemo(() => {
    if (mode === "dc") {
      const dc = analyzeDcResistor(dcVoltage, resistor);
      return buildContinuousPoints(() => dc.voltage, durationSeconds, pointCount);
    }

    const supply = new IdealSineSupply(acAmplitude, frequency);
    const waveform = supply.asWaveform();
    const phaseOffset = animatedTime;

    return buildContinuousPoints(
      (timeSeconds) => waveform.sampleAt(timeSeconds + phaseOffset),
      durationSeconds,
      pointCount
    );
  }, [
    mode,
    dcVoltage,
    resistor,
    durationSeconds,
    acAmplitude,
    frequency,
    animatedTime,
  ]);

  const currentPoints = useMemo(() => {
    if (mode === "dc") {
      const dc = analyzeDcResistor(dcVoltage, resistor);
      return buildContinuousPoints(() => dc.current, durationSeconds, pointCount);
    }

    const supply = new IdealSineSupply(acAmplitude, frequency);
    const waveform = supply.asWaveform();
    const phaseOffset = animatedTime;

    return buildContinuousPoints(
      (timeSeconds) =>
        resistor.currentForVoltage(waveform.sampleAt(timeSeconds + phaseOffset)),
      durationSeconds,
      pointCount
    );
  }, [
    mode,
    dcVoltage,
    resistor,
    durationSeconds,
    acAmplitude,
    frequency,
    animatedTime,
  ]);

  const sourceLabel =
    mode === "dc"
      ? `${formatNumber(new IdealDcSupply(dcVoltage).voltage)} V`
      : `${formatNumber(acAmplitude)} Vpk @ ${formatNumber(frequency)} Hz`;

  const voltageSummary =
    mode === "dc" ? `${formatNumber(dcVoltage)} V` : `${formatNumber(acAmplitude)} Vpk`;
  const currentPeak = mode === "dc" ? dcVoltage / resistance : acAmplitude / resistance;
  const currentSummary =
    mode === "dc" ? `${formatNumber(currentPeak)} A` : `${formatNumber(currentPeak)} Apk`;

  const visualizerTabs = [{ id: "resistors" as const, label: "Resistors" }];

  return (
    <div className="app-shell">
      <header className="hero">
        <div>
          <p className="hero__eyebrow">Electronics Showcase</p>
          <h1>Electronics Visualizers</h1>
          <p className="hero__subtitle">
            Interactive benches for passive components, starting with a single
            resistor, a fake power supply, and a fake oscilloscope.
          </p>
        </div>
      </header>

      <TabList
        items={visualizerTabs}
        activeTab={activeTab}
        onActiveChange={setActiveTab}
        ariaLabel="Electronics visualizers"
        className="mode-row"
        tabClassName="tab"
        activeTabClassName="tab--active"
      />

      {activeTab === "resistors" && (
        <>
          <section className="mode-row" aria-label="Source mode">
            <button
              className={mode === "dc" ? "tab tab--active" : "tab"}
              onClick={() => setMode("dc")}
            >
              DC source
            </button>
            <button
              className={mode === "ac" ? "tab tab--active" : "tab"}
              onClick={() => setMode("ac")}
            >
              AC source
            </button>
          </section>

          <section className="control-card">
            <h2>Controls</h2>
            <div className="control-grid">
              {mode === "dc" ? (
                <SliderControl
                  className="control slider-control"
                  label="Supply voltage"
                  value={dcVoltage}
                  min={1}
                  max={12}
                  step={0.5}
                  onChange={setDcVoltage}
                  unit="V"
                  formatValue={(value) => formatNumber(value)}
                />
              ) : (
                <>
                  <SliderControl
                    className="control slider-control"
                    label="Amplitude"
                    value={acAmplitude}
                    min={1}
                    max={12}
                    step={0.5}
                    onChange={setAcAmplitude}
                    unit="Vpk"
                    formatValue={(value) => formatNumber(value)}
                  />
                  <SliderControl
                    className="control slider-control"
                    label="Frequency"
                    value={frequency}
                    min={1}
                    max={10}
                    step={0.5}
                    onChange={setFrequency}
                    unit="Hz"
                    formatValue={(value) => formatNumber(value)}
                  />
                </>
              )}

              <SliderControl
                className="control slider-control"
                label="Resistance"
                value={resistance}
                min={10}
                max={500}
                step={10}
                onChange={setResistance}
                unit="ohms"
                formatValue={(value) => value.toFixed(0)}
              />
            </div>
          </section>

          <main className="main-stage">
            <BenchDiagram
              mode={mode}
              sourceLabel={sourceLabel}
              resistorOhms={resistance}
              voltageText={voltageSummary}
              currentText={currentSummary}
            />

            <section className="scope-shell">
              <div className="scope-shell__header">
                <h2>Oscilloscope</h2>
                <p>
                  Voltage and current are shown on separate channels so amplitude changes
                  stay obvious.
                </p>
              </div>
              <div className="plot-stack">
                <ScopePlot
                  title="Channel 1: Voltage"
                  points={voltagePoints}
                  yScaleMax={VOLTAGE_SCOPE_MAX}
                  yAxisLabel="volts"
                  colorClass="plot-card__series--voltage"
                />
                <ScopePlot
                  title="Channel 2: Current"
                  points={currentPoints}
                  yScaleMax={CURRENT_SCOPE_MAX}
                  yAxisLabel="amps"
                  colorClass="plot-card__series--current"
                />
              </div>
            </section>
          </main>
        </>
      )}
    </div>
  );
}
