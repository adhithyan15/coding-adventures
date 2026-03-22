/**
 * BJT Era — the solid-state revolution (1947).
 *
 * This tab shows how a Bipolar Junction Transistor works:
 *   - SliderControl adjusts base-emitter voltage (0 to 1.2V)
 *   - NpnDiagram shows the N-P-N layer structure with depletion regions
 *   - Particles flow from emitter through base to collector when conducting
 *   - Readout shows region, collector current, base current, and beta
 */

import {
  SliderControl,
  useTranslation,
} from "@coding-adventures/ui-components";
import { BJTRegion } from "@coding-adventures/transistors";
import { EraHeader } from "../EraHeader.js";
import { EducationalNarrative } from "../EducationalNarrative.js";
import { VoltageReadout } from "../VoltageReadout.js";
import { NpnDiagram } from "./NpnDiagram.js";
import { useBjtSim } from "../../hooks/useTransistorSim.js";
import "../../styles/bjt.css";

/**
 * Format a current value for display, automatically choosing
 * the most readable unit (A, mA, uA, nA).
 */
function formatCurrent(amps: number): string {
  if (amps === 0) return "0 A";
  if (amps >= 1) return `${amps.toFixed(2)} A`;
  if (amps >= 1e-3) return `${(amps * 1e3).toFixed(2)} mA`;
  if (amps >= 1e-6) return `${(amps * 1e6).toFixed(2)} uA`;
  return `${(amps * 1e9).toFixed(2)} nA`;
}

/**
 * Map the BJTRegion enum to an i18n key suffix.
 * The BJTRegion values are: "cutoff", "active", "saturation".
 */
function regionToKey(region: BJTRegion): string {
  switch (region) {
    case BJTRegion.CUTOFF:
      return "era2.readout.cutoff";
    case BJTRegion.ACTIVE:
      return "era2.readout.active";
    case BJTRegion.SATURATION:
      return "era2.readout.saturation";
  }
}

export function BjtEra() {
  const { t } = useTranslation();
  const { vbe, setVbe, region, ic, ib, conducting } = useBjtSim();

  // Calculate beta (current gain) — the ratio of collector to base current.
  // Only meaningful when both currents are nonzero.
  const beta = ib > 0 ? (ic / ib).toFixed(0) : "—";

  const readoutItems = [
    { label: t("era2.readout.region"), value: t(regionToKey(region)) },
    { label: t("era2.readout.collectorCurrent"), value: formatCurrent(ic) },
    { label: t("era2.readout.baseCurrent"), value: formatCurrent(ib) },
    { label: t("era2.readout.beta"), value: beta },
  ];

  return (
    <section className="era era--bjt">
      <EraHeader eraKey="era2" />

      <div className="era__content">
        <div className="era__diagram-panel">
          <NpnDiagram
            vbe={vbe}
            conducting={conducting}
            region={region}
          />

          <SliderControl
            label={t("era2.slider.baseVoltage")}
            value={vbe}
            min={0}
            max={1.2}
            step={0.01}
            onChange={setVbe}
            unit="V"
          />

          <VoltageReadout items={readoutItems} />
        </div>

        <div className="era__narrative-panel">
          <EducationalNarrative eraKey="era2" paragraphCount={3} />
        </div>
      </div>
    </section>
  );
}
