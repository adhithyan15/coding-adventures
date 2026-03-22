/**
 * Particle Layer — renders particles as SVG circles within a diagram.
 *
 * This is a pure rendering component with no simulation logic. It receives
 * a readonly array of particles from the useParticleSystem hook and maps
 * each one to an SVG <circle> element with the appropriate position,
 * radius, opacity, and color.
 *
 * The component is placed as a <g> element within an SVG, so it inherits
 * the parent SVG's coordinate system.
 */

import type { Particle } from "../lib/particle-system.js";

interface ParticleLayerProps {
  /** Array of particles to render. */
  particles: readonly Particle[];
  /** Radius of each particle circle. */
  radius: number;
  /** CSS color string for all particles. */
  color: string;
}

export function ParticleLayer({ particles, radius, color }: ParticleLayerProps) {
  return (
    <g className="particle-layer">
      {particles.map((p) => (
        <circle
          key={p.id}
          cx={p.x}
          cy={p.y}
          r={radius}
          fill={color}
          opacity={p.opacity}
        />
      ))}
    </g>
  );
}
