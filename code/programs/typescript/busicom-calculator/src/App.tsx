/**
 * Root application component.
 *
 * Manages which visualization layer is active and provides the layout
 * shell: a header with layer tabs, the active layer panel, and a footer
 * with educational context.
 *
 * The six layers mirror the computing stack from user-facing calculator
 * down to CMOS transistors, plus timing:
 *
 *   Layer 1: Calculator    — click buttons, see results
 *   Layer 2: CPU State     — registers, PC, instruction trace
 *   Layer 3: ALU Detail    — ripple carry adder chain
 *   Layer 4: Gate Level    — AND/OR/NOT gate activations
 *   Layer 5: Transistor    — CMOS NAND/NOR at the bottom
 *   Layer 6: Timing        — clock waveform, phase activity, instruction timeline
 *
 * The ExecutionFlow component shows on the Calculator tab — a vertical
 * pipeline from key press all the way down to transistor switching.
 */

import { useState, useCallback, useRef } from "react";
import { useTranslation } from "./i18n/index.js";
import { Calculator } from "./components/calculator/Calculator.js";
import { CpuView } from "./components/cpu-view/CpuView.js";
import { AluView } from "./components/alu-view/AluView.js";
import { GateView } from "./components/gate-view/GateView.js";
import { TransistorView } from "./components/transistor-view/TransistorView.js";
import { TimingView } from "./components/timing-view/TimingView.js";
import { ExecutionFlow } from "./components/execution-flow/ExecutionFlow.js";
import { useCalculator } from "./hooks/useCalculator.js";

/** The six visualization layers, from highest abstraction to lowest. */
type Layer = "calculator" | "cpu" | "alu" | "gate" | "transistor" | "timing";

export function App() {
  const { t } = useTranslation();
  const [activeLayer, setActiveLayer] = useState<Layer>("calculator");
  const calculator = useCalculator();

  const layers: Array<{ id: Layer; labelKey: string }> = [
    { id: "calculator", labelKey: "layer.calculator" },
    { id: "cpu", labelKey: "layer.cpu" },
    { id: "alu", labelKey: "layer.alu" },
    { id: "gate", labelKey: "layer.gate" },
    { id: "transistor", labelKey: "layer.transistor" },
    { id: "timing", labelKey: "layer.timing" },
  ];

  const tabListRef = useRef<HTMLElement>(null);

  const handleLayerChange = useCallback((layer: Layer) => {
    setActiveLayer(layer);
  }, []);

  /** Arrow key navigation within the tab list (WAI-ARIA Tabs pattern). */
  const handleTabKeyDown = useCallback(
    (e: React.KeyboardEvent) => {
      const currentIndex = layers.findIndex((l) => l.id === activeLayer);
      let nextIndex = currentIndex;

      if (e.key === "ArrowRight" || e.key === "ArrowDown") {
        nextIndex = (currentIndex + 1) % layers.length;
      } else if (e.key === "ArrowLeft" || e.key === "ArrowUp") {
        nextIndex = (currentIndex - 1 + layers.length) % layers.length;
      } else if (e.key === "Home") {
        nextIndex = 0;
      } else if (e.key === "End") {
        nextIndex = layers.length - 1;
      } else {
        return;
      }

      e.preventDefault();
      setActiveLayer(layers[nextIndex]!.id);

      // Focus the newly active tab
      const tabList = tabListRef.current;
      if (tabList) {
        const buttons = tabList.querySelectorAll<HTMLButtonElement>("[role=tab]");
        buttons[nextIndex]?.focus();
      }
    },
    [activeLayer, layers],
  );

  return (
    <div className="app">
      <header className="app-header" role="banner">
        <h1>{t("app.title")}</h1>
        <p className="app-subtitle">{t("app.subtitle")}</p>
        <nav
          className="layer-tabs"
          role="tablist"
          aria-label={t("nav.layers")}
          ref={tabListRef}
          onKeyDown={handleTabKeyDown}
        >
          {layers.map((layer) => (
            <button
              key={layer.id}
              role="tab"
              aria-selected={activeLayer === layer.id}
              aria-controls={`panel-${layer.id}`}
              tabIndex={activeLayer === layer.id ? 0 : -1}
              className={`layer-tab ${activeLayer === layer.id ? "layer-tab--active" : ""}`}
              onClick={() => handleLayerChange(layer.id)}
            >
              {t(layer.labelKey)}
            </button>
          ))}
        </nav>
      </header>

      <main className="layer-panel" role="tabpanel" id={`panel-${activeLayer}`}>
        {activeLayer === "calculator" && (
          <>
            <Calculator calculator={calculator} />
            <ExecutionFlow
              trace={calculator.lastTrace}
              traceHistory={calculator.traceHistory}
              displayDigits={calculator.displayDigits}
              pc={calculator.pc}
              accumulator={calculator.accumulator}
            />
          </>
        )}
        {activeLayer === "cpu" && (
          <CpuView calculator={calculator} />
        )}
        {activeLayer === "alu" && (
          <AluView
            trace={calculator.lastTrace}
            traceHistory={calculator.traceHistory}
          />
        )}
        {activeLayer === "gate" && (
          <GateView
            trace={calculator.lastTrace}
            traceHistory={calculator.traceHistory}
          />
        )}
        {activeLayer === "transistor" && (
          <TransistorView
            trace={calculator.lastTrace}
            traceHistory={calculator.traceHistory}
          />
        )}
        {activeLayer === "timing" && (
          <TimingView
            trace={calculator.lastTrace}
            traceHistory={calculator.traceHistory}
          />
        )}
      </main>

      <footer className="app-footer" role="contentinfo">
        <p>{t("footer.credit")}</p>
      </footer>
    </div>
  );
}
