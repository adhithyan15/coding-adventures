// index.test.ts — smoke test that the public surface is exported.
//
// The package's public surface comes from src/index.ts. Re-exports
// are easy to break (a refactor renames a file, the index forgets to
// re-export the new path) and the failure mode is silent — consumers
// just see undefined imports. This test enforces that every advertised
// symbol is actually exported.

import { describe, it, expect } from "vitest";
import * as pkg from "../src/index.js";

describe("@coding-adventures/mosaic-flux-react public surface", () => {
  it("exports MosaicStore class", () => {
    expect(typeof pkg.MosaicStore).toBe("function");
  });

  it("exports isMosaicAction type guard", () => {
    expect(typeof pkg.isMosaicAction).toBe("function");
  });

  it("exports composeMiddleware utility", () => {
    expect(typeof pkg.composeMiddleware).toBe("function");
  });

  it("exports loggerMiddleware factory", () => {
    expect(typeof pkg.loggerMiddleware).toBe("function");
  });

  it("exports createSelector factory", () => {
    expect(typeof pkg.createSelector).toBe("function");
  });

  it("exports devToolsMiddleware factory", () => {
    expect(typeof pkg.devToolsMiddleware).toBe("function");
  });
});
