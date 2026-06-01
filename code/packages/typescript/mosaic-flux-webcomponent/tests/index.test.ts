// index.test.ts — public surface smoke test.

import { describe, it, expect } from "vitest";
import * as pkg from "../src/index.js";

describe("@coding-adventures/mosaic-flux-webcomponent public surface", () => {
  it("exports core types (same as mosaic-flux-html)", () => {
    expect(typeof pkg.MosaicStore).toBe("function");
    expect(typeof pkg.isMosaicAction).toBe("function");
    expect(typeof pkg.composeMiddleware).toBe("function");
    expect(typeof pkg.loggerMiddleware).toBe("function");
    expect(typeof pkg.createSelector).toBe("function");
    expect(typeof pkg.devToolsMiddleware).toBe("function");
  });

  it("exports DOM binding helpers", () => {
    expect(typeof pkg.bindText).toBe("function");
    expect(typeof pkg.bindAttr).toBe("function");
    expect(typeof pkg.bindClass).toBe("function");
    expect(typeof pkg.bindStyle).toBe("function");
    expect(typeof pkg.bindList).toBe("function");
  });

  it("exports WebComponent-specific surface", () => {
    expect(typeof pkg.MosaicHostElement).toBe("function");
    expect(typeof pkg.defineMosaicElement).toBe("function");
  });
});
