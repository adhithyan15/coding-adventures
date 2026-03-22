/**
 * Vacuum Tube Era — the first electronic amplifier (1906).
 *
 * This tab shows how Lee De Forest's triode vacuum tube works:
 *   - A SliderControl adjusts the grid voltage (-15V to +5V)
 *   - The TriodeDiagram SVG shows the internal structure
 *   - Particles flow from cathode to anode when conducting
 *   - VoltageReadout shows plate current and conducting state
 *   - EducationalNarrative explains the history and physics
 */

import { useState, useMemo } from "react";
import {
  SliderControl,
  useTranslation,
} from "@coding-adventures/ui-components";
import { EraHeader } from "../EraHeader.js";
import { EducationalNarrative } from "../EducationalNarrative.js";
import { VoltageReadout } from "../VoltageReadout.js";
import { TriodeDiagram } from "./TriodeDiagram.js";
import {
  triodePlateCurrent,
  isConducting,
} from "../../lib/vacuum-tube-model.js";
import "../../styles/vacuum-tube.css";

export function VacuumTubeEra() {
  const { t } = useTranslation();
  const [gridVoltage, setGridVoltage] = useState(0);

  // Compute derived values from the triode model
  const plateCurrent = useMemo(
    () => triodePlateCurrent(gridVoltage),
    [gridVoltage],
  );
  const conducting = useMemo(
    () => isConducting(gridVoltage),
    [gridVoltage],
  );

  // Format plate current for display.
  // Show in mA for readability — typical triode currents are 1-100mA.
  const currentDisplay = `${(plateCurrent * 1000).toFixed(1)} mA`;

  // Build readout items — all labels from i18n
  const readoutItems = [
    {
      label: t("era1.readout.plateCurrent"),
      value: currentDisplay,
    },
    {
      label: t("era1.readout.state"),
      value: conducting ? t("era1.readout.conducting") : t("era1.readout.cutoff"),
    },
  ];

  return (
    <section className="era era--vacuum-tube">
      <EraHeader eraKey="era1" />

      <div className="era__content">
        <div className="era__diagram-panel">
          <TriodeDiagram
            gridVoltage={gridVoltage}
            conducting={conducting}
          />

          <SliderControl
            label={t("era1.slider.gridVoltage")}
            value={gridVoltage}
            min={-15}
            max={5}
            step={0.5}
            onChange={setGridVoltage}
            unit="V"
          />

          <VoltageReadout items={readoutItems} />
        </div>

        <div className="era__narrative-panel">
          <EducationalNarrative eraKey="era1" paragraphCount={3} />
        </div>
      </div>
    </section>
  );
}
