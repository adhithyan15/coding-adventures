/**
 * NMOS Diagram — SVG cross-section of an NMOS transistor.
 *
 * === Visual Structure ===
 *
 *                    Gate (metal)
 *                   ┌──────────┐
 *                   │  Metal   │
 *              ═════╧══════════╧═════  ← SiO2 oxide layer (yellow)
 *   ┌─────────┐                     ┌─────────┐
 *   │ N-type  │    P-type substrate  │ N-type  │
 *   │ Source  │    (pink background) │  Drain  │
 *   │ (blue)  │    ● B  ● B  ● B    │ (blue)  │
 *   │ ○ P ○ P │                     │ ○ P ○ P │
 *   └─────────┘                     └─────────┘
 *
 * Key features:
 *   - P-type substrate in pink (background)
 *   - N-type source and drain wells in blue
 *   - SiO2 gate oxide as a thin yellow/orange strip
 *   - Metal gate on top of the oxide
 *   - Dopant atoms: "P" circles in N regions, "B" circles in P region
 *   - Inversion channel appears under the gate when Vgs > Vth
 */

import { useMemo } from "react";
import { useTranslation } from "@coding-adventures/ui-components";
import { MOSFETRegion } from "@coding-adventures/transistors";
import { useParticleSystem } from "../../hooks/useParticleSystem.js";
import { ParticleLayer } from "../ParticleLayer.js";
import type { ParticleConfig } from "../../lib/particle-system.js";

interface NmosDiagramProps {
  vgs: number;
  conducting: boolean;
  region: MOSFETRegion;
}

export function NmosDiagram({ vgs, conducting, region }: NmosDiagramProps) {
  const { t } = useTranslation();

  // Particles flow from source (left) through the channel to drain (right)
  const particleConfig: ParticleConfig = useMemo(
    () => ({
      maxParticles: 20,
      spawnRate: conducting ? Math.min(3, (vgs - 0.3) * 3) : 0,
      // Source well (left side)
      spawnRegion: { x: 40, y: 180, width: 80, height: 40 },
      // Drain well (right side)
      targetRegion: { x: 330, y: 180, width: 80, height: 40 },
      speed: 2,
      particleRadius: 2.5,
      color: "#4488ff",
    }),
    [conducting, vgs],
  );

  const { particles } = useParticleSystem(particleConfig);

  // The inversion channel appears when Vgs exceeds the threshold voltage (0.4V).
  // Its opacity increases with Vgs to show the channel strengthening.
  const channelOpacity = conducting ? Math.min(1, (vgs - 0.4) * 2) : 0;

  const regionLabel =
    region === MOSFETRegion.CUTOFF
      ? t("era3.readout.cutoff")
      : region === MOSFETRegion.LINEAR
        ? t("era3.readout.linear")
        : t("era3.readout.saturation");
  const ariaLabel = `${t("era3.diagram.label")}. ${regionLabel}.`;

  return (
    <svg
      viewBox="0 0 450 350"
      className="nmos-diagram"
      role="img"
      aria-label={ariaLabel}
    >
      {/* P-type substrate — the pink background silicon wafer.
          Boron atoms are added (doped) to create "holes" — the absence
          of electrons that acts as a positive charge carrier. */}
      <rect
        x="20"
        y="130"
        width="410"
        height="180"
        fill="#ffe0e0"
        stroke="#cc9999"
        strokeWidth="2"
        rx="4"
      />
      <text x="225" y="290" textAnchor="middle" fontSize="11" fill="#996666">
        P-type Substrate
      </text>

      {/* Boron dopant atoms in the P-type substrate.
          Boron has 3 valence electrons vs silicon's 4, creating a "hole". */}
      {[
        [160, 250],
        [225, 260],
        [290, 245],
        [190, 275],
        [260, 280],
      ].map(([cx, cy], i) => (
        <g key={`b-${i}`}>
          <circle cx={cx} cy={cy} r="6" fill="#ff8888" stroke="#cc6666" strokeWidth="1" />
          <text x={cx} y={cy! + 4} textAnchor="middle" fontSize="8" fill="#fff">
            B
          </text>
        </g>
      ))}

      {/* N-type Source well (left) — phosphorus-doped region with
          excess free electrons. These electrons are the charge carriers. */}
      <rect
        x="40"
        y="140"
        width="100"
        height="90"
        fill="#cce0ff"
        stroke="#6699cc"
        strokeWidth="2"
        rx="4"
      />
      <text x="90" y="252" textAnchor="middle" fontSize="11" fill="#336699">
        Source (N)
      </text>

      {/* Phosphorus dopant atoms in the source */}
      {[
        [60, 170],
        [100, 185],
        [80, 200],
      ].map(([cx, cy], i) => (
        <g key={`ps-${i}`}>
          <circle cx={cx} cy={cy} r="6" fill="#88aaff" stroke="#6688cc" strokeWidth="1" />
          <text x={cx} y={cy! + 4} textAnchor="middle" fontSize="8" fill="#fff">
            P
          </text>
        </g>
      ))}

      {/* N-type Drain well (right) */}
      <rect
        x="310"
        y="140"
        width="100"
        height="90"
        fill="#cce0ff"
        stroke="#6699cc"
        strokeWidth="2"
        rx="4"
      />
      <text x="360" y="252" textAnchor="middle" fontSize="11" fill="#336699">
        Drain (N)
      </text>

      {/* Phosphorus dopant atoms in the drain */}
      {[
        [330, 170],
        [370, 185],
        [350, 200],
      ].map(([cx, cy], i) => (
        <g key={`pd-${i}`}>
          <circle cx={cx} cy={cy} r="6" fill="#88aaff" stroke="#6688cc" strokeWidth="1" />
          <text x={cx} y={cy! + 4} textAnchor="middle" fontSize="8" fill="#fff">
            P
          </text>
        </g>
      ))}

      {/* SiO2 gate oxide layer — the thin insulating layer of silicon dioxide.
          This is the "O" in MOSFET. It prevents any current from flowing into
          the gate, making MOSFETs voltage-controlled (unlike BJTs which are
          current-controlled). */}
      <rect
        x="140"
        y="125"
        width="170"
        height="15"
        fill="#ffdd88"
        stroke="#cc9944"
        strokeWidth="1"
        rx="2"
      />
      <text x="225" y="122" textAnchor="middle" fontSize="9" fill="#996600">
        SiO2
      </text>

      {/* Metal gate electrode — the "M" in MOSFET.
          Voltage applied here creates an electric field through the oxide. */}
      <rect
        x="155"
        y="85"
        width="140"
        height="40"
        fill="#999"
        stroke="#666"
        strokeWidth="2"
        rx="4"
      />
      <text x="225" y="110" textAnchor="middle" fontSize="12" fill="#fff" fontWeight="bold">
        Gate
      </text>

      {/* Gate lead */}
      <line x1="225" y1="55" x2="225" y2="85" stroke="#666" strokeWidth="2" />
      <text x="225" y="48" textAnchor="middle" fontSize="10" fill="#666">
        G ({vgs.toFixed(1)}V)
      </text>

      {/* Source lead */}
      <line x1="90" y1="140" x2="90" y2="55" stroke="#336699" strokeWidth="2" />
      <text x="90" y="48" textAnchor="middle" fontSize="10" fill="#336699">
        S
      </text>

      {/* Drain lead */}
      <line x1="360" y1="140" x2="360" y2="55" stroke="#336699" strokeWidth="2" />
      <text x="360" y="48" textAnchor="middle" fontSize="10" fill="#336699">
        D
      </text>

      {/* Inversion channel — the thin conducting layer that forms under
          the gate oxide when Vgs > Vth. This is the "field effect": the
          gate's electric field attracts electrons to the surface, creating
          a channel of free carriers connecting source to drain. */}
      <rect
        x="140"
        y="140"
        width="170"
        height="8"
        fill="#4488ff"
        opacity={channelOpacity}
        rx="2"
      />
      {conducting && (
        <text x="225" y="160" textAnchor="middle" fontSize="9" fill="#336699" opacity={channelOpacity}>
          channel
        </text>
      )}

      {/* Electron flow through the channel */}
      <ParticleLayer
        particles={particles}
        radius={2.5}
        color="#4488ff"
      />
    </svg>
  );
}
