/**
 * NPN Diagram — SVG cross-section of an NPN bipolar junction transistor.
 *
 * === Visual Structure ===
 *
 *   ┌────────────┬──────┬────────────┐
 *   │   N-type   │P-type│   N-type   │
 *   │  (Emitter) │(Base)│ (Collector)│
 *   │            │      │            │
 *   │  light     │pink  │  light     │
 *   │  blue      │      │  blue      │
 *   └────────────┴──────┴────────────┘
 *
 * The three colored bands represent the semiconductor layers:
 *   - N-type (blue): excess free electrons (from phosphorus doping)
 *   - P-type (pink): excess holes (from boron doping)
 *
 * Depletion regions appear as hatched zones at the N-P junctions.
 * When Vbe exceeds ~0.7V, electrons flow from emitter through the
 * thin base to the collector.
 */

import { useMemo } from "react";
import { useTranslation } from "@coding-adventures/ui-components";
import { BJTRegion } from "@coding-adventures/transistors";
import { useParticleSystem } from "../../hooks/useParticleSystem.js";
import { ParticleLayer } from "../ParticleLayer.js";
import type { ParticleConfig } from "../../lib/particle-system.js";

interface NpnDiagramProps {
  vbe: number;
  conducting: boolean;
  region: BJTRegion;
}

export function NpnDiagram({ vbe, conducting, region }: NpnDiagramProps) {
  const { t } = useTranslation();

  // Particles flow from emitter (left) through base (center) to collector (right)
  const particleConfig: ParticleConfig = useMemo(
    () => ({
      maxParticles: 25,
      spawnRate: conducting ? Math.min(3, (vbe - 0.5) * 6) : 0,
      // Emitter region (left side)
      spawnRegion: { x: 30, y: 120, width: 80, height: 120 },
      // Collector region (right side)
      targetRegion: { x: 290, y: 120, width: 80, height: 120 },
      speed: 2.5,
      particleRadius: 3,
      color: "#4488ff",
    }),
    [conducting, vbe],
  );

  const { particles } = useParticleSystem(particleConfig);

  // Depletion region width narrows when forward-biased.
  // This visual cue shows how the barrier shrinks as Vbe increases.
  const depletionWidth = conducting ? 5 : 15;

  // Build a descriptive aria-label based on current state
  const regionLabel =
    region === BJTRegion.CUTOFF
      ? t("era2.readout.cutoff")
      : region === BJTRegion.ACTIVE
        ? t("era2.readout.active")
        : t("era2.readout.saturation");
  const ariaLabel = `${t("era2.diagram.label")}. ${regionLabel}.`;

  return (
    <svg
      viewBox="0 0 400 400"
      className="npn-diagram"
      role="img"
      aria-label={ariaLabel}
    >
      <defs>
        {/* Hatched pattern for depletion regions */}
        <pattern
          id="depletion-hatch"
          patternUnits="userSpaceOnUse"
          width="6"
          height="6"
          patternTransform="rotate(45)"
        >
          <line x1="0" y1="0" x2="0" y2="6" stroke="#999" strokeWidth="1.5" />
        </pattern>
      </defs>

      {/* === N-type Emitter region (left) ===
          Light blue represents N-type semiconductor (excess electrons
          from phosphorus doping). */}
      <rect
        x="30"
        y="80"
        width="120"
        height="200"
        fill="#cce0ff"
        stroke="#6699cc"
        strokeWidth="2"
        rx="4"
      />
      <text x="90" y="70" textAnchor="middle" fontSize="13" fill="#336699" fontWeight="bold">
        N
      </text>
      <text x="90" y="310" textAnchor="middle" fontSize="11" fill="#336699">
        Emitter
      </text>

      {/* === P-type Base region (center) ===
          Light pink represents P-type semiconductor (excess holes
          from boron doping). The base is intentionally thin — this is
          the key to BJT operation. Most electrons pass through without
          recombining with holes. */}
      <rect
        x="150"
        y="80"
        width="100"
        height="200"
        fill="#ffcccc"
        stroke="#cc6666"
        strokeWidth="2"
        rx="4"
      />
      <text x="200" y="70" textAnchor="middle" fontSize="13" fill="#993333" fontWeight="bold">
        P
      </text>
      <text x="200" y="310" textAnchor="middle" fontSize="11" fill="#993333">
        Base
      </text>

      {/* === N-type Collector region (right) === */}
      <rect
        x="250"
        y="80"
        width="120"
        height="200"
        fill="#cce0ff"
        stroke="#6699cc"
        strokeWidth="2"
        rx="4"
      />
      <text x="310" y="70" textAnchor="middle" fontSize="13" fill="#336699" fontWeight="bold">
        N
      </text>
      <text x="310" y="310" textAnchor="middle" fontSize="11" fill="#336699">
        Collector
      </text>

      {/* === Depletion regions at N-P junctions ===
          These are the charge-free zones where electrons and holes have
          recombined. The width shrinks when forward-biased (Vbe > 0.7V). */}
      {/* Emitter-Base junction */}
      <rect
        x={150 - depletionWidth}
        y="80"
        width={depletionWidth * 2}
        height="200"
        fill="url(#depletion-hatch)"
        opacity="0.5"
      />
      {/* Base-Collector junction */}
      <rect
        x={250 - depletionWidth}
        y="80"
        width={depletionWidth * 2}
        height="200"
        fill="url(#depletion-hatch)"
        opacity="0.5"
      />

      {/* Terminal connections */}
      <line x1="90" y1="280" x2="90" y2="350" stroke="#336699" strokeWidth="2" />
      <text x="90" y="370" textAnchor="middle" fontSize="10" fill="#336699">E</text>

      <line x1="200" y1="280" x2="200" y2="350" stroke="#993333" strokeWidth="2" />
      <text x="200" y="370" textAnchor="middle" fontSize="10" fill="#993333">B</text>

      <line x1="310" y1="280" x2="310" y2="350" stroke="#336699" strokeWidth="2" />
      <text x="310" y="370" textAnchor="middle" fontSize="10" fill="#336699">C</text>

      {/* Electron flow particles */}
      <ParticleLayer
        particles={particles}
        radius={3}
        color="#4488ff"
      />
    </svg>
  );
}
