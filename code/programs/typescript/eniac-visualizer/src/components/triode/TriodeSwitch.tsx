/**
 * TriodeSwitch — Tab 1: the vacuum tube triode as a digital switch.
 *
 * Shows how a triode acts as an on/off switch controlled by grid voltage.
 * This bridges from the transistor visualizer to ENIAC-specific concepts.
 */

import { useState, useMemo } from "react";
import {
  triodePlateCurrent,
  isConducting,
} from "@coding-adventures/transistors";
import { useTranslation } from "@coding-adventures/ui-components";

export function TriodeSwitch() {
  const { t } = useTranslation();
  const [gridVoltage, setGridVoltage] = useState(0);

  const plateCurrent = useMemo(() => triodePlateCurrent(gridVoltage), [gridVoltage]);
  const conducting = useMemo(() => isConducting(gridVoltage), [gridVoltage]);
  const currentDisplay = `${(plateCurrent * 1000).toFixed(1)} mA`;

  return (
    <div className="triode-tab">
      <p className="triode-tab__intro">{t("triode.intro")}</p>

      <section className="eniac-card" aria-label={t("triode.ariaLabel")}>
        <h3 className="eniac-card__title">{t("triode.title")}</h3>

        {/* SVG Triode Diagram */}
        <svg className="triode-svg" viewBox="0 0 240 200" aria-hidden="true">
          {/* Glass envelope */}
          <ellipse cx="120" cy="100" rx="70" ry="85" fill="none" stroke="#555" strokeWidth="2" />

          {/* Cathode (bottom, heated filament) */}
          <line x1="90" y1="150" x2="150" y2="150" stroke={conducting ? "#ff6b35" : "#666"} strokeWidth="3" />
          <text x="120" y="175" textAnchor="middle" fill="#888" fontSize="11">Cathode (K)</text>

          {/* Grid (middle, wire mesh) */}
          {[85, 100, 115, 130, 145].map((x) => (
            <line key={x} x1={x} y1="110" x2={x} y2="120" stroke={gridVoltage > -12.5 ? "#4caf50" : "#666"} strokeWidth="1.5" />
          ))}
          <text x="55" y="120" textAnchor="end" fill="#888" fontSize="11">Grid (G)</text>

          {/* Plate/Anode (top) */}
          <rect x="95" y="50" width="50" height="10" fill={conducting ? "#4fc3f7" : "#444"} rx="2" />
          <text x="120" y="42" textAnchor="middle" fill="#888" fontSize="11">Plate (P)</text>

          {/* Electron flow arrows (when conducting) */}
          {conducting && (
            <>
              <line x1="110" y1="145" x2="110" y2="65" stroke="#ff6b35" strokeWidth="1" strokeDasharray="4,3" opacity="0.7" />
              <line x1="130" y1="145" x2="130" y2="65" stroke="#ff6b35" strokeWidth="1" strokeDasharray="4,3" opacity="0.7" />
              <text x="160" y="100" fill="#ff6b35" fontSize="9" opacity="0.8">e⁻ flow</text>
            </>
          )}
        </svg>

        {/* Grid voltage slider */}
        <div className="triode-controls">
          <label className="triode-slider__label" htmlFor="grid-voltage">
            {t("triode.gridVoltage")}: {gridVoltage.toFixed(1)}V
          </label>
          <input
            id="grid-voltage"
            type="range"
            min="-15"
            max="5"
            step="0.5"
            value={gridVoltage}
            onChange={(e) => setGridVoltage(Number(e.target.value))}
            className="triode-slider"
          />
        </div>

        {/* Readouts */}
        <div className="triode-readouts">
          <div className="triode-readout">
            <span className="triode-readout__label">{t("triode.plateCurrent")}</span>
            <span className="triode-readout__value">{currentDisplay}</span>
          </div>
          <div className="triode-readout">
            <span className="triode-readout__label">{t("triode.state")}</span>
            <span className={`triode-readout__value ${conducting ? "triode-readout__value--on" : "triode-readout__value--off"}`}>
              {conducting ? t("triode.conducting") : t("triode.cutoff")}
            </span>
          </div>
        </div>

        {/* Educational insight */}
        <div className="eniac-callout">
          <p>{t("triode.insight")}</p>
        </div>
      </section>
    </div>
  );
}
