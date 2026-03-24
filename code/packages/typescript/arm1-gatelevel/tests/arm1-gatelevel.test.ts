import { describe, it, expect } from "vitest";
import { VERSION } from "../src/index.js";

describe("arm1-gatelevel", () => {
  it("has a version", () => {
    expect(VERSION).toBe("0.1.0");
  });
});
