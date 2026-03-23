/**
 * CMOS Inverter Diagram — SVG schematic of a complementary transistor pair.
 *
 * === Visual Structure ===
 *
 *               Vdd
 *                │
 *         ┌──────┴──────┐
 *         │   PMOS      │  ← P-type regions (pink) in N-well
 *    Input─┤   (gate)    ├─── Output
 *         │   NMOS      │  ← N-type regions (blue) in P-substrate
 *         └──────┬──────┘
 *                │
 *               GND
 *
 * When input = 0: PMOS ON, NMOS OFF -> Output = Vdd (1)
 * When input = 1: NMOS ON, PMOS OFF -> Output = GND (0)
 *
 * The active transistor is highlighted and particles flow through it.
 * The inactive transistor is dimmed.
 */

import { useMemo } from "react";
import { useTranslation } from "@coding-adventures/ui-components";
import { useParticleSystem } from "../../hooks/useParticleSystem.js";
import { ParticleLayer } from "../ParticleLayer.js";
import type { ParticleConfig } from "../../lib/particle-system.js";

interface CmosInverterDiagramProps {
  inputDigital: 0 | 1;
}

export function CmosInverterDiagram({ inputDigital }: CmosInverterDiagramProps) {
  const { t } = useTranslation();

  const nmosOn = inputDigital === 1;
  const pmosOn = inputDigital === 0;

  // Particle config — current flows through the active transistor
  const particleConfig: ParticleConfig = useMemo(() => {
    if (nmosOn) {
      // NMOS on: current flows from output through NMOS to GND
      return {
        maxParticles: 15,
        spawnRate: 2,
        spawnRegion: { x: 300, y: 190, width: 40, height: 10 },
        targetRegion: { x: 300, y: 350, width: 40, height: 10 },
        speed: 2,
        particleRadius: 2.5,
        color: "#4488ff",
      };
    }
    // PMOS on: current flows from Vdd through PMOS to output
    return {
      maxParticles: 15,
      spawnRate: 2,
      spawnRegion: { x: 300, y: 30, width: 40, height: 10 },
      targetRegion: { x: 300, y: 170, width: 40, height: 10 },
      speed: 2,
      particleRadius: 2.5,
      color: "#ff6688",
    };
  }, [nmosOn]);

  const { particles } = useParticleSystem(particleConfig);

  const ariaLabel = `${t("era4.diagram.label")}. ${t("era4.toggle.input")} = ${inputDigital}, ${t("era4.readout.output")} = ${inputDigital === 0 ? 1 : 0}.`;

  return (
    <svg
      viewBox="0 0 500 400"
      className="cmos-diagram"
      role="img"
      aria-label={ariaLabel}
    >
      {/* === Power Rails === */}
      {/* Vdd rail at top */}
      <line x1="100" y1="30" x2="400" y2="30" stroke="#cc3333" strokeWidth="3" />
      <text x="80" y="35" textAnchor="end" fontSize="14" fill="#cc3333" fontWeight="bold">
        Vdd
      </text>

      {/* GND rail at bottom */}
      <line x1="100" y1="370" x2="400" y2="370" stroke="#333" strokeWidth="3" />
      <text x="80" y="375" textAnchor="end" fontSize="14" fill="#333" fontWeight="bold">
        GND
      </text>

      {/* === PMOS transistor (top half) ===
          PMOS is ON when input is LOW (0). It pulls the output to Vdd. */}
      <g opacity={pmosOn ? 1 : 0.3}>
        {/* N-well (light blue background for PMOS) */}
        <rect
          x="200"
          y="50"
          width="200"
          height="120"
          fill="#e8f0ff"
          stroke="#99aacc"
          strokeWidth="1.5"
          rx="6"
        />
        <text x="210" y="68" fontSize="9" fill="#6688aa">
          N-well
        </text>

        {/* P-type source region */}
        <rect x="220" y="80" width="50" height="60" fill="#ffcccc" stroke="#cc6666" strokeWidth="1.5" rx="3" />
        <text x="245" y="115" textAnchor="middle" fontSize="9" fill="#993333">P</text>

        {/* P-type drain region */}
        <rect x="330" y="80" width="50" height="60" fill="#ffcccc" stroke="#cc6666" strokeWidth="1.5" rx="3" />
        <text x="355" y="115" textAnchor="middle" fontSize="9" fill="#993333">P</text>

        {/* Gate oxide + gate */}
        <rect x="270" y="85" width="60" height="8" fill="#ffdd88" stroke="#cc9944" strokeWidth="1" rx="1" />
        <rect x="275" y="70" width="50" height="15" fill="#999" stroke="#666" strokeWidth="1.5" rx="3" />
      </g>

      {/* PMOS label */}
      <text x="300" y="155" textAnchor="middle" fontSize="11" fill="#993366" fontWeight="bold">
        PMOS {pmosOn ? t("era4.readout.on") : t("era4.readout.off")}
      </text>

      {/* === NMOS transistor (bottom half) ===
          NMOS is ON when input is HIGH (1). It pulls the output to GND. */}
      <g opacity={nmosOn ? 1 : 0.3}>
        {/* P-substrate region */}
        <rect
          x="200"
          y="220"
          width="200"
          height="120"
          fill="#ffe8e8"
          stroke="#ccaa99"
          strokeWidth="1.5"
          rx="6"
        />
        <text x="210" y="238" fontSize="9" fill="#aa8866">
          P-sub
        </text>

        {/* N-type source region */}
        <rect x="220" y="250" width="50" height="60" fill="#cce0ff" stroke="#6699cc" strokeWidth="1.5" rx="3" />
        <text x="245" y="285" textAnchor="middle" fontSize="9" fill="#336699">N</text>

        {/* N-type drain region */}
        <rect x="330" y="250" width="50" height="60" fill="#cce0ff" stroke="#6699cc" strokeWidth="1.5" rx="3" />
        <text x="355" y="285" textAnchor="middle" fontSize="9" fill="#336699">N</text>

        {/* Gate oxide + gate */}
        <rect x="270" y="255" width="60" height="8" fill="#ffdd88" stroke="#cc9944" strokeWidth="1" rx="1" />
        <rect x="275" y="240" width="50" height="15" fill="#999" stroke="#666" strokeWidth="1.5" rx="3" />
      </g>

      {/* NMOS label */}
      <text x="300" y="210" textAnchor="middle" fontSize="11" fill="#336699" fontWeight="bold">
        NMOS {nmosOn ? t("era4.readout.on") : t("era4.readout.off")}
      </text>

      {/* === Wiring === */}
      {/* Vdd to PMOS source */}
      <line x1="245" y1="30" x2="245" y2="80" stroke="#cc3333" strokeWidth="2" />

      {/* PMOS drain to output node */}
      <line x1="355" y1="140" x2="355" y2="190" stroke="#666" strokeWidth="2" />

      {/* Output node to NMOS drain */}
      <line x1="355" y1="190" x2="355" y2="250" stroke="#666" strokeWidth="2" />

      {/* NMOS source to GND */}
      <line x1="245" y1="310" x2="245" y2="370" stroke="#333" strokeWidth="2" />

      {/* Output wire (right) */}
      <line x1="355" y1="190" x2="450" y2="190" stroke="#666" strokeWidth="2" />
      <text x="460" y="195" fontSize="13" fill="#333" fontWeight="bold">
        {t("era4.readout.output")}
      </text>

      {/* Input wire — connected to both gates */}
      <line x1="50" y1="190" x2="140" y2="190" stroke="#666" strokeWidth="2" />
      {/* To PMOS gate */}
      <line x1="140" y1="190" x2="140" y2="77" stroke="#666" strokeWidth="1.5" />
      <line x1="140" y1="77" x2="275" y2="77" stroke="#666" strokeWidth="1.5" />
      {/* To NMOS gate */}
      <line x1="140" y1="190" x2="140" y2="247" stroke="#666" strokeWidth="1.5" />
      <line x1="140" y1="247" x2="275" y2="247" stroke="#666" strokeWidth="1.5" />

      <text x="40" y="195" textAnchor="end" fontSize="13" fill="#333" fontWeight="bold">
        {t("era4.toggle.input")}
      </text>

      {/* Input value indicator */}
      <circle cx="55" cy="190" r="14" fill={inputDigital === 1 ? "#44aa44" : "#cc4444"} />
      <text x="55" y="195" textAnchor="middle" fontSize="14" fill="#fff" fontWeight="bold">
        {inputDigital}
      </text>

      {/* Output value indicator */}
      <circle cx="450" cy="190" r="14" fill={inputDigital === 0 ? "#44aa44" : "#cc4444"} />
      <text x="450" y="195" textAnchor="middle" fontSize="14" fill="#fff" fontWeight="bold">
        {inputDigital === 0 ? 1 : 0}
      </text>

      {/* Current path highlight through active transistor */}
      {pmosOn && (
        <line
          x1="245"
          y1="30"
          x2="355"
          y2="190"
          stroke="#ff6688"
          strokeWidth="2"
          strokeDasharray="6 4"
          opacity="0.5"
        />
      )}
      {nmosOn && (
        <line
          x1="355"
          y1="190"
          x2="245"
          y2="370"
          stroke="#4488ff"
          strokeWidth="2"
          strokeDasharray="6 4"
          opacity="0.5"
        />
      )}

      {/* Electron flow particles */}
      <ParticleLayer
        particles={particles}
        radius={2.5}
        color={nmosOn ? "#4488ff" : "#ff6688"}
      />
    </svg>
  );
}
