/**
 * forme-pipeline-config — type/predicate tests
 */

import { describe, it, expect } from "vitest";
import { isStageRef } from "../src/index.js";

describe("isStageRef", () => {
  it("recognises a well-formed StageRef", () => {
    expect(isStageRef({ kind: "stage-ref", packageName: "@forme/source-fs" })).toBe(true);
  });

  it("rejects missing kind discriminator", () => {
    expect(isStageRef({ packageName: "@forme/source-fs" })).toBe(false);
  });

  it("rejects wrong kind discriminator", () => {
    expect(isStageRef({ kind: "stage", packageName: "@forme/source-fs" })).toBe(false);
  });

  it("rejects missing packageName", () => {
    expect(isStageRef({ kind: "stage-ref" })).toBe(false);
  });

  it("rejects non-string packageName", () => {
    expect(isStageRef({ kind: "stage-ref", packageName: 42 })).toBe(false);
  });

  it("rejects null and primitives", () => {
    expect(isStageRef(null)).toBe(false);
    expect(isStageRef(undefined)).toBe(false);
    expect(isStageRef("string")).toBe(false);
    expect(isStageRef(42)).toBe(false);
  });
});
