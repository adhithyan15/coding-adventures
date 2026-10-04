import { describe, expect, it } from "vitest";

import {
  EditorBoundaryError,
  createEditorPluginRequest,
  validateEditorContributions,
  validateThemeOptions,
} from "../src/plugin.js";
import type { AuthoringProject } from "@coding-adventures/forme-authoring-core";

const project: AuthoringProject = Object.freeze({
  schemaVersion: 1,
  projectId: "018f47a0-9b6c-7def-9234-56789abcdef0",
  title: "Demo",
  site: Object.freeze({ baseUrl: null, themeId: "forme-classless" }),
  documents: Object.freeze([]),
  activeDocumentId: null,
});

describe("declarative editor plugin boundary", () => {
  it("snapshots exact contribution data and freezes the result", () => {
    const input = [{
      pluginId: "word-count",
      actionId: "insert-summary",
      slot: "document-toolbar",
      label: "Insert summary",
    }];
    const result = validateEditorContributions(input);

    expect(result).toEqual(input);
    expect(result).not.toBe(input);
    expect(Object.isFrozen(result)).toBe(true);
    expect(Object.isFrozen(result[0])).toBe(true);
    input[0]!.label = "Changed later";
    expect(result[0]!.label).toBe("Insert summary");
  });

  it.each([
    ["prototype", [Object.create({ pluginId: "x" })]],
    ["unknown field", [{ pluginId: "x", actionId: "run", slot: "site-toolbar", label: "Run", extra: true }]],
    ["control character", [{ pluginId: "x", actionId: "run", slot: "site-toolbar", label: "Run\u0000" }]],
    ["bad slot", [{ pluginId: "x", actionId: "run", slot: "preview-toolbar", label: "Run" }]],
    ["duplicate", [
      { pluginId: "x", actionId: "run", slot: "site-toolbar", label: "One" },
      { pluginId: "x", actionId: "run", slot: "site-toolbar", label: "Two" },
    ]],
  ])("rejects hostile or ambiguous %s contribution data", (_name, input) => {
    expect(() => validateEditorContributions(input)).toThrow(EditorBoundaryError);
  });

  it("rejects accessors, sparse arrays, symbols, and over-count arrays", () => {
    const accessor = { pluginId: "x", actionId: "run", slot: "site-toolbar" } as Record<string, unknown>;
    Object.defineProperty(accessor, "label", { enumerable: true, get: () => "Run" });
    expect(() => validateEditorContributions([accessor])).toThrow(/data fields/);

    const sparse = new Array(1);
    expect(() => validateEditorContributions(sparse)).toThrow(/dense/);

    const symbol = { pluginId: "x", actionId: "run", slot: "site-toolbar", label: "Run" } as Record<PropertyKey, unknown>;
    symbol[Symbol("hidden")] = true;
    expect(() => validateEditorContributions([symbol])).toThrow(/string fields/);

    const many = Array.from({ length: 33 }, (_, index) => ({
      pluginId: `plugin-${index}`,
      actionId: "run",
      slot: "site-toolbar",
      label: `Run ${index}`,
    }));
    expect(() => validateEditorContributions(many)).toThrow(/32/);
  });

  it("creates a frozen least-authority bridge request", () => {
    const request = createEditorPluginRequest(
      { pluginId: "word-count", actionId: "insert-summary", slot: "block-toolbar", label: "Insert" },
      project,
      { documentId: "018f47a0-9b6c-7def-9234-56789abcdef1", blockIndex: 2 },
    );

    expect(Reflect.ownKeys(request).sort()).toEqual([
      "actionId", "pluginId", "project", "slot", "target",
    ]);
    expect(request.project).toBe(project);
    expect(request.target).toEqual({
      documentId: "018f47a0-9b6c-7def-9234-56789abcdef1",
      blockIndex: 2,
    });
    expect(Object.isFrozen(request)).toBe(true);
    expect(Object.isFrozen(request.target)).toBe(true);
  });

  it("validates and freezes host-supplied theme choices", () => {
    const result = validateThemeOptions([
      { id: "forme-classless", label: "Forme Classless" },
      { id: "high-contrast", label: "High contrast" },
    ]);
    expect(result.map((entry) => entry.id)).toEqual(["forme-classless", "high-contrast"]);
    expect(Object.isFrozen(result[0])).toBe(true);
    expect(() => validateThemeOptions([{ id: "Raw CSS", label: "Unsafe" }])).toThrow(/portable/);
    expect(() => validateThemeOptions([
      { id: "same", label: "One" },
      { id: "same", label: "Two" },
    ])).toThrow(/duplicate/);
  });
});
