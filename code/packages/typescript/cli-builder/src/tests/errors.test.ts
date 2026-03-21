/**
 * errors.test.ts — Tests for the error hierarchy in errors.ts.
 *
 * Covers:
 * 1. CliBuilderError — base class, instanceof, message, name
 * 2. SpecError — extends CliBuilderError, own name, instanceof chain
 * 3. ParseErrors — collects multiple errors, combined message, single-error message
 */

import { describe, it, expect } from "vitest";
import { CliBuilderError, SpecError, ParseErrors } from "../errors.js";
import type { ParseError } from "../errors.js";

// ---------------------------------------------------------------------------
// CliBuilderError
// ---------------------------------------------------------------------------

describe("CliBuilderError", () => {
  it("can be constructed and thrown", () => {
    expect(() => {
      throw new CliBuilderError("base error");
    }).toThrow("base error");
  });

  it("has the correct name property", () => {
    const err = new CliBuilderError("msg");
    expect(err.name).toBe("CliBuilderError");
  });

  it("is an instance of Error", () => {
    const err = new CliBuilderError("msg");
    expect(err instanceof Error).toBe(true);
  });

  it("is an instance of CliBuilderError", () => {
    const err = new CliBuilderError("msg");
    expect(err instanceof CliBuilderError).toBe(true);
  });

  it("carries the message", () => {
    const err = new CliBuilderError("something went wrong");
    expect(err.message).toBe("something went wrong");
  });
});

// ---------------------------------------------------------------------------
// SpecError
// ---------------------------------------------------------------------------

describe("SpecError", () => {
  it("can be constructed and thrown", () => {
    expect(() => {
      throw new SpecError("invalid spec");
    }).toThrow("invalid spec");
  });

  it("has the correct name property", () => {
    const err = new SpecError("invalid spec");
    expect(err.name).toBe("SpecError");
  });

  it("is an instance of Error", () => {
    const err = new SpecError("msg");
    expect(err instanceof Error).toBe(true);
  });

  it("is an instance of CliBuilderError", () => {
    const err = new SpecError("msg");
    expect(err instanceof CliBuilderError).toBe(true);
  });

  it("is an instance of SpecError", () => {
    const err = new SpecError("msg");
    expect(err instanceof SpecError).toBe(true);
  });

  it("is NOT an instance of ParseErrors", () => {
    const err = new SpecError("msg");
    expect(err instanceof ParseErrors).toBe(false);
  });

  it("carries the message", () => {
    const err = new SpecError("duplicate flag id");
    expect(err.message).toBe("duplicate flag id");
  });
});

// ---------------------------------------------------------------------------
// ParseErrors
// ---------------------------------------------------------------------------

describe("ParseErrors", () => {
  const singleError: ParseError = {
    errorType: "unknown_flag",
    message: "Unknown flag '--foo'",
    context: ["tool"],
  };

  const multipleErrors: ParseError[] = [
    {
      errorType: "unknown_flag",
      message: "Unknown flag '--foo'",
      context: ["tool"],
    },
    {
      errorType: "missing_required_flag",
      message: "--output is required",
      context: ["tool"],
    },
    {
      errorType: "conflicting_flags",
      message: "-v and -q cannot be used together",
      context: ["tool"],
    },
  ];

  it("can be constructed and thrown", () => {
    expect(() => {
      throw new ParseErrors([singleError]);
    }).toThrow(ParseErrors);
  });

  it("has the correct name property", () => {
    const err = new ParseErrors([singleError]);
    expect(err.name).toBe("ParseErrors");
  });

  it("is an instance of Error", () => {
    const err = new ParseErrors([singleError]);
    expect(err instanceof Error).toBe(true);
  });

  it("is an instance of CliBuilderError", () => {
    const err = new ParseErrors([singleError]);
    expect(err instanceof CliBuilderError).toBe(true);
  });

  it("is an instance of ParseErrors", () => {
    const err = new ParseErrors([singleError]);
    expect(err instanceof ParseErrors).toBe(true);
  });

  it("is NOT an instance of SpecError", () => {
    const err = new ParseErrors([singleError]);
    expect(err instanceof SpecError).toBe(false);
  });

  it("stores the errors array", () => {
    const err = new ParseErrors(multipleErrors);
    expect(err.errors).toHaveLength(3);
    expect(err.errors[0].errorType).toBe("unknown_flag");
  });

  it("single-error message is just the error message (no prefix)", () => {
    const err = new ParseErrors([singleError]);
    expect(err.message).toBe("Unknown flag '--foo'");
  });

  it("multiple-error message includes count and all messages", () => {
    const err = new ParseErrors(multipleErrors);
    expect(err.message).toContain("3 parse errors");
    expect(err.message).toContain("Unknown flag");
    expect(err.message).toContain("--output is required");
    expect(err.message).toContain("-v and -q cannot be used together");
  });

  it("errors are accessible by index", () => {
    const err = new ParseErrors(multipleErrors);
    expect(err.errors[1].errorType).toBe("missing_required_flag");
    expect(err.errors[2].errorType).toBe("conflicting_flags");
  });

  it("errors with suggestion field are preserved", () => {
    const errWithSuggestion: ParseError = {
      errorType: "unknown_flag",
      message: "Unknown flag '--mesage'. Did you mean '--message'?",
      suggestion: "--message",
      context: ["git", "commit"],
    };
    const err = new ParseErrors([errWithSuggestion]);
    expect(err.errors[0].suggestion).toBe("--message");
    expect(err.errors[0].context).toEqual(["git", "commit"]);
  });

  it("can be caught by catching CliBuilderError", () => {
    let caught: CliBuilderError | null = null;
    try {
      throw new ParseErrors([singleError]);
    } catch (e) {
      if (e instanceof CliBuilderError) {
        caught = e;
      }
    }
    expect(caught).not.toBeNull();
    expect(caught instanceof ParseErrors).toBe(true);
  });
});
