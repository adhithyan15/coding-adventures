import { describe, it, expect } from "vitest";
import { ManifestError, MANIFEST_ERROR_CODES } from "../src/errors.js";

describe("ManifestError", () => {
  it("constructs with code + message", () => {
    const e = new ManifestError({ code: "TOML_MALFORMED", message: "bad toml" });
    expect(e.code).toBe("TOML_MALFORMED");
    expect(e.path).toBe("");
    expect(e.message).toBe("bad toml");
    expect(e.errors).toEqual([]);
    expect(e.name).toBe("ManifestError");
    expect(e).toBeInstanceOf(Error);
  });

  it("includes path in single-error message", () => {
    const e = new ManifestError({
      code: "PLUGIN_NAME_INVALID",
      message: "name invalid",
      path: "plugin.name",
    });
    expect(e.message).toContain("plugin.name");
  });

  it("aggregates multiple errors and summarises in message", () => {
    const e = new ManifestError({
      code: "TOML_MALFORMED",
      message: "validation failed",
      errors: [
        { code: "PLUGIN_NAME_INVALID", path: "plugin.name", message: "bad name" },
        { code: "PLUGIN_VERSION_INVALID", path: "plugin.version", message: "bad version" },
      ],
    });
    expect(e.errors).toHaveLength(2);
    expect(e.message).toContain("2 violations");
    expect(e.message).toContain("plugin.name");
    expect(e.message).toContain("plugin.version");
  });

  it("singular message for 1 violation", () => {
    const e = new ManifestError({
      code: "TOML_MALFORMED",
      message: "validation failed",
      errors: [{ code: "PLUGIN_NAME_INVALID", path: "plugin.name", message: "bad name" }],
    });
    expect(e.message).toContain("1 violation):");
    expect(e.message).not.toContain("1 violations");
  });

  it("truncates errors list in message", () => {
    const errors = Array.from({ length: 12 }, (_, i) => ({
      code: "PLUGIN_NAME_INVALID" as const,
      path: `field[${i}]`,
      message: `error ${i}`,
    }));
    const e = new ManifestError({
      code: "TOML_MALFORMED",
      message: "lots of errors",
      errors,
    });
    expect(e.message).toContain("(7 more)");
  });

  it("freezes entries", () => {
    const e = new ManifestError({
      code: "TOML_MALFORMED",
      message: "x",
      errors: [{ code: "PLUGIN_NAME_INVALID", path: "p", message: "m" }],
    });
    expect(Object.isFrozen(e.errors[0])).toBe(true);
  });

  it("MANIFEST_ERROR_CODES is frozen and covers parser + validator + templating", () => {
    expect(Object.isFrozen(MANIFEST_ERROR_CODES)).toBe(true);
    expect(MANIFEST_ERROR_CODES).toContain("TOML_MALFORMED");
    expect(MANIFEST_ERROR_CODES).toContain("PLUGIN_NAME_INVALID");
    expect(MANIFEST_ERROR_CODES).toContain("TEMPLATE_UNKNOWN_VARIABLE");
  });

  it("renders root path as (root) in aggregate message", () => {
    const e = new ManifestError({
      code: "TOML_MALFORMED",
      message: "x",
      errors: [{ code: "PLUGIN_NAME_INVALID", path: "", message: "m" }],
    });
    expect(e.message).toContain("(root)");
  });
});
