/**
 * CMOS Era — the technology that powers everything (1963).
 *
 * This tab shows how CMOS logic works through three sub-visualizations:
 *   1. CmosInverterDiagram — the complementary NMOS/PMOS pair
 *   2. VtcChart — the voltage transfer characteristic curve
 *   3. ScalingTimeline — Moore's Law technology scaling data
 *
 * Unlike the other eras which use a continuous slider, CMOS uses a
 * digital toggle (0/1) to demonstrate the binary switching behavior.
 */

import { useTranslation } from "@coding-adventures/ui-components";
import { EraHeader } from "../EraHeader.js";
import { EducationalNarrative } from "../EducationalNarrative.js";
import { VoltageReadout } from "../VoltageReadout.js";
import { CmosInverterDiagram } from "./CmosInverterDiagram.js";
import { VtcChart } from "./VtcChart.js";
import { ScalingTimeline } from "./ScalingTimeline.js";
import { useCmosSim } from "../../hooks/useTransistorSim.js";
import "../../styles/cmos.css";

export function CmosEra() {
  const { t } = useTranslation();
  const { inputDigital, setInputDigital, output, vtc, scaling } = useCmosSim();

  // Toggle handler — flip between 0 and 1
  const handleToggle = () => {
    setInputDigital(inputDigital === 0 ? 1 : 0);
  };

  const readoutItems = [
    {
      label: t("era4.readout.output"),
      value: output.logicValue.toString(),
    },
    {
      label: t("era4.readout.voltage"),
      value: `${output.voltage.toFixed(2)} V`,
    },
    {
      label: t("era4.readout.current"),
      value: `${(output.currentDraw * 1e6).toFixed(2)} uA`,
    },
    {
      label: t("era4.readout.power"),
      value: `${(output.powerDissipation * 1e6).toFixed(2)} uW`,
    },
    {
      label: t("era4.readout.nmosState"),
      value: inputDigital === 1 ? t("era4.readout.on") : t("era4.readout.off"),
    },
    {
      label: t("era4.readout.pmosState"),
      value: inputDigital === 0 ? t("era4.readout.on") : t("era4.readout.off"),
    },
  ];

  return (
    <section className="era era--cmos">
      <EraHeader eraKey="era4" />

      <div className="era__content">
        <div className="era__diagram-panel">
          <CmosInverterDiagram
            inputDigital={inputDigital}
          />

          {/* Digital toggle button instead of slider */}
          <div className="cmos-toggle">
            <span className="cmos-toggle__label">{t("era4.toggle.input")}</span>
            <button
              className={`cmos-toggle__button ${inputDigital === 1 ? "cmos-toggle__button--high" : ""}`}
              onClick={handleToggle}
              aria-pressed={inputDigital === 1}
            >
              {inputDigital}
            </button>
          </div>

          <VoltageReadout items={readoutItems} />
        </div>

        <div className="era__narrative-panel">
          <EducationalNarrative eraKey="era4" paragraphCount={3} />

          <VtcChart vtc={vtc} />
          <ScalingTimeline scaling={scaling} />
        </div>
      </div>
    </section>
  );
}
