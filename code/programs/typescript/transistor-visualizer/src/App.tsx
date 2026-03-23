/**
 * Root application component.
 *
 * Renders a tabbed interface with four historical eras of transistor technology.
 * Uses the shared TabList component from ui-components for accessible tab
 * navigation (full keyboard support, ARIA roles, roving tabindex).
 *
 * === Layout ===
 *
 *   <header>   — Title and subtitle
 *   <nav>      — Tab buttons: Vacuum Tube | BJT | MOSFET | CMOS
 *   <main>     — Active era's visualization panel
 *   <footer>   — Credit line
 */

import { useState } from "react";
import { TabList, useTranslation } from "@coding-adventures/ui-components";
import { VacuumTubeEra } from "./components/vacuum-tube/VacuumTubeEra.js";
import { BjtEra } from "./components/bjt/BjtEra.js";
import { MosfetEra } from "./components/mosfet/MosfetEra.js";
import { CmosEra } from "./components/cmos/CmosEra.js";

/** The four transistor eras, ordered chronologically. */
type EraId = "vacuum-tube" | "bjt" | "mosfet" | "cmos";

export function App() {
  const { t } = useTranslation();
  const [activeTab, setActiveTab] = useState<EraId>("vacuum-tube");

  // Tab items with translated labels.
  // The TabList component expects labels to be pre-translated.
  const tabs = [
    { id: "vacuum-tube" as const, label: t("tabs.vacuumTube") },
    { id: "bjt" as const, label: t("tabs.bjt") },
    { id: "mosfet" as const, label: t("tabs.mosfet") },
    { id: "cmos" as const, label: t("tabs.cmos") },
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
        ariaLabel={t("nav.eras")}
      />

      <main className="app__main">
        {activeTab === "vacuum-tube" && <VacuumTubeEra />}
        {activeTab === "bjt" && <BjtEra />}
        {activeTab === "mosfet" && <MosfetEra />}
        {activeTab === "cmos" && <CmosEra />}
      </main>

      <footer className="app__footer">
        <p>{t("footer.credit")}</p>
      </footer>
    </div>
  );
}
