/**
 * Particle System — a pure TypeScript particle engine with NO React dependency.
 *
 * === Purpose ===
 *
 * This engine simulates the flow of electrons (or other charge carriers) through
 * transistor diagrams. Each particle represents an electron moving from a spawn
 * region (e.g., the emitter of a BJT) to a target region (e.g., the collector).
 *
 * The system is intentionally decoupled from React so it can be tested in pure
 * TypeScript without any DOM or rendering framework.
 *
 * === How it works ===
 *
 * On each tick():
 *   1. Spawn new particles in the spawn region at the configured rate
 *   2. Move each particle toward the target region center
 *   3. Add Brownian jitter for visual realism (electrons don't move in straight lines)
 *   4. Age particles and remove those past their maxAge or that reached the target
 *   5. Fade opacity in during the first 10% of life, out during the last 20%
 */

// ---------------------------------------------------------------------------
// Types
// ---------------------------------------------------------------------------

/** A single particle with position, velocity, opacity, and age. */
export interface Particle {
  id: number;
  x: number;
  y: number;
  vx: number;
  vy: number;
  opacity: number;
  age: number;
  maxAge: number;
}

/** An axis-aligned rectangle used for spawn and target regions. */
export interface Rect {
  x: number;
  y: number;
  width: number;
  height: number;
}

/** Configuration for the particle system's behavior. */
export interface ParticleConfig {
  /** Maximum number of particles alive at once. */
  maxParticles: number;
  /** How many particles to spawn per tick (fractional values accumulate). */
  spawnRate: number;
  /** Region where new particles appear. */
  spawnRegion: Rect;
  /** Region particles move toward. */
  targetRegion: Rect;
  /** Movement speed in pixels per tick. */
  speed: number;
  /** Visual radius of each particle circle (used by the renderer). */
  particleRadius: number;
  /** CSS color string for rendering. */
  color: string;
}

/** Public interface for interacting with the particle system. */
export interface ParticleSystem {
  /** Advance the simulation by one tick. */
  tick(): void;
  /** Get a readonly snapshot of all living particles. */
  getParticles(): readonly Particle[];
  /** Update configuration on the fly (e.g., change spawnRate when voltage changes). */
  updateConfig(partial: Partial<ParticleConfig>): void;
  /** Remove all particles and reset the spawn accumulator. */
  reset(): void;
}

// ---------------------------------------------------------------------------
// Implementation
// ---------------------------------------------------------------------------

/**
 * Create a new particle system with the given configuration.
 *
 * The returned object is a closure over the internal state — no class needed.
 * This keeps the API simple and makes the system easy to test.
 */
export function createParticleSystem(initialConfig: ParticleConfig): ParticleSystem {
  let config = { ...initialConfig };
  let particles: Particle[] = [];
  let nextId = 0;

  /**
   * Fractional spawn accumulator — because spawnRate can be less than 1,
   * we accumulate fractions across ticks and only spawn when the
   * accumulator reaches 1.0 or above.
   */
  let spawnAccumulator = 0;

  /**
   * Calculate the center point of a rectangle.
   * Used to determine the direction particles should move toward.
   */
  function rectCenter(rect: Rect): { cx: number; cy: number } {
    return {
      cx: rect.x + rect.width / 2,
      cy: rect.y + rect.height / 2,
    };
  }

  /**
   * Calculate particle opacity based on its age.
   *
   * The opacity profile creates a smooth fade-in and fade-out:
   *   - First 10% of life: ramp from 0 to 1 (fade in)
   *   - Middle 70% of life: hold at 1.0 (fully visible)
   *   - Last 20% of life: ramp from 1 to 0 (fade out)
   *
   * This prevents particles from "popping" into and out of existence.
   */
  function calculateOpacity(age: number, maxAge: number): number {
    if (maxAge <= 0) return 0;
    const lifeRatio = age / maxAge;

    // Fade in during the first 10%
    if (lifeRatio < 0.1) {
      return lifeRatio / 0.1;
    }

    // Fade out during the last 20%
    if (lifeRatio > 0.8) {
      return (1.0 - lifeRatio) / 0.2;
    }

    // Fully visible in the middle
    return 1.0;
  }

  /**
   * Spawn a single particle at a random position within the spawn region.
   */
  function spawnParticle(): Particle {
    const { spawnRegion } = config;
    return {
      id: nextId++,
      x: spawnRegion.x + Math.random() * spawnRegion.width,
      y: spawnRegion.y + Math.random() * spawnRegion.height,
      vx: 0,
      vy: 0,
      opacity: 0,
      age: 0,
      maxAge: 60 + Math.random() * 40, // 60-100 ticks of life
    };
  }

  return {
    tick() {
      // --- Step 1: Spawn new particles ---
      // Accumulate fractional spawn amounts across ticks
      spawnAccumulator += config.spawnRate;
      while (spawnAccumulator >= 1 && particles.length < config.maxParticles) {
        particles.push(spawnParticle());
        spawnAccumulator -= 1;
      }
      // If we're at max capacity, just drain the accumulator
      if (particles.length >= config.maxParticles) {
        spawnAccumulator = Math.min(spawnAccumulator, 1);
      }

      // --- Step 2: Move each particle toward the target ---
      const target = rectCenter(config.targetRegion);

      for (const p of particles) {
        // Calculate direction vector from particle to target center
        const dx = target.cx - p.x;
        const dy = target.cy - p.y;
        const dist = Math.sqrt(dx * dx + dy * dy);

        if (dist > 0.1) {
          // Normalize and scale by speed
          p.vx = (dx / dist) * config.speed;
          p.vy = (dy / dist) * config.speed;
        }

        // Add Brownian jitter for visual realism — electrons don't move
        // in perfectly straight lines through a semiconductor lattice
        p.vx += (Math.random() - 0.5);
        p.vy += (Math.random() - 0.5);

        // Apply velocity
        p.x += p.vx;
        p.y += p.vy;

        // Age the particle
        p.age += 1;

        // Update opacity based on age
        p.opacity = calculateOpacity(p.age, p.maxAge);
      }

      // --- Step 3: Remove dead particles ---
      // A particle is dead if it exceeded its maxAge or reached the target
      particles = particles.filter((p) => {
        if (p.age >= p.maxAge) return false;

        // Check if particle reached the target region
        const tr = config.targetRegion;
        const inTarget =
          p.x >= tr.x &&
          p.x <= tr.x + tr.width &&
          p.y >= tr.y &&
          p.y <= tr.y + tr.height;

        // Only remove if particle is old enough (at least 10 ticks) to have
        // visibly traveled — otherwise particles spawned near the target
        // would vanish immediately
        if (inTarget && p.age > 10) return false;

        return true;
      });
    },

    getParticles(): readonly Particle[] {
      return particles;
    },

    updateConfig(partial: Partial<ParticleConfig>) {
      config = { ...config, ...partial };
    },

    reset() {
      particles = [];
      spawnAccumulator = 0;
    },
  };
}
