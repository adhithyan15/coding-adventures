/**
 * Root application component.
 *
 * Renders a tabbed interface with four categories of arithmetic circuits,
 * building from simple adders up to CPU execution.
 *
 * === Layout ===
 *
 *   <header>   — Title and subtitle
 *   <nav>      — Tab buttons: Binary Adders | Everything is Addition | The ALU | CPU Step-Through
 *   <main>     — Active category's visualization panel
 *   <footer>   — Credit line
 *
 * === Central Theme ===
 *
 * "Everything reduces to addition." Subtraction is addition with two's
 * complement. Multiplication is shift-and-add. The adder is the CPU's
 * workhorse — everything else piggybacks on it.
 */

import { useState } from "react";
import { TabList, useTranslation } from "@coding-adventures/ui-components";
import { BinaryAdders } from "./components/adders/BinaryAdders.js";
import { EverythingIsAddition } from "./components/everything-is-addition/EverythingIsAddition.js";
import { ALUView } from "./components/alu/ALUView.js";
import { CpuView } from "./components/cpu/CpuView.js";

/** The four circuit categories, ordered from simple to complex. */
type TabId = "adders" | "addition" | "alu" | "cpu";

export function App() {
  const { t } = useTranslation();
  const [activeTab, setActiveTab] = useState<TabId>("adders");

  const tabs = [
    { id: "adders" as const, label: t("tabs.adders") },
    { id: "addition" as const, label: t("tabs.addition") },
    { id: "alu" as const, label: t("tabs.alu") },
    { id: "cpu" as const, label: t("tabs.cpu") },
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
        {activeTab === "adders" && <BinaryAdders />}
        {activeTab === "addition" && <EverythingIsAddition />}
        {activeTab === "alu" && <ALUView />}
        {activeTab === "cpu" && <CpuView />}
      </main>

      <footer className="app__footer">
        <p>{t("footer.credit")}</p>
      </footer>
    </div>
  );
}
