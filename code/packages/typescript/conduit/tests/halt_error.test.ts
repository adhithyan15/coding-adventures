/**
 * halt_error.test.ts — unit tests for HaltError, halt(), and redirect().
 *
 * These tests run entirely in Node.js with no native code — they only cover
 * the pure-TypeScript halt/redirect helpers.
 */

import { describe, it, expect } from "vitest";
import { HaltError, halt, redirect } from "../src/halt_error.js";

// ── HaltError construction ────────────────────────────────────────────────────

describe("HaltError", () => {
  it("stores status, body, and headers", () => {
    const err = new HaltError(503, "Maintenance", { "Retry-After": "3600" });
    expect(err.status).toBe(503);
    expect(err.body).toBe("Maintenance");
    expect(err.haltHeaderPairs).toEqual([["Retry-After", "3600"]]);
  });

  it("sets __conduit_halt sentinel to true", () => {
    const err = new HaltError(200, "");
    expect(err.__conduit_halt).toBe(true);
  });

  it("defaults body to empty string and headers to empty array", () => {
    const err = new HaltError(204);
    expect(err.body).toBe("");
    expect(err.haltHeaderPairs).toEqual([]);
  });

  it("is an instanceof Error", () => {
    const err = new HaltError(404, "Not Found");
    expect(err).toBeInstanceOf(Error);
    expect(err).toBeInstanceOf(HaltError);
  });

  it("has name = 'HaltError'", () => {
    expect(new HaltError(200, "").name).toBe("HaltError");
  });

  it("message contains the status code", () => {
    const err = new HaltError(422, "body");
    expect(err.message).toContain("422");
  });

  it("encodes multiple headers as pairs", () => {
    const err = new HaltError(200, "", {
      "X-A": "1",
      "X-B": "2",
    });
    // Order is Object.entries() order — insertion order in V8.
    expect(err.haltHeaderPairs).toHaveLength(2);
    const map = Object.fromEntries(err.haltHeaderPairs);
    expect(map["X-A"]).toBe("1");
    expect(map["X-B"]).toBe("2");
  });
});

// ── halt() ────────────────────────────────────────────────────────────────────

describe("halt()", () => {
  it("throws a HaltError with the given status and body", () => {
    expect(() => halt(404, "Not found")).toThrow(HaltError);
    expect(() => halt(404, "Not found")).toThrow("404");
  });

  it("the thrown HaltError carries correct fields", () => {
    let caught: HaltError | null = null;
    try {
      halt(503, "Under maintenance", { "Retry-After": "60" });
    } catch (e) {
      caught = e as HaltError;
    }
    expect(caught).not.toBeNull();
    expect(caught!.status).toBe(503);
    expect(caught!.body).toBe("Under maintenance");
    expect(caught!.haltHeaderPairs).toEqual([["Retry-After", "60"]]);
  });

  it("defaults body to empty string", () => {
    let caught: HaltError | null = null;
    try { halt(200); } catch (e) { caught = e as HaltError; }
    expect(caught!.body).toBe("");
  });

  it("defaults headers to empty", () => {
    let caught: HaltError | null = null;
    try { halt(200); } catch (e) { caught = e as HaltError; }
    expect(caught!.haltHeaderPairs).toEqual([]);
  });

  it("never returns (return type is never)", () => {
    // TypeScript enforces this statically, but we also verify at runtime.
    const fn = () => halt(200);
    expect(fn).toThrow(HaltError);
  });
});

// ── redirect() ────────────────────────────────────────────────────────────────

describe("redirect()", () => {
  it("throws a HaltError with a Location header", () => {
    let caught: HaltError | null = null;
    try { redirect("https://example.com"); } catch (e) { caught = e as HaltError; }
    expect(caught).not.toBeNull();
    expect(caught!.status).toBe(302);
    expect(caught!.haltHeaderPairs).toEqual([["Location", "https://example.com"]]);
    expect(caught!.body).toBe("");
  });

  it("defaults to 302 Found", () => {
    let caught: HaltError | null = null;
    try { redirect("/home"); } catch (e) { caught = e as HaltError; }
    expect(caught!.status).toBe(302);
  });

  it("accepts a custom status code (301 Moved Permanently)", () => {
    let caught: HaltError | null = null;
    try { redirect("/new-path", 301); } catch (e) { caught = e as HaltError; }
    expect(caught!.status).toBe(301);
  });

  it("accepts status 303 See Other", () => {
    let caught: HaltError | null = null;
    try { redirect("/result", 303); } catch (e) { caught = e as HaltError; }
    expect(caught!.status).toBe(303);
    expect(caught!.haltHeaderPairs).toEqual([["Location", "/result"]]);
  });
});
