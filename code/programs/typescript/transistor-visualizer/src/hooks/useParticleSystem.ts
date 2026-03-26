/**
 * React hook for the particle system.
 *
 * Bridges the pure TypeScript particle engine with React's rendering cycle
 * using requestAnimationFrame. Respects the user's motion preferences —
 * if they prefer reduced motion, particles are not rendered.
 *
 * === Why a separate hook? ===
 *
 * The particle system itself is a pure TypeScript closure with no React
 * dependency. This hook is the adapter that:
 *   1. Creates and holds a reference to the system
 *   2. Drives the tick() loop via useAnimationFrame
 *   3. Copies particle state into React state for rendering
 *   4. Pauses animation when reduced motion is preferred
 */

import { useRef, useState, useCallback } from "react";
import {
  useAnimationFrame,
  useReducedMotion,
} from "@coding-adventures/ui-components";
import {
  createParticleSystem,
  type ParticleConfig,
  type Particle,
} from "../lib/particle-system.js";

/**
 * Hook that runs a particle system and provides renderable particle data.
 *
 * @param config - Initial configuration for the particle system.
 * @returns particles array, reduced motion flag, and config updater.
 */
export function useParticleSystem(config: ParticleConfig) {
  const reducedMotion = useReducedMotion();

  // Hold the particle system in a ref so it persists across renders
  // without triggering re-renders when its internal state changes.
  // Only React state updates (setParticles) trigger renders.
  const systemRef = useRef(createParticleSystem(config));
  const [particles, setParticles] = useState<readonly Particle[]>([]);

  // Drive the particle system on each animation frame.
  // The second argument controls whether the loop is active:
  //   - Stop when user prefers reduced motion
  //   - Stop when spawnRate is 0 (no current flowing)
  useAnimationFrame(() => {
    const system = systemRef.current;
    system.tick();
    setParticles(system.getParticles());
  }, !reducedMotion && config.spawnRate > 0);

  // Allow parent components to update config (e.g., when voltage changes)
  const updateConfig = useCallback((partial: Partial<ParticleConfig>) => {
    systemRef.current.updateConfig(partial);
  }, []);

  return {
    // Return empty array when reduced motion is preferred — the renderer
    // should show static indicators instead
    particles: reducedMotion ? [] : particles,
    reducedMotion,
    updateConfig,
  };
}
