/**
 * forme-errors — CapabilityError tests
 */

import { describe, it, expect } from "vitest";
import {
  CapabilityError,
  ERROR_CODES,
  StageError,
} from "../src/index.js";

describe("CapabilityError", () => {
  it("is a StageError subclass", () => {
    const e = new CapabilityError({
      message:    "no network",
      capability: "network:api.github.com",
    });
    expect(e).toBeInstanceOf(StageError);
    expect(e).toBeInstanceOf(CapabilityError);
  });

  it("forces code to CAPABILITY_DENIED", () => {
    const e = new CapabilityError({
      message:    "no env",
      capability: "env:GITHUB_TOKEN",
    });
    expect(e.code).toBe(ERROR_CODES.CAPABILITY_DENIED);
  });

  it("forces recoverable to false", () => {
    const e = new CapabilityError({
      message:    "shell forbidden",
      capability: "system:shell",
    });
    expect(e.recoverable).toBe(false);
  });

  it("carries the offending capability string", () => {
    const e = new CapabilityError({
      message:    "no fs write",
      capability: "storage:write",
    });
    expect(e.capability).toBe("storage:write");
  });

  it("toJson includes the capability field alongside the StageError shape", () => {
    const e = new CapabilityError({
      message:    "no network",
      capability: "network:api.openai.com",
      stageName:  "@forme/transform-llm",
      inputPath:  "posts/draft.md",
    });
    const json = e.toJson() as Record<string, unknown>;
    expect(json.code).toBe("CAPABILITY_DENIED");
    expect(json.capability).toBe("network:api.openai.com");
    expect(json.stageName).toBe("@forme/transform-llm");
    expect(json.inputPath).toBe("posts/draft.md");
    expect(json.recoverable).toBe(false);
  });

  it("sets name to the subclass constructor name", () => {
    const e = new CapabilityError({ message: "x", capability: "x" });
    expect(e.name).toBe("CapabilityError");
  });
});
