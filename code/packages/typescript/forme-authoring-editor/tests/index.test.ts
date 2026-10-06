import { describe, expect, it } from "vitest";

import {
  AUTHORING_EDITOR_CSS,
  AuthoringEditor,
  VERSION,
  createDefaultBlock,
  validateEditorContributions,
} from "../src/index.js";

describe("forme-authoring-editor public surface", () => {
  it("exports the editor, pure helpers, styles, and version", () => {
    expect(VERSION).toBe("1.0.0");
    expect(AuthoringEditor).toBeTypeOf("function");
    expect(createDefaultBlock("paragraph")).toEqual({
      type: "paragraph",
      children: [{ type: "text", value: "" }],
    });
    expect(validateEditorContributions([])).toEqual([]);
    expect(AUTHORING_EDITOR_CSS).toContain(":focus-visible");
  });
});
