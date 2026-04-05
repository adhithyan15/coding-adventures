import { describe, expect, it } from "vitest";
import { VERSION } from "../src/index.js";

describe("wasm-validator", () => {
  it("exports VERSION 0.1.0", () => {
    expect(VERSION).toBe("0.1.0");
  });
});
