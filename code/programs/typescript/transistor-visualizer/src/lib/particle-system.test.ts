/**
 * Tests for the particle system engine.
 *
 * These tests verify the core simulation mechanics without any rendering
 * or React dependency — pure TypeScript logic.
 */

import { describe, it, expect } from "vitest";
import { createParticleSystem, type ParticleConfig } from "./particle-system.js";

/** Helper: create a default config for testing. */
function testConfig(overrides?: Partial<ParticleConfig>): ParticleConfig {
  return {
    maxParticles: 50,
    spawnRate: 2,
    spawnRegion: { x: 0, y: 0, width: 10, height: 10 },
    targetRegion: { x: 90, y: 90, width: 10, height: 10 },
    speed: 3,
    particleRadius: 2,
    color: "#00aaff",
    ...overrides,
  };
}

describe("ParticleSystem", () => {
  // -----------------------------------------------------------------------
  // Spawning
  // -----------------------------------------------------------------------

  it("spawns particles within the spawn region", () => {
    const config = testConfig({ spawnRate: 5 });
    const system = createParticleSystem(config);

    system.tick();
    const particles = system.getParticles();

    // With spawnRate=5, we should have 5 particles after 1 tick
    expect(particles.length).toBe(5);

    // After one tick, particles have moved slightly (velocity + jitter),
    // so we allow a margin beyond the spawn region boundaries.
    // The jitter is +/- 0.5 per axis per tick, plus speed-based movement.
    const margin = 5;
    for (const p of particles) {
      expect(p.x).toBeGreaterThanOrEqual(-margin);
      expect(p.x).toBeLessThanOrEqual(10 + margin);
      expect(p.y).toBeGreaterThanOrEqual(-margin);
      expect(p.y).toBeLessThanOrEqual(10 + margin);
    }
  });

  it("handles fractional spawnRate with accumulation", () => {
    const config = testConfig({ spawnRate: 0.5 });
    const system = createParticleSystem(config);

    // After 1 tick at 0.5/tick, accumulator = 0.5 — no spawn yet
    system.tick();
    expect(system.getParticles().length).toBe(0);

    // After 2nd tick, accumulator = 1.0 — one spawn
    system.tick();
    expect(system.getParticles().length).toBe(1);
  });

  it("does not spawn when spawnRate is 0", () => {
    const config = testConfig({ spawnRate: 0 });
    const system = createParticleSystem(config);

    for (let i = 0; i < 10; i++) system.tick();
    expect(system.getParticles().length).toBe(0);
  });

  // -----------------------------------------------------------------------
  // Movement
  // -----------------------------------------------------------------------

  it("moves particles toward the target over multiple ticks", () => {
    const config = testConfig({
      spawnRate: 1,
      spawnRegion: { x: 0, y: 0, width: 1, height: 1 },
      targetRegion: { x: 100, y: 100, width: 1, height: 1 },
      speed: 5,
    });
    const system = createParticleSystem(config);

    system.tick(); // Spawn one particle
    const initial = system.getParticles()[0]!;
    const startX = initial.x;
    const startY = initial.y;

    // Run several ticks
    for (let i = 0; i < 5; i++) system.tick();

    const after = system.getParticles()[0];
    // Particle should have moved closer to (100.5, 100.5)
    // Due to jitter it may not be perfectly aligned, but should be
    // significantly closer
    if (after) {
      const distBefore = Math.sqrt(
        (100.5 - startX) ** 2 + (100.5 - startY) ** 2,
      );
      const distAfter = Math.sqrt(
        (100.5 - after.x) ** 2 + (100.5 - after.y) ** 2,
      );
      expect(distAfter).toBeLessThan(distBefore);
    }
  });

  // -----------------------------------------------------------------------
  // Aging
  // -----------------------------------------------------------------------

  it("removes particles after maxAge", () => {
    const config = testConfig({ spawnRate: 1, maxParticles: 1 });
    const system = createParticleSystem(config);

    system.tick(); // Spawn one
    const maxAge = system.getParticles()[0]!.maxAge;

    // Tick until past maxAge (with some buffer for the random component)
    for (let i = 0; i < maxAge + 10; i++) system.tick();

    // The original particle should be removed (maybe replaced by a new one)
    // but we stopped spawning new ones after the first because maxParticles=1
    // and spawnRate=1. The old one should be gone.
    const remaining = system.getParticles();
    if (remaining.length > 0) {
      // If a particle exists, it must be a newer one (higher id)
      expect(remaining[0]!.id).toBeGreaterThan(0);
    }
  });

  // -----------------------------------------------------------------------
  // Opacity
  // -----------------------------------------------------------------------

  it("fades particles in and out based on age", () => {
    const config = testConfig({ spawnRate: 1, maxParticles: 1 });
    const system = createParticleSystem(config);

    system.tick();
    // At age 0, opacity should be near 0 (fade-in zone)
    // Actually after tick, age=1
    const p = system.getParticles()[0]!;
    // Opacity should be low at the start
    expect(p.opacity).toBeLessThan(0.5);

    // After many ticks (mid-life), opacity should be 1.0
    for (let i = 0; i < 30; i++) system.tick();
    const midLife = system.getParticles().find((pp) => pp.id === p.id);
    if (midLife) {
      expect(midLife.opacity).toBeCloseTo(1.0, 0);
    }
  });

  // -----------------------------------------------------------------------
  // Config update
  // -----------------------------------------------------------------------

  it("stops spawning when spawnRate is set to 0 via updateConfig", () => {
    const config = testConfig({ spawnRate: 3 });
    const system = createParticleSystem(config);

    system.tick();
    const countAfterFirst = system.getParticles().length;
    expect(countAfterFirst).toBeGreaterThan(0);

    // Stop spawning
    system.updateConfig({ spawnRate: 0 });

    // Run many ticks — particles should eventually all die off
    for (let i = 0; i < 200; i++) system.tick();
    expect(system.getParticles().length).toBe(0);
  });

  // -----------------------------------------------------------------------
  // Reset
  // -----------------------------------------------------------------------

  it("clears all particles on reset", () => {
    const config = testConfig({ spawnRate: 5 });
    const system = createParticleSystem(config);

    for (let i = 0; i < 5; i++) system.tick();
    expect(system.getParticles().length).toBeGreaterThan(0);

    system.reset();
    expect(system.getParticles().length).toBe(0);
  });

  // -----------------------------------------------------------------------
  // Max particles
  // -----------------------------------------------------------------------

  it("does not exceed maxParticles", () => {
    const config = testConfig({ spawnRate: 10, maxParticles: 5 });
    const system = createParticleSystem(config);

    for (let i = 0; i < 10; i++) system.tick();
    expect(system.getParticles().length).toBeLessThanOrEqual(5);
  });
});
