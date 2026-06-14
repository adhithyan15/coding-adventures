/**
 * forme-pipeline-config — ConfigError tests
 */

import { describe, it, expect } from "vitest";
import { CONFIG_ERROR_CODES, ConfigError } from "../src/index.js";

describe("ConfigError construction", () => {
  it("requires at least one entry", () => {
    expect(() => new ConfigError([])).toThrow(/at least one entry/);
  });

  it("name is ConfigError", () => {
    const e = new ConfigError([{ path: "x", code: "X", message: "y" }]);
    expect(e.name).toBe("ConfigError");
  });

  it("is an Error subclass", () => {
    const e = new ConfigError([{ path: "x", code: "X", message: "y" }]);
    expect(e).toBeInstanceOf(Error);
  });

  it("freezes individual entries (no mutation)", () => {
    const e = new ConfigError([{ path: "x", code: "X", message: "y" }]);
    expect(() => {
      // @ts-expect-error — readonly
      e.errors[0]!.message = "hack";
    }).toThrow(TypeError);
  });

  it("freezes the entries array (no push)", () => {
    const e = new ConfigError([{ path: "x", code: "X", message: "y" }]);
    expect(() => {
      // @ts-expect-error — readonly
      e.errors.push({ path: "z", code: "Z", message: "z" });
    }).toThrow(TypeError);
  });
});

describe("ConfigError.message summary", () => {
  it("single error → one-line summary with path/message/code", () => {
    const e = new ConfigError([{ path: "stages[0].id", code: "DUPLICATE_INSTANCE_ID", message: "uh oh" }]);
    expect(e.message).toContain("stages[0].id");
    expect(e.message).toContain("uh oh");
    expect(e.message).toContain("DUPLICATE_INSTANCE_ID");
  });

  it("multiple errors → multi-line summary with count", () => {
    const e = new ConfigError([
      { path: "a", code: "X", message: "first" },
      { path: "b", code: "Y", message: "second" },
    ]);
    expect(e.message).toContain("(2 errors)");
    expect(e.message).toContain("first");
    expect(e.message).toContain("second");
  });
});

describe("CONFIG_ERROR_CODES", () => {
  it("includes the FM03 §2.4 spec rules", () => {
    expect(CONFIG_ERROR_CODES.DUPLICATE_INSTANCE_ID).toBe("DUPLICATE_INSTANCE_ID");
    expect(CONFIG_ERROR_CODES.API_VERSION_MISMATCH).toBe("API_VERSION_MISMATCH");
    expect(CONFIG_ERROR_CODES.CAPABILITY_NOT_DECLARED).toBe("CAPABILITY_NOT_DECLARED");
    expect(CONFIG_ERROR_CODES.CONFIG_REQUIRED).toBe("CONFIG_REQUIRED");
    expect(CONFIG_ERROR_CODES.MULTIPLE_OUTPUTS_UNNAMED).toBe("MULTIPLE_OUTPUTS_UNNAMED");
    expect(CONFIG_ERROR_CODES.STAGE_REF_UNRESOLVED).toBe("STAGE_REF_UNRESOLVED");
  });

  it("is frozen", () => {
    expect(() => {
      // @ts-expect-error — readonly
      CONFIG_ERROR_CODES.NEW = "NEW";
    }).toThrow(TypeError);
  });
});
