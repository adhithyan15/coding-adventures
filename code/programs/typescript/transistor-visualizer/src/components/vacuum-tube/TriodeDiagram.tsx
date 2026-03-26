/**
 * Triode Diagram — SVG cross-section of a vacuum tube triode.
 *
 * === Visual Structure ===
 *
 * The diagram shows a classic triode from left to right:
 *
 *   ┌─────────────────────────────────┐  ← Glass envelope (pale amber)
 *   │                                 │
 *   │   ═══════════════════════       │  ← Anode plate (dark gray)
 *   │                                 │
 *   │   ─ ─ ─ ─ ─ ─ ─ ─ ─ ─ ─       │  ← Grid mesh (color shifts with voltage)
 *   │   ─ ─ ─ ─ ─ ─ ─ ─ ─ ─ ─       │
 *   │                                 │
 *   │        /\/\/\/\/\/\             │  ← Cathode filament (orange-red, glows)
 *   │                                 │
 *   └─────────────────────────────────┘
 *
 * Particles (electrons) flow upward from the cathode through the grid
 * to the anode when the tube is conducting.
 *
 * === Dynamic Behavior ===
 *
 * - Grid color shifts from green (positive) to red (negative)
 * - Cathode filament has a glow effect (SVG filter)
 * - Particles spawn at the cathode and move toward the anode
 * - aria-label describes the current state
 */

import { useMemo } from "react";
import { useTranslation } from "@coding-adventures/ui-components";
import { useParticleSystem } from "../../hooks/useParticleSystem.js";
import { ParticleLayer } from "../ParticleLayer.js";
import type { ParticleConfig } from "../../lib/particle-system.js";

interface TriodeDiagramProps {
  gridVoltage: number;
  conducting: boolean;
}

export function TriodeDiagram({ gridVoltage, conducting }: TriodeDiagramProps) {
  const { t } = useTranslation();

  // Particle system configuration — electrons flow from cathode (bottom)
  // to anode (top) within the SVG coordinate space
  const particleConfig: ParticleConfig = useMemo(
    () => ({
      maxParticles: 30,
      // More particles when conducting strongly, zero when cut off
      spawnRate: conducting ? Math.min(3, 1 + gridVoltage * 0.2) : 0,
      // Cathode region at bottom of tube
      spawnRegion: { x: 120, y: 370, width: 160, height: 20 },
      // Anode region at top of tube
      targetRegion: { x: 100, y: 100, width: 200, height: 30 },
      speed: 3,
      particleRadius: 3,
      color: "#66ccff",
    }),
    [conducting, gridVoltage],
  );

  const { particles } = useParticleSystem(particleConfig);

  // Grid mesh color: shifts from red (negative voltage, blocking electrons)
  // to green (positive voltage, attracting electrons) based on grid voltage.
  // The normalized value maps -15V..+5V to 0..1.
  const gridNormalized = Math.max(0, Math.min(1, (gridVoltage + 15) / 20));
  const gridRed = Math.round(255 * (1 - gridNormalized));
  const gridGreen = Math.round(200 * gridNormalized);
  const gridColor = `rgb(${gridRed}, ${gridGreen}, 80)`;

  // Dynamic aria-label that describes the current state
  const ariaLabel = conducting
    ? `${t("era1.diagram.label")}. ${t("era1.readout.conducting")}.`
    : `${t("era1.diagram.label")}. ${t("era1.readout.cutoff")}.`;

  return (
    <svg
      viewBox="0 0 400 500"
      className="triode-diagram"
      role="img"
      aria-label={ariaLabel}
    >
      {/* SVG filter for cathode glow effect */}
      <defs>
        <filter id="cathode-glow">
          <feGaussianBlur stdDeviation="4" result="blur" />
          <feMerge>
            <feMergeNode in="blur" />
            <feMergeNode in="SourceGraphic" />
          </feMerge>
        </filter>
      </defs>

      {/* Glass envelope — rounded rectangle with pale amber fill */}
      <rect
        x="60"
        y="40"
        width="280"
        height="420"
        rx="40"
        ry="40"
        fill="#fdf5e6"
        stroke="#c8a882"
        strokeWidth="3"
        opacity="0.9"
      />

      {/* Inner vacuum region — slightly darker to show the glass wall */}
      <rect
        x="75"
        y="55"
        width="250"
        height="390"
        rx="30"
        ry="30"
        fill="#faf0dc"
        stroke="none"
      />

      {/* Anode plate — dark gray rectangle at the top.
          The anode collects electrons that pass through the grid. */}
      <rect
        x="100"
        y="100"
        width="200"
        height="30"
        fill="#555"
        stroke="#333"
        strokeWidth="2"
        rx="3"
      />
      <text x="200" y="90" textAnchor="middle" fontSize="12" fill="#666">
        Anode (Plate)
      </text>

      {/* Grid mesh — horizontal lines in the middle.
          The grid is a fine wire mesh that electrons must pass through.
          Its voltage determines whether electrons can reach the anode.
          Color shifts from red (blocking) to green (conducting). */}
      {[230, 245, 260, 275, 290].map((y) => (
        <line
          key={y}
          x1="110"
          y1={y}
          x2="290"
          y2={y}
          stroke={gridColor}
          strokeWidth="2"
          strokeDasharray="8 6"
        />
      ))}
      <text x="200" y="220" textAnchor="middle" fontSize="12" fill={gridColor}>
        Grid ({gridVoltage.toFixed(1)}V)
      </text>

      {/* Cathode filament — zigzag path at the bottom.
          The heated filament emits electrons via thermionic emission.
          Orange-red color with a glow filter to simulate incandescence. */}
      <path
        d="M 140,380 L 160,360 L 180,380 L 200,360 L 220,380 L 240,360 L 260,380"
        fill="none"
        stroke="#ff6633"
        strokeWidth="3"
        filter="url(#cathode-glow)"
      />
      <text x="200" y="410" textAnchor="middle" fontSize="12" fill="#cc5522">
        Cathode (Filament)
      </text>

      {/* Terminal leads — wires connecting to external circuit */}
      {/* Anode lead */}
      <line x1="200" y1="60" x2="200" y2="100" stroke="#555" strokeWidth="2" />
      {/* Grid lead */}
      <line x1="60" y1="260" x2="110" y2="260" stroke={gridColor} strokeWidth="2" />
      {/* Cathode leads */}
      <line x1="140" y1="380" x2="140" y2="440" stroke="#cc5522" strokeWidth="2" />
      <line x1="260" y1="380" x2="260" y2="440" stroke="#cc5522" strokeWidth="2" />

      {/* Electron particles flowing from cathode to anode */}
      <ParticleLayer
        particles={particles}
        radius={3}
        color="#66ccff"
      />
    </svg>
  );
}
