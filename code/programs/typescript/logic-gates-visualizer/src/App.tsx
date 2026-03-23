/**
 * Root application component.
 *
 * Renders a tabbed interface with four categories of digital logic circuits.
 * Uses the shared TabList component from ui-components for accessible tab
 * navigation (full keyboard support, ARIA roles, roving tabindex).
 *
 * === Layout ===
 *
 *   <header>   — Title and subtitle
 *   <nav>      — Tab buttons: Basic Gates | NAND Universality | Combinational | Sequential
 *   <main>     — Active category's visualization panel
 *   <footer>   — Credit line
 *
 * === Implemented tabs ===
 *
 * Tab 1 — Basic Gates: NOT, AND, OR, XOR with truth tables and CMOS panels
 * Tab 2 — NAND Universality: every gate built from NAND alone
 * Tab 3 — Combinational Logic: MUX, decoder, priority encoder
 *
 * Tab 4 shows a placeholder message until its PR lands.
 */

import { useState } from "react";
import { TabList, useTranslation } from "@coding-adventures/ui-components";
import { FundamentalGates } from "./components/fundamental/FundamentalGates.js";
import { NandUniversality } from "./components/nand-universality/NandUniversality.js";
import { CombinationalLogic } from "./components/combinational/CombinationalLogic.js";

/** The four circuit categories, ordered from simple to complex. */
type TabId = "fundamental" | "nand" | "combinational" | "sequential";

export function App() {
  const { t } = useTranslation();
  const [activeTab, setActiveTab] = useState<TabId>("fundamental");

  // Tab items with translated labels.
  const tabs = [
    { id: "fundamental" as const, label: t("tabs.fundamental") },
    { id: "nand" as const, label: t("tabs.nand") },
    { id: "combinational" as const, label: t("tabs.combinational") },
    { id: "sequential" as const, label: t("tabs.sequential") },
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
        {activeTab === "fundamental" && <FundamentalGates />}
        {activeTab === "nand" && <NandUniversality />}
        {activeTab === "combinational" && <CombinationalLogic />}
        {activeTab === "sequential" && (
          <div className="placeholder">
            <p>{t("placeholder.comingSoon")}</p>
          </div>
        )}
      </main>

      <footer className="app__footer">
        <p>{t("footer.credit")}</p>
      </footer>
    </div>
  );
}
