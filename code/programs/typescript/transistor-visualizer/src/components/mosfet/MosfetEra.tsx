/**
 * MOSFET Era — the field-effect revolution (1959).
 *
 * This tab shows how an NMOS transistor works:
 *   - SliderControl adjusts gate-source voltage (0 to 3.3V)
 *   - NmosDiagram shows the cross-section with substrate, wells, oxide, gate
 *   - An inversion channel forms under the gate when Vgs > Vth
 *   - Particles flow through the channel from source to drain
 *   - Readout shows region, drain current, and channel state
 */

import {
  SliderControl,
  useTranslation,
} from "@coding-adventures/ui-components";
import { MOSFETRegion } from "@coding-adventures/transistors";
import { EraHeader } from "../EraHeader.js";
import { EducationalNarrative } from "../EducationalNarrative.js";
import { VoltageReadout } from "../VoltageReadout.js";
import { NmosDiagram } from "./NmosDiagram.js";
import { useMosfetSim } from "../../hooks/useTransistorSim.js";
import "../../styles/mosfet.css";

/** Format current for display, auto-selecting appropriate unit. */
function formatCurrent(amps: number): string {
  if (amps === 0) return "0 A";
  if (amps >= 1) return `${amps.toFixed(2)} A`;
  if (amps >= 1e-3) return `${(amps * 1e3).toFixed(2)} mA`;
  if (amps >= 1e-6) return `${(amps * 1e6).toFixed(2)} uA`;
  return `${(amps * 1e9).toFixed(2)} nA`;
}

/** Map MOSFETRegion enum to i18n key. */
function regionToKey(region: MOSFETRegion): string {
  switch (region) {
    case MOSFETRegion.CUTOFF:
      return "era3.readout.cutoff";
    case MOSFETRegion.LINEAR:
      return "era3.readout.linear";
    case MOSFETRegion.SATURATION:
      return "era3.readout.saturation";
  }
}

export function MosfetEra() {
  const { t } = useTranslation();
  const { vgs, setVgs, region, ids, conducting } = useMosfetSim();

  const readoutItems = [
    { label: t("era3.readout.region"), value: t(regionToKey(region)) },
    { label: t("era3.readout.drainCurrent"), value: formatCurrent(ids) },
    {
      label: t("era3.readout.channel"),
      value: conducting
        ? t("era3.readout.channelFormed")
        : t("era3.readout.noChannel"),
    },
  ];

  return (
    <section className="era era--mosfet">
      <EraHeader eraKey="era3" />

      <div className="era__content">
        <div className="era__diagram-panel">
          <NmosDiagram
            vgs={vgs}
            conducting={conducting}
            region={region}
          />

          <SliderControl
            label={t("era3.slider.gateVoltage")}
            value={vgs}
            min={0}
            max={3.3}
            step={0.05}
            onChange={setVgs}
            unit="V"
          />

          <VoltageReadout items={readoutItems} />
        </div>

        <div className="era__narrative-panel">
          <EducationalNarrative eraKey="era3" paragraphCount={3} />
        </div>
      </div>
    </section>
  );
}
