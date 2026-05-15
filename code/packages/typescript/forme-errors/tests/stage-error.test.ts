/**
 * forme-errors — StageError tests
 */

import { describe, it, expect } from "vitest";
import { StageError, ERROR_CODES } from "../src/index.js";
import type { LogicalId } from "@coding-adventures/forme-types";

const SAMPLE_ID = "01952c0d-7e63-7000-8000-000000000000" as LogicalId;

describe("StageError construction", () => {
  it("requires only code and message", () => {
    const e = new StageError({ code: "X", message: "y" });
    expect(e.code).toBe("X");
    expect(e.message).toBe("y");
    expect(e.inputPath).toBeNull();
    expect(e.inputId).toBeNull();
    expect(e.stageName).toBeNull();
    expect(e.cause).toBeUndefined();
    expect(e.recoverable).toBe(false);
    expect(e.fields).toEqual({});
  });

  it("populates every optional field when provided", () => {
    const cause = new Error("boom");
    const e = new StageError({
      code:        ERROR_CODES.PARSE_ERROR,
      message:     "Bad markdown",
      inputPath:   "posts/hello.md",
      inputId:     SAMPLE_ID,
      stageName:   "@forme/parse-markdown",
      cause,
      recoverable: true,
      fields:      { line: 3, col: 12 },
    });
    expect(e.code).toBe("PARSE_ERROR");
    expect(e.inputPath).toBe("posts/hello.md");
    expect(e.inputId).toBe(SAMPLE_ID);
    expect(e.stageName).toBe("@forme/parse-markdown");
    expect(e.cause).toBe(cause);
    expect(e.recoverable).toBe(true);
    expect(e.fields).toEqual({ line: 3, col: 12 });
  });

  it("freezes fields so async hops can't mutate them", () => {
    const e = new StageError({
      code: "X", message: "y", fields: { a: 1 },
    });
    expect(() => {
      // @ts-expect-error — readonly at the type level.
      e.fields.a = 99;
    }).toThrow(TypeError);
    expect(e.fields.a).toBe(1);
  });

  it("is an Error instance and an instanceof StageError", () => {
    const e = new StageError({ code: "X", message: "y" });
    expect(e).toBeInstanceOf(Error);
    expect(e).toBeInstanceOf(StageError);
  });

  it("sets `name` to the constructor name", () => {
    const e = new StageError({ code: "X", message: "y" });
    expect(e.name).toBe("StageError");
  });

  it("propagates cause through ES2022 Error chaining", () => {
    const cause = new Error("upstream");
    const e = new StageError({ code: "X", message: "y", cause });
    expect(e.cause).toBe(cause);
  });

  it("distinguishes 'no cause' from 'cause was null'", () => {
    const eNone = new StageError({ code: "X", message: "y" });
    const eNull = new StageError({ code: "X", message: "y", cause: null });
    expect(eNone.cause).toBeUndefined();
    expect(eNull.cause).toBeNull();
  });
});

describe("StageError.toJson", () => {
  it("emits a stable structured shape", () => {
    const e = new StageError({
      code:      ERROR_CODES.IO_NOT_FOUND,
      message:   "missing",
      inputPath: "posts/missing.md",
      stageName: "@forme/source-fs",
    });
    expect(e.toJson()).toEqual({
      name:        "StageError",
      code:        "IO_NOT_FOUND",
      message:     "missing",
      inputPath:   "posts/missing.md",
      inputId:     null,
      stageName:   "@forme/source-fs",
      recoverable: false,
      fields:      {},
      cause:       null,
    });
  });

  it("stringifies cause to keep the JSON shape stable", () => {
    const cause = new Error("upstream");
    const e = new StageError({ code: "X", message: "y", cause });
    const json = e.toJson() as { cause: string };
    expect(typeof json.cause).toBe("string");
    expect(json.cause).toContain("upstream");
  });

  it("renders absent cause as JSON null, not undefined", () => {
    const e = new StageError({ code: "X", message: "y" });
    const json = e.toJson() as { cause: unknown };
    expect(json.cause).toBeNull();
  });
});
