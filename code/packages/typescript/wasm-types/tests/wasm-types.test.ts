import assert from "node:assert/strict";
import { describe, it } from "node:test";
import { VERSION } from "../src/index.js";

describe("wasm-types", () => {
  it("has a version", () => {
    assert.equal(VERSION, "0.1.0");
  });
});
