/**
 * Root application component for the ENIAC visualizer.
 *
 * 4 tabs tracing how ENIAC did decimal arithmetic with vacuum tubes:
 * 1. The Triode Switch — vacuum tube as digital on/off
 * 2. Decade Ring Counter — 10 tubes = one decimal digit
 * 3. ENIAC Accumulator — chained ring counters for addition
 * 4. ENIAC vs Binary — side-by-side comparison with modern approach
 */

import { useState } from "react";
import { TabList, useTranslation } from "@coding-adventures/ui-components";
import { TriodeSwitch } from "./components/triode/TriodeSwitch.js";
import { RingCounterView } from "./components/ring-counter/RingCounterView.js";
import { AccumulatorView } from "./components/accumulator/AccumulatorView.js";
import { ComparisonView } from "./components/comparison/ComparisonView.js";

type TabId = "triode" | "ring" | "accumulator" | "comparison";

export function App() {
  const { t } = useTranslation();
  const [activeTab, setActiveTab] = useState<TabId>("triode");

  const tabs = [
    { id: "triode" as const, label: t("tabs.triode") },
    { id: "ring" as const, label: t("tabs.ring") },
    { id: "accumulator" as const, label: t("tabs.accumulator") },
    { id: "comparison" as const, label: t("tabs.comparison") },
  ];

  return (
    <div className="app">
      <header className="app__header">
        <h1 className="app__title">{t("app.title")}</h1>
        <p className="app__subtitle">{t("app.subtitle")}</p>
      </header>

      <TabList
        items={tabs}
        activeTab={activeTab}
        onActiveChange={setActiveTab}
        ariaLabel={t("nav.tabs")}
      />

      <main className="app__main">
        {activeTab === "triode" && <TriodeSwitch />}
        {activeTab === "ring" && <RingCounterView />}
        {activeTab === "accumulator" && <AccumulatorView />}
        {activeTab === "comparison" && <ComparisonView />}
      </main>

      <footer className="app__footer">
        <p>{t("footer.credit")}</p>
      </footer>
    </div>
  );
}
