/**
 * seed.test.ts — Validates that all seed templates are well-formed.
 *
 * These tests verify that:
 * - seedTemplates creates exactly 3 templates
 * - Each template passes createInstance without throwing
 * - Each template has at least one item
 * - The decision-tree templates include at least one decision item
 */

import { describe, it, expect, beforeEach } from "vitest";
import { createState, createInstance } from "../state.js";
import { seedTemplates } from "../seed.js";
import type { AppState } from "../state.js";

describe("seedTemplates", () => {
  let state: AppState;

  beforeEach(() => {
    state = createState();
    seedTemplates(state);
  });

  it("creates exactly 3 templates", () => {
    expect(state.templates).toHaveLength(3);
  });

  it("all templates have non-empty names", () => {
    for (const t of state.templates) {
      expect(t.name.length).toBeGreaterThan(0);
    }
  });

  it("all templates have at least one item", () => {
    for (const t of state.templates) {
      expect(t.items.length).toBeGreaterThan(0);
    }
  });

  it("Morning Routine is a flat checklist (no decisions)", () => {
    const morning = state.templates.find((t) => t.name === "Morning Routine");
    expect(morning).toBeDefined();
    for (const item of morning!.items) {
      expect(item.type).toBe("check");
    }
  });

  it("Deployment Runbook contains at least one decision", () => {
    const deploy = state.templates.find((t) => t.name === "Deployment Runbook");
    expect(deploy).toBeDefined();
    const hasDecision = deploy!.items.some((i) => i.type === "decision");
    expect(hasDecision).toBe(true);
  });

  it("Troubleshooting Guide contains nested decisions", () => {
    const guide = state.templates.find(
      (t) => t.name === "Troubleshooting Guide",
    );
    expect(guide).toBeDefined();
    // The first decision item's yes-branch should itself contain a decision
    const outerDecision = guide!.items.find((i) => i.type === "decision");
    expect(outerDecision?.type).toBe("decision");
    if (outerDecision?.type === "decision") {
      const innerDecision = outerDecision.yesBranch.find(
        (i) => i.type === "decision",
      );
      expect(innerDecision?.type).toBe("decision");
    }
  });

  it("createInstance succeeds for all seed templates", () => {
    for (const template of state.templates) {
      expect(() => createInstance(state, template.id)).not.toThrow();
    }
  });

  it("all seed item ids are unique across templates", () => {
    function collectIds(items: typeof state.templates[0]["items"]): string[] {
      const ids: string[] = [];
      for (const item of items) {
        ids.push(item.id);
        if (item.type === "decision") {
          ids.push(...collectIds(item.yesBranch));
          ids.push(...collectIds(item.noBranch));
        }
      }
      return ids;
    }

    const allIds: string[] = [];
    for (const t of state.templates) {
      allIds.push(...collectIds(t.items));
    }
    const unique = new Set(allIds);
    expect(unique.size).toBe(allIds.length);
  });
});
