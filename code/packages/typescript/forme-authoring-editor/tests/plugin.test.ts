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
    expect(request.project).toEqual(project);
    expect(request.project).not.toBe(project);
    expect(request.target).toEqual({
      documentId: "018f47a0-9b6c-7def-9234-56789abcdef1",
      blockIndex: 2,
    });
    expect(Object.isFrozen(request)).toBe(true);
    expect(Object.isFrozen(request.target)).toBe(true);
    expect(Object.isFrozen(request.project.site)).toBe(true);
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

  it.each([
    ["non-array", null],
    ["empty text", [{ pluginId: "", actionId: "run", slot: "site-toolbar", label: "Run" }]],
    ["non-text", [{ pluginId: 4, actionId: "run", slot: "site-toolbar", label: "Run" }]],
    ["surrounding whitespace", [{ pluginId: "x", actionId: "run", slot: "site-toolbar", label: " Run" }]],
    ["overlong text", [{ pluginId: "x", actionId: "run", slot: "site-toolbar", label: "x".repeat(81) }]],
    ["lone high surrogate", [{ pluginId: "x", actionId: "run", slot: "site-toolbar", label: "\ud800" }]],
    ["lone low surrogate", [{ pluginId: "x", actionId: "run", slot: "site-toolbar", label: "\udc00" }]],
    ["bidi override", [{ pluginId: "x", actionId: "run", slot: "site-toolbar", label: "Run\u202e" }]],
  ])("rejects %s descriptor input", (_name, input) => {
    expect(() => validateEditorContributions(input)).toThrow(EditorBoundaryError);
  });

  it("accepts a well-formed supplementary Unicode scalar", () => {
    expect(validateEditorContributions([
      { pluginId: "x", actionId: "run", slot: "site-toolbar", label: "Run 🚀" },
    ])[0]!.label).toBe("Run 🚀");
  });

  it("rejects array accessors and inspection traps without invoking data getters", () => {
    const accessor: unknown[] = [];
    Object.defineProperty(accessor, "0", { enumerable: true, get: () => ({}) });
    Object.defineProperty(accessor, "length", { value: 1 });
    expect(() => validateEditorContributions(accessor)).toThrow(/data entries/);

    const trappedArray = new Proxy([], { ownKeys: () => { throw new Error("secret"); } });
    expect(() => validateEditorContributions(trappedArray)).toThrow(/safely/);

    const trappedLength = new Proxy([], {
      getOwnPropertyDescriptor: (_target, key) => {
        if (key === "length") throw new Error("secret");
        return undefined;
      },
    });
    expect(() => validateEditorContributions(trappedLength)).toThrow(/safely/);

    const trappedIndex = new Proxy([{}], {
      getOwnPropertyDescriptor: (target, key) => {
        if (key === "0") throw new Error("secret");
        return Reflect.getOwnPropertyDescriptor(target, key);
      },
    });
    expect(() => validateEditorContributions(trappedIndex)).toThrow(/safely/);

    const trappedObject = new Proxy({}, { getPrototypeOf: () => { throw new Error("secret"); } });
    expect(() => validateEditorContributions([trappedObject])).toThrow(/safely/);

    const oversizedBeforeEnumeration = new Proxy(new Array(33), {
      ownKeys: () => { throw new Error("must not enumerate"); },
    });
    expect(() => validateEditorContributions(oversizedBeforeEnumeration)).toThrow(/32/);

    const hiddenIndex: unknown[] = [];
    Object.defineProperty(hiddenIndex, "0", { value: {}, enumerable: false });
    expect(() => validateEditorContributions(hiddenIndex)).toThrow(/data entries/);
  });

  it("rejects empty theme lists, invalid targets, and mutable project snapshots", () => {
    expect(() => validateThemeOptions([])).toThrow(/at least one/);
    expect(() => createEditorPluginRequest(
      { pluginId: "x", actionId: "run", slot: "site-toolbar", label: "Run" },
      project,
      { documentId: 4 as unknown as string, blockIndex: null },
    )).toThrow(/documentId/);
    expect(() => createEditorPluginRequest(
      { pluginId: "x", actionId: "run", slot: "site-toolbar", label: "Run" },
      project,
      { documentId: null, blockIndex: -1 },
    )).toThrow(/blockIndex/);
    expect(() => createEditorPluginRequest(
      { pluginId: "x", actionId: "run", slot: "site-toolbar", label: "Run" },
      { ...project },
      { documentId: null, blockIndex: null },
    )).toThrow(/frozen/);
    expect(() => createEditorPluginRequest(
      { pluginId: "x", actionId: "run", slot: "site-toolbar", label: "Run" },
      Object.freeze({ ...project, schemaVersion: 2 }) as unknown as AuthoringProject,
      { documentId: null, blockIndex: null },
    )).toThrow(/valid authoring snapshot/);
    expect(() => validateEditorContributions([null])).toThrow(/plain object/);
  });

  it("creates a private deep snapshot from a shallow-frozen valid project", () => {
    const site = { baseUrl: null, themeId: "forme-classless" };
    const shallow = Object.freeze({ ...project, site });
    const request = createEditorPluginRequest(
      { pluginId: "x", actionId: "run", slot: "site-toolbar", label: "Run" },
      shallow,
      { documentId: null, blockIndex: null },
    );
    site.themeId = "changed-after-validation";
    expect(request.project.site.themeId).toBe("forme-classless");
    expect(Object.isFrozen(request.project.site)).toBe(true);

    const trapped = new Proxy(project, { isExtensible: () => { throw new Error("secret"); } });
    expect(() => createEditorPluginRequest(
      { pluginId: "x", actionId: "run", slot: "site-toolbar", label: "Run" },
      trapped,
      { documentId: null, blockIndex: null },
    )).toThrow(/safely/);
  });
});
