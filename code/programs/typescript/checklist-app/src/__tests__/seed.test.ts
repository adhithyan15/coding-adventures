/**
 * seed.test.ts — Validates that all seed templates are well-formed.
 *
 * V0.3: Uses a Store with the reducer instead of the old createState().
 * seedTemplates now dispatches TEMPLATE_CREATE actions to the store.
 *
 * These tests verify that:
 * - seedTemplates creates exactly 3 templates
 * - Each template passes INSTANCE_CREATE without throwing
 * - Each template has at least one item
 * - The decision-tree templates include at least one decision item
 */

import { describe, it, expect, beforeEach } from "vitest";
import { Store } from "@coding-adventures/store";
import { reducer } from "../reducer.js";
import type { AppState } from "../reducer.js";
import { seedTemplates } from "../seed.js";
import { createInstanceAction } from "../actions.js";
import type { Template } from "../types.js";

describe("seedTemplates", () => {
  let s: Store<AppState>;

  beforeEach(() => {
    s = new Store<AppState>({ templates: [], instances: [], todos: [] }, reducer);
    seedTemplates(s);
  });

  it("creates exactly 3 templates", () => {
    expect(s.getState().templates).toHaveLength(3);
  });

  it("all templates have non-empty names", () => {
    for (const t of s.getState().templates) {
      expect(t.name.length).toBeGreaterThan(0);
    }
  });

  it("all templates have at least one item", () => {
    for (const t of s.getState().templates) {
      expect(t.items.length).toBeGreaterThan(0);
    }
  });

  it("Morning Routine is a flat checklist (no decisions)", () => {
    const morning = s.getState().templates.find((t) => t.name === "Morning Routine");
    expect(morning).toBeDefined();
    for (const item of morning!.items) {
      expect(item.type).toBe("check");
    }
  });

  it("Deployment Runbook contains at least one decision", () => {
    const deploy = s.getState().templates.find((t) => t.name === "Deployment Runbook");
    expect(deploy).toBeDefined();
    const hasDecision = deploy!.items.some((i) => i.type === "decision");
    expect(hasDecision).toBe(true);
  });

  it("Troubleshooting Guide contains nested decisions", () => {
    const guide = s.getState().templates.find(
      (t) => t.name === "Troubleshooting Guide",
    );
    expect(guide).toBeDefined();
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
    for (const template of s.getState().templates) {
      expect(() => s.dispatch(createInstanceAction(template.id))).not.toThrow();
    }
  });

  it("all seed item ids are unique across templates", () => {
    type TemplateItems = Template["items"];
    function collectIds(items: TemplateItems): string[] {
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
    for (const t of s.getState().templates) {
      allIds.push(...collectIds(t.items));
    }
    const unique = new Set(allIds);
    expect(unique.size).toBe(allIds.length);
  });
});
